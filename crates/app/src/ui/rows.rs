//! A list whose rows' heights are known before they are drawn: only the rows in view are made,
//! each at its exact place, and the wheel glides as `Wheel.qml` made it glide.

use gpui::{
    AnyElement, App as Cx, AvailableSpace, Bounds, ContentMask, Element, ElementId, EntityId,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId, Pixels,
    ScrollDelta, ScrollEvent, Size, Style, Window, point, px, relative, size,
};
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// How long a wheel notch takes to land, as `Wheel.qml` eased it.
const GLIDE: Duration = Duration::from_millis(200);

/// Where a list is scrolled, and where a wheel notch is taking it.
#[derive(Default)]
pub struct Scroll {
    top: Pixels,
    from: Pixels,
    goal: Pixels,
    started: Option<Instant>,
    /// The rows' whole height and the height shown, as last drawn.
    total: Pixels,
    shown: Pixels,
    /// While the bar's thumb is held: how far below the thumb's top it was taken.
    pub held: Option<Pixels>,
    reduce_motion: bool,
}

pub type Scrolling = Rc<RefCell<Scroll>>;

impl Scroll {
    pub fn most(&self) -> Pixels {
        (self.total - self.shown).max(px(0.))
    }

    pub fn total(&self) -> Pixels {
        self.total
    }

    pub fn shown(&self) -> Pixels {
        self.shown
    }

    pub fn top(&self) -> Pixels {
        self.top
    }

    /// Goes straight to `top`, as the bar's thumb and a kept place do.
    pub fn set_top(&mut self, top: Pixels) {
        self.top = top.clamp(px(0.), self.most());
        self.goal = self.top;
        self.started = None;
    }

    pub fn set_reduce_motion(&mut self, reduce: bool) {
        self.reduce_motion = reduce;
    }

    /// A wheel's notches glide by `notch` each; a touchpad's pixels move the list as they come.
    pub fn wheel(&mut self, delta: ScrollDelta, line: Pixels) {
        match delta {
            ScrollDelta::Pixels(moved) => self.set_top(self.top - moved.y),
            ScrollDelta::Lines(lines) if lines.y != 0. => {
                let from = if self.started.is_some() {
                    self.goal
                } else {
                    self.top
                };
                self.goal = (from - line * lines.y).clamp(px(0.), self.most());
                self.from = self.top;
                self.started = Some(Instant::now());
                if self.reduce_motion {
                    self.set_top(self.goal);
                }
            }
            ScrollDelta::Lines(_) => {}
        }
    }

    /// Moves the glide on to now; true while it has further to go.
    fn advance(&mut self) -> bool {
        let Some(started) = self.started else {
            return false;
        };
        let part = (started.elapsed().as_secs_f32() / GLIDE.as_secs_f32()).min(1.);
        let eased = 1. - (1. - part).powi(3);
        self.top = self.from + (self.goal - self.from) * eased;
        if part >= 1. {
            self.started = None;
        }
        part < 1.
    }

    fn fit(&mut self, total: Pixels, shown: Pixels) {
        (self.total, self.shown) = (total, shown);
        self.top = self.top.clamp(px(0.), self.most());
        self.goal = self.goal.clamp(px(0.), self.most());
    }
}

type Draw = Box<dyn Fn(Range<usize>, Pixels, &mut Window, &mut Cx) -> Vec<AnyElement>>;

/// `tops` holds each row's top and, last, the whole height.
pub fn rows(
    id: impl Into<ElementId>,
    scroll: Scrolling,
    tops: Rc<[Pixels]>,
    line: Pixels,
    view: EntityId,
    draw: impl Fn(Range<usize>, Pixels, &mut Window, &mut Cx) -> Vec<AnyElement> + 'static,
) -> Rows {
    Rows {
        id: id.into(),
        scroll,
        tops,
        line,
        view,
        draw: Box::new(draw),
    }
}

pub struct Rows {
    id: ElementId,
    scroll: Scrolling,
    tops: Rc<[Pixels]>,
    line: Pixels,
    view: EntityId,
    draw: Draw,
}

impl IntoElement for Rows {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Rows {
    type RequestLayoutState = ();
    type PrepaintState = (Vec<AnyElement>, Hitbox);

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut Cx,
    ) -> (LayoutId, ()) {
        let style = Style {
            size: Size {
                width: relative(1.).into(),
                height: relative(1.).into(),
            },
            ..Style::default()
        };
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut Cx,
    ) -> (Vec<AnyElement>, Hitbox) {
        let total = self.tops.last().copied().unwrap_or_default();
        let gliding = {
            let mut scroll = self.scroll.borrow_mut();
            scroll.fit(total, bounds.size.height);
            scroll.advance()
        };
        if gliding {
            window.request_animation_frame();
        }
        let top = self.scroll.borrow().top();
        let count = self.tops.len().saturating_sub(1);
        let first = self
            .tops
            .partition_point(|row| *row <= top)
            .saturating_sub(1);
        let end = self
            .tops
            .partition_point(|row| *row < top + bounds.size.height)
            .min(count);
        let mut drawn = (self.draw)(first..end, bounds.size.width, window, cx);
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for (at, row) in (first..end).zip(drawn.iter_mut()) {
                let height = self.tops[at + 1] - self.tops[at];
                let room = size(
                    AvailableSpace::Definite(bounds.size.width),
                    AvailableSpace::Definite(height),
                );
                row.layout_as_root(room, window, cx);
                row.prepaint_at(
                    bounds.origin + point(px(0.), self.tops[at] - top),
                    window,
                    cx,
                );
            }
        });
        (drawn, window.insert_hitbox(bounds, HitboxBehavior::Normal))
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        (drawn, hitbox): &mut (Vec<AnyElement>, Hitbox),
        window: &mut Window,
        cx: &mut Cx,
    ) {
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for row in drawn.iter_mut() {
                row.paint(window, cx);
            }
        });
        let (scroll, line, view, hitbox) =
            (self.scroll.clone(), self.line, self.view, hitbox.clone());
        window.on_mouse_event(move |event: &ScrollEvent, phase, window, cx| {
            if phase.bubble() && hitbox.should_handle_scroll(window) {
                scroll.borrow_mut().wheel(event.delta, line);
                cx.notify(view);
                cx.stop_propagation();
            }
        });
    }
}
