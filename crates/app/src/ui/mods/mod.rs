//! `ModsScreen.qml`: the inactive mods, the load order and the details, side by side.

mod about;
mod active;
mod asked;
mod bands;
mod bench;
mod chosen;
mod columns;
mod compare;
mod details;
mod edit;
mod filter;
mod inactive;
mod latest;
mod left_off;
mod list;
mod menus;
mod notes;
mod panes;
mod problems;
mod rows;
mod selection;
mod sort_moves;
mod tabs;

use super::text_field::{Edited, TextField};
use super::theme::{self, Theme};
use crate::changes::Changes;
use crate::jobs::Lane;
use crate::workspace::load_order::{Details, LoadOrder, Orders, Overview, RowView, Side};
use gpui::{
    AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement, ParentElement, Pixels,
    Render, Styled, Window, div, prelude::FluentBuilder, px,
};
use rimstack_core::mods::library::ModId;
use rimstack_core::mods::list::Category;
use std::rc::Rc;

/// A list's rows as last read from the load order.
#[derive(Default)]
pub struct Shown {
    pub rows: Rc<Vec<RowView>>,
}

gpui::actions!(mods, [FocusFilter, Undo, Redo, Save]);

pub use edit::acting;
pub use menus::Wanted;

/// glibc keeps what a thread freed in that thread's heap; after the scan's threads that was
/// 100 MB (measured 2026-11-12). Capping the heaps instead made the scan wait on their locks.
fn give_back_freed() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    // SAFETY: only returns free pages to the system.
    unsafe {
        libc::malloc_trim(0);
    }
}

/// Asked of the window: show this section.
pub struct ShowSection(pub usize);

impl gpui::EventEmitter<ShowSection> for Mods {}

pub struct Mods {
    order: LoadOrder,
    active: Shown,
    inactive: Shown,
    orders: Orders,
    pub filter: Entity<TextField>,
    active_list: Entity<list::ModList>,
    inactive_list: Entity<list::ModList>,
    focus: gpui::FocusHandle,
    overview: Option<Rc<Overview>>,
    details: Option<Rc<Details>>,
    details_view: Entity<chosen::DetailsView>,
    /// A row to choose once the list is read: its place and the tab to show.
    wanted: Option<(usize, usize)>,
    summary: Option<Rc<crate::reports::Summary>>,
    last_start: Option<Rc<crate::workspace::logs::LastStartSaid>>,
    /// Mods changed since the game last started, read off the window's thread.
    changed_since: usize,
    /// The active mods with a newer Workshop version, as Downloads counts them.
    behind: usize,
    since_reading: Lane,
    conflicts: usize,
    /// Inactive mods installed since the game was last started from RimStack.
    fresh: usize,
    /// `RowView::mod_id`s, the first chosen first.
    selection: Vec<i32>,
    selected_active: bool,
    demo: bool,
    loading: Lane,
    counting: Lane,
    files_read: bool,
    sort_pending: usize,
    inactive_width: Pixels,
    details_width: Pixels,
    saving: bool,
    save_problem: String,
    /// ModsConfig.xml was written outside RimStack since it was read: Save refused to overwrite it.
    file_changed: bool,
    /// The Sort note closed, by its words; and the one whose closing is on its way.
    closed_note: String,
    note_timer: String,
    /// The row of the problem last shown by the arrows beside the problems.
    problem_row: Option<usize>,
    /// Four mods to choose once the list is read, as `--several` asks.
    several: bool,
    problems_open: bool,
    /// Told by the window while the search for the broken mod runs.
    hunting: bool,
    /// The other mod of a rule of your own being chosen.
    picking: Option<notes::Picking>,
    drafts: notes::Drafts,
    /// "Saved" shows beside the notes for a while after they are kept.
    notes_saved: bool,
    chosen_read: Lane,
    /// What the Files tab shows of the chosen mod, read off the window's thread.
    chosen_files: Option<(ModId, Rc<chosen::FilesRead>)>,
    /// A Git checkout's last commits, for its Latest changes.
    chosen_commits: Option<(ModId, Rc<Vec<rimstack_core::sources::changes::Note>>)>,
    commits_read: Lane,
    unhide_problem: String,
    /// The shared textures of two mods, shown in place of the lists.
    comparing: Option<Entity<compare::CompareView>>,
    /// What the comparison shows, as it last told.
    compared: Option<Rc<crate::workspace::textures::Comparison>>,
    /// Two mods to compare once their files are read, as `--comparing` asks.
    compare_wanted: bool,
    /// Sort's preview to show once the files are read, as `--sorting` asks.
    sort_wanted: bool,
    /// How the tab is shown, as `[Mods]` keeps it.
    view: Rc<crate::workspace::ModsView>,
    /// Where the View and the inactive order buttons were drawn, for `--open=`.
    menu_buttons: columns::Placed,
    /// A menu to open once its button is drawn, as `--open=view` or `--open=order` asks.
    pub menu_wanted: Option<&'static str>,
    /// The inactive mods shown over the load order, from the rail.
    pub drawer_open: bool,
    /// The drawer's own filter, as `ModsFilter` mirrored the load order's.
    mirror: Entity<TextField>,
    widths_kept: Lane,
    left_off: left_off::LeftOff,
    /// Where a heading or a column's edge was pressed, and the column's width then.
    column_drag: Option<(Pixels, Pixels)>,
    /// The filter's terms shown as chips; the field holds what is typed after them.
    filter_terms: Vec<String>,
    /// The offers under the filter put away until it is typed in or clicked again.
    offers_shut: bool,
}

