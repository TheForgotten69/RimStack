//! `DownloadsScreen.qml`: what is newer, what changed since the game last started, what Steam or
//! Git should fetch, and the download list beside it.

mod changed;
mod screen;
mod side;
mod tasks;
#[cfg(test)]
mod tests;

use super::page::Paging;
use super::text_field::TextField;
use super::theme::Theme;
use crate::changes::Changes;
use crate::jobs::Lane;
use crate::reports::SettingsReport;
use crate::workspace::changed::{ChangeNotes, NotesState, Only, SinceLaunch};
use crate::workspace::download_list::ListShown;
use crate::workspace::downloads::{Queue, Updates};
use gpui::{AppContext, Context, Entity, EventEmitter};
use rimstack_core::base::error::Error;
use rimstack_core::kept::settings::Switch;
use rimstack_core::sources::downloads::Replaceable;
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;
use std::time::Duration;

/// How often the time for an update check is looked at, when updates come by themselves.
const TICK: Duration = Duration::from_secs(60);

/// Asked of the window: show this mod in the Mods tab.
pub struct ShowMod(pub i32);

/// How many active mods have a newer Workshop version, told when it changes.
pub struct BehindActive(pub usize);

/// A task waiting for the one before it: two would write the same SteamCMD folder.
struct Waiting {
    doing: String,
    updating: Option<i32>,
    work: Box<dyn FnOnce() -> crate::workspace::fetch::Done + Send>,
}

#[derive(Default)]
struct Tasks {
    running: bool,
    note: String,
    /// The checkout being updated, by its `ModId::to_packed`.
    updating: Option<i32>,
    /// The repository being cloned, by its place in what was looked up.
    cloning: Option<usize>,
    waiting: VecDeque<Waiting>,
}

struct Read {
    report: SettingsReport,
    replacements: Vec<Replaceable>,
    steamcmd: bool,
    list: ListShown,
}

pub struct Downloads {
    demo: bool,
    report: Rc<SettingsReport>,
    updates: Updates,
    updates_busy: bool,
    asked_of_steam: HashSet<String>,
    steam_waits: bool,
    tried: u64,
    looked_up: Result<Queue, Error>,
    queue_busy: bool,
    pasted: Entity<TextField>,
    changed: SinceLaunch,
    fetched: ChangeNotes,
    since_only: Only,
    since_reading: Lane,
    reading: Lane,
    notes_busy: bool,
    notes_state: Option<NotesState>,
    replacements: Vec<Replaceable>,
    steamcmd: bool,
    list: ListShown,
    tasks: Tasks,
    /// Open parts: the inactive mods' updates, all the changed mods, the mods already installed.
    inactive_open: bool,
    since_open: bool,
    installed_open: bool,
    /// The cards folded against how they start, by their key.
    folded: HashSet<&'static str>,
    paging: Paging,
    side_paging: Paging,
    list_paging: Paging,
    mods: Entity<super::mods::Mods>,
    /// The changed mods' list, reached by Tab: the row its arrows are on, and where each row was
    /// last drawn, to scroll the one reached into view.
    changed_keys: gpui::FocusHandle,
    changed_at: usize,
    changed_drawn: Rc<std::cell::RefCell<Vec<gpui::Bounds<gpui::Pixels>>>>,
}

impl EventEmitter<ShowMod> for Downloads {}
impl EventEmitter<BehindActive> for Downloads {}

impl Downloads {
    /// A Workshop mod's change notes, once fetched.
    pub fn fetched_notes(
        &self,
        workshop_id: &str,
    ) -> Option<&Vec<rimstack_core::sources::changes::Note>> {
        self.fetched.get(workshop_id)
    }

    pub fn notes_busy(&self) -> bool {
        self.notes_busy
    }
}

