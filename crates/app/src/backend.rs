//! What the screens other than Mods show. Each list is worked out on a
//! background thread and handed to QML as JSON text in a property.

use rimstack_core::locks::Shared;
use rimstack_core::say;
use rimstack_core::{
    id, launches, library::Library, lists, loadfolders, logs, owntheme, paths::Source, rimsaves,
    saves, workshop,
};
use serde_json::{json, Value};
use std::collections::HashMap;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("rimstack-app/src/window_icon.h");
        /// Every window of the app carries the icon at this resource path.
        #[rust_name = "set_window_icon"]
        fn setWindowIcon(resource: &QString);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, release)]
        #[qproperty(bool, release_busy, cxx_name = "releaseBusy")]
        type Backend = super::BackendRust;
    }

    impl cxx_qt::Threading for Backend {}

    extern "RustQt" {
        /// Starts the game's own program, without Steam, with the launch
        /// arguments from the settings. Returns what went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "startGameDirectly"]
        fn start_game_directly(self: &Backend) -> QString;

        /// Moves a mod's folder to the system's bin. Returns what went
        /// wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "binMod"]
        fn bin_mod(self: &Backend, folder: &QString) -> QString;

        /// Copies a mod's folder into the game's own `Mods` folder, in
        /// the background; `copied` then says what went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "copyToLocal"]
        fn copy_to_local(self: Pin<&mut Backend>, folder: &QString);

        #[qsignal]
        fn copied(self: Pin<&mut Backend>, problem: QString);

        /// Moves the settings every mod keeps out of the game's `Config`
        /// folder into a dated folder of RimStack's. Returns a sentence
        /// saying what happened.
        #[qinvokable]
        #[cxx_name = "setModSettingsAside"]
        fn set_mod_settings_aside(self: &Backend) -> QString;

        /// The languages there are files for, as a JSON array of names.
        #[qinvokable]
        fn languages(self: &Backend) -> QString;

        /// A language's texts, as a JSON object from English to it;
        /// "{}" for English or a file that is not one.
        #[qinvokable]
        #[cxx_name = "languageWords"]
        fn language_words(self: &Backend, name: &QString) -> QString;

        /// Writes `languages/<name>.json` listing every text of the
        /// interface, unless it is there. Returns `{ file }`, or
        /// `{ problem }`.
        #[qinvokable]
        #[cxx_name = "startLanguage"]
        fn start_language(self: &Backend, name: &QString) -> QString;

        /// The colours the user's `theme.json` gives for this theme, as
        /// a JSON object; "{}" when it gives none.
        #[qinvokable]
        #[cxx_name = "ownTheme"]
        fn own_theme(self: &Backend, theme: &QString) -> QString;

        /// Of the colours on show, given as a JSON object of name to
        /// `#rrggbb`, the pairs under the contrast they need: a JSON
        /// array of { what, found, needed }.
        #[qinvokable]
        #[cxx_name = "shortContrast"]
        fn short_contrast(self: &Backend, colours: &QString) -> QString;

        /// Writes `theme.json` with these colours for this theme, or adds
        /// them to a file that has none for it. Returns `{ file }`, or
        /// `{ problem }`.
        #[qinvokable]
        #[cxx_name = "startOwnTheme"]
        fn start_own_theme(self: &Backend, theme: &QString, colours: &QString) -> QString;

        /// The folders RimStack uses and its own settings, as JSON.
        #[qinvokable]
        fn settings(self: &Backend) -> QString;

        /// One setting as it is kept; empty when it was never set.
        #[qinvokable]
        fn setting(self: &Backend, name: &QString) -> QString;

        /// Returns what went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "setSetting"]
        fn set_setting(self: &Backend, name: &QString, value: &QString) -> QString;

        /// Copy your own rules from RimSort, if it is installed. Returns a
        /// sentence saying what happened.
        #[qinvokable]
        #[cxx_name = "importRimSortRules"]
        fn import_rimsort_rules(self: &Backend) -> QString;

        /// Ask GitHub whether a newer RimStack is published; fills
        /// `release`. Uses the network.
        #[qinvokable]
        #[cxx_name = "checkRelease"]
        fn check_release(self: Pin<&mut Backend>);

        /// Small facts for the summary on the Mods screen, as a JSON
        /// object: the last launch, and how many saves, snapshots,
        /// replacements and rules of your own there are.
        #[qinvokable]
        fn summary(self: &Backend) -> QString;

        /// When each thing shown was last changed on disk, as a JSON
        /// object of times by name: `mods`, `saves`, `log`, `modsConfig`,
        /// `theme`, `language`. A time that differs from the one before
        /// says what to read again.
        #[qinvokable]
        fn stamps(self: &Backend) -> QString;

        /// New texts take this font family from now on; empty for the
        /// system's own. Returns the family in use, for `Theme.refont`
        /// to give the texts already on screen.
        #[qinvokable]
        #[cxx_name = "useFont"]
        fn use_font(self: &Backend, family: &QString) -> QString;

        /// The chosen columns ("author,version,load") without this one,
        /// or with it last.
        #[qinvokable]
        #[cxx_name = "columnsToggled"]
        fn columns_toggled(self: &Backend, kept: &QString, column: &QString) -> QString;

        /// The chosen columns with this one at place `to` among the others.
        #[qinvokable]
        #[cxx_name = "columnsMoved"]
        fn columns_moved(self: &Backend, kept: &QString, column: &QString, to: i32) -> QString;

        /// The widths kept ("author=180,load=70") as a JSON object.
        #[qinvokable]
        #[cxx_name = "columnWidths"]
        fn column_widths(self: &Backend, kept: &QString) -> QString;

        /// The widths kept with this column at `width`, at 100% text.
        #[qinvokable]
        #[cxx_name = "columnSized"]
        fn column_sized(self: &Backend, kept: &QString, column: &QString, width: f64) -> QString;

        /// What the list is shown by once the heading `asked` is pressed,
        /// as `[column, descending]`; the empty column is the load order.
        #[qinvokable]
        #[cxx_name = "columnPressed"]
        fn column_pressed(
            self: &Backend,
            by: &QString,
            descending: bool,
            asked: &QString,
        ) -> QString;

        #[qinvokable]
        #[cxx_name = "recordLaunch"]
        fn record_launch(self: &Backend);

        #[qinvokable]
        #[cxx_name = "gameRunning"]
        fn game_running(self: &Backend) -> bool;

    }
}

