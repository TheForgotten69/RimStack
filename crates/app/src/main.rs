use rimstack_core::said;

mod changes;
mod downloads_data;
mod game_data;
mod hub;
mod jobs;
mod lists_data;
mod logs_data;
mod mod_rows;
mod mods_data;
mod qml;
mod reports;
mod saves_data;
mod search_data;
mod settings_data;
mod textures_data;
mod workspace;

pub use workspace::*;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};
use std::sync::Arc;
use std::time::Instant;

/// The desktop entry and the icon, kept in the program and written where
/// the desktop looks for them.
#[cfg(target_os = "linux")]
fn put_among_applications() {
    let home = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| Some(std::path::PathBuf::from(std::env::var_os("HOME")?).join(".local/share")));
    let (Some(home), Ok(exe)) = (home, std::env::current_exe()) else {
        return;
    };
    let entry = include_str!("../assets/rimstack.desktop");
    let icon = include_bytes!("../assets/rimstack.svg");
    match rimstack_core::kept::desktop::install(&home, &exe, entry, icon) {
        // A running desktop holds the icons it has looked up: KDE reads
        // them again on this signal. Without the tool, the next login does.
        Ok(true) => {
            let told = std::process::Command::new("dbus-send")
                .args(["--session", "--type=signal", "/KIconLoader"])
                .args(["org.kde.KIconLoader.iconChanged", "int32:0"])
                .status();
            if let Err(problem) = told {
                rimstack_core::problem!("icons changed, desktop not told: {problem}");
            }
        }
        Ok(false) => {}
        Err(problem) => rimstack_core::problem!("desktop entry: {problem}"),
    }
}

/// The short-lived process that asks Steam to subscribe or unsubscribe:
/// `rimstack --steamworks=subscribe:1,2`. It ends with 0 when Steam took
/// the requests, and says why not otherwise.
fn steamworks(app: &App, request: &str) -> std::process::ExitCode {
    use rimstack_core::base::error::Error;
    use rimstack_core::rimworld::steamworks;
    let asked = steamworks::parse(request).ok_or_else(|| Error::refused("not a request"));
    let library = app.paths().game.as_deref().and_then(steamworks::library);
    let done = asked.and_then(|(action, ids)| {
        let library = library
            .ok_or_else(|| Error::program(said!("the game's Steam library was not found")))?;
        steamworks::act(&library, action, &ids)
    });
    match done {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(problem) => {
            eprintln!("{problem}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// `--check` drives the Mods screen and saves the load order it finds:
/// that is for a copy of the game's settings, never the real ones.
fn check_on_real_settings() -> bool {
    let checking = std::env::args().any(|a| a.starts_with("--check"));
    checking && std::env::var_os("RIMSTACK_CONFIG").is_none()
}

fn main() -> std::process::ExitCode {
    if check_on_real_settings() {
        eprintln!("--check saves the load order: run tools/check-app.sh, which gives it a copy through RIMSTACK_CONFIG");
        return std::process::ExitCode::from(2);
    }
    let steam_request =
        std::env::args().find_map(|a| a.strip_prefix("--steamworks=").map(String::from));
    // Before any thread is started, as changing the environment must be.
    if steam_request.is_some() {
        for (name, value) in rimstack_core::rimworld::steamworks::environment() {
            std::env::set_var(name, value);
        }
    }
    let app = Arc::new(App::from_environment());
    if let Some(request) = steam_request {
        return steamworks(&app, &request);
    }
    STARTED.get_or_init(Instant::now);
    if let Err(problem) = rimstack_core::base::diagnostics::start(&app.diagnostics_file()) {
        rimstack_core::problem!("{}: {problem}", app.diagnostics_file().display());
    }
    rimstack_core::base::diagnostics::record_panics();
    rimstack_core::note!(
        "RimStack {} started on {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    );
    // Before the mods are read: their problems are put in words as they are.
    app.speak();
    if !std::env::args().any(|a| a == "--demo" || a == "--selftest") {
        let early = app.clone();
        crate::jobs::spawn("early read", move || {
            early.community();
            early.library();
        });
    }
    #[cfg(target_os = "linux")]
    if !std::env::args().any(|a| a.starts_with("--shot") || a.starts_with("--check")) {
        put_among_applications();
    }
    let mut qt = QGuiApplication::new();
    hub::use_window_icon();
    let font = app.settings().get("font").map(String::from);
    if let Some(family) = font {
        settings_data::qobject::set_font_family(&QString::from(&family));
    }
    // Names the file the window's own settings (pane widths) are kept in.
    if let Some(qt) = qt.as_mut() {
        QGuiApplication::set_organization_name(qt, &QString::from("RimStack"));
    }
    if let Some(qt) = qt.as_mut() {
        QGuiApplication::set_application_name(qt, &QString::from("RimStack"));
    }
    QGuiApplication::set_desktop_file_name(&QString::from("rimstack"));
    hub::hand_over(app);
    let mut engine = QQmlApplicationEngine::new();

    if let Some(mut engine) = engine.as_mut() {
        // Without its window the app would run on unseen: stop instead.
        engine
            .as_mut()
            .on_object_creation_failed(|_, _| std::process::exit(1))
            .release();
        engine.load(&QUrl::from("qrc:/qt/qml/RimStack/Ui/qml/Main.qml"));
    }
    // What the window ended with: the checks end it with 1 when one failed.
    let ended = qt.as_mut().map_or(1, |qt| qt.exec());
    std::process::ExitCode::from(u8::try_from(ended).unwrap_or(1))
}
