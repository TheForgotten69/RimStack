//! `Hub.textures`: the Textures screen, and the comparison of two mods'
//! textures on the Mods screen. Textures are named by their place in the
//! comparison and mods by id: the window never names a file to act on.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        /// The rows on show and what the screen says of them all.
        #[qproperty(QVariant, report)]
        #[qproperty(bool, busy)]
        /// How far a conversion is, then what came of it.
        #[qproperty(QString, progress)]
        /// The DDS files the game would refuse, once looked for; null before.
        #[qproperty(QVariant, refused)]
        #[qproperty(bool, auditing)]
        /// Two mods' shared textures: `{ earlier, later, total, keptEarlier, yours }`.
        #[qproperty(QVariant, comparison)]
        /// The shared textures whose name holds `sharedFilter`.
        #[qproperty(QVariant, shared)]
        #[qproperty(QString, shared_filter, cxx_name = "sharedFilter")]
        #[qproperty(bool, active_only, cxx_name = "activeOnly")]
        /// The Textures screen is shown: the textures are counted, and
        /// counted again on change.
        #[qproperty(bool, wanted)]
        type TexturesData = super::TexturesDataRust;
    }

    impl cxx_qt::Threading for TexturesData {}

    extern "RustQt" {
        /// Makes DDS files for these mods.
        #[qinvokable]
        fn convert(self: Pin<&mut TexturesData>, ids: &QVariant);
        /// Deletes the DDS files RimStack made for these mods.
        #[qinvokable]
        fn remove(self: Pin<&mut TexturesData>, ids: &QVariant);
        /// A conversion stops after the mod it is on.
        #[qinvokable]
        fn cancel(self: &TexturesData);
        #[qinvokable]
        fn skip(self: &TexturesData, mod_id: i32, skip: bool);

        #[qinvokable]
        fn audit(self: Pin<&mut TexturesData>);
        #[qinvokable]
        #[cxx_name = "closeAudit"]
        fn close_audit(self: Pin<&mut TexturesData>);
        /// Renames every refused file out of the game's way, then looks again.
        #[qinvokable]
        #[cxx_name = "setRefusedAside"]
        fn set_refused_aside(self: Pin<&mut TexturesData>);

        /// Compares two mods' textures; `earlier` loads first.
        #[qinvokable]
        fn compare(self: Pin<&mut TexturesData>, earlier: i32, later: i32);
        #[qinvokable]
        #[cxx_name = "endCompare"]
        fn end_compare(self: Pin<&mut TexturesData>);
        /// Which mod's version of the shared texture at `at` the game uses;
        /// says why not.
        #[qinvokable]
        fn keep(self: Pin<&mut TexturesData>, at: i32, keep_earlier: bool) -> QString;
        /// The same for every shared texture: `{ said }` or `{ problem }`.
        #[qinvokable]
        #[cxx_name = "keepAll"]
        fn keep_all(self: Pin<&mut TexturesData>, keep_earlier: bool) -> QVariant;
        /// Where the two versions of the texture at `at` differ:
        /// `{ file, differing, total }` or `{ problem }`.
        #[qinvokable]
        fn difference(self: &TexturesData, at: i32) -> QVariant;

        #[qinvokable]
        #[cxx_name = "hiddenIn"]
        fn hidden_in(self: &TexturesData, mod_id: i32) -> i32;
        /// Puts back every texture set aside in a mod; says why not.
        #[qinvokable]
        #[cxx_name = "unhideMod"]
        fn unhide_mod(self: &TexturesData, mod_id: i32) -> QString;
        /// The same for an installed mod's folder, as the history names it.
        #[qinvokable]
        #[cxx_name = "unhideFolder"]
        fn unhide_folder(self: &TexturesData, folder: &QString) -> QString;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[cxx_name = "make_unique"]
        #[namespace = "rust::cxxqtlib1"]
        fn make_textures_data() -> UniquePtr<TexturesData>;
    }
}

