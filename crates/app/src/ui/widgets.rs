use super::theme::{self, Theme};
use gpui::{
    AbsoluteLength, AnyElement, App as Cx, ClickEvent, ElementId, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px, svg,
};

pub trait TextSized: Styled + Sized {
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

impl<T: Styled> TextSized for T {}

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
    enabled: bool,
    tone: Option<Hsla>,
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
            enabled: true,
            tone: None,
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

    pub fn primary_if(mut self, primary: bool) -> Self {
        self.primary = primary;
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

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Tinted: the tone as its ground and its ink.
    pub fn tone(mut self, tone: Hsla) -> Self {
        self.tone = Some(tone);
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
        let toned = self.tone.filter(|_| !self.primary);
        let ink = match (self.primary, toned) {
            (true, _) => c.accent_ink,
            (false, Some(tone)) => tone,
            (false, None) => c.ink,
        };
        let padding = match (self.large, self.text.is_empty() && !self.menu) {
            (true, _) => theme.sized(36.),
            (false, true) => theme.sized(14.),
            (false, false) => theme.sized(20.),
        };
        let (ground, hovered) = match (self.primary, toned) {
            (true, _) => (c.accent, c.accent_hi),
            (false, Some(tone)) => (with_alpha(tone, 0.18), with_alpha(tone, 0.3)),
            (false, None) => (c.ctl, c.ctl_hi),
        };
        let frame = if self.enabled { 1. } else { 0.4 };
        let size = if self.large {
            theme.body()
        } else {
            theme.small()
        };
        let weight = match (self.large, self.primary || toned.is_some()) {
            (true, _) => theme.semibold,
            (false, true) => theme.medium,
            (false, false) => FontWeight::NORMAL,
        };
        let height = self.height.unwrap_or(if self.large {
            theme.control_large()
        } else {
            theme.control()
        });
        let line = toned.unwrap_or(c.field_line);
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
            .bg(with_alpha(ground, frame))
            .when(self.enabled, |d| d.hover(move |s| s.bg(hovered)))
            .when(theme.appearance.high_contrast, |d| {
                d.border_1().border_color(with_alpha(line, frame))
            })
            .when(self.primary && self.enabled, |d| {
                d.children(halo(c.accent, theme::CORNER))
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(theme::GAP_SMALL)
                    .when(!self.enabled, |d| d.opacity(0.5))
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
                    }),
            )
            .when_some(self.on_click.filter(|_| self.enabled), |d, run| {
                d.on_click(run)
            })
    }
}

/// `Notice.qml`: a band in a tone, with a bar at its left.
pub fn notice(tone: Hsla) -> gpui::Div {
    div()
        .relative()
        .rounded(theme::GAP_HAIR)
        .bg(with_alpha(tone, 0.06))
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .bottom_0()
                .w(px(2.))
                .bg(tone),
        )
}

/// `Chip.qml`.
#[derive(IntoElement)]
pub struct Chip {
    text: SharedString,
    tone: Option<Hsla>,
    icon: Option<&'static str>,
    on_accent: bool,
    pressable: bool,
    max_width: Option<Pixels>,
}

impl Chip {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            tone: None,
            icon: None,
            on_accent: false,
            pressable: false,
            max_width: None,
        }
    }

    pub fn icon(mut self, name: &'static str) -> Self {
        self.icon = Some(name);
        self
    }

    /// On a chosen row: a dark ground and the chosen row's ink.
    pub fn on_accent(mut self, on: bool) -> Self {
        self.on_accent = on;
        self
    }

    /// As tall as a button, for a chip that is pressed.
    pub fn pressable(mut self) -> Self {
        self.pressable = true;
        self
    }

    pub fn max_width(mut self, width: Pixels) -> Self {
        self.max_width = Some(width);
        self
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
        let ink = if self.on_accent {
            theme.colours.on_selected
        } else {
            tone
        };
        let ground = if self.on_accent {
            theme::hex("#38000000")
        } else {
            with_alpha(tone, 0.16)
        };
        let height = if self.pressable {
            theme.control()
        } else {
            theme.tag()
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(theme::GAP_TINY)
            .h(height)
            .px(theme::GAP_CARD / 2.)
            .rounded(height / 2.)
            .bg(ground)
            .when_some(self.max_width, |d, most| d.max_w(most))
            .when_some(self.icon, |d, name| {
                d.child(icon(name, theme.sized(11.), ink))
            })
            .when(!self.text.is_empty(), |d| {
                d.child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .sized_text(theme, theme.caption())
                        .font_weight(theme.medium)
                        .text_color(ink)
                        .child(self.text),
                )
            })
    }
}

