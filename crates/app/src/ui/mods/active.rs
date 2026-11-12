//! `ActivePane.qml` and `Columns.qml`: the load order, its filter and its column headings.

use super::Mods;
use super::list::bar_room;
use super::rows::Columns;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{Btn, Chip, TextSized, btn_width, icon, label, measure, notice};
use crate::words::{count, put};
use crate::workspace::load_order::Side;
use gpui::{
    Context, FontWeight, InteractiveElement, MouseButton, ParentElement, Pixels,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};
use rimstack_core::{say, say_n};
use std::rc::Rc;

impl Mods {
    fn heading_label(&self, key: &str) -> String {
        self.orders
            .active_by
            .get(key)
            .map(|entry| put(&entry.label))
            .unwrap_or_default()
    }

    /// `Columns.widths` and `ModList.fitting` for a list this wide.
    fn columns(&self, theme: &Theme, width: Pixels, window: &Window) -> Columns {
        super::columns::columns(theme, &self.orders, &self.view, width, window)
    }

    pub(super) fn active_pane(
        &mut self,
        theme: &Theme,
        width: Pixels,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let c = &theme.colours;
        let columns = Rc::new(self.columns(theme, width, window));
        let me = cx.entity().downgrade();
        let tip = put(&say!(
            "Click to order the list by this column; click again to reverse it. Right-click to choose the columns. Drag it, or press Ctrl + Alt + Left or Right, to move it."
        ));
        let heading = |key: &'static str, text: String, on: bool| {
            let (pressed, asked) = (me.clone(), me.clone());
            let said = match key {
                "" => put(&say!("The load order itself")),
                _ => tip.clone(),
            };
            crate::ui::tip::tipped(
                div()
                    .id(gpui::SharedString::from(format!("heading-{key}")))
                    .cursor_pointer()
                    .child(
                        label(
                            theme,
                            theme.caption(),
                            if on { c.ink } else { c.ink3 },
                            text,
                        )
                        .font_weight(theme.medium),
                    )
                    .on_click(move |_, _, cx| {
                        let _ = pressed.update(cx, |mods, cx| mods.heading_pressed(key, cx));
                    })
                    .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                        let at = event.position;
                        let _ = asked.update(cx, |mods, cx| mods.columns_menu(at, window, cx));
                    }),
                said.into(),
            )
        };
        let name_on = self.orders.active_by.get("name").is_some_and(|e| e.on);
        let view_menu = cx.entity().downgrade();
        let headings = div()
            .id("headings")
            .relative()
            .flex_none()
            .h(theme.control())
            .on_drag_move::<super::columns::ColumnEdge>(cx.listener(
                |mods, event: &gpui::DragMoveEvent<super::columns::ColumnEdge>, _, cx| {
                    let key = event.drag(cx).0;
                    mods.size_column(key, event.event.position.x, cx);
                },
            ))
            .on_drop(
                cx.listener(|mods, edge: &super::columns::ColumnEdge, _, cx| {
                    mods.column_drag = None;
                    let _ = (edge, cx);
                }),
            )
            .on_drop(
                cx.listener(|mods, held: &super::columns::ColumnHeld, window, cx| {
                    mods.drop_column(*held, window.mouse_position().x, cx);
                }),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .pl(theme::GAP_WIDE)
                    .pr(theme::GAP_WIDE + bar_room(theme))
                    .gap(theme::GAP_LARGE)
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .justify_end()
                            .w(columns.number)
                            .child(heading("", "#".into(), false)),
                    )
                    .child(div().flex_1().min_w_0().child(heading(
                        "name",
                        self.heading_label("name"),
                        name_on,
                    )))
                    .children(
                        columns
                            .fitting
                            .iter()
                            .enumerate()
                            .map(|(at, (key, width))| {
                                let on = self.orders.active_by.get(key).is_some_and(|e| e.on);
                                let (key, width) = (*key, *width);
                                div()
                                    .id(gpui::SharedString::from(format!("column-{key}")))
                                    .relative()
                                    .flex()
                                    .flex_none()
                                    .h_full()
                                    .items_center()
                                    .w(width)
                                    .when(matches!(key, "load" | "size"), |d| d.justify_end())
                                    .child(super::columns::edge(theme, key, width, cx))
                                    .child(heading(key, self.heading_label(key), on))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            move |mods, event: &gpui::MouseDownEvent, _, _| {
                                                mods.column_drag = Some((event.position.x, width));
                                            },
                                        ),
                                    )
                                    .on_drag(super::columns::ColumnHeld(key, at), |_, _, _, cx| {
                                        gpui::AppContext::new(cx, |_| super::rows::Ghost)
                                    })
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .justify_end()
                            .w(columns.quick)
                            .child(super::columns::placed(
                                Btn::new("view", put(&say!("View"))).opens(
                                    move |below, window, cx| {
                                        let _ = view_menu.update(cx, |mods, cx| {
                                            mods.view_menu(below, window, cx)
                                        });
                                    },
                                ),
                                self.menu_buttons.view.clone(),
                            )),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .bottom_0()
                    .left(theme::GAP_SMALL)
                    .right(theme::GAP_SMALL)
                    .h(px(1.))
                    .bg(c.line),
            );
        div()
            .flex()
            .flex_col()
            .h_full()
            .child(self.head(theme, width, window, cx))
            .children(self.notices(theme, width, window, cx))
            .children(self.bands(theme, cx))
            .child(
                div()
                    .flex_none()
                    .px(theme::GAP_LARGE)
                    .pb(theme::GAP)
                    .child(self.filter_field(theme, window, cx)),
            )
            .child(headings)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(self.list(Side::Active).clone()),
            )
    }

    /// The pane's title row: the count, the problems, Undo and Redo, Sort and Save.
    fn head(
        &self,
        theme: &Theme,
        width: Pixels,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let c = &theme.colours;
        let app = crate::ui::app(cx);
        let (errors, warnings) = self.order.errors_and_warnings();
        let problems = self
            .order
            .troubles()
            .iter()
            .filter(|t| !t.is_empty())
            .count();
        let previewing = self.order.previewing();
        let (version_only, other_warnings) = self
            .overview
            .as_ref()
            .map_or((0, 0), |o| (o.version_only, o.other_warnings));
        let version_line = version_only >= 3 && !previewing && !self.demo;
        let warnings_shown = if version_line {
            other_warnings
        } else {
            warnings
        };
        let (dirty, can_undo, can_redo) = match self.demo {
            true => (false, false, false),
            false => {
                let session = app.session();
                (session.is_dirty(), session.can_undo(), session.can_redo())
            }
        };
        let title = put(&say!("Active"));
        let counted = count(self.count(Side::Active));
        let sort = match (self.files_read, self.sort_pending) {
            (false, _) => put(&say!("Reading files…")),
            (true, 0) => put(&say!("Sort")),
            (true, n) => put(&say_n!(n, "Sort · {} move", "Sort · {} moves")),
        };
        let save = put(&say!("Save"));
        let full = |text: &str| {
            measure(window, theme, text, theme.small(), theme.medium)
                + theme.icon()
                + theme::GAP_SMALL
                + theme.sized(20.)
        };
        let head_wide = theme::GAP_CARD
            + theme::GAP_LARGE
            + measure(window, theme, &title, theme.heading(), FontWeight::BOLD)
            + measure(window, theme, &counted, theme.body(), FontWeight::NORMAL)
            + full(&sort)
            + full(&save)
            + theme::GAP * 4.;
        let warned = match version_line {
            true => put(&say_n!(
                warnings_shown,
                "{} more warning",
                "{} more warnings"
            )),
            false => put(&say_n!(warnings_shown, "{} warning", "{} warnings")),
        };
        let only_warnings = errors == 0;
        let problems_said = match only_warnings {
            true => put(&say_n!(problems, "{} warning", "{} warnings")),
            false => put(&say_n!(
                problems,
                "{} problem · {} with errors",
                "{} problems · {} with errors",
                count(errors)
            )),
        };
        let problems_button = !(only_warnings && warnings_shown > 0 && warnings_shown == problems);
        let chip_width = |text: &str| {
            theme.sized(11.)
                + theme::GAP_TINY
                + measure(window, theme, text, theme.caption(), theme.medium)
                + theme::GAP_CARD
        };
        let icon_button = theme.icon() + theme.sized(14.);
        let extras = if warnings_shown > 0 {
            chip_width(&warned) + theme::GAP
        } else {
            px(0.)
        } + if problems > 0 {
            let said = if problems_button {
                btn_width(window, theme, &problems_said, true, false) + theme::GAP_LINE
            } else {
                px(0.)
            };
            said + icon_button * 2. + theme::GAP_LINE + theme::GAP
        } else {
            px(0.)
        } + if self.demo {
            px(0.)
        } else {
            icon_button * 2. + theme::GAP_LINE + theme::GAP
        };
        let updates = put(&say_n!(self.behind, "{} update", "{} updates"));
        let extras = extras
            + if self.behind > 0 {
                btn_width(window, theme, &updates, false, false) + theme::GAP
            } else {
                px(0.)
            };
        let tight = width < head_wide + extras;
        let problem_tone = if errors > 0 { c.err } else { c.warn };
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(theme::GAP)
            .h(theme.header())
            .pl(theme::GAP_CARD)
            .pr(theme::GAP_LARGE)
            .child(label(theme, theme.heading(), c.ink, title).font_weight(FontWeight::BOLD))
            .child(label(theme, theme.body(), c.ink3, counted))
            .when(warnings_shown > 0 && !tight, |d| {
                d.child(
                    Chip::new(warned)
                        .tone(c.warn)
                        .icon("triangle-alert")
                        .tip(put(&say!("Mods with a warning: the game runs, but parts of these mods may not work. Click to show only these.")))
                        .on_click("warnings-chip", cx.listener(|m, _, _, cx| m.toggle_term("is:warning", cx))),
                )
            })
            .when(problems > 0 && !tight, |d| {
                d.child(
                    div()
                        .flex()
                        .gap(theme::GAP_LINE)
                        .when(problems_button, |d| {
                            d.child(
                                Btn::new("problems", problems_said)
                                    .icon("triangle-alert")
                                    .tone(problem_tone),
                            )
                        })
                        .child(
                            Btn::new("problem-up", "")
                                .icon("chevron-up")
                                .spoken(put(&say!("Previous problem")))
                                .tip(put(&say!("Previous problem")))
                                .on_click(cx.listener(|m, _, _, cx| m.show_problem(false, cx))),
                        )
                        .child(
                            Btn::new("problem-down", "")
                                .icon("chevron-down")
                                .spoken(put(&say!("Next problem")))
                                .tip(put(&say!("Next problem")))
                                .on_click(cx.listener(|m, _, _, cx| m.show_problem(true, cx))),
                        ),
                )
            })
            .child(div().flex_1())
            .when(self.behind > 0 && !tight, |d| {
                d.child(
                    Btn::new("updates", updates)
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(super::ShowSection(1)))),
                )
            })
            .when(!self.demo && !tight, |d| {
                d.child(
                    div()
                        .flex()
                        .gap(theme::GAP_LINE)
                        .child(
                            Btn::new("undo", "")
                                .icon("undo-2")
                                .tip(put(&say!("Undo (Ctrl + Z)")))
                                .enabled(can_undo && !previewing)
                                .on_click(cx.listener(|m, _, _, cx| m.undo(cx))),
                        )
                        .child(
                            Btn::new("redo", "")
                                .icon("redo-2")
                                .tip(put(&say!("Redo (Ctrl + Shift + Z)")))
                                .enabled(can_redo && !previewing)
                                .on_click(cx.listener(|m, _, _, cx| m.redo(cx))),
                        ),
                )
            })
            .when(!self.demo, |d| {
                d.child(
                    Btn::new("sort", sort)
                        .icon("arrow-up-down")
                        .tone(c.moved)
                        .tip(put(&say!("Shows the order the rules ask for, to keep or discard. Nothing is written before Save.")))
                        .enabled(!previewing && self.files_read)
                        .on_click(cx.listener(|m, _, _, cx| m.preview_sort(cx))),
                )
                .child(
                    Btn::new("save", save)
                        .icon("save")
                        .tip(put(&say!("Writes the load order to ModsConfig.xml (Ctrl + S). The file as it was is copied beside it first.")))
                        .enabled(dirty)
                        .primary_if(dirty)
                        .on_click(cx.listener(|m, _, _, cx| m.save(None, cx))),
                )
            })
            .when(tight && !self.demo, |d| {
                let more = Btn::new(
                    "more",
                    if problems > 0 {
                        count(problems)
                    } else {
                        String::new()
                    },
                )
                .icon("ellipsis")
                .menu();
                d.child(if problems > 0 {
                    more.tone(problem_tone)
                } else {
                    more
                })
            })
    }

    /// The bands under the title: what is not saved, and mods made for another game version.
    fn notices(
        &self,
        theme: &Theme,
        width: Pixels,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::Div> {
        let c = &theme.colours;
        let mut shown = Vec::new();
        if self.demo {
            return shown;
        }
        let version_only = self.overview.as_ref().map_or(0, |o| o.version_only);
        if version_only >= 3 && !self.order.previewing() {
            let version = self.game_version();
            let said = put(&say_n!(
                version_only,
                "{} mod does not list RimWorld {}",
                "{} mods do not list RimWorld {}",
                version.clone()
            ));
            let show = put(&say!("Show"));
            let ignore = put(&say!("Ignore for {}", version));
            let does = btn_width(window, theme, &show, false, false)
                + theme::GAP
                + btn_width(window, theme, &ignore, false, false);
            let room = width - theme::GAP_LARGE * 3. - theme::GAP_TINY;
            let stacked = theme.icon()
                + measure(window, theme, &said, theme.body(), FontWeight::NORMAL)
                + does
                + theme::GAP * 2.
                > room;
            let buttons = div()
                .flex()
                .flex_none()
                .justify_end()
                .gap(theme::GAP)
                .child(
                    Btn::new("show-warnings", show)
                        .tip(put(&say!("Only the mods with a warning")))
                        .on_click(cx.listener(|m, _, _, cx| m.toggle_term("is:warning", cx))),
                )
                .child(
                    Btn::new("ignore-version", ignore)
                        .tip(put(&say!("No warning about the game version while RimWorld stays on this version. The switch is in Settings, Sorting.")))
                        .on_click(cx.listener(move |m, _, _, cx| {
                            let version = m.game_version();
                            m.set_version_aside(version, cx)
                        })),
                );
            shown.push(
                div().flex_none().px(theme::GAP_LARGE).pb(theme::GAP).child(
                    notice(c.warn).child(
                        div()
                            .flex()
                            .when(stacked, |d| d.flex_col().gap(theme::GAP_SMALL))
                            .when(!stacked, |d| d.items_center().gap(theme::GAP))
                            .pl(theme::GAP_LARGE)
                            .pr(theme::GAP_TINY)
                            .py(theme::GAP_SMALL)
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(theme::GAP)
                                    .child(icon("triangle-alert", theme.icon(), c.warn).mt(px(2.)))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .sized_text(theme, theme.body())
                                            .text_color(c.ink)
                                            .child(said),
                                    ),
                            )
                            .child(buttons),
                    ),
                ),
            );
        }
        shown
    }

    /// `ModsFilter.qml` and `FilterField.qml`: the filter, its count of what it lets through, and its clear button.
    fn filter_field(
        &self,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let typed = !self.filter.read(cx).text().is_empty();
        crate::ui::field::Field {
            id: "filter",
            text: &self.filter,
            icon: "funnel",
            placeholder: put(&say!("Filter active and inactive mods")),
            found: typed.then(|| put(&self.order.filter_summary())),
            keys: Some(put(&say!("Ctrl + F"))),
        }
        .render(theme, window, cx)
    }
}
