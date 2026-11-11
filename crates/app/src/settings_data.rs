#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;

        include!("cxx-qt-lib/qlist.h");
        type QList_QRect = cxx_qt_lib::QList<cxx_qt_lib::QRect>;

        include!("rimstack-app/src/application.h");
        #[rust_name = "set_font_family"]
        fn setFontFamily(family: &QString) -> QString;
        #[rust_name = "screen_areas"]
        fn screenAreas() -> QList_QRect;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(QVariant, report)]
        #[qproperty(QVariant, folder_checks, cxx_name = "folderChecks")]
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
        /// `{ stored, available, refused, checking, note }`; `note` is null for none.
        #[qproperty(QVariant, steam_key, cxx_name = "steamKey")]
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
        #[qinvokable]
        fn keep(self: Pin<&mut SettingsData>, name: &QString, value: &QString) -> bool;

        /// Judges a typed folder after a pause in the typing, off the window's thread, and uses it when it is fine.
        #[qinvokable]
        #[cxx_name = "checkFolder"]
        fn check_folder(self: Pin<&mut SettingsData>, name: &QString, typed: &QString);

        /// The same judgement for a folder chosen in a dialog.
        #[qinvokable]
        #[cxx_name = "chooseFolder"]
        fn choose_folder(self: Pin<&mut SettingsData>, name: &QString, chosen: &QUrl);

        /// Stores a typed setting after a pause in the typing.
        #[qinvokable]
        #[cxx_name = "keepLater"]
        fn keep_later(self: Pin<&mut SettingsData>, name: &QString, value: &QString);

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

        #[qinvokable]
        fn counted(
            self: &SettingsData,
            n: i32,
            one: &QString,
            many: &QString,
            locale: &QString,
        ) -> QString;

        /// `{ file }` or `{ problem }`.
        #[qinvokable]
        #[cxx_name = "startLanguage"]
        fn start_language(self: &SettingsData, name: &QString) -> QVariant;

        /// `{ file }` or `{ problem }`.
        #[qinvokable]
        #[cxx_name = "startOwnTheme"]
        fn start_own_theme(self: &SettingsData, colours: &QVariant) -> QVariant;

        /// `[{ what, found, needed }]`.
        #[qinvokable]
        #[cxx_name = "shortContrast"]
        fn short_contrast(self: &SettingsData, colours: &QVariant) -> QVariant;

        #[qinvokable]
        #[cxx_name = "useFont"]
        fn use_font(self: Pin<&mut SettingsData>, family: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "checkKey"]
        fn check_key(self: Pin<&mut SettingsData>, typed: &QString);

        #[qinvokable]
        #[cxx_name = "forgetKey"]
        fn forget_key(self: Pin<&mut SettingsData>);

        #[qinvokable]
        #[cxx_name = "downloadCommunity"]
        fn download_community(self: Pin<&mut SettingsData>);

        #[qinvokable]
        #[cxx_name = "checkRelease"]
        fn check_release(self: Pin<&mut SettingsData>);

        #[qinvokable]
        #[cxx_name = "importRimSortRules"]
        fn import_rimsort_rules(self: &SettingsData) -> QString;

        #[qinvokable]
        #[cxx_name = "setModSettingsAside"]
        fn set_mod_settings_aside(self: &SettingsData) -> QString;

        #[qinvokable]
        #[cxx_name = "filesChanged"]
        fn files_changed(self: &SettingsData);

        /// `{ x, y, width, height, visibility }`, read once as the window is made.
        #[qinvokable]
        #[cxx_name = "windowStart"]
        fn window_start(self: &SettingsData) -> QVariant;

        #[qinvokable]
        #[cxx_name = "keepWindow"]
        fn keep_window(
            self: &SettingsData,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            visibility: i32,
        );
    }
}

use crate::App;
use crate::changes::Changes;
use crate::qml::values::{from_qml, to_qml};
use crate::reports::{Answer, WindowStart, Written};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl, QVariant};
use rimstack_core::base::window::Area;
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
pub struct SettingsDataRust {
    report: QVariant,
    folder_checks: QVariant,
    typing: std::collections::HashMap<String, crate::jobs::Lane>,
    folder_verdicts: std::collections::BTreeMap<String, crate::reports::FolderVerdict>,
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
    steam_key: QVariant,
    key_checking: bool,
    key_known: Option<crate::workspace::KeyState>,
    key_failure: Option<rimstack_core::base::sentence::Sentence>,
    key_reading: crate::jobs::Lane,
    app: crate::hub::Given,
}

fn written(file: Result<std::path::PathBuf, rimstack_core::base::error::Error>) -> QVariant {
    let file = file.map(|file| Written {
        file: file.to_string_lossy().into_owned(),
    });
    to_qml(&Answer::from(file))
}