impl Mods {
    pub fn new(demo: bool, cx: &mut Context<Self>) -> Self {
        let theme = cx.global::<Theme>();
        let ((first_inactive, first_details), size) = (panes::first_widths(theme), theme.body());
        let view = Rc::new(match demo {
            true => crate::workspace::ModsView::default(),
            false => super::app(cx).mods_view(),
        });
        let kept = |width: f32, first: Pixels| match width > 0. {
            true => px(width),
            false => first,
        };
        let (inactive_width, details_width) = (
            kept(view.inactive, first_inactive),
            kept(view.details, first_details),
        );
        let mirror = cx.new(|cx| TextField::new(size, cx));
        cx.subscribe(&mirror, |mods, field, _: &Edited, cx| {
            mods.filter_edited(&field, cx)
        })
        .detach();
        let order = if demo {
            LoadOrder::demo(2000, 300)
        } else {
            LoadOrder::demo(0, 0)
        };
        let filter = cx.new(|cx| TextField::new(size, cx));
        cx.subscribe(&filter, |mods, field, _: &Edited, cx| {
            mods.filter_edited(&field, cx)
        })
        .detach();
        let thumbs = super::thumbs::Shared::default();
        let orders = Rc::new(order.orders());
        let lists = [Side::Active, Side::Inactive].map(|side| {
            let shown = list::Shown {
                rows: Rc::default(),
                orders: orders.clone(),
                view: view.clone(),
                marks: Rc::default(),
                chosen: Rc::default(),
                movable: false,
            };
            let (thumbs, mods) = (thumbs.clone(), cx.weak_entity());
            cx.new(|cx| list::ModList::new(side, shown, thumbs, mods, cx))
        });
        let [active_list, inactive_list] = lists;
        let me = cx.weak_entity();
        let details_view = cx.new(|cx| chosen::DetailsView::new(me, cx));
        cx.subscribe(
            &details_view,
            |mods, _, asked: &chosen::Asked, cx| match asked {
                chosen::Asked::Section(section) => cx.emit(ShowSection(*section)),
                chosen::Asked::Filter(text) => mods.filter_by(text, cx),
                chosen::Asked::Tab(tab) => mods.keep_left_tab(*tab, cx),
            },
        )
        .detach();
        let mut mods = Self {
            focus: cx.focus_handle(),
            filter,
            active_list,
            inactive_list,
            orders: order.orders(),
            overview: None,
            details: None,
            details_view,
            wanted: None,
            summary: None,
            last_start: None,
            changed_since: 0,
            behind: 0,
            since_reading: Lane::default(),
            conflicts: 0,
            fresh: 0,
            order,
            active: Shown::default(),
            inactive: Shown::default(),
            selection: Vec::new(),
            selected_active: true,
            demo,
            loading: Lane::default(),
            counting: Lane::default(),
            files_read: false,
            sort_pending: 0,
            inactive_width,
            details_width,
            saving: false,
            save_problem: String::new(),
            file_changed: false,
            closed_note: String::new(),
            note_timer: String::new(),
            problem_row: None,
            several: false,
            problems_open: false,
            hunting: false,
            picking: None,
            drafts: notes::Drafts::default(),
            notes_saved: false,
            chosen_read: Lane::default(),
            chosen_files: None,
            chosen_commits: None,
            commits_read: Lane::default(),
            unhide_problem: String::new(),
            comparing: None,
            compared: None,
            compare_wanted: false,
            sort_wanted: false,
            view,
            menu_buttons: Default::default(),
            menu_wanted: None,
            drawer_open: false,
            mirror,
            widths_kept: Lane::default(),
            column_drag: None,
            filter_terms: Vec::new(),
            offers_shut: false,
            left_off: match demo {
                true => left_off::LeftOff::Off,
                false => left_off::LeftOff::Waiting,
            },
        };
        mods.show(cx);
        if !demo {
            mods.hear(cx);
            mods.reread(Changes::LAUNCHES | Changes::LISTS, cx);
            mods.load(false, cx);
        }
        mods
    }

