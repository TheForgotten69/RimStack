//! How `SavesScreen.qml` is laid out: the heading with the RimSaves folders, the saves with their
//! verdicts, and the chosen save beside them.

use super::Saves;
use crate::ui::button::{Btn, Chip};
use crate::ui::field::Field;
use crate::ui::page::{page, room};
use crate::ui::rows::{bar, rows};
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{TextSized, breakable, icon, label, with_alpha};
use crate::words::put;
use crate::workspace::logs::moment;
use crate::workspace::saves::{SaveShown, shown};
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, KeyDownEvent, ObjectFit, ParentElement,
    Pixels, Render, Role, StatefulInteractiveElement, Styled, StyledImage, Window, div, img,
    prelude::FluentBuilder, px,
};
use rimstack_core::rimworld::saves::Verdict;
use rimstack_core::{say, say_n};
use std::rc::Rc;

impl Saves {
    fn side_width(theme: &Theme, window: &Window) -> Pixels {
        theme.sized(360.).min(window.viewport_size().width * 0.36)
    }

    /// A save's row: its picture or its facts, whichever is taller, and the room around them.
    fn row_height(theme: &Theme, save: &SaveShown<'_>, pictured: bool) -> Pixels {
        let chips = chips_of(save).next().is_some();
        let facts = theme.line(theme.title())
            + theme::GAP_TINY
            + theme.line(theme.small())
            + theme::GAP_TINY
            + if chips { theme.tag() } else { px(0.) };
        let picture = if pictured { theme.sized(62.) } else { px(0.) };
        facts.max(picture) + theme::GAP_MEDIUM * 2.
    }

    fn heading(&self, heading: String, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let c = &theme.colours;
        let report = &self.report;
        let mut folders = vec![String::new()];
        folders.extend(report.folders.iter().cloned());
        let current = report.current_folder.clone().unwrap_or_default();
        let tabs = (report.folders.len() > 1).then(|| {
            div()
                .ml(theme::GAP)
                .p(px(2.))
                .rounded(theme::CORNER_LARGE)
                .bg(c.seg)
                .flex()
                .gap(theme::GAP_LINE)
                .children(folders.into_iter().enumerate().map(|(at, folder)| {
                    let on = folder == self.folder;
                    let said = match folder.is_empty() {
                        true => put(&say!("All")),
                        false if folder == current => format!("{folder} ●"),
                        false => folder.clone(),
                    };
                    div()
                        .id(("saves-folder", at))
                        .role(Role::Tab)
                        .aria_label(if folder.is_empty() {
                            put(&say!("All folders"))
                        } else {
                            folder.clone()
                        })
                        .flex()
                        .items_center()
                        .h(theme.sized(24.))
                        .px(theme::GAP_MEDIUM)
                        .rounded(theme::CORNER)
                        .when(on, |d| d.bg(c.ctl_hi))
                        .cursor_pointer()
                        .child(
                            label(theme, theme.small(), if on { c.ink } else { c.ink2 }, said)
                                .font_weight(theme.medium),
                        )
                        .on_click(cx.listener(move |saves, _, _, cx| {
                            saves.folder = folder.clone();
                            saves.selected = 0;
                            cx.notify();
                        }))
                }))
        });
        div()
            .flex()
            .items_center()
            .gap(theme::GAP)
            .h(theme.header())
            .px(theme::GAP_CARD)
            .child(
                label(theme, theme.heading(), c.ink, put(&say!("Saves")))
                    .font_weight(theme.semibold),
            )
            .child(label(theme, theme.body(), c.ink3, heading))
            .children(tabs)
            .into_any_element()
    }