use crate::changes::Changes;
use crate::qml::values::{mod_ids, to_qml};
use crate::reports::Answer;
use crate::workspace::textures::{shown, Comparison, Refused};
use crate::App;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QVariant};
use rimstack_core::base::error::Error;
use rimstack_core::base::locks::Held;
use rimstack_core::mods::library::ModId;
use rimstack_core::mods::textures::Row;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct TexturesDataRust {
    report: QVariant,
    busy: bool,
    progress: QString,
    refused: QVariant,
    auditing: bool,
    comparison: QVariant,
    shared: QVariant,
    shared_filter: QString,
    active_only: bool,
    wanted: bool,
    app: Option<Arc<App>>,
    rows: Vec<Row>,
    found: Option<Vec<Refused>>,
    compared: Option<Comparison>,
    stop: Arc<AtomicBool>,
    /// What was counted no longer holds.
    stale: bool,
}

fn said(done: Result<(), Error>) -> QString {
    done.err().map_or_else(QString::default, |problem| {
        QString::from(&problem.to_string())
    })
}

impl qobject::TexturesData {
    fn app(&self) -> Arc<App> {
        self.app
            .clone()
            .expect("connected by the Hub before QML sees it")
    }

    pub(crate) fn connect(mut self: Pin<&mut Self>, app: Arc<App>) {
        let thread = Mutex::new(self.qt_thread());
        app.listen(move |changes| {
            let _ = thread.held().queue(move |me| me.heard(changes));
        });
        let me = self.as_mut().rust_mut().get_mut();
        me.app = Some(app);
        me.stale = true;
        me.active_only = true;
        self.as_mut().on_wanted_changed(|me| me.count()).release();
        self.as_mut()
            .on_active_only_changed(|me| me.show())
            .release();
        self.as_mut()
            .on_shared_filter_changed(|me| me.show_comparison())
            .release();
    }

    fn heard(mut self: Pin<&mut Self>, changes: Changes) {
        let counted = Changes::LIBRARY | Changes::TEXTURES | Changes::LOAD_ORDER;
        if changes.contains(counted) {
            self.as_mut().rust_mut().get_mut().stale = true;
            self.as_mut().count();
        } else if changes.contains(Changes::LANGUAGE) {
            self.as_mut().show();
        }
        if changes.contains(Changes::LIBRARY) {
            self.as_mut().end_compare();
        } else if changes.contains(Changes::TEXTURES | Changes::LOAD_ORDER) {
            self.recompare();
        }
    }

    fn count(mut self: Pin<&mut Self>) {
        if !self.wanted || !self.stale || self.busy {
            return;
        }
        self.as_mut().rust_mut().get_mut().stale = false;
        self.as_mut().set_busy(true);
        let app = self.app();
        crate::background::run(
            self.qt_thread(),
            move || app.texture_rows(),
            |mut me, rows| {
                me.as_mut().rust_mut().get_mut().rows = rows;
                me.as_mut().set_busy(false);
                me.as_mut().show();
                me.count();
            },
        );
    }

    fn show(mut self: Pin<&mut Self>) {
        let report = to_qml(&shown(&self.rows, self.active_only));
        self.as_mut().set_report(report);
    }

    fn work(mut self: Pin<&mut Self>, ids: &QVariant, remove: bool) {
        if self.busy {
            return;
        }
        let ids = mod_ids(ids);
        self.as_mut().set_busy(true);
        self.stop.store(false, Ordering::Relaxed);
        let (app, stop, thread) = (self.app(), self.stop.clone(), self.qt_thread());
        crate::background::run(
            self.qt_thread(),
            move || {
                let progress = |text: String| {
                    let _ = thread.queue(move |me| me.set_progress(QString::from(&text)));
                };
                let note = app.convert_textures(&ids, remove, &stop, &progress);
                (app.texture_rows(), note)
            },
            |mut me, (rows, note)| {
                let me_rust = me.as_mut().rust_mut().get_mut();
                me_rust.rows = rows;
                me_rust.stale = false;
                me.as_mut().set_progress(QString::from(&note));
                me.as_mut().set_busy(false);
                me.show();
            },
        );
    }

    fn convert(self: Pin<&mut Self>, ids: &QVariant) {
        self.work(ids, false);
    }

    fn remove(self: Pin<&mut Self>, ids: &QVariant) {
        self.work(ids, true);
    }