    /// The `App`'s news, handed to the window's thread as it comes.
    fn hear(&self, cx: &mut Context<Self>) {
        let (tell, mut told) = futures::channel::mpsc::unbounded();
        super::app(cx).listen(move |changes| {
            let _ = tell.unbounded_send(changes);
        });
        cx.spawn(async move |mods, cx| {
            use futures::StreamExt;
            while let Some(changes) = told.next().await {
                if mods.update(cx, |mods, cx| mods.heard(changes, cx)).is_err() {
                    return;
                }
            }
        })
        .detach();
    }

    fn heard(&mut self, changes: Changes, cx: &mut Context<Self>) {
        let app = super::app(cx);
        self.reread(changes, cx);
        if changes.contains(Changes::LIBRARY | Changes::LANGUAGE) {
            return self.load(changes.contains(Changes::LIBRARY), cx);
        }
        let order = &mut self.order;
        if changes.contains(Changes::NOTES) {
            order.sync(&app);
        }
        if changes.contains(Changes::SETTINGS) {
            order.follow_left_out(&app);
        }
        if changes.contains(Changes::PROBLEMS | Changes::COMMUNITY) {
            order.recheck(&app);
        }
        if changes.contains(Changes::COMMUNITY | Changes::FILES_READ) {
            order.refile(&app);
        }
        if changes.contains(Changes::UPDATES | Changes::LAUNCHES | Changes::FIND) {
            order.settle(&app);
        }
        if changes.contains(Changes::FACTS) {
            order.facts_read(&app);
            if changes == Changes::FACTS && !order.needs_facts() {
                return;
            }
        }
        if changes.contains(Changes::FILES_READ) {
            self.files_read = true;
            if std::mem::take(&mut self.compare_wanted) {
                self.compare_first(cx);
            }
            if std::mem::take(&mut self.sort_wanted) {
                self.preview_sort(cx);
            }
        }
        if changes.contains(Changes::TEXTURES) {
            self.read_files(cx);
        }
        self.show(cx);
    }

    fn read_files(&mut self, cx: &mut Context<Self>) {
        let app = super::app(cx);
        let Some(members) = self.order.files_to_read(&app) else {
            self.files_read = self.order.is_installed();
            return;
        };
        self.files_read = false;
        crate::jobs::spawn("active files", move || app.read_active_files(members));
    }

    /// How many mods Sort would move, counted off the window's thread once the list is quiet.
    fn count_sort_moves(&mut self, cx: &mut Context<Self>) {
        const QUIET: std::time::Duration = std::time::Duration::from_millis(400);
        let Some(count) = self.order.sort_count().filter(|_| self.files_read) else {
            self.counting.cancel();
            self.sort_pending = 0;
            return;
        };
        let app = super::app(cx);
        self.counting.run(
            "sort count",
            cx,
            move |turn| turn.after(QUIET).then(|| count.moves(&app)),
            |mods, moves, cx| {
                if let Some(moves) = moves {
                    mods.sort_pending = moves;
                    cx.notify();
                }
            },
        );
    }

