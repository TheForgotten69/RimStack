//! `GameData` as the window held it: whether the game runs, what changed on disk while the window
//! is open, and the game started.

use crate::changes::Changes;
use gpui::{Context, EventEmitter};
use rimstack_core::base::watch::{Stamps, Watcher};
use std::time::Duration;

/// What the window is told of the game and of the files it writes.
pub enum Told {
    /// The game emptied its list on a new version; the last launch from RimStack had these.
    ResetByGame(Vec<String>),
    /// ModsConfig.xml was written outside RimStack and now lists these.
    WrittenOutside(Vec<String>),
    /// The game could not be started, and why.
    NotStarted(String),
}

pub struct Game {
    running: bool,
    starting: bool,
    watcher: Watcher,
    demo: bool,
    /// What is under way on the load order: a change seen on disk waits for it.
    held: Box<dyn Fn(&gpui::App) -> bool>,
}

impl EventEmitter<Told> for Game {}

/// How often the disk is looked at; `None` never, as for the demo and for pictures.
fn every(demo: bool, still: bool) -> Option<Duration> {
    (!demo && !still).then_some(Duration::from_secs(2))
}

impl Game {
    pub fn new(
        demo: bool,
        still: bool,
        held: impl Fn(&gpui::App) -> bool + 'static,
        cx: &mut Context<Self>,
    ) -> Self {
        let game = Self {
            running: false,
            starting: false,
            watcher: Watcher::default(),
            demo,
            held: Box::new(held),
        };
        if let Some(every) = every(demo, still) {
            game.watch(every, cx);
        }
        game
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn starting(&self) -> bool {
        self.starting
    }

    fn watch(&self, every: Duration, cx: &mut Context<Self>) {
        let app = super::app(cx);
        let at_start = app.clone();
        crate::jobs::run(
            "emptied list",
            cx,
            move || at_start.list_reset_by_game(),
            |_, ids, cx| {
                if let Some(ids) = ids {
                    cx.emit(Told::ResetByGame(ids));
                }
            },
        );
        cx.spawn(async move |game, cx| {
            loop {
                cx.background_executor().timer(every).await;
                let app = app.clone();
                let looked =
                    crate::jobs::off_thread("watch", move || (app.game_running(), app.stamps()));
                let Ok((running, stamps)) = looked.await else {
                    return;
                };
                if game
                    .update(cx, |game, cx| game.looked(running, stamps, cx))
                    .is_err()
                {
                    return;
                }
            }
        })
        .detach();
    }

    fn looked(&mut self, running: bool, stamps: Stamps, cx: &mut Context<Self>) {
        let app = super::app(cx);
        if running != self.running {
            self.running = running;
            if running {
                let app = app.clone();
                crate::jobs::spawn("versions at start", move || app.record_versions());
            }
            cx.notify();
        }
        let held = (self.held)(cx);
        let noticed = self.watcher.look(stamps, running, held);
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
        // Never taken unasked: the game empties the file when it first starts on a new version.
        if noticed.mods_config
            && let Some(ids) = app.mods_config_list()
        {
            cx.emit(Told::WrittenOutside(ids));
        }
    }

    /// Starts the game; `round` for a round of the search for a broken mod, which is not recorded.
    pub fn play(&mut self, round: bool, cx: &mut Context<Self>) {
        if self.starting || self.running {
            return;
        }
        self.starting = true;
        let app = super::app(cx);
        let record = !self.demo && !round;
        crate::jobs::run(
            "game start",
            cx,
            move || app.launch(record),
            move |game, started, cx| {
                game.starting = false;
                match started {
                    Ok(address) => {
                        if let Some(address) = address {
                            cx.open_url(&address);
                        }
                        if record {
                            super::app(cx).publish(Changes::LAUNCHES);
                        }
                    }
                    Err(problem) => {
                        rimstack_core::problem!("game not started: {problem}");
                        cx.emit(Told::NotStarted(crate::words::said_of(&problem)));
                    }
                }
                cx.notify();
            },
        );
        cx.notify();
    }
}