    fn list(&self, theme: &Theme, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let c = theme.colours;
        let report = self.report.clone();
        let (folder, filter) = (self.folder.clone(), self.filter_text.clone());
        let on_show = shown(&report, &folder, &filter);
        let (saves, pictured) = (&on_show.saves, on_show.pictured);
        let all_from = report.all_from.is_some();
        if saves.is_empty() {
            let width = (window.viewport_size().width
                - Self::side_width(theme, window)
                - theme::GAP_PAGE * 4.)
                .min(theme.sized(460.));
            let title = if self.busy {
                put(&say!("Reading saves…"))
            } else {
                put(&say!("No saves"))
            };
            let words = if self.folder.is_empty() {
                String::new()
            } else {
                put(&say!("RimSaves folder {} is empty", self.folder.clone()))
            };
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(theme::GAP_SMALL)
                        .w(width)
                        .child(
                            label(theme, theme.title(), c.ink, title).font_weight(theme.semibold),
                        )
                        .when(!self.busy, |d| {
                            d.child(
                                div()
                                    .text_center()
                                    .spaced_text(theme, theme.body(), 1.3)
                                    .text_color(c.ink2)
                                    .child(words),
                            )
                        }),
                )
                .into_any_element();
        }
        let mut at = theme::GAP_HAIR;
        let mut tops = Vec::with_capacity(saves.len() + 1);
        for save in saves.iter() {
            tops.push(at);
            at += Self::row_height(theme, save, pictured) + theme::GAP_HAIR;
        }
        tops.push(at - theme::GAP_HAIR + theme::GAP_MEDIUM);
        let tops: Rc<[Pixels]> = tops.into();
        let (selected, me, thumbs, view) = (
            self.selected,
            cx.entity().downgrade(),
            self.thumbs.clone(),
            cx.entity_id(),
        );
        let theme_owned = theme.clone();
        let (held, folder, filter) = (report.clone(), folder.clone(), filter.clone());
        let list = rows(
            "save-rows",
            self.scroll.clone(),
            tops.clone(),
            theme.sized(40.),
            cx.entity_id(),
            move |range, width, window, cx| {
                let on_show = shown(&held, &folder, &filter);
                range
                    .filter_map(|at| {
                        let save = on_show.saves.get(at)?;
                        let height = Self::row_height(&theme_owned, save, pictured);
                        Some(save_row(
                            &theme_owned,
                            at,
                            save,
                            at == selected,
                            (pictured, all_from),
                            width,
                            height,
                            &thumbs,
                            view,
                            me.clone(),
                            window,
                            cx,
                        ))
                    })
                    .collect()
            },
        );
        let count = saves.len();
        let usable: Vec<Option<Vec<String>>> = saves
            .iter()
            .map(|s| {
                s.usable
                    .then(|| s.row.compared.as_ref().map(|c| c.ids.clone()))
                    .flatten()
            })
            .collect();
        div()
            .id("saves")
            .track_focus(&self.focus)
            .role(Role::List)
            .aria_label(put(&say!("Saves")))
            .relative()
            .flex_1()
            .min_h_0()
            .on_key_down(cx.listener(move |saves, event: &KeyDownEvent, _, cx| {
                let to = match event.keystroke.key.as_str() {
                    "up" => saves.selected.saturating_sub(1),
                    "down" => (saves.selected + 1).min(count - 1),
                    "home" => 0,
                    "end" => count - 1,
                    "enter" => {
                        if let Some(Some(ids)) = usable.get(saves.selected) {
                            cx.emit(super::UseList(ids.clone()));
                        }
                        return;
                    }
                    _ => return,
                };
                saves.selected = to;
                saves.scroll.borrow_mut().contain(tops[to], tops[to + 1]);
                cx.notify();
            }))
            .child(list)
            .child(bar(theme, self.scroll.clone(), cx.entity_id()))
            .into_any_element()
    }

    fn side(
        &self,
        save: Option<&SaveShown<'_>>,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colours;
        let width = Self::side_width(theme, window);
        let inner = width - theme::GAP_CARD * 2. - room(theme);
        let group = |text: String| {
            label(theme, theme.small(), c.ink2, text)
                .font_weight(theme.semibold)
                .pt(theme::GAP_WIDE)
                .pb(theme::GAP_SMALL)
        };
        let body = |colour, text: String| {
            div()
                .sized_text(theme, theme.body())
                .text_color(colour)
                .child(text)
        };
        let content = save.map(|save| {
            let row = save.row;
            let compared = row.compared.as_ref();
            let info = match self.report.rimsaves {
                true => put(&say!(
                    "In {}, {}, {} MB",
                    row.folder.clone(),
                    moment(row.modified),
                    format!("{:.1}", row.megabytes)
                )),
                false => put(&say!(
                    "{}, {} MB",
                    moment(row.modified),
                    format!("{:.1}", row.megabytes)
                )),
            };
            let ids = compared.map(|c| c.ids.clone());
            let breaks = row.verdict == Verdict::Breaks;
            let dir = save.dir.clone();
            let missing = compared.map(|c| c.missing.clone()).unwrap_or_default();
            let added = compared.map(|c| c.added.clone()).unwrap_or_default();
            let typed = !self.list_name.read(cx).text().trim().is_empty();
            let field = Field {
                id: "save-list-name",
                text: &self.list_name,
                icon: "",
                placeholder: put(&say!("Save its list as…")),
                found: None,
                keys: None,
            };
            let more = |n: usize, open: bool| {
                if open {
                    put(&say!("Show five"))
                } else {
                    put(&say!("{} more", n - 5))
                }
            };
            div()
                .flex()
                .flex_col()
                .when_some(row.preview.clone(), |d, preview| {
                    d.child(
                        div()
                            .w(inner)
                            .h(theme.sized(190.))
                            .rounded(theme::CORNER_LARGE)
                            .bg(c.well)
                            .overflow_hidden()
                            .child(
                                img(std::path::PathBuf::from(preview))
                                    .w(inner)
                                    .h(theme.sized(190.))
                                    .object_fit(ObjectFit::Cover),
                            ),
                    )
                    .child(div().h(theme::GAP_LARGE))
                })
                .child(
                    div()
                        .sized_text(theme, theme.heading())
                        .font_weight(theme.semibold)
                        .text_color(c.ink)
                        .child(breakable(&row.name)),
                )
                .child(
                    div()
                        .pt(theme::GAP_HAIR)
                        .sized_text(theme, theme.body())
                        .text_color(c.ink2)
                        .child(info),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(theme::GAP_SMALL)
                        .pt(theme::GAP_MEDIUM)
                        .when(save.usable, |d| {
                            let ids = ids.clone().unwrap_or_default();
                            d.child(
                                Btn::new("use-save-list", put(&say!("Use this save's mod list")))
                                    .icon("list")
                                    .primary_if(breaks)
                                    .on_click(cx.listener(move |_, _, _, cx| {
                                        cx.emit(super::UseList(ids.clone()))
                                    })),
                            )
                        })
                        .child(
                            Btn::new("open-save-folder", put(&say!("Open folder")))
                                .icon("folder-open")
                                .on_click(move |_, _, cx| {
                                    cx.open_with_system(std::path::Path::new(&dir))
                                }),
                        ),
                )
                .when(!row.facts.is_empty(), |d| {
                    d.child(group(put(&say!("Colony"))))
                        .children(row.facts.iter().map(|(name, value)| {
                            div()
                                .flex()
                                .items_center()
                                .gap(theme::GAP)
                                .min_h(theme.sized(22.))
                                .child(
                                    div()
                                        .w(theme.sized(110.))
                                        .max_w(inner / 2.)
                                        .flex_none()
                                        .sized_text(theme, theme.body())
                                        .text_color(c.ink2)
                                        .child(put(
                                            &rimstack_core::base::sentence::Sentence::named(name),
                                        )),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .sized_text(theme, theme.body())
                                        .text_color(c.ink)
                                        .child(value.clone()),
                                )
                        }))
                })
                .when(ids.is_some(), |d| {
                    d.child(div().h(theme::GAP_LARGE)).child(
                        div()
                            .flex()
                            .items_center()
                            .gap(theme::GAP_SMALL)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(field.render(theme, window, cx)),
                            )
                            .child(
                                Btn::new("keep-save-list", put(&say!("Keep")))
                                    .enabled(typed)
                                    .on_click(cx.listener(|saves, _, _, cx| saves.keep_list(cx))),
                            ),
                    )
                })
                .when(!self.list_problem.is_empty(), |d| {
                    d.child(body(c.err, self.list_problem.clone()))
                })
                .when_some(row.problem.clone(), |d, problem| {
                    d.child(group(put(&say!("Problem"))))
                        .child(body(c.err, put(&say!("Unreadable: {}", problem))))
                })
                .when(ids.is_some(), |d| {
                    let same = row.verdict == Verdict::Same;
                    d.child(group(put(&say!("Differences from the current list"))))
                        .when(missing.is_empty() && !same, |d| {
                            d.child(body(
                                c.ink2,
                                put(&say!("Nothing the save needs is missing.")),
                            ))
                        })
                        .when(same, |d| {
                            d.child(
                                div()
                                    .spaced_text(theme, theme.body(), 1.3)
                                    .text_color(c.ink)
                                    .child(put(&say!("Same mods, same order as the load order"))),
                            )
                        })
                })
                .when(compared.is_some_and(|c| c.other_game), |d| {
                    let version = compared.map(|c| c.game_version.clone()).unwrap_or_default();
                    d.child(group(put(&say!("Other game version")))).child(
                        div()
                            .spaced_text(theme, theme.body(), 1.3)
                            .text_color(c.warn)
                            .child(put(&say!(
                                "Saved with RimWorld {}, not the installed version",
                                version
                            ))),
                    )
                })
                .when(!missing.is_empty(), |d| {
                    let shown = if self.all_missing {
                        missing.len()
                    } else {
                        missing.len().min(5)
                    };
                    d.child(
                        div()
                            .pt(theme::GAP_TINY)
                            .sized_text(theme, theme.body())
                            .text_color(if save.not_installed > 0 {
                                c.err
                            } else {
                                c.ink2
                            })
                            .child(put(&save.missing_said)),
                    )
                    .children(missing.iter().take(shown).map(|gone| {
                        div()
                            .flex()
                            .items_center()
                            .gap(theme::GAP)
                            .min_h(theme.sized(22.))
                            .child(icon(
                                if gone.installed {
                                    "minus"
                                } else {
                                    "circle-alert"
                                },
                                theme.icon(),
                                if gone.installed { c.ink3 } else { c.err },
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .sized_text(theme, theme.body())
                                    .text_color(if gone.installed { c.ink } else { c.err })
                                    .child(gone.name.clone()),
                            )
                            .child(label(
                                theme,
                                theme.small(),
                                c.ink3,
                                if gone.installed {
                                    put(&say!("not active"))
                                } else {
                                    put(&say!("not installed"))
                                },
                            ))
                    }))
                    .when(missing.len() > 5, |d| {
                        d.child(
                            div().flex().child(
                                Btn::new("all-missing", more(missing.len(), self.all_missing))
                                    .on_click(cx.listener(|saves, _, _, cx| {
                                        saves.all_missing = !saves.all_missing;
                                        cx.notify();
                                    })),
                            ),
                        )
                    })
                })
                .when(!added.is_empty(), |d| {
                    let shown = if self.all_added {
                        added.len()
                    } else {
                        added.len().min(5)
                    };
                    d.child(
                        div()
                            .pt(theme::GAP)
                            .sized_text(theme, theme.body())
                            .text_color(c.ink2)
                            .child(put(&say_n!(
                                added.len(),
                                "{} mod added since",
                                "{} mods added since"
                            ))),
                    )
                    .children(added.iter().take(shown).map(|name| {
                        div()
                            .flex()
                            .items_center()
                            .gap(theme::GAP)
                            .min_h(theme.sized(22.))
                            .child(icon("plus", theme.icon(), c.ink3))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .sized_text(theme, theme.body())
                                    .text_color(c.ink)
                                    .child(name.clone()),
                            )
                    }))
                    .when(added.len() > 5, |d| {
                        d.child(div().flex().child(
                            Btn::new("all-added", more(added.len(), self.all_added)).on_click(
                                cx.listener(|saves, _, _, cx| {
                                    saves.all_added = !saves.all_added;
                                    cx.notify();
                                }),
                            ),
                        ))
                    })
                })
                .when(compared.is_some_and(|c| c.reordered), |d| {
                    d.child(
                        div()
                            .pt(theme::GAP_TINY)
                            .spaced_text(theme, theme.body(), 1.3)
                            .text_color(c.ink)
                            .child(put(&say!("Shared mods are in a different order now"))),
                    )
                })
                .child(group(put(&say!("Backups"))))
                .child(body(
                    c.ink,
                    put(&say!(
                        "The newest save is copied before each launch; the last 5 are kept"
                    )),
                ))
                .child(div().h(theme::GAP_SMALL))
                .child(div().flex().child(
                    Btn::new("open-backups", put(&say!("Open the backups folder"))).on_click(
                        |_, _, cx| {
                            let folder = crate::ui::app(cx).data_dir().join("save-backups");
                            cx.open_with_system(&folder);
                        },
                    ),
                ))
        });
        div()
            .flex_none()
            .w(width)
            .h_full()
            .bg(c.panel)
            .border_l_1()
            .border_color(c.edge)
            .when_some(content, |d, content| {
                d.child(page(
                    "save-side",
                    &self.side_paging,
                    theme,
                    cx.entity_id(),
                    div()
                        .p(theme::GAP_CARD)
                        .pr(theme::GAP_CARD + room(theme))
                        .child(content),
                ))
            })
            .into_any_element()
    }
}

