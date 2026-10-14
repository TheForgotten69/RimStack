//! What the screens other than Mods show. Each list is worked out on a
//! background thread and handed to QML as JSON text in a property.

use rimstack_core::base::locks::Shared;
use rimstack_core::say;
use rimstack_core::{
    kept::launches,
    kept::lists,
    mods::library::{Library, ModId},
    rimworld::id,
    rimworld::loadfolders,
    rimworld::logs,
    rimworld::paths::Source,
    rimworld::rimsaves,
    rimworld::saves,
    rimworld::workshop,
};
use std::collections::HashMap;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("rimstack-app/src/application.h");
        #[rust_name = "set_window_icon"]
        fn setWindowIcon(resource: &QString);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
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

mod appearance;
mod columns;
pub(crate) mod download_report;
mod kept_lists;
pub(crate) mod log_report;
pub(crate) mod save_report;

use log_report::*;

#[derive(Default)]
pub struct BackendRust {
    watcher: rimstack_core::base::watch::Watcher,
}

pub fn use_window_icon() {
    qobject::set_window_icon(&QString::from(":/qt/qml/RimStack/Ui/assets/rimstack.svg"));
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

    fn summary(&self) -> QString {
        crate::reports::said(&summary(crate::app()))
    }
}