    fn cancel(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    fn skip(&self, mod_id: i32, skip: bool) {
        let Some(id) = ModId::from_qml(mod_id) else {
            return;
        };
        if let Err(problem) = self.app().skip_textures(id, skip) {
            rimstack_core::problem!("{problem}");
        }
    }

    fn audit(mut self: Pin<&mut Self>) {
        if self.auditing {
            return;
        }
        self.as_mut().set_auditing(true);
        let app = self.app();
        crate::background::run(
            self.qt_thread(),
            move || app.refused_dds(),
            |mut me, found| {
                let refused = to_qml(&found);
                me.as_mut().rust_mut().get_mut().found = Some(found);
                me.as_mut().set_refused(refused);
                me.set_auditing(false);
            },
        );
    }

    fn close_audit(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().get_mut().found = None;
        self.set_refused(QVariant::default());
    }

    fn set_refused_aside(mut self: Pin<&mut Self>) {
        let Some(found) = self.as_mut().rust_mut().get_mut().found.take() else {
            return;
        };
        let failed = self.app().set_dds_aside(&found);
        if failed > 0 {
            rimstack_core::problem!("{failed} DDS files could not be set aside");
        }
        self.audit();
    }

    fn compare(mut self: Pin<&mut Self>, earlier: i32, later: i32) {
        let compared = match (ModId::from_qml(earlier), ModId::from_qml(later)) {
            (Some(earlier), Some(later)) => self.app().compare_textures(earlier, later),
            _ => None,
        };
        self.as_mut().rust_mut().get_mut().compared = compared;
        self.show_comparison();
    }

    fn recompare(self: Pin<&mut Self>) {
        let Some(sides) = self.compared.as_ref().map(|c| (c.earlier.id, c.later.id)) else {
            return;
        };
        self.compare(sides.0.to_qml(), sides.1.to_qml());
    }

    fn end_compare(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().get_mut().compared = None;
        self.as_mut().set_shared_filter(QString::default());
        self.show_comparison();
    }

    fn show_comparison(mut self: Pin<&mut Self>) {
        let filter = self.shared_filter.to_string();
        let (comparison, shared) = match &self.compared {
            Some(compared) => (to_qml(compared), to_qml(&compared.named(&filter))),
            None => Default::default(),
        };
        self.as_mut().set_comparison(comparison);
        self.set_shared(shared);
    }

    fn keep(mut self: Pin<&mut Self>, at: i32, keep_earlier: bool) -> QString {
        let chosen = usize::try_from(at).ok().and_then(|at| {
            let compared = self.compared.as_ref()?;
            compared.textures.get(at).cloned()
        });
        let Some(chosen) = chosen else {
            return QString::default();
        };
        let done = self.app().keep_textures(&[&chosen], keep_earlier);
        self.as_mut().recompare();
        said(done)
    }

    fn keep_all(mut self: Pin<&mut Self>, keep_earlier: bool) -> QVariant {
        let Some(compared) = self.compared.clone() else {
            return QVariant::default();
        };
        let done = self.app().keep_all_textures(&compared, keep_earlier);
        self.as_mut().recompare();
        to_qml(&Answer::from(
            done.map(|said| serde_json::json!({ "said": said })),
        ))
    }

    fn difference(&self, at: i32) -> QVariant {
        let chosen = usize::try_from(at).ok().and_then(|at| {
            let compared = self.compared.as_ref()?;
            compared.textures.get(at)
        });
        match chosen {
            Some(shared) => to_qml(&Answer::from(self.app().texture_difference(shared))),
            None => QVariant::default(),
        }
    }

    fn hidden_in(&self, mod_id: i32) -> i32 {
        let hidden = ModId::from_qml(mod_id).map_or(0, |id| self.app().hidden_in(id));
        i32::try_from(hidden).unwrap_or(i32::MAX)
    }

    fn unhide_mod(&self, mod_id: i32) -> QString {
        match ModId::from_qml(mod_id) {
            Some(id) => said(self.app().unhide_mod(id)),
            None => QString::default(),
        }
    }

    fn unhide_folder(&self, folder: &QString) -> QString {
        said(self.app().unhide_folder(Path::new(&folder.to_string())))
    }
}
