//! `ModRow.qml`: a category heading, a group under a tag, or a mod.

mod cells;
mod headings;

use super::Mods;
use crate::ui::theme::{self, CATEGORY_ICONS, Theme};
use crate::ui::thumbs;
use crate::ui::widgets::{TextSized, halo, icon, with_alpha};
use crate::words::put;
use crate::workspace::load_order::{RowView, Side};
use gpui::AppContext;
use gpui::{
    AnyElement, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, prelude::FluentBuilder, px,
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
    } else if columns && theme.appearance.comfortable {
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

/// A mod held in a drag: its list, its place there, and the mods it takes along.
#[derive(Clone)]
pub struct Held {
    pub side: Side,
    pub index: usize,
    pub ids: Vec<rimstack_core::mods::library::ModId>,
}

/// What follows the pointer while a divider or a heading is dragged: nothing; the drag shows
/// in what it moves.
pub struct Ghost;

impl gpui::Render for Ghost {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
    }
}

/// What follows the pointer while mods are dragged, as `ModsScreen.qml`'s ghost: what is held,
/// beside the pointer. `grabbed` is where the row was taken, from the row's corner, which GPUI
/// draws this at.
pub struct HeldGhost {
    pub name: String,
    pub grabbed: gpui::Point<Pixels>,
}

impl gpui::Render for HeldGhost {
    fn render(&mut self, _: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let tall = theme.sized(26.);
        div()
            .ml(self.grabbed.x + theme::GAP_MEDIUM)
            .mt(self.grabbed.y - tall / 2.)
            .w(theme.sized(300.))
            .h(tall)
            .px(theme::GAP_MEDIUM)
            .flex()
            .items_center()
            .rounded(theme::CORNER)
            .bg(theme.colours.accent)
            .opacity(0.9)
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .sized_text(&theme, theme.body())
                    .text_color(gpui::white())
                    .child(self.name.clone()),
            )
    }
}

/// What a row takes from its list and the screen beyond itself.
pub struct Around<'a> {
    /// Whether its mods may be dragged.
    pub movable: bool,
    pub chosen_ids: &'a [i32],
    /// The mods sharing textures with the one chosen: whether each loses them to it.
    pub marks: &'a Marks,
    /// Whether one mod alone is chosen, in either list.
    pub single: bool,
    /// Whether a row's own buttons show: not while Sort is shown nor while a mod is picked.
    pub buttons: bool,
    pub previewing: bool,
    /// Whether the load order's tags head groups, so a row tells only how many tags it has.
    pub headed: bool,
    /// Whether the column the load order is ordered by is shown, which then says the value.
    pub order_shown: bool,
}

pub type Marks = std::collections::BTreeMap<
    rimstack_core::mods::library::ModId,
    rimstack_core::mods::details::Mark,
>;

