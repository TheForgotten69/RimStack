//! The Logs screen's own object: reads a log on a background thread and
//! hands the report to QML as JSON text.

use crate::backend::log_report;
use std::path::PathBuf;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, logs)]
        #[qproperty(bool, logs_busy, cxx_name = "logsBusy")]
        type LogReader = super::LogReaderRust;
    }

    impl cxx_qt::Threading for LogReader {}

    extern "RustQt" {
        /// Read the game's log (or, with `previous`, the log of the launch
        /// before) and the launch record. Fills `logs` with a JSON object:
        /// the errors grouped, the mods they point at, the launches, and
        /// what changed since the last launch without errors.
        #[qinvokable]
        #[cxx_name = "loadLogs"]
        fn load_logs(self: Pin<&mut LogReader>, previous: bool);

        /// The same for a log RimStack kept, by its path.
        #[qinvokable]
        #[cxx_name = "loadKeptLog"]
        fn load_kept_log(self: Pin<&mut LogReader>, path: &QString);

        /// Copy the game's logs among the kept ones, up to the limit in
        /// the settings; the oldest give way.
        #[qinvokable]
        #[cxx_name = "keepLogs"]
        fn keep_logs(self: &LogReader);

        /// The errors of the report in `logs` that this filter text lists,
        /// as a JSON array; `open_runs` is a JSON array of the runs of
        /// alike errors that are opened, each by where it begins.
        #[qinvokable]
        #[cxx_name = "errorsListed"]
        fn errors_listed(self: &LogReader, filter: &QString, open_runs: &QString) -> QString;

        /// What the filter box holds to list the errors pointing at no mod.
        #[qinvokable]
        #[cxx_name = "noModFilter"]
        fn no_mod_filter(self: &LogReader) -> QString;

        /// Drop the report: the mod folders it names were read again.
        #[qinvokable]
        fn forget(self: Pin<&mut LogReader>);
    }
}

use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

#[derive(Default)]
pub struct LogReaderRust {
    logs: QString,
    logs_busy: bool,
    /// The errors of the report in `logs`, kept for filtering.
    errors: Vec<serde_json::Value>,
}

impl qobject::LogReader {
    fn load_logs(self: Pin<&mut Self>, previous: bool) {
        self.read_log(previous, None);
    }

    fn load_kept_log(self: Pin<&mut Self>, path: &QString) {
        self.read_log(false, Some(PathBuf::from(path.to_string())));
    }

    fn keep_logs(&self) {
        std::thread::spawn(|| log_report::keep_logs(crate::app()));
    }

    fn forget(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().errors = Vec::new();
        self.set_logs(QString::default());
    }

    fn errors_listed(&self, filter: &QString, open_runs: &QString) -> QString {
        let filter = filter.to_string();
        let open: Vec<u64> = serde_json::from_str(&open_runs.to_string()).unwrap_or_default();
        let listed = |error: &&serde_json::Value| {
            let run = error["run"].as_u64();
            rimstack_core::logs::listed(
                &filter,
                error["head"].as_str().unwrap_or_default(),
                error["mod"].as_str(),
                run == error["at"].as_u64(),
                run.is_some_and(|run| open.contains(&run)),
            )
        };
        let shown: Vec<&serde_json::Value> = self.errors.iter().filter(listed).collect();
        QString::from(&serde_json::json!(shown).to_string())
    }

    fn no_mod_filter(&self) -> QString {
        QString::from(rimstack_core::logs::NO_MOD)
    }

    fn read_log(mut self: Pin<&mut Self>, previous: bool, kept: Option<PathBuf>) {
        if self.logs_busy {
            return;
        }
        self.as_mut().set_logs_busy(true);
        let read = move || {
            let report = match &*crate::app().library() {
                Ok((library, _)) => log_report::logs_report(crate::app(), library, previous, kept),
                Err(problem) => serde_json::json!({ "problem": problem }),
            };
            let errors = report["errors"].as_array().cloned().unwrap_or_default();
            (report.to_string(), errors)
        };
        crate::background::run(self.qt_thread(), read, |mut reader, (report, errors)| {
            reader.as_mut().rust_mut().errors = errors;
            reader.as_mut().set_logs(QString::from(&report));
            reader.as_mut().set_logs_busy(false);
        });
    }
}
