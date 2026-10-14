//! Getting mods and their updates: what Steam and Git say is newer, pasted
//! Workshop links, SteamCMD, and what changed since the last launch.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, task_note, cxx_name = "taskNote")]
        #[qproperty(bool, task_failed, cxx_name = "taskFailed")]
        #[qproperty(bool, task_running, cxx_name = "taskRunning")]
        #[qproperty(i32, mods_written, cxx_name = "modsWritten")]
        #[qproperty(QString, updates)]
        #[qproperty(bool, updates_busy, cxx_name = "updatesBusy")]
        #[qproperty(QString, change_notes, cxx_name = "changeNotes")]
        #[qproperty(bool, change_notes_busy, cxx_name = "changeNotesBusy")]
        #[qproperty(QString, change_notes_state, cxx_name = "changeNotesState")]
        #[qproperty(QString, queue)]
        #[qproperty(bool, queue_busy, cxx_name = "queueBusy")]
        type Downloads = super::DownloadsRust;
    }

    impl cxx_qt::Threading for Downloads {}

    extern "RustQt" {
        /// Ask Steam which installed Workshop mods have a newer version.
        /// Fills `updates` with a JSON object. Uses the network.
        #[qinvokable]
        #[cxx_name = "checkUpdates"]
        fn check_updates(self: Pin<&mut Downloads>);

        /// Fills `updates` with what the last check learnt, against the
        /// mods as they are on disk now. Does not use the network.
        #[qinvokable]
        #[cxx_name = "restoreUpdates"]
        fn restore_updates(self: Pin<&mut Downloads>);

        /// Look up the Workshop links or ids in pasted text. Fills `queue`
        /// with a JSON object. Uses the network.
        #[qinvokable]
        #[cxx_name = "lookUp"]
        fn look_up(self: Pin<&mut Downloads>, pasted: &QString);

        /// Bring a mod kept as a Git checkout up to its remote, in the
        /// background. `taskNote` then says what happened, and `taskFailed`
        /// whether nothing was changed.
        #[qinvokable]
        #[cxx_name = "updateFromGit"]
        fn update_from_git(self: Pin<&mut Downloads>, folder: &QString);

        /// Ask Steam to "subscribe" to, "unsubscribe" from or "download" these
        /// Workshop items (a JSON array of ids), in the background;
        /// `taskNote` says what happened. Steam must be running.
        #[qinvokable]
        #[cxx_name = "askSteam"]
        fn ask_steam(self: Pin<&mut Downloads>, what: &QString, ids: &QString);

        /// Clone a repository (`https://host/owner/repo`) into the game's
        /// own Mods folder, in the background; `taskNote` says what happened.
        #[qinvokable]
        #[cxx_name = "cloneMod"]
        fn clone_mod(self: Pin<&mut Downloads>, link: &QString);

        /// Download Valve's SteamCMD into RimStack's data folder, in the
        /// background; `taskNote` says what happened.
        #[qinvokable]
        #[cxx_name = "installSteamcmd"]
        fn install_steamcmd(self: Pin<&mut Downloads>);

        /// Download these Workshop items (a JSON array of ids) with
        /// SteamCMD into the game's own `Mods` folder, in the background;
        /// `taskNote` says what happened.
        #[qinvokable]
        #[cxx_name = "downloadWithSteamcmd"]
        fn download_with_steamcmd(self: Pin<&mut Downloads>, ids: &QString);

        /// The Workshop id of a mod known by this name or package id from
        /// the Steam Workshop Database; empty when it is not known.
        #[qinvokable]
        #[cxx_name = "workshopIdOf"]
        fn workshop_id_of(self: &Downloads, name: &QString) -> QString;

        /// Download a Workshop mod again with SteamCMD and put it in the
        /// place of the copy in `folder`, in the background. `taskNote`
        /// says what happened, as for `updateFromGit`.
        #[qinvokable]
        fn redownload(self: Pin<&mut Downloads>, workshop_id: &QString, folder: &QString);

        /// The installed Workshop mods that have a maintained replacement
        /// (the Use This Instead list), active ones first, as a JSON array.
        #[qinvokable]
        fn replacements(self: &Downloads) -> QString;

        #[qinvokable]
        #[cxx_name = "sinceLaunch"]
        fn since_launch(self: &Downloads) -> QString;

        /// The newest commits of the Git checkout in this folder, as a
        /// JSON array of `{ time, text }`.
        #[qinvokable]
        #[cxx_name = "gitNotes"]
        fn git_notes(self: &Downloads, folder: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "fetchChangeNotes"]
        fn fetch_change_notes(self: Pin<&mut Downloads>, asked: &QString);
    }
}

