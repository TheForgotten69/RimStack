//! `ModHeader.qml`, `Choice.qml` and the tabs under it: the mod chosen in a list.

use crate::ui::card::Fold;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{Btn, Chip, TextSized, icon, label, measure};
use crate::words::put;
use crate::workspace::load_order::{Details, Overview};
use gpui::{
    Context, FontWeight, InteractiveElement, IntoElement, ObjectFit, ParentElement, Pixels,
    StatefulInteractiveElement, Styled, StyledImage, Window, div, img, prelude::FluentBuilder, px,
};
use rimstack_core::say;
use std::rc::Rc;

/// What the details show, handed over by the Mods screen when it changes.
#[derive(Clone, Default)]
pub struct DetailsShown {
    pub demo: bool,
    pub active: usize,
    pub inactive: usize,
    pub game_version: String,
    pub overview: Option<Rc<Overview>>,
    pub details: Option<Rc<Details>>,
    pub summary: Option<Rc<crate::reports::Summary>>,
    pub last_start: Option<Rc<crate::workspace::logs::LastStartSaid>>,
    pub warnings: usize,
    pub troubled: bool,
    pub dirty: bool,
    pub unsaved: String,
    pub conflicts: usize,
    pub changed_since: usize,
    /// The active mods with a newer Workshop version.
    pub behind: usize,
    pub selected_active: bool,
    /// What `pane_widths` is given.
    pub panes: super::panes::Panes,
    /// The problems panel shown in place of the rest.
    pub problems_open: bool,
    pub picking: Option<super::notes::Picking>,
    /// The chosen mod's place in the load order, from 1; 0 off it.
    pub position: usize,
    pub notes_saved: bool,
    pub files: Option<Rc<FilesRead>>,
    pub unhide_problem: String,
    /// Two mods' shared textures being compared: the details tell what was kept.
    pub comparison: Option<Rc<crate::workspace::textures::Comparison>>,
    /// The search for the broken mod runs: it is not offered again.
    pub hunting: bool,
    pub previewing: bool,
    /// With several mods chosen, how many and what they are.
    pub selected: usize,
    pub selection: Option<Rc<crate::workspace::load_order::SelectionSummary>>,
}

/// What the Files tab shows: the textures, the files by folder, and the textures set aside.
pub type FilesRead = (
    crate::workspace::load_order::Textures,
    crate::workspace::load_order::Files,
    usize,
);

/// What the details ask of the Mods tab.
pub enum Asked {
    Section(usize),
    Filter(&'static str),
    /// The tab shown changed.
    Tab(usize),
}

impl gpui::EventEmitter<Asked> for DetailsView {}

/// The details pane, a view of its own: choosing a mod or a tab draws nothing else.
pub struct DetailsView {
    pub shown: DetailsShown,
    pub tab: usize,
    /// The cards folded against how they start, by their key, as `Theme.folded` kept them.
    pub folded: std::collections::HashSet<&'static str>,
    /// The names of the mods chosen all shown, not the first ones only.
    pub summary_open: bool,
    pub mods: gpui::WeakEntity<super::Mods>,
    /// The About tab's fields: the tags, the group and the note of the mod shown.
    pub(super) tags: gpui::Entity<crate::ui::text_field::TextField>,
    pub(super) group: gpui::Entity<crate::ui::text_field::TextField>,
    pub(super) note: gpui::Entity<crate::ui::text_field::TextField>,
    /// The picture shown small, as a click on it asks.
    pub(super) small_picture: bool,
}

impl DetailsView {
    pub fn new(mods: gpui::WeakEntity<super::Mods>, cx: &mut Context<Self>) -> Self {
        use crate::ui::text_field::{Edited, Entered, TextField};
        use gpui::AppContext;
        let size = cx.global::<Theme>().body();
        let field = |cx: &mut Context<Self>| cx.new(|cx| TextField::new(size, cx).kept_on_escape());
        let (tags, group) = (field(cx), field(cx));
        let note = cx.new(|cx| TextField::new(size, cx).several_lines(2));
        for noted in [&tags, &note] {
            cx.subscribe(noted, |view, _, _: &Edited, cx| view.draft_note(cx))
                .detach();
            cx.subscribe(noted, |view, _, _: &Entered, cx| view.keep_drafts(cx))
                .detach();
        }
        cx.subscribe(&group, |view, field, _: &Edited, cx| {
            if field.read(cx).is_focused_now() {
                let name = field.read(cx).text().to_string();
                let _ = view.mods.update(cx, |mods, cx| mods.draft_group(name, cx));
            }
        })
        .detach();
        cx.subscribe(&group, |view, _, _: &Entered, cx| view.keep_drafts(cx))
            .detach();
        Self {
            shown: Default::default(),
            tab: 0,
            folded: Default::default(),
            summary_open: false,
            mods,
            tags,
            group,
            note,
            small_picture: false,
        }
    }

    /// The tags and the note typed, kept once typing stops.
    fn draft_note(&mut self, cx: &mut Context<Self>) {
        let typing = self.tags.read(cx).is_focused_now() || self.note.read(cx).is_focused_now();
        if !typing {
            return;
        }
        let (tags, note) = (
            self.tags.read(cx).text().to_string(),
            self.note.read(cx).text().to_string(),
        );
        let _ = self
            .mods
            .update(cx, |mods, cx| mods.draft_note(tags, note, cx));
    }

