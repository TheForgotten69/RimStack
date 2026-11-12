//! `ActivePane.qml` and `Columns.qml`: the load order, its filter and its column headings.

use super::Mods;
use super::rows::Columns;
use super::scrollbar::bar_room;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{Btn, Sized, btn_width, icon, label, measure};
use crate::words::{count, put};
use crate::workspace::load_order::Side;
use gpui::{
    Context, FontWeight, IntoElement, ParentElement, Pixels, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use rimstack_core::say;
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
        let heading = |key: &str| {
            let text = format!("{} ▾", put(&column_name(key)));
            measure(window, theme, &text, theme.caption(), theme.medium)
        };
        let widest = |key: &str, size: Pixels| {
            let text = self.orders.widest.get(key).cloned().unwrap_or_default();
            measure(window, theme, &text, size, FontWeight::NORMAL)
        };
        let column_width = |key: &str| match key {
            "author" => heading(key).max(widest(key, theme.body())),
            "version" => heading(key).max(widest(key, theme.body())),
            "load" => heading(key).max(widest(key, theme.small())),
            _ => heading(key),
        };
        let timed = self.orders.active_by.contains_key("load");
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
        let tile = width - bar_room(theme) >= theme.sized(420.);
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
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(theme::GAP)
                    .h(theme.header())
                    .pl(theme::GAP_CARD)
                    .pr(theme::GAP_LARGE)
                    .child(
                        label(theme, theme.heading(), c.ink, put(&say!("Active")))
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(label(
                        theme,
                        theme.body(),
                        c.ink3,
                        count(self.count(Side::Active)),
                    )),
            )
            .child(
                div()
                    .flex_none()
                    .px(theme::GAP_LARGE)
                    .pb(theme::GAP)
                    .child(self.filter_field(theme)),
            )
            .child(headings)
            .child(self.list_area(theme, Side::Active, Some(columns), width, cx))
    }

    /// `ModsFilter.qml` as it stands empty.
    fn filter_field(&self, theme: &Theme) -> gpui::Div {
        let c = &theme.colours;
        div()
            .flex()
            .items_center()
            .h(theme.field())
            .pl(theme::GAP)
            .pr(theme::GAP)
            .rounded(theme::CORNER)
            .bg(c.well)
            .border_1()
            .border_color(c.field_line)
            .child(icon("funnel", theme.icon() - px(1.), c.ink3))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .ml(theme::GAP_SMALL + theme::GAP_TINY + px(1.))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .sized_text(theme, theme.body())
                    .text_color(c.ink3)
                    .child(put(&say!("Filter active and inactive mods"))),
            )
            .child(label(
                theme,
                theme.caption(),
                c.ink3,
                put(&say!("Ctrl + F")),
            ))
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
