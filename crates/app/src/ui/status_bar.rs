//! `StatusBar.qml`: what the list is, and the keys.

use super::theme::{self, Theme};
use super::widgets::{Btn, label};
use crate::words::put;
use gpui::{App as Cx, IntoElement, ParentElement, Styled, div, prelude::FluentBuilder, px};
use rimstack_core::say;

pub fn status_bar(demo: bool, cx: &Cx) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    let c = &theme.colours;
    let height = theme.strip() - px(2.);
    div()
        .flex()
        .flex_none()
        .items_center()
        .w_full()
        .h(theme.strip())
        .pl(theme::GAP_CARD)
        .pr(theme::GAP_MEDIUM)
        .bg(c.bar)
        .border_t_1()
        .border_color(c.edge)
        .child(div().flex_1().when(demo, |d| {
            d.child(label(theme, theme.small(), c.ink2, put(&say!("Demo data"))))
        }))
        .child(
            div()
                .flex()
                .items_center()
                .gap(theme::GAP_LARGE)
                .child(Btn::new("keys", put(&say!("? Keys"))).height(height))
                .when(!demo, |d| {
                    d.child(
                        Btn::new("history", put(&say!("History")))
                            .icon("history")
                            .height(height),
                    )
                }),
        )
}
