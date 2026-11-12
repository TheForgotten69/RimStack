//! `ActivePane.qml` and `Columns.qml`: the load order, its filter and its column headings.

use super::Mods;
use super::list::bar_room;
use super::rows::Columns;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{Btn, Chip, TextSized, btn_width, icon, label, measure, notice};
use crate::words::{count, put};
use crate::workspace::load_order::{Orders, Side};
use gpui::{
    Context, FontWeight, ParentElement, Pixels, Styled, Window, div, prelude::FluentBuilder, px,
};
use rimstack_core::{say, say_n};
use std::rc::Rc;

/// The columns chosen in the View menu, before any is left out for room.
const CHOSEN: [&str; 3] = ["author", "version", "load"];
/// The order columns are left out in when the name would have too little room.
const DROPPED_FIRST: [&str; 7] = [
    "size",
    "downloaded",
    "activated",
    "updated",
    "load",
    "author",
    "version",
];

impl Mods {
    fn heading_label(&self, key: &str) -> String {
        self.orders
            .active_by
            .get(key)
            .map(|entry| put(&entry.label))
            .unwrap_or_default()
    }

    /// `Columns.widths` and `ModList.fitting` for a list this wide.
    fn columns(&self, theme: &Theme, width: Pixels, window: &Window) -> Columns {
        columns(theme, &self.orders, width, window)
    }

