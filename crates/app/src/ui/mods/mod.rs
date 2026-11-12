//! `ModsScreen.qml`: the inactive mods, the load order and the details, side by side.

mod active;
mod chosen;
mod details;
mod inactive;
mod rows;
mod scrollbar;

use super::text_field::{Edited, TextField};
use super::theme::Theme;
use crate::changes::Changes;
use crate::jobs::Lane;
use crate::workspace::load_order::{Details, LoadOrder, Orders, Overview, RowView, Side};
use gpui::{
    AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement, ListAlignment,
    ListState, ParentElement, Pixels, Render, Styled, Window, div, px,
};
use rimstack_core::mods::library::ModId;
use rimstack_core::mods::list::Category;
use std::rc::Rc;

/// A list's rows, with the spacer above them and the footer below that `ModList.qml` gives it.
pub struct Shown {
    pub rows: Rc<Vec<RowView>>,
    pub list: ListState,
    pub count: usize,
}

impl Shown {
    fn new() -> Self {
        Self {
            rows: Rc::new(Vec::new()),
            list: ListState::new(2, ListAlignment::Top, px(200.)),
            count: 0,
        }
    }
}

gpui::actions!(mods, [FocusFilter]);

pub struct Mods {
    order: LoadOrder,
    active: Shown,
    inactive: Shown,
    orders: Orders,
    filter: Entity<TextField>,
    focus: gpui::FocusHandle,
    overview: Option<Overview>,
    details: Option<Details>,
    tab: usize,
    /// A row to choose once the list is read: its place and the tab to show.
    wanted: Option<(usize, usize)>,
    summary: Option<crate::reports::Summary>,
    last_start: Option<crate::workspace::logs::LastStartSaid>,
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
}

impl Mods {
    pub fn new(demo: bool, cx: &mut Context<Self>) -> Self {
        let theme = cx.global::<Theme>();
        let (inactive_width, details_width, size) =
            (theme.sized(290.), theme.sized(360.), theme.body());
        let order = if demo {
            LoadOrder::demo(2000, 300)
        } else {
            LoadOrder::demo(0, 0)
        };
        let filter = cx.new(|cx| TextField::new(size, cx));
        cx.subscribe(&filter, |mods, field, _: &Edited, cx| {
            let text = field.read(cx).text().to_string();
            mods.order.set_filter(&text);
            mods.show(cx);
        })
        .detach();
        let mut mods = Self {
            focus: cx.focus_handle(),
            filter,
            orders: order.orders(),
            overview: None,
            details: None,
            tab: 0,
            wanted: None,
            summary: None,
            last_start: None,
            order,
            active: Shown::new(),
            inactive: Shown::new(),
            selection: Vec::new(),
            selected_active: true,
            demo,
            loading: Lane::default(),
            counting: Lane::default(),
            files_read: false,
            sort_pending: 0,
            inactive_width,
            details_width,
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
            self.summary = Some(app.summary());
        }
        if changes
            .contains(Changes::LAUNCHES | Changes::LOGS | Changes::FOLDERS | Changes::LANGUAGE)
        {
            self.last_start = app.last_start_said();
        }
    }

    /// `anew`: the mods were read again, so every id held before means nothing.
    fn load(&mut self, anew: bool, cx: &mut Context<Self>) {
        let app = super::app(cx);
        self.loading.run(
            "load order",
            cx,
            move |_| LoadOrder::read(&app, true, None),
            move |mods, mut order, cx| {
                let text = mods.filter.read(cx).text().to_string();
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
                mods.read_files(cx);
                if let Some((row, tab)) = mods.wanted.take() {
                    mods.pick_row(Side::Active, row, tab, cx);
                }
                if let Some(started) = crate::STARTED.get() {
                    rimstack_core::note!("mods_shown_ms={}", started.elapsed().as_millis());
                }
            },
        );
    }

    fn show(&mut self, cx: &mut Context<Self>) {
        let app = super::app(cx);
        let theme = cx.global::<Theme>().clone();
        for side in [Side::Active, Side::Inactive] {
            let rows = self.order.rows(&app, side);
            let shown = self.shown_mut(side);
            shown.count = rows.len();
            // A reset scrolls to the top: the place kept is given back.
            let kept = shown.list.logical_scroll_top();
            shown
                .list
                .reset_with_uniform_height(rows.len() + 2, theme.row());
            shown.list.scroll_to(kept);
            shown.rows = Rc::new(rows);
        }
        self.orders = self.order.orders();
        if self.details.is_some() {
            self.describe(cx);
        }
        self.overview = (!self.demo).then(|| self.order.overview(&app));
        if !self.demo {
            self.count_sort_moves(cx);
        }
        cx.notify();
    }

