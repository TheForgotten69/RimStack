//! Load orders kept beside the one being edited: saved lists, snapshots
//! taken before a change, and the history of what RimStack changed.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type KeptLists = super::KeptListsRust;
    }

    impl cxx_qt::Threading for KeptLists {}

    extern "RustQt" {
        /// The package ids in a list someone pasted, as a JSON array.
        #[qinvokable]
        #[cxx_name = "idsInText"]
        fn ids_in_text(self: &KeptLists, text: &QString) -> QString;

        /// Keep the load order as it is now under `name`, among the
        /// snapshots; the ten newest are kept. Taken before anything that
        /// rearranges the whole list.
        #[qinvokable]
        fn snapshot(self: &KeptLists, name: &QString);

        #[qinvokable]
        #[cxx_name = "snapshotSaved"]
        fn snapshot_saved(self: &KeptLists, name: &QString) -> QString;

        #[qinvokable]
        fn snapshots(self: &KeptLists) -> QString;

        /// The package ids of a snapshot, as a JSON array; empty text if
        /// there is no such snapshot.
        #[qinvokable]
        #[cxx_name = "snapshotList"]
        fn snapshot_list(self: &KeptLists, name: &QString) -> QString;

        /// Keep a list of package ids (a JSON array) under a name, among
        /// the saved lists. Returns what went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "saveIdsAs"]
        fn save_ids_as(self: &KeptLists, name: &QString, ids: &QString) -> QString;

        /// The load order as it was before RimStack first saved (kept as
        /// ModsConfig.xml.rimstack-original), as a JSON array of package
        /// ids; empty text if RimStack has never saved.
        #[qinvokable]
        #[cxx_name = "originalList"]
        fn original_list(self: &KeptLists) -> QString;

        #[qinvokable]
        #[cxx_name = "savedLists"]
        fn saved_lists(self: &KeptLists) -> QString;

        /// Keep the load order being edited under a name. Returns what
        /// went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "saveListAs"]
        fn save_list_as(self: &KeptLists, name: &QString) -> QString;

        /// The package ids kept under a name, as a JSON array; empty text
        /// if there is no such list.
        #[qinvokable]
        #[cxx_name = "savedList"]
        fn saved_list(self: &KeptLists, name: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "deleteList"]
        fn delete_list(self: &KeptLists, name: &QString);

        /// Adds a line to the history of what RimStack changed; `revert`
        /// is "list <snapshot>", "unhide <mod folder>" or nothing.
        #[qinvokable]
        #[cxx_name = "noteChange"]
        fn note_change(self: &KeptLists, what: &QString, revert: &QString);

        #[qinvokable]
        fn journal(self: &KeptLists) -> QString;
    }
}

use crate::workspace::now_secs;
use cxx_qt_lib::QString;
use rimstack_core::{kept::journal, kept::lists, rimworld::modsconfig, say};
use std::path::PathBuf;

fn journal_file(app: &crate::App) -> PathBuf {
    app.data_dir().join("journal.json")
}

#[derive(Default)]
pub struct KeptListsRust {}

impl qobject::KeptLists {
    fn ids_in_text(&self, text: &QString) -> QString {
        let ids = rimstack_core::mods::share::ids_in(&text.to_string());
        crate::reports::said(&ids)
    }

    fn snapshot(&self, name: &QString) {
        let dir = crate::app().snapshots_dir();
        let list = crate::app().session().current().to_vec();
        match lists::save(&dir, &name.to_string(), &list) {
            Ok(()) => lists::prune(&dir, 30),
            Err(problem) => rimstack_core::problem!("{problem}"),
        }
    }

    /// Keeps the list as it is on disk among the snapshots; returns the
    /// name it was kept under, or nothing if it could not be.
    fn snapshot_saved(&self, name: &QString) -> QString {
        let dir = crate::app().snapshots_dir();
        let list = crate::app().session().saved().to_vec();
        match lists::save(&dir, &name.to_string(), &list) {
            Ok(()) => {
                lists::prune(&dir, 30);
                name.clone()
            }
            Err(problem) => {
                rimstack_core::problem!("{problem}");
                QString::default()
            }
        }
    }

    fn snapshots(&self) -> QString {
        let mut found = lists::all(&crate::app().snapshots_dir());
        found.sort_by_key(|l| std::cmp::Reverse(l.modified));
        crate::reports::said(&found)
    }

    fn snapshot_list(&self, name: &QString) -> QString {
        match lists::load(&crate::app().snapshots_dir(), &name.to_string()) {
            Ok(ids) => crate::reports::said(&ids),
            Err(_) => QString::default(),
        }
    }

    fn save_ids_as(&self, name: &QString, ids: &QString) -> QString {
        let Ok(ids) = serde_json::from_str::<Vec<String>>(&ids.to_string()) else {
            return QString::from(&say!("That is not a list of mods."));
        };
        match lists::save(&crate::app().lists_dir(), &name.to_string(), &ids) {
            Ok(()) => QString::default(),
            Err(problem) => QString::from(&problem.to_string()),
        }
    }

    fn original_list(&self) -> QString {
        let original = crate::app()
            .paths()
            .mods_config()
            .map(|c| c.with_file_name("ModsConfig.xml.rimstack-original"));
        match original.and_then(|path| modsconfig::read(&path).ok()) {
            Some(config) => crate::reports::said(&config.active),
            None => QString::default(),
        }
    }

    fn saved_lists(&self) -> QString {
        let found = lists::all(&crate::app().lists_dir());
        crate::reports::said(&found)
    }

    fn save_list_as(&self, name: &QString) -> QString {
        let list = crate::app().session().current().to_vec();
        match lists::save(&crate::app().lists_dir(), &name.to_string(), &list) {
            Ok(()) => QString::default(),
            Err(problem) => QString::from(&problem.to_string()),
        }
    }

    fn saved_list(&self, name: &QString) -> QString {
        match lists::load(&crate::app().lists_dir(), &name.to_string()) {
            Ok(ids) => crate::reports::said(&ids),
            Err(_) => QString::default(),
        }
    }

    fn delete_list(&self, name: &QString) {
        if let Err(problem) = lists::delete(&crate::app().lists_dir(), &name.to_string()) {
            rimstack_core::problem!("{problem}");
        }
    }

    fn note_change(&self, what: &QString, revert: &QString) {
        let entry = journal::Entry {
            time: now_secs(),
            what: what.to_string(),
            revert: revert.to_string(),
        };
        if let Err(problem) = journal::add(&journal_file(crate::app()), entry) {
            rimstack_core::problem!("{problem}");
        }
    }

    /// The journal, newest first, as JSON: `{ time, what, revert }`, the
    /// last empty where the entry can no longer be taken back.
    fn journal(&self) -> QString {
        let kept: Vec<String> = lists::all(&crate::app().snapshots_dir())
            .into_iter()
            .map(|l| l.name)
            .collect();
        let still = |revert: &str| match revert.strip_prefix("list ") {
            Some(name) => kept.iter().any(|k| k == name),
            None => true,
        };
        let kept = journal::read(&journal_file(crate::app()));
        let rows: Vec<crate::reports::JournalRow> = kept
            .iter()
            .rev()
            .map(|e| crate::reports::JournalRow {
                time: e.time,
                what: &e.what,
                revert: if still(&e.revert) { &e.revert } else { "" },
            })
            .collect();
        crate::reports::said(&rows)
    }
}
