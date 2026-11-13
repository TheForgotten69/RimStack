//! What the window and the other sections ask of the Mods tab, and tell it.

use super::Mods;
use crate::workspace::load_order::Side;
use gpui::Context;
use rimstack_core::mods::library::ModId;
use std::rc::Rc;

impl Mods {
    /// What Play warns of: how many things are in the way, and whether a mod has errors.
    pub fn in_the_way(&self, cx: &gpui::App) -> (usize, bool) {
        if self.demo {
            return (0, false);
        }
        let (errors, _) = self.order.errors_and_warnings();
        let dirty = crate::ui::app(cx)
            .session_if_read()
            .is_some_and(|session| session.is_dirty());
        let ways =
            usize::from(errors > 0) + usize::from(self.sort_pending > 0) + usize::from(dirty);
        (ways, errors > 0)
    }

    /// The last start of the game: when, and how many errors its log held.
    pub fn last_start(&self) -> Option<(u64, Option<usize>)> {
        self.reads
            .last_start
            .as_ref()
            .map(|said| (said.start.time, said.start.errors))
    }

    /// The mods the load order's rules ask for that are not installed.
    pub fn to_get(&self) -> Vec<crate::workspace::load_order::ProblemSaid> {
        self.overview
            .as_ref()
            .map(|o| o.to_get.clone())
            .unwrap_or_default()
    }

    /// Chooses a mod wherever it is and brings its row into view.
    pub fn show_mod(&mut self, mod_id: ModId, cx: &mut Context<Self>) {
        for side in [Side::Active, Side::Inactive] {
            let at = self
                .shown(side)
                .rows
                .iter()
                .position(|row| row.mod_id == Some(mod_id) && !row.is_header);
            if let Some(at) = at {
                if side == Side::Inactive && self.view.inactive_hidden {
                    self.set_hidden(false, cx);
                }
                self.choose(side, mod_id, cx);
                self.list(side).update(cx, |list, cx| list.reveal(at, cx));
                return;
            }
        }
    }

    /// The mod chosen first, as `Mods.selectedId` gave it.
    pub fn selected_id(&self) -> Option<ModId> {
        self.selection.first().copied()
    }

    pub fn game_version(&self) -> String {
        self.overview
            .as_ref()
            .map(|o| o.game_version.clone())
            .unwrap_or_default()
    }

    /// How many mods of a list the filter lets through.
    pub fn matching(&self, side: Side) -> usize {
        self.order.list(side).matching()
    }

    pub fn start_bench(&mut self, cx: &mut Context<Self>) {
        self.active_list.update(cx, |list, cx| list.start_bench(cx));
    }

    pub fn overview(&self) -> Option<Rc<crate::workspace::load_order::Overview>> {
        self.overview.clone()
    }

    /// Marks the mods as working on this game version, or takes the mark off.
    pub fn trust(
        &mut self,
        ids: Vec<rimstack_core::mods::library::ModId>,
        yes: bool,
        cx: &mut Context<Self>,
    ) {
        let app = crate::ui::app(cx);
        app.set_version_ok(&ids, yes);
        self.order.recheck(&app);
        self.show(cx);
    }

    /// The load order becomes `ids`, unsaved; the history keeps the list it was.
    pub fn use_list(&mut self, ids: Vec<String>, cx: &mut Context<Self>) {
        let app = crate::ui::app(cx);
        let before = app.snapshot("@list", app.session().current());
        let count = ids.len();
        self.order.use_list(&app, ids);
        self.changed(cx);
        if !self.demo {
            use rimstack_core::say_n;
            let taken = say_n!(
                count,
                "Another list taken: {} mod",
                "Another list taken: {} mods"
            );
            app.note_change(taken, before.as_deref());
            let said = say_n!(count, "Using a list of {} mod", "Using a list of {} mods");
            self.done_said(said, cx);
        }
    }

