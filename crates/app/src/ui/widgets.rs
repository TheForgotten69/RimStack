use super::theme::{self, Theme};
use gpui::{
    AbsoluteLength, AnyElement, App as Cx, ClickEvent, ElementId, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px, svg,
};

pub trait Sized: Styled + std::marker::Sized {
    /// A text at this size, as tall as Qt makes its line.
    fn sized_text(self, theme: &Theme, size: Pixels) -> Self {
        self.text_size(size).line_height(theme.line(size))
    }

    fn border_width(mut self, width: Pixels) -> Self {
        let width: AbsoluteLength = width.into();
        let widths = &mut self.style().border_widths;
        (widths.top, widths.right, widths.bottom, widths.left) =
            (Some(width), Some(width), Some(width), Some(width));
        self
    }
}

impl<T: Styled> Sized for T {}

pub fn icon(name: &str, size: Pixels, colour: Hsla) -> gpui::Svg {
    svg()
        .path(format!("icons/{name}.svg"))
        .size(size)
        .flex_none()
        .text_color(colour)
}

pub fn with_alpha(colour: Hsla, alpha: f32) -> Hsla {
    Hsla {
        a: colour.a * alpha,
        ..colour
    }
}

/// `Halo.qml`: rings of the tone, fading outwards, around something to press.
pub fn halo(tone: Hsla, corner: Pixels) -> impl Iterator<Item = AnyElement> {
    const RINGS: usize = 4;
    const STEP: f32 = 1.5;
    (0..RINGS).map(move |at| {
        let out = px(-(at as f32 + 1.) * STEP);
        div()
            .absolute()
            .top(out)
            .bottom(out)
            .left(out)
            .right(out)
            .rounded(corner + px((at as f32 + 1.) * STEP))
            .border_width(px(STEP))
            .border_color(with_alpha(tone, 0.3 * 0.62f32.powi(at as i32)))
            .into_any_element()
    })
}

type Click = Box<dyn Fn(&ClickEvent, &mut Window, &mut Cx)>;

/// `Btn.qml`.
#[derive(IntoElement)]
pub struct Btn {
    id: ElementId,
    text: SharedString,
    icon: Option<&'static str>,
    primary: bool,
    large: bool,
    menu: bool,
    height: Option<Pixels>,
    on_click: Option<Click>,
}

impl Btn {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            icon: None,
            primary: false,
            large: false,
            menu: false,
            height: None,
            on_click: None,
        }
    }

    pub fn icon(mut self, name: &'static str) -> Self {
        self.icon = Some(name);
        self
    }

    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    pub fn large(mut self) -> Self {
        self.large = true;
        self
    }

    pub fn menu(mut self) -> Self {
        self.menu = true;
        self
    }

    pub fn height(mut self, height: Pixels) -> Self {
        self.height = Some(height);
        self
    }

    pub fn on_click(mut self, run: impl Fn(&ClickEvent, &mut Window, &mut Cx) + 'static) -> Self {
        self.on_click = Some(Box::new(run));
        self
    }
}

impl RenderOnce for Btn {
    fn render(self, _: &mut Window, cx: &mut Cx) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let c = &theme.colours;
        let ink = if self.primary { c.accent_ink } else { c.ink };
        let padding = match (self.large, self.text.is_empty() && !self.menu) {
            (true, _) => theme.sized(36.),
            (false, true) => theme.sized(14.),
            (false, false) => theme.sized(20.),
        };
        let (ground, hovered) = match self.primary {
            true => (c.accent, c.accent_hi),
            false => (c.ctl, c.ctl_hi),
        };
        let size = if self.large {
            theme.body()
        } else {
            theme.small()
        };
        let weight = match (self.large, self.primary) {
            (true, _) => theme.semibold,
            (false, true) => theme.medium,
            (false, false) => FontWeight::NORMAL,
        };
        let height = self.height.unwrap_or(if self.large {
            theme.control_large()
        } else {
            theme.control()
        });
        div()
            .id(self.id)
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(theme::GAP_SMALL)
            .h(height)
            .px(padding / 2.)
            .rounded(theme::CORNER)
            .bg(ground)
            .hover(move |s| s.bg(hovered))
            .when(theme.appearance.high_contrast, |d| {
                d.border_1().border_color(c.field_line)
            })
            .when(self.primary, |d| d.children(halo(c.accent, theme::CORNER)))
            .when_some(self.icon, |d, name| d.child(icon(name, theme.icon(), ink)))
            .when(!self.text.is_empty(), |d| {
                d.child(
                    div()
                        .whitespace_nowrap()
                        .sized_text(theme, size)
                        .font_weight(weight)
                        .text_color(ink)
                        .child(self.text),
                )
            })
            .when(self.menu, |d| {
                d.child(icon("chevron-down", theme.sized(12.), with_alpha(ink, 0.8)))
            })
            .when_some(self.on_click, |d, run| d.on_click(run))
    }
}

/// `Chip.qml`, not pressable.
#[derive(IntoElement)]
pub struct Chip {
    text: SharedString,
    tone: Option<Hsla>,
}

impl Chip {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            tone: None,
        }
    }

    pub fn tone(mut self, tone: Hsla) -> Self {
        self.tone = Some(tone);
        self
    }
}

impl RenderOnce for Chip {
    fn render(self, _: &mut Window, cx: &mut Cx) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let tone = self.tone.unwrap_or(theme.colours.ink2);
        div()
            .flex()
            .flex_none()
            .items_center()
            .h(theme.tag())
            .px(theme::GAP_CARD / 2.)
            .rounded(theme.tag() / 2.)
            .bg(with_alpha(tone, 0.16))
            .child(
                div()
                    .whitespace_nowrap()
                    .sized_text(theme, theme.caption())
                    .font_weight(theme.medium)
                    .text_color(tone)
                    .child(self.text),
            )
    }
}

/// A text in the Theme's terms: `label(theme, theme.body(), theme.colours.ink, "…")`.
pub fn label(
    theme: &Theme,
    size: Pixels,
    colour: Hsla,
    text: impl Into<SharedString>,
) -> gpui::Div {
    div()
        .flex_none()
        .whitespace_nowrap()
        .sized_text(theme, size)
        .text_color(colour)
        .child(text.into())
}
