//! What the window and the other sections ask of the Mods tab, and tell it.

use super::Mods;
use crate::workspace::load_order::Side;
use gpui::Context;
use std::rc::Rc;

impl Mods {
    /// What Play warns of: how many things are in the way, and whether a mod has errors.
    pub fn in_the_way(&self, cx: &gpui::App) -> (usize, bool) {
        if self.demo {
            return (0, false);
        }
        let (errors, _) = self.order.errors_and_warnings();
        let dirty = crate::ui::app(cx).session().is_dirty();
        let ways =
            usize::from(errors > 0) + usize::from(self.sort_pending > 0) + usize::from(dirty);
        (ways, errors > 0)
    }

    /// The last start of the game: when, and how many errors its log held.
    pub fn last_start(&self) -> Option<(u64, Option<usize>)> {
        self.last_start
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
    pub fn show_mod(&mut self, mod_id: i32, cx: &mut Context<Self>) {
        for side in [Side::Active, Side::Inactive] {
            let at = self
                .shown(side)
                .rows
                .iter()
                .position(|row| row.mod_id == mod_id && !row.is_header);
            if let Some(at) = at {
                self.choose(side, mod_id, cx);
                self.list(side).update(cx, |list, cx| list.reveal(at, cx));
                return;
            }
        }
    }

    /// The mod chosen first, as `Mods.selectedId` gave it.
    pub fn selected_id(&self) -> Option<i32> {
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

    /// How many active mods Downloads found behind their Workshop version.
    pub fn set_behind(&mut self, behind: usize, cx: &mut Context<Self>) {
        self.behind = behind;
        self.sync_details(cx);
        cx.notify();
    }

    pub fn filter_by(&mut self, text: &str, cx: &mut Context<Self>) {
        self.filter.update(cx, |field, cx| field.set_text(text, cx));
    }

    pub fn filter_text(&self, cx: &gpui::App) -> String {
        self.filter.read(cx).text().to_string()
    }
}
