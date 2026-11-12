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

/// What each section holds, as its tab's tip says.
fn about_section(at: usize) -> String {
    put(&match at {
        0 => say!("The load order and every installed mod"),
        1 => say!("Workshop updates, looked-up mods and Git checkouts"),
        2 => say!("Faster-loading DDS textures and the files the game would refuse"),
        3 => say!("Browse the Workshop's mods and subscribe in Steam"),
        4 => say!("Saves and how their mods differ from the load order"),
        5 => say!("The game's log: errors and the mods they point at"),
        6 => say!("Words and patterns in the files of the mods"),
        _ => say!("Folders, appearance, sorting and upkeep"),
    })
}

/// The sections as a menu, from the one tab shown while they are folded.
fn pick_section(
    below: gpui::Bounds<Pixels>,
    section: usize,
    window: &mut Window,
    cx: &mut Context<Shell>,
) {
    let me = cx.entity().downgrade();
    let entries = sections()
        .into_iter()
        .enumerate()
        .map(|(at, name)| {
            let me = me.clone();
            super::menu::Entry::item(name, move |_, cx| {
                let _ = me.update(cx, |shell, cx| shell.go(at, cx));
            })
            .off(at == section)
            .into()
        })
        .collect();
    super::menu::open(below, entries, window, cx);
}

/// `in_the_way`: what Play warns of, as `Mods::in_the_way` counts it.
/// The section whose tab counts the active mods with a newer Workshop version.
const DOWNLOADS: usize = 1;

/// What the top bar shows of the window's state.
pub struct TopState {
    pub section: usize,
    pub demo: bool,
    pub in_the_way: (usize, bool),
    pub behind: usize,
    pub running: bool,
    pub lists_view: gpui::Entity<super::lists::Lists>,
}

pub fn top_bar(state: TopState, window: &Window, cx: &mut Context<Shell>) -> impl IntoElement {
    let TopState {
        section,
        demo,
        in_the_way,
        behind,
        running,
        lists_view,
    } = state;
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
    let badge = (behind > 0).then(|| {
        let said = behind.to_string();
        let wide = measure(window, &theme, &said, theme.caption(), theme.semibold);
        (said, theme.tag().max(wide + theme::GAP_SMALL * 2.))
    });
    let badge_room = |at: usize| match (&badge, at == DOWNLOADS) {
        (Some((_, wide)), true) => *wide + theme::GAP_SMALL,
        _ => px(0.),
    };
    let natural: Vec<Pixels> = names
        .iter()
        .enumerate()
        .map(|(at, name)| {
            measure(window, &theme, name, theme.body(), weight(at)) + inset * 2. + badge_room(at)
        })
        .collect();
    let brand = put(&say!("RimStack"));
    let list_name = super::app(cx)
        .prefs()
        .get("Mods", "listName")
        .unwrap_or_default();
    let lists = match list_name.is_empty() {
        true => put(&say!("Lists")),
        false => put(&say!("List: {}", list_name)),
    };
    let dirty = !demo && super::app(cx).session().is_dirty();
    let play = match running {
        true => put(&say!("Running")),
        false => put(&say!("Play")),
    };
    let play_tip = match (running, in_the_way.0) {
        (true, _) => put(&say!(
            "RimWorld is running. It reads the list of mods only when it starts."
        )),
        (false, 0) => put(&say!(
            "Starts RimWorld through Steam. The newest save is copied first."
        )),
        (false, _) => String::new(),
    };
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
    let placed: std::rc::Rc<std::cell::Cell<gpui::Bounds<Pixels>>> = Default::default();
    let placed_box = placed.clone();
    let tabs = shown.into_iter().map(|at| {
        let on = at == section;
        let ink = if on { c.ink } else { c.ink2 };
        let tip = format!(
            "{}\n{}",
            put(&say!("{} · Ctrl + {}", names[at].clone(), at + 1)),
            about_section(at)
        );
        let (held, kept) = (placed.clone(), placed.clone());
        let tab = div()
            .id(("section", at))
            .role(gpui::Role::Tab)
            .aria_label(names[at].clone())
            .aria_selected(on)
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
            .when_some(
                badge.clone().filter(|_| at == DOWNLOADS),
                |d, (said, wide)| {
                    d.child(
                        div()
                            .absolute()
                            .right(theme::GAP)
                            .top_0()
                            .bottom_0()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(wide)
                                    .h(theme.sized(16.))
                                    .rounded(theme.sized(8.))
                                    .bg(c.accent)
                                    .child(
                                        label(&theme, theme.caption(), c.accent_ink, said)
                                            .font_weight(theme.semibold),
                                    ),
                            ),
                    )
                },
            )
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
            .child(
                gpui::canvas(move |bounds, _, _| held.set(bounds), |_, _, _, _| {})
                    .absolute()
                    .inset_0(),
            )
            .on_click(cx.listener(move |shell, _, window, cx| match folded {
                true => pick_section(kept.get(), section, window, cx),
                false => shell.go(at, cx),
            }));
        super::tip::tipped(tab, tip.into())
    });
    let search = div()
        .id("top-search")
        .role(gpui::Role::Button)
        .aria_label(put(&say!("Search mods, commands and settings")))
        .tab_index(0)
        .capture_any_mouse_down(|_, window, _| window.prevent_default())
        .focus_visible(|s| s.border_2().border_color(c.focus))
        .cursor_text()
        .on_click(cx.listener(|shell, _, window, cx| shell.open_commands(window, cx)))
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
    let search = super::tip::tipped(
        search,
        put(&say!(
            "Finds mods in both lists, commands and settings (Ctrl + K)"
        ))
        .into(),
    );
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
                        .id("sections")
                        .role(gpui::Role::TabList)
                        .aria_label(put(&say!("Sections")))
                        .tab_index(0)
                        .capture_any_mouse_down(|_, window, _| window.prevent_default())
                        .focus_visible(|s| s.border_2().border_color(c.focus))
                        .on_key_down(cx.listener(move |shell, event: &gpui::KeyDownEvent, window, cx| {
                            let keys = &event.keystroke;
                            if keys.modifiers.modified() {
                                return;
                            }
                            match keys.key.as_str() {
                                "left" => shell.go(section.saturating_sub(1), cx),
                                "right" => shell.go((section + 1).min(sections().len() - 1), cx),
                                "enter" | "space" if folded => {
                                    pick_section(placed_box.get(), section, window, cx)
                                }
                                _ => return,
                            }
                            cx.stop_propagation();
                        }))
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
                    d.child(
                        div()
                            .relative()
                            .child(
                                Btn::new("lists", lists)
                                    .icon("list")
                                    .tip(put(&say!("Load orders kept under a name, and the order as it was before each Sort")))
                                    .opens(move |below, window, cx| {
                                        lists_view.update(cx, |lists, cx| {
                                            lists.toggle(super::lists::Which::Lists, below, window, cx)
                                        })
                                    }),
                            )
                            .when(dirty, |d| {
                                let dot = div()
                                    .id("unsaved-dot")
                                    .absolute()
                                    .top(px(-3.))
                                    .right(px(-3.))
                                    .size(px(8.))
                                    .rounded(px(4.))
                                    .bg(c.warn);
                                d.child(super::tip::tipped(
                                    dot,
                                    put(&say!("The list has changes that are not saved")).into(),
                                ))
                            }),
                    )
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
                        .child(
                            Btn::new("play", play)
                                .icon("play")
                                .primary()
                                .large()
                                .enabled(!running)
                                .tip(play_tip)
                                .on_click(cx.listener(|shell, _, _, cx| shell.press_play(cx))),
                        )
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
