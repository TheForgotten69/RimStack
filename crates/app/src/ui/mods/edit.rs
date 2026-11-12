//! The load order changed from the Mods tab, as `Mods.qml` and the `ModsData` commands did:
//! activating and moving mods, Undo and Redo, Save, Sort, and choosing several mods.

use super::Mods;
use crate::changes::Changes;
use crate::ui::notices;
use crate::words::put;
use crate::workspace::load_order::Side;
use gpui::{Context, KeyDownEvent, Window};
use rimstack_core::base::sentence::Sentence;
use rimstack_core::mods::library::ModId;
use rimstack_core::rimworld::modsconfig;
use rimstack_core::{say, say_n};

/// What is done once a save asked for ends well.
pub type AfterSave = Box<dyn FnOnce(&mut Mods, &mut Context<Mods>)>;

impl Mods {
    /// The lists shown again and the other screens told, after the load order changed.
    pub(super) fn changed(&mut self, cx: &mut Context<Self>) {
        self.follow_selection();
        self.show(cx);
        if !self.demo {
            super::super::app(cx).publish(Changes::LOAD_ORDER);
        }
    }

    /// A toast that says what was done, with Undo.
    fn done_said(&self, said: Sentence, cx: &mut Context<Self>) {
        if self.demo {
            return;
        }
        let me = cx.entity().downgrade();
        notices::offer(
            put(&said),
            put(&say!("Undo")),
            move |_, cx| {
                let _ = me.update(cx, |mods, cx| mods.undo(cx));
            },
            cx,
        );
    }

    fn ids(&self) -> Vec<ModId> {
        self.selection
            .iter()
            .filter_map(|id| ModId::from_qml(*id))
            .collect()
    }

    pub(super) fn move_mod(&mut self, side: Side, from: usize, to: usize, cx: &mut Context<Self>) {
        let app = super::super::app(cx);
        self.order.move_mod(&app, side, from, to);
        self.changed(cx);
    }

    /// The mod at `index` of `from` moved to the other list, at `to` or where it belongs.
    pub(super) fn take(
        &mut self,
        from: Side,
        index: usize,
        to: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        let Some(name) = self
            .order
            .list(from)
            .mods()
            .get(index)
            .map(|m| m.name.clone())
        else {
            return;
        };
        let app = super::super::app(cx);
        self.order.take(&app, from, index, to);
        self.changed(cx);
        let said = match from {
            Side::Inactive => say!("Activated {}", name),
            Side::Active => say!("Deactivated {}", name),
        };
        self.done_said(said, cx);
    }

    fn take_ids(&mut self, from: Side, ids: &[ModId], to: Option<usize>, cx: &mut Context<Self>) {
        if ids.is_empty() {
            return;
        }
        let app = super::super::app(cx);
        self.order.take_all(&app, from, ids, to);
        self.changed(cx);
        let said = match from {
            Side::Inactive => say_n!(ids.len(), "Activated {} mod", "Activated {} mods"),
            Side::Active => say_n!(ids.len(), "Deactivated {} mod", "Deactivated {} mods"),
        };
        self.done_said(said, cx);
    }

    /// The mods chosen moved to the other list, and nothing chosen then.
    pub fn move_selected(&mut self, cx: &mut Context<Self>) {
        let (from, ids) = (Side::of(self.selected_active), self.ids());
        self.take_ids(from, &ids, None, cx);
        self.select(None, true, cx);
    }

    /// The mod chosen moved across, and the one beside it chosen, as Enter does.
    pub(super) fn activate_selected(&mut self, side: Side, cx: &mut Context<Self>) {
        let Some(first) = self.selected_id().and_then(ModId::from_qml) else {
            return;
        };
        let list = self.order.list(side);
        let (Some(position), false) = (list.position_of(first), self.order.previewing()) else {
            return;
        };
        let next = list.beside_mod(first);
        self.take(side, position, None, cx);
        if let Some(next) = next {
            self.select(Some(next.to_qml()), side == Side::Active, cx);
        }
    }

    /// The mod chosen moved across or back, as the row's button and a double click do.
    pub(super) fn toggle_mod(&mut self, side: Side, mod_id: i32, cx: &mut Context<Self>) {
        let position = ModId::from_qml(mod_id).and_then(|id| self.order.list(side).position_of(id));
        if let (Some(position), false) = (position, self.order.previewing()) {
            self.take(side, position, None, cx);
        }
    }

