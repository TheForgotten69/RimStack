//! `InactivePane.qml`: the mods not in the load order.

use super::Mods;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{Btn, btn_width, label, measure};
use crate::words::{count, put};
use crate::workspace::load_order::Side;
use gpui::{Context, FontWeight, ParentElement, Styled, Window, div, prelude::FluentBuilder};
use rimstack_core::say;

impl Mods {
    pub(super) fn inactive_pane(
        &self,
        theme: &Theme,
        width: gpui::Pixels,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let c = &theme.colours;
        let title = put(&say!("Inactive"));
        let counted = count(self.count(Side::Inactive));
        let order = put(&self.orders.inactive_said);
        let hide = put(&say!("Hide"));
        // Qt's RowLayout gives its empty filler a gap too.
        let named = measure(window, theme, &title, theme.heading(), FontWeight::BOLD)
            + theme::GAP * 2.
            + measure(window, theme, &counted, theme.body(), FontWeight::NORMAL);
        let acts = btn_width(window, theme, &order, true, true)
            + theme::GAP
            + btn_width(window, theme, &hide, false, false);
        let room = width - theme::GAP_CARD - theme::GAP_LARGE;
        let stacked = named + acts + theme::GAP > room;
        let buttons = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(theme::GAP)
            .child(
                Btn::new("inactive-order", order)
                    .icon("arrow-up-down")
                    .menu(),
            )
            .child(Btn::new("hide-inactive", hide));
        div()
            .flex()
            .flex_col()
            .h_full()
            .bg(c.panel)
            .child(
                div()
                    .flex()
                    .when(stacked, |d| d.flex_col().items_end())
                    .when(!stacked, |d| d.items_center())
                    .pl(theme::GAP_CARD)
                    .pr(theme::GAP_LARGE)
                    .gap(theme::GAP)
                    .when(stacked, |d| d.gap_0())
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .w_full()
                            .items_center()
                            .gap(theme::GAP)
                            .h(theme.header())
                            .child(
                                label(theme, theme.heading(), c.ink, title)
                                    .font_weight(FontWeight::BOLD),
                            )
                            .child(label(theme, theme.body(), c.ink3, counted)),
                    )
                    .child(buttons.when(stacked, |d| d.mb(theme::GAP))),
            )
            .child(self.list_area(theme, Side::Inactive, None, width, cx))
    }
}
