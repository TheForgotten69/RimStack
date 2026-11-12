//! Play as `Main.qml` pressed it, and the `beforePlay` popup of `TopBar.qml`: what stands in the
//! way, and what can be done about it before the game starts.

use super::button::Btn;
use super::game::Told;
use super::notices;
use super::shell::Shell;
use super::theme::{self, Theme};
use super::widgets::{TextSized, label};
use crate::words::put;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, div,
    prelude::FluentBuilder, px,
};
use rimstack_core::{say, say_n};

impl Shell {
    /// Play: the game starts, unless something is in the way; then the popup says what.
    pub(crate) fn press_play(&mut self, cx: &mut Context<Self>) {
        if self.game.read(cx).running() {
            return;
        }
        let ways = self.mods.read(cx).in_the_way_said(cx);
        match ways.is_empty() {
            true => self.play(cx),
            false => {
                self.before_play = !self.before_play;
                cx.notify();
            }
        }
    }

    fn play(&mut self, cx: &mut Context<Self>) {
        self.before_play = false;
        self.game.update(cx, |game, cx| game.play(false, cx));
        cx.notify();
    }

    /// What the game and its files told: a toast, with what can be done about it.
    pub(crate) fn heard_game(&mut self, told: &Told, cx: &mut Context<Self>) {
        match told {
            Told::ResetByGame(ids) => {
                let said = say_n!(
                    ids.len(),
                    "The game emptied its list of mods, as it does on a new version. The last launch from RimStack had {} mod.",
                    "The game emptied its list of mods, as it does on a new version. The last launch from RimStack had {} mods."
                );
                let (ids, mods) = (ids.clone(), self.mods.downgrade());
                notices::offer(
                    put(&said),
                    put(&say!("Use that list")),
                    move |_, cx| {
                        let ids = ids.clone();
                        let _ = mods.update(cx, |mods, cx| mods.use_list(ids, cx));
                    },
                    cx,
                );
            }
            Told::WrittenOutside(ids) => {
                let here = self
                    .mods
                    .read(cx)
                    .count(crate::workspace::load_order::Side::Active);
                let said = say_n!(
                    ids.len(),
                    "ModsConfig.xml was written outside RimStack and now lists {} mod; the list here has {}.",
                    "ModsConfig.xml was written outside RimStack and now lists {} mods; the list here has {}.",
                    here
                );
                let mods = self.mods.downgrade();
                notices::offer(
                    put(&said),
                    put(&say!("Use the file's")),
                    move |_, cx| {
                        let _ = mods.update(cx, |mods, cx| mods.keep_file(true, cx));
                    },
                    cx,
                );
            }
            Told::NotStarted(why) => notices::say(why.clone(), notices::Tone::Err, cx),
        }
    }

    /// The popup under Play while it is open.
    pub(crate) fn before_play_popup(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.before_play {
            return None;
        }
        let theme = cx.global::<Theme>().clone();
        let c = theme.colours;
        let mods = self.mods.read(cx);
        let ways = mods.in_the_way_said(cx);
        let (errors, sort_pending, dirty) = mods.what_is_in_the_way(cx);
        let demo = self.demo;
        let lines = ways.into_iter().map(|way| {
            div()
                .flex()
                .items_start()
                .gap(theme::GAP)
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .h(theme.line(theme.body()))
                        .child(div().size(px(7.)).rounded(px(3.5)).bg(c.warn)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .sized_text(&theme, theme.body())
                        .text_color(c.ink2)
                        .child(way),
                )
        });
        let popup = div()
            .id("before-play")
            .absolute()
            .top(theme.top_bar() - theme::GAP_SMALL)
            .right(theme::GAP_CARD)
            .w(theme.sized(320.))
            .flex()
            .flex_col()
            .gap(theme::GAP)
            .p(theme::GAP_LARGE)
            .rounded(theme::CORNER_LARGE)
            .bg(c.bar)
            .border_1()
            .border_color(c.line)
            .occlude()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                label(&theme, theme.title(), c.ink, put(&say!("Before you play")))
                    .font_weight(theme.semibold),
            )
            .children(lines)
            .when(dirty, |d| {
                d.child(
                    Btn::new("save-and-play", put(&say!("Save and play")))
                        .primary()
                        .fill()
                        .on_click(cx.listener(|shell, _, _, cx| {
                            shell.before_play = false;
                            let game = shell.game.downgrade();
                            shell.mods.update(cx, |mods, cx| {
                                mods.save(
                                    Some(Box::new(move |_, cx| {
                                        let _ = game.update(cx, |game, cx| game.play(false, cx));
                                    })),
                                    cx,
                                )
                            });
                            cx.notify();
                        })),
                )
            })
            .when(sort_pending && !demo, |d| {
                d.child(
                    Btn::new("sort-first", put(&say!("Sort first")))
                        .icon("arrow-up-down")
                        .fill()
                        .tip(put(&say!("Shows the order the rules ask for, to keep or discard. Nothing is written before Save.")))
                        .on_click(cx.listener(|shell, _, _, cx| {
                            shell.before_play = false;
                            shell.go(0, cx);
                            shell.mods.update(cx, |mods, cx| mods.preview_sort(cx));
                        })),
                )
            })
            .when(errors, |d| {
                d.child(
                    Btn::new("show-problems", put(&say!("Show the problems")))
                        .fill()
                        .on_click(cx.listener(|shell, _, _, cx| {
                            shell.before_play = false;
                            shell.go(0, cx);
                            shell.mods.update(cx, |mods, cx| mods.open_problems(cx));
                        })),
                )
            })
            .when(!demo && !self.hunt.active, |d| {
                d.child(
                    Btn::new("find-broken-play", put(&say!("Find the broken mod")))
                        .icon("search")
                        .fill()
                        .on_click(cx.listener(|shell, _, _, cx| shell.start_hunt(cx))),
                )
            })
            .child(
                Btn::new(
                    "play-anyway",
                    match dirty {
                        true => put(&say!("Play with the saved list")),
                        false => put(&say!("Play anyway")),
                    },
                )
                .fill()
                .on_click(cx.listener(|shell, _, _, cx| shell.play(cx))),
            );
        Some(
            div()
                .id("before-play-scrim")
                .absolute()
                .inset_0()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|shell, _, _, cx| {
                        shell.before_play = false;
                        cx.notify();
                    }),
                )
                .child(popup)
                .into_any_element(),
        )
    }
}
