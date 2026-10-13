use rimstack_core::say;

mod backend;
mod model;
mod state;

pub use state::*;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};
use std::sync::LazyLock;
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
    match rimstack_core::desktop::install(&home, &exe, entry, icon) {
        // A running desktop holds the icons it has looked up: KDE reads
        // them again on this signal. Without the tool, the next login does.
        Ok(true) => {
            let told = std::process::Command::new("dbus-send")
                .args(["--session", "--type=signal", "/KIconLoader"])
                .args(["org.kde.KIconLoader.iconChanged", "int32:0"])
                .status();
            if let Err(problem) = told {
                eprintln!("icons changed, desktop not told: {problem}");
            }
        }
        Ok(false) => {}
        Err(problem) => eprintln!("desktop entry: {problem}"),
    }
}

/// The short-lived process that asks Steam to subscribe or unsubscribe:
/// `rimstack --steamworks=subscribe:1,2`. It ends with 0 when Steam took
/// the requests, and says why not otherwise.
fn steamworks(request: &str) -> std::process::ExitCode {
    use rimstack_core::steamworks;
    let asked = steamworks::parse(request).ok_or("not a request".to_string());
    let library = PATHS.game.as_deref().and_then(steamworks::library);
    let done = asked.and_then(|(action, ids)| {
        let library = library.ok_or_else(|| say!("the game's Steam library was not found"))?;
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

fn main() -> std::process::ExitCode {
    if let Some(request) =
        std::env::args().find_map(|a| a.strip_prefix("--steamworks=").map(String::from))
    {
        return steamworks(&request);
    }
    STARTED.get_or_init(Instant::now);
    // Before the mods are read: their problems are put in words as they are.
    backend::speak(settings().get("language").unwrap_or_default());
    if !std::env::args().any(|a| a == "--demo" || a == "--selftest") {
        std::thread::spawn(|| {
            LazyLock::force(&COMMUNITY);
            library();
        });
    }
    #[cfg(target_os = "linux")]
    if !std::env::args().any(|a| a.starts_with("--shot") || a.starts_with("--check")) {
        put_among_applications();
    }
    let mut app = QGuiApplication::new();
    if let Some(app) = app.as_mut() {
        keep_app(app);
    }
    backend::use_window_icon();
    let font = settings().get("font").map(String::from);
    if let Some(family) = font {
        use_font(&family);
    }
    // Names the file the window's own settings (pane widths) are kept in.
    if let Some(app) = app.as_mut() {
        QGuiApplication::set_organization_name(app, &QString::from("RimStack"));
    }
    if let Some(app) = app.as_mut() {
        QGuiApplication::set_application_name(app, &QString::from("RimStack"));
    }
    QGuiApplication::set_desktop_file_name(&QString::from("rimstack"));
    let mut engine = QQmlApplicationEngine::new();

    if let Some(mut engine) = engine.as_mut() {
        // Without its window the app would run on unseen: stop instead.
        engine
            .as_mut()
            .on_object_creation_failed(|_, _| std::process::exit(1))
            .release();
        engine.load(&QUrl::from("qrc:/qt/qml/RimStack/Ui/qml/Main.qml"));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
    std::process::ExitCode::SUCCESS
}
