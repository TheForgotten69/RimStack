//! The middle of `LogsScreen.qml`: the log's heading and its controls, what changed since the
//! last clean launch, the filter, and the errors, the mods they point at, or the log's lines.

use super::{Logs, View};
use crate::ui::button::{Btn, Chip};
use crate::ui::controls::{Choice, Toggle};
use crate::ui::field::Field;
use crate::ui::menu::{self, Entry};
use crate::ui::rows::{bar, rows};
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{TextSized, breakable, btn_width, label, measure, measure_in, with_alpha};
use crate::words::{count, put};
use crate::workspace::logs::{head, moment, since};
use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, ParentElement, Pixels, Role, SharedString, StatefulInteractiveElement, Styled,
    Window, div, prelude::FluentBuilder, px,
};
use rimstack_core::{say, say_n};
use std::rc::Rc;

impl Logs {
    /// The middle's width: the window less the launches, the side and their edges.
    pub(super) fn middle_width(theme: &Theme, window: &Window) -> Pixels {
        window.viewport_size().width
            - Self::pane_width(theme, window)
            - Self::side_width(theme, window)
            - px(2.)
    }

    fn head(&self, theme: &Theme, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let c = &theme.colours;
        let report = self.report.clone();
        let facts = report.as_deref().map(head);
        let path = facts
            .as_ref()
            .map(|f| f.path.to_string())
            .unwrap_or_default();
        let written = match facts.as_ref().filter(|f| f.written > 0) {
            Some(facts) => moment(facts.written),
            None if self.previous => put(&say!("Previous launch")),
            None => put(&say!("Last launch")),
        };
        let about = facts.as_ref().map(|f| put(&f.about)).unwrap_or_default();
        let room = Self::middle_width(theme, window) - theme::GAP_CARD - theme::GAP_LARGE;
        let (open, copy, find) = (
            put(&say!("Open file")),
            put(&say!("Copy path")),
            put(&say!("Find the broken mod")),
        );
        let views: Vec<SharedString> = [say!("Errors"), say!("Mods"), say!("Log")]
            .iter()
            .map(|o| put(o).into())
            .collect();
        let finds = !self.demo && !self.hunting;
        let full = |text: &str| btn_width(window, theme, text, true, false);
        let choice = Choice::width(window, theme, &views);
        let tight = room
            < full(&open)
                + full(&copy)
                + if finds { full(&find) } else { px(0.) }
                + choice
                + theme::GAP * 3.;
        let compact = theme.icon() + theme.sized(14.);
        let controls_width = if tight {
            compact * 2. + if finds { compact } else { px(0.) }
        } else {
            full(&open) + full(&copy) + if finds { full(&find) } else { px(0.) }
        } + choice
            + theme::GAP * 3.;
        let wrapped = room
            < measure(window, theme, &written, theme.title(), theme.semibold)
                + controls_width
                + theme::GAP_WIDE;
        let button = |id: &'static str, text: String, icon: &'static str| match tight {
            true => Btn::new(id, "").icon(icon).tip(text),
            false => Btn::new(id, text).icon(icon),
        };
        let (opened, copied) = (path.clone(), path.clone());
        let controls = div()
            .flex()
            .items_center()
            .gap(theme::GAP)
            .child(
                button("open-log", open, "external-link")
                    .tip(path.clone())
                    .enabled(!path.is_empty())
                    .on_click(move |_, _, cx| cx.open_with_system(std::path::Path::new(&opened))),
            )
            .child(button("copy-log-path", copy, "copy").enabled(!path.is_empty()).on_click(move |_, _, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(copied.clone()))))
            .when(finds, |d| {
                let tip = format!("{}. {}", find, put(&say!("Deactivates half of the mods; you play and say whether it still breaks. This repeats until one mod is left. Your list is kept and comes back at the end.")));
                d.child(button("find-broken", find.clone(), "search").tip(tip).on_click(cx.listener(|_, _, _, cx| cx.emit(super::FindBroken))))
            })
            .child(Choice::new("log-view", views, self.view as usize).title(put(&say!("View"))).on_pick(crate::ui::act(cx, |logs, at: usize, _, cx| {
                logs.view = [View::Errors, View::Mods, View::Log][at];
                cx.notify();
            })));
        div()
            .flex()
            .when(wrapped, |d| {
                d.flex_col()
                    .items_start()
                    .gap(theme::GAP_SMALL)
                    .py(theme::GAP_SMALL)
            })
            .when(!wrapped, |d| d.items_center().gap(theme::GAP))
            .min_h(theme.header())
            .pl(theme::GAP_CARD)
            .pr(theme::GAP_LARGE)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(label(theme, theme.title(), c.ink, written).font_weight(theme.semibold))
                    .child(
                        div()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .sized_text(theme, theme.small())
                            .text_color(c.ink3)
                            .child(about),
                    ),
            )
            .child(controls)
            .into_any_element()
    }

    fn since_box(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let c = &theme.colours;
        let on_last = !self.previous && self.kept.is_none();
        let report = self.report.clone()?;
        let since = since(&report).filter(|_| on_last && self.view == View::Errors)?;
        let ids = since.ids.to_vec();
        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap(px(2.))
            .child(
                div()
                    .sized_text(theme, theme.body())
                    .font_weight(theme.semibold)
                    .text_color(c.ink)
                    .child(put(&say!("Since the last clean launch"))),
            )
            .child(label(theme, theme.small(), c.ink2, moment(since.time)));
        if since.same {
            column = column.child(label(
                theme,
                theme.body(),
                c.ink2,
                put(&say!("Same load order")),
            ));
        } else {
            if !since.added.is_empty() {
                column = column.child(
                    div()
                        .sized_text(theme, theme.body())
                        .text_color(c.ink)
                        .child(format!("{} {}", put(&say!("Added")), since.added)),
                );
            }
            if !since.removed.is_empty() {
                column = column.child(
                    div()
                        .sized_text(theme, theme.body())
                        .text_color(c.ink)
                        .child(format!("{} {}", put(&say!("Removed")), since.removed)),
                );
            }
            if since.reordered {
                column = column.child(label(
                    theme,
                    theme.body(),
                    c.ink2,
                    put(&say!("Order changed")),
                ));
            }
        }
        Some(
            div()
                .flex()
                .items_center()
                .gap(theme::GAP)
                .mx(theme::GAP_LARGE)
                .mb(theme::GAP)
                .py(theme::GAP_MEDIUM)
                .pl(theme::GAP_LARGE)
                .pr(theme::GAP_MEDIUM)
                .rounded(theme::CORNER_LARGE)
                .bg(c.card)
                .child(column)
                .when(!since.same, |d| {
                    d.child(
                        Btn::new("use-clean-list", put(&say!("Use")))
                            .tip(put(&say!("Uses the load order of the last clean launch as the list on the Mods screen. Save writes it.")))
                            .on_click(cx.listener(move |_, _, _, cx| cx.emit(super::UseList(ids.clone())))),
                    )
                })
                .into_any_element(),
        )
    }

    fn filter_row(
        &self,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if self.view == View::Mods {
            return None;
        }
        let placeholder = match self.view {
            View::Log => put(&say!("Search the log")),
            _ => put(&say!("Filter by text or mod")),
        };
        let field = Field {
            id: "log-filter",
            text: &self.filter,
            icon: "",
            placeholder,
            found: None,
            keys: None,
        };
        Some(
            div()
                .flex()
                .items_center()
                .gap(theme::GAP)
                .mx(theme::GAP_LARGE)
                .mb(theme::GAP)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(field.render(theme, window, cx)),
                )
                .when(self.view == View::Log, |d| {
                    d.child(self.wrap_switch(
                        "wrap-log",
                        "wrapLog",
                        self.wrap_log,
                        put(&say!("Wrap long lines in the log")),
                        theme,
                        cx,
                    ))
                })
                .into_any_element(),
        )
    }

    /// `WrapSwitch`: a silent switch and its words, kept as `key` in `[Appearance]`.
    pub(super) fn wrap_switch(
        &self,
        id: &'static str,
        key: &'static str,
        on: bool,
        tip: String,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let title = put(&say!("Wrap long lines"));
        let switch = div()
            .id(id)
            .flex()
            .items_center()
            .gap(theme::GAP)
            .child(
                Toggle::new((id, 1usize), on, title.clone())
                    .silent()
                    .on_flip(crate::ui::act(cx, move |logs, on: bool, _, cx| {
                        logs.keep_wrap(key, on, cx)
                    })),
            )
            .child(label(theme, theme.small(), theme.colours.ink2, title));
        crate::ui::tip::tipped(switch, tip.into()).into_any_element()
    }

    /// The errors' rows: as tall as their words and the room around them, `gapHair` apart.
    fn error_tops(&self, theme: &Theme) -> Rc<[Pixels]> {
        let Some(report) = &self.report else {
            return Rc::from(vec![theme::GAP_TINY]);
        };
        let mut at = theme::GAP_TINY;
        let mut tops = Vec::new();
        for shown in self.errors(report) {
            tops.push(at);
            let words = theme.line(theme.body())
                + if shown.detail.is_empty() {
                    px(0.)
                } else {
                    theme::GAP_THIN + theme.line(theme.small())
                };
            at += theme.sized(40.).max(words + theme.sized(18.)) + theme::GAP_HAIR;
        }
        tops.push(at);
        tops.into()
    }

    /// The log's rows: one line each, or as many as a line wraps to in the monospace family.
    pub(super) fn log_tops(&mut self, theme: &Theme, window: &Window) -> Rc<[Pixels]> {
        let width = Self::middle_width(theme, window);
        if let Some((at, tops)) = &self.log_tops
            && *at == width
        {
            return tops.clone();
        }
        let line = theme.line(theme.small());
        let row = |rows: usize| theme.sized(20.).max(line * rows as f32 + px(4.));
        let room = width - Self::log_text_x(theme) - theme::GAP_WIDE;
        let advance = measure_in(
            window,
            theme,
            theme.mono.clone(),
            "n",
            theme.small(),
            FontWeight::NORMAL,
        );
        let per_row = ((room / advance).floor() as usize).max(1);
        let mut at = theme::GAP_TINY;
        let mut tops = Vec::with_capacity(self.lines.len() + 1);
        let text = self.report.as_ref().map(|r| &r.text);
        for &line_at in self.lines.iter() {
            tops.push(at);
            let chars = text.map_or(0, |t| t[line_at].chars().count());
            let rows = if self.wrap_log {
                chars.div_ceil(per_row).max(1)
            } else {
                1
            };
            at += row(rows);
        }
        tops.push(at);
        let tops: Rc<[Pixels]> = tops.into();
        self.log_tops = Some((width, tops.clone()));
        tops
    }

    fn log_text_x(theme: &Theme) -> Pixels {
        theme::GAP + theme.sized(52.) + theme::GAP_SMALL + theme.sized(16.) + theme::GAP
    }

    fn errors_view(&self, theme: &Theme, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let c = theme.colours;
        let report = self.report.clone();
        let count_rows = report.as_ref().map_or(0, |r| self.errors(r).len());
        if count_rows == 0 {
            let facts = report.as_deref().map(head);
            let filtered = !self.filter_text.is_empty();
            let title = match () {
                _ if self.busy => put(&say!("Reading the log…")),
                _ if self.problem.is_some() => put(&say!("Log could not be read")),
                _ if filtered => put(&say!("No error matches")),
                _ if facts.as_ref().is_some_and(|f| f.lines > 0) => put(&say!("No errors")),
                _ => put(&say!("No log")),
            };
            let words = match &self.problem {
                Some(problem) => problem.clone(),
                None if filtered => String::new(),
                None => match facts.as_ref().filter(|f| f.lines > 0) {
                    Some(f) => put(&say_n!(f.lines, "{} line", "{} lines")),
                    None => put(&say!("Written when the game starts")),
                },
            };
            let width =
                (Self::middle_width(theme, window) - theme::GAP_PAGE * 4.).min(theme.sized(480.));
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
                            div()
                                .text_center()
                                .sized_text(theme, theme.title())
                                .font_weight(theme.semibold)
                                .text_color(if self.problem.is_some() { c.err } else { c.ink })
                                .child(title),
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
        let tops = self.error_tops(theme);
        let selected = self.selected;
        let me = cx.entity().downgrade();
        let draw_tops = tops.clone();
        let theme_owned = theme.clone();
        let list = rows(
            "error-rows",
            self.errors_scroll.clone(),
            tops.clone(),
            theme.sized(40.),
            cx.entity_id(),
            move |range, width, window, cx| {
                let Some(logs) = me.upgrade() else {
                    return Vec::new();
                };
                let report = logs.read(cx).report.clone();
                let Some(report) = report else {
                    return Vec::new();
                };
                let shown = logs.read(cx).errors(&report);
                range
                    .filter_map(|at| {
                        Some(error_row(
                            &theme_owned,
                            at,
                            shown.get(at)?,
                            at == selected,
                            width,
                            draw_tops[at + 1] - draw_tops[at] - theme::GAP_HAIR,
                            me.clone(),
                            window,
                            cx,
                        ))
                    })
                    .collect()
            },
        );
        let count = count_rows;
        let keys_tops = tops;
        div()
            .id("errors")
            .track_focus(&self.focus.errors)
            .role(Role::List)
            .aria_label(put(&say!("Errors")))
            .relative()
            .flex_1()
            .min_h_0()
            .on_key_down(cx.listener(move |logs, event: &KeyDownEvent, _, cx| {
                if let Some(to) = step(event, logs.selected, count) {
                    logs.selected = to;
                    logs.errors_scroll
                        .borrow_mut()
                        .contain(keys_tops[to], keys_tops[to + 1]);
                    cx.notify();
                }
            }))
            .child(list)
            .child(bar(theme, self.errors_scroll.clone(), cx.entity_id()))
            .into_any_element()
    }

    fn culprits_view(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let c = theme.colours;
        let culprits: Rc<Vec<rimstack_core::mods::blame::Blamed>> = Rc::new(
            self.report
                .as_ref()
                .map(|r| r.culprits.clone())
                .unwrap_or_default(),
        );
        let height = theme.sized(34.);
        let tops: Rc<[Pixels]> = (0..=culprits.len())
            .map(|at| theme::GAP_TINY + height * at as f32)
            .collect::<Vec<_>>()
            .into();
        let (current, me) = (self.culprit, cx.entity().downgrade());
        let theme_owned = theme.clone();
        let shown = culprits.clone();
        let list = rows(
            "culprit-rows",
            self.culprits_scroll.clone(),
            tops.clone(),
            height,
            cx.entity_id(),
            move |range, width, _, _| {
                let theme = &theme_owned;
                range
                    .filter_map(|at| {
                        let culprit = shown.get(at)?.clone();
                        let on = at == current;
                        let name = if culprit.name.is_empty() {
                            put(&say!("No mod recognised"))
                        } else {
                            culprit.name.clone()
                        };
                        let said = format!(
                            "{}, {}",
                            put(&say_n!(culprit.kinds, "{} error", "{} errors")),
                            put(&say_n!(culprit.times, "logged {} time", "logged {} times"))
                        );
                        let (me, hover) = (me.clone(), with_alpha(c.hover, 0.04));
                        Some(
                            div()
                                .id(("culprit", at))
                                .role(Role::ListItem)
                                .aria_label(put(&say_n!(
                                    culprit.kinds,
                                    "{} error, {}",
                                    "{} errors, {}",
                                    name.clone()
                                )))
                                .relative()
                                .w(width)
                                .h(height)
                                .flex()
                                .items_center()
                                .gap(theme::GAP_LARGE)
                                .px(theme::GAP_WIDE)
                                .cursor_pointer()
                                .child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .bottom_0()
                                        .left(theme::GAP_SMALL)
                                        .right(theme::GAP_SMALL)
                                        .rounded(theme::CORNER)
                                        .when(on, |d| d.bg(c.chosen))
                                        .when(!on, |d| d.hover(move |s| s.bg(hover))),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .sized_text(theme, theme.body())
                                        .text_color(if on {
                                            c.on_selected
                                        } else if culprit.name.is_empty() {
                                            c.ink2
                                        } else {
                                            c.ink
                                        })
                                        .child(name),
                                )
                                .child(label(
                                    theme,
                                    theme.body(),
                                    if on { c.on_selected_dim } else { c.ink2 },
                                    said,
                                ))
                                .on_click(move |event: &ClickEvent, _, cx| {
                                    let name = culprit.name.clone();
                                    let _ = me.update(cx, |logs, cx| {
                                        logs.culprit = at;
                                        if event.click_count() == 2 {
                                            logs.open_culprit(&name, cx);
                                        }
                                        cx.notify();
                                    });
                                })
                                .into_any_element(),
                        )
                    })
                    .collect()
            },
        );
        let count = culprits.len();
        div()
            .id("culprits")
            .track_focus(&self.focus.culprits)
            .role(Role::List)
            .aria_label(put(&say!("Mods the errors point at")))
            .relative()
            .flex_1()
            .min_h_0()
            .on_key_down(cx.listener(move |logs, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "enter" {
                    let name = culprits
                        .get(logs.culprit)
                        .map(|c| c.name.clone())
                        .unwrap_or_default();
                    return logs.open_culprit(&name, cx);
                }
                if let Some(to) = step(event, logs.culprit, count) {
                    logs.culprit = to;
                    logs.culprits_scroll
                        .borrow_mut()
                        .contain(tops[to], tops[to + 1]);
                    cx.notify();
                }
            }))
            .child(list)
            .child(bar(theme, self.culprits_scroll.clone(), cx.entity_id()))
            .into_any_element()
    }

    fn log_view(&mut self, theme: &Theme, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let c = theme.colours;
        let tops = self.log_tops(theme, window);
        let lines = self.lines.clone();
        if lines.is_empty() && !self.busy {
            let said = if self.filter_text.is_empty() {
                put(&say!("No log"))
            } else {
                put(&say!("No line matches"))
            };
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(label(theme, theme.body(), c.ink2, said))
                .into_any_element();
        }
        let report = self.report.clone();
        let (marked, wrap, me) = (self.marked, self.wrap_log, cx.entity().downgrade());
        let theme_owned = theme.clone();
        let draw_tops = tops.clone();
        let list = rows(
            "log-rows",
            self.log_scroll.clone(),
            tops.clone(),
            theme.sized(40.),
            cx.entity_id(),
            move |range, width, _, _| {
                let theme = &theme_owned;
                let Some(report) = &report else {
                    return Vec::new();
                };
                range
                    .filter_map(|at| {
                        let line_at = *lines.get(at)?;
                        let number = line_at + 1;
                        let text = report.text[line_at].clone();
                        let level = report.levels[line_at];
                        let bad = level == Some(rimstack_core::rimworld::logs::Level::Error);
                        let warning = level == Some(rimstack_core::rimworld::logs::Level::Warning);
                        let x = Self::log_text_x(theme);
                        let (me, copied) = (me.clone(), text.clone());
                        let mark_me = me.clone();
                        Some(
                            div()
                                .id(("log-line", at))
                                .role(Role::ListItem)
                                .relative()
                                .w(width)
                                .h(draw_tops[at + 1] - draw_tops[at])
                                .when(number == marked, |d| d.bg(with_alpha(c.accent, 0.25)))
                                .child(
                                    div()
                                        .absolute()
                                        .left(theme::GAP)
                                        .top_0()
                                        .bottom_0()
                                        .w(theme.sized(52.))
                                        .flex()
                                        .items_center()
                                        .justify_end()
                                        .child(
                                            div()
                                                .sized_text(theme, theme.small())
                                                .font_family(theme.mono.clone())
                                                .text_color(c.ink3)
                                                .child(number.to_string()),
                                        ),
                                )
                                .when(bad || warning, |d| {
                                    d.child(
                                        div()
                                            .absolute()
                                            .left(theme::GAP + theme.sized(52.) + theme::GAP_SMALL)
                                            .top_0()
                                            .bottom_0()
                                            .flex()
                                            .items_center()
                                            .child(
                                                div()
                                                    .size(theme.sized(16.))
                                                    .rounded(theme::CORNER)
                                                    .when(bad, |d| d.bg(c.danger_bg))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(
                                                        div()
                                                            .sized_text(theme, theme.small())
                                                            .font_family(theme.mono.clone())
                                                            .font_weight(FontWeight::BOLD)
                                                            .text_color(if bad {
                                                                c.err
                                                            } else {
                                                                c.warn
                                                            })
                                                            .child(if bad {
                                                                put(&say!("E"))
                                                            } else {
                                                                put(&say!("W"))
                                                            }),
                                                    ),
                                            ),
                                    )
                                })
                                .child(
                                    div()
                                        .absolute()
                                        .left(x)
                                        .right(theme::GAP_WIDE)
                                        .top_0()
                                        .bottom_0()
                                        .flex()
                                        .items_center()
                                        .child(
                                            div()
                                                .w_full()
                                                .sized_text(theme, theme.small())
                                                .font_family(theme.mono.clone())
                                                .text_color(if bad { c.err } else { c.ink })
                                                .when(!wrap, |d| {
                                                    d.overflow_hidden()
                                                        .whitespace_nowrap()
                                                        .text_ellipsis()
                                                })
                                                .child(if wrap { breakable(&text) } else { text }),
                                        ),
                                )
                                .on_click(move |_, _, cx| {
                                    let _ = mark_me.update(cx, |logs, cx| {
                                        logs.marked = number;
                                        cx.notify();
                                    });
                                })
                                .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                                    let copied = copied.clone();
                                    let entries: Vec<Entry> = vec![
                                        Entry::item(put(&say!("Copy line")), move |_, cx| {
                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                                copied.clone(),
                                            ))
                                        })
                                        .icon("copy")
                                        .into(),
                                    ];
                                    menu::open_at(event.position, entries, window, cx);
                                })
                                .into_any_element(),
                        )
                    })
                    .collect()
            },
        );
        let count = self.lines.len();
        let current = self
            .lines
            .iter()
            .position(|l| l + 1 == self.marked)
            .unwrap_or(0);
        let (keys_lines, keys_tops) = (self.lines.clone(), tops);
        div()
            .id("log")
            .track_focus(&self.focus.log)
            .role(Role::List)
            .aria_label(put(&say!("Log")))
            .relative()
            .flex_1()
            .min_h_0()
            .on_key_down(cx.listener(move |logs, event: &KeyDownEvent, _, cx| {
                if let Some(to) = step(event, current, count) {
                    logs.marked = keys_lines[to] + 1;
                    logs.log_scroll
                        .borrow_mut()
                        .contain(keys_tops[to], keys_tops[to + 1]);
                    cx.notify();
                }
            }))
            .child(list)
            .child(bar(theme, self.log_scroll.clone(), cx.entity_id()))
            .into_any_element()
    }

    pub(super) fn middle(
        &mut self,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = theme.colours;
        let cut = self
            .report
            .as_deref()
            .map(|r| put(&head(r).cut))
            .unwrap_or_default();
        let view = match self.view {
            View::Errors => self.errors_view(theme, window, cx),
            View::Mods => self.culprits_view(theme, cx),
            View::Log => self.log_view(theme, window, cx),
        };
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_hidden()
            .child(self.head(theme, window, cx))
            .children(self.since_box(theme, cx))
            .children(self.filter_row(theme, window, cx))
            .child(view)
            .when(self.view == View::Log && !cut.is_empty(), |d| {
                d.child(
                    label(theme, theme.small(), c.ink3, cut)
                        .pl(theme::GAP_WIDE)
                        .pb(theme::GAP_SMALL),
                )
            })
            .into_any_element()
    }
}

