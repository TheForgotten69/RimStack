//! The Mods tab as a whole: its keys, its panes or what stands in for them, and its bars.

use super::{FocusFilter, Mods, Redo, Save, Undo, pane_widths};
use crate::ui::theme::{self, Theme};
use gpui::{
    Context, Focusable, InteractiveElement, IntoElement, ParentElement, Render, Styled, Window,
    div, prelude::FluentBuilder, px,
};

impl Focusable for Mods {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

impl Render for Mods {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let handle = || {
            div()
                .flex()
                .flex_none()
                .justify_center()
                .w(theme::HANDLE)
                .h_full()
                .child(div().w(px(1.)).h_full().bg(theme.colours.edge))
        };
        let (_, _, details_width) =
            pane_widths(&theme, window.viewport_size().width, self.panes_wanted());
        let bulk = self.bulk_bar(&theme, cx);
        let unread = self.unread(&theme, cx);
        let is_unread = unread.is_some();
        self.hint_anchors.forget_drawn();
        let panes =
            (self.comparing.is_none() && !is_unread).then(|| self.panes(&theme, window, cx));
        let hint = match panes.is_some() {
            true => self.hint(&theme, window, cx),
            false => None,
        };
        self.open_wanted_menu(window, cx);
        div()
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .track_focus(&self.focus)
            .on_action(cx.listener(|mods, _: &FocusFilter, window, cx| {
                let field = match mods.drawer_open && mods.view.inactive_hidden {
                    true => &mods.mirror,
                    false => &mods.filter,
                };
                window.focus(&field.focus_handle(cx), cx);
            }))
            .capture_key_down(cx.listener(|mods, event: &gpui::KeyDownEvent, _, cx| {
                if mods.drawer_open && event.keystroke.key == "escape" {
                    mods.set_drawer(false, cx);
                    cx.stop_propagation();
                }
            }))
            .on_action(cx.listener(|mods, _: &Undo, _, cx| mods.undo(cx)))
            .on_action(cx.listener(|mods, _: &Redo, _, cx| mods.redo(cx)))
            .on_action(cx.listener(|mods, _: &Save, _, cx| {
                if crate::ui::app(cx).session().is_dirty() {
                    mods.save(None, cx);
                }
            }))
            .when_some(self.comparing.clone(), |d, view| {
                d.child(
                    div()
                        .flex()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .child(div().flex_1().min_w_0().h_full().child(view))
                        .child(handle())
                        .child(
                            div()
                                .w(details_width)
                                .flex_none()
                                .h_full()
                                .child(self.details_view.clone()),
                        ),
                )
            })
            .when_some(unread, |d, unread| d.child(unread))
            .when(self.comparing.is_none() && !is_unread, |d| {
                d.children(panes)
            })
            .children(bulk)
            .children(hint)
    }
}