/// The buttons a mod's row shows while pointed at or chosen: across, its folder, its menu.
fn hover_buttons(
    theme: &Theme,
    side: Side,
    at: usize,
    row: &RowView,
    alone: bool,
    mods: &WeakEntity<Mods>,
) -> AnyElement {
    let c = &theme.colours;
    let size = theme.sized(24.);
    let (mod_id, index) = (row.mod_id, row.mod_index);
    let button = |key: &'static str, name: &'static str, tip: String| {
        let shown = div()
            .id((key, at))
            .flex()
            .items_center()
            .justify_center()
            .size(size)
            .rounded(theme::CORNER)
            .bg(c.ctl)
            .hover(|d| d.bg(c.ctl_hi))
            .cursor_pointer()
            .role(gpui::Role::Button)
            .aria_label(tip.clone())
            .child(icon(name, theme.sized(13.), c.ink));
        crate::ui::tip::tipped(shown, tip.into())
    };
    let (across, folder, menu) = (mods.clone(), mods.clone(), mods.clone());
    let active = side == Side::Active;
    div()
        .absolute()
        .right(theme::GAP_CARD)
        .top_0()
        .bottom_0()
        .flex()
        .items_center()
        .gap(theme::GAP_HAIR)
        .child(
            button(
                "row-across",
                if active { "minus" } else { "plus" },
                match active {
                    true => put(&say!("Deactivate")),
                    false => put(&say!("Activate")),
                },
            )
            .invisible()
            .group_hover("mod-row", |s| s.visible())
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                let _ = across.update(cx, |mods, cx| mods.toggle_mod(side, mod_id, cx));
            }),
        )
        .when(mod_id >= 0, |d| {
            d.child(
                button("row-folder", "folder-open", put(&say!("Open folder")))
                    .invisible()
                    .group_hover("mod-row", |s| s.visible())
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        let _ = folder.update(cx, |mods, cx| mods.open_folder(mod_id, cx));
                    }),
            )
        })
        .child(
            button("row-menu", "ellipsis", put(&say!("Menu (right-click)")))
                .when(!alone, |d| {
                    d.invisible().group_hover("mod-row", |s| s.visible())
                })
                .on_click(move |event, window, cx| {
                    cx.stop_propagation();
                    let at = event.position();
                    let _ = menu.update(cx, |mods, cx| {
                        mods.row_menu(side, index, mod_id, at, window, cx)
                    });
                }),
        )
        .into_any_element()
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
    around: &Around<'_>,
    mods: WeakEntity<Mods>,
    thumbs: &thumbs::Shared,
    shown_by: gpui::EntityId,
    cx: &mut gpui::App,
) -> AnyElement {
    let c = &theme.colours;
    let colour = category_colour(theme, row.category);
    let big_header = row.is_header && !row.is_sub && !row.again;
    let mark = rimstack_core::mods::library::ModId::from_qml(row.mod_id)
        .and_then(|id| around.marks.get(&id))
        .filter(|_| columns.is_some() && !row.is_header);
    let body = if row.is_sub {
        headings::sub(theme, row, side, at, mods.clone())
    } else if row.again {
        headings::again(theme, row)
    } else if row.is_header {
        headings::heading(theme, row, colour)
    } else {
        let fitted = cells::fit(window, theme, row, name_room, columns, around);
        let picture = match columns.is_some_and(|c| c.tile) && !row.picture.is_empty() {
            true => {
                let scale = window.scale_factor();
                let device =
                    |logical: f32| f32::from(theme.sized(logical) - theme::GAP_LINE * 2.) * scale;
                let size = (device(64.).round() as u32, device(40.).round() as u32);
                thumbs::thumb(thumbs, &row.picture, size, shown_by, cx)
            }
            false => None,
        };
        let shown = cells::Shown {
            columns,
            chosen,
            alone: chosen && around.single,
            colour,
            picture,
            mark,
        };
        cells::cells(theme, row, fitted, shown)
    };
    let (is_header, is_sub, category, mod_id) =
        (row.is_header, row.is_sub, row.category, row.mod_id);
    let (mods_for_menu, at_row, mod_index) = (mods.clone(), at, row.mod_index);
    let previewing = around.previewing;
    let ground = if chosen {
        Some(c.chosen)
    } else if mark.is_some() {
        Some(with_alpha(c.warn, 0.12))
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
        .role(match is_header {
            true => gpui::Role::Heading,
            false => gpui::Role::ListItem,
        })
        .aria_label(put(&row.accessible_name))
        .when(!is_header, |d| d.aria_selected(chosen))
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
        .when(
            !is_header && row.mod_index >= 0 && around.buttons && !cx.has_active_drag(),
            |d| {
                d.child(hover_buttons(
                    theme,
                    side,
                    at,
                    row,
                    chosen && around.single,
                    &mods,
                ))
            },
        )
        .when(!is_header, |d| {
            let landing = mods.clone();
            let index = row.mod_index.max(0) as usize;
            d.child(
                div()
                    .absolute()
                    .left(theme::GAP_SMALL)
                    .right(theme::GAP_SMALL)
                    .top(px(-1.5))
                    .h(px(3.))
                    .rounded(px(1.5))
                    .bg(c.focus)
                    .child(
                        div()
                            .absolute()
                            .left(px(-3.))
                            .top(px(-3.))
                            .size(px(9.))
                            .rounded_full()
                            .bg(c.focus),
                    )
                    .invisible()
                    .group_drag_over::<Held>("mod-row", |s| s.visible()),
            )
            .on_drop(move |held: &Held, _, cx| {
                let held = held.clone();
                let _ = landing.update(cx, |mods, cx| {
                    mods.drop_on((held.side, held.index, held.ids), (side, index), cx)
                });
            })
        })
        .when(around.movable && !is_header && row.mod_index >= 0, |d| {
            let ids = match chosen && around.chosen_ids.len() > 1 {
                true => around.chosen_ids.to_vec(),
                false => vec![mod_id],
            };
            let name = match ids.len() > 1 {
                true => put(&rimstack_core::say_n!(ids.len(), "{} mod", "{} mods")),
                false => put(&row.name),
            };
            let held = Held {
                side,
                index: row.mod_index as usize,
                ids: ids
                    .into_iter()
                    .filter_map(rimstack_core::mods::library::ModId::from_qml)
                    .collect(),
            };
            d.on_drag(held, move |_, grabbed, _, cx| {
                cx.new(|_| HeldGhost {
                    name: name.clone(),
                    grabbed,
                })
            })
        })
        .on_click(move |event, _, cx| {
            let (modifiers, command) = (event.modifiers(), event.modifiers().secondary(cx));
            let twice = event.click_count() >= 2 && !previewing;
            let active = side == Side::Active;
            let _ = mods.update(cx, |mods, cx| match (is_header, is_sub) {
                (true, true) => mods.toggle_sub(side, at, cx),
                (true, false) => mods.toggle_category(side, category, cx),
                (false, _) if twice => mods.toggle_mod(side, mod_id, cx),
                (false, _) if command => mods.select_also(mod_id, active, cx),
                (false, _) if modifiers.shift => mods.select_through(mod_id, active, cx),
                (false, _) => mods.choose(side, mod_id, cx),
            });
        })
        .on_mouse_down(gpui::MouseButton::Right, {
            let menu = mods_for_menu;
            move |event, window, cx| {
                let at = event.position;
                let _ = menu.update(cx, |mods, cx| match (is_header, is_sub) {
                    (true, true) => mods.sub_menu(side, at_row, at, window, cx),
                    (true, false) => mods.category_menu(category, at, window, cx),
                    (false, _) => mods.row_menu(side, mod_index, mod_id, at, window, cx),
                });
            }
        })
        .into_any_element()
}
