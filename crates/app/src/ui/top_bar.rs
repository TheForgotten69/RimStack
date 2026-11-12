//! `TopBar.qml`: the app's name, the sections, Search and Play.

use super::shell::Shell;
use super::theme::{self, Theme};
use super::widgets::{
    Btn, TextSized, btn_width, btn_width_of, halo, icon, label, measure, with_alpha,
};
use crate::words::{count, put};
use gpui::{
    Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Pixels,
    StatefulInteractiveElement, Styled, Window, div, img, prelude::FluentBuilder, px,
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

/// `in_the_way`: what Play warns of, as `Mods::in_the_way` counts it.
pub fn top_bar(
    section: usize,
    demo: bool,
    in_the_way: (usize, bool),
    window: &Window,
    cx: &mut Context<Shell>,
) -> impl IntoElement {
    let theme = cx.global::<Theme>().clone();
    let c = theme.colours;
    let width = window.viewport_size().width;
    let tight = width < theme.sized(960.);
    let inset = if tight { theme::GAP } else { theme::GAP_CARD };
    let names = sections();
    let weight = |at: usize| {
        if at == section {
            theme.semibold
        } else {
            theme.medium
        }
    };
    let natural: Vec<Pixels> = names
        .iter()
        .enumerate()
        .map(|(at, name)| measure(window, &theme, name, theme.body(), weight(at)) + inset * 2.)
        .collect();
    let brand = put(&say!("RimStack"));
    let lists = put(&say!("Lists"));
    let play = put(&say!("Play"));
    let left_end = theme::GAP_LARGE
        + theme.sized(20.)
        + match tight {
            true => px(0.),
            false => {
                theme::GAP
                    + measure(window, &theme, &brand, theme.title(), theme.semibold)
                    + theme::GAP_SMALL
            }
        }
        + match demo {
            true => px(0.),
            false => theme::GAP + btn_width(window, &theme, &lists, true, true),
        };
    let play_width = btn_width_of(window, &theme, &play, true, false, true);
    let all_tabs = natural
        .iter()
        .fold(px(0.), |all, tab| all + *tab + theme::GAP_LINE);
    let folded = left_end
        + theme::GAP_MEDIUM
        + all_tabs
        + theme::GAP_CARD
        + theme.field()
        + theme::GAP_SMALL
        + theme::GAP_LARGE
        + play_width
        + theme::GAP_CARD
        > width;
    let tab_width = |at: usize| {
        natural[at]
            + if folded {
                theme.sized(12.) + theme::GAP_TINY
            } else {
                px(0.)
            }
    };
    let shown: Vec<usize> = (0..names.len())
        .filter(|at| !folded || *at == section)
        .collect();
    let nav = shown.iter().fold(px(0.), |all, at| all + tab_width(*at))
        + theme::GAP_LINE * shown.len().saturating_sub(1) as f32;
    let box_width = nav + px(4.);
    let room = width
        - theme::GAP_CARD
        - play_width
        - theme::GAP_LARGE
        - (left_end + theme::GAP_MEDIUM + box_width + theme::GAP_MEDIUM);
    let small = room < theme.sized(130.);
    let search_width = if small {
        theme.field() + px(6.)
    } else {
        theme.sized(250.).min(room)
    };
    let right_x = width - theme::GAP_CARD - (search_width + theme::GAP_LARGE + play_width);
    let box_x = (left_end + theme::GAP_MEDIUM)
        .max(((width - box_width) / 2.).min(right_x - box_width - theme::GAP_MEDIUM));
    let tabs = shown.into_iter().map(|at| {
        let on = at == section;
        let ink = if on { c.ink } else { c.ink2 };
        div()
            .id(("section", at))
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .w(tab_width(at))
            .h(theme.strip())
            .pl(inset)
            .rounded(theme::CORNER)
            .when(on, |d| {
                d.bg(c.ctl_hi)
                    .border_1()
                    .border_color(with_alpha(c.accent, 0.7))
                    .children(halo(c.accent, theme::CORNER))
            })
            .child(label(&theme, theme.body(), ink, names[at].clone()).font_weight(weight(at)))
            .when(folded, |d| {
                d.child(
                    div()
                        .absolute()
                        .right(theme::GAP)
                        .top_0()
                        .bottom_0()
                        .flex()
                        .items_center()
                        .child(icon("chevron-down", theme.sized(12.), ink)),
                )
            })
            .on_click(cx.listener(move |shell, _, _, cx| shell.go(at, cx)))
    });
    let search = div()
        .flex()
        .flex_none()
        .items_center()
        .w(search_width)
        .h(theme.field())
        .px(theme::GAP)
        .gap(theme::GAP)
        .rounded(theme::CORNER)
        .bg(c.well)
        .border_1()
        .border_color(c.field_line)
        .child(icon("search", theme.icon() - px(1.), c.ink3))
        .when(!small, |d| {
            d.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .sized_text(&theme, theme.body())
                    .text_color(c.ink3)
                    .child(put(&say!("Search"))),
            )
            .child(label(
                &theme,
                theme.caption(),
                c.ink3,
                put(&say!("Ctrl + K")),
            ))
        });
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
                .top_0()
                .bottom_0()
                .left(box_x)
                .flex()
                .items_center()
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
                .when(!tight, |d| {
                    d.child(
                        label(&theme, theme.title(), c.brand, brand)
                            .font_weight(theme.semibold)
                            .pr(theme::GAP_SMALL),
                    )
                })
                .when(!demo, |d| {
                    d.child(Btn::new("lists", lists).icon("list").menu())
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
                    div()
                        .relative()
                        .child(Btn::new("play", play).icon("play").primary().large())
                        .when(in_the_way.0 > 0, |d| {
                            let said = count(in_the_way.0);
                            let height = theme.sized(17.);
                            let wide = height.max(
                                measure(window, &theme, &said, theme.caption(), FontWeight::BOLD)
                                    + theme.sized(8.),
                            );
                            d.child(
                                div()
                                    .absolute()
                                    .top(-theme.sized(6.))
                                    .right(-theme.sized(6.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(wide)
                                    .h(height)
                                    .rounded(height / 2.)
                                    .bg(if in_the_way.1 { c.err } else { c.warn })
                                    .border_2()
                                    .border_color(c.bar)
                                    .child(
                                        label(&theme, theme.caption(), c.win, said)
                                            .font_weight(FontWeight::BOLD),
                                    ),
                            )
                        }),
                ),
        )
}
