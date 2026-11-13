//! The inactive mods, the load order and the details, side by side.

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
mod frame;
mod hints;
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
mod state;
mod tabs;
mod unread;

use super::text_field::TextField;
use super::theme::Theme;
use crate::changes::Changes;
use crate::jobs::Lane;
use crate::workspace::load_order::{Details, LoadOrder, Orders, Overview, RowView, Side};
use gpui::{AppContext, Context, Entity, Pixels, px};
use rimstack_core::mods::library::ModId;
use rimstack_core::mods::list::Category;
use std::rc::Rc;

const READS: Changes = Changes::ALL;

/// A list's rows as last read from the load order.
#[derive(Default)]
pub struct Shown {
    pub rows: Rc<Vec<RowView>>,
}

gpui::actions!(mods, [FocusFilter, Undo, Redo, Save]);

pub use columns::Menu;
pub use edit::acting;

/// glibc keeps what a thread freed in that thread's heap; after the scan's threads that was
/// 100 MB (measured 2026-11-12). Capping the heaps instead made the scan wait on their locks.
fn give_back_freed() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    // SAFETY: only returns free pages to the system.
    unsafe {
        libc::malloc_trim(0);
    }
}

/// What changes nothing of the lists' rows, only the details.
/// Facts arrive in batches while the mods' files are read: the lists follow once they pause.
const FACTS_QUIET: std::time::Duration = std::time::Duration::from_millis(150);

const ROWS_UNTOUCHED: Changes = Changes::LOGS
    .or(Changes::SAVES)
    .or(Changes::LISTS)
    .or(Changes::THEME)
    .or(Changes::DOWNLOAD_LIST)
    .or(Changes::KEY)
    .or(Changes::SOURCE)
    .or(Changes::MOD_FILES);

type AfterDrawn = Box<dyn FnMut(&mut Mods, &mut gpui::Window, &mut Context<Mods>) -> bool>;

/// How far the mods are read, for what waits on them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    ListRead,
    FilesRead,
}

impl gpui::EventEmitter<Progress> for Mods {}

/// The list's session as last shown: read once each time the Mods tab shows, never while drawn.
#[derive(Default, Clone, PartialEq)]
pub struct Standing {
    pub dirty: bool,
    /// The changes not saved, in words.
    pub unsaved: String,
    pub can_undo: bool,
    pub can_redo: bool,
}