    pub(super) fn active_pane(
        &self,
        theme: &Theme,
        width: Pixels,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let c = &theme.colours;
        let columns = Rc::new(self.columns(theme, width, window));
        let heading = |text: String, on: bool| {
            label(
                theme,
                theme.caption(),
                if on { c.ink } else { c.ink3 },
                text,
            )
            .font_weight(theme.medium)
        };
        let name_on = self.orders.active_by.get("name").is_some_and(|e| e.on);
        let headings = div()
            .relative()
            .flex_none()
            .h(theme.control())
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .pl(theme::GAP_WIDE)
                    .pr(theme::GAP_WIDE + bar_room(theme))
                    .gap(theme::GAP_LARGE)
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .justify_end()
                            .w(columns.number)
                            .child(heading("#".into(), false)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(heading(self.heading_label("name"), name_on)),
                    )
                    .children(columns.fitting.iter().map(|(key, width)| {
                        let on = self.orders.active_by.get(key).is_some_and(|e| e.on);
                        div()
                            .relative()
                            .flex()
                            .flex_none()
                            .h_full()
                            .items_center()
                            .w(*width)
                            .when(matches!(*key, "load" | "size"), |d| d.justify_end())
                            .child(
                                div()
                                    .absolute()
                                    .left(-(theme::GAP_LARGE / 2.))
                                    .top(px(2.))
                                    .bottom(px(2.))
                                    .w(px(1.))
                                    .bg(c.field_line),
                            )
                            .child(heading(self.heading_label(key), on))
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .justify_end()
                            .w(columns.quick)
                            .child(Btn::new("view", put(&say!("View"))).menu()),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .bottom_0()
                    .left(theme::GAP_SMALL)
                    .right(theme::GAP_SMALL)
                    .h(px(1.))
                    .bg(c.line),
            );
        div()
            .flex()
            .flex_col()
            .h_full()
            .child(self.head(theme, width, window, cx))
            .children(self.notices(theme, width, window))
            .child(
                div()
                    .flex_none()
                    .px(theme::GAP_LARGE)
                    .pb(theme::GAP)
                    .child(self.filter_field(theme, window, cx)),
            )
            .child(headings)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(self.list(Side::Active).clone()),
            )
    }

    /// The pane's title row: the count, the problems, Undo and Redo, Sort and Save.
    fn head(
        &self,
        theme: &Theme,
        width: Pixels,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let c = &theme.colours;
        let app = crate::ui::app(cx);
        let (errors, warnings) = self.order.errors_and_warnings();
        let problems = self
            .order
            .troubles()
            .iter()
            .filter(|t| !t.is_empty())
            .count();
        let previewing = self.order.previewing();
        let (version_only, other_warnings) = self
            .overview
            .as_ref()
            .map_or((0, 0), |o| (o.version_only, o.other_warnings));
        let version_line = version_only >= 3 && !previewing && !self.demo;
        let warnings_shown = if version_line {
            other_warnings
        } else {
            warnings
        };
        let (dirty, can_undo, can_redo) = match self.demo {
            true => (false, false, false),
            false => {
                let session = app.session();
                (session.is_dirty(), session.can_undo(), session.can_redo())
            }
        };
        let title = put(&say!("Active"));
        let counted = count(self.count(Side::Active));
        let sort = match (self.files_read, self.sort_pending) {
            (false, _) => put(&say!("Reading files…")),
            (true, 0) => put(&say!("Sort")),
            (true, n) => put(&say_n!(n, "Sort · {} move", "Sort · {} moves")),
        };
        let save = put(&say!("Save"));
        let full = |text: &str| {
            measure(window, theme, text, theme.small(), theme.medium)
                + theme.icon()
                + theme::GAP_SMALL
                + theme.sized(20.)
        };
        let head_wide = theme::GAP_CARD
            + theme::GAP_LARGE
            + measure(window, theme, &title, theme.heading(), FontWeight::BOLD)
            + measure(window, theme, &counted, theme.body(), FontWeight::NORMAL)
            + full(&sort)
            + full(&save)
            + theme::GAP * 4.;
        let warned = match version_line {
            true => put(&say_n!(
                warnings_shown,
                "{} more warning",
                "{} more warnings"
            )),
            false => put(&say_n!(warnings_shown, "{} warning", "{} warnings")),
        };
        let only_warnings = errors == 0;
        let problems_said = match only_warnings {
            true => put(&say_n!(problems, "{} warning", "{} warnings")),
            false => put(&say_n!(
                problems,
                "{} problem · {} with errors",
                "{} problems · {} with errors",
                count(errors)
            )),
        };
        let problems_button = !(only_warnings && warnings_shown > 0 && warnings_shown == problems);
        let chip_width = |text: &str| {
            theme.sized(11.)
                + theme::GAP_TINY
                + measure(window, theme, text, theme.caption(), theme.medium)
                + theme::GAP_CARD
        };
        let icon_button = theme.icon() + theme.sized(14.);
        let extras = if warnings_shown > 0 {
            chip_width(&warned) + theme::GAP
        } else {
            px(0.)
        } + if problems > 0 {
            let said = if problems_button {
                btn_width(window, theme, &problems_said, true, false) + theme::GAP_LINE
            } else {
                px(0.)
            };
            said + icon_button * 2. + theme::GAP_LINE + theme::GAP
        } else {
            px(0.)
        } + if self.demo {
            px(0.)
        } else {
            icon_button * 2. + theme::GAP_LINE + theme::GAP
        };
        let updates = put(&say_n!(self.behind, "{} update", "{} updates"));
        let extras = extras
            + if self.behind > 0 {
                btn_width(window, theme, &updates, false, false) + theme::GAP
            } else {
                px(0.)
            };
        let tight = width < head_wide + extras;
        let problem_tone = if errors > 0 { c.err } else { c.warn };
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(theme::GAP)
            .h(theme.header())
            .pl(theme::GAP_CARD)
            .pr(theme::GAP_LARGE)
            .child(label(theme, theme.heading(), c.ink, title).font_weight(FontWeight::BOLD))
            .child(label(theme, theme.body(), c.ink3, counted))
            .when(warnings_shown > 0 && !tight, |d| {
                d.child(
                    Chip::new(warned)
                        .tone(c.warn)
                        .icon("triangle-alert")
                        .pressable(),
                )
            })
            .when(problems > 0 && !tight, |d| {
                d.child(
                    div()
                        .flex()
                        .gap(theme::GAP_LINE)
                        .when(problems_button, |d| {
                            d.child(
                                Btn::new("problems", problems_said)
                                    .icon("triangle-alert")
                                    .tone(problem_tone),
                            )
                        })
                        .child(Btn::new("problem-up", "").icon("chevron-up"))
                        .child(Btn::new("problem-down", "").icon("chevron-down")),
                )
            })
            .child(div().flex_1())
            .when(self.behind > 0 && !tight, |d| {
                d.child(
                    Btn::new("updates", updates)
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(super::ShowSection(1)))),
                )
            })
            .when(!self.demo && !tight, |d| {
                d.child(
                    div()
                        .flex()
                        .gap(theme::GAP_LINE)
                        .child(
                            Btn::new("undo", "")
                                .icon("undo-2")
                                .enabled(can_undo && !previewing),
                        )
                        .child(
                            Btn::new("redo", "")
                                .icon("redo-2")
                                .enabled(can_redo && !previewing),
                        ),
                )
            })
            .when(!self.demo, |d| {
                d.child(
                    Btn::new("sort", sort)
                        .icon("arrow-up-down")
                        .tone(c.moved)
                        .enabled(!previewing && self.files_read),
                )
                .child(
                    Btn::new("save", save)
                        .icon("save")
                        .enabled(dirty)
                        .primary_if(dirty),
                )
            })
            .when(tight && !self.demo, |d| {
                let more = Btn::new(
                    "more",
                    if problems > 0 {
                        count(problems)
                    } else {
                        String::new()
                    },
                )
                .icon("ellipsis")
                .menu();
                d.child(if problems > 0 {
                    more.tone(problem_tone)
                } else {
                    more
                })
            })
    }

    /// The bands under the title: what is not saved, and mods made for another game version.
    fn notices(&self, theme: &Theme, width: Pixels, window: &Window) -> Vec<gpui::Div> {
        let c = &theme.colours;
        let mut shown = Vec::new();
        if self.demo {
            return shown;
        }
        let version_only = self.overview.as_ref().map_or(0, |o| o.version_only);
        if version_only >= 3 && !self.order.previewing() {
            let version = self.game_version();
            let said = put(&say_n!(
                version_only,
                "{} mod does not list RimWorld {}",
                "{} mods do not list RimWorld {}",
                version.clone()
            ));
            let show = put(&say!("Show"));
            let ignore = put(&say!("Ignore for {}", version));
            let does = btn_width(window, theme, &show, false, false)
                + theme::GAP
                + btn_width(window, theme, &ignore, false, false);
            let room = width - theme::GAP_LARGE * 3. - theme::GAP_TINY;
            let stacked = theme.icon()
                + measure(window, theme, &said, theme.body(), FontWeight::NORMAL)
                + does
                + theme::GAP * 2.
                > room;
            let buttons = div()
                .flex()
                .flex_none()
                .justify_end()
                .gap(theme::GAP)
                .child(Btn::new("show-warnings", show))
                .child(Btn::new("ignore-version", ignore));
            shown.push(
                div().flex_none().px(theme::GAP_LARGE).pb(theme::GAP).child(
                    notice(c.warn).child(
                        div()
                            .flex()
                            .when(stacked, |d| d.flex_col().gap(theme::GAP_SMALL))
                            .when(!stacked, |d| d.items_center().gap(theme::GAP))
                            .pl(theme::GAP_LARGE)
                            .pr(theme::GAP_TINY)
                            .py(theme::GAP_SMALL)
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(theme::GAP)
                                    .child(icon("triangle-alert", theme.icon(), c.warn).mt(px(2.)))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .sized_text(theme, theme.body())
                                            .text_color(c.ink)
                                            .child(said),
                                    ),
                            )
                            .child(buttons),
                    ),
                ),
            );
        }
        shown
    }

    /// `ModsFilter.qml` and `FilterField.qml`: the filter, its count of what it lets through, and its clear button.
    fn filter_field(
        &self,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let typed = !self.filter.read(cx).text().is_empty();
        crate::ui::field::Field {
            id: "filter",
            text: &self.filter,
            icon: "funnel",
            placeholder: put(&say!("Filter active and inactive mods")),
            found: typed.then(|| put(&self.order.filter_summary())),
            keys: Some(put(&say!("Ctrl + F"))),
        }
        .render(theme, window, cx)
    }
}

