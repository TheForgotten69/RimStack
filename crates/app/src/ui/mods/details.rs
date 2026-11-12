//! `DetailsPane.qml` and `ListSummary.qml`: the list as a whole when no mod is chosen.

use super::Mods;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{Btn, Chip, TextSized, icon, label};
use crate::words::{count, put};
use crate::workspace::load_order::{ProblemSaid, Side};
use gpui::{
    AnyElement, Context, FontWeight, HighlightStyle, Hsla, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, StyledText, div, prelude::FluentBuilder, px,
};
use rimstack_core::base::sentence::Sentence;
use rimstack_core::{say, say_n};

/// `Card.qml`: a titled part of the details, with a note beside its title and an arrow when it folds.
pub(super) struct Card {
    pub title: String,
    pub tone: Hsla,
    pub note: String,
    /// `Some(folded)` for a card that folds.
    pub folds: Option<bool>,
    /// A card with room for actions beside its title has a taller head.
    pub actions: bool,
    pub spacing: gpui::Pixels,
    pub body: Vec<AnyElement>,
}

impl Card {
    pub(super) fn render(self, theme: &Theme) -> gpui::Div {
        let c = &theme.colours;
        let folded = self.folds == Some(true);
        div()
            .flex()
            .flex_col()
            .w_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(theme.sized(if self.actions { 34. } else { 28. }))
                    .gap(theme::GAP)
                    .child(
                        label(theme, theme.small(), self.tone, self.title)
                            .font_weight(theme.semibold),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .mr(theme::GAP_SMALL)
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .sized_text(theme, theme.small())
                            .text_color(c.ink3)
                            .child(self.note),
                    )
                    .when_some(self.folds, |d, folded| {
                        d.child(icon(
                            if folded {
                                "chevron-right"
                            } else {
                                "chevron-down"
                            },
                            theme.icon(),
                            c.ink3,
                        ))
                    }),
            )
            .when(!folded, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(self.spacing)
                        .pb(px(6.))
                        .children(self.body),
                )
            })
    }
}

/// `ListSummary.Line`: words, and a button beside them.
fn line(
    theme: &Theme,
    id: &'static str,
    text: String,
    action: Option<(String, bool)>,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(theme::GAP)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .sized_text(theme, theme.body())
                .text_color(theme.colours.ink)
                .child(text),
        )
        .when_some(action, |d, (said, primary)| {
            d.child(Btn::new(id, said).primary_if(primary))
        })
        .into_any_element()
}

/// `ProblemLine.qml`: the mod in bold, what is wrong, and its fix.
fn problem(theme: &Theme, at: usize, said: &ProblemSaid) -> AnyElement {
    let c = &theme.colours;
    let tone = if said.error { c.err } else { c.warn };
    let text = format!("{} {}", said.name, put(&said.words));
    let bold = HighlightStyle {
        font_weight: Some(FontWeight::BOLD),
        ..HighlightStyle::default()
    };
    let fix =
        (!said.fix.action.is_empty()).then(|| put(&Sentence::said(said.fix.label, Vec::new())));
    div()
        .flex()
        .gap(theme::GAP)
        .child(
            icon(
                if said.error {
                    "circle-alert"
                } else {
                    "triangle-alert"
                },
                theme.icon(),
                tone,
            )
            .mt(theme::GAP_HAIR),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(theme::GAP_TINY)
                .child(
                    // Qt's `lineHeight: 1.25` puts the added room under each line, not around it.
                    div()
                        .mt(-(theme.line(theme.body()) * 0.125))
                        .mb(theme.line(theme.body()) * 0.125)
                        .text_size(theme.body())
                        .line_height(theme.line(theme.body()) * 1.25)
                        .text_color(c.ink)
                        .child(StyledText::new(text).with_highlights([(0..said.name.len(), bold)])),
                )
                .child(
                    div()
                        .flex()
                        .gap(theme::GAP_SMALL)
                        .when_some(fix, |d, fix| d.child(Btn::new(("fix", at), fix)))
                        .when(!said.ignore.is_empty(), |d| {
                            d.child(Btn::new(("ignore", at), "").icon("eye-off"))
                        }),
                ),
        )
        .into_any_element()
}