use core::pin::Pin;
use cxx_qt::Threading;
use cxx_qt_lib::QString;
use std::path::PathBuf;

/// Asked once: the program does not appear or leave while RimStack runs.
static GIT: std::sync::LazyLock<Option<String>> =
    std::sync::LazyLock::new(rimstack_core::git::installed);

mod appearance;
mod columns;
pub use appearance::speak;
mod databases;
pub(crate) mod download_report;
mod kept_lists;
pub(crate) mod log_report;
pub(crate) mod save_report;

use log_report::*;

#[derive(Default)]
pub struct BackendRust {
    /// A newer RimStack, as `{ version, page }`; `{}` once asked and
    /// none is; `{ problem }` where GitHub could not be asked.
    release: QString,
    release_busy: bool,
}

fn languages_dir() -> PathBuf {
    crate::app().settings_file().with_file_name("languages")
}

/// The app's icon on its window, for the desktop to show with it.
pub fn use_window_icon() {
    qobject::set_window_icon(&QString::from(":/qt/qml/RimStack/Ui/assets/rimstack.svg"));
}

fn theme_file() -> PathBuf {
    crate::app().settings_file().with_file_name("theme.json")
}

/// RimSort's file of your own rules: beside RimStack's data folder.
fn rimsort_rules_file() -> PathBuf {
    crate::app()
        .data_dir()
        .with_file_name("RimSort")
        .join("dbs")
        .join("userRules.json")
}

fn rimsort_rules() -> Result<Vec<(String, String)>, rimstack_core::error::Error> {
    let path = rimsort_rules_file();
    let text =
        std::fs::read_to_string(&path).map_err(|e| rimstack_core::error::Error::file(&path, e))?;
    Ok(rimstack_core::community::parse(&text)?.pairs())
}

pub(crate) fn hidden_file() -> PathBuf {
    crate::app().data_dir().join("hidden.json")
}

/// The folders the game loads a mod from with the load order being edited.
pub(crate) fn loaded_folders(library: &Library, mod_id: usize) -> Vec<PathBuf> {
    let list = crate::app().session().current().to_vec();
    let arranged = library.arrange(&list);
    let active: std::collections::HashSet<String> = arranged
        .active
        .iter()
        .filter_map(|m| library.installed.get(m.installed?)?.key())
        .collect();
    let root = &library.installed[mod_id].path;
    loadfolders::folders(root, &library.game_version, &|k| active.contains(k))
}

pub(crate) fn snapshots_dir() -> PathBuf {
    crate::app().data_dir().join("snapshots")
}

pub(crate) fn lists_dir() -> PathBuf {
    crate::app().data_dir().join("lists")
}

pub(crate) fn ids_from(json: &QString) -> Vec<usize> {
    let json = json.to_string();
    serde_json::from_str(&json).unwrap_or_else(|e| {
        rimstack_core::problem!("mod ids given as {json:?}: {e}");
        Vec::new()
    })
}