    /// A drop: `held` (more than one) move together, one alone moves from `index`; `to` is the
    /// place dropped on, the list's length for its end.
    pub(super) fn drop_on(
        &mut self,
        (from, index, held): (Side, usize, Vec<ModId>),
        (into, to): (Side, usize),
        cx: &mut Context<Self>,
    ) {
        match (held.len() > 1, from == into) {
            (true, true) => {
                let app = super::super::app(cx);
                if self.order.move_mods(&app, from, &held, to) {
                    self.changed(cx);
                    self.done_said(say_n!(held.len(), "Moved {} mod", "Moved {} mods"), cx);
                }
            }
            (true, false) => self.take_ids(from, &held, Some(to), cx),
            (false, true) => {
                let last = self.order.list(from).mods().len().saturating_sub(1);
                let place = to.min(last);
                if index != place {
                    self.move_mod(from, index, place, cx);
                }
            }
            (false, false) => self.take(from, index, Some(to), cx),
        }
    }

    /// The mod chosen moved `by` places, as Alt + an arrow does.
    fn nudge(&mut self, side: Side, by: isize, cx: &mut Context<Self>) {
        let Some(first) = self.selected_id().and_then(ModId::from_qml) else {
            return;
        };
        let sorted_by_column = side == Side::Active && !self.orders.active_key.is_empty();
        if self.order.previewing() || sorted_by_column {
            return;
        }
        if let Some((from, to)) = self.order.list(side).nudged(first, by) {
            self.move_mod(side, from, to, cx);
            self.reveal_selected(side, false, cx);
        }
    }

    pub fn to_edge(&mut self, top: bool, cx: &mut Context<Self>) {
        let ids = self.ids();
        let app = super::super::app(cx);
        self.order.to_edge(&app, &ids, top);
        self.changed(cx);
        self.done_said(say_n!(ids.len(), "Moved {} mod", "Moved {} mods"), cx);
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        let app = super::super::app(cx);
        if !self.order.previewing() && self.order.undo(&app) {
            self.changed(cx);
        }
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        let app = super::super::app(cx);
        if !self.order.previewing() && self.order.redo(&app) {
            self.changed(cx);
        }
    }

    pub(super) fn discard_changes(&mut self, cx: &mut Context<Self>) {
        let app = super::super::app(cx);
        self.order.discard_changes(&app);
        self.changed(cx);
        self.done_said(say!("Back to the saved list"), cx);
    }

    /// The list on disk taken in place of this one, or this one kept to overwrite it.
    pub fn keep_file(&mut self, take: bool, cx: &mut Context<Self>) {
        let app = super::super::app(cx);
        self.order.keep_file(&app, take);
        self.save_problem.clear();
        self.file_changed = false;
        self.changed(cx);
    }

    /// Writes the load order to ModsConfig.xml off the window's thread; `then` runs once written.
    pub fn save(&mut self, then: Option<AfterSave>, cx: &mut Context<Self>) {
        if self.saving || self.demo {
            return;
        }
        if self.order.previewing() {
            self.end_sort(true, cx);
        }
        let app = super::super::app(cx);
        let saving = match self.order.to_save(&app) {
            Ok(saving) => saving,
            Err(problem) => {
                self.save_problem = crate::words::said_of(&problem);
                cx.notify();
                return;
            }
        };
        let changes = app.session().changes().said();
        let before = app.session().saved().to_vec();
        self.saving = true;
        let worker = app.clone();
        crate::jobs::run(
            "save",
            cx,
            move || {
                let snapshot = worker.snapshot("@saving", &before);
                let written = worker.write_list(&saving.file, &saving.written, &saving.expected);
                if written.is_ok() {
                    let said = say!("Saved the load order: {}", changes.inside());
                    worker.note_change(said, snapshot.as_deref());
                }
                (saving.list, written)
            },
            move |mods, (list, written), cx| {
                mods.saving = false;
                if written.is_ok() {
                    mods.order.saved(&app, list);
                }
                mods.save_problem = crate::words::why_not(&written);
                mods.file_changed = written.as_ref().err() == Some(&modsconfig::changed_outside());
                mods.changed(cx);
                if let (Ok(()), Some(then)) = (written, then) {
                    then(mods, cx);
                }
            },
        );
        cx.notify();
    }

