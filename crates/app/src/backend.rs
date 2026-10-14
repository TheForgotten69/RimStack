//! What the screens other than Mods show. Each list is worked out on a
//! background thread and handed to QML as JSON text in a property.

use rimstack_core::base::locks::Shared;
use rimstack_core::kept::settings::Switch;
use rimstack_core::{
    kept::launches,
    kept::lists,
    kept::owntheme,
    mods::library::{Library, ModId},
    rimworld::id,
    rimworld::loadfolders,
    rimworld::logs,
    rimworld::paths::Source,
    rimworld::rimsaves,
    rimworld::saves,
    rimworld::workshop,
};
use rimstack_core::{say, say_n};
use serde_json::{json, Value};
use std::collections::HashMap;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("rimstack-app/src/application.h");
        #[rust_name = "set_window_icon"]
        fn setWindowIcon(resource: &QString);
        #[rust_name = "set_font_family"]
        fn setFontFamily(family: &QString) -> QString;
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

        /// What changed on disk since the last look, to be read again:
        /// `{ mods, saves, log, modsConfig, theme, language }`, each true
        /// or false. `in_the_way`: the mods cannot be read again now.
        #[qinvokable]
        fn look(self: Pin<&mut Backend>, game_running: bool, in_the_way: bool) -> QString;

        /// The last start of the game, `{ time, errors }`, or null: from
        /// the launches on record, the log last read (when it was
        /// written, 0 for none; whether it is the one before the last;
        /// its kinds of error) and when the game last wrote a log.
        #[qinvokable]
        #[cxx_name = "lastStart"]
        fn last_start(self: &Backend, log_written: i64, previous: bool, errors: i32) -> QString;

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
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::path::PathBuf;

/// Asked once: the program does not appear or leave while RimStack runs.
static GIT: std::sync::LazyLock<Option<String>> =
    std::sync::LazyLock::new(rimstack_core::sources::git::installed);

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
    watcher: rimstack_core::base::watch::Watcher,
}

fn languages_dir(app: &crate::App) -> PathBuf {
    app.settings_file().with_file_name("languages")
}

pub fn use_window_icon() {
    qobject::set_window_icon(&QString::from(":/qt/qml/RimStack/Ui/assets/rimstack.svg"));
}

/// Call on the window's thread only.
pub fn use_font(family: &str) -> String {
    qobject::set_font_family(&QString::from(family)).to_string()
}

fn theme_file(app: &crate::App) -> PathBuf {
    app.settings_file().with_file_name("theme.json")
}

/// RimSort's file of your own rules: beside RimStack's data folder.
fn rimsort_rules_file(app: &crate::App) -> PathBuf {
    app.data_dir()
        .with_file_name("RimSort")
        .join("dbs")
        .join("userRules.json")
}

fn rimsort_rules(
    app: &crate::App,
) -> Result<Vec<(String, String)>, rimstack_core::base::error::Error> {
    let path = rimsort_rules_file(app);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| rimstack_core::base::error::Error::file(&path, e))?;
    Ok(rimstack_core::mods::community::parse(&text)?.pairs())
}

pub(crate) fn hidden_file(app: &crate::App) -> PathBuf {
    app.data_dir().join("hidden.json")
}

/// The folders the game loads a mod from with the load order being edited.
pub(crate) fn loaded_folders(app: &crate::App, library: &Library, mod_id: ModId) -> Vec<PathBuf> {
    let list = app.session().current().to_vec();
    let arranged = library.arrange(&list);
    let active: std::collections::HashSet<String> = arranged
        .active
        .iter()
        .filter_map(|m| library.get(m.installed?)?.key())
        .collect();
    let Some(root) = library.get(mod_id).map(|m| &m.path) else {
        return Vec::new();
    };
    loadfolders::folders(root, &library.game_version, &|k| active.contains(k))
}

pub(crate) fn snapshots_dir(app: &crate::App) -> PathBuf {
    app.data_dir().join("snapshots")
}

pub(crate) fn lists_dir(app: &crate::App) -> PathBuf {
    app.data_dir().join("lists")
}

