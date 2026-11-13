//! One list of rows, a view of its own so that scrolling it draws
//! nothing else, with its bar always shown and its thumb held to scroll.

use super::Mods;
use super::columns::columns;
use super::rows;
use super::rows::Columns;
use crate::ui::rows::{Scroll, Scrolling};
use crate::ui::theme::{self, Theme};
use crate::ui::thumbs;
use crate::workspace::load_order::{Orders, RowView, Side};
use gpui::AnimationExt;
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
    /// Whether the filter leaves it out and it is dimmed rather than hidden.
    pub dimmed: bool,
    /// Whether one mod alone is chosen, in either list.
    pub single: bool,
    pub previewing: bool,
    /// Whether a mod is being picked for a rule.
    pub picking: bool,
    /// Its mods, headings and filter aside: where a drop past its last row lands.
    pub count: usize,
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
    /// Where the pointer is while mods are dragged over the list, and the list's bounds then.
    dragged_at: Option<(gpui::Point<Pixels>, gpui::Bounds<Pixels>)>,
    pulling: Option<gpui::Task<()>>,
    /// How far down the window the pointer is while over the list, and where the list's top was
    /// drawn: which row it is on is read from them as the list scrolls under it.
    pointer: Option<Pixels>,
    drawn_top: Rc<std::cell::Cell<Pixels>>,
    /// The mods sliding to the places they were moved to, and how many rows each comes from;
    /// a new slide is a new turn, which starts each row's animation again.
    slides: Rc<std::collections::HashMap<i32, i32>>,
    slide_turn: u64,
    slide_end: Option<gpui::Task<()>>,
}

/// The row's slide: `Theme.quick`, eased out; the rows slide for 400 ms
/// before a row drawn anew stops starting one.
const SLIDE: std::time::Duration = std::time::Duration::from_millis(140);
const SLIDES_FOR: std::time::Duration = std::time::Duration::from_millis(400);