    fn keep_drafts(&mut self, cx: &mut Context<Self>) {
        let _ = self.mods.update(cx, |mods, cx| mods.keep_drafts(cx));
    }

    /// The fields hold what the mod shown has, unless they are being typed in.
    fn sync_fields(&mut self, cx: &mut Context<Self>) {
        let Some(details) = self.shown.details.clone() else {
            return;
        };
        for (field, text) in [
            (&self.tags, &details.tags),
            (&self.group, &details.sub),
            (&self.note, &details.note),
        ] {
            let typing = field.read(cx).is_focused_now();
            if !typing && field.read(cx).text() != text.as_str() {
                let text = text.clone();
                field.update(cx, |field, cx| field.set_text_quietly(&text, cx));
            }
        }
    }

    /// Whether the card `key` is folded, and what folds or opens it.
    pub(super) fn fold(
        &self,
        key: &'static str,
        starts_folded: bool,
        cx: &mut Context<Self>,
    ) -> (bool, Fold) {
        let folded = self.folded.contains(key) != starts_folded;
        let fold = cx.listener(move |view: &mut Self, _, _, cx| {
            if !view.folded.remove(key) {
                view.folded.insert(key);
            }
            cx.notify();
        });
        (folded, Box::new(fold))
    }

    pub fn show(&mut self, shown: DetailsShown, cx: &mut Context<Self>) {
        let other = self.shown.details.as_ref().map(|d| d.path.clone())
            != shown.details.as_ref().map(|d| d.path.clone());
        if other {
            self.small_picture = false;
        }
        self.shown = shown;
        self.sync_fields(cx);
        cx.notify();
    }
}

impl gpui::Render for DetailsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let (_, _, width) =
            super::pane_widths(&theme, window.viewport_size().width, self.shown.panes);
        self.details_pane(&theme, width, window, cx)
    }
}

fn tabs() -> [String; 3] {
    [
        put(&say!("About")),
        put(&say!("Load order")),
        put(&say!("Files")),
    ]
}

/// `CopyText` and `Text` in a column of the details: words that wrap.
fn words(
    theme: &Theme,
    size: Pixels,
    colour: gpui::Hsla,
    text: impl Into<gpui::SharedString>,
) -> gpui::Div {
    div()
        .w_full()
        .sized_text(theme, size)
        .text_color(colour)
        .child(text.into())
}

/// `Defined.qml`: a term of the details, underlined with dots when it has a meaning to show.
pub(super) fn defined(theme: &Theme, term: String) -> gpui::Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(theme::GAP_SMALL)
        .child(
            label(theme, theme.body(), theme.colours.ink2, term)
                .border_b_1()
                .border_dashed()
                .border_color(theme.colours.ink3),
        )
        .child(icon("info", theme.sized(12.), theme.colours.ink3))
}

