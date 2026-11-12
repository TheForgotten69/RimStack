//! `Toggle.qml`, `Slide.qml` and `Choice.qml`: a switch, a slider and a row of segments. Each is
//! reached by Tab, moved by its keys, and tells a screen reader its state.

use super::focus;
use super::theme::{self, Theme};
use super::widgets::{TextSized, measure, watched};
use crate::words::put;
use gpui::{
    AnyElement, App, Bounds, ElementId, FontWeight, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, Pixels, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, Styled, Toggled, Window, canvas, div,
    prelude::FluentBuilder, px,
};
use rimstack_core::say;
use std::cell::Cell;
use std::rc::Rc;

type Flip = Box<dyn Fn(bool, &mut Window, &mut App)>;
type Moved = Rc<dyn Fn(f32, &mut Window, &mut App)>;
type Picked = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// `Toggle.qml`: a switch, and "On" or "Off" beside it unless `silent`.
#[derive(IntoElement)]
pub struct Toggle {
    id: ElementId,
    on: bool,
    title: SharedString,
    silent: bool,
    dimmed: bool,
    flip: Option<Flip>,
}

impl Toggle {
    pub fn new(id: impl Into<ElementId>, on: bool, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            on,
            title: title.into(),
            silent: false,
            dimmed: false,
            flip: None,
        }
    }

    pub fn silent(mut self) -> Self {
        self.silent = true;
        self
    }

    /// On but not in use yet, such as a database not downloaded.
    pub fn dimmed(mut self, dimmed: bool) -> Self {
        self.dimmed = dimmed;
        self
    }

    pub fn on_flip(mut self, flip: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.flip = Some(Box::new(flip));
        self
    }
}

impl RenderOnce for Toggle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = focus::handle(self.id.clone(), window, cx);
        let theme = cx.global::<Theme>();
        let c = &theme.colours;
        let (height, on) = (theme.segment(), self.on);
        let knob = height - px(6.);
        let track = div()
            .id(self.id)
            .role(Role::CheckBox)
            .aria_label(self.title.clone())
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .relative()
            .flex_none()
            .w(theme.sized(38.))
            .h(height)
            .rounded(height / 2.)
            .bg(if on { c.accent } else { c.ctl })
            .when(!on, |d| d.border_1().border_color(c.field_line))
            .cursor_pointer()
            .child(
                div()
                    .absolute()
                    .top(px(3.) - if on { px(0.) } else { px(1.) })
                    .when(on, |d| d.right(px(3.)))
                    .when(!on, |d| d.left(px(2.)))
                    .size(knob)
                    .rounded(knob / 2.)
                    .bg(if on { c.accent_ink } else { c.ink2 }),
            )
            .when_some(self.flip, |d, flip| {
                d.on_click(move |_, window, cx| flip(!on, window, cx))
            });
        let track = focus::reachable(track, &handle, height / 2., false, window, theme);
        let said = if on {
            put(&say!("On"))
        } else {
            put(&say!("Off"))
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(theme::GAP)
            .when(self.dimmed, |d| d.opacity(0.45))
            .child(track)
            .when(!self.silent, |d| {
                d.child(watched(
                    div()
                        .whitespace_nowrap()
                        .sized_text(theme, theme.small())
                        .text_color(c.ink2),
                    said.clone().into(),
                    said,
                    false,
                ))
            })
    }
}

/// `Slide.qml`: a value between two ends, in steps, set by a click, a drag or the arrow keys.
#[derive(IntoElement)]
pub struct Slide {
    id: ElementId,
    from: f32,
    to: f32,
    step: f32,
    value: f32,
    title: SharedString,
    reading: SharedString,
    width: Option<Pixels>,
    moved: Option<Moved>,
}

impl Slide {
    pub fn new(id: impl Into<ElementId>, (from, to, step): (f32, f32, f32), value: f32) -> Self {
        Self {
            id: id.into(),
            from,
            to,
            step,
            value,
            title: SharedString::default(),
            reading: SharedString::default(),
            width: None,
            moved: None,
        }
    }

