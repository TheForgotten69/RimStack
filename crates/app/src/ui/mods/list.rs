//! `ModList.qml` and `Scroller.qml`: one list of rows, a view of its own so that scrolling it draws
//! nothing else, with its bar always shown and its thumb held to scroll.

use super::Mods;
use super::columns::columns;
use super::rows;
use super::rows::Columns;
use crate::ui::rows::{Scroll, Scrolling};
use crate::ui::theme::{self, Theme};
use crate::ui::thumbs;
use crate::workspace::load_order::{Orders, RowView, Side};
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Pixels, Render,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, prelude::FluentBuilder, px,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// What a list shows, handed over by the Mods screen.
#[derive(Clone)]
pub struct Shown {
    pub rows: Rc<Vec<RowView>>,
    pub orders: Rc<Orders>,
    /// The columns chosen and their widths.
    pub view: Rc<crate::workspace::ModsView>,
    pub marks: Rc<rows::Marks>,
    /// The `mod_id`s chosen in this list.
    pub chosen: Rc<HashSet<i32>>,
    /// Whether its mods may be dragged: not while Sort is shown, nor in an order of a column.
    pub movable: bool,
}

/// The columns fitted to the width the list was last drawn at.
#[derive(Default)]
struct Made {
    width: Pixels,
    view: Rc<crate::workspace::ModsView>,
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
    focus: gpui::FocusHandle,
}

pub use crate::ui::rows::bar_room;

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
        cx.observe_global::<Theme>(|list, cx| {
            let theme = cx.global::<Theme>().clone();
            list.tops = tops_of(&theme, list.side, &list.shown.rows);
            list.scroll
                .borrow_mut()
                .set_reduce_motion(theme.appearance.reduce_motion);
            *list.made.borrow_mut() = Made::default();
            cx.notify();
        })
        .detach();
        Self {
            side,
            tops: tops_of(&theme, side, &shown.rows),
            shown,
            scroll: Rc::new(std::cell::RefCell::new(scroll)),
            thumbs,
            mods,
            made: Rc::default(),
            bench: None,
            focus: cx.focus_handle(),
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

    /// Scrolls so that row `at` is in the middle of the list, as `positionViewAtIndex` centred it.
    pub fn reveal(&mut self, at: usize, cx: &mut Context<Self>) {
        let Some(&top) = self.tops.get(at) else {
            return;
        };
        let mut scroll = self.scroll.borrow_mut();
        let shown = scroll.shown();
        scroll.set_top(top - shown / 2.);
        drop(scroll);
        cx.notify();
    }

    /// Scrolls just enough for row `at` to show whole, as `ListView.Contain` did.
    pub fn contain(&mut self, at: usize, cx: &mut Context<Self>) {
        if let (Some(&top), Some(&bottom)) = (self.tops.get(at), self.tops.get(at + 1)) {
            self.scroll.borrow_mut().contain(top, bottom);
            cx.notify();
        }
    }

    /// The row at the list's top.
    pub fn top_row(&self) -> usize {
        let top = self.scroll.borrow().top();
        self.tops
            .partition_point(|row| *row <= top)
            .saturating_sub(1)
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
                if made.width != width || !Rc::ptr_eq(&made.view, &shown.view) {
                    let columns = (side == Side::Active).then(|| {
                        Rc::new(columns(theme, &shown.orders, &shown.view, width, window))
                    });
                    *made = Made {
                        width,
                        view: shown.view.clone(),
                        columns,
                    };
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
                let chosen_ids: Vec<i32> = shown.chosen.iter().copied().collect();
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
                            rows::Moving {
                                movable: shown.movable,
                                chosen_ids: &chosen_ids,
                                marks: &shown.marks,
                            },
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
        let focused = self.focus.is_focused(window);
        let (keyed, focus) = (self.mods.clone(), self.focus.clone());
        div()
            .id(("mod-list-pane", side as usize))
            .track_focus(&self.focus)
            .role(gpui::Role::List)
            .relative()
            .size_full()
            .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                window.focus(&focus, cx)
            })
            .on_key_down(move |event: &gpui::KeyDownEvent, window, cx| {
                let _ = keyed.update(cx, |mods, cx| mods.list_key(side, event, window, cx));
            })
            .child(list)
            .when(focused, |d| {
                d.child(
                    div()
                        .absolute()
                        .inset(theme::GAP_LINE)
                        .rounded(theme::CORNER)
                        .border_2()
                        .border_color(theme.colours.focus),
                )
            })
            .child(crate::ui::rows::bar(
                &theme,
                self.scroll.clone(),
                cx.entity_id(),
            ))
    }
}
