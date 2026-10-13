//! What the screens other than Mods show. Each list is worked out on a
//! background thread and handed to QML as JSON text in a property.

use rayon::prelude::*;
#[allow(unused_imports)]
use rimstack_core::tongue::named;
use rimstack_core::{
    conflicts, hidden::Hidden, id, launches, library::Library, lists, loadfolders, logs,
    modsconfig, owntheme, paths::Source, rimsaves, saves, textures, workshop,
};
use rimstack_core::{keep, say};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

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
        #[qproperty(QString, saves)]
        #[qproperty(bool, saves_busy, cxx_name = "savesBusy")]
        #[qproperty(QString, task_note, cxx_name = "taskNote")]
        #[qproperty(QString, release)]
        #[qproperty(bool, release_busy, cxx_name = "releaseBusy")]
        #[qproperty(i32, mods_written, cxx_name = "modsWritten")]
        #[qproperty(QString, logs)]
        #[qproperty(bool, logs_busy, cxx_name = "logsBusy")]
        #[qproperty(QString, updates)]
        #[qproperty(bool, updates_busy, cxx_name = "updatesBusy")]
        #[qproperty(QString, change_notes, cxx_name = "changeNotes")]
        #[qproperty(bool, change_notes_busy, cxx_name = "changeNotesBusy")]
        #[qproperty(QString, change_notes_state, cxx_name = "changeNotesState")]
        #[qproperty(QString, queue)]
        #[qproperty(bool, queue_busy, cxx_name = "queueBusy")]
        #[qproperty(QString, textures)]
        #[qproperty(bool, textures_busy, cxx_name = "texturesBusy")]
        #[qproperty(QString, textures_progress, cxx_name = "texturesProgress")]
        type Backend = super::BackendRust;
    }

    impl cxx_qt::Threading for Backend {}

    extern "RustQt" {
        /// List the saves and compare each with the load order being
        /// edited. Fills `saves` with a JSON object: the saves, newest
        /// first, and the RimSaves folders if that mod is in use.
        #[qinvokable]
        #[cxx_name = "loadSaves"]
        fn load_saves(self: Pin<&mut Backend>);

        /// Read the game's log (or, with `previous`, the log of the launch
        /// before) and the launch record. Fills `logs` with a JSON object:
        /// the errors grouped, the launches, and what changed since the
        /// last launch without errors.
        #[qinvokable]
        #[cxx_name = "loadLogs"]
        fn load_logs(self: Pin<&mut Backend>, previous: bool);

        /// The same for a log RimStack kept, by its path.
        #[qinvokable]
        #[cxx_name = "loadKeptLog"]
        fn load_kept_log(self: Pin<&mut Backend>, path: &QString);

        /// Copy the game's logs among the kept ones, up to the limit in
        /// the settings; the oldest give way.
        #[qinvokable]
        #[cxx_name = "keepLogs"]
        fn keep_logs(self: &Backend);

        /// Count every installed mod's textures and what stands beside
        /// them. Fills `textures` with a JSON array of the mods that have any.
        #[qinvokable]
        #[cxx_name = "loadTextures"]
        fn load_textures(self: Pin<&mut Backend>);

        /// Make DDS files for the mods with these library ids (a JSON
        /// array), or delete the ones RimStack made. `texturesProgress`
        /// says how far it is; the list is counted again at the end.
        #[qinvokable]
        #[cxx_name = "convertTextures"]
        fn convert_textures(self: Pin<&mut Backend>, mod_ids: &QString);
        #[qinvokable]
        #[cxx_name = "removeTextures"]
        fn remove_textures(self: Pin<&mut Backend>, mod_ids: &QString);

        /// Ask Steam which installed Workshop mods have a newer version.
        /// Fills `updates` with a JSON object. Uses the network.
        #[qinvokable]
        #[cxx_name = "checkUpdates"]
        fn check_updates(self: Pin<&mut Backend>);

        /// Fills `updates` with what the last check learnt, against the
        /// mods as they are on disk now. Does not use the network.
        #[qinvokable]
        #[cxx_name = "restoreUpdates"]
        fn restore_updates(self: Pin<&mut Backend>);

        /// Look up the Workshop links or ids in pasted text. Fills `queue`
        /// with a JSON object. Uses the network.
        #[qinvokable]
        #[cxx_name = "lookUp"]
        fn look_up(self: Pin<&mut Backend>, pasted: &QString);

        /// Bring a mod kept as a Git checkout up to its remote, in the
        /// background. `taskNote` then says what happened; it starts with
        /// "!" when nothing was changed.
        #[qinvokable]
        #[cxx_name = "updateFromGit"]
        fn update_from_git(self: Pin<&mut Backend>, folder: &QString);

        /// Starts the game's own program, without Steam, with the launch
        /// arguments from the settings. Returns what went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "startGameDirectly"]
        fn start_game_directly(self: &Backend) -> QString;

        /// Ask Steam to "subscribe" to, "unsubscribe" from or "download" these
        /// Workshop items (a JSON array of ids), in the background;
        /// `taskNote` says what happened. Steam must be running.
        #[qinvokable]
        #[cxx_name = "askSteam"]
        fn ask_steam(self: Pin<&mut Backend>, what: &QString, ids: &QString);

        /// Download Valve's SteamCMD into RimStack's data folder, in the
        /// background; `taskNote` says what happened.
        #[qinvokable]
        #[cxx_name = "installSteamcmd"]
        fn install_steamcmd(self: Pin<&mut Backend>);

        /// Download these Workshop items (a JSON array of ids) with
        /// SteamCMD into the game's own `Mods` folder, in the background;
        /// `taskNote` says what happened.
        #[qinvokable]
        #[cxx_name = "downloadWithSteamcmd"]
        fn download_with_steamcmd(self: Pin<&mut Backend>, ids: &QString);

        /// The DDS files of the installed mods the game would refuse, as a
        /// JSON array of { mod, file, why }.
        #[qinvokable]
        #[cxx_name = "ddsAudit"]
        fn dds_audit(self: &Backend) -> QString;

        /// Renames these DDS files (a JSON array of paths) out of the
        /// game's way. Returns how many could not be.
        #[qinvokable]
        #[cxx_name = "setDdsAside"]
        fn set_dds_aside(self: &Backend, files: &QString) -> i32;

        /// The package ids in a list someone pasted, as a JSON array.
        #[qinvokable]
        #[cxx_name = "idsInText"]
        fn ids_in_text(self: &Backend, text: &QString) -> QString;

        /// Moves a mod's folder to the system's bin. Returns what went
        /// wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "binMod"]
        fn bin_mod(self: &Backend, folder: &QString) -> QString;

        /// Copies a mod's folder into the game's own `Mods` folder.
        /// Returns what went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "copyToLocal"]
        fn copy_to_local(self: &Backend, folder: &QString) -> QString;

        /// Moves the settings every mod keeps out of the game's `Config`
        /// folder into a dated folder of RimStack's. Returns a sentence
        /// saying what happened.
        #[qinvokable]
        #[cxx_name = "setModSettingsAside"]
        fn set_mod_settings_aside(self: &Backend) -> QString;

        /// The Workshop id of a mod known by this name or package id from
        /// the Steam Workshop Database; empty when it is not known.
        #[qinvokable]
        #[cxx_name = "workshopIdOf"]
        fn workshop_id_of(self: &Backend, name: &QString) -> QString;

        /// The languages there are files for, as a JSON array of names.
        #[qinvokable]
        fn languages(self: &Backend) -> QString;

        /// A language's texts, as a JSON object from English to it;
        /// "{}" for English or a file that is not one.
        #[qinvokable]
        #[cxx_name = "languageWords"]
        fn language_words(self: &Backend, name: &QString) -> QString;

        /// Writes `languages/<name>.json` listing every text of the
        /// interface, unless it is there. Returns the file's path, or
        /// "!" and what went wrong.
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
        /// them to a file that has none for it. Returns the file's path,
        /// or "!" and what went wrong.
        #[qinvokable]
        #[cxx_name = "startOwnTheme"]
        fn start_own_theme(self: &Backend, theme: &QString, colours: &QString) -> QString;

        /// Download a Workshop mod again with SteamCMD and put it in the
        /// place of the copy in `folder`, in the background. `taskNote`
        /// says what happened, as for `updateFromGit`.
        #[qinvokable]
        fn redownload(self: Pin<&mut Backend>, workshop_id: &QString, folder: &QString);

        /// The folders RimStack uses and its own settings, as JSON.
        #[qinvokable]
        fn settings(self: &Backend) -> QString;

        /// Change one setting. Returns what went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "setSetting"]
        fn set_setting(self: &Backend, name: &QString, value: &QString) -> QString;

        /// Drop every list worked out from the mod folders as they were
        /// read before; each screen asks again when it is next shown.
        #[qinvokable]
        fn forget(self: Pin<&mut Backend>);

        /// The textures two active mods share, as a JSON object. Give the
        /// mod that loads earlier first (both by library id).
        #[qinvokable]
        fn compare(self: &Backend, earlier: i32, later: i32) -> QString;

        /// Choose which of two mods' versions of a texture the game uses,
        /// given both files. Keeping the earlier mod's hides the later
        /// mod's file by renaming it; keeping the later one's puts every
        /// file back. Returns what went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "keepTexture"]
        fn keep_texture(
            self: &Backend,
            earlier_file: &QString,
            later_file: &QString,
            keep_earlier: bool,
        ) -> QString;

        #[qinvokable]
        #[cxx_name = "hiddenIn"]
        fn hidden_in(self: &Backend, mod_id: i32) -> i32;

        /// Put every hidden texture of a mod back. Returns what went
        /// wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "unhideAll"]
        fn unhide_all(self: &Backend, mod_id: i32) -> QString;

        /// Copy your own rules from RimSort, if it is installed. Returns a
        /// sentence saying what happened.
        #[qinvokable]
        #[cxx_name = "importRimSortRules"]
        fn import_rimsort_rules(self: &Backend) -> QString;

        /// Keep the load order as it is now under `name`, among the
        /// snapshots; the ten newest are kept. Taken before anything that
        /// rearranges the whole list.
        #[qinvokable]
        fn snapshot(self: &Backend, name: &QString);

        #[qinvokable]
        fn snapshots(self: &Backend) -> QString;

        /// The package ids of a snapshot, as a JSON array; empty text if
        /// there is no such snapshot.
        #[qinvokable]
        #[cxx_name = "snapshotList"]
        fn snapshot_list(self: &Backend, name: &QString) -> QString;

        /// The installed Workshop mods that have a maintained replacement
        /// (the Use This Instead list), active ones first, as a JSON array.
        #[qinvokable]
        fn replacements(self: &Backend) -> QString;

        /// Clone a repository (`https://host/owner/repo`) into the game's
        /// own Mods folder, in the background; `taskNote` says what happened.
        #[qinvokable]
        #[cxx_name = "cloneMod"]
        fn clone_mod(self: Pin<&mut Backend>, link: &QString);

        /// Ask GitHub whether a newer RimStack is published; fills
        /// `release`. Uses the network.
        #[qinvokable]
        #[cxx_name = "checkRelease"]
        fn check_release(self: Pin<&mut Backend>);

        /// The community databases, as a JSON array of
        /// `{ name, use, updated }`; `updated` is 0 for one not downloaded.
        #[qinvokable]
        fn databases(self: &Backend) -> QString;

        /// Small facts for the summary on the Mods screen, as a JSON
        /// object: the last launch, and how many saves, snapshots,
        /// replacements and rules of your own there are.
        #[qinvokable]
        fn summary(self: &Backend) -> QString;

        /// Keep a list of package ids (a JSON array) under a name, among
        /// the saved lists. Returns what went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "saveIdsAs"]
        fn save_ids_as(self: &Backend, name: &QString, ids: &QString) -> QString;

        /// The load order as it was before RimStack first saved (kept as
        /// ModsConfig.xml.rimstack-original), as a JSON array of package
        /// ids; empty text if RimStack has never saved.
        /// When each thing shown was last changed on disk, as a JSON
        /// object of times by name: `mods`, `saves`, `log`, `modsConfig`,
        /// `theme`, `language`. A time that differs from the one before
        /// says what to read again.
        #[qinvokable]
        fn stamps(self: &Backend) -> QString;

        /// The load order in ModsConfig.xml as it is on disk now, as a
        /// JSON array of package ids; empty if it cannot be read.
        #[qinvokable]
        #[cxx_name = "listOnDisk"]
        fn list_on_disk(self: &Backend) -> QString;

        /// New texts take this font family from now on; empty for the
        /// system's own. Returns the family in use, for `Theme.refont`
        /// to give the texts already on screen.
        #[qinvokable]
        #[cxx_name = "useFont"]
        fn use_font(self: &Backend, family: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "originalList"]
        fn original_list(self: &Backend) -> QString;

        #[qinvokable]
        #[cxx_name = "savedLists"]
        fn saved_lists(self: &Backend) -> QString;

        /// Keep the load order being edited under a name. Returns what
        /// went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "saveListAs"]
        fn save_list_as(self: &Backend, name: &QString) -> QString;

        /// The package ids kept under a name, as a JSON array; empty text
        /// if there is no such list.
        #[qinvokable]
        #[cxx_name = "savedList"]
        fn saved_list(self: &Backend, name: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "deleteList"]
        fn delete_list(self: &Backend, name: &QString);

        #[qinvokable]
        #[cxx_name = "skipTextures"]
        fn skip_textures(self: Pin<&mut Backend>, mod_id: i32, skip: bool);

        #[qinvokable]
        #[cxx_name = "recordLaunch"]
        fn record_launch(self: &Backend);

        /// The game was seen starting without Play: in a round of the
        /// search for the broken mod, or from Steam. Notes the mods'
        /// versions, for "what changed since".
        #[qinvokable]
        #[cxx_name = "noteGameStarted"]
        fn note_game_started(self: &Backend);

        /// The search for the one mod that breaks the game, as JSON; see
        /// `broken_mod::state`.
        #[qinvokable]
        #[cxx_name = "huntState"]
        fn hunt_state(self: &Backend) -> QString;
        #[qinvokable]
        #[cxx_name = "huntStart"]
        fn hunt_start(self: &Backend) -> QString;
        #[qinvokable]
        #[cxx_name = "huntAnswer"]
        fn hunt_answer(self: &Backend, broken: bool) -> QString;
        #[qinvokable]
        #[cxx_name = "huntStop"]
        fn hunt_stop(self: &Backend);
        #[qinvokable]
        #[cxx_name = "textureDifference"]
        fn texture_difference(self: &Backend, earlier: &QString, later: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "sinceLaunch"]
        fn since_launch(self: &Backend) -> QString;

        /// The newest commits of the Git checkout in this folder, as a
        /// JSON array of `{ time, text }`.
        #[qinvokable]
        #[cxx_name = "gitNotes"]
        fn git_notes(self: &Backend, folder: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "fetchChangeNotes"]
        fn fetch_change_notes(self: Pin<&mut Backend>, asked: &QString);
        #[qinvokable]
        #[cxx_name = "listResetByGame"]
        fn list_reset_by_game(self: &Backend) -> QString;
        #[qinvokable]
        #[cxx_name = "gameRunning"]
        fn game_running(self: &Backend) -> bool;
        #[qinvokable]
        #[cxx_name = "snapshotSaved"]
        fn snapshot_saved(self: &Backend, name: &QString) -> QString;
        /// Adds a line to the history of what RimStack changed; `revert`
        /// is "list <snapshot>", "unhide <mod folder>" or nothing.
        #[qinvokable]
        #[cxx_name = "noteChange"]
        fn note_change(self: &Backend, what: &QString, revert: &QString);
        #[qinvokable]
        fn journal(self: &Backend) -> QString;
        #[qinvokable]
        #[cxx_name = "unhideIn"]
        fn unhide_in(self: &Backend, folder: &QString) -> QString;
        /// A conversion of textures stops after the mod it is on.
        #[qinvokable]
        #[cxx_name = "cancelTextures"]
        fn cancel_textures(self: &Backend);

        /// Copy the newest save into RimStack's data folder, keeping the
        /// last five copies. Returns what went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "backUpNewestSave"]
        fn back_up_newest_save(self: &Backend) -> QString;
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
pub use appearance::speak;
mod broken_mod;
mod databases;
mod download_report;
mod history;
mod kept_lists;
mod log_report;
mod save_report;
mod since_launch;
mod steam;
mod texture_report;
mod texture_work;

use download_report::*;
use log_report::*;
use save_report::*;
use texture_report::*;

#[derive(Default)]
pub struct BackendRust {
    task_note: QString,
    /// A newer RimStack, as `{ version, page }`; `{}` once asked and
    /// none is; `{ problem }` where GitHub could not be asked.
    release: QString,
    release_busy: bool,
    /// Goes up when a task here changed a mod's folder on disk.
    mods_written: i32,
    saves: QString,
    saves_busy: bool,
    logs: QString,
    logs_busy: bool,
    textures: QString,
    textures_busy: bool,
    textures_progress: QString,
    updates: QString,
    updates_busy: bool,
    change_notes: QString,
    change_notes_busy: bool,
    /// `{ done, asked, resume, refused }`: how far the asking is, when it goes on after Steam refused, whether it stopped for that.
    change_notes_state: QString,
    queue: QString,
    queue_busy: bool,
}

/// RimSort's file of your own rules: beside RimStack's data folder.
fn languages_dir() -> PathBuf {
    crate::settings_file().with_file_name("languages")
}

/// The app's icon on its window, for the desktop to show with it.
pub fn use_window_icon() {
    qobject::set_window_icon(&QString::from(":/qt/qml/RimStack/Ui/assets/rimstack.svg"));
}

fn theme_file() -> PathBuf {
    crate::settings_file().with_file_name("theme.json")
}

fn rimsort_rules_file() -> PathBuf {
    crate::data_dir()
        .with_file_name("RimSort")
        .join("dbs")
        .join("userRules.json")
}

fn rimsort_rules() -> Result<Vec<(String, String)>, String> {
    let path = rimsort_rules_file();
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(rimstack_core::community::parse(&text)?.pairs())
}

fn hidden_file() -> PathBuf {
    crate::data_dir().join("hidden.json")
}

/// The folders the game loads a mod from with the load order being edited.
fn loaded_folders(library: &Library, mod_id: usize) -> Vec<PathBuf> {
    let list = crate::session().current().to_vec();
    let arranged = library.arrange(&list);
    let active: std::collections::HashSet<String> = arranged
        .active
        .iter()
        .filter_map(|m| library.installed.get(m.installed?)?.key())
        .collect();
    let root = &library.installed[mod_id].path;
    loadfolders::folders(root, &library.game_version, &|k| active.contains(k))
}

fn snapshots_dir() -> PathBuf {
    crate::data_dir().join("snapshots")
}

fn lists_dir() -> PathBuf {
    crate::data_dir().join("lists")
}

fn ids_from(json: &QString) -> Vec<usize> {
    serde_json::from_str(&json.to_string()).unwrap_or_default()
}

fn now_secs() -> u64 {
    now()
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl qobject::Backend {
    fn load_saves(mut self: Pin<&mut Self>) {
        if self.saves_busy {
            return;
        }
        self.as_mut().set_saves_busy(true);
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let rows: Vec<Value> = match (crate::library(), saves_dir()) {
                (Ok((library, _)), Some(dir)) => {
                    let list = crate::session().current().to_vec();
                    let found = saves::list(&dir);
                    found.iter().map(|s| save_row(s, library, &list)).collect()
                }
                _ => Vec::new(),
            };
            // RimSaves' folders, when the mod has been used: those it
            // lists, and any that only show up in a file name.
            let config = crate::PATHS.config.clone().unwrap_or_default();
            let kept = rimsaves::settings(&config);
            let mut folders = kept.as_ref().map_or(Vec::new(), |s| s.folders.clone());
            for row in &rows {
                let folder = row["folder"].as_str().unwrap_or_default().to_string();
                if !folders.contains(&folder) {
                    folders.push(folder);
                }
            }
            let versions: Vec<(&str, bool)> = rows
                .iter()
                .map(|row| {
                    (
                        row["gameVersion"].as_str().unwrap_or_default(),
                        row["otherGame"] == true,
                    )
                })
                .collect();
            let text = json!({
                "saves": rows,
                "allFrom": saves::version_all_share(&versions),
                "rimsaves": kept.is_some(),
                "folders": folders,
                "currentFolder": kept.map(|s| s.current),
            })
            .to_string();
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_saves(QString::from(&text));
                backend.as_mut().set_saves_busy(false);
            });
        });
    }

    fn load_logs(self: Pin<&mut Self>, previous: bool) {
        self.read_log(previous, None);
    }

    fn load_kept_log(self: Pin<&mut Self>, path: &QString) {
        self.read_log(false, Some(PathBuf::from(path.to_string())));
    }

    fn keep_logs(&self) {
        std::thread::spawn(log_report::keep_logs);
    }

    fn read_log(mut self: Pin<&mut Self>, previous: bool, kept: Option<PathBuf>) {
        if self.logs_busy {
            return;
        }
        self.as_mut().set_logs_busy(true);
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let report = match crate::library() {
                Ok((library, _)) => logs_report(library, previous, kept),
                Err(problem) => json!({ "problem": problem }),
            };
            let text = report.to_string();
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_logs(QString::from(&text));
                backend.as_mut().set_logs_busy(false);
            });
        });
    }

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

    fn copy_to_local(&self, folder: &QString) -> QString {
        let Some(local) = crate::PATHS.game.as_ref().map(|game| game.join("Mods")) else {
            return QString::from(&say!("The game's folder is not known"));
        };
        let from = PathBuf::from(folder.to_string());
        match rimstack_core::upkeep::copy_to_local(&from, &local) {
            Ok(_) => QString::default(),
            Err(problem) => QString::from(&problem),
        }
    }

    fn set_mod_settings_aside(&self) -> QString {
        let Some(config) = crate::PATHS.config.as_ref() else {
            return QString::from(&say!("The game's settings folder is not known"));
        };
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
        let aside =
            crate::data_dir().join(format!("mod-settings-{}", now.map_or(0, |n| n.as_secs())));
        let said = match rimstack_core::upkeep::set_mod_settings_aside(config, &aside) {
            Ok(0) => say!("No mod has settings"),
            Ok(count) => say!("{} files moved to {}", count, aside.display()),
            Err(problem) => problem,
        };
        QString::from(&said)
    }

    fn settings(&self) -> QString {
        let paths = &*crate::PATHS;
        let shown = |p: &Option<PathBuf>| p.as_ref().map(|p| p.to_string_lossy().into_owned());
        let settings = crate::settings();
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
            "steamcmd": rimstack_core::redownload::steamcmd(&crate::data_dir()).is_some(),
            "interfaceScale": settings.get("interfaceScale").unwrap_or("1"),
            "font": settings.get("font"),
            "communityRules": settings.is_on("communityRules"),
            "versionWarnings": settings.get("versionWarnings") != Some("off"),
            "versionAsideFor": settings.get("versionAsideFor"),
            "fileHints": settings.get("fileHints") == Some("on"),
            "gitForWorkshop": settings.get("gitForWorkshop") == Some("on"),
            "git": *GIT,
            "selfCheck": settings.get("selfCheck") != Some("off"),
            "logsKept": log_report::logs_kept_limit(),
            "logsKeptNow": logs::kept(&log_report::kept_logs_dir()).len(),
            "replacementsIgnored": settings.get("replacementsIgnored").unwrap_or_default().split(',').filter(|id| !id.is_empty()).collect::<Vec<_>>(),
            "setupDone": settings.get("setupDone").is_some(),
            "rimsortRules": rimsort_rules_file().is_file(),
            "dataFolder": crate::data_dir().to_string_lossy(),
            "settingsFile": crate::settings_file().to_string_lossy(),
            "version": env!("CARGO_PKG_VERSION"),
        });
        QString::from(&report.to_string())
    }

    fn set_setting(&self, name: &QString, value: &QString) -> QString {
        match crate::set_setting(&name.to_string(), &value.to_string()) {
            Ok(()) => QString::default(),
            Err(problem) => QString::from(&problem),
        }
    }

    fn forget(mut self: Pin<&mut Self>) {
        self.as_mut().set_textures(QString::default());
        self.as_mut().set_saves(QString::default());
        self.as_mut().set_logs(QString::default());
    }

    fn summary(&self) -> QString {
        let config = crate::PATHS.config.clone().unwrap_or_default();
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
        let last = launches::read(&launches_file()).pop();
        let ignored = crate::replacements_ignored();
        let replacements = match crate::library() {
            Ok((library, _)) => workshop_mods(library)
                .keys()
                .filter(|id| crate::REPLACEMENTS.contains_key(**id))
                .filter(|id| !ignored.contains(**id))
                .count(),
            Err(_) => 0,
        };
        let rules = crate::USER_RULES
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .len();
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