    /// What a screen reader names it, and what it reads of its value.
    pub fn said(
        mut self,
        title: impl Into<SharedString>,
        reading: impl Into<SharedString>,
    ) -> Self {
        self.title = title.into();
        self.reading = reading.into();
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn on_move(mut self, moved: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.moved = Some(Rc::new(moved));
        self
    }
}

impl RenderOnce for Slide {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = focus::handle(self.id.clone(), window, cx);
        let held = window.use_keyed_state((self.id.clone(), "held"), cx, |_, _| false);
        let theme = cx.global::<Theme>();
        let c = &theme.colours;
        let knob = theme.sized(16.);
        let part = ((self.value - self.from) / (self.to - self.from)).clamp(0., 1.);
        let bounds: Rc<Cell<Bounds<Pixels>>> = Rc::default();
        let (from, to, step, value) = (self.from, self.to, self.step, self.value);
        let at = {
            let bounds = bounds.clone();
            move |x: Pixels| {
                let b = bounds.get();
                let track = b.size.width - knob;
                let raw = from + ((x - b.left() - knob / 2.) / track).clamp(0., 1.) * (to - from);
                (raw / step).round() * step
            }
        };
        let put_at = {
            let moved = self.moved.clone();
            move |x: Pixels, window: &mut Window, cx: &mut App| {
                let next = at(x);
                if let Some(moved) = &moved
                    && (next - value).abs() > 1e-6
                {
                    moved(next, window, cx);
                }
            }
        };
        let put_at = Rc::new(put_at);
        let width = self.width.unwrap_or(theme.sized(180.));
        let control = div()
            .id(self.id)
            .role(Role::Slider)
            .aria_label(self.title)
            .aria_value(self.reading)
            .aria_numeric_value(f64::from(value))
            .aria_min_numeric_value(f64::from(from))
            .aria_max_numeric_value(f64::from(to))
            .aria_numeric_value_step(f64::from(step))
            .relative()
            .flex_none()
            .w(width)
            .h(theme.control())
            .cursor_pointer()
            .child(
                canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {})
                    .absolute()
                    .inset_0(),
            )
            .child(
                div()
                    .absolute()
                    .left(knob / 2.)
                    .right(knob / 2.)
                    .top((theme.control() - theme::GAP_TINY) / 2.)
                    .h(theme::GAP_TINY)
                    .rounded(theme::GAP_TINY / 2.)
                    .bg(c.ctl)
                    .child(
                        div()
                            .h_full()
                            .w(gpui::relative(part))
                            .rounded(theme::GAP_TINY / 2.)
                            .bg(c.accent),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .left((width - knob) * part)
                    .top((theme.control() - knob) / 2.)
                    .size(knob)
                    .rounded(knob / 2.)
                    .bg(c.ink),
            )
            .on_mouse_down(MouseButton::Left, {
                let (put_at, held) = (put_at.clone(), held.clone());
                move |event: &MouseDownEvent, window, cx| {
                    held.update(cx, |held, _| *held = true);
                    put_at(event.position.x, window, cx);
                }
            })
            .on_mouse_move({
                let (put_at, held) = (put_at.clone(), held.clone());
                move |event: &MouseMoveEvent, window, cx| {
                    if *held.read(cx) && event.pressed_button == Some(MouseButton::Left) {
                        put_at(event.position.x, window, cx);
                    }
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let held = held.clone();
                move |_, _, cx| held.update(cx, |held, _| *held = false)
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                held.update(cx, |held, _| *held = false)
            })
            .when_some(self.moved, |d, moved| {
                d.on_key_down(move |event: &KeyDownEvent, window, cx| {
                    let next = match event.keystroke.key.as_str() {
                        "left" => (value - step).max(from),
                        "right" => (value + step).min(to),
                        _ => return,
                    };
                    moved(next, window, cx);
                })
            });
        focus::reachable(control, &handle, knob / 2., false, window, theme)
    }
}

/// `Choice.qml`: one of a few, side by side; `stretch` to a width, stacked when it is too narrow.
#[derive(IntoElement)]
pub struct Choice {
    id: ElementId,
    options: Vec<SharedString>,
    current: usize,
    title: SharedString,
    stretch: Option<Pixels>,
    picked: Option<Picked>,
}

impl Choice {
    pub fn new(id: impl Into<ElementId>, options: Vec<SharedString>, current: usize) -> Self {
        Self {
            id: id.into(),
            options,
            current,
            title: SharedString::default(),
            stretch: None,
            picked: None,
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    pub fn stretch(mut self, width: Pixels) -> Self {
        self.stretch = Some(width);
        self
    }

    pub fn on_pick(mut self, picked: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.picked = Some(Rc::new(picked));
        self
    }

    /// Its width when not stretched: its words, the room around each, and its frame.
    pub fn width(window: &Window, theme: &Theme, options: &[SharedString]) -> Pixels {
        options.iter().fold(px(4.), |all, option| {
            all + measure(window, theme, option, theme.small(), theme.semibold)
                + theme::GAP_LARGE * 2.
        })
    }
}

impl RenderOnce for Choice {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = focus::handle(self.id.clone(), window, cx);
        let theme = cx.global::<Theme>();
        let c = &theme.colours;
        let words: Vec<Pixels> = self
            .options
            .iter()
            .map(|o| measure(window, theme, o, theme.small(), theme.semibold))
            .collect();
        let all_words = words.iter().fold(px(0.), |all, w| all + *w);
        let count = self.options.len().max(1) as f32;
        let stacked = self
            .stretch
            .is_some_and(|width| (width - px(4.) - all_words) / count < px(6.));
        let current = self.current;
        let segments: Vec<AnyElement> = self
            .options
            .iter()
            .enumerate()
            .map(|(at, option)| {
                let on = at == current;
                let width = match self.stretch {
                    None => words[at] + theme::GAP_LARGE * 2.,
                    Some(whole) if stacked => whole - px(4.),
                    Some(whole) => words[at] + (whole - px(4.) - all_words) / count,
                };
                let ink = if on { c.ink } else { c.ink2 };
                let weight = if on {
                    theme.semibold
                } else {
                    FontWeight::NORMAL
                };
                let text = watched(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .sized_text(theme, theme.small())
                        .font_weight(weight)
                        .text_color(ink),
                    option.clone(),
                    option.clone(),
                    true,
                );
                div()
                    .id(("segment", at))
                    .role(Role::RadioButton)
                    .aria_label(option.clone())
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .flex()
                    .items_center()
                    .when(!stacked, |d| d.justify_center())
                    .when(stacked, |d| d.px(theme::GAP_MEDIUM))
                    .w(width)
                    .h(theme.segment())
                    .rounded(theme::CORNER - px(1.))
                    .when(on, |d| d.bg(c.ctl_hi))
                    .child(text)
                    .when_some(self.picked.clone(), |d, picked| {
                        d.on_click(move |_, window, cx| picked(at, window, cx))
                    })
                    .into_any_element()
            })
            .collect();
        let last = self.options.len().saturating_sub(1);
        let control = div()
            .id(self.id)
            .role(Role::RadioGroup)
            .aria_label(self.title)
            .relative()
            .flex()
            .flex_none()
            .when(stacked, |d| d.flex_col())
            .p(px(2.))
            .rounded(theme::CORNER)
            .bg(c.seg)
            .border_width(theme.outline())
            .border_color(c.field_line)
            .children(segments)
            .when_some(self.picked, |d, picked| {
                d.on_key_down(move |event: &KeyDownEvent, window, cx| {
                    let next = match event.keystroke.key.as_str() {
                        "left" | "up" if current > 0 => current - 1,
                        "right" | "down" if current < last => current + 1,
                        _ => return,
                    };
                    picked(next, window, cx);
                })
            });
        focus::reachable(control, &handle, theme::CORNER, false, window, theme)
    }
}