/// How often and how far a drag held at the list's edge scrolls it.
const PULL_EVERY: std::time::Duration = std::time::Duration::from_millis(16);

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
            focus: cx.focus_handle().tab_stop(true),
            dragged_at: None,
            pulling: None,
            pointer: None,
            drawn_top: Rc::default(),
            slides: Rc::default(),
            slide_turn: 0,
            slide_end: None,
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
        if let (Some(&top), Some(&bottom)) = (self.tops.get(at), self.tops.get(at + 1)) {
            self.scroll.borrow_mut().centre(top, bottom);
            cx.notify();
        }
    }

    /// Scrolls just enough for row `at` to show whole, as `ListView.Contain` did, and clear of
    /// the category heading kept at the top.
    pub fn contain(&mut self, at: usize, cx: &mut Context<Self>) {
        if let (Some(&top), Some(&bottom)) = (self.tops.get(at), self.tops.get(at + 1)) {
            let headed = self.shown.rows.first().is_some_and(|r| r.is_header);
            let clear = match headed {
                true => cx.global::<Theme>().control(),
                false => px(0.),
            };
            self.scroll.borrow_mut().contain(top - clear, bottom);
            cx.notify();
        }
    }

    pub fn focus_handle(&self) -> gpui::FocusHandle {
        self.focus.clone()
    }

    /// The category whose mods are at the list's top, while its heading has scrolled away.
    fn top_category(&self) -> Option<(i32, Pixels)> {
        let rows = &self.shown.rows;
        if !rows.first().is_some_and(|r| r.is_header) {
            return None;
        }
        let top = self.scroll.borrow().top();
        let first = self.top_row();
        let row = rows.get(first).filter(|r| r.is_sub || !r.is_header)?;
        Some((row.category, top))
    }

    /// Each mod of `slides` comes to its row from that many rows away, unless motion is reduced.
    pub fn slide(&mut self, slides: std::collections::HashMap<i32, i32>, cx: &mut Context<Self>) {
        if cx.global::<Theme>().appearance.reduce_motion || slides.is_empty() {
            return;
        }
        self.slides = Rc::new(slides);
        self.slide_turn += 1;
        let turn = self.slide_turn;
        self.slide_end = Some(cx.spawn(async move |list, cx| {
            cx.background_executor().timer(SLIDES_FOR).await;
            let _ = list.update(cx, |list, _| {
                if list.slide_turn == turn {
                    list.slides = Rc::default();
                }
            });
        }));
        cx.notify();
    }

    /// The row the pointer is on, which alone has its buttons.
    pub fn pointed(&self) -> Option<usize> {
        self.pointed_row(self.pointer)
    }

    /// Where row `at` is drawn in the window, from its top: as a test points at it.
    #[cfg(test)]
    pub fn drawn_at(&self, at: usize) -> Pixels {
        self.drawn_top.get() + self.tops[at] - self.scroll.borrow().top()
    }

    fn pointed_row(&self, pointer: Option<Pixels>) -> Option<usize> {
        let at = pointer? - self.drawn_top.get() + self.scroll.borrow().top();
        let (first, end) = (*self.tops.first()?, *self.tops.last()?);
        (at >= first && at < end).then(|| {
            self.tops
                .partition_point(|row| *row <= at)
                .saturating_sub(1)
                .min(self.shown.rows.len().saturating_sub(1))
        })
    }

    /// The pointer moved over the list, or off it: drawn again when it is on another row.
    fn point(&mut self, pointer: Option<Pixels>, cx: &mut Context<Self>) {
        let (was, now) = (self.pointed_row(self.pointer), self.pointed_row(pointer));
        self.pointer = pointer;
        if was != now {
            cx.notify();
        }
    }

    /// A drag held near the list's top or bottom scrolls it, faster the nearer the edge.
    fn dragged_over(
        &mut self,
        at: gpui::Point<Pixels>,
        bounds: gpui::Bounds<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.dragged_at = Some((at, bounds));
        if self.pulling.is_some() {
            return;
        }
        self.pulling = Some(cx.spawn(async move |list, cx| {
            loop {
                cx.background_executor().timer(PULL_EVERY).await;
                let going = list.update(cx, |list, cx| list.pull(cx)).unwrap_or(false);
                if !going {
                    break;
                }
            }
        }));
    }

    fn pull(&mut self, cx: &mut Context<Self>) -> bool {
        let Some((at, bounds)) = self.dragged_at.filter(|_| cx.has_active_drag()) else {
            self.dragged_at = None;
            self.pulling = None;
            return false;
        };
        let theme = cx.global::<Theme>();
        let (edge, step) = (theme.sized(40.), theme.sized(22.));
        let (y, height) = (at.y - bounds.top(), bounds.size.height);
        let outside = at.x < bounds.left() || at.x > bounds.right();
        if outside || y < -edge || y > height + edge {
            return true;
        }
        let pull = if y < edge {
            -((edge - y.max(px(0.))) / edge)
        } else if y > height - edge {
            (y.min(height) - (height - edge)) / edge
        } else {
            0.
        };
        if pull != 0. {
            let mut scroll = self.scroll.borrow_mut();
            let top = scroll.top();
            scroll.set_top(top + step * pull);
            drop(scroll);
            cx.notify();
        }
        true
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
        let pointed = self.pointed_row(self.pointer);
        let (slides, slide_turn) = (self.slides.clone(), self.slide_turn);
        let drawn_top = self.drawn_top.clone();
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
                let around = rows::Around {
                    pointed,
                    movable: shown.movable,
                    chosen_ids: &chosen_ids,
                    marks: &shown.marks,
                    single: shown.single,
                    buttons: !shown.previewing && !shown.picking,
                    previewing: shown.previewing,
                    headed: shown.orders.active_key.is_empty() && shown.view.headings,
                    order_shown: made.columns.as_ref().is_some_and(|columns| {
                        columns
                            .fitting
                            .iter()
                            .any(|(key, _)| *key == shown.orders.active_key)
                    }),
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
                            &around,
                            mods.clone(),
                            &thumbs,
                            list_id,
                            cx,
                        );
                        let shown = div().pr(room).child(drawn);
                        match slides.get(&row.mod_id) {
                            Some(&places) if !row.is_header => {
                                let most = theme.sized(400.);
                                let from = (rows::height(theme, row, side == Side::Active)
                                    * places as f32)
                                    .clamp(-most, most);
                                div()
                                    .size_full()
                                    .child(
                                        shown.w_full().h_full().flex_none().with_animation(
                                            ("slide", (slide_turn as usize) << 20 | at),
                                            gpui::Animation::new(SLIDE)
                                                .with_easing(|t| 1. - (1. - t).powi(3)),
                                            move |row, done| row.mt(from * (1. - done)),
                                        ),
                                    )
                                    .into_any_element()
                            }
                            _ => shown.into_any_element(),
                        }
                    })
                    .collect()
            },
        );
        let focused = self.focus.is_focused(window);
        let (keyed, focus) = (self.mods.clone(), self.focus.clone());
        let heading = self
            .top_category()
            .map(|(category, top)| self.top_heading(&theme, category, top, cx));
        let footer = self.footer(&theme, cx);
        div()
            .id(("mod-list-pane", side as usize))
            .track_focus(&self.focus)
            .role(gpui::Role::List)
            .aria_label(crate::words::put(&match side {
                Side::Active => rimstack_core::say!("Load order"),
                Side::Inactive => rimstack_core::say!("Inactive mods"),
            }))
            .relative()
            .size_full()
            .when(self.shown.dimmed, |d| d.opacity(0.5))
            .child(
                gpui::canvas(
                    move |bounds, _, _| drawn_top.set(bounds.top()),
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .on_mouse_move(cx.listener(|list, event: &gpui::MouseMoveEvent, _, cx| {
                list.point(Some(event.position.y), cx)
            }))
            .on_hover(cx.listener(|list, over: &bool, _, cx| {
                if !over {
                    list.point(None, cx);
                }
            }))
            .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                window.focus(&focus, cx)
            })
            .on_drag_move::<rows::Held>(cx.listener(
                |list, event: &gpui::DragMoveEvent<rows::Held>, _, cx| {
                    list.dragged_over(event.event.position, event.bounds, cx)
                },
            ))
            .on_key_down(move |event: &gpui::KeyDownEvent, window, cx| {
                let _ = keyed.update(cx, |mods, cx| mods.list_key(side, event, window, cx));
            })
            .child(list)
            .children(footer)
            .children(heading)
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