    /// What the details and the status bar read from disk, read again only when it changed.
    fn reread(&mut self, changes: Changes, cx: &mut Context<Self>) {
        if self.demo {
            return;
        }
        let app = super::app(cx);
        let counted = Changes::LIBRARY
            | Changes::LISTS
            | Changes::LAUNCHES
            | Changes::SAVES
            | Changes::RULES
            | Changes::UPDATES
            | Changes::COMMUNITY
            | Changes::LANGUAGE;
        if changes.contains(counted) {
            self.summary = Some(Rc::new(app.summary()));
        }
        if changes
            .contains(Changes::LAUNCHES | Changes::LOGS | Changes::FOLDERS | Changes::LANGUAGE)
        {
            self.last_start = app.last_start_said().map(Rc::new);
        }
        if changes.contains(Changes::LIBRARY | Changes::UPDATES | Changes::LAUNCHES) {
            self.since_reading.run(
                "changed since launch",
                cx,
                move |_| app.since_launch().items.len(),
                |mods, changed, cx| {
                    mods.changed_since = changed;
                    mods.sync_details(cx);
                },
            );
        }
    }

    /// Hands the details pane what it shows.
    fn sync_details(&mut self, cx: &mut Context<Self>) {
        let (dirty, unsaved) = match self.demo {
            true => (false, String::new()),
            false => {
                let app = super::app(cx);
                let session = app.session();
                (
                    session.is_dirty(),
                    crate::words::put(&session.changes().said()),
                )
            }
        };
        let shown = chosen::DetailsShown {
            demo: self.demo,
            active: self.count(Side::Active),
            inactive: self.count(Side::Inactive),
            game_version: self.game_version(),
            overview: self.overview.clone(),
            details: self.details.clone(),
            summary: self.summary.clone(),
            last_start: self.last_start.clone(),
            warnings: self.order.errors_and_warnings().1,
            troubled: self.order.troubles().iter().any(|t| !t.is_empty()),
            dirty,
            unsaved,
            conflicts: self.conflicts,
            changed_since: self.changed_since,
            behind: self.behind,
            selected_active: self.selected_active,
            panes: self.panes_wanted(),
            problems_open: self.problems_open,
            position: self
                .selected_id()
                .and_then(ModId::from_qml)
                .and_then(|id| self.order.list(Side::Active).position_of(id))
                .map_or(0, |at| at + 1),
            picking: self.picking,
            notes_saved: self.notes_saved,
            files: self
                .chosen_files
                .as_ref()
                .filter(|(of, _)| self.selected_id().and_then(ModId::from_qml) == Some(*of))
                .map(|(_, read)| read.clone()),
            unhide_problem: self.unhide_problem.clone(),
            comparison: self.compared.clone(),
            sort_moves: self
                .order
                .previewing()
                .then(|| Rc::new(self.order.sort_moves())),
            chosen_id: self.selected_id(),
            commits: self
                .chosen_commits
                .as_ref()
                .filter(|(of, _)| self.selected_id().and_then(ModId::from_qml) == Some(*of))
                .map(|(_, read)| read.clone()),
            hunting: self.hunting,
            previewing: self.order.previewing(),
            selected: self.selection.len(),
            selection: self.selection_summary(),
        };
        self.details_view
            .update(cx, |view, cx| view.show(shown, cx));
    }

    /// `anew`: the mods were read again, so every id held before means nothing.
    fn load(&mut self, anew: bool, cx: &mut Context<Self>) {
        let app = super::app(cx);
        let (headings, inactive_by) = (
            self.view.headings,
            rimstack_core::mods::columns::from_kept(&self.view.inactive_order),
        );
        let active_by = self.order.order_of(Side::Active);
        self.loading.run(
            "load order",
            cx,
            move |_| {
                let mut order = LoadOrder::read(&app, headings, inactive_by);
                order.set_active_by(active_by);
                order
            },
            move |mods, mut order, cx| {
                let text = mods.filter_text(cx);
                order.set_filter(&text);
                if anew {
                    let same = mods.order.same_mods_in(&order);
                    let held: Vec<ModId> = mods
                        .selection
                        .iter()
                        .filter_map(|id| ModId::from_qml(*id))
                        .collect();
                    let kept = mods.order.carried(&same, &held);
                    mods.selection = kept.ids.iter().map(|id| id.to_qml()).collect();
                    mods.selected_active = kept.active;
                }
                mods.order = order;
                mods.show(cx);
                mods.describe(cx);
                give_back_freed();
                mods.read_files(cx);
                mods.restore_left_off(cx);
                if let Some((row, tab)) = mods.wanted.take() {
                    mods.pick_row(Side::Active, row, tab, cx);
                }
                if std::mem::take(&mut mods.several) {
                    mods.choose_several(cx);
                }
                if let Some(started) = crate::STARTED.get() {
                    rimstack_core::note!("mods_shown_ms={}", started.elapsed().as_millis());
                }
            },
        );
    }

