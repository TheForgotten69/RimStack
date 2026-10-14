//! `Hub.settings`: the Settings screen, the first-run card, the language
//! and the user's own colours.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;

        include!("rimstack-app/src/application.h");
        #[rust_name = "set_font_family"]
        fn setFontFamily(family: &QString) -> QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(QVariant, report)]
        #[qproperty(QVariant, databases)]
        #[qproperty(QVariant, languages)]
        #[qproperty(QVariant, words)]
        #[qproperty(QString, theme)]
        #[qproperty(QVariant, own_colours, cxx_name = "ownColours")]
        #[qproperty(bool, community_busy, cxx_name = "communityBusy")]
        #[qproperty(bool, community_current, cxx_name = "communityCurrent")]
        #[qproperty(QString, community_problem, cxx_name = "communityProblem")]
        #[qproperty(QVariant, release)]
        #[qproperty(bool, release_busy, cxx_name = "releaseBusy")]
        #[qproperty(QString, problem)]
        type SettingsData = super::SettingsDataRust;
    }

    impl cxx_qt::Threading for SettingsData {}

    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[cxx_name = "make_unique"]
        #[namespace = "rust::cxxqtlib1"]
        fn make_settings_data() -> UniquePtr<SettingsData>;
    }

    extern "RustQt" {
        /// Keeps a setting; `problem` says why it was not.
        #[qinvokable]
        fn keep(self: Pin<&mut SettingsData>, name: &QString, value: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "useDatabase"]
        fn use_database(self: Pin<&mut SettingsData>, id: &QString, on: bool);

        /// 0 warn about the game version, 1 not for the version installed, 2 never.
        #[qinvokable]
        #[cxx_name = "versionWarnings"]
        fn version_warnings(self: Pin<&mut SettingsData>, choice: i32);

        #[qinvokable]
        #[cxx_name = "useLanguage"]
        fn use_language(self: Pin<&mut SettingsData>, name: &QString);

        /// `{ file }` of the language's own file, written when it is not
        /// there, or `{ problem }`.
        #[qinvokable]
        #[cxx_name = "startLanguage"]
        fn start_language(self: &SettingsData, name: &QString) -> QVariant;

        /// `{ file }` of `theme.json`, given these colours for the theme
        /// in use when it has none for it, or `{ problem }`.
        #[qinvokable]
        #[cxx_name = "startOwnTheme"]
        fn start_own_theme(self: &SettingsData, colours: &QVariant) -> QVariant;

        /// Of the colours on show, `{ name: "#rrggbb" }`, the pairs under
        /// their contrast: `[{ what, found, needed }]`.
        #[qinvokable]
        #[cxx_name = "shortContrast"]
        fn short_contrast(self: &SettingsData, colours: &QVariant) -> QVariant;

        /// Keeps the font and uses it for new texts; gives the family in use.
        #[qinvokable]
        #[cxx_name = "useFont"]
        fn use_font(self: Pin<&mut SettingsData>, family: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "downloadCommunity"]
        fn download_community(self: Pin<&mut SettingsData>);

        /// Asks GitHub for a newer RimStack; fills `release`.
        #[qinvokable]
        #[cxx_name = "checkRelease"]
        fn check_release(self: Pin<&mut SettingsData>);

        #[qinvokable]
        #[cxx_name = "importRimSortRules"]
        fn import_rimsort_rules(self: &SettingsData) -> QString;

        #[qinvokable]
        #[cxx_name = "setModSettingsAside"]
        fn set_mod_settings_aside(self: &SettingsData) -> QString;

        /// Its `theme.json` or the language's file changed on disk.
        #[qinvokable]
        #[cxx_name = "filesChanged"]
        fn files_changed(self: &SettingsData);
    }
}

use crate::changes::Changes;
use crate::qml::values::{from_qml, to_qml};
use crate::reports::{Answer, Written};
use crate::App;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QVariant};
use rimstack_core::base::locks::Held;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct SettingsDataRust {
    report: QVariant,
    databases: QVariant,
    languages: QVariant,
    words: QVariant,
    theme: QString,
    own_colours: QVariant,
    community_busy: bool,
    community_current: bool,
    community_problem: QString,
    release: QVariant,
    release_busy: bool,
    problem: QString,
    app: Option<Arc<App>>,
}

fn written(file: Result<std::path::PathBuf, rimstack_core::base::error::Error>) -> QVariant {
    let file = file.map(|file| Written {
        file: file.to_string_lossy().into_owned(),
    });
    to_qml(&Answer::from(file))
}

impl qobject::SettingsData {
    fn app(&self) -> Arc<App> {
        self.app
            .clone()
            .expect("connected by the Hub before QML sees it")
    }