/// Every folder and setting RimStack uses, as Settings shows them.
fn settings_report(app: &crate::App) -> crate::reports::SettingsReport {
    let paths = app.paths();
    let shown = |p: &Option<PathBuf>| p.as_ref().map(|p| p.to_string_lossy().into_owned());
    let settings = app.settings();
    let owned = |name: &str| settings.get(name).map(String::from);
    let config = paths.mods_config();
    let backup = config
        .as_ref()
        .map(|c| c.with_file_name("ModsConfig.xml.rimstack-backup"));
    crate::reports::SettingsReport {
        game: shown(&paths.game),
        config: shown(&paths.config),
        workshop: shown(&paths.workshop),
        game_version: paths.game_version.clone(),
        mods_config: shown(&config),
        backup: shown(&backup.filter(|b| b.exists())),
        set_game: owned("game"),
        set_config: owned("config"),
        set_workshop: owned("workshop"),
        launch_arguments: owned("launchArguments"),
        launch_directly: settings.launch_directly(),
        language: owned("language"),
        steamcmd: app.steamcmd().is_some(),
        interface_scale: settings.get("interfaceScale").unwrap_or("1").to_string(),
        font: owned("font"),
        community_rules: settings.switch(Switch::CommunityRules),
        version_warnings: settings.switch(Switch::VersionWarnings),
        version_aside_for: owned("versionAsideFor"),
        file_hints: settings.switch(Switch::FileHints),
        git_for_workshop: settings.switch(Switch::GitForWorkshop),
        git: GIT.clone(),
        self_check: settings.switch(Switch::SelfCheck),
        logs_kept: log_report::logs_kept_limit(app),
        logs_kept_now: logs::kept(&log_report::kept_logs_dir(app)).len(),
        replacements_ignored: settings
            .listed("replacementsIgnored")
            .map(String::from)
            .collect(),
        setup_done: settings.get("setupDone").is_some(),
        rimsort_rules: rimsort_rules_file(app).is_file(),
        data_folder: app.data_dir().to_string_lossy().into_owned(),
        settings_file: app.settings_file().to_string_lossy().into_owned(),
        diagnostics_file: app.diagnostics_file().to_string_lossy().into_owned(),
        version: env!("CARGO_PKG_VERSION"),
    }
}

/// The counts Settings shows beside its sections.
fn summary(app: &crate::App) -> crate::reports::Summary {
    let config = app.paths().config.clone();
    let saves = config
        .as_ref()
        .and_then(|config| std::fs::read_dir(config.join("Saves")).ok())
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
    let kept = config.as_deref().and_then(rimsaves::settings);
    let last = launches::read(&launches_file(app)).pop();
    let ignored = app.replacements_ignored();
    let replacements = match &*app.library() {
        Ok((library, _)) => library
            .workshop_mods()
            .keys()
            .filter(|id| app.replacements().contains_key(**id))
            .filter(|id| !ignored.contains(**id))
            .count(),
        Err(_) => 0,
    };
    crate::reports::Summary {
        saves,
        folders: kept.map_or(1, |s| s.folders.len()),
        snapshots: lists::count(&snapshots_dir(app)),
        replacements,
        rules: app.user_rules().reading().len(),
        last_launch: last.map(|l| crate::reports::LastLaunch {
            time: l.time,
            errors: l.errors,
            mods: l.ids.len(),
        }),
    }
}

pub(crate) fn ids_from(json: &QString) -> Vec<ModId> {
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
                let copied = rimstack_core::sources::maintenance::copy_to_local(&from, &local);
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
        let said = match rimstack_core::sources::maintenance::set_mod_settings_aside(config, &aside)
        {
            Ok(0) => say!("No mod has settings"),
            Ok(count) => say_n!(
                count,
                "{} file moved to {}",
                "{} files moved to {}",
                aside.display()
            ),
            Err(problem) => problem.into(),
        };
        QString::from(&said)
    }

    fn settings(&self) -> QString {
        crate::reports::said(&settings_report(crate::app()))
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
        crate::reports::said(&summary(crate::app()))
    }
}
