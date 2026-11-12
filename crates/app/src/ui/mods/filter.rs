//! `ModsFilter.qml` and `FilterField.qml` with its chips: the filter of the Mods tab, its terms as
//! chips inside it, and the terms offered while one is typed.

use super::Mods;
use crate::ui::button::Chip;
use crate::ui::text_field::{self, TextField};
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{TextSized, halo, icon, label, measure_in};
use crate::words::put;
use crate::workspace::load_order::Side;
use gpui::{
    Context, Entity, Focusable, FontWeight, InteractiveElement, ParentElement, Pixels,
    StatefulInteractiveElement, Styled, Window, anchored, deferred, div, prelude::FluentBuilder,
    px,
};
use rimstack_core::mods::list::flag;
use rimstack_core::mods::list::offers::Counted;
use rimstack_core::say;

/// A word the filter shows as a chip: `is:new`, `!tag:hud`.
fn is_term(word: &str) -> bool {
    let word = word.strip_prefix('!').unwrap_or(word);
    ["is:", "has:", "in:", "tag:", "author:", "patches:"]
        .iter()
        .any(|kind| word.len() > kind.len() && word.starts_with(kind))
}

impl Mods {
    /// The whole filter: the terms, then what is typed.
    pub fn filter_text(&self, cx: &gpui::App) -> String {
        let typed = self.filter.read(cx).text().trim().to_string();
        let mut words = self.filter_terms.clone();
        if !typed.is_empty() {
            words.push(typed);
        }
        words.join(" ")
    }

    pub fn filter_by(&mut self, text: &str, cx: &mut Context<Self>) {
        let words: Vec<&str> = text.split(' ').filter(|w| !w.is_empty()).collect();
        self.filter_terms = words
            .iter()
            .filter(|w| is_term(w))
            .map(|w| w.to_string())
            .collect();
        let typed: Vec<&str> = words.into_iter().filter(|w| !is_term(w)).collect();
        self.filter
            .update(cx, |field, cx| field.set_text(&typed.join(" "), cx));
    }

    /// A field typed in: each finished term becomes a chip, and the list follows the filter.
    pub(super) fn filter_edited(&mut self, field: &Entity<TextField>, cx: &mut Context<Self>) {
        let text = field.read(cx).text().to_string();
        let words: Vec<&str> = text.split(' ').collect();
        let last = words.len().saturating_sub(1);
        let done: Vec<String> = words[..last]
            .iter()
            .filter(|w| is_term(w))
            .map(|w| w.to_string())
            .collect();
        let rest = match done.is_empty() {
            true => text.clone(),
            false => {
                let kept: Vec<&str> = words
                    .iter()
                    .enumerate()
                    .filter(|(at, w)| *at == last || !is_term(w))
                    .map(|(_, w)| *w)
                    .collect();
                kept.join(" ")
            }
        };
        self.filter_terms.extend(done);
        for other in [self.filter.clone(), self.mirror.clone()] {
            if other.read(cx).text() != rest {
                other.update(cx, |field, cx| field.set_text_quietly(&rest, cx));
            }
        }
        self.offers_shut = false;
        let composed = self.filter_text(cx);
        self.order.set_filter(&composed);
        self.show(cx);
        self.keep_left_filter(composed, cx);
    }

    pub(super) fn drop_term(&mut self, at: usize, cx: &mut Context<Self>) {
        if at < self.filter_terms.len() {
            self.filter_terms.remove(at);
        }
        let field = self.filter.clone();
        self.filter_edited(&field, cx);
    }

    /// An offer taken: a kind of term (`tag:`) goes on being typed, a whole term becomes a chip.
    fn take_offer(&mut self, term: String, field: Entity<TextField>, cx: &mut Context<Self>) {
        let text = field.read(cx).text().to_string();
        let mut words: Vec<&str> = text.split(' ').collect();
        words.pop();
        if term.ends_with(':') {
            words.push(&term);
            let typed = words.join(" ");
            field.update(cx, |field, cx| field.set_text(&typed, cx));
            return;
        }
        if !self.filter_terms.contains(&term) {
            self.filter_terms.push(term.clone());
        }
        let typed = words.join(" ");
        field.update(cx, |field, cx| field.set_text(&typed, cx));
    }