/// The chips a save's row shows, as `SavesScreen.qml` lists them, each with its tone: 0 the
/// error colour, 1 the good one, 2 the warning, 3 the other game's, 4 the plain one.
fn chips_of<'a>(save: &'a SaveShown<'_>) -> impl Iterator<Item = (Chip, usize)> + 'a {
    let row = save.row;
    let compared = row.compared.as_ref();
    let added = compared.map_or(0, |c| c.added.len());
    [
        (row.verdict == Verdict::Unreadable).then(|| {
            (
                say!("cannot be read"),
                0,
                Some("circle-alert"),
                String::new(),
            )
        }),
        (row.verdict == Verdict::Same)
            .then(|| (say!("same mods as now"), 1, Some("check"), String::new())),
        (save.not_installed > 0).then(|| {
            (
                say!("{} not installed", save.not_installed),
                0,
                Some("circle-alert"),
                put(&say!(
                    "Mods the save was played with that are not installed"
                )),
            )
        }),
        (save.not_active > 0).then(|| {
            (
                say!("{} not active", save.not_active),
                2,
                Some("minus"),
                put(&say!(
                    "Mods the save was played with that are not active now"
                )),
            )
        }),
        compared
            .filter(|c| c.other_game)
            .map(|_| (say!("other game version"), 3, None, String::new())),
        (added > 0).then(|| {
            (
                say!("{} added", added),
                4,
                None,
                put(&say!("Mods active now that the save was not played with")),
            )
        }),
        compared
            .filter(|c| c.reordered)
            .map(|_| (say!("reordered"), 4, None, String::new())),
    ]
    .into_iter()
    .flatten()
    .map(|(text, tone, icon, tip)| {
        let chip = Chip::new(put(&text)).tip(tip);
        let chip = match icon {
            Some(name) => chip.icon(name),
            None => chip,
        };
        (chip, tone)
    })
}

