//! The pieces `SettingsScreen.qml` lays each setting out with: `Line` (its name, what sets it,
//! the way back to the default, the words under it), `Folder`, `GroupTitle` and `PathText`.

use super::Settings;
use crate::ui::field::Field;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{Btn, TextSized, elide_middle_in, measure_in};
use crate::words::put;
use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Pixels,
    SharedString, Styled, Window, div, prelude::FluentBuilder,
};
use rimstack_core::say;
use std::rc::Rc;

pub(super) type Reset = Rc<dyn Fn(&mut Settings, &mut Window, &mut Context<Settings>)>;

/// One setting.
pub(super) struct Line {
    pub part: usize,
    pub id: &'static str,
    pub title: String,
    pub tip: String,
    pub changed: bool,
    pub reset: Option<Reset>,
    pub held: Vec<AnyElement>,
}

impl Line {
    pub fn new(part: usize, id: &'static str, title: String) -> Self {
        Self {
            part,
            id,
            title,
            tip: String::new(),
            changed: false,
            reset: None,
            held: Vec::new(),
        }
    }

    pub fn tip(mut self, tip: String) -> Self {
        self.tip = tip;
        self
    }

    /// Not at its default: marked, and `reset` brings it back.
    pub fn changed(
        mut self,
        changed: bool,
        reset: impl Fn(&mut Settings, &mut Window, &mut Context<Settings>) + 'static,
    ) -> Self {
        self.changed = changed;
        self.reset = Some(Rc::new(reset));
        self
    }

    pub fn held(mut self, held: impl IntoElement) -> Self {
        self.held.push(held.into_any_element());
        self
    }

    /// What follows the controls: the room left in the row.
    pub fn filled(self) -> Self {
        self.held(div().flex_1())
    }
}

pub(super) enum Row {
    /// A group's title, hidden while a search runs.
    Group(usize, String),
    /// Words under a group's title, hidden while a search runs.
    Note(usize, String),
    Line(Line),
    /// Something under the lines, shown with its part.
    Under(usize, AnyElement),
}

/// The width of the names at the left of the lines.
pub(super) fn named(theme: &Theme) -> Pixels {
    theme.sized(190.)
}

/// `GroupTitle`: a defined term is underlined with dots, with an info mark, and means its tip.
pub(super) fn group(theme: &Theme, title: String) -> AnyElement {
    crate::ui::card::group_title(theme, title)
}

pub(super) fn note(theme: &Theme, text: String) -> AnyElement {
    div()
        .pb(theme::GAP_SMALL)
        .sized_text(theme, theme.small())
        .text_color(theme.colours.ink3)
        .child(text)
        .into_any_element()
}

/// Words in the room of a line's controls, under them, as the error or the note.
pub(super) fn under(theme: &Theme, colour: gpui::Hsla, size: Pixels, text: String) -> AnyElement {
    div()
        .pl(named(theme) + theme::GAP)
        .sized_text(theme, size)
        .text_color(colour)
        .child(text)
        .into_any_element()
}

/// A text that takes the room left beside the controls and wraps.
pub(super) fn words(
    theme: &Theme,
    colour: gpui::Hsla,
    text: impl Into<SharedString>,
) -> AnyElement {
    div()
        .flex_1()
        .min_w_0()
        .sized_text(theme, theme.body())
        .text_color(colour)
        .child(text.into())
        .into_any_element()
}