impl Mods {
    pub(super) fn details_pane(
        &self,
        theme: &Theme,
        width: gpui::Pixels,
        window: &gpui::Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let c = &theme.colours;
        // `Scrolled.qml` keeps its bar's width free at the right.
        let bar = theme.sized(9.) + theme::GAP_HAIR * 2.;
        let scrolled = div()
            .id("details")
            .flex()
            .flex_col()
            .h_full()
            .bg(c.panel)
            .p(theme::GAP_CARD)
            .pr(theme::GAP_CARD + bar)
            .overflow_y_scroll();
        if let Some(details) = &self.details {
            return scrolled.child(self.chosen_pane(theme, details, width, window, cx));
        }
        let title = match self.demo {
            true => put(&say!("Demo data")),
            false => put(&say!("Load order")),
        };
        let head = div()
            .flex()
            .flex_col()
            .child(label(theme, theme.heading(), c.ink, title).font_weight(FontWeight::BOLD))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .pt(theme::GAP_SMALL)
                    .gap(theme::GAP_TINY)
                    .when(!self.demo, |d| {
                        d.child(Chip::new(put(&say!("RimWorld {}", self.game_version()))))
                    })
                    .child(Chip::new(put(&say!(
                        "{} active",
                        count(self.count(Side::Active))
                    ))))
                    .child(Chip::new(put(&say!(
                        "{} inactive",
                        count(self.count(Side::Inactive))
                    )))),
            );
        scrolled
            .gap(theme::GAP_CARD)
            .child(head)
            .when(!self.demo, |d| {
                d.children(self.verdict(theme, cx))
                    .children(self.kept(theme))
            })
    }

    fn verdict(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<gpui::Div> {
        let c = &theme.colours;
        let overview = self.overview.as_ref()?;
        let (_, warnings) = self.order.errors_and_warnings();
        let dirty = crate::ui::app(cx).session().is_dirty();
        let errors = overview.error_count;
        let playable = errors == 0 && !dirty;
        let title = match (playable, warnings) {
            (false, _) => put(&say!("Before you play")),
            (true, 0) => put(&say!("Ready to play")),
            (true, n) => put(&say_n!(n, "Playable, {} warning", "Playable, {} warnings")),
        };
        let tone = if errors > 0 {
            c.err
        } else if dirty {
            c.warn
        } else if warnings > 0 {
            c.ink
        } else {
            c.ok
        };
        let mut body: Vec<AnyElement> = overview
            .first_errors
            .iter()
            .enumerate()
            .map(|(at, said)| problem(theme, at, said))
            .collect();
        let troubled = self.order.troubles().iter().any(|t| !t.is_empty());
        if troubled {
            let text = if overview.more_errors > 0 {
                put(&say_n!(
                    overview.more_errors,
                    "{} more problem",
                    "{} more problems"
                ))
            } else if warnings == 0 {
                String::new()
            } else if overview.version_only == warnings {
                put(&say!("All for the game version"))
            } else {
                put(&say_n!(
                    warnings,
                    "{} mod with a warning",
                    "{} mods with a warning"
                ))
            };
            body.push(line(
                theme,
                "all-problems",
                text,
                Some((put(&say!("All problems")), false)),
            ));
        }
        if let Some(start) = &self.last_start {
            let logged = start.start.errors.unwrap_or(0) > 0;
            body.push(
                div()
                    .flex()
                    .items_center()
                    .gap(theme::GAP)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .sized_text(theme, theme.body())
                            .text_color(c.ink2)
                            .child(put(&start.line)),
                    )
                    .when(logged, |d| {
                        d.child(Btn::new("open-log", put(&say!("Open log"))))
                    })
                    .into_any_element(),
            );
        }
        if dirty {
            let unsaved = crate::ui::app(cx).session().changes().said();
            let said = put(&say!("Not saved: {}", put(&unsaved)));
            body.push(line(
                theme,
                "save-list",
                said,
                Some((put(&say!("Save")), true)),
            ));
        }
        if errors == 0 && !dirty && !troubled {
            body.push(line(
                theme,
                "saved",
                put(&say!("No problems, and the list is saved.")),
                None,
            ));
        }
        let note = if errors > 0 {
            count(errors)
        } else {
            String::new()
        };
        Some(
            Card {
                title,
                tone,
                note,
                folds: None,
                actions: false,
                spacing: theme::GAP,
                body,
            }
            .render(theme),
        )
    }

    fn kept(&self, theme: &Theme) -> Option<gpui::Div> {
        let summary = self.summary.as_ref()?;
        Some(
            Card {
                title: put(&say!("Kept")),
                tone: theme.colours.ink2,
                note: put(&summary.kept_note),
                folds: Some(true),
                actions: false,
                spacing: theme::GAP_SMALL,
                body: Vec::new(),
            }
            .render(theme),
        )
    }
}
