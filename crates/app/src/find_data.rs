#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("rimstack-app/src/mod_rows.cxxqt.h");
        type QAbstractListModel = crate::mod_rows::qobject::QAbstractListModel;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qvector.h");
        type QVector_i32 = cxx_qt_lib::QVector<i32>;
    }

    extern "RustQt" {
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(QString, text)]
        #[qproperty(bool, running)]
        #[qproperty(i32, done)]
        #[qproperty(i32, total)]
        #[qproperty(QString, progress)]
        #[qproperty(QString, summary)]
        #[qproperty(QString, problem)]
        #[qproperty(bool, capped)]
        #[qproperty(i32, count)]
        /// -1 for none.
        #[qproperty(i32, selected_mod, cxx_name = "selectedMod")]
        /// -1 for none.
        #[qproperty(i32, current)]
        #[qproperty(QVariant, recent)]
        #[qproperty(QVariant, preview)]
        #[qproperty(QVariant, options)]
        #[qproperty(QVariant, mods)]
        #[qproperty(QString, file_types, cxx_name = "fileTypes")]
        #[qproperty(QVariant, type_suggestions, cxx_name = "typeSuggestions")]
        #[qproperty(QString, inside_text, cxx_name = "insideText")]
        #[qproperty(QString, inside_said, cxx_name = "insideSaid")]
        #[qproperty(i32, inside_found, cxx_name = "insideFound")]
        #[qproperty(bool, wrap)]
        type FindData = super::FindDataRust;
    }

    impl cxx_qt::Threading for FindData {}

    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[cxx_name = "make_unique"]
        #[namespace = "rust::cxxqtlib1"]
        fn make_find_data() -> UniquePtr<FindData>;
    }

    extern "RustQt" {
        #[qinvokable]
        #[cxx_override]
        fn data(self: &FindData, index: &QModelIndex, role: i32) -> QVariant;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &FindData, parent: &QModelIndex) -> i32;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &FindData) -> QHash_i32_QByteArray;
    }

    extern "RustQt" {
        #[qinvokable]
        fn search(self: Pin<&mut FindData>);

        #[qinvokable]
        fn stop(self: &FindData);

        #[qinvokable]
        fn select(self: Pin<&mut FindData>, row: i32);

        /// `by` is 1 for the next match, -1 for the one before.
        #[qinvokable]
        #[cxx_name = "insideStep"]
        fn inside_step(self: Pin<&mut FindData>, by: i32);

        /// -1 for every mod.
        #[qinvokable]
        #[cxx_name = "pickMod"]
        fn pick_mod(self: Pin<&mut FindData>, group: i32);

        /// A mod's row and a gap are passed over; `row` itself when there is none.
        #[qinvokable]
        fn step(self: &FindData, row: i32, by: i32) -> i32;

        /// Folds the file of `row` (`how` -1), unfolds it (1) or flips it (0); gives the row to choose then.
        #[qinvokable]
        fn fold(self: Pin<&mut FindData>, row: i32, how: i32) -> i32;

        /// Empty for a mod's row.
        #[qinvokable]
        #[cxx_name = "openFile"]
        fn open_file(self: &FindData, row: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "openFolder"]
        fn open_folder(self: &FindData, row: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "copyPath"]
        fn copy_path(self: &FindData, row: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "copyLine"]
        fn copy_line(self: &FindData, row: i32) -> QString;

        #[qinvokable]
        fn toggle(self: Pin<&mut FindData>, row: i32);

        #[qinvokable]
        #[cxx_name = "showInMods"]
        fn show_in_mods(self: Pin<&mut FindData>) -> QString;

        #[qinvokable]
        #[cxx_name = "useRecent"]
        fn use_recent(self: Pin<&mut FindData>, at: i32);

        #[qinvokable]
        #[cxx_name = "filterResults"]
        fn filter_results(self: Pin<&mut FindData>, text: &QString);

        #[qinvokable]
        #[cxx_name = "completeType"]
        fn complete_type(self: &FindData, index: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "stepType"]
        fn step_type(self: &FindData, at: i32, by: i32) -> i32;

        /// A switch: "case", "word", "regex", "languages", "source", "textures" or "git".
        #[qinvokable]
        fn flip(self: Pin<&mut FindData>, id: &QString);

        #[qinvokable]
        fn choose(self: Pin<&mut FindData>, id: &QString);
    }

    unsafe extern "RustQt" {
        #[inherit]
        #[qsignal]
        #[cxx_name = "dataChanged"]
        fn data_changed(
            self: Pin<&mut FindData>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
            roles: &QVector_i32,
        );

        #[inherit]
        fn index(self: &FindData, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;
    }

    extern "RustQt" {
        #[inherit]
        #[cxx_name = "beginInsertRows"]
        unsafe fn begin_insert_rows(
            self: Pin<&mut FindData>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        #[cxx_name = "endInsertRows"]
        unsafe fn end_insert_rows(self: Pin<&mut FindData>);
        #[inherit]
        #[cxx_name = "beginRemoveRows"]
        unsafe fn begin_remove_rows(
            self: Pin<&mut FindData>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        #[cxx_name = "endRemoveRows"]
        unsafe fn end_remove_rows(self: Pin<&mut FindData>);
        #[inherit]
        #[cxx_name = "beginMoveRows"]
        unsafe fn begin_move_rows(
            self: Pin<&mut FindData>,
            source_parent: &QModelIndex,
            source_first: i32,
            source_last: i32,
            destination_parent: &QModelIndex,
            destination_child: i32,
        ) -> bool;
        #[inherit]
        #[cxx_name = "endMoveRows"]
        unsafe fn end_move_rows(self: Pin<&mut FindData>);
        #[inherit]
        #[cxx_name = "beginResetModel"]
        unsafe fn begin_reset_model(self: Pin<&mut FindData>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        unsafe fn end_reset_model(self: Pin<&mut FindData>);
    }
}

use crate::App;
use crate::changes::Changes;
use crate::qml::diff::{Step, steps};
use crate::qml::values::to_qml;
use crate::workspace::find::{Cell, Folded, Op, Prepared, ROLES, Request, RowRef, type_offers};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant, QVector,
};
use rimstack_core::base::locks::Held;
use rimstack_core::base::sentence::Sentence;
use rimstack_core::mods::find::{Batch, Summary};
use rimstack_core::mods::library::ModId;
use rimstack_core::say;
use std::sync::{Arc, Mutex};

mod look;

const FIRST_ROLE: i32 = 257;
const IN_SEARCH: &str = "in:search";
const TYPES_PAUSE: std::time::Duration = std::time::Duration::from_millis(400);
const MODS_EVERY: std::time::Duration = std::time::Duration::from_millis(200);

pub struct FindDataRust {
    text: QString,
    running: bool,
    done: i32,
    total: i32,
    progress: QString,
    summary: QString,
    problem: QString,
    capped: bool,
    count: i32,
    selected_mod: i32,
    current: i32,
    recent: QVariant,
    preview: QVariant,
    options: QVariant,
    mods: QVariant,
    file_types: QString,
    type_suggestions: QVariant,
    extensions: Vec<(String, usize)>,
    counted: crate::jobs::Lane,
    inside_text: QString,
    inside_said: QString,
    inside_found: i32,
    wrap: bool,
    viewing: look::Viewing,
    app: crate::hub::Given,
    request: Request,
    generation: u64,
    shown: Vec<RowRef>,
    said: Option<Sentence>,
    working: Option<Sentence>,
    runs: crate::jobs::Lane,
    previews: crate::jobs::Lane,
    retyped: crate::jobs::Lane,
    checks: crate::jobs::Lane,
    prepares: crate::jobs::Lane,
    mods_shown: Option<std::time::Instant>,
}

impl Default for FindDataRust {
    fn default() -> Self {
        Self {
            text: QString::default(),
            running: false,
            done: 0,
            total: 0,
            progress: QString::default(),
            summary: QString::default(),
            problem: QString::default(),
            capped: false,
            count: 0,
            selected_mod: -1,
            current: -1,
            recent: QVariant::default(),
            preview: QVariant::default(),
            options: QVariant::default(),
            mods: QVariant::default(),
            file_types: QString::from("xml"),
            type_suggestions: QVariant::default(),
            extensions: Vec::new(),
            counted: crate::jobs::Lane::default(),
            inside_text: QString::default(),
            inside_said: QString::default(),
            inside_found: 0,
            wrap: false,
            viewing: look::Viewing::default(),
            app: crate::hub::Given::default(),
            request: Request::default(),
            generation: 0,
            shown: Vec::new(),
            said: None,
            working: None,
            runs: crate::jobs::Lane::default(),
            previews: crate::jobs::Lane::default(),
            retyped: crate::jobs::Lane::default(),
            checks: crate::jobs::Lane::default(),
            prepares: crate::jobs::Lane::default(),
            mods_shown: None,
        }
    }
}

fn int(n: impl TryInto<i32>) -> i32 {
    n.try_into().unwrap_or(i32::MAX)
}

fn variant(cell: Cell<'_>) -> QVariant {
    match cell {
        Cell::Flag(b) => QVariant::from(&b),
        Cell::Number(n) => QVariant::from(&n),
        Cell::Text(t) => QVariant::from(&QString::from(t.as_ref())),
        Cell::Said(s) => QVariant::from(&crate::words::said(s)),
        Cell::Parts(parts) => to_qml(&parts),
    }
}

impl qobject::FindData {
    fn app(&self) -> Arc<App> {
        self.app.app()
    }

    pub(crate) fn connect(mut self: Pin<&mut Self>, app: Arc<App>) {
        crate::hub::hear(&app, self.qt_thread(), Self::heard);
        self.as_mut().rust_mut().get_mut().app = crate::hub::Given::of(app.clone());
        self.as_mut().on_text_changed(|me| me.check()).release();
        self.as_mut()
            .on_selected_mod_changed(|mut me| {
                me.as_mut().count_extensions();
                me.show_options()
            })
            .release();
        self.as_mut()
            .on_file_types_changed(|me| me.types_typed())
            .release();
        self.as_mut()
            .on_inside_text_changed(|me| me.inside_typed())
            .release();
        self.as_mut().show_options();
        self.as_mut().show_mods();
        self.as_mut().count_extensions();
        crate::jobs::run(
            "find recent",
            self.qt_thread(),
            move || app.find_recent(),
            |mut me, recent| me.as_mut().set_recent(to_qml(&recent)),
        );
    }

    fn heard(mut self: Pin<&mut Self>, changes: Changes) {
        if changes.contains(Changes::LIBRARY | Changes::LOAD_ORDER) {
            self.as_mut().count_extensions();
        }
        if !changes.contains(Changes::LANGUAGE) {
            return;
        }
        self.as_mut().show_suggestions();
        self.as_mut().show_options();
        self.as_mut().show_texts();
        self.as_mut().show_mods();
        self.as_mut().show_inside();
        let recent = self.app().find_recent();
        self.as_mut().set_recent(to_qml(&recent));
        self.changed_all();
    }

    fn changed_all(self: Pin<&mut Self>) {
        let last = int(self.shown.len()) - 1;
        if last < 0 {
            return;
        }
        let root = QModelIndex::default();
        let (top, bottom) = (self.index(0, 0, &root), self.index(last, 0, &root));
        self.data_changed(&top, &bottom, &QVector::default());
    }

    fn show_texts(mut self: Pin<&mut Self>) {
        let said = |s: &Option<Sentence>| s.as_ref().map(crate::words::said).unwrap_or_default();
        let (summary, progress) = (said(&self.said), said(&self.working));
        self.as_mut().set_summary(summary);
        self.set_progress(progress);
    }

    fn request_now(&self) -> Request {
        Request {
            text: self.text.to_string(),
            types: self.file_types.to_string(),
            selected: ModId::from_qml(self.selected_mod),
            ..self.request.clone()
        }
    }

    fn show_options(self: Pin<&mut Self>) {
        let request = self.request_now();
        let options = to_qml(&self.app().find_options(&request));
        self.set_options(options);
    }

    fn check(self: Pin<&mut Self>) {
        let (app, text, switches) = (self.app(), self.text.to_string(), self.request.switches);
        self.checks.run(
            "find check",
            self.qt_thread(),
            move |_| app.find_check(&text, switches),
            |me, checked| me.set_problem(crate::words::why_not(&checked)),
        );
    }

    fn count_extensions(self: Pin<&mut Self>) {
        let (app, request) = (self.app(), self.request_now());
        self.counted.run(
            "find extensions",
            self.qt_thread(),
            move |_| app.find_extensions(&request),
            |mut me, counts| {
                me.as_mut().rust_mut().get_mut().extensions = counts;
                me.show_suggestions();
            },
        );
    }

    fn show_suggestions(self: Pin<&mut Self>) {
        let offers = type_offers(&self.extensions, &self.file_types.to_string());
        self.set_type_suggestions(to_qml(&offers));
    }

    fn complete_type(&self, index: i32) -> QString {
        let text = self.file_types.to_string();
        let offers = type_offers(&self.extensions, &text);
        let picked = usize::try_from(index).ok().and_then(|at| offers.get(at));
        let done = picked.map_or(text.clone(), |o| {
            rimstack_core::mods::find::complete(&text, &o.ext)
        });
        QString::from(done.as_str())
    }

    fn step_type(&self, at: i32, by: i32) -> i32 {
        let count = int(type_offers(&self.extensions, &self.file_types.to_string()).len());
        match count {
            0 => 0,
            n => (at + by).rem_euclid(n),
        }
    }

    fn flip(mut self: Pin<&mut Self>, id: &QString) {
        let folders = self
            .as_mut()
            .rust_mut()
            .get_mut()
            .request
            .flip(&id.to_string());
        self.as_mut().check();
        self.options_moved(folders);
    }

    fn choose(mut self: Pin<&mut Self>, id: &QString) {
        self.as_mut()
            .rust_mut()
            .get_mut()
            .request
            .choose(&id.to_string());
        self.options_moved(true);
    }

    fn options_moved(mut self: Pin<&mut Self>, folders: bool) {
        if folders {
            self.as_mut().count_extensions();
        }
        self.as_mut().show_options();
        self.rerun();
    }

    fn types_typed(mut self: Pin<&mut Self>) {
        let typed = self.file_types.to_string();
        self.as_mut().show_suggestions();
        if typed == self.request.types {
            return;
        }
        self.as_mut().rust_mut().get_mut().request.types = typed;
        self.as_mut().show_options();
        self.retyped.run(
            "find types",
            self.qt_thread(),
            |turn| turn.after(TYPES_PAUSE),
            |me, _| me.rerun(),
        );
    }

    fn rerun(self: Pin<&mut Self>) {
        if !self.text.to_string().trim().is_empty() {
            self.begin(true);
        }
    }

    fn search(self: Pin<&mut Self>) {
        self.begin(false);
    }

    fn begin(mut self: Pin<&mut Self>, revised: bool) {
        self.retyped.cancel();
        self.as_mut().rust_mut().get_mut().generation = u64::MAX;
        self.as_mut().set_running(true);
        let app = self.app();
        let request = self.request_now();
        let (compiling, compiled) = (app, request.clone());
        self.prepares.run(
            "find prepare",
            self.qt_thread(),
            move |_| compiling.find_prepare(&compiled),
            move |mut me, prepared| match prepared {
                Ok(prepared) => me.launch(request, revised, prepared),
                Err(problem) => {
                    me.as_mut().set_running(false);
                    if !revised {
                        me.set_problem(crate::words::said_of(&problem));
                    }
                }
            },
        );
    }

    fn launch(mut self: Pin<&mut Self>, request: Request, revised: bool, prepared: Prepared) {
        let app = self.app();
        let started = app.find_launch(prepared);
        let generation = started.generation;
        self.as_mut().set_problem(QString::default());
        self.as_mut().renew(Vec::new());
        let me = self.as_mut().rust_mut().get_mut();
        me.generation = generation;
        me.said = None;
        me.working = Some(say!("Searching"));
        self.as_mut().set_preview(QVariant::default());
        self.as_mut().rust_mut().get_mut().viewing = look::Viewing::default();
        self.as_mut().show_inside();
        self.as_mut().show_mods();
        self.as_mut().rust_mut().get_mut().mods_shown = Some(std::time::Instant::now());
        self.as_mut().set_capped(false);
        self.as_mut().set_count(0);
        self.as_mut().set_done(0);
        self.as_mut().set_total(0);
        self.as_mut().set_running(true);
        self.as_mut().show_texts();
        let remember = app.clone();
        crate::jobs::run(
            "find remember",
            self.qt_thread(),
            move || {
                remember.find_remember(&request, revised);
                remember.find_recent()
            },
            |mut me, recent| me.as_mut().set_recent(to_qml(&recent)),
        );
        let batches = Mutex::new(self.qt_thread());
        self.runs.run(
            "find",
            self.qt_thread(),
            move |_| {
                app.find_search(&started, |batch| {
                    let _ = batches.held().queue(move |me| me.arrive(generation, batch));
                })
            },
            move |me, summary| me.finished(generation, summary),
        );
    }

    fn arrive(mut self: Pin<&mut Self>, generation: u64, batch: Batch<Option<ModId>>) {
        if generation != self.generation {
            return;
        }
        let Some(arrived) = self.app().find_add(generation, batch) else {
            return;
        };
        for op in arrived.ops {
            self.as_mut().apply(op);
        }
        let (done, total) = (arrived.progress.done, arrived.progress.total);
        self.as_mut().set_done(int(done));
        self.as_mut().set_total(int(total));
        self.as_mut().set_count(int(arrived.mods));
        if self.mods_shown.is_none_or(|at| at.elapsed() >= MODS_EVERY) {
            self.as_mut().rust_mut().get_mut().mods_shown = Some(std::time::Instant::now());
            self.as_mut().show_mods();
        }
        self.as_mut().set_capped(arrived.capped);
        self.as_mut().rust_mut().get_mut().working =
            Some(say!("Mods searched: {} of {}", done, total));
        self.show_texts();
    }

    fn finished(mut self: Pin<&mut Self>, generation: u64, summary: Summary) {
        if generation != self.generation {
            return;
        }
        let said = self.app().find_finish(generation, summary);
        let me = self.as_mut().rust_mut().get_mut();
        me.said = said;
        me.working = None;
        self.as_mut().set_running(false);
        self.as_mut().show_mods();
        self.show_texts();
    }

    fn stop(&self) {
        self.app().find_stop();
    }

    fn apply(self: Pin<&mut Self>, op: Op) {
        let root = QModelIndex::default();
        match op {
            Op::Insert { at, rows } => self.insert(at, rows),
            Op::Changed { at } => {
                let index = self.index(int(at), 0, &root);
                self.data_changed(&index, &index, &QVector::default());
            }
        }
    }

    fn insert(mut self: Pin<&mut Self>, at: usize, rows: Vec<RowRef>) {
        if rows.is_empty() || at > self.shown.len() {
            return;
        }
        let coming = rows.len();
        let root = QModelIndex::default();
        // SAFETY: the insert ends right after the rows are in.
        unsafe {
            self.as_mut()
                .begin_insert_rows(&root, int(at), int(at + rows.len() - 1));
            self.as_mut()
                .rust_mut()
                .get_mut()
                .shown
                .splice(at..at, rows);
            self.as_mut().end_insert_rows();
        }
        let current = self.current;
        if current >= int(at) {
            self.set_current(current + int(coming));
        }
    }

    fn remove(mut self: Pin<&mut Self>, first: usize, last: usize) {
        let root = QModelIndex::default();
        // SAFETY: the removal ends right after the rows are out.
        unsafe {
            self.as_mut()
                .begin_remove_rows(&root, int(first), int(last));
            self.as_mut().rust_mut().get_mut().shown.drain(first..=last);
            self.as_mut().end_remove_rows();
        }
        let (current, gone) = (self.current, int(last - first + 1));
        if current >= int(first) {
            let kept = (current > int(last)).then(|| current - gone);
            self.set_current(kept.unwrap_or(-1));
        }
    }

    fn renew(mut self: Pin<&mut Self>, rows: Vec<RowRef>) {
        // SAFETY: the reset ends below, after the rows changed.
        unsafe { self.as_mut().begin_reset_model() };
        self.as_mut().rust_mut().get_mut().shown = rows;
        // SAFETY: ends the reset begun above.
        unsafe { self.as_mut().end_reset_model() };
        self.set_current(-1);
    }

    fn reach(mut self: Pin<&mut Self>, rows: Vec<RowRef>) {
        match steps(&self.shown, &rows) {
            Some(steps) => {
                for step in steps {
                    match step {
                        Step::Remove { first, last } => self.as_mut().remove(first, last),
                        Step::Insert { first, last } => {
                            let coming = rows[first..=last].to_vec();
                            self.as_mut().insert(first, coming);
                        }
                        Step::Move { .. } => return self.renew(rows),
                    }
                }
            }
            None => self.renew(rows),
        }
    }

    pub(self) fn row(&self, row: i32) -> Option<RowRef> {
        self.shown.get(usize::try_from(row).ok()?).copied()
    }

    fn step(&self, row: i32, by: i32) -> i32 {
        let moved = self
            .app()
            .find_results(|r| r.step(&self.shown, usize::try_from(row).ok(), by));
        moved.map_or(row, int)
    }

    fn fold(self: Pin<&mut Self>, row: i32, how: i32) -> i32 {
        let Some(at) = self.row(row).filter(|at| !at.is_mod()) else {
            return row;
        };
        let file_row = match at.is_file() {
            true => row,
            false => {
                let before = &self.shown[..usize::try_from(row).unwrap_or_default()];
                before
                    .iter()
                    .rposition(|other| other.same_file(at) && other.is_file())
                    .map_or(row, int)
            }
        };
        let folded = self.app().find_results(|r| r.is_folded(at));
        if how != 0 && folded == (how < 0) {
            return row;
        }
        self.toggle(file_row);
        file_row
    }

    fn path_text(
        &self,
        row: i32,
        of: impl FnOnce(&App, RowRef) -> Option<std::path::PathBuf>,
    ) -> QString {
        let path = self.row(row).and_then(|at| of(&self.app(), at));
        QString::from(
            path.map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default()
                .as_str(),
        )
    }

    fn open_file(&self, row: i32) -> QString {
        self.path_text(row, App::find_file)
    }

    fn open_folder(&self, row: i32) -> QString {
        self.path_text(row, App::find_folder)
    }

    fn copy_path(&self, row: i32) -> QString {
        self.path_text(row, App::find_file)
    }

    fn copy_line(&self, row: i32) -> QString {
        let line = self.row(row).and_then(|at| self.app().find_line(at));
        QString::from(line.unwrap_or_default().as_str())
    }

    fn toggle(mut self: Pin<&mut Self>, row: i32) {
        let Some(at) = self.row(row) else {
            return;
        };
        let Some(folded) = self.app().find_change(|results| results.toggle(at)) else {
            return;
        };
        let first = usize::try_from(row).unwrap_or_default();
        match folded {
            Folded::Hid => {
                let inside = |other: &&RowRef| match at.is_mod() {
                    true => other.same_mod(at),
                    false => other.same_file(at),
                };
                let below = self.shown[first + 1..].iter().take_while(inside).count();
                if below > 0 {
                    self.as_mut().remove(first + 1, first + below);
                }
            }
            Folded::Showed(rows) => self.as_mut().insert(first + 1, rows),
        }
        let index = self.index(row, 0, &QModelIndex::default());
        self.data_changed(&index, &index, &QVector::default());
    }

    fn show_in_mods(self: Pin<&mut Self>) -> QString {
        let count = self.app().find_show_in_mods();
        self.set_count(int(count));
        QString::from(IN_SEARCH)
    }

    fn use_recent(mut self: Pin<&mut Self>, at: i32) {
        let selected = ModId::from_qml(self.selected_mod);
        let Some(request) = usize::try_from(at)
            .ok()
            .and_then(|at| self.app().find_recall(at, selected))
        else {
            return;
        };
        let text = QString::from(request.text.as_str());
        let types = QString::from(request.types.as_str());
        self.as_mut().rust_mut().get_mut().request = request;
        self.as_mut().set_text(text);
        self.as_mut().set_file_types(types);
        self.as_mut().show_options();
        self.search();
    }

    fn filter_results(mut self: Pin<&mut Self>, text: &QString) {
        let text = text.to_string();
        let rows = self.app().find_change(|results| results.set_filter(&text));
        self.as_mut().reach(rows);
        self.show_mods();
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let field = usize::try_from(role - FIRST_ROLE).ok();
        let at = self.row(index.row()).zip(field);
        let cell =
            at.and_then(|(at, field)| self.app().find_results(|r| r.cell(at, field).map(variant)));
        cell.unwrap_or_default()
    }

    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        int(self.shown.len())
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut names = QHash::<QHashPair_i32_QByteArray>::default();
        for (at, name) in ROLES.iter().enumerate() {
            names.insert(FIRST_ROLE + int(at), QByteArray::from(*name));
        }
        names
    }
}