/// Shots, checks and the demo neither open where the window was closed nor keep it.
fn window_unkept() -> bool {
    ["--shot", "--check", "--demo", "--selftest"]
        .iter()
        .any(|flag| crate::hub::given(flag))
}

fn screens() -> Vec<Area> {
    qobject::screen_areas()
        .iter()
        .map(|r| Area {
            x: r.x(),
            y: r.y(),
            width: r.width(),
            height: r.height(),
        })
        .collect()
}

const FOLDER_PAUSE: Duration = Duration::from_millis(400);
const SETTING_PAUSE: Duration = Duration::from_millis(600);

impl qobject::SettingsData {
    fn typing_lane(self: Pin<&mut Self>, name: &str) -> &mut crate::jobs::Lane {
        self.rust_mut()
            .get_mut()
            .typing
            .entry(name.to_string())
            .or_default()
    }

    fn app(&self) -> Arc<App> {
        self.app.app()
    }

    pub(crate) fn connect(mut self: Pin<&mut Self>, app: Arc<App>) {
        crate::hub::hear(&app, self.qt_thread(), Self::heard);
        self.as_mut().rust_mut().app = crate::hub::Given::of(app);
        self.as_mut().on_theme_changed(|me| me.recolour()).release();
        self.heard(
            Changes::SETTINGS
                | Changes::LANGUAGE
                | Changes::THEME
                | Changes::COMMUNITY
                | Changes::KEY,
        );
    }

    fn heard(mut self: Pin<&mut Self>, changes: Changes) {
        let app = self.app();
        let settings = Changes::SETTINGS | Changes::FOLDERS | Changes::LIBRARY | Changes::LOGS;
        if changes.contains(settings) {
            self.as_mut().set_report(to_qml(&app.settings_report()));
        }
        if changes.contains(Changes::SETTINGS | Changes::COMMUNITY | Changes::LANGUAGE) {
            self.as_mut().set_databases(to_qml(&app.databases()));
        }
        if changes.contains(Changes::LANGUAGE) {
            self.as_mut().set_languages(to_qml(&app.languages()));
            self.as_mut().set_words(to_qml(&app.speak()));
        }
        if changes.contains(Changes::THEME) {
            self.as_mut().recolour();
        }
        if changes.contains(Changes::KEY) {
            self.as_mut().read_key();
        } else if changes.contains(Changes::LANGUAGE)
            && let Some(state) = self.key_known.clone()
        {
            self.show_key(state);
        }
    }

    fn read_key(self: Pin<&mut Self>) {
        let app = self.app();
        self.key_reading.run(
            "steam key state",
            self.qt_thread(),
            move |_| app.key_state(),
            |mut me, state| me.as_mut().show_key(state),
        );
    }

    fn show_key(mut self: Pin<&mut Self>, state: crate::workspace::KeyState) {
        let shown = crate::reports::KeyShown::of(
            state.clone(),
            self.key_checking,
            self.key_failure.clone(),
        );
        self.as_mut().rust_mut().get_mut().key_known = Some(state);
        self.as_mut().set_steam_key(to_qml(&shown));
    }

    fn check_key(mut self: Pin<&mut Self>, typed: &QString) {
        if self.key_checking {
            return;
        }
        let typed = typed.to_string();
        let me = self.as_mut().rust_mut().get_mut();
        me.key_checking = true;
        me.key_failure = None;
        if let Some(state) = self.key_known.clone() {
            self.as_mut().show_key(state);
        }
        let app = self.app();
        crate::jobs::run(
            "steam key check",
            self.qt_thread(),
            move || app.check_and_keep(&typed).err(),
            |mut me, failure| {
                let inner = me.as_mut().rust_mut().get_mut();
                inner.key_checking = false;
                inner.key_failure = failure;
                me.read_key();
            },
        );
    }

    fn forget_key(mut self: Pin<&mut Self>) {
        if self.key_checking {
            return;
        }
        let me = self.as_mut().rust_mut().get_mut();
        me.key_checking = true;
        me.key_failure = None;
        let app = self.app();
        crate::jobs::run(
            "steam key forget",
            self.qt_thread(),
            move || app.forget_key().err(),
            |mut me, failure| {
                let inner = me.as_mut().rust_mut().get_mut();
                inner.key_checking = false;
                inner.key_failure = failure;
                me.read_key();
            },
        );
    }

    fn recolour(mut self: Pin<&mut Self>) {
        let colours = self.app().own_colours(&self.theme.to_string());
        self.as_mut().set_own_colours(to_qml(&colours));
    }

    fn keep(mut self: Pin<&mut Self>, name: &QString, value: &QString) -> bool {
        let kept = self
            .app()
            .keep_setting(&name.to_string(), &value.to_string());
        self.as_mut().set_problem(crate::words::why_not(&kept));
        kept.is_ok()
    }