    fn show(&mut self, cx: &mut Context<Self>) {
        let app = super::app(cx);
        for side in [Side::Active, Side::Inactive] {
            let rows = self.order.rows(&app, side);
            self.shown_mut(side).rows = Rc::new(rows);
        }
        self.orders = self.order.orders();
        self.sync_lists(cx);
        if self.details.is_some() {
            self.describe(cx);
        }
        self.overview = (!self.demo).then(|| Rc::new(self.order.overview(&app)));
        self.fresh = if self.demo {
            0
        } else {
            self.order.fresh(&app, Side::Inactive).len()
        };
        self.conflicts = if self.demo {
            0
        } else {
            self.order.in_conflict(&app).len()
        };
        if !self.demo {
            self.count_sort_moves(cx);
        }
        self.sync_details(cx);
        cx.notify();
    }

    fn shown_mut(&mut self, side: Side) -> &mut Shown {
        match side {
            Side::Active => &mut self.active,
            Side::Inactive => &mut self.inactive,
        }
    }

    pub fn count(&self, side: Side) -> usize {
        self.order.list(side).mods().len()
    }

    pub fn toggle_category(&mut self, side: Side, category: i32, cx: &mut Context<Self>) {
        let found = usize::try_from(category)
            .ok()
            .and_then(|at| Category::ALL.get(at));
        if let Some(&category) = found {
            let app = super::app(cx);
            self.order.toggle_category(&app, side, category);
            self.show(cx);
        }
    }

    pub fn toggle_sub(&mut self, side: Side, row: usize, cx: &mut Context<Self>) {
        let app = super::app(cx);
        self.order.toggle_sub(&app, side, row);
        self.show(cx);
    }

    pub fn choose(&mut self, side: Side, mod_id: i32, cx: &mut Context<Self>) {
        let picks = self.picking.is_some() && side == Side::Active && mod_id >= 0;
        if picks && Some(mod_id) != self.selected_id() {
            return self.pick(mod_id, cx);
        }
        self.select(Some(mod_id), side == Side::Active, cx);
    }

    /// What the details show of the mod chosen, when one alone is.
    fn describe(&mut self, cx: &mut Context<Self>) {
        let app = super::app(cx);
        let alone = match self.selection.as_slice() {
            [one] => rimstack_core::mods::library::ModId::from_qml(*one),
            _ => None,
        };
        let marks_before = self.details.as_ref().map(|d| d.conflict_marks.clone());
        self.details = alone
            .filter(|_| self.order.is_installed())
            .map(|id| Rc::new(self.order.details(&app, id)));
        if self.details.as_ref().map(|d| d.conflict_marks.clone()) != marks_before {
            self.sync_lists(cx);
        }
        self.read_chosen(cx);
        self.sync_details(cx);
        self.keep_left_mod(cx);
    }

    /// Chooses a row of a list by its place, headings counted, as `--pick` does.
    pub fn pick_row(&mut self, side: Side, row: usize, tab: usize, cx: &mut Context<Self>) {
        let rows = match side {
            Side::Active => self.active.rows.clone(),
            Side::Inactive => self.inactive.rows.clone(),
        };
        match rows.get(row) {
            Some(found) if !found.is_header => {
                self.details_view.update(cx, |view, _| view.tab = tab);
                self.choose(side, found.mod_id, cx);
            }
            Some(_) => {}
            None => self.wanted = Some((row, tab)),
        }
    }

    /// Hands each list its rows and what is chosen in it.
    fn sync_lists(&mut self, cx: &mut Context<Self>) {
        let orders = Rc::new(self.order.orders());
        let marks: Rc<rows::Marks> = Rc::new(
            self.details
                .as_ref()
                .map(|d| d.conflict_marks.clone())
                .unwrap_or_default(),
        );
        for side in [Side::Active, Side::Inactive] {
            let on_side = self.selected_active == (side == Side::Active);
            let chosen = match on_side {
                true => self
                    .selection
                    .iter()
                    .copied()
                    .filter(|id| *id >= 0)
                    .collect(),
                false => std::collections::HashSet::new(),
            };
            let in_order = side == Side::Inactive || orders.active_key.is_empty();
            let shown = list::Shown {
                rows: self.shown(side).rows.clone(),
                orders: orders.clone(),
                view: self.view.clone(),
                marks: match side {
                    Side::Active => marks.clone(),
                    Side::Inactive => Rc::default(),
                },
                chosen: Rc::new(chosen),
                movable: in_order && !self.order.previewing() && !self.demo,
            };
            self.list(side).update(cx, |list, cx| list.show(shown, cx));
        }
    }