/// A text's width as drawn in the Theme's family, as `TextMetrics.advanceWidth` gives it.
pub fn measure(
    window: &Window,
    theme: &Theme,
    text: &str,
    size: Pixels,
    weight: FontWeight,
) -> Pixels {
    measure_in(window, theme, theme.family.clone(), text, size, weight)
}

pub fn measure_in(
    window: &Window,
    theme: &Theme,
    family: SharedString,
    text: &str,
    size: Pixels,
    weight: FontWeight,
) -> Pixels {
    if text.is_empty() {
        return Pixels::ZERO;
    }
    let mut font = gpui::font(family);
    font.weight = weight;
    let run = gpui::TextRun {
        len: text.len(),
        font,
        color: theme.colours.ink,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(SharedString::from(text.to_string()), size, &[run], None)
        .width
}

/// `Text.ElideMiddle`: as many characters kept at each end as fit, with "…" between.
pub fn elide_middle(
    window: &Window,
    theme: &Theme,
    text: &str,
    size: Pixels,
    room: Pixels,
) -> String {
    elide_middle_in(window, theme, theme.family.clone(), text, size, room)
}

pub fn elide_middle_in(
    window: &Window,
    theme: &Theme,
    family: SharedString,
    text: &str,
    size: Pixels,
    room: Pixels,
) -> String {
    let fits = |shown: &str| {
        measure_in(
            window,
            theme,
            family.clone(),
            shown,
            size,
            FontWeight::NORMAL,
        ) <= room
    };
    if fits(text) {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let shown = |kept: usize| {
        let (left, right) = (kept.div_ceil(2), kept / 2);
        let start: String = chars[..left].iter().collect();
        let end: String = chars[chars.len() - right..].iter().collect();
        format!("{start}…{end}")
    };
    let (mut low, mut high) = (0, chars.len().saturating_sub(1));
    while low < high {
        let middle = (low + high).div_ceil(2);
        match fits(&shown(middle)) {
            true => low = middle,
            false => high = middle - 1,
        }
    }
    shown(low)
}

/// The width `Chip.qml` takes: its icon, its text and the room around them.
pub fn chip_width(window: &Window, theme: &Theme, text: &str, icon: bool) -> Pixels {
    let shown = if icon {
        theme.sized(11.) + theme::GAP_TINY
    } else {
        Pixels::ZERO
    };
    shown + measure(window, theme, text, theme.caption(), theme.medium) + theme::GAP_CARD
}

/// The width `Btn.qml` takes: its icon, its text, its menu arrow and the room around them.
pub fn btn_width(window: &Window, theme: &Theme, text: &str, icon: bool, menu: bool) -> Pixels {
    btn_width_of(window, theme, text, icon, menu, false)
}

/// As `btn_width`, for a large button too (`Btn.large`, as Play is).
pub fn btn_width_of(
    window: &Window,
    theme: &Theme,
    text: &str,
    icon: bool,
    menu: bool,
    large: bool,
) -> Pixels {
    let (size, weight) = match large {
        true => (theme.body(), theme.semibold),
        false => (theme.small(), FontWeight::NORMAL),
    };
    let parts = [
        icon.then(|| theme.icon()),
        (!text.is_empty()).then(|| measure(window, theme, text, size, weight)),
        menu.then(|| theme.sized(12.)),
    ];
    let shown: Vec<Pixels> = parts.into_iter().flatten().collect();
    let gaps = theme::GAP_SMALL * shown.len().saturating_sub(1) as f32;
    let room = match (large, text.is_empty() && !menu) {
        (true, _) => theme.sized(36.),
        (false, true) => theme.sized(14.),
        (false, false) => theme.sized(20.),
    };
    shown.into_iter().fold(gaps + room, |all, part| all + part)
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
