//! `ModList.qml` and `Scroller.qml`: one list of rows, a view of its own so that scrolling it draws
//! nothing else, with its bar always shown and its thumb held to scroll.

use super::active::columns;
use super::rows;
use super::rows::Columns;
use super::{Mods, thumbs};
use crate::ui::rows::{Scroll, Scrolling};
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::with_alpha;
use crate::workspace::load_order::{Orders, RowView, Side};
use gpui::{
    Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, Render, Styled, WeakEntity, Window, canvas, div, fill,
    point, px, size,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// What a list shows, handed over by the Mods screen.
#[derive(Clone)]
pub struct Shown {
    pub rows: Rc<Vec<RowView>>,
    pub orders: Rc<Orders>,
    /// The `mod_id`s chosen in this list.
    pub chosen: Rc<HashSet<i32>>,
}

/// The columns fitted to the width the list was last drawn at.
#[derive(Default)]
struct Made {
    width: Pixels,
    columns: Option<Rc<Columns>>,
}

pub struct ModList {
    side: Side,
    shown: Shown,
    /// Each row's top, then the whole height, the room above and the footer counted.
    tops: Rc<[Pixels]>,
    scroll: Scrolling,
    thumbs: thumbs::Shared,
    mods: WeakEntity<Mods>,
    made: Rc<RefCell<Made>>,
    pub(super) bench: Option<super::bench::Bench>,
}

/// The room the bar takes at the list's right: its thumb, and the padding Qt gives it.
pub fn bar_room(theme: &Theme) -> Pixels {
    theme::GAP_HAIR + theme.sized(9.) + theme::GAP_LINE + px(2.) + theme::GAP_HAIR
}

/// The room above the rows (the load order's) and the footer below them.
fn spacer(theme: &Theme, side: Side) -> (Pixels, Pixels) {
    let footer = theme.sized(30.);
    match side {
        Side::Active => (theme::GAP_TINY, footer + theme::GAP_TINY),
        Side::Inactive => (px(0.), footer),
    }
}

fn tops_of(theme: &Theme, side: Side, rows: &[RowView]) -> Rc<[Pixels]> {
    let (above, below) = spacer(theme, side);
    let mut tops = Vec::with_capacity(rows.len() + 1);
    let mut at = above;
    for row in rows {
        tops.push(at);
        at += rows::height(theme, row, side == Side::Active);
    }
    tops.push(at + below);
    tops.into()
}

impl ModList {
    pub fn new(
        side: Side,
        shown: Shown,
        thumbs: thumbs::Shared,
        mods: WeakEntity<Mods>,
        cx: &mut Context<Self>,
    ) -> Self {
        let theme = cx.global::<Theme>().clone();
        let mut scroll = Scroll::default();
        scroll.set_reduce_motion(theme.appearance.reduce_motion);
        Self {
            side,
            tops: tops_of(&theme, side, &shown.rows),
            shown,
            scroll: Rc::new(std::cell::RefCell::new(scroll)),
            thumbs,
            mods,
            made: Rc::default(),
            bench: None,
        }
    }

    /// New rows: the list stays where it was, as far down as it can.
    pub fn show(&mut self, shown: Shown, cx: &mut Context<Self>) {
        if !Rc::ptr_eq(&shown.rows, &self.shown.rows) {
            let theme = cx.global::<Theme>().clone();
            self.tops = tops_of(&theme, self.side, &shown.rows);
        }
        self.shown = shown;
        *self.made.borrow_mut() = Made::default();
        cx.notify();
    }

    pub fn scroll(&self) -> &Scrolling {
        &self.scroll
    }

    /// The row at the list's top.
    pub fn top_row(&self) -> usize {
        let top = self.scroll.borrow().top();
        self.tops
            .partition_point(|row| *row <= top)
            .saturating_sub(1)
    }

    fn bar(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let colour = with_alpha(theme.colours.ink3, 0.6);
        let (width, shortest) = (theme.sized(9.), theme.sized(32.));
        let thumb = move |scroll: &Scroll, track: Pixels| {
            let length = (track * (scroll.shown() / scroll.total().max(px(1.)))).max(shortest);
            let part = if scroll.most() > px(0.) {
                scroll.top() / scroll.most()
            } else {
                0.
            };
            (length, (track - length) * part)
        };
        let (drawn, view) = (self.scroll.clone(), cx.entity_id());
        canvas(
            |_, _, _| (),
            move |bounds, (), window, _| {
                let scroll = drawn.borrow();
                if scroll.total() <= scroll.shown() {
                    return;
                }
                let track = bounds.size.height - theme::GAP_HAIR * 2.;
                let (length, top) = thumb(&scroll, track);
                let at = point(
                    bounds.left() + theme::GAP_HAIR,
                    bounds.top() + theme::GAP_HAIR + top,
                );
                window.paint_quad(
                    fill(gpui::Bounds::new(at, size(width, length)), colour)
                        .corner_radii(width / 2.),
                );
                drop(scroll);
                let (held, moved) = (drawn.clone(), drawn.clone());
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if !phase.bubble()
                        || event.button != MouseButton::Left
                        || !bounds.contains(&event.position)
                    {
                        return;
                    }
                    let mut scroll = held.borrow_mut();
                    let (length, top) = thumb(&scroll, track);
                    let within = event.position.y - bounds.top() - theme::GAP_HAIR;
                    let grip = if within >= top && within <= top + length {
                        within - top
                    } else {
                        length / 2.
                    };
                    scroll.held = Some(grip);
                    let room = (track - length).max(px(1.));
                    let most = scroll.most();
                    scroll.set_top(most * ((within - grip) / room).clamp(0., 1.));
                    drop(scroll);
                    cx.notify(view);
                    cx.stop_propagation();
                    let _ = window;
                });
                window.on_mouse_event(move |event: &MouseMoveEvent, _, _, cx| {
                    let mut scroll = moved.borrow_mut();
                    let Some(grip) = scroll.held else {
                        return;
                    };
                    if !event.dragging() {
                        scroll.held = None;
                        return;
                    }
                    let (length, _) = thumb(&scroll, track);
                    let within = event.position.y - bounds.top() - theme::GAP_HAIR;
                    let room = (track - length).max(px(1.));
                    let most = scroll.most();
                    scroll.set_top(most * ((within - grip) / room).clamp(0., 1.));
                    cx.notify(view);
                });
                let released = drawn.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, _, _, _| {
                    released.borrow_mut().held = None
                });
            },
        )
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .w(bar_room(&theme.clone()))
    }
}