impl Downloads {
    pub fn new(demo: bool, mods: Entity<super::mods::Mods>, cx: &mut Context<Self>) -> Self {
        let theme = cx.global::<Theme>().clone();
        let pasted = cx.new(|cx| TextField::new(theme.body(), cx));
        cx.observe(&pasted, |_, _, cx| cx.notify()).detach();
        cx.subscribe(
            &pasted,
            |downloads, _, _: &super::text_field::Entered, cx| downloads.look_up(cx),
        )
        .detach();
        cx.observe(&mods, |_, _, cx| cx.notify()).detach();
        let reduce = theme.appearance.reduce_motion;
        let mut downloads = Self {
            demo,
            report: Rc::default(),
            updates: Updates::default(),
            updates_busy: false,
            asked_of_steam: HashSet::new(),
            steam_waits: false,
            tried: 0,
            looked_up: Ok(Queue::default()),
            queue_busy: false,
            pasted,
            changed: SinceLaunch::default(),
            fetched: ChangeNotes::new(),
            since_only: Only::All,
            since_reading: Lane::default(),
            notes_busy: false,
            notes_state: None,
            replacements: Vec::new(),
            steamcmd: false,
            list: ListShown::default(),
            reading: Lane::default(),
            tasks: Tasks::default(),
            inactive_open: false,
            since_open: false,
            installed_open: false,
            folded: HashSet::new(),
            paging: Paging::new(reduce),
            side_paging: Paging::new(reduce),
            list_paging: Paging::new(reduce),
            mods,
            changed_keys: cx.focus_handle().tab_stop(true),
            changed_at: 0,
            changed_drawn: Rc::default(),
        };
        downloads.read(cx);
        if !demo {
            downloads.hear(cx);
            downloads.heard(Changes::LIBRARY | Changes::SETTINGS, cx);
            downloads.keep_time(cx);
        }
        downloads
    }

    /// The `App`'s news, handed to the window's thread as it comes.
    fn hear(&self, cx: &mut Context<Self>) {
        super::hear(cx, Self::heard);
    }

    /// What Downloads shows of the `App` beside the updates and the changes, read off the
    /// window's thread: the library may still be being read. The demo shows no list and no
    /// SteamCMD, as the Qt demo did not ask for them.
    fn read(&mut self, cx: &mut Context<Self>) {
        let app = super::app(cx);
        if self.demo {
            self.report = Rc::new(app.settings_report());
            return;
        }
        self.reading.run(
            "downloads read",
            cx,
            move |_| Read {
                report: app.settings_report(),
                replacements: app.replaceable(),
                steamcmd: app.steamcmd().is_some(),
                list: app.list_shown(),
            },
            |downloads, read, cx| {
                downloads.report = Rc::new(read.report);
                downloads.replacements = read.replacements;
                downloads.steamcmd = read.steamcmd;
                downloads.list = read.list;
                cx.notify();
            },
        );
    }

    fn heard(&mut self, changes: Changes, cx: &mut Context<Self>) {
        if changes.contains(Changes::SETTINGS) {
            self.tick(cx);
        }
        let read = Changes::SETTINGS
            | Changes::LANGUAGE
            | Changes::FOLDERS
            | Changes::LIBRARY
            | Changes::COMMUNITY
            | Changes::DOWNLOAD_LIST;
        if changes.contains(read) {
            self.read(cx);
        }
        if changes.contains(Changes::LIBRARY | Changes::UPDATES) {
            self.restore_updates(cx);
        }
        if changes.contains(Changes::LIBRARY | Changes::UPDATES | Changes::LAUNCHES | Changes::LOGS)
        {
            self.read_since(cx);
        }
        cx.notify();
    }

