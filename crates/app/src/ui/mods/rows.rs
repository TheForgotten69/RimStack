//! `ModRow.qml`: a category heading, a group under a tag, or a mod.

use super::Mods;
use crate::ui::theme::{self, CATEGORY_ICONS, Theme};
use crate::ui::widgets::{
    Chip, TextSized, chip_width, elide_middle, halo, icon, label, with_alpha,
};
use crate::words::{count, put};
use crate::workspace::load_order::{RowView, Side};
use gpui::{
    AnyElement, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels, SharedString,
    StatefulInteractiveElement, Styled, StyledImage, TextOverflow, WeakEntity, Window, div, img,
    prelude::FluentBuilder, px,
};
use rimstack_core::say;

/// What the load order shows beside a mod's name, as `ModList.qml` fits it to its width.
pub struct Columns {
    pub number: Pixels,
    /// The room a mod's name has before it is cut in the middle.
    pub name_room: Pixels,
    /// A column's key and width, in order.
    pub fitting: Vec<(&'static str, Pixels)>,
    pub quick: Pixels,
    pub wide: bool,
    pub tile: bool,
}

/// How tall a row is drawn; `columns` for the load order.
pub fn height(theme: &Theme, row: &RowView, columns: bool) -> Pixels {
    if row.is_sub {
        theme.sized(24.)
    } else if row.again {
        theme.sized(22.)
    } else if row.is_header {
        theme.sized(32.)
    } else if columns {
        theme.sized(52.).max(theme.row() + theme::GAP_MEDIUM)
    } else {
        theme.row()
    }
}

fn category_colour(theme: &Theme, category: i32) -> Hsla {
    usize::try_from(category).map_or(theme.colours.ink3, |at| theme.category_colour(at))
}

fn category_icon(category: i32) -> &'static str {
    usize::try_from(category)
        .ok()
        .and_then(|at| CATEGORY_ICONS.get(at))
        .copied()
        .unwrap_or("package")
}

#[allow(clippy::too_many_arguments)]
pub fn row(
    window: &Window,
    theme: &Theme,
    name_room: Pixels,
    side: Side,
    at: usize,
    row: &RowView,
    columns: Option<&Columns>,
    chosen: bool,
    mods: WeakEntity<Mods>,
) -> AnyElement {
    let c = &theme.colours;
    let colour = category_colour(theme, row.category);
    let big_header = row.is_header && !row.is_sub && !row.again;
    let body = if row.is_sub {
        sub(theme, row)
    } else if row.again {
        again(theme, row)
    } else if row.is_header {
        heading(theme, row, colour)
    } else {
        let mut room = name_room;
        let mut beside = |text: &str, most: Option<Pixels>| {
            let wide = chip_width(window, theme, text, false);
            room -= most.map_or(wide, |most| wide.min(most)) + theme::GAP_SMALL;
        };
        if row.outdated {
            beside(&row.made_for, None);
        }
        if row.fresh {
            beside(&put(&say!("New")), None);
        }
        for tag in row.tags.split(", ").filter(|t| !t.is_empty()).take(2) {
            beside(tag, Some(theme.sized(110.)));
        }
        if row.version_ignored {
            room -= theme.sized(12.) + theme::GAP_SMALL;
        }
        let name = elide_middle(window, theme, &put(&row.name), theme.body(), room);
        cells(theme, row, name, columns, chosen, colour)
    };
    let (is_header, is_sub, category, mod_id) =
        (row.is_header, row.is_sub, row.category, row.mod_id);
    let ground = if chosen {
        Some(c.chosen)
    } else if row.moved_from > 0 {
        Some(with_alpha(c.moved, 0.1))
    } else {
        None
    };
    div()
        .id((
            match side {
                Side::Active => "active-row",
                Side::Inactive => "inactive-row",
            },
            at,
        ))
        .group("mod-row")
        .relative()
        .w_full()
        .h(height(theme, row, columns.is_some()))
        .child(
            div()
                .absolute()
                .left(px(2.))
                .top(if big_header {
                    theme::GAP_MEDIUM
                } else {
                    px(0.)
                })
                .bottom_0()
                .w(px(3.))
                .when(big_header, |d| d.rounded(px(1.5)))
                .bg(if row.again {
                    with_alpha(colour, 0.55)
                } else {
                    colour
                }),
        )
        .when(row.outdated && !chosen, |d| {
            d.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(theme::GAP)
                    .right(theme::GAP_SMALL)
                    .bg(with_alpha(c.warn, 0.06))
                    .child(div().w(px(2.)).h_full().bg(c.warn)),
            )
        })
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(theme::GAP)
                .right(theme::GAP_SMALL)
                .rounded(theme::CORNER)
                .when_some(ground, |d, ground| d.bg(ground))
                .when(ground.is_none() && !is_header, |d| {
                    let hover = with_alpha(c.hover, 0.04);
                    d.group_hover("mod-row", move |s| s.bg(hover))
                }),
        )
        .when(chosen, |d| {
            d.child(
                div()
                    .absolute()
                    .left(theme::GAP)
                    .top(theme::GAP_TINY)
                    .bottom(theme::GAP_TINY)
                    .w(px(3.))
                    .rounded(px(1.5))
                    .bg(c.focus)
                    .children(halo(c.focus, px(1.5))),
            )
        })
        .child(body)
        .on_click(move |_, _, cx| {
            let _ = mods.update(cx, |mods, cx| match (is_header, is_sub) {
                (true, true) => mods.toggle_sub(side, at, cx),
                (true, false) => mods.toggle_category(side, category, cx),
                (false, _) => mods.choose(side, mod_id, cx),
            });
        })
        .into_any_element()
}