    /// Given its `App` by the Hub, before QML sees it.
    pub(crate) fn connect(mut self: Pin<&mut Self>, app: Arc<App>) {
        let thread = Mutex::new(self.qt_thread());
        app.listen(move |changes| {
            let _ = thread.held().queue(move |me| me.heard(changes));
        });
        self.as_mut().rust_mut().app = Some(app);
        self.as_mut().on_theme_changed(|me| me.recolour()).release();
        self.heard(Changes::SETTINGS | Changes::LANGUAGE | Changes::THEME | Changes::COMMUNITY);
    }

    fn heard(mut self: Pin<&mut Self>, changes: Changes) {
        let app = self.app();
        let settings = Changes::SETTINGS | Changes::FOLDERS | Changes::LIBRARY | Changes::LOGS;
        if changes.contains(settings) {
            self.as_mut().set_report(to_qml(&app.settings_report()));
        }
        if changes.contains(Changes::SETTINGS | Changes::COMMUNITY) {
            self.as_mut().set_databases(to_qml(&app.databases()));
        }
        if changes.contains(Changes::LANGUAGE) {
            self.as_mut().set_languages(to_qml(&app.languages()));
            self.as_mut().set_words(to_qml(&app.speak()));
        }
        if changes.contains(Changes::THEME) {
            self.recolour();
        }
    }

    fn recolour(mut self: Pin<&mut Self>) {
        let colours = self.app().own_colours(&self.theme.to_string());
        self.as_mut().set_own_colours(to_qml(&colours));
    }

    fn keep(mut self: Pin<&mut Self>, name: &QString, value: &QString) -> bool {
        let kept = self
            .app()
            .keep_setting(&name.to_string(), &value.to_string());
        let problem = kept.as_ref().err().map(ToString::to_string);
        self.as_mut()
            .set_problem(QString::from(&problem.unwrap_or_default()));
        kept.is_ok()
    }

    fn use_database(mut self: Pin<&mut Self>, id: &QString, on: bool) {
        let kept = self.app().use_database(&id.to_string(), on);
        let problem = kept.err().map(|e| e.to_string()).unwrap_or_default();
        self.as_mut().set_problem(QString::from(&problem));
    }

    fn version_warnings(mut self: Pin<&mut Self>, choice: i32) {
        let kept = self.app().version_warnings(choice);
        let problem = kept.err().map(|e| e.to_string()).unwrap_or_default();
        self.as_mut().set_problem(QString::from(&problem));
    }

    fn use_language(self: Pin<&mut Self>, name: &QString) {
        self.keep(&QString::from("language"), name);
    }

    fn start_language(&self, name: &QString) -> QVariant {
        written(self.app().start_language(&name.to_string()))
    }

    fn start_own_theme(&self, colours: &QVariant) -> QVariant {
        let colours = from_qml::<serde_json::Value>(colours).unwrap_or_default();
        written(
            self.app()
                .start_own_theme(&self.theme.to_string(), &colours),
        )
    }

    fn short_contrast(&self, colours: &QVariant) -> QVariant {
        let colours = from_qml::<serde_json::Map<String, serde_json::Value>>(colours);
        to_qml(&crate::workspace::short_contrast(
            &colours.unwrap_or_default(),
        ))
    }

    fn use_font(self: Pin<&mut Self>, family: &QString) -> QString {
        self.keep(&QString::from("font"), family);
        qobject::set_font_family(family)
    }

    fn download_community(mut self: Pin<&mut Self>) {
        if self.community_busy {
            return;
        }
        self.as_mut().set_community_busy(true);
        self.as_mut().set_community_problem(QString::default());
        let app = self.app();
        crate::background::run(
            self.qt_thread(),
            move || app.download_databases().err().map(String::from),
            |mut me, problem| {
                me.as_mut().set_community_current(problem.is_none());
                me.as_mut()
                    .set_community_problem(QString::from(&problem.unwrap_or_default()));
                me.as_mut().set_community_busy(false);
            },
        );
    }

    fn check_release(mut self: Pin<&mut Self>) {
        if self.release_busy {
            return;
        }
        self.as_mut().set_release_busy(true);
        crate::background::run(
            self.qt_thread(),
            crate::workspace::newest_release,
            |mut me, found| {
                me.as_mut().set_release(to_qml(&found));
                me.as_mut().set_release_busy(false);
            },
        );
    }

    fn import_rimsort_rules(&self) -> QString {
        QString::from(&self.app().import_rimsort_rules())
    }

    fn set_mod_settings_aside(&self) -> QString {
        QString::from(&self.app().set_mod_settings_aside())
    }

    fn files_changed(&self) {
        self.app().publish(Changes::THEME | Changes::LANGUAGE);
    }
}