/// Where the arrows, Home and End take a list's current row, as `RowKeys` did.
fn step(event: &KeyDownEvent, current: usize, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    match event.keystroke.key.as_str() {
        "up" => Some(current.saturating_sub(1)),
        "down" => Some((current + 1).min(count - 1)),
        "home" => Some(0),
        "end" => Some(count - 1),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn error_row(
    theme: &Theme,
    at: usize,
    shown: &crate::workspace::logs::ErrorShown<'_>,
    current: bool,
    width: Pixels,
    height: Pixels,
    me: gpui::WeakEntity<Logs>,
    window: &Window,
    cx: &mut gpui::App,
) -> AnyElement {
    let _ = (window, cx);
    let c = &theme.colours;
    let error = shown.error;
    let hover = with_alpha(c.hover, 0.05);
    let run = error.run;
    let mod_id = error.mod_id;
    let (pick, more_me, show_me) = (me.clone(), me.clone(), me);
    let times = format!("×{}", count(error.count));
    let mod_room = theme.sized(300.).min(width * 0.36);
    div()
        .id(("error", at))
        .role(Role::ListItem)
        .aria_label(error.head.clone())
        .relative()
        .w(width)
        .h(height)
        .flex()
        .items_center()
        .gap(theme::GAP_LARGE)
        .pl(theme::GAP_PAGE)
        .pr(theme::GAP_PAGE + theme::GAP_SMALL)
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
                    .right(theme::GAP_PAGE)
                    .h(px(1.))
                    .bg(with_alpha(c.line, 0.6)),
            )
        })
        .when(error.count > 1, |d| {
            let counted = crate::ui::tip::tipped(
                div()
                    .id(("times", at))
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .min_w(theme.control())
                    .h(theme.control())
                    .px(theme::GAP_SMALL)
                    .rounded(theme::CORNER)
                    .bg(if current {
                        theme::hex("#38000000")
                    } else {
                        with_alpha(c.err, 0.16)
                    })
                    .child(
                        label(
                            theme,
                            theme.small(),
                            if current { c.on_selected } else { c.err },
                            times,
                        )
                        .font_weight(theme.semibold),
                    ),
                put(&say!("How many times the log shows it")).into(),
            );
            d.child(counted)
        })
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(theme::GAP_THIN)
                .child(
                    div()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .sized_text(theme, theme.body())
                        .font_weight(theme.medium)
                        .text_color(if current { c.on_selected } else { c.ink })
                        .child(shown.kind.to_string()),
                )
                .when(!shown.detail.is_empty(), |d| {
                    d.child(
                        div()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .sized_text(theme, theme.small())
                            .text_color(if current { c.on_selected_dim } else { c.ink2 })
                            .child(shown.detail.to_string()),
                    )
                }),
        )
        .when(shown.more > 0, |d| {
            d.child(
                Chip::new(put(&say!("{} more like it", shown.more)))
                    .on_accent(current)
                    .icon(if shown.opened {
                        "chevron-down"
                    } else {
                        "chevron-right"
                    })
                    .tip(put(&say!(
                        "Later errors that differ from this one only in their last word"
                    )))
                    .on_click(("more-like", at), move |_, _, cx| {
                        let _ = more_me.update(cx, |logs, cx| logs.toggle_run(run, cx));
                    }),
            )
        })
        .when_some(error.first.clone(), |d, name| {
            let tip = match mod_id >= 0 {
                true => put(&say!("The mod the error points at. Click to go to it.")),
                false => put(&say!("The mod the error points at")),
            };
            let chip = Chip::new(name)
                .max_width(mod_room)
                .on_accent(current)
                .tone(c.ink)
                .icon("package")
                .tip(tip);
            d.child(match mod_id >= 0 {
                true => chip.on_click(("error-mod", at), move |_, _, cx| {
                    let _ = show_me.update(cx, |_, cx| cx.emit(super::ShowMod(mod_id as i32)));
                }),
                false => chip,
            })
        })
        .on_click(move |_, _, cx| {
            let _ = pick.update(cx, |logs, cx| {
                logs.selected = at;
                cx.notify();
            });
        })
        .into_any_element()
}
