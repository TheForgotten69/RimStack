use super::Options;
use super::theme::Theme;
use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div};

pub struct Shell {
    section: usize,
}

impl Shell {
    pub fn new(options: &Options, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            section: options.section,
        }
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        div()
            .size_full()
            .bg(theme.colours.win)
            .text_color(theme.colours.ink)
            .font_family(theme.family.clone())
            .text_size(theme.body())
            .child(format!("section {}", self.section))
    }
}