#[allow(clippy::too_many_arguments)]
fn save_row(
    theme: &Theme,
    at: usize,
    save: &SaveShown<'_>,
    current: bool,
    (pictured, all_from): (bool, bool),
    width: Pixels,
    height: Pixels,
    thumbs: &crate::ui::thumbs::Shared,
    view: gpui::EntityId,
    me: gpui::WeakEntity<Saves>,
    window: &Window,
    cx: &mut gpui::App,
) -> AnyElement {
    let c = &theme.colours;
    let row = save.row;
    let quiet = if current { c.on_selected_dim } else { c.ink2 };
    let end = theme::GAP_PAGE + theme::GAP_SMALL;
    let hover = with_alpha(c.hover, 0.05);
    let picture = row
        .preview
        .as_deref()
        .filter(|_| pictured)
        .and_then(|preview| {
            let scale = window.scale_factor();
            let size = (
                (f32::from(theme.sized(104.)) * scale).round() as u32,
                (f32::from(theme.sized(62.)) * scale).round() as u32,
            );
            crate::ui::thumbs::thumb(thumbs, preview, size, view, cx)
        });
    let about = format!("{}   {}", moment(row.modified), put(&save.about));
    let other = if all_from { c.ink2 } else { c.warn };
    let tones = [c.err, c.ok, c.warn, other, c.ink2];
    let chips: Vec<Chip> = chips_of(save)
        .map(|(chip, tone)| chip.tone(tones[tone]).on_accent(current))
        .collect();
    div()
        .id(("save", at))
        .role(Role::ListItem)
        .aria_label(format!("{}, {}", row.name, put(&save.said)))
        .relative()
        .w(width)
        .h(height)
        .flex()
        .items_center()
        .gap(theme::GAP_LARGE)
        .pl(theme::GAP_PAGE)
        .pr(end)
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(theme::GAP_LARGE)
                .right(theme::GAP_CARD)
                .rounded(theme::CORNER)
                .when(current, |d| d.bg(c.chosen))
                .when(!current, |d| d.hover(move |s| s.bg(hover))),
        )
        .when(!current, |d| {
            d.child(
                div()
                    .absolute()
                    .bottom(px(-1.))
                    .left(theme::GAP_PAGE)
                    .right(end)
                    .h(px(1.))
                    .bg(with_alpha(c.line, 0.6)),
            )
        })
        .when(pictured, |d| {
            d.child(
                div()
                    .relative()
                    .flex_none()
                    .w(theme.sized(104.))
                    .h(theme.sized(62.))
                    .rounded(theme::CORNER)
                    .bg(c.card)
                    .overflow_hidden()
                    .when(row.preview.is_none(), |d| {
                        d.border_1()
                            .border_color(c.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon("save", theme.sized(22.), c.ink3))
                    })
                    .children(picture.map(|picture| img(picture).absolute().inset_0().size_full())),
            )
        })
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(theme::GAP_TINY)
                .child(
                    div()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .sized_text(theme, theme.title())
                        .font_weight(theme.semibold)
                        .text_color(if current { c.on_selected } else { c.ink })
                        .child(row.name.clone()),
                )
                .child(
                    div()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .sized_text(theme, theme.small())
                        .text_color(quiet)
                        .child(about),
                )
                .when(!chips.is_empty(), |d| {
                    d.child(div().flex().gap(theme::GAP_TINY).children(chips))
                }),
        )
        .when(row.megabytes > 0.0, |d| {
            d.child(label(
                theme,
                theme.small(),
                quiet,
                put(&say!("{} MB", format!("{:.1}", row.megabytes))),
            ))
        })
        .on_click(move |_, _, cx| {
            let _ = me.update(cx, |saves, cx| {
                saves.selected = at;
                cx.notify();
            });
        })
        .into_any_element()
}

impl Render for Saves {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let report = self.report.clone();
        let on_show = shown(&report, &self.folder, &self.filter_text);
        let heading = put(&on_show.heading);
        let searchable = on_show.searchable;
        let chosen = on_show.saves.get(self.selected);
        let side = self.side(chosen, &theme, window, cx);
        let list = self.list(&theme, window, cx);
        let filter = searchable.then(|| {
            let field = Field {
                id: "saves-filter",
                text: &self.filter,
                icon: "funnel",
                placeholder: put(&say!("Filter saves")),
                found: None,
                keys: None,
            };
            div()
                .px(theme::GAP_CARD)
                .pb(theme::GAP_SMALL)
                .child(field.render(&theme, window, cx).h(theme.sized(30.)))
        });
        div()
            .flex()
            .size_full()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(self.heading(heading, &theme, cx))
                    .children(filter)
                    .child(list),
            )
            .child(side)
    }
}
