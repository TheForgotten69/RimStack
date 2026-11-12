//! `StatusBar.qml`: what the list is, and the keys.

use super::theme::{self, Theme};
use super::widgets::{Btn, icon, label};
use crate::words::put;
use crate::workspace::logs::moment;
use gpui::{App as Cx, IntoElement, ParentElement, Styled, div, prelude::FluentBuilder, px};
use rimstack_core::{say, say_n};

/// `last`: when the game last started, and the errors its log held.
pub fn status_bar(demo: bool, last: Option<(u64, Option<usize>)>, cx: &Cx) -> impl IntoElement {
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
        .child(
            div()
                .flex()
                .flex_1()
                .items_center()
                .gap(theme::GAP_WIDE)
                .when(demo, |d| {
                    d.child(label(theme, theme.small(), c.ink2, put(&say!("Demo data"))))
                })
                .when_some(last, |d, (time, errors)| {
                    let started = put(&say!("Last start of the game {}", moment(time)));
                    let said = errors.map(|n| match n {
                        0 => (put(&say!("no errors")), c.ink2),
                        n => (
                            put(&say_n!(n, "{} different error", "{} different errors")),
                            c.err,
                        ),
                    });
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(theme::GAP_SMALL)
                            .child(label(theme, theme.small(), c.ink2, started))
                            .when_some(said, |d, (said, tone)| {
                                d.child(label(theme, theme.small(), tone, said))
                            })
                            .child(icon("chevron-right", theme.sized(11.), c.ink3)),
                    )
                }),
        )
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