fn arrow(theme: &Theme, collapsed: bool, colour: Hsla) -> gpui::Div {
    label(
        theme,
        theme.caption(),
        colour,
        if collapsed { "▸" } else { "▾" },
    )
    .w(theme.sized(10.))
}

fn heading(theme: &Theme, row: &RowView, colour: Hsla) -> AnyElement {
    let c = &theme.colours;
    div()
        .absolute()
        .left(theme::GAP_WIDE)
        .bottom(theme::GAP_TINY)
        .flex()
        .items_center()
        .gap(theme::GAP_SMALL)
        .child(arrow(theme, row.collapsed, c.ink2))
        .child(icon(category_icon(row.category), theme.icon(), colour))
        .child(label(theme, theme.small(), c.ink, put(&row.name)).font_weight(theme.semibold))
        .child(label(
            theme,
            theme.small(),
            c.ink3,
            count(row.count.max(0) as usize),
        ))
        .into_any_element()
}

fn sub(theme: &Theme, row: &RowView) -> AnyElement {
    let c = &theme.colours;
    let counted = match row.group_size > row.count {
        true => put(&say!(
            "{} of {}",
            count(row.count as usize),
            count(row.group_size as usize)
        )),
        false => count(row.count.max(0) as usize),
    };
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .pl(theme::GAP_WIDE + theme.sized(28.))
        .gap(theme::GAP_SMALL)
        .child(arrow(theme, row.collapsed, c.ink3))
        .child(label(theme, theme.small(), c.ink2, put(&row.name)))
        .child(label(theme, theme.small(), c.ink3, counted))
        .when(row.again, |d| {
            d.child(label(
                theme,
                theme.small(),
                c.ink3,
                put(&say!("the rest of it")),
            ))
        })
        .into_any_element()
}

fn again(theme: &Theme, row: &RowView) -> AnyElement {
    let c = &theme.colours;
    let said = format!("{}  {}", put(&row.name), count(row.count.max(0) as usize));
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .px(theme::GAP_WIDE)
        .gap(theme::GAP_SMALL)
        .child(arrow(theme, row.collapsed, c.ink3))
        .child(label(theme, theme.caption(), c.ink3, said))
        .child(label(
            theme,
            theme.caption(),
            c.warn,
            put(&say!("out of category order")),
        ))
        .child(div().flex_1().ml(theme::GAP_SMALL).h(px(1.)).bg(c.line))
        .into_any_element()
}

fn tile(theme: &Theme, row: &RowView, colour: Hsla) -> AnyElement {
    let first: String = put(&row.name)
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default();
    div()
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .w(theme.sized(64.))
        .h(theme.sized(40.))
        .rounded(theme::CORNER)
        .bg(theme.colours.ctl)
        .map(|d| match row.picture.is_empty() {
            true => d
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded(theme::CORNER)
                        .bg(with_alpha(colour, 0.16))
                        .border_1()
                        .border_color(with_alpha(colour, 0.28)),
                )
                .child(label(theme, theme.title(), colour, first).font_weight(theme.semibold)),
            false => d.child(
                img(std::path::PathBuf::from(&row.picture))
                    .absolute()
                    .left(theme::GAP_LINE)
                    .top(theme::GAP_LINE)
                    .w(theme.sized(64.) - theme::GAP_LINE * 2.)
                    .h(theme.sized(40.) - theme::GAP_LINE * 2.)
                    .rounded(theme::CORNER)
                    .object_fit(gpui::ObjectFit::Cover),
            ),
        })
        .into_any_element()
}

