//! The community databases: which there are, and downloading them.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, rules_busy, cxx_name = "rulesBusy")]
        #[qproperty(bool, rules_current, cxx_name = "rulesCurrent")]
        #[qproperty(QString, rules_problem, cxx_name = "rulesProblem")]
        type CommunityData = super::CommunityDataRust;
    }

    impl cxx_qt::Threading for CommunityData {}

    extern "RustQt" {
        /// Download the databases that are switched on, in the background.
        /// `rulesBusy` is true meanwhile; `rulesProblem` says why if it
        /// failed. What the lists show is checked again when it ends.
        #[qinvokable]
        #[cxx_name = "downloadRules"]
        fn download_rules(self: Pin<&mut CommunityData>);

        /// The community databases, as a JSON array of
        /// `{ name, use, updated }`; `updated` is 0 for one not downloaded.
        #[qinvokable]
        fn databases(self: &CommunityData) -> QString;
    }
}

use core::pin::Pin;
use cxx_qt::Threading;
use cxx_qt_lib::QString;
use rimstack_core::base::locks::Shared;
use rimstack_core::base::translate::named;
use rimstack_core::{mods::community, rimworld::workshopdb, sources::replacements};

#[derive(Default)]
pub struct CommunityDataRust {
    rules_busy: bool,
    /// Downloaded while this window was open, so known to be the latest.
    rules_current: bool,
    rules_problem: QString,
}

/// Downloads every database that is in use and puts each to use. What
/// comes back is the first failure, the one shown; each is in the record.
fn download_all(app: &crate::App) -> String {
    let data = app.data_dir();
    let mut failed: Vec<rimstack_core::base::error::Error> = Vec::new();
    if app.database_on("rules") {
        match community::download(&community::file_in(data)) {
            Ok(fresh) => *app.community().writing() = Some(fresh),
            Err(why) => failed.push(why),
        }
    }
    let paths = app.paths();
    let version = paths.game_version.as_deref();
    if let Some(version) = version.filter(|_| app.database_on("worksOn")) {
        match community::download_works_on(version, &community::works_on_file(data, version)) {
            Ok(fresh) => *app.works_on().writing() = fresh,
            Err(why) => failed.push(why),
        }
    }
    if app.database_on("replacements") {
        failed.extend(replacements::download(&replacements::file_in(data)).err());
    }
    if app.database_on("names") {
        match workshopdb::download(&workshopdb::file_in(data)) {
            Ok(fresh) => *app.workshop_names().writing() = fresh,
            Err(why) => failed.push(why),
        }
    }
    for why in &failed {
        rimstack_core::problem!("{why}");
    }
    failed
        .into_iter()
        .next()
        .map(String::from)
        .unwrap_or_default()
}

impl qobject::CommunityData {
    fn download_rules(mut self: Pin<&mut Self>) {
        if self.rules_busy {
            return;
        }
        self.as_mut().set_rules_busy(true);
        self.as_mut().set_rules_problem(QString::default());
        crate::background::run(
            self.qt_thread(),
            || download_all(crate::app()),
            |mut data, problem| {
                data.as_mut().set_rules_current(problem.is_empty());
                data.as_mut().set_rules_problem(QString::from(&problem));
                data.as_mut().set_rules_busy(false);
            },
        );
    }

    /// Each database as `{ id, name, use, updated, on }`: `updated` in
    /// seconds since 1970, 0 for one not downloaded; `on` unless the
    /// settings leave it out.
    fn databases(&self) -> QString {
        let data = crate::app().data_dir();
        let version = crate::app()
            .paths()
            .game_version
            .clone()
            .unwrap_or_default();
        let all = [
            (
                "rules",
                "Community Rules Database",
                named("Load-order rules, for Sort"),
                community::file_in(data),
            ),
            (
                "worksOn",
                "No Version Warning",
                named("Mods that work on this game version without saying so"),
                community::works_on_file(data, &version),
            ),
            (
                "replacements",
                "Use This Instead",
                named("Maintained replacements for abandoned mods"),
                replacements::file_in(data),
            ),
            (
                "names",
                "Steam Workshop Database",
                named("Names of mods that are not installed"),
                workshopdb::file_in(data),
            ),
        ];
        let rows: Vec<crate::reports::Database> = all
            .iter()
            .map(|(id, name, used_for, file)| crate::reports::Database {
                id,
                name,
                used_for,
                updated: rimstack_core::base::watch::changed_at(file) / 1000,
                on: crate::app().database_on(id),
            })
            .collect();
        crate::reports::said(&rows)
    }
}