impl DetailsView {
    pub(super) fn chosen_pane(
        &self,
        theme: &Theme,
        details: &Details,
        width: Pixels,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let c = &theme.colours;
        let inner = width - theme::GAP_CARD * 2. - theme.sized(9.) - theme::GAP_HAIR * 2.;
        let tile = || {
            let first: String = details
                .name
                .chars()
                .next()
                .map(|c| c.to_uppercase().collect())
                .unwrap_or("?".into());
            div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(theme.sized(64.))
                .rounded(theme::CORNER_LARGE)
                .bg(c.well)
                .child(label(theme, theme.display(), c.brand, first).font_weight(theme.semibold))
        };
        let large = details.preview.as_ref();
        let made_for_warns = details.made_for.is_some() && !details.version_ok && !details.official;
        let chips = div()
            .flex()
            .flex_wrap()
            .pt(theme::GAP_THIN)
            .gap(theme::GAP_TINY)
            .when(!details.source.is_empty(), |d| {
                let said = put(&rimstack_core::base::sentence::Sentence::said(
                    details.source,
                    Vec::new(),
                ));
                d.child(
                    Chip::new(said)
                        .icon(if details.workshop {
                            "download"
                        } else {
                            "hard-drive"
                        })
                        .max_width(inner),
                )
            })
            .when(!details.repository.is_empty() && !details.workshop, |d| {
                d.child(Chip::new(put(&say!("Git"))).icon("git-branch"))
            })
            .when(!details.version.is_empty(), |d| {
                d.child(Chip::new(details.version.clone()).max_width(inner))
            })
            .when(!details.game_versions.is_empty(), |d| {
                let chip = Chip::new(put(&say!("RimWorld {}", details.game_versions.clone())))
                    .max_width(inner);
                d.child(if made_for_warns {
                    chip.tone(c.warn)
                } else {
                    chip
                })
            })
            .when_some(details.megabytes, |d, mb| {
                let shown = if mb < 10. {
                    format!("{mb:.1}")
                } else {
                    format!("{}", mb.round())
                };
                d.child(Chip::new(put(&say!("{} MB", shown))))
            });
        let active = self.shown.selected_active;
        let toggle = if active {
            put(&say!("Deactivate"))
        } else {
            put(&say!("Activate"))
        };
        let (open, page) = (put(&say!("Open folder")), put(&say!("Workshop page")));
        // As Qt counts it: the Workshop button's room even when it is not shown.
        let full = |text: &str| {
            measure(window, theme, text, theme.small(), FontWeight::NORMAL)
                + theme.icon()
                + theme::GAP_SMALL
                + theme.sized(20.)
        };
        let narrow = inner < full(&toggle) + full(&open) + full(&page) + theme::GAP_SMALL * 2.;
        let tab = self.tab;
        let names_of_tabs = tabs();
        let all_words = names_of_tabs.iter().fold(px(0.), |all, name| {
            all + measure(window, theme, name, theme.small(), theme.semibold)
        });
        let share = (inner - px(4.) - all_words) / names_of_tabs.len() as f32;
        let choice = div()
            .p(px(2.))
            .rounded(theme::CORNER)
            .bg(c.seg)
            .w_full()
            .child(
                div()
                    .flex()
                    .children(names_of_tabs.into_iter().enumerate().map(|(at, name)| {
                        let on = at == tab;
                        let weight = if on {
                            theme.semibold
                        } else {
                            FontWeight::NORMAL
                        };
                        let wide =
                            measure(window, theme, &name, theme.small(), theme.semibold) + share;
                        div()
                            .id(("tab", at))
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(wide)
                            .h(theme.segment())
                            .rounded(theme::CORNER - px(1.))
                            .when(on, |d| d.bg(c.ctl_hi))
                            .child(
                                label(theme, theme.small(), if on { c.ink } else { c.ink2 }, name)
                                    .font_weight(weight),
                            )
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.tab = at;
                                cx.emit(Asked::Tab(at));
                                cx.notify();
                            }))
                    })),
            );
        let header = div()
            .flex()
            .flex_col()
            .when_some(large, |d, path| {
                let tall = (inner * (9. / 16.)).min(theme.sized(220.));
                d.child(
                    div()
                        .w_full()
                        .h(tall)
                        .rounded(theme::CORNER_LARGE)
                        .bg(c.well)
                        .overflow_hidden()
                        .child(
                            img(std::path::PathBuf::from(path))
                                .w(inner)
                                .h(tall)
                                // The picture's own ratio would otherwise set the height.
                                .aspect_ratio(inner / tall)
                                .object_fit(ObjectFit::Contain),
                        ),
                )
                .child(div().h(theme::GAP_MEDIUM))
            })
            .child(
                div()
                    .flex()
                    .gap(theme::GAP_LARGE)
                    .when(large.is_none(), |d| d.child(tile()))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap(theme::GAP_THIN)
                            .child(
                                words(theme, theme.heading(), c.ink, details.name.clone())
                                    .font_weight(FontWeight::BOLD),
                            )
                            .child(
                                div()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .sized_text(theme, theme.body())
                                    .text_color(c.ink2)
                                    .child(details.author.clone()),
                            )
                            .child(chips),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .pt(theme::GAP_LARGE)
                    .gap(theme::GAP_SMALL)
                    .when(!self.shown.previewing, |d| {
                        d.child(
                            Btn::new("toggle", toggle)
                                .icon(if active { "minus" } else { "plus" })
                                .primary_if(!active)
                                .on_click(self.on_mods(|mods, cx| mods.toggle_selected(cx))),
                        )
                    })
                    .child({
                        let path = details.path.clone();
                        Btn::new(
                            "open-folder",
                            if narrow { String::new() } else { open.clone() },
                        )
                        .icon("folder-open")
                        .tip(open)
                        .on_click(move |_, _, cx| cx.open_with_system(std::path::Path::new(&path)))
                    })
                    .when(details.workshop, |d| {
                        let folder = details.folder.clone();
                        d.child(
                            Btn::new(
                                "workshop-page",
                                if narrow { String::new() } else { page.clone() },
                            )
                            .icon("external-link")
                            .tip(page)
                            .on_click(move |_, _, cx| {
                                cx.open_url(&crate::ui::workshop_page(&folder))
                            }),
                        )
                    })
                    .when(!details.workshop && !details.repository.is_empty(), |d| {
                        let repository = details.repository.clone();
                        d.child(
                            Btn::new("repository", put(&say!("Repository page")))
                                .icon("git-branch")
                                .tip(repository.clone())
                                .on_click(move |_, _, cx| cx.open_url(&repository)),
                        )
                    }),
            )
            .child(div().h(theme::GAP_LARGE))
            .child(choice);
        let body = match tab {
            0 => div()
                .flex()
                .flex_col()
                .pt(theme::GAP_LARGE)
                .gap(theme::GAP_CARD)
                .children(self.about(theme, details, inner, window, cx)),
            1 => div()
                .flex()
                .flex_col()
                .children(self.placed(theme, details, inner, cx)),
            _ => div()
                .flex()
                .flex_col()
                .children(self.files(theme, details, inner, window, cx)),
        };
        div().flex().flex_col().w_full().child(header).child(body)
    }
}