impl ModList {
    /// The list's footer: below the last row, where a drop puts the mods at the list's end.
    fn footer(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let (_, below) = spacer(theme, self.side);
        let end = *self.tops.last()? - below;
        let scroll = self.scroll.borrow();
        let from = end - scroll.top();
        if from > scroll.shown() && scroll.shown() > px(0.) {
            return None;
        }
        drop(scroll);
        let (mods, side, count) = (self.mods.clone(), self.side, self.shown.count);
        let line = div()
            .absolute()
            .left(theme::GAP_SMALL)
            .right(theme::GAP_SMALL)
            .top_0()
            .h(px(2.))
            .rounded(px(1.))
            .bg(theme.colours.focus)
            .invisible()
            .group_drag_over::<rows::Held>("list-end", |s| s.visible());
        let _ = cx;
        Some(
            div()
                .id(("list-end", side as usize))
                .group("list-end")
                .absolute()
                .left_0()
                .right(bar_room(theme))
                .top(from.max(px(0.)))
                .bottom_0()
                .child(line)
                .on_drop(move |held: &rows::Held, _, cx| {
                    let held = held.clone();
                    let _ = mods.update(cx, |mods, cx| {
                        mods.drop_on((held.side, held.index, held.ids), (side, count), cx)
                    });
                })
                .into_any_element(),
        )
    }

    /// The heading of the category at the list's top, kept there while its mods scroll under it
    /// and pushed up by the next category's heading.
    fn top_heading(
        &self,
        theme: &Theme,
        category: i32,
        top: Pixels,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let c = &theme.colours;
        let tall = theme.control();
        let next = self
            .tops
            .partition_point(|row| *row <= top + tall)
            .saturating_sub(1);
        let pushed = self
            .shown
            .rows
            .get(next)
            .filter(|r| r.is_header && !r.is_sub && !r.again)
            .map_or(px(0.), |_| (self.tops[next] - top - tall).min(px(0.)));
        let colour = usize::try_from(category).map_or(c.ink3, |at| theme.category_colour(at));
        let icon = usize::try_from(category)
            .ok()
            .and_then(|at| theme::CATEGORY_ICONS.get(at))
            .copied()
            .unwrap_or("package");
        let name = usize::try_from(category)
            .ok()
            .and_then(|at| theme::CATEGORY_NAMES.get(at))
            .map(|name| {
                crate::words::put(&rimstack_core::base::sentence::Sentence::said(
                    name,
                    Vec::new(),
                ))
            })
            .unwrap_or_default();
        let mods = self.mods.clone();
        let _ = cx;
        div()
            .id(("top-heading", self.side as usize))
            .absolute()
            .left_0()
            .right(bar_room(theme))
            .top(pushed)
            .h(tall)
            .bg(match self.side {
                Side::Active => c.win,
                Side::Inactive => c.panel,
            })
            .child(
                div()
                    .absolute()
                    .left(px(2.))
                    .top_0()
                    .bottom_0()
                    .w(px(3.))
                    .bg(colour),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .pl(theme::GAP_WIDE + theme.sized(10.) + theme::GAP_SMALL)
                    .flex()
                    .items_center()
                    .gap(theme::GAP_SMALL)
                    .child(crate::ui::widgets::icon(icon, theme.icon(), colour))
                    .child(
                        crate::ui::widgets::label(theme, theme.small(), c.ink, name)
                            .font_weight(theme.semibold),
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
            )
            .on_mouse_down(gpui::MouseButton::Right, move |event, window, cx| {
                let at = event.position;
                let _ = mods.update(cx, |mods, cx| mods.category_menu(category, at, window, cx));
            })
            .into_any_element()
    }
}
