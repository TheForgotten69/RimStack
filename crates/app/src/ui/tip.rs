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

/// `control` with its tip; an empty one shows none.
pub fn tipped(control: Stateful<gpui::Div>, text: SharedString) -> Stateful<gpui::Div> {
    if text.is_empty() {
        return control;
    }
    control
        .tooltip(move |_: &mut Window, cx: &mut App| -> AnyView {
            cx.new(|_| Tip(text.clone())).into()
        })
        .tooltip_show_delay(DELAY)
}
