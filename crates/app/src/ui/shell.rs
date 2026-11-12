//! `Main.qml`: the top bar, the section shown, the status bar.

use super::Options;
use super::status_bar::status_bar;
use super::theme::Theme;
use super::top_bar::top_bar;
use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div};

pub struct Shell {
    section: usize,
    demo: bool,
}

impl Shell {
    pub fn new(options: &Options, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            section: options.section,
            demo: options.demo,
        }
    }

    pub fn go(&mut self, section: usize, cx: &mut Context<Self>) {
        self.section = section;
        cx.notify();
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let (win, ink, family, body) = (
            theme.colours.win,
            theme.colours.ink,
            theme.family.clone(),
            theme.body(),
        );
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(win)
            .text_color(ink)
            .font_family(family)
            .text_size(body)
            .child(top_bar(self.section, self.demo, cx))
            .child(div().flex_1())
            .child(status_bar(self.demo, cx))
    }
}
