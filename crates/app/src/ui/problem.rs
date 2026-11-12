//! `ProblemLine.qml`: the mod in bold, what is wrong, and its fix, on the line's right when there
//! is room for twenty characters beside it, under it otherwise.

use super::theme::{self, Theme};
use super::widgets::{Btn, TextSized, btn_width, icon, measure};
use crate::words::put;
use crate::workspace::load_order::ProblemSaid;
use gpui::{
    AnyElement, FontWeight, HighlightStyle, IntoElement, ParentElement, Pixels, Styled, StyledText,
    Window, div, prelude::FluentBuilder,
};
use rimstack_core::base::sentence::Sentence;

/// `room`: the line's width and the window, to know whether the fix fits beside the words.
pub fn problem(
    theme: &Theme,
    at: usize,
    said: &ProblemSaid,
    room: Option<(Pixels, &Window)>,
) -> AnyElement {
    let c = &theme.colours;
    let tone = if said.error { c.err } else { c.warn };
    let text = format!("{} {}", said.name, put(&said.words));
    let bold = HighlightStyle {
        font_weight: Some(FontWeight::BOLD),
        ..HighlightStyle::default()
    };
    let fix =
        (!said.fix.action.is_empty()).then(|| put(&Sentence::said(said.fix.label, Vec::new())));
    let stacked = room.is_none_or(|(width, window)| {
        let actions = fix.as_deref().map_or(Pixels::ZERO, |fix| {
            btn_width(window, theme, fix, false, false)
        }) + if said.ignore.is_empty() {
            Pixels::ZERO
        } else {
            theme.icon() + theme.sized(14.) + theme::GAP_SMALL
        };
        let twenty = measure(window, theme, "n", theme.body(), FontWeight::NORMAL) * 20.;
        width - actions - theme.icon() - theme::GAP * 2. < twenty
    });
    let words = div()
        .spaced_text(theme, theme.body(), 1.25)
        .text_color(c.ink)
        .child(StyledText::new(text).with_highlights([(0..said.name.len(), bold)]));
    let actions = div()
        .flex()
        .flex_none()
        .gap(theme::GAP_SMALL)
        .when_some(fix, |d, fix| d.child(Btn::new(("fix", at), fix)))
        .when(!said.ignore.is_empty(), |d| {
            d.child(Btn::new(("ignore", at), "").icon("eye-off"))
        });
    let mark = icon(
        if said.error {
            "circle-alert"
        } else {
            "triangle-alert"
        },
        theme.icon(),
        tone,
    )
    .mt(theme::GAP_HAIR);
    match stacked {
        true => div()
            .flex()
            .gap(theme::GAP)
            .child(mark)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(theme::GAP_TINY)
                    .child(words)
                    .child(actions),
            )
            .into_any_element(),
        false => div()
            .flex()
            .items_start()
            .gap(theme::GAP)
            .child(mark)
            .child(div().flex_1().min_w_0().child(words))
            .child(actions)
            .into_any_element(),
    }
}
