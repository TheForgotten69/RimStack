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
        /// `{ name, mods, modified }`.
        #[qproperty(QVariant, saved)]
        /// `{ name, title }`.
        #[qproperty(QVariant, snapshots)]
        /// Newest first; `{ time, said, restore, bringBack }`.
        #[qproperty(QVariant, history)]
        #[qproperty(bool, busy)]
        #[qproperty(QString, problem, cxx_name = "clipboardProblem")]
        type ListsData = super::ListsDataRust;
    }

    extern "RustQt" {
        /// Package ids.
        #[qsignal]
        fn found(self: Pin<&mut ListsData>, ids: QVariant);

        /// "" for none.
        #[qsignal]
        fn renamed(self: Pin<&mut ListsData>, name: QString);

        #[qsignal]
        fn shared(self: Pin<&mut ListsData>, link: QString);

        /// `tone` is "" or "ok".
        #[qsignal]
        fn told(self: Pin<&mut ListsData>, note: QString, tone: QString);
    }

    impl cxx_qt::Threading for ListsData {}

    extern "RustQt" {
        #[qinvokable]
        fn keep(self: &ListsData, name: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "keepIds"]
        fn keep_ids(self: &ListsData, name: &QString, ids: &QVariant) -> QString;

        #[qinvokable]
        fn remove(self: &ListsData, name: &QString);

        /// `None` for no such list.
        #[qinvokable]
        fn named(self: &ListsData, name: &QString) -> QVariant;

        #[qinvokable]
        #[cxx_name = "snapshotNamed"]
        fn snapshot_named(self: &ListsData, name: &QString) -> QVariant;

        #[qinvokable]
        #[cxx_name = "useCopied"]
        fn use_copied(self: Pin<&mut ListsData>, text: &QString);

        #[qinvokable]
        fn share(self: Pin<&mut ListsData>, listing: &QString);

        #[qinvokable]
        #[cxx_name = "startWithGame"]
        fn start_with_game(self: Pin<&mut ListsData>);

        #[qinvokable]
        #[cxx_name = "startWithExpansions"]
        fn start_with_expansions(self: Pin<&mut ListsData>);

        #[qinvokable]
        fn original(self: &ListsData) -> QVariant;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[cxx_name = "make_unique"]
        #[namespace = "rust::cxxqtlib1"]
        fn make_lists_data() -> UniquePtr<ListsData>;
    }
}

use crate::App;
use crate::changes::Changes;
use crate::qml::values::{from_qml, to_qml};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QVariant};
use rimstack_core::base::error::Error;
use rimstack_core::say;
use std::sync::Arc;

#[derive(Default)]
pub struct ListsDataRust {
    saved: QVariant,
    snapshots: QVariant,
    history: QVariant,
    busy: bool,
    problem: QString,
    app: crate::hub::Given,
}

fn said(done: Result<(), Error>) -> QString {
    crate::words::why_not(&done)
}

impl qobject::ListsData {
    fn app(&self) -> Arc<App> {
        self.app.app()
    }

    pub(crate) fn connect(mut self: Pin<&mut Self>, app: Arc<App>, demo: bool) {
        self.as_mut().rust_mut().get_mut().app = crate::hub::Given::of(app.clone());
        if demo {
            return;
        }
        crate::hub::hear(&app, self.qt_thread(), Self::heard);
        self.heard(Changes::LISTS);
    }

    fn heard(mut self: Pin<&mut Self>, changes: Changes) {
        if !changes.contains(Changes::LISTS | Changes::LANGUAGE) {
            return;
        }
        let app = self.app();
        self.as_mut().set_saved(to_qml(&app.saved_lists()));
        self.as_mut().set_snapshots(to_qml(&app.recent_snapshots()));
        self.set_history(to_qml(&app.history()));
    }

    fn keep(&self, name: &QString) -> QString {
        said(self.app().keep_list(&name.to_string(), None))
    }

    fn keep_ids(&self, name: &QString, ids: &QVariant) -> QString {
        match from_qml::<Vec<String>>(ids) {
            Some(ids) => said(self.app().keep_list(&name.to_string(), Some(ids))),
            None => crate::words::said(&say!("That is not a list of mods.")),
        }
    }

    fn remove(&self, name: &QString) {
        self.app().delete_list(&name.to_string());
    }

    fn named(&self, name: &QString) -> QVariant {
        to_qml(&self.app().list_named(&name.to_string()))
    }

    fn snapshot_named(&self, name: &QString) -> QVariant {
        to_qml(&self.app().snapshot_named(&name.to_string()))
    }

    fn start_with_game(self: Pin<&mut Self>) {
        self.start_fresh(false);
    }

    fn start_with_expansions(self: Pin<&mut Self>) {
        self.start_fresh(true);
    }

    fn start_fresh(mut self: Pin<&mut Self>, with_expansions: bool) {
        let ids = self.app().fresh_list(with_expansions);
        self.as_mut().found(to_qml(&ids));
        self.renamed(QString::default());
    }

    fn use_copied(mut self: Pin<&mut Self>, text: &QString) {
        if *self.busy() {
            return;
        }
        self.as_mut().set_problem(QString::default());
        self.as_mut().set_busy(true);
        let (app, text) = (self.app(), text.to_string());
        crate::jobs::run(
            "copied list",
            self.qt_thread(),
            move || app.ids_in_copied(&text),
            |mut me, read| {
                me.as_mut().set_busy(false);
                me.arrived(read);
            },
        );
    }

    fn arrived(mut self: Pin<&mut Self>, read: Result<Vec<String>, Error>) {
        match read {
            Ok(ids) if ids.is_empty() => {
                let none = say!("No mod found on the clipboard");
                self.set_problem(crate::words::said(&none));
            }
            Ok(ids) => {
                self.as_mut().found(to_qml(&ids));
                self.renamed(QString::default());
            }
            Err(problem) => self.set_problem(crate::words::said_of(&problem)),
        }
    }

    fn share(mut self: Pin<&mut Self>, listing: &QString) {
        if *self.busy() {
            return;
        }
        let listing = listing.to_string();
        if listing.is_empty() {
            let none = say!("There is no load order to share");
            return self.set_problem(crate::words::said(&none));
        }
        self.as_mut().set_problem(QString::default());
        self.as_mut().set_busy(true);
        let app = self.app();
        crate::jobs::run(
            "rentry publish",
            self.qt_thread(),
            move || app.publish_list(&listing),
            |mut me, published| {
                me.as_mut().set_busy(false);
                match published {
                    Ok(page) => {
                        let note = say!(
                            "The load order is at {}, and the link is copied. Its edit code, which RimStack does not keep: {}",
                            page.url,
                            page.edit_code
                        );
                        me.as_mut().shared(QString::from(&page.url));
                        me.told(crate::words::said(&note), QString::from("ok"));
                    }
                    Err(problem) => me.set_problem(crate::words::said_of(&problem)),
                }
            },
        );
    }

    fn original(&self) -> QVariant {
        to_qml(&self.app().original_list())
    }
}
