//! `AddMods.qml`: what a look-up found, the download list, and SteamCMD; beside the page when
//! the window is wide enough, at its top otherwise.

use super::Downloads;
use super::screen::text_fill;
use crate::ui::card::Card;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{Btn, Chip, TextSized, chip_width, icon, label};
use crate::words::put;
use gpui::{
    AnyElement, Context, IntoElement, ParentElement, Pixels, Styled, Window, div,
    prelude::FluentBuilder,
};
use rimstack_core::sources::downloads::Asked;
use rimstack_core::{say, say_n};

impl Downloads {
    /// The download list as last read.
    pub(crate) fn listed(&self) -> &crate::workspace::download_list::ListShown {
        &self.list
    }

    fn asked_row(&self, theme: &Theme, at: usize, asked: &Asked) -> AnyElement {
        let c = &theme.colours;
        let megabytes = asked.megabytes.map(|n| match n < 10. {
            true => put(&say!("{} MB", format!("{n:.1}"))),
            false => put(&say!("{} MB", crate::words::count(n.round() as usize))),
        });
        let page = crate::ui::workshop_page(&asked.id);
        div()
            .flex()
            .items_center()
            .gap(theme::GAP)
            .child(text_fill(
                theme,
                theme.body(),
                c.ink,
                asked.title.clone().unwrap_or_else(|| asked.id.clone()),
            ))
            .when_some(megabytes, |d, said| {
                d.child(label(theme, theme.small(), c.ink3, said))
            })
            .when(asked.title.is_none(), |d| {
                d.child(
                    Chip::new(put(&say!("Not on the Workshop")))
                        .tone(c.err)
                        .icon("circle-alert"),
                )
            })
            .when(asked.title.is_some() && asked.installed, |d| {
                d.child(Chip::new(put(&say!("Installed"))).tone(c.ok).icon("check"))
            })
            .when(asked.title.is_some() && !asked.installed, |d| {
                d.child(
                    Btn::new(("asked-page", at), "")
                        .tip(put(&say!("Workshop page")))
                        .spoken(put(&say!(
                            "Show {}",
                            asked.title.clone().unwrap_or_default()
                        )))
                        .icon("external-link")
                        .on_click(move |_, _, cx| cx.open_url(&page)),
                )
            })
            .into_any_element()
    }