    /// The load order as read, for the first run: its active and inactive mods, or why it was not.
    pub fn read_as(&self) -> (usize, usize, String) {
        (
            self.count(Side::Active),
            self.count(Side::Inactive),
            crate::words::put(&self.order.problem),
        )
    }

    /// Whether the active mods' files are still being read.
    pub fn reading_files(&self) -> bool {
        !self.files_read && self.count(Side::Active) > 0 && !self.demo
    }

    /// The changes not saved, in words; empty when the list is saved.
    pub fn unsaved(&self, cx: &gpui::App) -> Option<String> {
        if self.demo {
            return None;
        }
        let app = crate::ui::app(cx);
        let session = app.session();
        (session.is_dirty() && self.order.is_installed())
            .then(|| crate::words::put(&session.changes().said()))
    }

    /// The active mods as text, one a line, as `Copy the load order` gives them.
    pub fn listing(&self, cx: &gpui::App) -> String {
        use rimstack_core::mods::share::{Entry, listing};
        let app = crate::ui::app(cx);
        let loaded = app.library();
        let Ok((library, _)) = &*loaded else {
            return String::new();
        };
        let mods = self.order.list(Side::Active).mods().iter();
        let entries: Vec<Entry> = mods
            .filter_map(|m| library.get(m.installed?))
            .map(Entry::of)
            .collect();
        let paths = app.paths();
        let version = paths
            .full_game_version
            .as_deref()
            .unwrap_or(&library.game_version);
        listing(version, &entries)
    }

    /// Whether the search for the broken mod runs, as the window tells it.
    pub fn set_hunting(&mut self, hunting: bool, cx: &mut Context<Self>) {
        self.hunting = hunting;
        self.sync_details(cx);
    }

    /// The load order becomes `ids`, unsaved, nothing kept nor said: a round of the search.
    pub fn use_list_quietly(&mut self, ids: Vec<String>, cx: &mut Context<Self>) {
        let app = crate::ui::app(cx);
        self.order.use_list(&app, ids);
        self.changed(cx);
    }

    /// Whether the inactive mods are folded into the rail.
    pub fn inactive_hidden(&self) -> bool {
        self.view.inactive_hidden
    }

    pub fn toggle_hidden(&mut self, cx: &mut Context<Self>) {
        let hidden = !self.view.inactive_hidden;
        self.set_hidden(hidden, cx);
    }

    /// Whether the inactive mods are shown by category, their own order.
    pub fn inactive_by_category(&self) -> bool {
        self.view.inactive_order.is_empty()
    }

    /// The inactive mods by name, or by category again, as the command list offers.
    pub fn toggle_inactive_order(&mut self, cx: &mut Context<Self>) {
        let then = if self.inactive_by_category() {
            "name"
        } else {
            ""
        };
        self.order_inactive(then, cx);
    }

    /// `act` after the next draw, and after each one until it answers that it was done.
    pub fn when_drawn(&mut self, act: super::AfterDrawn, cx: &mut Context<Self>) {
        self.when_drawn.push(act);
        cx.notify();
    }

    pub(super) fn run_when_drawn(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        if self.when_drawn.is_empty() {
            return;
        }
        let waiting = std::mem::take(&mut self.when_drawn);
        cx.defer_in(window, move |mods, window, cx| {
            for mut act in waiting {
                if !act(mods, window, cx) {
                    mods.when_drawn.push(act);
                }
            }
            if !mods.when_drawn.is_empty() {
                cx.notify();
            }
        });
    }

    pub fn files_read(&self) -> bool {
        self.files_read
    }

    /// Downloads, where a mod's change notes are fetched: the details redraw as it changes.
    pub fn set_downloads(
        &mut self,
        downloads: &gpui::Entity<crate::ui::downloads::Downloads>,
        cx: &mut Context<Self>,
    ) {
        let weak = downloads.downgrade();
        let observed = downloads.clone();
        self.details_view.update(cx, |view, cx| {
            view.downloads = Some(weak);
            cx.observe(&observed, |_, _, cx| cx.notify()).detach();
        });
    }
}
