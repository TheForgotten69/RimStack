//! `DetailsPane.qml`: what is chosen, or the list as a whole.

use super::Mods;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{Chip, label};
use crate::words::{count, put};
use crate::workspace::load_order::Side;
use gpui::{Context, FontWeight, IntoElement, ParentElement, Styled, div, prelude::FluentBuilder};
use rimstack_core::say;

impl Mods {
    pub(super) fn details_pane(&self, theme: &Theme, _cx: &mut Context<Self>) -> gpui::Div {
        let c = &theme.colours;
        let title = match self.demo {
            true => put(&say!("Demo data")),
            false => put(&say!("Load order")),
        };
        div()
            .flex()
            .flex_col()
            .h_full()
            .bg(c.panel)
            .p(theme::GAP_CARD)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        label(theme, theme.heading(), c.ink, title).font_weight(FontWeight::BOLD),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .pt(theme::GAP_SMALL)
                            .gap(theme::GAP_TINY)
                            .when(!self.demo, |d| {
                                d.child(Chip::new(put(&say!("RimWorld {}", self.game_version()))))
                            })
                            .child(Chip::new(put(&say!(
                                "{} active",
                                count(self.count(Side::Active))
                            ))))
                            .child(Chip::new(put(&say!(
                                "{} inactive",
                                count(self.count(Side::Inactive))
                            )))),
                    ),
            )
    }
}