    fn looked_up_card(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let c = &theme.colours;
        let (queue, problem) = match &self.looked_up {
            Ok(queue) => (Some(queue), None),
            Err(problem) => (None, Some(put(problem.sentence()))),
        };
        let empty = Default::default();
        let queue = queue.unwrap_or(&empty);
        let note = queue.note.as_ref().map(put);
        if queue.items.len() + queue.installed.len() + queue.repositories.len() == 0
            && problem.is_none()
            && note.is_none()
        {
            return None;
        }
        let found = queue.items.len() + queue.repositories.len();
        let mut body: Vec<AnyElement> = Vec::new();
        if queue.items.is_empty() && (problem.is_some() || note.is_some()) {
            let colour = if problem.is_some() { c.err } else { c.ink2 };
            body.push(
                div()
                    .sized_text(theme, theme.body())
                    .text_color(colour)
                    .child(problem.or(note).unwrap_or_default())
                    .into_any_element(),
            );
        }
        body.extend(queue.collections.iter().map(|name| {
            div()
                .flex()
                .items_center()
                .gap(theme::GAP)
                .child(icon("package", theme.icon(), c.ink3))
                .child(
                    text_fill(theme, theme.body(), c.ink, name.clone()).font_weight(theme.semibold),
                )
                .into_any_element()
        }));
        body.extend(
            queue
                .repositories
                .iter()
                .enumerate()
                .map(|(at, repository)| {
                    let cloning = self.tasks.cloning == Some(at);
                    let link = repository.link.clone();
                    let clone = if cloning {
                        put(&say!("Cloning…"))
                    } else {
                        put(&say!("Clone…"))
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap(theme::GAP)
                        .child(icon("git-branch", theme.icon(), c.ink3))
                        .child(text_fill(
                            theme,
                            theme.body(),
                            c.ink,
                            repository.name.clone(),
                        ))
                        .when(repository.installed, |d| {
                            d.child(Chip::new(put(&say!("Installed"))).tone(c.ok).icon("check"))
                        })
                        .child(
                            Btn::new(("repository-page", at), "")
                                .tip(put(&say!("Repository page")))
                                .icon("external-link")
                                .on_click(move |_, _, cx| cx.open_url(&link)),
                        )
                        .when(!repository.installed, |d| {
                            d.child(
                                Btn::new(("clone", at), clone)
                                    .tip(match self.report.git.is_some() {
                                        true => put(&say!("git clone into the game's own Mods folder; Check for updates then asks its repository")),
                                        false => put(&say!("Git is not installed")),
                                    })
                                    .icon("download")
                                    .primary_if(!cloning)
                                    .enabled(
                                        self.report.git.is_some()
                                            && repository.into.is_some()
                                            && self.tasks.cloning.is_none(),
                                    )
                                    .on_click(cx.listener(move |d, _, window, cx| {
                                        d.ask_clone(at, window, cx)
                                    })),
                            )
                        })
                        .into_any_element()
                }),
        );
        body.extend(
            queue
                .items
                .iter()
                .enumerate()
                .map(|(at, asked)| self.asked_row(theme, at, asked)),
        );
        if !queue.installed.is_empty() {
            let said = put(&say_n!(
                queue.installed.len(),
                "{} mod already installed",
                "{} mods already installed"
            ));
            body.push(
                div()
                    .flex()
                    .child(
                        Btn::new("installed-open", said)
                            .icon("check")
                            .on_click(cx.listener(|d, _, _, cx| {
                                d.installed_open = !d.installed_open;
                                cx.notify();
                            })),
                    )
                    .into_any_element(),
            );
            if self.installed_open {
                body.extend(
                    queue
                        .installed
                        .iter()
                        .enumerate()
                        .map(|(at, asked)| self.asked_row(theme, 1000 + at, asked)),
                );
            }
        }
        let note = if found > 0 {
            found.to_string()
        } else {
            String::new()
        };
        Some(
            Card::new("looked-up", put(&say!("Look up")))
                .note(note)
                .body(body)
                .render(theme, None)
                .into_any_element(),
        )
    }

    /// `ListPane.qml`: the list, and what can be done with it. In a `pane` of its own, as the
    /// Workshop shows it, the rows fill the pane and SteamCMD's install is offered with it.
    pub(crate) fn download_list(
        &self,
        theme: &Theme,
        pane: bool,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let offers_install = pane;
        let c = &theme.colours;
        let list = &self.list;
        if list.count == 0 {
            let said = put(&say!("Nothing on the list yet. Add mods from the results."));
            return vec![
                div()
                    .sized_text(theme, theme.body())
                    .text_color(c.ink3)
                    .pb(theme::GAP)
                    .child(said)
                    .into_any_element(),
                div().when(pane, |d| d.flex_1()).into_any_element(),
            ];
        }
        let rows: Vec<AnyElement> = list
            .rows
            .iter()
            .enumerate()
            .map(|(at, row)| {
                let title = if row.title.is_empty() {
                    row.id.clone()
                } else {
                    row.title.clone()
                };
                let id = row.id.clone();
                div()
                    .flex()
                    .items_center()
                    .gap(theme::GAP)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(text_fill(theme, theme.body(), c.ink, title))
                            .child(text_fill(theme, theme.small(), c.ink3, put(&row.reason)))
                            .when_some(row.failed.as_ref(), |d, failed| {
                                d.child(
                                    div()
                                        .sized_text(theme, theme.small())
                                        .text_color(c.err)
                                        .child(put(failed)),
                                )
                            }),
                    )
                    .when_some(row.size_said.as_ref(), |d, size| {
                        d.child(label(theme, theme.small(), c.ink3, put(size)))
                    })
                    .child(
                        Btn::new(("unlist", at), "")
                            .tip(put(&say!("Take off the list")))
                            .icon("x")
                            .on_click(cx.listener(move |_, _, _, cx| {
                                crate::ui::app(cx).remove_from_list(&id)
                            })),
                    )
                    .into_any_element()
            })
            .collect();
        let failed = list.rows.iter().filter(|row| row.failed.is_some()).count();
        let tall = (theme.line(theme.body()) + theme.line(theme.small())) * list.rows.len() as f32
            + theme.line(theme.small()) * failed as f32
            + theme::GAP_TINY * list.rows.len().saturating_sub(1) as f32;
        let held = div()
            .flex()
            .flex_col()
            .gap(theme::GAP_TINY)
            .pr(crate::ui::page::room(theme))
            .children(rows);
        let mut shown = vec![
            div()
                .when(pane, |d| d.flex_1().min_h_0())
                .when(!pane, |d| d.h(tall.min(theme.sized(240.))))
                .child(crate::ui::page::page(
                    "download-list-rows",
                    &self.list_paging,
                    theme,
                    cx.entity_id(),
                    held,
                ))
                .into_any_element(),
        ];
        if let Some(stuck) = &list.action.stuck {
            shown.push(
                div()
                    .sized_text(theme, theme.small())
                    .text_color(c.ink3)
                    .child(put(stuck))
                    .into_any_element(),
            );
        }
        let action = &list.action;
        let asks_install = offers_install && action.install_steamcmd;
        shown.push(
            div()
                .flex()
                .flex_wrap()
                .gap(theme::GAP)
                .when(action.download, |d| {
                    d.child(
                        Btn::new("download-wanted", put(&action.label))
                            .tip(put(&say!("Into the game's own Mods folder, {}, without subscribing: Steam will not update these.", self.report.local_mods.clone().unwrap_or_default())))
                            .icon("download")
                            .primary()
                            .enabled(action.enabled)
                            .on_click(cx.listener(|d, _, window, cx| d.ask_download(window, cx))),
                    )
                })
                .when(action.subscribe, |d| {
                    d.child(
                        Btn::new("subscribe-wanted", put(&say!("Subscribe in Steam")))
                            .tip(put(&say!("Steam must be running. It subscribes to each and downloads it, then keeps them up to date.")))
                            .icon("plus")
                            .primary_if(!action.download && !asks_install)
                            .enabled(action.subscribe_enabled)
                            .on_click(cx.listener(|d, _, _, cx| d.subscribe_wanted(cx))),
                    )
                })
                .when(asks_install, |d| {
                    d.child(
                        Btn::new("install-steamcmd-listed", put(&action.label))
                            .icon("download")
                            .primary()
                            .enabled(action.enabled)
                            .on_click(cx.listener(|d, _, window, cx| d.ask_install_steamcmd(window, cx))),
                    )
                })
                .when(list.failures, |d| {
                    d.child(
                        Btn::new("retry-list", put(&say!("Retry")))
                            .tip(put(&say!("Forgets what went wrong, so the next download tries these again.")))
                            .icon("refresh-cw")
                            .on_click(
                                cx.listener(|_, _, _, cx| crate::ui::app(cx).clear_list_failures()),
                            ),
                    )
                })
                .child(
                    Btn::new("empty-list", put(&say!("Empty the list")))
                        .on_click(cx.listener(|_, _, _, cx| crate::ui::app(cx).empty_list())),
                )
                .into_any_element(),
        );
        shown
    }