use crate::workspace::now_secs;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use report::{check, checkout_updated, last_check, queue_report, steam_record, updates_report};
use rimstack_core::base::error::Error;
use rimstack_core::base::locks::Shared;
use rimstack_core::{keep, kept::launches, rimworld::workshop, say};
use std::path::PathBuf;

#[path = "downloads/report.rs"]
pub(crate) mod report;
#[path = "downloads/since_launch.rs"]
pub(crate) mod since_launch;
#[path = "downloads/steam.rs"]
mod steam;

#[derive(Default)]
pub struct DownloadsRust {
    /// What the task under way is doing, then how it ended; `task_failed`
    /// says whether that changed nothing.
    task_note: QString,
    task_failed: bool,
    task_running: bool,
    /// Goes up when a task here changed a mod's folder on disk.
    mods_written: i32,
    updates: QString,
    updates_busy: bool,
    change_notes: QString,
    change_notes_busy: bool,
    /// `{ done, asked, resume, refused }`: how far the asking is, when it goes on after Steam refused, whether it stopped for that.
    change_notes_state: QString,
    queue: QString,
    queue_busy: bool,
    /// The tasks asked for while one runs, each with what it says it is doing.
    waiting: std::collections::VecDeque<(String, Work)>,
}

type Work = Box<dyn FnOnce() -> Ended + Send>;

/// How a task ended, as worked out off the window's thread.
struct Ended {
    /// What it did, or why it changed nothing.
    note: Result<String, Error>,
    /// It changed a mod's folder on disk.
    wrote: bool,
    /// The report `updates` then holds, where the task changed it.
    updates: Option<String>,
}

impl Ended {
    fn told(note: Result<String, Error>) -> Self {
        Self {
            note,
            wrote: false,
            updates: None,
        }
    }

    /// For a task that changes a mod's folder unless it fails.
    fn of_a_write(note: Result<String, Error>) -> Self {
        Self {
            wrote: note.is_ok(),
            ..Self::told(note)
        }
    }
}

impl qobject::Downloads {
    /// Runs `work` in the background, then shows how it ended. One at a
    /// time, in the order asked: two would write the same SteamCMD
    /// folder, and `taskRunning` could say of only one.
    fn task(mut self: Pin<&mut Self>, doing: &str, work: impl FnOnce() -> Ended + Send + 'static) {
        let work: Work = Box::new(work);
        if self.task_running {
            let waiting = &mut self.as_mut().rust_mut().waiting;
            waiting.push_back((doing.to_string(), work));
        } else {
            self.start(doing, work);
        }
    }

    fn start(mut self: Pin<&mut Self>, doing: &str, work: Work) {
        self.as_mut().begin(doing);
        crate::background::run(self.qt_thread(), work, |mut downloads, ended| {
            if let Some(report) = ended.updates {
                downloads.as_mut().set_updates(QString::from(&report));
            }
            if ended.wrote {
                let written = *downloads.mods_written() + 1;
                downloads.as_mut().set_mods_written(written);
            }
            downloads.as_mut().finish(ended.note);
            if let Some((doing, work)) = downloads.as_mut().rust_mut().waiting.pop_front() {
                downloads.start(&doing, work);
            }
        });
    }

    fn begin(mut self: Pin<&mut Self>, doing: &str) {
        self.as_mut().set_task_failed(false);
        self.as_mut().set_task_running(true);
        self.as_mut().set_task_note(QString::from(doing));
    }

    /// How a task ended: what it did, or why it changed nothing.
    /// `taskNote` is set last: what listens to it reads the other two.
    fn finish(mut self: Pin<&mut Self>, outcome: Result<String, Error>) {
        self.as_mut().set_task_failed(outcome.is_err());
        self.as_mut().set_task_running(false);
        let note = outcome.unwrap_or_else(String::from);
        self.as_mut().set_task_note(QString::from(&note));
    }
}
