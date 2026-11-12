//! `StatusBar.qml`: what is not saved, the last start of the game, the work under way, the keys
//! and the history.

use super::button::Btn;
use super::lists::Which;
use super::shell::Shell;
use super::theme::{self, Theme};
use super::widgets::{icon, label};
use crate::words::put;
use crate::workspace::logs::moment;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};
use rimstack_core::{say, say_n};

impl Shell {
    /// The work under way across the window, as `Main.qml`'s `tasks`.
    fn tasks(&self, cx: &gpui::App) -> Vec<String> {
        let mut tasks = Vec::new();
        if self.mods.read(cx).reading_files() {
            tasks.push(put(&say!("Reading the active mods' files…")));
        }
        if let Some(textures) = &self.textures
            && let Some(said) = textures.read(cx).task_said()
        {
            tasks.push(said);
        }
        if let Some(downloads) = &self.downloads {
            tasks.extend(downloads.read(cx).tasks_said());
        }
        tasks
    }

    pub(crate) fn status_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.global::<Theme>().clone();
        let c = theme.colours;
        let height = theme.strip() - px(2.);
        let demo = self.demo;
        let unsaved = self.mods.read(cx).unsaved(cx);
        let last = self.mods.read(cx).last_start();
        let tasks = self.tasks(cx);
        let lists = self.lists.clone();
        let tray = (!tasks.is_empty()).then(|| {
            let said = match tasks.len() {
                1 => tasks[0].clone(),
                n => put(&say!("{} tasks running", n)),
            };
            div()
                .flex()
                .items_center()
                .gap(theme::GAP_SMALL)
                .child(icon("refresh-cw", theme.sized(12.), c.moved))
                .child(label(&theme, theme.small(), c.ink, said))
        });
        div()
            .flex()
            .flex_none()
            .items_center()
            .w_full()
            .h(theme.strip())
            .pl(theme::GAP_CARD)
            .pr(theme::GAP_MEDIUM)
            .bg(c.bar)
            .border_t_1()
            .border_color(c.edge)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap(theme::GAP_WIDE)
                    .when(demo, |d| {
                        d.child(label(
                            &theme,
                            theme.small(),
                            c.ink2,
                            put(&say!("Demo data")),
                        ))
                    })
                    .when_some(unsaved, |d, unsaved| {
                        d.child(label(
                            &theme,
                            theme.small(),
                            c.warn,
                            put(&say!("Not saved: {}", unsaved)),
                        ))
                    })
                    .when_some(last, |d, (time, errors)| {
                        let started = put(&say!("Last start of the game {}", moment(time)));
                        let said = errors.map(|n| match n {
                            0 => (put(&say!("no errors")), c.ink2),
                            n => (
                                put(&say_n!(n, "{} different error", "{} different errors")),
                                c.err,
                            ),
                        });
                        let tip = put(&say!("Opens Logs on this launch"));
                        let last = div()
                            .id("last-start")
                            .flex()
                            .items_center()
                            .gap(theme::GAP_SMALL)
                            .h(theme.control())
                            .cursor_pointer()
                            .role(gpui::Role::Button)
                            .aria_label(started.clone())
                            .child(label(&theme, theme.small(), c.ink2, started))
                            .when_some(said, |d, (said, tone)| {
                                d.child(label(&theme, theme.small(), tone, said))
                            })
                            .child(icon("chevron-right", theme.sized(11.), c.ink3))
                            .on_click(cx.listener(|shell, _, _, cx| shell.go(5, cx)));
                        d.child(super::tip::tipped(last, tip.into()))
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(theme::GAP_LARGE)
                    .children(tray)
                    .child(
                        Btn::new("keys", put(&say!("? Keys")))
                            .height(height)
                            .spoken(put(&say!("Keyboard shortcuts (F1)")))
                            .tip(put(&say!("Keyboard shortcuts (F1)")))
                            .on_click(
                                cx.listener(|shell, _, window, cx| shell.open_keys(window, cx)),
                            ),
                    )
                    .when(!demo, |d| {
                        d.child(
                            Btn::new("history", put(&say!("History")))
                                .icon("history")
                                .height(height)
                                .tip(put(&say!("What RimStack changed, and how to take it back")))
                                .pops(move |below, window, cx| {
                                    lists.update(cx, |lists, cx| {
                                        lists.toggle(Which::History, below, window, cx)
                                    })
                                }),
                        )
                    }),
            )
            .into_any_element()
    }
}