    fn shown(&self, side: Side) -> &Shown {
        match side {
            Side::Active => &self.active,
            Side::Inactive => &self.inactive,
        }
    }

    pub fn list(&self, side: Side) -> &Entity<list::ModList> {
        match side {
            Side::Active => &self.active_list,
            Side::Inactive => &self.inactive_list,
        }
    }

    pub fn is_chosen(&self, side: Side, mod_id: i32) -> bool {
        mod_id >= 0
            && self.selected_active == (side == Side::Active)
            && self.selection.contains(&mod_id)
    }

    /// Four mods of the load order chosen, as `--several` asks: once the list is read.
    pub fn choose_several(&mut self, cx: &mut Context<Self>) {
        if self.active.rows.is_empty() {
            self.several = true;
            return;
        }
        self.choose_nth(Side::Active, 14, false, cx);
        for at in [15, 16, 19] {
            self.choose_nth(Side::Active, at, true, cx);
        }
    }

    /// Chooses the `nth` mod of a list, as `--several` and the checks do.
    pub fn choose_nth(&mut self, side: Side, nth: usize, also: bool, cx: &mut Context<Self>) {
        let rows = match side {
            Side::Active => &self.active.rows,
            Side::Inactive => &self.inactive.rows,
        };
        let Some(id) = rows
            .iter()
            .filter(|r| !r.is_header)
            .nth(nth)
            .map(|r| r.mod_id)
        else {
            return;
        };
        let mut ids = match also {
            true => self.selection.clone(),
            false => Vec::new(),
        };
        ids.push(id);
        self.keep_selection(ids, side == Side::Active, cx);
    }
}

pub use panes::pane_widths;

impl Focusable for Mods {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

impl Render for Mods {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let handle = || {
            div()
                .flex()
                .flex_none()
                .justify_center()
                .w(theme::HANDLE)
                .h_full()
                .child(div().w(px(1.)).h_full().bg(theme.colours.edge))
        };
        let (_, _, details_width) =
            pane_widths(&theme, window.viewport_size().width, self.panes_wanted());
        let bulk = self.bulk_bar(&theme, cx);
        self.open_wanted_menu(window, cx);
        div()
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .track_focus(&self.focus)
            .on_action(cx.listener(|mods, _: &FocusFilter, window, cx| {
                let field = match mods.drawer_open && mods.view.inactive_hidden {
                    true => &mods.mirror,
                    false => &mods.filter,
                };
                window.focus(&field.focus_handle(cx), cx);
            }))
            .capture_key_down(cx.listener(|mods, event: &gpui::KeyDownEvent, _, cx| {
                if mods.drawer_open && event.keystroke.key == "escape" {
                    mods.set_drawer(false, cx);
                    cx.stop_propagation();
                }
            }))
            .on_action(cx.listener(|mods, _: &Undo, _, cx| mods.undo(cx)))
            .on_action(cx.listener(|mods, _: &Redo, _, cx| mods.redo(cx)))
            .on_action(cx.listener(|mods, _: &Save, _, cx| {
                if crate::ui::app(cx).session().is_dirty() {
                    mods.save(None, cx);
                }
            }))
            .when_some(self.comparing.clone(), |d, view| {
                d.child(
                    div()
                        .flex()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .child(div().flex_1().min_w_0().h_full().child(view))
                        .child(handle())
                        .child(
                            div()
                                .w(details_width)
                                .flex_none()
                                .h_full()
                                .child(self.details_view.clone()),
                        ),
                )
            })
            .when(self.comparing.is_none(), |d| {
                d.child(self.panes(&theme, window, cx))
            })
            .children(bulk)
    }
}

#[cfg(test)]
mod tests;

#[cfg(all(test, target_os = "linux"))]
mod edit_tests;

#[cfg(all(test, target_os = "linux"))]
mod details_tests;

#[cfg(all(test, target_os = "linux"))]
mod view_tests;