impl Render for ModList {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        self.bench_frame(window, cx);
        let (side, shown, thumbs, mods) = (
            self.side,
            self.shown.clone(),
            self.thumbs.clone(),
            self.mods.clone(),
        );
        let (made, list_id) = (self.made.clone(), cx.entity_id());
        let drawn_theme = theme.clone();
        let room = bar_room(&theme);
        let line = theme.sized(40.);
        let list = crate::ui::rows::rows(
            ("mod-list", side as usize),
            self.scroll.clone(),
            self.tops.clone(),
            line,
            cx.entity_id(),
            move |range, width, window, cx| {
                let theme = &drawn_theme;
                let mut made = made.borrow_mut();
                if made.width != width {
                    let columns = (side == Side::Active)
                        .then(|| Rc::new(columns(theme, &shown.orders, width, window)));
                    *made = Made { width, columns };
                }
                let name_room = match &made.columns {
                    Some(columns) => columns.name_room,
                    None => {
                        width
                            - room
                            - theme::GAP_WIDE * 2.
                            - theme.sized(13.)
                            - theme::GAP_SMALL * 2.
                    }
                };
                range
                    .map(|at| {
                        let row = &shown.rows[at];
                        let chosen = row.mod_id >= 0 && shown.chosen.contains(&row.mod_id);
                        let drawn = rows::row(
                            window,
                            theme,
                            name_room,
                            side,
                            at,
                            row,
                            made.columns.as_deref(),
                            chosen,
                            mods.clone(),
                            &thumbs,
                            list_id,
                            cx,
                        );
                        div().pr(room).child(drawn).into_any_element()
                    })
                    .collect()
            },
        );
        div()
            .id(("mod-list-pane", side as usize))
            .relative()
            .size_full()
            .child(list)
            .child(self.bar(&theme, cx))
    }
}
