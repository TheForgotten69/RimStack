//! `TopBar.qml`: the app's name, the sections, Search and Play.

use super::shell::Shell;
use super::theme::{self, Theme};
use super::widgets::{Btn, halo, icon, label, with_alpha};
use crate::words::put;
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div, img, prelude::FluentBuilder, px,
};
use rimstack_core::say;

pub fn sections() -> [String; 8] {
    [
        put(&say!("Mods")),
        put(&say!("Downloads")),
        put(&say!("Textures")),
        put(&say!("Workshop")),
        put(&say!("Saves")),
        put(&say!("Logs")),
        put(&say!("Search")),
        put(&say!("Settings")),
    ]
}

pub fn top_bar(section: usize, demo: bool, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.global::<Theme>().clone();
    let c = theme.colours;
    let tabs = sections().into_iter().enumerate().map(|(at, name)| {
        let on = at == section;
        div()
            .id(("section", at))
            .relative()
            .flex()
            .items_center()
            .h(theme.strip())
            .px(theme::GAP_CARD)
            .rounded(theme::CORNER)
            .when(on, |d| {
                d.bg(c.ctl_hi)
                    .border_1()
                    .border_color(with_alpha(c.accent, 0.7))
                    .children(halo(c.accent, theme::CORNER))
            })
            .child(
                label(&theme, theme.body(), if on { c.ink } else { c.ink2 }, name)
                    .font_weight(if on { theme.semibold } else { theme.medium }),
            )
            .on_click(cx.listener(move |shell, _, _, cx| shell.go(at, cx)))
    });
    let search = div()
        .flex()
        .flex_none()
        .items_center()
        .w(theme.sized(250.))
        .h(theme.field())
        .px(theme::GAP)
        .gap(theme::GAP)
        .rounded(theme::CORNER)
        .bg(c.well)
        .border_1()
        .border_color(c.field_line)
        .child(icon("search", theme.icon() - px(1.), c.ink3))
        .child(label(&theme, theme.body(), c.ink3, put(&say!("Search"))).flex_1())
        .child(label(
            &theme,
            theme.caption(),
            c.ink3,
            put(&say!("Ctrl + K")),
        ));
    div()
        .relative()
        .flex_none()
        .w_full()
        .h(theme.top_bar())
        .bg(c.bar)
        .border_b_1()
        .border_color(c.edge)
        .child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .gap(theme::GAP_LINE)
                        .p(px(2.))
                        .rounded(theme::CORNER_LARGE)
                        .bg(c.seg)
                        .when(theme.appearance.high_contrast, |d| {
                            d.border_1().border_color(c.field_line)
                        })
                        .children(tabs),
                ),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(theme::GAP_LARGE)
                .flex()
                .items_center()
                .gap(theme::GAP)
                .child(img("rimstack.svg").size(theme.sized(20.)).flex_none())
                .child(
                    label(&theme, theme.title(), c.brand, put(&say!("RimStack")))
                        .font_weight(theme.semibold)
                        .pr(theme::GAP_SMALL),
                )
                .when(!demo, |d| {
                    d.child(Btn::new("lists", put(&say!("Lists"))).icon("list").menu())
                }),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right(theme::GAP_CARD)
                .flex()
                .items_center()
                .gap(theme::GAP_LARGE)
                .child(search)
                .child(
                    Btn::new("play", put(&say!("Play")))
                        .icon("play")
                        .primary()
                        .large(),
                ),
        )
}
