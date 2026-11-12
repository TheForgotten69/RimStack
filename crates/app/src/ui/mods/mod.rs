//! `ModsScreen.qml`: the inactive mods, the load order and the details, side by side.

mod active;
mod details;
mod inactive;
mod rows;
mod scrollbar;

use super::theme::Theme;
use crate::jobs::Lane;
use crate::workspace::load_order::{LoadOrder, Orders, Overview, RowView, Side};
use gpui::{
    Context, IntoElement, ListAlignment, ListState, ParentElement, Pixels, Render, Styled, Window,
    div, px,
};
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

pub struct Mods {
    order: LoadOrder,
    active: Shown,
    inactive: Shown,
    orders: Orders,
    overview: Option<Overview>,
    /// `RowView::mod_id`s, the first chosen first.
    selection: Vec<i32>,
    selected_active: bool,
    demo: bool,
    loading: Lane,
    inactive_width: Pixels,
    details_width: Pixels,
}

impl Mods {
    pub fn new(demo: bool, cx: &mut Context<Self>) -> Self {
        let theme = cx.global::<Theme>();
        let (inactive_width, details_width) = (theme.sized(290.), theme.sized(360.));
        let order = if demo {
            LoadOrder::demo(2000, 300)
        } else {
            LoadOrder::demo(0, 0)
        };
        let mut mods = Self {
            orders: order.orders(),
            overview: None,
            order,
            active: Shown::new(),
            inactive: Shown::new(),
            selection: Vec::new(),
            selected_active: true,
            demo,
            loading: Lane::default(),
            inactive_width,
            details_width,
        };
        mods.show(cx);
        if !demo {
            mods.load(cx);
        }
        mods
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let app = super::app(cx);
        self.loading.run(
            "load order",
            cx,
            move |_| LoadOrder::read(&app, true, None),
            |mods, order, cx| {
                mods.order = order;
                mods.show(cx);
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
            shown
                .list
                .reset_with_uniform_height(rows.len() + 2, theme.row());
            shown.rows = Rc::new(rows);
        }
        self.orders = self.order.orders();
        self.overview = (!self.demo).then(|| self.order.overview(&app));
        cx.notify();
    }

    fn shown_mut(&mut self, side: Side) -> &mut Shown {
        match side {
            Side::Active => &mut self.active,
            Side::Inactive => &mut self.inactive,
        }
    }

    pub fn game_version(&self) -> String {
        self.overview
            .as_ref()
            .map(|o| o.game_version.clone())
            .unwrap_or_default()
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
        cx.notify();
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
        let active_width =
            window.viewport_size().width - self.inactive_width - self.details_width - px(14.);
        div()
            .flex()
            .size_full()
            .min_h_0()
            .child(
                self.inactive_pane(&theme, window, cx)
                    .w(self.inactive_width)
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
                self.details_pane(&theme, cx)
                    .w(self.details_width)
                    .flex_none(),
            )
    }
}