    fn shown_mut(&mut self, side: Side) -> &mut Shown {
        match side {
            Side::Active => &mut self.active,
            Side::Inactive => &mut self.inactive,
        }
    }

    /// What Play warns of: how many things are in the way, and whether a mod has errors.
    pub fn in_the_way(&self, cx: &gpui::App) -> (usize, bool) {
        if self.demo {
            return (0, false);
        }
        let (errors, _) = self.order.errors_and_warnings();
        let dirty = super::app(cx).session().is_dirty();
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

    pub fn filter_by(&mut self, text: &str, cx: &mut Context<Self>) {
        self.filter.update(cx, |field, cx| field.set_text(text, cx));
    }

    pub fn filter_text(&self, cx: &gpui::App) -> String {
        self.filter.read(cx).text().to_string()
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
        self.selection = vec![mod_id];
        self.selected_active = side == Side::Active;
        self.describe(cx);
        cx.notify();
    }

    /// What the details show of the mod chosen, when one alone is.
    fn describe(&mut self, cx: &mut Context<Self>) {
        let app = super::app(cx);
        let alone = match self.selection.as_slice() {
            [one] => rimstack_core::mods::library::ModId::from_qml(*one),
            _ => None,
        };
        self.details = alone
            .filter(|_| self.order.is_installed())
            .map(|id| self.order.details(&app, id));
    }

    /// Chooses a row of a list by its place, headings counted, as `--pick` does.
    pub fn pick_row(&mut self, side: Side, row: usize, tab: usize, cx: &mut Context<Self>) {
        let rows = match side {
            Side::Active => self.active.rows.clone(),
            Side::Inactive => self.inactive.rows.clone(),
        };
        match rows.get(row) {
            Some(found) if !found.is_header => {
                self.tab = tab;
                self.choose(side, found.mod_id, cx);
            }
            Some(_) => {}
            None => self.wanted = Some((row, tab)),
        }
    }

    pub fn is_chosen(&self, side: Side, mod_id: i32) -> bool {
        mod_id >= 0
            && self.selected_active == (side == Side::Active)
            && self.selection.contains(&mod_id)
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
        match also {
            true => self.selection.push(id),
            false => self.selection = vec![id],
        }
        self.selected_active = side == Side::Active;
        cx.notify();
    }
}

impl Mods {
    /// The three panes' widths as Qt's SplitView gives them: the load order keeps its least width,
    /// taken from the details first, then from the inactive mods, each down to its own least.
    fn widths(&self, theme: &Theme, window: Pixels) -> (Pixels, Pixels, Pixels) {
        let room = window - px(14.);
        let (mut inactive, mut details) = (self.inactive_width, self.details_width);
        let mut over = inactive + details + theme.sized(380.) - room;
        if over > px(0.) {
            let cut = over.min((details - theme.sized(260.)).max(px(0.)));
            details -= cut;
            over -= cut;
        }
        if over > px(0.) {
            inactive -= over.min((inactive - theme.sized(200.)).max(px(0.)));
        }
        (inactive, (room - inactive - details).max(px(0.)), details)
    }
}

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
                .w(px(7.))
                .h_full()
                .child(div().w(px(1.)).h_full().bg(theme.colours.edge))
        };
        let (inactive_width, active_width, details_width) =
            self.widths(&theme, window.viewport_size().width);
        div()
            .flex()
            .size_full()
            .min_h_0()
            .track_focus(&self.focus)
            .on_action(cx.listener(|mods, _: &FocusFilter, window, cx| {
                window.focus(&mods.filter.focus_handle(cx), cx);
            }))
            .child(
                self.inactive_pane(&theme, inactive_width, window, cx)
                    .w(inactive_width)
                    .flex_none(),
            )
            .child(handle())
            .child(
                self.active_pane(&theme, active_width, window, cx)
                    .flex_1()
                    .min_w_0(),
            )
            .child(handle())
            .child(
                self.details_pane(&theme, details_width, window, cx)
                    .w(details_width)
                    .flex_none(),
            )
    }
}

#[cfg(test)]
mod tests;