fn cells(
    theme: &Theme,
    row: &RowView,
    name: String,
    columns: Option<&Columns>,
    chosen: bool,
    colour: Hsla,
) -> AnyElement {
    let c = &theme.colours;
    let quiet = if chosen { c.on_selected_dim } else { c.ink2 };
    let ink = if chosen { c.on_selected } else { c.ink };
    let tags: Vec<String> = match row.tags.is_empty() {
        true => Vec::new(),
        false => row.tags.split(", ").take(2).map(String::from).collect(),
    };
    let name = div()
        .flex()
        .flex_1()
        .min_w_0()
        .items_center()
        .gap(theme::GAP_SMALL)
        .child(icon(
            category_icon(row.category),
            theme.sized(13.),
            if chosen { c.on_selected } else { colour },
        ))
        .when(columns.is_some_and(|c| c.tile), |d| {
            d.child(tile(theme, row, colour))
        })
        .when(row.version_ignored, |d| {
            d.child(icon(
                "eye-off",
                theme.sized(12.),
                if chosen { c.on_selected } else { c.ink3 },
            ))
        })
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_overflow(TextOverflow::TruncateMiddle("…".into()))
                .sized_text(theme, theme.body())
                .text_color(ink)
                .child(name),
        )
        .when(row.outdated, |d| {
            d.child(
                Chip::new(row.made_for.clone())
                    .tone(c.warn)
                    .on_accent(chosen),
            )
        })
        .when(row.fresh, |d| {
            d.child(Chip::new(put(&say!("New"))).tone(c.ok).on_accent(chosen))
        })
        .children(tags.into_iter().map(|tag| {
            Chip::new(tag)
                .max_width(theme.sized(110.))
                .on_accent(chosen)
        }))
        .child(div().flex_1())
        .when(!row.ordered_by.is_empty() && columns.is_none(), |d| {
            d.child(label(theme, theme.small(), quiet, put(&row.ordered_by)))
        })
        .when(columns.is_none() && row.source == 2, |d| {
            d.child(icon(
                "git-branch",
                theme.sized(12.),
                if chosen { c.on_selected } else { c.ink3 },
            ))
        });
    let Some(columns) = columns else {
        return div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .px(theme::GAP_WIDE)
            .gap(theme::GAP_LARGE)
            .child(name)
            .into_any_element();
    };
    let value = |key: &str| -> SharedString {
        match key {
            "author" => row.author.clone().into(),
            "version" => row.version.clone().into(),
            "load" => row.load_time.clone().into(),
            "updated" => put(&row.updated_ago).into(),
            "downloaded" => put(&row.downloaded_ago).into(),
            "activated" => put(&row.activated_ago).into(),
            _ => put(&row.size_text).into(),
        }
    };
    let quick = quick_chip(theme, row, columns.wide, chosen);
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .px(theme::GAP_WIDE)
        .gap(theme::GAP_LARGE)
        .child(
            div()
                .flex()
                .flex_none()
                .justify_end()
                .w(columns.number)
                .child(label(
                    theme,
                    theme.small(),
                    if chosen { c.on_selected_dim } else { c.ink3 },
                    row.number.to_string(),
                )),
        )
        .child(name)
        .children(columns.fitting.iter().map(|(key, width)| {
            let right = matches!(*key, "load" | "size");
            let colour = match (*key, chosen) {
                ("load", true) => c.on_selected,
                ("load", false) if row.load_seconds >= 0.1 => c.ink2,
                ("load", false) => c.ink3,
                _ => quiet,
            };
            div()
                .flex()
                .flex_none()
                .w(*width)
                .when(right, |d| d.justify_end())
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .sized_text(theme, if right { theme.small() } else { theme.body() })
                        .text_color(colour)
                        .child(value(key)),
                )
        }))
        .child(
            div()
                .flex()
                .flex_none()
                .justify_end()
                .w(columns.quick)
                .children(quick),
        )
        .into_any_element()
}

/// The chip at a row's end: how far Sort moves it, or its problem.
fn quick_chip(theme: &Theme, row: &RowView, wide: bool, chosen: bool) -> Option<Chip> {
    let c = &theme.colours;
    let chip = if row.moved_from > 0 {
        let up = row.moved_from > row.number;
        Chip::new(count(row.moved_from.abs_diff(row.number) as usize))
            .tone(c.moved)
            .icon(if up { "arrow-up" } else { "arrow-down" })
    } else if row.trouble > 0 {
        let error = row.trouble == 2;
        Chip::new(if wide {
            put(&row.trouble_text)
        } else {
            String::new()
        })
        .tone(if error { c.err } else { c.warn })
        .icon(if error {
            "circle-alert"
        } else {
            "triangle-alert"
        })
    } else {
        return None;
    };
    Some(chip.on_accent(chosen))
}
