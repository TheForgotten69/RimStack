//! `Tip.qml` and the window's `ToolTip`: words about a control, shown when the pointer has
//! rested on it.

use super::theme::{self, Theme};
use super::widgets::TextSized;
use gpui::{
    AnyView, App, AppContext, Context, IntoElement, ParentElement, Render, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Window, div,
};
use std::time::Duration;

/// Main.qml's timer before a tip shows.
const DELAY: Duration = Duration::from_millis(600);

struct Tip(SharedString);

impl Render for Tip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let c = &theme.colours;
        div()
            .max_w(theme.sized(340.) + theme::GAP * 2.)
            .p(theme::GAP)
            .rounded(theme::CORNER)
            .bg(c.bar)
            .border_1()
            .border_color(c.line)
            .child(
                div()
                    .spaced_text(theme, theme.small(), 1.25)
                    .text_color(c.ink)
                    .child(self.0.clone()),
            )
    }
}

/// The tip of what the pointer is on, as `Notices.hint`: the status bar shows its first line.
#[derive(Default)]
pub struct Hint(pub SharedString);

impl gpui::Global for Hint {}

/// What the status bar says of the control pointed at, empty when none.
pub fn hint(cx: &App) -> SharedString {
    cx.try_global::<Hint>()
        .map(|hint| hint.0.clone())
        .unwrap_or_default()
}

/// The hint let go of, as a press, a key or the wheel may take the control away under a pointer
/// that does not move, which would then never tell it left.
pub fn forget_hint(cx: &mut App) {
    if !hint(cx).is_empty() {
        cx.set_global(Hint::default());
    }
}

/// `control` with its tip; an empty one shows none.
pub fn tipped(control: Stateful<gpui::Div>, text: SharedString) -> Stateful<gpui::Div> {
    if text.is_empty() {
        return control;
    }
    let pointed = text.clone();
    control
        .on_hover(move |over, _, cx| {
            if *over {
                cx.set_global(Hint(pointed.clone()));
            } else if hint(cx) == pointed {
                cx.set_global(Hint::default());
            }
        })
        .tooltip(move |_: &mut Window, cx: &mut App| -> AnyView {
            cx.new(|_| Tip(text.clone())).into()
        })
        .tooltip_show_delay(DELAY)
}