    pub fn preview_sort(&mut self, cx: &mut Context<Self>) {
        if self.order.previewing() || !self.files_read || self.demo {
            return;
        }
        let app = super::super::app(cx);
        self.order.preview_sort(&app);
        self.closed_note.clear();
        self.show(cx);
        if let Some(position) = self.order.first_moved()
            && let Some(row) = self.order.list(Side::Active).row_of(position)
        {
            self.list(Side::Active)
                .update(cx, |list, cx| list.reveal(row, cx));
        }
    }

    pub(super) fn end_sort(&mut self, keep: bool, cx: &mut Context<Self>) {
        let app = super::super::app(cx);
        if !keep {
            self.order.drop_sort(&app);
            return self.show(cx);
        }
        if !self.order.previewing() {
            return;
        }
        let moves = self.order.sort_move_count();
        let before = app.snapshot("@sorting", app.session().current());
        self.order.keep_sort(&app);
        self.changed(cx);
        let sorted = say_n!(moves, "Sorted: {} mod moved", "Sorted: {} mods moved");
        app.note_change(sorted.clone(), before.as_deref());
        self.done_said(sorted, cx);
    }
}

impl Mods {
    /// One mod chosen, or none.
    pub fn select(&mut self, mod_id: Option<i32>, in_active: bool, cx: &mut Context<Self>) {
        self.keep_selection(mod_id.into_iter().collect(), in_active, cx);
    }

    /// `mod_id` added to the mods chosen in its list, or taken out of them, as Ctrl + click does.
    pub(super) fn select_also(&mut self, mod_id: i32, in_active: bool, cx: &mut Context<Self>) {
        let first = self
            .selected_id()
            .filter(|_| self.selected_active == in_active);
        let Some(first) = first else {
            return self.select(Some(mod_id), in_active, cx);
        };
        if mod_id == first {
            return;
        }
        let mut ids = self.selection.clone();
        match ids.iter().position(|held| *held == mod_id) {
            Some(at) => drop(ids.remove(at)),
            None => ids.push(mod_id),
        }
        self.keep_selection(ids, in_active, cx);
    }

    /// Every mod from the first chosen to `mod_id`, as Shift + click does.
    pub(super) fn select_through(&mut self, mod_id: i32, in_active: bool, cx: &mut Context<Self>) {
        let first = self
            .selected_id()
            .filter(|_| self.selected_active == in_active)
            .and_then(ModId::from_qml);
        let (Some(first), Some(to)) = (first, ModId::from_qml(mod_id)) else {
            return self.select(Some(mod_id), in_active, cx);
        };
        let list = self.order.list(Side::of(in_active));
        let Some(others) = list.others_through(first, to) else {
            return;
        };
        let ids = std::iter::once(first)
            .chain(others)
            .map(ModId::to_qml)
            .collect();
        self.keep_selection(ids, in_active, cx);
    }

    pub(super) fn invert_selection(&mut self, in_active: bool, cx: &mut Context<Self>) {
        let (side, on) = (Side::of(in_active), Side::of(self.selected_active));
        let inverted = self.order.inverted(side, on, &self.ids());
        let ids = inverted.ids.iter().map(|id| id.to_qml()).collect();
        self.keep_selection(ids, in_active, cx);
    }

    fn select_all(&mut self, side: Side, cx: &mut Context<Self>) {
        let ids = self
            .order
            .list(side)
            .mods()
            .iter()
            .filter_map(|m| m.installed)
            .map(ModId::to_qml)
            .collect();
        self.keep_selection(ids, side == Side::Active, cx);
    }

    /// The first chosen first, then the others by id, as `Selection::of` kept them.
    pub(super) fn keep_selection(
        &mut self,
        ids: Vec<i32>,
        in_active: bool,
        cx: &mut Context<Self>,
    ) {
        let mut ids = ids.into_iter();
        let first = ids.next();
        let mut others: Vec<i32> = ids.filter(|id| Some(*id) != first).collect();
        others.sort();
        others.dedup();
        self.selection = first.into_iter().chain(others).collect();
        self.selected_active = in_active;
        self.describe(cx);
        self.sync_lists(cx);
        cx.notify();
    }

