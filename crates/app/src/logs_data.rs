//! `Hub.logs`: the Logs screen. The log is read again whenever it, the
//! mods or the load order change, while the screen wants it.

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
        #[qproperty(QVariant, head)]
        /// The errors the filter lets through.
        #[qproperty(QVariant, errors)]
        /// The mods the errors point at, most errors first.
        #[qproperty(QVariant, culprits)]
        /// The launches on record, newest first.
        #[qproperty(QVariant, launches)]
        #[qproperty(QVariant, since)]
        /// The log's lines that hold the filter text.
        #[qproperty(QVariant, lines)]
        #[qproperty(QString, problem)]
        #[qproperty(bool, busy)]
        /// What the filter holds to list the errors that point at no mod.
        #[qproperty(QString, no_mod, cxx_name = "noMod")]
        #[qproperty(QString, filter)]
        /// The log before the game's last.
        #[qproperty(bool, previous)]
        /// A kept log, by its path, in place of the game's.
        #[qproperty(QString, kept)]
        /// The screen is shown: the log is read, and read again on change.
        #[qproperty(bool, wanted)]
        type LogsData = super::LogsDataRust;
    }

    impl cxx_qt::Threading for LogsData {}

    extern "RustQt" {
        /// Opens or closes a run of alike errors, by where it begins.
        #[qinvokable]
        #[cxx_name = "toggleRun"]
        fn toggle_run(self: Pin<&mut LogsData>, run: i32);

        /// The listed error at `at` as text for asking for help.
        #[qinvokable]
        #[cxx_name = "helpText"]
        fn help_text(self: &LogsData, at: i32) -> QString;

        /// The errors counted against the mod of this name.
        #[qinvokable]
        #[cxx_name = "errorsOf"]
        fn errors_of(self: &LogsData, name: &QString) -> QVariant;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[cxx_name = "make_unique"]
        #[namespace = "rust::cxxqtlib1"]
        fn make_logs_data() -> UniquePtr<LogsData>;
    }
}

use crate::changes::Changes;
use crate::qml::values::to_qml;
use crate::workspace::logs::{self, LogsReport};
use crate::App;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QVariant};
use rimstack_core::base::locks::Held;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct LogsDataRust {
    head: QVariant,
    errors: QVariant,
    culprits: QVariant,
    launches: QVariant,
    since: QVariant,
    lines: QVariant,
    problem: QString,
    busy: bool,
    no_mod: QString,
    filter: QString,
    previous: bool,
    kept: QString,
    wanted: bool,
    app: Option<Arc<App>>,
    report: Option<LogsReport>,
    open_runs: HashSet<usize>,
    /// What was read no longer holds.
    stale: bool,
    keep: bool,
    /// The launches were told changed by this reading itself.
    told_launches: bool,
}

impl qobject::LogsData {
    fn app(&self) -> Arc<App> {
        self.app
            .clone()
            .expect("connected by the Hub before QML sees it")
    }

    /// `keep`: the game's logs are copied among the kept ones now and
    /// whenever they change.
    pub(crate) fn connect(mut self: Pin<&mut Self>, app: Arc<App>, keep: bool) {
        let thread = Mutex::new(self.qt_thread());
        app.listen(move |changes| {
            let _ = thread.held().queue(move |me| me.heard(changes));
        });
        if keep {
            let app = app.clone();
            crate::jobs::spawn("keep logs", move || app.keep_logs());
        }
        let me = self.as_mut().rust_mut().get_mut();
        me.app = Some(app);
        me.stale = true;
        me.keep = keep;
        self.as_mut()
            .set_no_mod(QString::from(rimstack_core::rimworld::logs::NO_MOD));
        self.as_mut().on_wanted_changed(|me| me.read()).release();
        self.as_mut()
            .on_previous_changed(|me| me.choose())
            .release();
        self.as_mut().on_kept_changed(|me| me.choose()).release();
        self.as_mut().on_filter_changed(|me| me.list()).release();
    }

