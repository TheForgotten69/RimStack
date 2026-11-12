//! `ChangedCard.qml`: the mods changed since the game last started, by day, with their notes.

use super::screen::text_fill;
use super::{Downloads, ShowMod};
use crate::ui::card::Card;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{Btn, Chip, TextSized, btn_width, icon, label, measure};
use crate::words::{count, put};
use crate::workspace::changed::{ChangedShown, Only, since_shown};
use crate::workspace::logs::{clock, day, moment};
use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Pixels,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder,
};
use rimstack_core::{say, say_n};

/// The changed mods shown before "more".
const FIRST: usize = 5;

impl Downloads {
    fn changed_row(
        &self,
        theme: &Theme,
        at: usize,
        shown: &ChangedShown,
        today: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colours;
        let row = shown.row;
        let mod_id = row.mod_id.to_qml();
        let now = row.now.unwrap_or_default();
        let when = match shown.same_day {
            true => format!("{} → {}", clock(row.was), clock(now)),
            false => {
                let to = if day(now) == today {
                    put(&say!("today"))
                } else {
                    day(now)
                };
                format!("{} → {to}", day(row.was))
            }
        };
        let notes = format!(
            "https://steamcommunity.com/sharedfiles/filedetails/changelog/{}",
            row.id
        );
        let line = div()
            .flex()
            .items_center()
            .gap(theme::GAP)
            .child(icon(
                if row.git { "git-branch" } else { "cloud" },
                theme.icon(),
                c.ink3,
            ))
            .child(
                text_fill(theme, theme.body(), c.ink, row.name.clone()).font_weight(theme.medium),
            )
            .when(!row.active, |d| d.child(Chip::new(put(&say!("inactive")))))
            .when(row.git, |d| {
                d.child(
                    Chip::new(put(&say_n!(row.notes.len(), "{} commit", "{} commits")))
                        .tone(c.moved),
                )
            })
            .when(!row.git, |d| {
                d.child(label(theme, theme.small(), c.ink3, when))
            })
            .child(
                Btn::new(("changed-show", at), put(&say!("Show")))
                    .on_click(cx.listener(move |_, _, _, cx| cx.emit(ShowMod(mod_id)))),
            )
            .when(!row.git, |d| {
                d.child(
                    Btn::new(("changed-notes", at), "")
                        .icon("external-link")
                        .on_click(move |_, _, cx| cx.open_url(&notes)),
                )
            });
        let said = shown.said.iter().map(|note| {
            let text = match note.text.is_empty() {
                true => put(&say!("No note written")),
                false => note.text.clone(),
            };
            let colour = if note.time > 0 { c.ink2 } else { c.err };
            div()
                .flex()
                .gap(theme::GAP)
                .child(div().w(theme.sized(22.)).flex_none())
                .child(
                    label(
                        theme,
                        theme.small(),
                        c.ink3,
                        if note.time > 0 {
                            day(note.time)
                        } else {
                            String::new()
                        },
                    )
                    .w(theme.sized(74.)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .line_clamp(12)
                        .spaced_text(theme, theme.small(), 1.25)
                        .text_color(colour)
                        .child(text),
                )
        });
        div()
            .flex()
            .flex_col()
            .gap(theme::GAP_TINY)
            .when(shown.opens_day, |d| {
                let head = format!(
                    "{}  {}",
                    day(now),
                    put(&say_n!(shown.day_count, "{} mod", "{} mods"))
                );
                d.child(
                    label(theme, theme.small(), c.ink2, head)
                        .font_weight(theme.semibold)
                        .when(at > 0, |d| d.pt(theme::GAP))
                        .pb(theme::GAP_THIN),
                )
            })
            .child(line)
            .children(said)
            .into_any_element()
    }

    pub(super) fn changed_card(
        &self,
        theme: &Theme,
        width: Pixels,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let c = &theme.colours;
        let since = since_shown(&self.changed, self.since_only, &self.fetched);
        if since.count == 0 {
            return None;
        }
        let today = day(crate::workspace::now_secs());
        let most = if self.since_open {
            since.rows.len()
        } else {
            FIRST
        };
        let rows: Vec<AnyElement> = since
            .rows
            .iter()
            .take(most)
            .enumerate()
            .map(|(at, shown)| self.changed_row(theme, at, shown, &today, cx))
            .collect();
        let mut body = vec![
            div()
                .flex()
                .flex_col()
                .gap(theme::GAP_LARGE)
                .children(rows)
                .into_any_element(),
        ];
        if since.rows.len() > FIRST {
            let more = match self.since_open {
                true => put(&say!("Show {}", FIRST)),
                false => put(&say!("{} more", since.rows.len() - FIRST)),
            };
            body.push(
                div()
                    .flex()
                    .child(
                        Btn::new("changed-more", more).on_click(cx.listener(|d, _, _, cx| {
                            d.since_open = !d.since_open;
                            cx.notify();
                        })),
                    )
                    .into_any_element(),
            );
        }
        let mut actions: Vec<AnyElement> = Vec::new();
        let mut actions_width = Pixels::ZERO;
        if since.active > 0 && since.active < since.count {
            let names = [
                put(&say!("All")),
                put(&say!("Active {}", since.active)),
                put(&say!("Inactive {}", since.count - since.active)),
            ];
            let picked = match self.since_only {
                Only::All => 0,
                Only::Active => 1,
                Only::Inactive => 2,
            };
            let wide = |name: &String| {
                measure(window, theme, name, theme.small(), theme.semibold) + theme::GAP_LARGE * 2.
            };
            actions_width += names
                .iter()
                .map(wide)
                .fold(Pixels::ZERO, |all, one| all + one)
                + gpui::px(4.);
            let segments = names.into_iter().enumerate().map(|(at, name)| {
                let on = at == picked;
                div()
                    .id(("changed-only", at))
                    .flex()
                    .items_center()
                    .justify_center()
                    .px(theme::GAP_LARGE)
                    .h(theme.segment())
                    .rounded(theme::CORNER - gpui::px(1.))
                    .when(on, |d| d.bg(c.ctl_hi))
                    .child(
                        label(theme, theme.small(), if on { c.ink } else { c.ink2 }, name)
                            .font_weight(if on {
                                theme.semibold
                            } else {
                                FontWeight::NORMAL
                            }),
                    )
                    .on_click(cx.listener(move |d, _, _, cx| {
                        d.since_only = [Only::All, Only::Active, Only::Inactive][at];
                        cx.notify();
                    }))
            });
            actions.push(
                div()
                    .flex()
                    .p(gpui::px(2.))
                    .rounded(theme::CORNER)
                    .bg(c.seg)
                    .children(segments)
                    .into_any_element(),
            );
        }
        if since.workshop > 0 {
            let ask = self.ask_steam_label();
            actions_width += btn_width(window, theme, &ask, true, false) + theme::GAP_SMALL;
            actions.push(
                Btn::new("fetch-notes", ask)
                    .icon("download")
                    .enabled(!self.notes_busy)
                    .on_click(cx.listener(|d, _, _, cx| d.fetch_since_notes(cx)))
                    .into_any_element(),
            );
        }
        let note = put(&say!(
            "{}, since {}",
            count(since.count),
            moment(since.time)
        ));
        let fold = cx.listener(|d, _, _, cx| d.fold("downloads-changed", cx));
        let card = Card::new(
            "downloads-changed",
            put(&say!("Changed since the game last started")),
        )
        .note(note)
        .folds(self.is_folded("downloads-changed", false), fold)
        .actions(actions, actions_width)
        .spacing(theme::GAP_LARGE)
        .body(body);
        Some(card.render(theme, Some((width, window))).into_any_element())
    }

    /// `AskSteam.qml`: what the notes button says while Steam is asked, or waited for.
    fn ask_steam_label(&self) -> String {
        let Some(state) = self.notes_state.as_ref().filter(|_| self.notes_busy) else {
            return match self.notes_busy {
                true => put(&say!("Asking Steam…")),
                false => put(&say!("Fetch change notes")),
            };
        };
        let waits = state.resume.saturating_sub(crate::workspace::now_secs());
        if waits > 0 {
            put(&say!("Steam asks to wait, {} s", waits))
        } else if state.asked > 1 {
            put(&say!(
                "Asking Steam, {} of {}",
                (state.done + 1).min(state.asked),
                state.asked
            ))
        } else {
            put(&say!("Asking Steam…"))
        }
    }
}
