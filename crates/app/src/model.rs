//! The load-order list exposed to QML as a list model.

use rimstack_core::locks::{Held, Shared};
use rimstack_core::{
    conflicts, demo, infer,
    library::Library,
    list::{Category, Change, GroupedList, Mod, Row},
    loadfolders, modsconfig,
    rules::{Kind, Rule},
    sort,
};
use rimstack_core::{say, say_n};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++Qt" {
        include!(<QtCore/QAbstractListModel>);
        #[qobject]
        type QAbstractListModel;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qvector.h");
        type QVector_i32 = cxx_qt_lib::QVector<i32>;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qmap.h");
        type QMap_QString_QVariant = cxx_qt_lib::QMap<cxx_qt_lib::QMapPair_QString_QVariant>;
    }

    extern "RustQt" {
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        #[qproperty(i32, demo_count, cxx_name = "demoCount")]
        #[qproperty(QString, status)]
        #[qproperty(QString, problem)]
        #[qproperty(i32, count)]
        /// How many of them the filter lets through.
        #[qproperty(i32, matching)]
        #[qproperty(bool, previewing)]
        #[qproperty(QString, sort_note, cxx_name = "sortNote")]
        #[qproperty(bool, dirty)]
        #[qproperty(QString, unsaved)]
        #[qproperty(bool, can_undo, cxx_name = "canUndo")]
        #[qproperty(bool, can_redo, cxx_name = "canRedo")]
        #[qproperty(i32, errors)]
        #[qproperty(i32, conflicts)]
        #[qproperty(i32, warnings)]
        #[qproperty(i32, conflicts_version, cxx_name = "conflictsVersion")]
        #[qproperty(bool, files_read, cxx_name = "filesRead")]
        #[qproperty(bool, by_name, cxx_name = "byName")]
        #[qproperty(i32, headings)]
        #[qproperty(i32, library_version, cxx_name = "libraryVersion")]
        #[qproperty(bool, rescanning)]
        type ModListModel = super::ModListModelRust;
    }

    impl cxx_qt::Threading for ModListModel {}

    extern "RustQt" {
        #[qinvokable]
        #[cxx_override]
        fn data(self: &ModListModel, index: &QModelIndex, role: i32) -> QVariant;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &ModListModel, parent: &QModelIndex) -> i32;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &ModListModel) -> QHash_i32_QByteArray;

        #[qinvokable]
        fn reload(self: Pin<&mut ModListModel>);

        /// Fill the model with the installed mods: the active list in load
        /// order, or the rest. Returns at once; the rows arrive when the
        /// scan is done.
        #[qinvokable]
        #[cxx_name = "loadInstalled"]
        fn load_installed(self: Pin<&mut ModListModel>, active: bool);

        /// What the details panel shows for an installed mod, by its
        /// `modId` role. Empty for an unknown id.
        #[qinvokable]
        fn details(self: &ModListModel, mod_id: i32) -> QString;

        /// The folders of an installed mod with how many files each
        /// holds, and its assemblies: `{ tree, assemblies }`. Walks the
        /// mod's folder.
        #[qinvokable]
        fn files(self: &ModListModel, mod_id: i32) -> QString;

        /// The name an installed mod is shown by; empty for an unknown id.
        #[qinvokable]
        #[cxx_name = "nameOf"]
        fn name_of(self: &ModListModel, mod_id: i32) -> QString;

        /// Load-order position of an installed mod in this list, or -1.
        #[qinvokable]
        #[cxx_name = "indexOf"]
        fn index_of(self: &ModListModel, mod_id: i32) -> i32;

        /// The installed mods whose name holds this text, closest first,
        /// as a JSON array of { name, modId, active }; at most `most`.
        #[qinvokable]
        #[cxx_name = "findMods"]
        fn find_mods(self: &ModListModel, text: &QString, most: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "subsBeside"]
        fn subs_beside(self: &ModListModel, mod_id: i32) -> QString;

        /// Show the list ordered by a column ("name", "author", "version",
        /// "load"), or by anything else in load order again. Only the
        /// view changes.
        #[qinvokable]
        #[cxx_name = "showBy"]
        fn show_by(self: Pin<&mut ModListModel>, column: &QString, descending: bool);

        /// How many mods Sort would move now.
        #[qinvokable]
        #[cxx_name = "sortPending"]
        fn sort_pending(self: &ModListModel) -> i32;

        #[qinvokable]
        fn listing(self: &ModListModel) -> QString;

        /// The row showing the mod at a load-order position, or -1 when it
        /// is filtered out or under a collapsed heading.
        #[qinvokable]
        #[cxx_name = "rowOf"]
        fn row_of(self: &ModListModel, position: i32) -> i32;

        /// Facts about the whole install, once the lists have arrived.
        #[qinvokable]
        fn overview(self: &ModListModel) -> QString;

        /// Work out the order the mods' own rules ask for and show it as a
        /// preview: `previewing` turns true if anything moves, and
        /// `sortNote` says what happened either way.
        #[qinvokable]
        #[cxx_name = "previewSort"]
        fn preview_sort(self: Pin<&mut ModListModel>);

        #[qinvokable]
        #[cxx_name = "applySort"]
        fn apply_sort(self: Pin<&mut ModListModel>);

        #[qinvokable]
        #[cxx_name = "discardSort"]
        fn discard_sort(self: Pin<&mut ModListModel>);

        /// Read the mod folders again, in the background, and show the
        /// result. `libraryVersion` goes up when it is done; the other
        /// list must then be loaded again, and library ids from before
        /// mean nothing any more.
        #[qinvokable]
        fn rescan(self: Pin<&mut ModListModel>);

        /// The folders were changed in Settings: find them again, read
        /// the mods there in the background, and start from the load
        /// order on disk there. Ends like `rescan`.
        #[qinvokable]
        #[cxx_name = "rereadFolders"]
        fn reread_folders(self: Pin<&mut ModListModel>);

        /// Work out the problems and texture conflicts again: something
        /// outside the list changed what the game will load.
        #[qinvokable]
        fn recheck(self: Pin<&mut ModListModel>);

        /// The tags of the mod with this library id were changed (see
        /// `ModEdits`): its row and the groups read them again.
        #[qinvokable]
        #[cxx_name = "tagsChanged"]
        fn tags_changed(self: Pin<&mut ModListModel>, mod_id: i32);

        /// The community databases were downloaded again: the problems
        /// and the categories that follow from them are worked out anew.
        #[qinvokable]
        #[cxx_name = "communityChanged"]
        fn community_changed(self: Pin<&mut ModListModel>);

        /// Step the load order back or forward. Call `sync` on the other
        /// list afterwards.
        #[qinvokable]
        #[cxx_name = "moveToEdge"]
        fn move_to_edge(self: Pin<&mut ModListModel>, from: i32, top: bool);
        #[qinvokable]
        #[cxx_name = "discardChanges"]
        fn discard_changes(self: Pin<&mut ModListModel>);
        #[qinvokable]
        fn undo(self: Pin<&mut ModListModel>);
        #[qinvokable]
        fn redo(self: Pin<&mut ModListModel>);

        /// Make these package ids (a JSON array of strings) the load
        /// order. Call `sync` on the other list afterwards.
        #[qinvokable]
        #[cxx_name = "useList"]
        fn use_list(self: Pin<&mut ModListModel>, ids: &QString);

        #[qinvokable]
        fn sync(self: Pin<&mut ModListModel>);

        /// Write the load order to the game's `ModsConfig.xml`. Returns
        /// what went wrong, or nothing.
        #[qinvokable]
        fn save(self: Pin<&mut ModListModel>) -> QString;

        /// The next row after `row` (or, going back, before it) whose mod
        /// has a problem, wrapping round; -1 when there is none.
        #[qinvokable]
        #[cxx_name = "nextProblem"]
        fn next_problem(self: &ModListModel, row: i32, forward: bool) -> i32;

        /// Where the mod on `row` stands among the mods with a problem,
        /// in load order and counted from 1; 0 when it has none. A folded
        /// group's mods count though no row shows them.
        #[qinvokable]
        #[cxx_name = "problemPlace"]
        fn problem_place(self: &ModListModel, row: i32) -> i32;
        #[qinvokable]
        #[cxx_name = "problemCount"]
        fn problem_count(self: &ModListModel) -> i32;

        /// Arrange the list as `byName` (the inactive mods) or `headings`
        /// (the load order) now say.
        #[qinvokable]
        fn reshape(self: Pin<&mut ModListModel>);

        /// The mods the preview moves, by the kind of reason, the largest
        /// kind first: a JSON array of `{ name, moves }`, each move
        /// `{ name, from, to, modId, why }`.
        #[qinvokable]
        #[cxx_name = "sortMoves"]
        fn sort_moves(self: &ModListModel) -> QString;

        /// What changed since the save, as a sentence kept for the journal.
        #[qinvokable]
        #[cxx_name = "unsavedKept"]
        fn unsaved_kept(self: &ModListModel) -> QString;

        #[qinvokable]
        #[cxx_name = "firstMoved"]
        fn first_moved(self: &ModListModel) -> i32;

        #[qinvokable]
        #[cxx_name = "toggleCategory"]
        fn toggle_category(self: Pin<&mut ModListModel>, category: i32);

        /// Fold the group whose heading is on this row away, or show it
        /// again.
        #[qinvokable]
        #[cxx_name = "toggleSub"]
        fn toggle_sub(self: Pin<&mut ModListModel>, row: i32);

        /// Take the tag whose heading is on this row off every mod under
        /// it: the group is gone, the mods stay. Call `sync` on both
        /// models afterwards.
        #[qinvokable]
        #[cxx_name = "removeSub"]
        fn remove_sub(self: Pin<&mut ModListModel>, row: i32);

        #[qinvokable]
        #[cxx_name = "setFilter"]
        fn set_filter(self: Pin<&mut ModListModel>, text: &QString);

        #[qinvokable]
        #[cxx_name = "moveMod"]
        fn move_mod(self: Pin<&mut ModListModel>, from: i32, to: i32);

        /// # Safety
        /// `other` must be null or a live `ModListModel` other than `self`.
        #[qinvokable]
        #[cxx_name = "takeFrom"]
        unsafe fn take_from(self: Pin<&mut ModListModel>, other: *mut ModListModel, index: i32);

        /// # Safety
        /// `other` must be null or a live `ModListModel` other than `self`.
        #[qinvokable]
        #[cxx_name = "takeFromAt"]
        unsafe fn take_from_at(
            self: Pin<&mut ModListModel>,
            other: *mut ModListModel,
            index: i32,
            to: i32,
        );

        /// Take the mods with these library ids (a JSON array) from the
        /// other list, in the order they have there, as one step for Undo.
        ///
        /// # Safety
        /// `other` must be null or a live `ModListModel` other than `self`.
        #[qinvokable]
        #[cxx_name = "takeAllFrom"]
        unsafe fn take_all_from(
            self: Pin<&mut ModListModel>,
            other: *mut ModListModel,
            ids: &QString,
        );

        /// Each of these mods (library ids, a JSON array) to the top or
        /// bottom of its category's stretch, keeping their order among
        /// themselves, as one step for Undo.
        #[qinvokable]
        #[cxx_name = "moveAllToEdge"]
        fn move_all_to_edge(self: Pin<&mut ModListModel>, ids: &QString, top: bool);

        #[qinvokable]
        #[cxx_name = "markReady"]
        fn mark_ready(self: &ModListModel);

        /// The library id of the mod on this row; -1 for a heading or a
        /// mod that is not installed.
        #[qinvokable]
        #[cxx_name = "modIdAt"]
        fn mod_id_at(self: &ModListModel, row: i32) -> i32;

        /// The row of the nearest installed mod from `from`, going down
        /// (`by` 1) or up (-1), headings skipped; -1 at the end. `from`
        /// may be -1 or `rows()` to start at an end.
        #[qinvokable]
        #[cxx_name = "stepRow"]
        fn step_row(self: &ModListModel, from: i32, by: i32) -> i32;

        /// The library ids of the installed mods on the rows from one to
        /// the other, as a JSON array.
        #[qinvokable]
        #[cxx_name = "idsBetween"]
        fn ids_between(self: &ModListModel, a: i32, b: i32) -> QString;

        /// The library id of the mod the selection goes to when the one
        /// on this row leaves the list; -1 for none.
        #[qinvokable]
        #[cxx_name = "idBeside"]
        fn id_beside(self: &ModListModel, row: i32) -> i32;

        /// Number of rows on screen; for the self-test.
        #[qinvokable]
        fn rows(self: &ModListModel) -> i32;

        #[qinvokable]
        #[cxx_name = "packageMod"]
        fn package_mod(self: &ModListModel, key: &QString) -> i32;

        #[qinvokable]
        #[cxx_name = "freshIds"]
        fn fresh_ids(self: &ModListModel) -> QString;
    }

    unsafe extern "RustQt" {
        #[inherit]
        #[qsignal]
        #[cxx_name = "dataChanged"]
        fn data_changed(
            self: Pin<&mut ModListModel>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
            roles: &QVector_i32,
        );

        #[inherit]
        fn index(self: &ModListModel, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;
    }

    extern "RustQt" {
        #[inherit]
        #[cxx_name = "beginInsertRows"]
        unsafe fn begin_insert_rows(
            self: Pin<&mut ModListModel>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        #[cxx_name = "endInsertRows"]
        unsafe fn end_insert_rows(self: Pin<&mut ModListModel>);
        #[inherit]
        #[cxx_name = "beginRemoveRows"]
        unsafe fn begin_remove_rows(
            self: Pin<&mut ModListModel>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        #[cxx_name = "endRemoveRows"]
        unsafe fn end_remove_rows(self: Pin<&mut ModListModel>);
        #[inherit]
        #[cxx_name = "beginResetModel"]
        unsafe fn begin_reset_model(self: Pin<&mut ModListModel>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        unsafe fn end_reset_model(self: Pin<&mut ModListModel>);
    }
}

use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant, QVector,
};
use rayon::prelude::*;

mod details;
mod fields;
pub use fields::ModListModelRust;
use fields::Preview;
mod editing;
mod reset;
mod rows;
mod shared_textures;
mod sorting;
mod sources;
mod trust;

use rimstack_core::share;
use rimstack_core::tree::folder_tree;
use sources::*;
use trust::troubles;

use rimstack_core::ordering::{self, Sources};
use rimstack_core::words::{self as words, Trouble};

fn text(s: &str) -> QVariant {
    QVariant::from(&QString::from(s))
}

/// The installed mods split by the load order being edited, each with
/// your tags.
fn arranged(library: &Library) -> rimstack_core::library::Arranged {
    let mut now = library.arrange(crate::app().session().current());
    for m in now.active.iter_mut() {
        let took = crate::app()
            .startup()
            .of(&rimstack_core::id::split_steam(&m.id).0);
        m.load_micros = took.map(|ms| (ms * 1000.0) as u32);
    }
    let notes = crate::app().notes().reading();
    if !notes.is_empty() {
        for m in now.active.iter_mut().chain(now.inactive.iter_mut()) {
            let key = m.installed.and_then(|i| library.installed[i].key());
            if let Some(note) = key.and_then(|k| notes.get(&k)) {
                m.tags = note.tags.clone();
                m.sub = note.group().to_string();
                // Filed by hand: that stands over what RimStack worked out.
                if let Some(chosen) = Category::from_kept(&note.category) {
                    m.category = chosen;
                }
            }
        }
    }
    now
}

/// One of the two lists as first shown, with the line under it and why
/// the mods could not be read, if they could not.
fn listed(loaded: &crate::Loaded, active: bool, headings: bool) -> (GroupedList, String, String) {
    let (library, scan) = match loaded {
        Ok(read) => read,
        Err(problem) => {
            return (
                GroupedList::new(Vec::new()),
                String::new(),
                problem.to_string(),
            )
        }
    };
    if !active {
        let status = say!(
            "{} inactive, read in {} ms",
            library.inactive.len(),
            scan.as_millis()
        );
        return (
            GroupedList::new(arranged(library).inactive),
            status,
            String::new(),
        );
    }
    let (mods, _) = sorted_by_category(library, arranged(library).active);
    let mut list = GroupedList::flat(mods);
    list.set_grouped(headings);
    let status = match library.missing {
        0 => say!("{} active", library.active.len()),
        n => say!("{} active, {} not installed", library.active.len(), n),
    };
    (list, status, String::new())
}

impl qobject::ModListModel {
    fn reload(mut self: Pin<&mut Self>) {
        let count = self.demo_count.max(0) as usize;
        self.as_mut()
            .reset_with(|list| *list = GroupedList::new(demo::generate(count)));
    }

    fn load_installed(self: Pin<&mut Self>, active: bool) {
        let headings = self.wants_headings();
        // Waits there, off the interface thread, until the scan is done.
        let read = move || listed(&crate::app().library(), active, headings);
        crate::background::run(self.qt_thread(), read, move |mut model, listed| {
            let (list, status, problem) = listed;
            model.as_mut().rust_mut().checked = active && problem.is_empty();
            model.as_mut().rust_mut().installed = problem.is_empty();
            model.as_mut().reset_with(|l| *l = list);
            model.as_mut().reshape();
            model.as_mut().set_problem(QString::from(&problem));
            model.as_mut().set_status(QString::from(&status));
            if let (true, Some(started)) = (active, crate::STARTED.get()) {
                rimstack_core::note!("mods_shown_ms={}", started.elapsed().as_millis());
            }
        });
    }

    fn next_problem(&self, row: i32, forward: bool) -> i32 {
        let rows = self.list.rows();
        let troubled = |r: &usize| matches!(rows[*r], Row::Mod { index } if self.troubles.get(index).is_some_and(|t| !t.is_empty()));
        let count = rows.len() as i32;
        if count == 0 {
            return -1;
        }
        let step = if forward { 1 } else { -1 };
        let from = if row < 0 && !forward { 0 } else { row };
        let mut others = (1..=count).map(|n| (from + step * n).rem_euclid(count) as usize);
        others.find(troubled).map_or(-1, |r| r as i32)
    }

    fn troubled(&self) -> Vec<bool> {
        let mods = 0..self.list.mods().len();
        mods.map(|i| self.troubles.get(i).is_some_and(|t| !t.is_empty()))
            .collect()
    }

    fn problem_place(&self, row: i32) -> i32 {
        let rows = self.list.rows();
        match usize::try_from(row).ok().and_then(|r| rows.get(r)) {
            Some(Row::Mod { index }) => {
                rimstack_core::checks::place_among(&self.troubled(), *index).0 as i32
            }
            _ => 0,
        }
    }

    fn problem_count(&self) -> i32 {
        self.troubled().iter().filter(|t| **t).count() as i32
    }

    fn index_of(&self, mod_id: i32) -> i32 {
        let Ok(id) = usize::try_from(mod_id) else {
            return -1;
        };
        let found = self
            .list
            .mods()
            .iter()
            .position(|m| m.installed == Some(id));
        found.map_or(-1, |i| i as i32)
    }

    fn listing(&self) -> QString {
        let loaded = crate::app().library();
        let Ok((library, _)) = &*loaded else {
            return QString::default();
        };
        let installed = |m: &Mod| library.installed.get(m.installed?);
        let entries: Vec<share::Entry> = self
            .list
            .mods()
            .iter()
            .filter_map(installed)
            .map(share::Entry::of)
            .collect();
        QString::from(&share::listing(&library.game_version, &entries))
    }

    fn find_mods(&self, text: &QString, most: i32) -> QString {
        let loaded = crate::app().library();
        let Ok((library, _)) = &*loaded else {
            return QString::from("[]");
        };
        let active: std::collections::HashSet<usize> = library
            .arrange(crate::app().session().current())
            .active
            .iter()
            .filter_map(|m| m.installed)
            .collect();
        let found: Vec<serde_json::Value> = library
            .find(&text.to_string(), usize::try_from(most).unwrap_or(0))
            .into_iter()
            .map(|i| {
                serde_json::json!({
                    "name": library.installed[i].display_name(),
                    "modId": i,
                    "active": active.contains(&i),
                })
            })
            .collect();
        QString::from(&serde_json::Value::Array(found).to_string())
    }

    fn mod_id_at(&self, row: i32) -> i32 {
        let on_row = usize::try_from(row)
            .ok()
            .and_then(|row| self.list.installed_on(row));
        on_row.map_or(-1, |id| id as i32)
    }

    fn step_row(&self, from: i32, by: i32) -> i32 {
        let found = self.list.step(from as isize, by.signum() as isize);
        found.map_or(-1, |row| row as i32)
    }

    fn ids_between(&self, a: i32, b: i32) -> QString {
        let ids = match (usize::try_from(a), usize::try_from(b)) {
            (Ok(a), Ok(b)) => self.list.installed_between(a, b),
            _ => Vec::new(),
        };
        QString::from(&serde_json::json!(ids).to_string())
    }

    fn id_beside(&self, row: i32) -> i32 {
        let beside = usize::try_from(row)
            .ok()
            .and_then(|row| self.list.beside(row));
        beside.map_or(-1, |id| id as i32)
    }

    fn row_of(&self, position: i32) -> i32 {
        let rows = self.list.rows();
        let found = rows
            .iter()
            .position(|r| matches!(r, Row::Mod { index } if *index as i32 == position));
        found.map_or(-1, |r| r as i32)
    }

    fn rescan(self: Pin<&mut Self>) {
        self.read_again(|| crate::app().rescan());
    }

    fn reread_folders(self: Pin<&mut Self>) {
        self.read_again(|| crate::app().reread_folders());
    }

    fn read_again(mut self: Pin<&mut Self>, read: fn()) {
        if self.rescanning || !self.installed {
            return;
        }
        self.as_mut().set_rescanning(true);
        let active = self.checked;
        crate::background::run(self.qt_thread(), read, move |mut model, ()| {
            if model.preview.is_some() {
                model.as_mut().end_preview();
            }
            // Positions in the old reading mean nothing in the new one.
            model.as_mut().rust_mut().checked_order = Vec::new();
            model.as_mut().load_installed(active);
            let next = model.library_version + 1;
            model.as_mut().set_library_version(next);
            model.as_mut().set_rescanning(false);
        });
    }

    fn rows(&self) -> i32 {
        self.list.rows().len() as i32
    }

    fn mark_ready(&self) {
        if let Some(started) = crate::STARTED.get() {
            rimstack_core::note!("startup_ms={}", started.elapsed().as_millis());
        }
    }
}