    fn heard(mut self: Pin<&mut Self>, changes: Changes) {
        if changes.contains(Changes::LOGS) && self.keep {
            let app = self.app();
            crate::jobs::spawn("keep logs", move || app.keep_logs());
        }
        let mut read_again = Changes::LIBRARY | Changes::LOAD_ORDER | Changes::LOGS;
        if !std::mem::take(&mut self.as_mut().rust_mut().get_mut().told_launches) {
            read_again |= Changes::LAUNCHES;
        }
        if changes.contains(read_again) {
            self.as_mut().rust_mut().get_mut().stale = true;
            self.read();
        } else if changes.contains(Changes::LANGUAGE) {
            self.show();
        }
    }

    fn choose(mut self: Pin<&mut Self>) {
        let me = self.as_mut().rust_mut().get_mut();
        me.stale = true;
        me.open_runs.clear();
        self.read();
    }

    fn read(mut self: Pin<&mut Self>) {
        if !self.wanted || !self.stale || self.busy {
            return;
        }
        self.as_mut().rust_mut().get_mut().stale = false;
        self.as_mut().set_busy(true);
        let app = self.app();
        let previous = self.previous;
        let kept = Some(self.kept.to_string())
            .filter(|kept| !kept.is_empty())
            .map(PathBuf::from);
        crate::jobs::run(
            "log",
            self.qt_thread(),
            move || app.logs_report(previous, kept.as_deref()),
            |mut me, read| {
                let (report, problem) = match read {
                    Ok((report, changed)) => {
                        if changed {
                            me.as_mut().rust_mut().get_mut().told_launches = true;
                            me.app().publish(Changes::LAUNCHES);
                        }
                        (Some(report), QString::default())
                    }
                    Err(problem) => (None, crate::words::said_of(&problem)),
                };
                me.as_mut().rust_mut().get_mut().report = report;
                me.as_mut().set_problem(problem);
                me.as_mut().set_busy(false);
                me.as_mut().show();
                me.read();
            },
        );
    }

    fn show(mut self: Pin<&mut Self>) {
        let (head, culprits, launches, since) = match &self.report {
            Some(report) => (
                to_qml(&logs::head(report)),
                to_qml(&report.culprits),
                to_qml(&logs::launches(report)),
                to_qml(&logs::since(report)),
            ),
            None => Default::default(),
        };
        self.as_mut().set_head(head);
        self.as_mut().set_culprits(culprits);
        self.as_mut().set_launches(launches);
        self.as_mut().set_since(since);
        self.list();
    }

    fn filter_text(&self) -> String {
        self.filter.to_string().trim().to_lowercase()
    }

    fn list(mut self: Pin<&mut Self>) {
        let filter = self.filter_text();
        let (errors, lines) = match &self.report {
            Some(report) => (
                to_qml(&logs::listed(&report.errors, &filter, &self.open_runs)),
                to_qml(&logs::lines(report, &filter)),
            ),
            None => Default::default(),
        };
        self.as_mut().set_errors(errors);
        self.set_lines(lines);
    }

    fn toggle_run(mut self: Pin<&mut Self>, run: i32) {
        let Ok(run) = usize::try_from(run) else {
            return;
        };
        let open = &mut self.as_mut().rust_mut().get_mut().open_runs;
        if !open.remove(&run) {
            open.insert(run);
        }
        self.list();
    }

    fn help_text(&self, at: i32) -> QString {
        let Some(report) = &self.report else {
            return QString::default();
        };
        let listed = logs::listed(&report.errors, &self.filter_text(), &self.open_runs);
        let error = usize::try_from(at).ok().and_then(|at| listed.get(at));
        error.map_or_else(QString::default, |e| {
            QString::from(&logs::help_text(report, e.error))
        })
    }

    fn errors_of(&self, name: &QString) -> QVariant {
        match &self.report {
            Some(report) => to_qml(&logs::errors_of(report, &name.to_string())),
            None => QVariant::default(),
        }
    }
}
