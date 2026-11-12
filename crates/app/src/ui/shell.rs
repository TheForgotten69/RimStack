//! `Main.qml`: the top bar, the section shown, the status bar.

use super::Options;
use super::mods::Mods;
use super::status_bar::status_bar;
use super::theme::Theme;
use super::top_bar::top_bar;
use crate::workspace::load_order::Side;
use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div,
    prelude::FluentBuilder,
};

pub struct Shell {
    section: usize,
    demo: bool,
    mods: Entity<Mods>,
}

impl Shell {
    pub fn new(options: &Options, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let demo = options.demo;
        let mods = cx.new(|cx| Mods::new(demo, cx));
        if options.several {
            mods.update(cx, |mods, cx| {
                mods.choose_nth(Side::Active, 14, false, cx);
                for at in [15, 16, 19] {
                    mods.choose_nth(Side::Active, at, true, cx);
                }
            });
        }
        Self {
            section: options.section,
            demo,
            mods,
        }
    }

    pub fn go(&mut self, section: usize, cx: &mut Context<Self>) {
        self.section = section;
        cx.notify();
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (in_the_way, last_start) = {
            let mods = self.mods.read(cx);
            (mods.in_the_way(cx), mods.last_start())
        };
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
            .child(top_bar(self.section, self.demo, in_the_way, window, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .when(self.section == 0, |d| d.child(self.mods.clone())),
            )
            .child(status_bar(self.demo, last_start, cx))
    }
}