pub struct Mods {
    order: LoadOrder,
    standing: Standing,
    active: Shown,
    inactive: Shown,
    orders: Orders,
    pub filter: Entity<TextField>,
    filtering: Entity<filter::Filter>,
    active_list: Entity<list::ModList>,
    /// The load order takes the keys at the next frame, as it does once a comparison closes.
    focus_wanted: bool,
    keys: columns::Reachable,
    inactive_list: Entity<list::ModList>,
    status: state::Status,
    layout: state::Layout,
    hints: state::Hinting,
    chosen_reads: state::ChosenReads,
    reads: state::Reads,
    problem_nav: state::ProblemNav,
    /// Asked to be done once the view is drawn; each answers whether it was done.
    when_drawn: Vec<AfterDrawn>,
    thumbs: super::thumbs::Shared,
    /// What `LoadOrder::settings_read` gave last.
    settings_read: Vec<String>,
    facts_quiet: Lane,
    focus: gpui::FocusHandle,
    overview: Option<Rc<Overview>>,
    details: Option<Rc<Details>>,
    details_view: Entity<chosen::DetailsView>,
    /// `RowView::mod_id`s, the first chosen first.
    selection: Vec<ModId>,
    selected_active: bool,
    demo: bool,
    loading: Lane,
    counting: Lane,
    files_reading: crate::jobs::Single<Vec<ModId>>,
    /// Whether no mod folder was found, and what is wrong with the game folder chosen, while the
    /// mods could not be read.
    why_unread: Option<(bool, Option<rimstack_core::base::sentence::Sentence>)>,
    files_read: bool,
    sort_pending: usize,
    /// Told by the window while the search for the broken mod runs.
    hunting: bool,
    /// The mod folders being read again.
    pub rescanning: bool,
    /// The shared textures of two mods, shown in place of the lists.
    comparing: Option<Entity<compare::CompareView>>,
    /// What the comparison shows, as it last told.
    compared: Option<Rc<crate::workspace::textures::Comparison>>,
    /// How the tab is shown, as `[Mods]` keeps it.
    view: Rc<crate::workspace::ModsView>,
    /// The drawer's own filter, as `ModsFilter` mirrored the load order's.
    mirror: Entity<TextField>,
    /// The view kept once dragging rests.
    view_kept: Lane,
    left_off: left_off::LeftOff,
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
        let filtering = cx.new(|cx| filter::Filter::new(size, cx));
        let (filter, mirror) = {
            let filtering = filtering.read(cx);
            (filtering.field.clone(), filtering.mirror.clone())
        };
        cx.subscribe(&filtering, |mods, _, filter::Changed(text), cx| {
            mods.filter_changed(text.clone(), cx)
        })
        .detach();
        let order = if demo {
            LoadOrder::demo(2000, 300)
        } else {
            LoadOrder::demo(0, 0)
        };
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
                dimmed: false,
                single: false,
                previewing: false,
                picking: false,
                count: 0,
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
                chosen::Asked::Section(section) => {
                    crate::ui::navigate::go(crate::ui::navigate::Go::Section(*section), cx)
                }
                chosen::Asked::Filter(text) => mods.filter_by(text, cx),
                chosen::Asked::Tab(tab) => mods.keep_left_tab(*tab, cx),
            },
        )
        .detach();
        let mut mods = Self {
            standing: Standing::default(),
            thumbs,
            status: Default::default(),
            layout: state::Layout::new(inactive_width, details_width),
            hints: Default::default(),
            chosen_reads: Default::default(),
            reads: Default::default(),
            problem_nav: Default::default(),
            when_drawn: Vec::new(),
            settings_read: LoadOrder::settings_read(&super::app(cx)),
            facts_quiet: Lane::default(),
            focus: cx.focus_handle(),
            filter,
            active_list,
            inactive_list,
            orders: order.orders(),
            overview: None,
            details: None,
            details_view,
            order,
            active: Shown::default(),
            inactive: Shown::default(),
            selection: Vec::new(),
            selected_active: true,
            demo,
            loading: Lane::default(),
            counting: Lane::default(),
            files_reading: Default::default(),
            why_unread: None,
            files_read: false,
            sort_pending: 0,
            hunting: false,
            focus_wanted: false,
            keys: Default::default(),
            rescanning: false,
            comparing: None,
            compared: None,
            view,
            mirror,
            view_kept: Lane::default(),
            filtering,
            left_off: match demo {
                true => left_off::LeftOff::Off,
                false => left_off::LeftOff::Waiting,
            },
        };
        mods.show(cx);
        if !demo {
            mods.hear(cx);
            mods.load(false, cx);
        }
        mods
    }

    /// The `App`'s news, handed to the window's thread as it comes.
    fn hear(&self, cx: &mut Context<Self>) {
        super::hear(cx, READS, Self::heard);
    }

    fn heard(&mut self, changes: Changes, cx: &mut Context<Self>) {
        let app = super::app(cx);
        let mut untouched = ROWS_UNTOUCHED;
        if changes.contains(Changes::SETTINGS) {
            let read = LoadOrder::settings_read(&app);
            if read == self.settings_read {
                untouched |= Changes::SETTINGS;
            }
            self.settings_read = read;
        }
        if changes == Changes::FACTS {
            self.facts_quiet.after(FACTS_QUIET, cx, |mods, cx| {
                let app = super::app(cx);
                mods.order.facts_read(&app);
                if mods.order.needs_facts() {
                    mods.show(cx);
                }
            });
            return;
        }
        if changes.within(untouched) {
            if self.details.is_some() {
                self.describe(cx);
            }
            self.sync_details(cx);
            cx.notify();
            return;
        }
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
        }
        if changes.contains(Changes::FILES_READ) {
            self.files_read = true;
            cx.emit(Progress::FilesRead);
        }
        if changes.contains(Changes::TEXTURES) {
            self.read_files(cx);
        }
        if changes.contains(Changes::SETTINGS | Changes::FOLDERS) {
            self.judge_unread(cx);
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
        app.outdate_files_read();
        self.files_reading
            .ask("active files", members, move |members| {
                app.read_active_files(members)
            });
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
        self.counting.run_after(
            QUIET,
            "sort count",
            cx,
            move |_| count.moves(&app),
            |mods, moves, cx| {
                mods.sort_pending = moves;
                cx.notify();
            },
        );
    }

    /// Hands the details pane what it shows.
    fn sync_details(&mut self, cx: &mut Context<Self>) {
        let (dirty, unsaved) = (self.standing.dirty, self.standing.unsaved.clone());
        let shown = chosen::DetailsShown {
            demo: self.demo,
            active: self.count(Side::Active),
            inactive: self.count(Side::Inactive),
            game_version: self.game_version(),
            overview: self.overview.clone(),
            details: self.details.clone(),
            warnings: self.order.errors_and_warnings().1,
            troubled: self.order.troubles().iter().any(|t| !t.is_empty()),
            dirty,
            unsaved,
            conflicts: self.reads.conflicts,
            selected_active: self.selected_active,
            panes: self.panes_wanted(),
            problems_open: self.problem_nav.open,
            position: self
                .selected_id()
                .and_then(|id| self.order.list(Side::Active).position_of(id))
                .map_or(0, |at| at + 1),
            picking: self.chosen_reads.picking,
            notes_saved: self.chosen_reads.notes_saved,
            files: self
                .chosen_reads
                .files
                .as_ref()
                .filter(|(of, _)| self.selected_id() == Some(*of))
                .map(|(_, read)| read.clone()),
            unhide_problem: self.status.unhide_problem.clone(),
            comparison: self.compared.clone(),
            sort_moves: self
                .order
                .previewing()
                .then(|| Rc::new(self.order.sort_moves())),
            chosen_id: self.selected_id(),
            fresh: self.reads.fresh,
            community_problem: match self.demo {
                true => String::new(),
                false => super::app(cx)
                    .community_problem()
                    .map(|why| crate::words::put(&why))
                    .unwrap_or_default(),
            },
            commits: self
                .chosen_reads
                .commits
                .as_ref()
                .filter(|(of, _)| self.selected_id() == Some(*of))
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
        self.rescanning = true;
        self.loading.run(
            "load order",
            cx,
            move |_| {
                let mut order = LoadOrder::read(&app, headings, inactive_by);
                order.set_active_by(active_by);
                order
            },
            move |mods, mut order, cx| {
                mods.rescanning = false;
                let text = mods.filter_text(cx);
                order.set_filter(&text);
                if anew {
                    let same = mods.order.same_mods_in(&order);
                    let kept = mods.order.carried(&same, &mods.selection);
                    mods.selection = kept.ids;
                    mods.selected_active = kept.active;
                }
                mods.order = order;
                if anew {
                    super::thumbs::renew(&mods.thumbs, cx);
                }
                mods.judge_unread(cx);
                mods.show(cx);
                mods.describe(cx);
                give_back_freed();
                mods.read_files(cx);
                mods.restore_left_off(cx);
                cx.emit(Progress::ListRead);
                if let Some(started) = crate::STARTED.get() {
                    rimstack_core::note!("mods_shown_ms={}", started.elapsed().as_millis());
                }
            },
        );
    }

    fn show(&mut self, cx: &mut Context<Self>) {
        let app = super::app(cx);
        self.standing = match self.demo {
            true => Standing::default(),
            false => app
                .session_if_read()
                .map_or_else(Standing::default, |session| Standing {
                    dirty: session.is_dirty(),
                    unsaved: crate::words::put(&session.changes().said()),
                    can_undo: session.can_undo(),
                    can_redo: session.can_redo(),
                }),
        };
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
        self.reads.fresh = if self.demo {
            0
        } else {
            self.order.fresh(&app, Side::Inactive).len()
        };
        self.reads.conflicts = if self.demo {
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

    pub fn session_standing(&self) -> &Standing {
        &self.standing
    }

    pub fn count(&self, side: Side) -> usize {
        self.order.list(side).mods().len()
    }

    pub fn toggle_category(&mut self, side: Side, category: Category, cx: &mut Context<Self>) {
        let app = super::app(cx);
        self.order.toggle_category(&app, side, category);
        self.show(cx);
    }

    pub fn toggle_sub(&mut self, side: Side, row: usize, cx: &mut Context<Self>) {
        let app = super::app(cx);
        self.order.toggle_sub(&app, side, row);
        self.show(cx);
    }

    pub fn choose(&mut self, side: Side, mod_id: ModId, cx: &mut Context<Self>) {
        let picks = self.chosen_reads.picking.is_some() && side == Side::Active;
        if picks && Some(mod_id) != self.selected_id() {
            return self.pick(mod_id, cx);
        }
        self.select(Some(mod_id), side == Side::Active, cx);
    }

    /// What the details show of the mod chosen, when one alone is.
    fn describe(&mut self, cx: &mut Context<Self>) {
        let app = super::app(cx);
        let alone = match self.selection.as_slice() {
            [one] => Some(*one),
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
                match found.mod_id {
                    Some(id) => self.choose(side, id, cx),
                    None => self.select(None, side == Side::Active, cx),
                }
                self.reveal_selected(side, false, cx);
            }
            _ => {}
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
                true => self.selection.iter().copied().collect(),
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
                dimmed: self.order.list(side).left_out()
                    == Some(rimstack_core::mods::list::LeftOut::Dimmed),
                single: self.selection.len() == 1,
                previewing: self.order.previewing(),
                picking: self.chosen_reads.picking.is_some(),
                count: self.order.list(side).mods().len(),
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

    pub fn is_chosen(&self, side: Side, mod_id: Option<ModId>) -> bool {
        mod_id.is_some_and(|id| {
            self.selected_active == (side == Side::Active) && self.selection.contains(&id)
        })
    }

    /// Four mods of the load order chosen, as `--several` asks.
    pub fn choose_several(&mut self, cx: &mut Context<Self>) {
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
            .and_then(|r| r.mod_id)
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

#[cfg(test)]
mod tests;

#[cfg(all(test, target_os = "linux"))]
mod edit_tests;

#[cfg(all(test, target_os = "linux"))]
mod details_tests;

#[cfg(all(test, target_os = "linux"))]
mod view_tests;

#[cfg(all(test, target_os = "linux"))]
mod layout_tests;