    pub(super) fn add_mods(
        &self,
        theme: &Theme,
        width: Pixels,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let c = &theme.colours;
        let list_note = self.list.size_said.as_ref().map(put).unwrap_or_default();
        let list = Card::new("download-list", put(&say!("Download list")))
            .note(list_note)
            .body(self.download_list(theme, false, cx));
        let installed = put(&say!("Installed"));
        let steamcmd_actions = match self.steamcmd {
            true => (
                vec![
                    Chip::new(installed.clone())
                        .tone(c.ok)
                        .icon("check")
                        .into_any_element(),
                ],
                chip_width(window, theme, &installed, true),
            ),
            false => (Vec::new(), Pixels::ZERO),
        };
        let action = &self.list.action;
        let fold = cx.listener(|d, _, _, cx| d.fold("downloads-steamcmd", cx));
        let steamcmd = Card::new("downloads-steamcmd", put(&say!("SteamCMD")))
            .folds(self.is_folded("downloads-steamcmd", false), fold)
            .tall()
            .actions(steamcmd_actions.0, steamcmd_actions.1)
            .child(
                div()
                    .sized_text(theme, theme.small())
                    .text_color(c.ink3)
                    .child(put(&say!("Fetches a mod without subscribing."))),
            )
            .when_some(self.report.steamcmd_refused.as_ref(), |card, refused| {
                card.child(
                    div()
                        .sized_text(theme, theme.small())
                        .text_color(c.ink3)
                        .child(put(refused)),
                )
            })
            .when(action.install_steamcmd, |card| {
                card.child(
                    div().flex().child(
                        Btn::new("install-steamcmd", put(&action.label))
                            .icon("download")
                            .primary()
                            .enabled(action.enabled)
                            .on_click(
                                cx.listener(|d, _, window, cx| d.ask_install_steamcmd(window, cx)),
                            ),
                    ),
                )
            });
        div()
            .flex()
            .flex_col()
            .gap(theme::GAP_PAGE)
            .children(self.looked_up_card(theme, cx))
            .child(list.render(theme, None))
            .child(steamcmd.render(theme, Some((width, window))))
    }
}
