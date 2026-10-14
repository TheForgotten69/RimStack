//! The Saves screen's own object: lists the saves against the load order
//! being edited, and keeps copies of the newest.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, saves)]
        #[qproperty(bool, saves_busy, cxx_name = "savesBusy")]
        type SaveReader = super::SaveReaderRust;
    }

    impl cxx_qt::Threading for SaveReader {}

    extern "RustQt" {
        /// List the saves and compare each with the load order being
        /// edited. Fills `saves` with a JSON object: the saves, newest
        /// first, and the RimSaves folders if that mod is in use.
        #[qinvokable]
        #[cxx_name = "loadSaves"]
        fn load_saves(self: Pin<&mut SaveReader>);

        /// Copy the newest save into RimStack's data folder, keeping the
        /// last five copies. Returns what went wrong, or nothing.
        #[qinvokable]
        #[cxx_name = "backUpNewestSave"]
        fn back_up_newest_save(self: &SaveReader) -> QString;

        /// Drop what was worked out from the mod folders as they were read
        /// before; the screen asks again when it is next shown.
        #[qinvokable]
        fn forget(self: Pin<&mut SaveReader>);
    }
}

use crate::backend::now_secs;
use crate::backend::save_report::{save_row, saves_dir};
use core::pin::Pin;
use cxx_qt::Threading;
use cxx_qt_lib::QString;
use rimstack_core::{rimsaves, saves};
use serde_json::json;

#[derive(Default)]
pub struct SaveReaderRust {
    saves: QString,
    saves_busy: bool,
}

impl qobject::SaveReader {
    fn load_saves(mut self: Pin<&mut Self>) {
        if self.saves_busy {
            return;
        }
        self.as_mut().set_saves_busy(true);
        let listed = || {
            let rows: Vec<saves::Row> = match (&*crate::app().library(), saves_dir(crate::app())) {
                (Ok((library, _)), Some(dir)) => {
                    let list = crate::app().session().current().to_vec();
                    let found = saves::list(&dir);
                    found
                        .iter()
                        .map(|s| save_row(crate::app(), s, library, &list))
                        .collect()
                }
                _ => Vec::new(),
            };
            // RimSaves' folders, when the mod has been used: those it
            // lists, and any that only show up in a file name.
            let config = crate::app().paths().config.clone().unwrap_or_default();
            let kept = rimsaves::settings(&config);
            let mut folders = kept.as_ref().map_or(Vec::new(), |s| s.folders.clone());
            for row in &rows {
                if !folders.contains(&row.folder) {
                    folders.push(row.folder.clone());
                }
            }
            let versions: Vec<(&str, bool)> = rows
                .iter()
                .map(|row| match &row.compared {
                    Some(compared) => (compared.game_version.as_str(), compared.other_game),
                    None => ("", false),
                })
                .collect();
            json!({
                "saves": rows,
                "allFrom": saves::version_all_share(&versions),
                "rimsaves": kept.is_some(),
                "folders": folders,
                "currentFolder": kept.map(|s| s.current),
            })
            .to_string()
        };
        crate::background::run(self.qt_thread(), listed, |mut reader, saves| {
            reader.as_mut().set_saves(QString::from(&saves));
            reader.as_mut().set_saves_busy(false);
        });
    }

    fn back_up_newest_save(&self) -> QString {
        let Some(dir) = saves_dir(crate::app()) else {
            return QString::default();
        };
        // `list` is newest first.
        let Some(newest) = saves::list(&dir).into_iter().next() else {
            return QString::default();
        };
        let backups = crate::app().data_dir().join("save-backups");
        match saves::back_up(&newest.path, &backups, now_secs(), 5) {
            Ok(_) => QString::default(),
            Err(problem) => QString::from(&problem.to_string()),
        }
    }

    fn forget(mut self: Pin<&mut Self>) {
        self.as_mut().set_saves(QString::default());
    }
}