    pub(super) fn clear_filter(&mut self, cx: &mut Context<Self>) {
        self.filter_terms.clear();
        self.offers_shut = true;
        self.filter.update(cx, |field, cx| field.set_text("", cx));
    }

    fn offers(
        &self,
        field: &Entity<TextField>,
        cx: &gpui::App,
    ) -> rimstack_core::mods::list::offers::Offers {
        let typed = field.read(cx).text().to_string();
        let partial = typed.rsplit(' ').next().unwrap_or_default().to_string();
        let (errors, warnings) = self.order.errors_and_warnings();
        let counted = Counted {
            errors,
            warnings,
            conflicts: self.conflicts,
            updated: self.order.list(Side::Active).flagged(flag::UPDATED),
        };
        self.order
            .filter_offers(&partial, &self.filter_text(cx), &counted)
    }

    /// The filter as `ModsFilter` drew it, `room` wide: chips, what is typed, what it found, and
    /// the terms offered under it while it has the keys.
    pub(super) fn filter_field(
        &self,
        field: &Entity<TextField>,
        id: &'static str,
        room: Pixels,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let c = &theme.colours;
        let typed = field.read(cx).text().to_string();
        let focused = field.read(cx).is_focused(window);
        let focus = field.focus_handle(cx);
        let empty = typed.is_empty() && self.filter_terms.is_empty();
        let found = (!empty).then(|| put(&self.order.filter_summary()));
        let keys = put(&say!("Ctrl + F"));
        let placeholder = put(&say!("Filter active and inactive mods"));
        let keys_fit = room
            > theme.icon()
                + theme::GAP
                + crate::ui::widgets::measure(
                    window,
                    theme,
                    &placeholder,
                    theme.body(),
                    FontWeight::NORMAL,
                )
                + crate::ui::widgets::measure(
                    window,
                    theme,
                    &keys,
                    theme.caption(),
                    FontWeight::NORMAL,
                )
                + theme::GAP * 4.;
        let chips = self.filter_terms.iter().enumerate().map(|(at, term)| {
            Chip::new(term.clone())
                .tone(c.moved)
                .icon("x")
                .tip(put(&say!("Remove")))
                .on_click(
                    ("filter-term", at),
                    cx.listener(move |mods, _, window, cx| {
                        mods.drop_term(at, cx);
                        let focus = mods.filter.focus_handle(cx);
                        window.focus(&focus, cx);
                    }),
                )
        });
        let offered = match focused && !self.offers_shut {
            true => self.offers(field, cx),
            false => Default::default(),
        };
        let suggestions = (!offered.offers.is_empty()).then(|| {
            let wide = room.min(theme.sized(420.));
            let key_room = measure_in(
                window,
                theme,
                theme.mono.clone(),
                &offered.widest,
                theme.small(),
                FontWeight::NORMAL,
            )
            .ceil()
            .min(wide / 2.);
            let held = field.clone();
            let rows = offered.offers.into_iter().enumerate().map(|(at, offer)| {
                let (term, about) = (offer.term.clone(), put(&offer.about));
                let held = held.clone();
                div()
                    .id(("filter-offer", at))
                    .role(gpui::Role::Button)
                    .aria_label(put(&say!("Filter by {}", term.clone())))
                    .flex()
                    .items_center()
                    .gap(theme.sized(16.))
                    .h(theme.sized(26.))
                    .px(theme::GAP)
                    .rounded(theme::CORNER)
                    .cursor_pointer()
                    .hover(|s| s.bg(c.chip))
                    .child(
                        div()
                            .flex_none()
                            .w(key_room)
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .font_family(theme.mono.clone())
                            .sized_text(theme, theme.small())
                            .text_color(c.ink)
                            .child(term.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .sized_text(theme, theme.small())
                            .text_color(c.ink2)
                            .child(about),
                    )
                    .on_click(cx.listener(move |mods, _, window, cx| {
                        mods.take_offer(term.clone(), held.clone(), cx);
                        let focus = held.focus_handle(cx);
                        window.focus(&focus, cx);
                    }))
            });
            div().absolute().top_full().left_0().child(deferred(
                anchored().child(
                    div()
                        .id("filter-offers")
                        .mt(px(4.))
                        .w(wide)
                        .p(px(6.))
                        .rounded(theme::CORNER_LARGE)
                        .bg(c.bar)
                        .border_1()
                        .border_color(c.line)
                        .occlude()
                        .on_mouse_down_out(cx.listener(|mods, _, _, cx| {
                            mods.offers_shut = true;
                            cx.notify();
                        }))
                        .children(rows)
                        .child(
                            div()
                                .px(theme::GAP)
                                .pt(theme::GAP_SMALL)
                                .pb(theme::GAP_TINY)
                                .sized_text(theme, theme.small())
                                .text_color(c.ink3)
                                .child(put(&say!("Use ! to exclude, as in !tag:hud"))),
                        ),
                ),
            ))
        });
        let held = field.clone();
        div()
            .id(id)
            .relative()
            .flex()
            .items_center()
            .h(theme.field())
            .pl(theme::GAP_SMALL)
            .pr(theme::GAP)
            .gap(theme::GAP)
            .rounded(theme::CORNER)
            .bg(c.well)
            .border_1()
            .border_color(if focused { c.focus } else { c.field_line })
            .when(focused, |d| d.children(halo(c.focus, theme::CORNER)))
            .cursor_text()
            .capture_action(cx.listener(|mods, _: &text_field::Escape, _, cx| {
                mods.clear_filter(cx);
                cx.stop_propagation();
            }))
            .capture_action(cx.listener(move |mods, _: &text_field::Backspace, _, cx| {
                match held.read(cx).text().is_empty() && !mods.filter_terms.is_empty() {
                    true => {
                        mods.drop_term(mods.filter_terms.len() - 1, cx);
                        cx.stop_propagation();
                    }
                    false => cx.propagate(),
                }
            }))
            .on_click(cx.listener(move |mods, _, window, cx| {
                mods.offers_shut = false;
                window.focus(&focus, cx);
            }))
            .child(icon("funnel", theme.icon() - px(1.), c.ink3))
            .when(!self.filter_terms.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_none()
                        .gap(theme::GAP_TINY)
                        .mr(theme::GAP_LARGE - theme::GAP)
                        .children(chips),
                )
            })
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .when(empty, |d| {
                        d.child(
                            div()
                                .absolute()
                                .inset_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .sized_text(theme, theme.body())
                                .text_color(c.ink3)
                                .child(placeholder),
                        )
                    })
                    .child(field.clone()),
            )
            .when_some(found, |d, found| {
                d.child(label(theme, theme.small(), c.ink2, found))
            })
            .when(empty && !focused && keys_fit, |d| {
                d.child(label(theme, theme.caption(), c.ink3, keys))
            })
            .when(!empty, |d| {
                d.child(crate::ui::tip::tipped(
                    div()
                        .id((id, 1usize))
                        .role(gpui::Role::Button)
                        .aria_label(put(&say!("Clear (Esc)")))
                        .cursor_pointer()
                        .child(icon("x", theme.sized(12.), c.ink3))
                        .on_click(cx.listener(|mods, _, window, cx| {
                            mods.clear_filter(cx);
                            let focus = mods.filter.focus_handle(cx);
                            window.focus(&focus, cx);
                        })),
                    put(&say!("Clear (Esc)")).into(),
                ))
            })
            .children(suggestions)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_term_is_a_kind_and_something_after_it() {
        for word in ["is:new", "!tag:hud", "author:someone", "patches:x"] {
            assert!(super::is_term(word), "{word}");
        }
        for word in ["is:", "hud", "tag", "!is:"] {
            assert!(!super::is_term(word), "{word}");
        }
    }
}
