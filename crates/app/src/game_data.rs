//! `Hub.game`: whether the game runs, its last start, and what changed on
//! disk while the window was open, told to every part as a change.

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
        #[qproperty(bool, running, READ, NOTIFY)]
        /// `{ time, errors }`, or null.
        #[qproperty(QVariant, last_start, cxx_name = "lastStart")]
        /// The last launch's list, when the game has emptied its own since.
        #[qproperty(QVariant, reset_by_game, cxx_name = "resetByGame")]
        /// The mod folders cannot be read again now: a reading under way,
        /// a sort on show, a drag, a comparison.
        #[qproperty(bool, held)]
        /// The disk is looked at: the window shows.
        #[qproperty(bool, watching)]
        type GameData = super::GameDataRust;
    }

    impl cxx_qt::Threading for GameData {}

    extern "RustQt" {
        /// `ModsConfig.xml` was written outside RimStack and holds `ids`.
        #[qsignal]
        #[cxx_name = "writtenOutside"]
        fn written_outside(self: Pin<&mut GameData>, ids: QVariant);

        /// Starts the game. `round`: one try of the search for the broken
        /// mod, not a launch to record. Gives the address to open for
        /// Steam to start it, or nothing when it was started here.
        #[qinvokable]
        fn play(self: &GameData, round: bool) -> QString;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[cxx_name = "make_unique"]
        #[namespace = "rust::cxxqtlib1"]
        fn make_game_data() -> UniquePtr<GameData>;
    }
}

use crate::changes::Changes;
use crate::qml::values::to_qml;
use crate::App;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QVariant};
use rimstack_core::base::locks::Held;
use rimstack_core::base::watch::{Stamps, Watcher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
pub struct GameDataRust {
    running: bool,
    last_start: QVariant,
    reset_by_game: QVariant,
    held: bool,
    watching: bool,
    app: Option<Arc<App>>,
    watcher: Watcher,
    /// Read by the thread that looks at the disk.
    looking: Arc<AtomicBool>,
    /// Made-up mods are shown: nothing is recorded or backed up.
    demo: bool,
}

impl qobject::GameData {
    fn app(&self) -> Arc<App> {
        self.app
            .clone()
            .expect("connected by the Hub before QML sees it")
    }

    /// `every`: how often the game and the disk are looked at; never
    /// without.
    pub(crate) fn connect(
        mut self: Pin<&mut Self>,
        app: Arc<App>,
        demo: bool,
        every: Option<Duration>,
    ) {
        let thread = Mutex::new(self.qt_thread());
        app.listen(move |changes| {
            let _ = thread.held().queue(move |me| me.heard(changes));
        });
        let me = self.as_mut().rust_mut().get_mut();
        me.app = Some(app.clone());
        me.demo = demo;
        let looking = me.looking.clone();
        self.as_mut()
            .on_watching_changed(|me| me.looking.store(me.watching, Ordering::Relaxed))
            .release();
        self.as_mut().heard(Changes::LAUNCHES);
        let Some(every) = every else {
            return;
        };
        let thread = self.qt_thread();
        let at_start = app.clone();
        crate::jobs::run(
            "emptied list",
            self.qt_thread(),
            move || at_start.list_reset_by_game(),
            |me, ids| {
                if let Some(ids) = ids {
                    me.set_reset_by_game(to_qml(&ids));
                }
            },
        );
        crate::jobs::spawn("watch", move || loop {
            std::thread::sleep(every);
            let running = app.game_running();
            let stamps = looking.load(Ordering::Relaxed).then(|| app.stamps());
            if thread.queue(move |me| me.looked(running, stamps)).is_err() {
                break;
            }
        });
    }

    fn heard(mut self: Pin<&mut Self>, changes: Changes) {
        if changes.contains(Changes::LAUNCHES | Changes::LOGS | Changes::FOLDERS) && !self.demo {
            let last = to_qml(&self.app().last_start());
            self.as_mut().set_last_start(last);
        }
    }

    fn looked(mut self: Pin<&mut Self>, running: bool, stamps: Option<Stamps>) {
        let app = self.app();
        if running != self.running {
            self.as_mut().rust_mut().get_mut().running = running;
            self.as_mut().running_changed();
            // Whoever started it: what it loads now is what later changes are counted from.
            if running {
                let app = app.clone();
                crate::jobs::spawn("versions at start", move || app.record_versions());
            }
        }
        let Some(stamps) = stamps else {
            return;
        };
        let held = self.held;
        let noticed = self
            .as_mut()
            .rust_mut()
            .get_mut()
            .watcher
            .look(stamps, running, held);
        let mut changes = Changes::NONE;
        for (seen, change) in [
            (noticed.mods, Changes::MOD_FILES),
            (noticed.saves, Changes::SAVES),
            (noticed.log, Changes::LOGS),
            (noticed.theme, Changes::THEME),
            (noticed.language, Changes::LANGUAGE),
        ] {
            if seen {
                changes |= change;
            }
        }
        if !changes.is_empty() {
            app.publish(changes);
        }
        // Never taken unasked: the game empties the file when it first
        // starts on a new version.
        if noticed.mods_config {
            if let Some(ids) = app.mods_config_list() {
                self.written_outside(to_qml(&ids));
            }
        }
    }

    fn play(&self, round: bool) -> QString {
        let app = self.app();
        if !self.demo && !round {
            app.note_launch();
            app.publish(Changes::LAUNCHES);
        }
        match app.start_game() {
            Ok(address) => QString::from(address.as_deref().unwrap_or_default()),
            Err(problem) => {
                rimstack_core::problem!("game not started: {problem}");
                QString::default()
            }
        }
    }
}