impl Settings {
    pub(super) fn line(&self, line: Line, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let c = &theme.colours;
        let name = div()
            .flex()
            .flex_none()
            .self_start()
            .items_center()
            .gap(theme::GAP_SMALL)
            .w(named(theme))
            .pt(theme::GAP_TINY)
            .when(line.changed, |d| {
                d.child(crate::ui::tip::tipped(
                    div()
                        .id("not-the-default")
                        .flex_none()
                        .size(gpui::px(6.))
                        .rounded(gpui::px(3.))
                        .bg(c.moved),
                    crate::words::put(&rimstack_core::say!("Not the default")).into(),
                ))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .sized_text(theme, theme.body())
                    .text_color(c.ink)
                    .child(line.title.clone()),
            );
        let reset = line.reset.clone().filter(|_| line.changed);
        div()
            .flex()
            .flex_col()
            .gap(theme::GAP_LINE)
            .pb(theme::GAP_SMALL)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(theme::GAP)
                    .child(name)
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .min_h(theme.sized(28.))
                            .items_center()
                            .gap(theme::GAP)
                            .children(line.held),
                    )
                    .when_some(reset, |d, reset| {
                        d.child(
                            Btn::new((line.id, 1usize), "")
                                .icon("undo-2")
                                .tip(put(&say!("Back to the default")))
                                .on_click(cx.listener(move |settings, _, window, cx| {
                                    reset(settings, window, cx)
                                })),
                        )
                    }),
            )
            .when(!line.tip.is_empty(), |d| {
                d.child(
                    div()
                        .pl(named(theme) + theme::GAP)
                        .spaced_text(theme, theme.small(), 1.2)
                        .text_color(c.ink3)
                        .child(line.tip),
                )
            })
            .into_any_element()
    }

    /// `Folder`: the folder in use or typed by hand, the way to open it, and what was found.
    pub(super) fn folder(
        &self,
        at: usize,
        title: String,
        note: String,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Line {
        let c = &theme.colours;
        let name = super::FOLDERS[at];
        let report = &self.report;
        let (in_use, by_hand) = match at {
            0 => (&report.game, &report.set_game),
            1 => (&report.config, &report.set_config),
            _ => (&report.workshop, &report.set_workshop),
        };
        let in_use = in_use.clone().unwrap_or_default();
        let field = Field {
            id: ["folder-game", "folder-config", "folder-workshop"][at],
            text: &self.fields.folders[at],
            icon: "",
            placeholder: if in_use.is_empty() {
                put(&say!("Not found"))
            } else {
                in_use.clone()
            },
            found: None,
            keys: None,
        };
        let said = match self.folder_checks.get(name) {
            Some(verdict) => (if verdict.ok { c.ok } else { c.err }, put(&verdict.said)),
            None => {
                let how = match (in_use.is_empty(), by_hand.is_some()) {
                    (true, _) => put(&say!("Not found")),
                    (false, true) => put(&say!("Set by hand")),
                    (false, false) => put(&say!("Found automatically")),
                };
                let said = [how, note]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(", ");
                (if in_use.is_empty() { c.err } else { c.ink3 }, said)
            }
        };
        let open = in_use.clone();
        let column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap(theme::GAP_THIN)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(theme::GAP)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(field.render(theme, window, cx)),
                    )
                    .child(
                        Btn::new(("open-folder", at), "")
                            .icon("folder-open")
                            .tip(put(&say!("Open folder")))
                            .enabled(!in_use.is_empty())
                            .on_click(move |_, _, cx| {
                                cx.open_with_system(std::path::Path::new(&open))
                            }),
                    ),
            )
            .child(
                div()
                    .sized_text(theme, theme.small())
                    .text_color(said.0)
                    .child(said.1),
            );
        Line::new(
            0,
            ["folder-game", "folder-config", "folder-workshop"][at],
            title,
        )
        .held(column)
    }
}

/// `PathText`: a path on one line, cut in its middle, with the ways to copy and open it.
pub(super) fn path(
    id: &'static str,
    path: String,
    tone: gpui::Hsla,
    room: Pixels,
    open: Option<(&'static str, String)>,
    theme: &Theme,
    window: &Window,
) -> AnyElement {
    let size = theme.small();
    let buttons = (theme.icon() + theme.sized(14.) + theme::GAP_SMALL)
        * (1 + usize::from(open.is_some())) as f32;
    let shown = elide_middle_in(
        window,
        theme,
        theme.mono.clone(),
        &path,
        size,
        room - buttons,
    );
    let cut = measure_in(
        window,
        theme,
        theme.mono.clone(),
        &path,
        size,
        FontWeight::NORMAL,
    ) > room - buttons;
    let (copied, opened) = (path.clone(), path.clone());
    let text = div()
        .id((id, 0usize))
        .flex_1()
        .min_w_0()
        .whitespace_nowrap()
        .overflow_hidden()
        .sized_text(theme, size)
        .font_family(theme.mono.clone())
        .text_color(tone)
        .child(shown);
    let text = crate::ui::tip::tipped(
        text,
        if cut {
            path.clone().into()
        } else {
            SharedString::default()
        },
    );
    div()
        .flex()
        .flex_1()
        .min_w_0()
        .items_center()
        .gap(theme::GAP_SMALL)
        .child(text)
        .when(!path.is_empty(), |d| {
            d.child(
                Btn::new((id, 1usize), "")
                    .icon("copy")
                    .tip(put(&say!("Copy path")))
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(copied.clone()))
                    }),
            )
        })
        .when_some(open, |d, (icon, tip)| {
            d.child(
                Btn::new((id, 2usize), "")
                    .icon(icon)
                    .tip(tip)
                    .enabled(!path.is_empty())
                    .on_click(move |_, _, cx| cx.open_with_system(std::path::Path::new(&opened))),
            )
        })
        .into_any_element()
}