    /// Every minute, whether an update check is due, when updates come by themselves.
    fn keep_time(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |downloads, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                if downloads
                    .update(cx, |downloads, cx| downloads.tick(cx))
                    .is_err()
                {
                    return;
                }
            }
        })
        .detach();
    }

    fn by_itself(&self, cx: &Context<Self>) -> bool {
        super::app(cx).settings().switch(Switch::AutoUpdates)
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        if !self.by_itself(cx) || self.updates_busy {
            return;
        }
        let every = super::app(cx).settings().update_every_minutes() * 60;
        let last = self.updates.checked.unwrap_or(0).max(self.tried);
        if crate::workspace::now_secs().saturating_sub(last) >= every {
            self.tried = crate::workspace::now_secs();
            self.check_updates(cx);
        }
    }

    fn restore_updates(&mut self, cx: &mut Context<Self>) {
        let app = super::app(cx);
        crate::jobs::run(
            "updates restore",
            cx,
            move || app.kept_updates(),
            |downloads, kept, cx| {
                let Some(mut kept) = kept.filter(|_| !downloads.updates_busy) else {
                    return;
                };
                if kept.checked == downloads.updates.checked {
                    kept.problem = downloads.updates.problem.clone();
                }
                downloads.show_updates(kept, cx);
            },
        );
    }

    fn show_updates(&mut self, updates: Updates, cx: &mut Context<Self>) {
        let behind: HashSet<&String> = updates.items.iter().map(|row| &row.id).collect();
        self.asked_of_steam.retain(|id| behind.contains(id));
        let active = updates.items.iter().filter(|row| row.active).count();
        let was = self.updates.items.iter().filter(|row| row.active).count();
        self.updates = updates;
        if active != was {
            cx.emit(BehindActive(active));
        }
        cx.notify();
    }

    pub fn check_updates(&mut self, cx: &mut Context<Self>) {
        if self.updates_busy || self.demo {
            return;
        }
        self.updates_busy = true;
        let app = super::app(cx);
        crate::jobs::run(
            "updates check",
            cx,
            move || {
                let checked = app.check_updates();
                app.publish(Changes::UPDATES);
                checked
            },
            |downloads, checked, cx| {
                downloads.updates_busy = false;
                match checked {
                    Ok(updates) => {
                        downloads.show_updates(updates, cx);
                        if downloads.by_itself(cx) {
                            downloads.apply(cx);
                        }
                    }
                    Err(problem) => {
                        let failed = Updates {
                            problem: Some(problem.sentence().clone()),
                            ..Updates::default()
                        };
                        downloads.show_updates(failed, cx);
                    }
                }
            },
        );
        cx.notify();
    }

    pub fn look_up(&mut self, cx: &mut Context<Self>) {
        let pasted = self.pasted.read(cx).text().trim().to_string();
        if self.queue_busy || self.demo || pasted.is_empty() {
            return;
        }
        self.queue_busy = true;
        let app = super::app(cx);
        crate::jobs::run(
            "look up",
            cx,
            move || app.look_up(&pasted),
            |downloads, looked_up, cx| {
                downloads.looked_up = looked_up;
                downloads.queue_busy = false;
                cx.notify();
            },
        );
        cx.notify();
    }

    fn read_since(&mut self, cx: &mut Context<Self>) {
        let app = super::app(cx);
        self.since_reading.run(
            "changed since launch",
            cx,
            move |_| app.since_launch(),
            |downloads, changed, cx| {
                downloads.changed = changed;
                cx.notify();
            },
        );
    }

    pub fn keep_setting(&mut self, name: &str, value: &str, cx: &mut Context<Self>) {
        if let Err(problem) = super::app(cx).keep_setting(name, value) {
            rimstack_core::problem!("{problem}");
        }
    }

    pub fn ignore_replacement(&mut self, mod_id: Option<i32>, cx: &mut Context<Self>) {
        let id = mod_id.and_then(rimstack_core::mods::library::ModId::from_packed);
        if let Err(problem) = super::app(cx).ignore_replacement(id) {
            rimstack_core::problem!("{problem}");
        }
    }

    /// Folds or opens the card `key`.
    fn fold(&mut self, key: &'static str, cx: &mut Context<Self>) {
        if !self.folded.remove(key) {
            self.folded.insert(key);
        }
        cx.notify();
    }

    fn is_folded(&self, key: &'static str, starts_folded: bool) -> bool {
        self.folded.contains(key) != starts_folded
    }

    /// The load order's mods its rules ask for that are not installed.
    fn needed(&self, cx: &Context<Self>) -> Vec<crate::workspace::load_order::ProblemSaid> {
        self.mods.read(cx).to_get()
    }
}