    fn check_folder(mut self: Pin<&mut Self>, name: &QString, typed: &QString) {
        let (name, typed) = (name.to_string(), typed.to_string().trim().to_string());
        let thread = self.qt_thread();
        let lane = self.as_mut().typing_lane(&name);
        let judged = (name.clone(), typed.clone());
        lane.run(
            "folder check",
            thread,
            move |turn| {
                turn.after(FOLDER_PAUSE)
                    .then(|| crate::workspace::judge_folder(&judged.0, &judged.1))
            },
            move |mut me, verdict| {
                let Some(verdict) = verdict else {
                    return;
                };
                if me.app().settings().get(&name).unwrap_or_default() == typed {
                    me.as_mut()
                        .rust_mut()
                        .get_mut()
                        .folder_verdicts
                        .remove(&name);
                } else {
                    if verdict.ok {
                        me.as_mut()
                            .keep(&QString::from(&name), &QString::from(&typed));
                    }
                    me.as_mut()
                        .rust_mut()
                        .get_mut()
                        .folder_verdicts
                        .insert(name, verdict);
                }
                let shown = to_qml(&me.folder_verdicts);
                me.as_mut().set_folder_checks(shown);
            },
        );
    }

    fn choose_folder(self: Pin<&mut Self>, name: &QString, chosen: &QUrl) {
        if let Some(path) = chosen.to_local_file() {
            self.check_folder(name, &path);
        }
    }

    fn keep_later(mut self: Pin<&mut Self>, name: &QString, value: &QString) {
        let (name, value) = (name.to_string(), value.to_string().trim().to_string());
        let thread = self.qt_thread();
        let lane = self.as_mut().typing_lane(&name);
        lane.run(
            "setting typed",
            thread,
            |turn| turn.after(SETTING_PAUSE),
            move |mut me, quiet| {
                let same = me.app().settings().get(&name).unwrap_or_default() == value;
                if quiet && !same {
                    me.as_mut()
                        .keep(&QString::from(&name), &QString::from(&value));
                }
            },
        );
    }

    fn use_database(mut self: Pin<&mut Self>, id: &QString, on: bool) {
        let kept = self.app().use_database(&id.to_string(), on);
        self.as_mut().set_problem(crate::words::why_not(&kept));
    }

    fn version_warnings(mut self: Pin<&mut Self>, choice: i32) {
        let kept = self.app().version_warnings(choice);
        self.as_mut().set_problem(crate::words::why_not(&kept));
    }

    fn counted(&self, n: i32, one: &QString, many: &QString, locale: &QString) -> QString {
        let n = usize::try_from(n).unwrap_or_default();
        let (one, many) = (one.to_string(), many.to_string());
        QString::from(&crate::words::counted(n, &one, &many, &locale.to_string()))
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
        crate::jobs::run(
            "community download",
            self.qt_thread(),
            move || app.download_databases(),
            |mut me, done| {
                me.as_mut().set_community_current(done.is_ok());
                me.as_mut()
                    .set_community_problem(crate::words::why_not(&done));
                me.as_mut().set_community_busy(false);
            },
        );
    }

    fn check_release(mut self: Pin<&mut Self>) {
        if self.release_busy {
            return;
        }
        self.as_mut().set_release_busy(true);
        let nightly = self.app().settings().nightly_updates();
        crate::jobs::run(
            "release check",
            self.qt_thread(),
            move || crate::workspace::newest_release(nightly),
            |mut me, found| {
                me.as_mut().set_release(to_qml(&found));
                me.as_mut().set_release_busy(false);
            },
        );
    }

    fn import_rimsort_rules(&self) -> QString {
        crate::words::said(&self.app().import_rimsort_rules())
    }

    fn set_mod_settings_aside(&self) -> QString {
        crate::words::said(&self.app().set_mod_settings_aside())
    }

    fn files_changed(&self) {
        self.app().publish(Changes::THEME | Changes::LANGUAGE);
    }

    fn window_start(&self) -> QVariant {
        let screens = screens();
        let (area, maximized) = match window_unkept() {
            true => (App::window_unkept(asked_size(), &screens), false),
            false => self.app().window_start(&screens),
        };
        to_qml(&WindowStart::of(area, maximized))
    }

    fn keep_window(&self, x: i32, y: i32, width: i32, height: i32, visibility: i32) {
        let maximized = match visibility {
            _ if window_unkept() => return,
            WindowStart::WINDOWED => false,
            WindowStart::MAXIMIZED => true,
            _ => return,
        };
        let area = Area {
            x,
            y,
            width,
            height,
        };
        if let Err(problem) = self.app().keep_window(area, maximized) {
            rimstack_core::problem!("window not kept: {problem}");
        }
    }
}

/// `--size=1000x560`.
fn asked_size() -> Option<(i32, i32)> {
    let given = crate::hub::value("--size")?;
    let (width, height) = given.split_once('x')?;
    Some((width.parse().ok()?, height.parse().ok()?))
}