/// `Columns.widths` and `ModList.fitting` for a load order this wide.
pub(super) fn columns(theme: &Theme, orders: &Orders, width: Pixels, window: &Window) -> Columns {
    let heading = |key: &str| {
        let text = format!("{} ▾", put(&column_name(key)));
        measure(window, theme, &text, theme.caption(), theme.medium)
    };
    let widest = |key: &str, size: Pixels| {
        let text = orders.widest.get(key).cloned().unwrap_or_default();
        measure(window, theme, &text, size, FontWeight::NORMAL)
    };
    let column_width = |key: &str| match key {
        "author" => heading(key).max(widest(key, theme.body())),
        "version" => heading(key).max(widest(key, theme.body())),
        "load" => heading(key).max(widest(key, theme.small())),
        _ => heading(key),
    };
    let timed = orders.active_by.contains_key("load");
    let chosen: Vec<&'static str> = CHOSEN
        .into_iter()
        .filter(|k| *k != "load" || timed)
        .collect();
    let wide = width >= theme.sized(620.);
    let roomy = width >= theme.sized(440.);
    let view = btn_width(window, theme, &put(&say!("View")), false, true);
    let quick = view.max(theme.sized(if wide { 124. } else { 44. }));
    let number = theme.sized(28.);
    let room_beside = |shown: &[&str]| {
        let taken = shown.iter().fold(px(0.), |all, key| {
            all + column_width(key) + theme::GAP_LARGE
        });
        width
            - bar_room(theme)
            - quick
            - number
            - theme::GAP_WIDE * 2.
            - theme::GAP_LARGE * 2.
            - taken
    };
    let mut shown: Vec<&'static str> = if roomy { chosen } else { Vec::new() };
    for dropped in DROPPED_FIRST {
        if room_beside(&shown) >= theme.sized(180.) {
            break;
        }
        shown.retain(|key| *key != dropped);
    }
    let tile = theme.appearance.comfortable && width - bar_room(theme) >= theme.sized(420.);
    let fitting: Vec<(&'static str, Pixels)> = shown
        .into_iter()
        .map(|key| (key, column_width(key)))
        .collect();
    let beside = fitting
        .iter()
        .fold(px(0.), |all, (_, width)| all + *width + theme::GAP_LARGE);
    let tile_room = if tile {
        theme.sized(64.) + theme::GAP_SMALL
    } else {
        px(0.)
    };
    let name_room = width
        - bar_room(theme)
        - theme::GAP_WIDE * 2.
        - number
        - theme::GAP_LARGE * 2.
        - beside
        - quick
        - theme.sized(13.)
        - theme::GAP_SMALL * 2.
        - tile_room;
    Columns {
        number,
        name_room,
        fitting,
        quick,
        wide,
        tile,
    }
}

fn column_name(key: &str) -> rimstack_core::base::sentence::Sentence {
    match key {
        "author" => say!("Author"),
        "version" => say!("Version"),
        "load" => say!("Load"),
        "updated" => say!("Updated"),
        "downloaded" => say!("Downloaded"),
        "activated" => say!("Activated"),
        _ => say!("Size"),
    }
}
