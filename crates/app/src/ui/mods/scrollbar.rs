//! `ModList.qml` and `Scroller.qml`: a list of rows, virtual, with its bar always shown.

use super::rows::{self, Columns};
use super::{Mods, Shown};
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::with_alpha;
use crate::workspace::load_order::Side;
use gpui::{
    Context, IntoElement, ListState, ParentElement, Pixels, Styled, Window, canvas, div, fill,
    list, point, px, size,
};
use std::rc::Rc;

/// The room the bar takes at the list's right: its thumb, and the padding Qt gives it.
pub fn bar_room(theme: &Theme) -> Pixels {
    theme::GAP_HAIR + theme.sized(9.) + theme::GAP_LINE + px(2.) + theme::GAP_HAIR
}

/// The rows' height with the room above (the load order's) and the footer below.
pub fn spacer(theme: &Theme, side: Side) -> (Pixels, Pixels) {
    let footer = theme.sized(30.);
    match side {
        Side::Active => (theme::GAP_TINY, footer + theme::GAP_TINY),
        Side::Inactive => (px(0.), footer),
    }
}

impl Mods {
    pub(super) fn list_area(
        &self,
        theme: &Theme,
        side: Side,
        columns: Option<Rc<Columns>>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let shown: &Shown = match side {
            Side::Active => &self.active,
            Side::Inactive => &self.inactive,
        };
        let (above, below) = spacer(theme, side);
        let total = shown.rows.iter().fold(above + below, |all, row| {
            all + rows::height(theme, row, columns.is_some())
        });
        let room = bar_room(theme);
        let rows_shown = shown.rows.clone();
        let theme_shown = theme.clone();
        let mods = cx.weak_entity();
        let chosen: Vec<bool> = shown
            .rows
            .iter()
            .map(|row| self.is_chosen(side, row.mod_id))
            .collect();
        let last = shown.rows.len() + 1;
        let name_room = match &columns {
            Some(columns) => columns.name_room,
            None => {
                self.inactive_width
                    - room
                    - theme::GAP_WIDE * 2.
                    - theme.sized(13.)
                    - theme::GAP_SMALL * 2.
            }
        };
        let rows_list = list(shown.list.clone(), move |at, window: &mut Window, _cx| {
            let height = match at {
                0 => above,
                at if at == last => below,
                _ => px(0.),
            };
            let Some(row) = at.checked_sub(1).and_then(|at| rows_shown.get(at)) else {
                return div().h(height).into_any_element();
            };
            div()
                .pr(room)
                .child(rows::row(
                    window,
                    &theme_shown,
                    name_room,
                    side,
                    at - 1,
                    row,
                    columns.as_deref(),
                    chosen[at - 1],
                    mods.clone(),
                ))
                .into_any_element()
        })
        .size_full();
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(rows_list)
            .child(
                thumb(theme, shown.list.clone(), total)
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .w(room),
            )
    }
}

fn thumb(theme: &Theme, state: ListState, total: Pixels) -> gpui::Canvas<()> {
    let colour = with_alpha(theme.colours.ink3, 0.6);
    let (width, shortest) = (theme.sized(9.), theme.sized(32.));
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            let shown = bounds.size.height;
            if total <= shown {
                return;
            }
            let track = shown - theme::GAP_HAIR * 2.;
            let length = (track * (shown / total)).max(shortest);
            let scrolled = -state.scroll_px_offset_for_scrollbar().y;
            let part = (scrolled / (total - shown)).clamp(0., 1.);
            let top = theme::GAP_HAIR + (track - length) * part;
            let at = point(bounds.left() + theme::GAP_HAIR, bounds.top() + top);
            window.paint_quad(
                fill(gpui::Bounds::new(at, size(width, length)), colour).corner_radii(width / 2.),
            );
        },
    )
}