    /// The selection follows its first mod into the list it went to.
    fn follow_selection(&mut self) {
        let Some(first) = self.selected_id().and_then(ModId::from_qml) else {
            return;
        };
        let on = |side| self.order.list(side).position_of(first).is_some();
        match (on(Side::Active), on(Side::Inactive)) {
            (true, _) => self.selected_active = true,
            (_, true) => self.selected_active = false,
            _ => {}
        }
    }

    /// The chosen mod's row brought into view, in the middle when `centre`.
    fn reveal_selected(&mut self, side: Side, centre: bool, cx: &mut Context<Self>) {
        let Some(id) = self.selected_id() else {
            return;
        };
        let rows = self.shown(side).rows.clone();
        if let Some(at) = rows.iter().position(|r| r.mod_id == id && !r.is_header) {
            self.list(side).update(cx, |list, cx| match centre {
                true => list.reveal(at, cx),
                false => list.contain(at, cx),
            });
        }
    }

    /// The arrows step through the list and Alt moves the chosen mod; Enter moves it across;
    /// Ctrl + A chooses every mod and Ctrl + I the others.
    pub(super) fn list_key(
        &mut self,
        side: Side,
        event: &KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keys = &event.keystroke;
        let (alt, command) = (keys.modifiers.alt, keys.modifiers.secondary(cx));
        let by = match keys.key.as_str() {
            "up" => -1,
            "down" => 1,
            _ => 0,
        };
        match keys.key.as_str() {
            "up" | "down" if alt => self.nudge(side, by, cx),
            "up" | "down" => {
                let current = self
                    .selected_id()
                    .filter(|_| self.selected_active == (side == Side::Active));
                let list = self.order.list(side);
                let row = list.step_from(current.and_then(ModId::from_qml), by);
                let id = row
                    .and_then(|row| self.shown(side).rows.get(row))
                    .map(|r| r.mod_id);
                if let Some(id) = id.filter(|id| *id >= 0) {
                    self.select(Some(id), side == Side::Active, cx);
                    self.reveal_selected(side, false, cx);
                }
            }
            "enter" => self.activate_selected(side, cx),
            "a" if command => self.select_all(side, cx),
            "i" if command => self.invert_selection(side == Side::Active, cx),
            "escape" if !self.selection.is_empty() => self.select(None, true, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Mods {
    /// The next mod with a problem chosen and shown, or the one before.
    pub(super) fn show_problem(&mut self, forward: bool, cx: &mut Context<Self>) {
        use rimstack_core::mods::list::Row;
        let list = self.order.list(Side::Active);
        let rows = list.rows();
        let troubles = self.order.troubles();
        let troubled = |index: usize| troubles.get(index).is_some_and(|t| !t.is_empty());
        let count = rows.len() as isize;
        if count == 0 {
            return;
        }
        let step = if forward { 1 } else { -1 };
        let from = match self.problem_row {
            Some(row) => row as isize,
            None if forward => -1,
            None => 0,
        };
        let found = (1..=count)
            .map(|n| (from + step * n).rem_euclid(count) as usize)
            .find(|&row| matches!(rows[row], Row::Mod { index } if troubled(index)));
        self.problem_row = found;
        let Some(row) = found else {
            return;
        };
        if let Some(id) = self.active.rows.get(row).map(|r| r.mod_id) {
            self.select(Some(id), true, cx);
            self.list(Side::Active)
                .update(cx, |list, cx| list.contain(row, cx));
        }
    }

    /// No warning about the game version while the game stays on `version`; "" brings them back.
    pub(super) fn set_version_aside(&mut self, version: String, cx: &mut Context<Self>) {
        let app = super::super::app(cx);
        let before = app.settings_report().version_aside_for.unwrap_or_default();
        if let Err(problem) = app.keep_setting("versionAsideFor", &version) {
            notices::say(crate::words::said_of(&problem), notices::Tone::Err, cx);
            return;
        }
        if !version.is_empty() {
            let me = cx.entity().downgrade();
            notices::offer(
                put(&say!(
                    "Game version warnings are ignored for RimWorld {}",
                    version
                )),
                put(&say!("Undo")),
                move |_, cx| {
                    let before = before.clone();
                    let _ = me.update(cx, |mods, cx| mods.set_version_aside(before, cx));
                },
                cx,
            );
        }
    }
}