pub(crate) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl qobject::Backend {
    fn bin_mod(&self, folder: &QString) -> QString {
        let folder = PathBuf::from(folder.to_string());
        // Only a mod's own folder: one that holds an About folder.
        if !folder.join("About").is_dir() {
            return QString::from(&say!("That folder is not a mod's"));
        }
        match trash::delete(&folder) {
            Ok(()) => QString::default(),
            Err(problem) => QString::from(&problem.to_string()),
        }
    }

    fn copy_to_local(self: Pin<&mut Self>, folder: &QString) {
        let from = PathBuf::from(folder.to_string());
        let local = crate::app().paths().local_mods();
        crate::background::run(
            self.qt_thread(),
            move || {
                let Some(local) = local else {
                    return say!("The game's folder is not known");
                };
                let copied = rimstack_core::upkeep::copy_to_local(&from, &local);
                copied.err().map(String::from).unwrap_or_default()
            },
            |backend, problem| backend.copied(QString::from(&problem)),
        );
    }

    fn set_mod_settings_aside(&self) -> QString {
        let paths = crate::app().paths();
        let Some(config) = paths.config.as_ref() else {
            return QString::from(&say!("The game's settings folder is not known"));
        };
        let aside = crate::app()
            .data_dir()
            .join(format!("mod-settings-{}", now_secs()));
        let said = match rimstack_core::upkeep::set_mod_settings_aside(config, &aside) {
            Ok(0) => say!("No mod has settings"),
            Ok(count) => say!("{} files moved to {}", count, aside.display()),
            Err(problem) => problem.into(),
        };
        QString::from(&said)
    }

    fn settings(&self) -> QString {
        let paths = crate::app().paths();
        let shown = |p: &Option<PathBuf>| p.as_ref().map(|p| p.to_string_lossy().into_owned());
        let settings = crate::app().settings();
        let config = paths.mods_config();
        let backup = config
            .as_ref()
            .map(|c| c.with_file_name("ModsConfig.xml.rimstack-backup"));
        let report = json!({
            "game": shown(&paths.game),
            "config": shown(&paths.config),
            "workshop": shown(&paths.workshop),
            "gameVersion": paths.game_version,
            "modsConfig": shown(&config),
            "backup": backup.filter(|b| b.exists()).map(|b| b.to_string_lossy().into_owned()),
            "setGame": settings.get("game"),
            "setConfig": settings.get("config"),
            "setWorkshop": settings.get("workshop"),
            "launchArguments": settings.get("launchArguments"),
            "launchDirectly": settings.get("launch") == Some("direct"),
            "language": settings.get("language"),
            "steamcmd": rimstack_core::redownload::steamcmd(crate::app().data_dir()).is_some(),
            "interfaceScale": settings.get("interfaceScale").unwrap_or("1"),
            "font": settings.get("font"),
            "communityRules": settings.is_on("communityRules"),
            "versionWarnings": settings.get("versionWarnings") != Some("off"),
            "versionAsideFor": settings.get("versionAsideFor"),
            "fileHints": settings.get("fileHints") == Some("on"),
            "gitForWorkshop": settings.get("gitForWorkshop") == Some("on"),
            "git": *GIT,
            "selfCheck": settings.get("selfCheck") != Some("off"),
            "logsKept": log_report::logs_kept_limit(crate::app()),
            "logsKeptNow": logs::kept(&log_report::kept_logs_dir(crate::app())).len(),
            "replacementsIgnored": settings.listed("replacementsIgnored").collect::<Vec<_>>(),
            "setupDone": settings.get("setupDone").is_some(),
            "rimsortRules": rimsort_rules_file().is_file(),
            "dataFolder": crate::app().data_dir().to_string_lossy(),
            "settingsFile": crate::app().settings_file().to_string_lossy(),
            "diagnosticsFile": crate::app().diagnostics_file().to_string_lossy(),
            "version": env!("CARGO_PKG_VERSION"),
        });
        QString::from(&report.to_string())
    }

    fn setting(&self, name: &QString) -> QString {
        let kept = crate::app().settings();
        QString::from(kept.get(&name.to_string()).unwrap_or_default())
    }

    fn set_setting(&self, name: &QString, value: &QString) -> QString {
        match crate::app().set_setting(&name.to_string(), &value.to_string()) {
            Ok(()) => QString::default(),
            Err(problem) => QString::from(&problem.to_string()),
        }
    }

    fn summary(&self) -> QString {
        let config = crate::app().paths().config.clone().unwrap_or_default();
        let saves = std::fs::read_dir(config.join("Saves"))
            .into_iter()
            .flatten()
            .flatten();
        let saves = saves
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("rws"))
            })
            .count();
        let folders = rimsaves::settings(&config).map_or(1, |s| s.folders.len());
        let last = launches::read(&launches_file(crate::app())).pop();
        let ignored = crate::app().replacements_ignored();
        let replacements = match &*crate::app().library() {
            Ok((library, _)) => library
                .workshop_mods()
                .keys()
                .filter(|id| crate::app().replacements().contains_key(**id))
                .filter(|id| !ignored.contains(**id))
                .count(),
            Err(_) => 0,
        };
        let rules = crate::app().user_rules().reading().len();
        let report = json!({
            "saves": saves,
            "folders": folders,
            "snapshots": lists::all(&snapshots_dir()).len(),
            "replacements": replacements,
            "rules": rules,
            "lastLaunch": last.map(|l| json!({ "time": l.time, "errors": l.errors, "mods": l.ids.len() })),
        });
        QString::from(&report.to_string())
    }
}
