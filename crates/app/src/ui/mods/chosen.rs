//! `ModHeader.qml`, `Choice.qml` and the tabs under it: the mod chosen in a list.

use crate::ui::card::Card;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{Btn, Chip, TextSized, elide_middle_in, icon, label, measure};
use crate::words::put;
use crate::workspace::load_order::{Details, Overview};
use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ObjectFit, ParentElement,
    Pixels, StatefulInteractiveElement, Styled, StyledImage, TextOverflow, Window, div, img,
    prelude::FluentBuilder, px,
};
use rimstack_core::base::sentence::Sentence;
use rimstack_core::mods::details::Named;
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
    /// The widths wanted for the inactive mods and the details, for `pane_widths`.
    pub panes: (Pixels, Pixels),
    /// The problems panel shown in place of the rest.
    pub problems_open: bool,
    /// The search for the broken mod runs: it is not offered again.
    pub hunting: bool,
    pub previewing: bool,
    /// With several mods chosen, how many and what they are.
    pub selected: usize,
    pub selection: Option<Rc<crate::workspace::load_order::SelectionSummary>>,
}

/// What the details ask of the Mods tab.
pub enum Asked {
    Section(usize),
    Filter(&'static str),
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
}

impl DetailsView {
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
        self.shown = shown;
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

type Fold = Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App)>;

fn card(
    theme: &Theme,
    id: &'static str,
    title: String,
    tone: gpui::Hsla,
    fold: Option<(bool, Fold)>,
    body: Vec<AnyElement>,
) -> AnyElement {
    let card = Card::new(id, title).tone(tone).body(body);
    // The cards that fold here hold a "Saved" mark beside their title.
    let card = match fold {
        Some((folded, fold)) => card.tall().folds(folded, fold),
        None => card,
    };
    card.render(theme, None).into_any_element()
}

/// `Defined.qml`: a term of the details, underlined with dots when it has a meaning to show.
fn defined(theme: &Theme, term: String) -> gpui::Div {
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

/// `FilterField.qml` as it stands holding `text`, or its placeholder.
fn field_look(theme: &Theme, text: String, placeholder: String) -> gpui::Div {
    let c = &theme.colours;
    let (shown, colour) = if text.is_empty() {
        (placeholder, c.ink3)
    } else {
        (text, c.ink)
    };
    div()
        .flex()
        .items_center()
        .h(theme.field())
        .px(theme::GAP_MEDIUM)
        .rounded(theme::CORNER)
        .bg(c.well)
        .border_1()
        .border_color(c.field_line)
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .sized_text(theme, theme.body())
                .text_color(colour)
                .child(shown),
        )
}

fn names(theme: &Theme, title: String, named: &[Named]) -> Option<AnyElement> {
    let c = &theme.colours;
    (!named.is_empty()).then(|| {
        let lines = named.iter().map(|one| {
            let note = put(&one.note);
            div()
                .flex()
                .gap(theme::GAP)
                .child(label(
                    theme,
                    theme.body(),
                    if one.trouble { c.warn } else { c.ink },
                    one.name.clone(),
                ))
                .when(!note.is_empty(), |d| {
                    d.child(label(theme, theme.small(), c.ink3, note))
                })
                .into_any_element()
        });
        card(theme, "names", title, c.ink2, None, lines.collect())
    })
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
                    .child(
                        Btn::new("toggle", toggle)
                            .icon(if active { "minus" } else { "plus" })
                            .primary_if(!active),
                    )
                    .child(
                        Btn::new("open-folder", if narrow { String::new() } else { open })
                            .icon("folder-open"),
                    )
                    .when(details.workshop, |d| {
                        d.child(
                            Btn::new("workshop-page", if narrow { String::new() } else { page })
                                .icon("external-link"),
                        )
                    })
                    .when(!details.workshop && !details.repository.is_empty(), |d| {
                        d.child(
                            Btn::new("repository", put(&say!("Repository page")))
                                .icon("git-branch"),
                        )
                    }),
            )
            .child(div().h(theme::GAP_LARGE))
            .child(choice);
        let body = match tab {
            0 => self.about(theme, details, inner, window, cx),
            1 => self.placed(theme, details),
            _ => self.files(theme, details),
        };
        div().flex().flex_col().w_full().child(header).child(
            div()
                .flex()
                .flex_col()
                .pt(theme::GAP_LARGE)
                .gap(theme::GAP_CARD)
                .children(body),
        )
    }

    fn about(
        &self,
        theme: &Theme,
        details: &Details,
        inner: Pixels,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let c = &theme.colours;
        let mut shown = Vec::new();
        if !details.problems.is_empty() {
            let errors = details.problems.iter().any(|p| p.error) || details.unreadable.is_some();
            let lines = details
                .problems
                .iter()
                .map(|said| words(theme, theme.body(), c.ink, put(&said.words)).into_any_element())
                .collect();
            shown.push(card(
                theme,
                "about-problems",
                put(&say!("Before you play")),
                if errors { c.err } else { c.warn },
                None,
                lines,
            ));
        }
        if !details.description.is_empty() {
            let text =
                words(theme, theme.body(), c.ink, details.description.clone()).into_any_element();
            shown.push(card(
                theme,
                "about-description",
                put(&say!("Description")),
                c.ink2,
                None,
                vec![text],
            ));
        }
        let labels = [
            say!("Contains"),
            say!("Code"),
            say!("Load time"),
            say!("Package id"),
            say!("Folder"),
            say!("Filed under"),
            say!("Group"),
        ];
        let label_width = labels
            .iter()
            .map(|name| measure(window, theme, &put(name), theme.body(), FontWeight::NORMAL))
            .fold(px(0.), Pixels::max)
            + theme::GAP_SMALL
            + theme.sized(12.)
            + theme::GAP;
        let facts = [
            (say!("Contains"), put(&details.contains)),
            (say!("Code"), put(&details.code)),
            (say!("Load time"), details.load_time.clone()),
            (say!("Package id"), details.package_id.clone()),
        ];
        let mut lines: Vec<AnyElement> = facts
            .into_iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(name, value)| {
                div()
                    .flex()
                    .gap(theme::GAP_TINY)
                    .child(label(theme, theme.body(), c.ink2, put(&name)).w(label_width))
                    .child(words(theme, theme.body(), c.ink, value).flex_1().min_w_0())
                    .into_any_element()
            })
            .collect();
        if !details.path.is_empty() {
            let button = theme.icon() + theme.sized(14.);
            let room = inner - label_width - theme::GAP - (button + theme::GAP_SMALL) * 2.;
            let path = elide_middle_in(
                window,
                theme,
                theme.mono.clone(),
                &details.path,
                theme.small(),
                room,
            );
            lines.push(
                div()
                    .flex()
                    .items_center()
                    .gap(theme::GAP)
                    .child(label(theme, theme.body(), c.ink2, put(&say!("Folder"))).w(label_width))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .items_center()
                            .gap(theme::GAP_SMALL)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_overflow(TextOverflow::TruncateMiddle("…".into()))
                                    .font_family(theme.mono.clone())
                                    .sized_text(theme, theme.small())
                                    .text_color(c.ink)
                                    .child(path),
                            )
                            .child(Btn::new("copy-path", "").icon("copy"))
                            .child(Btn::new("open-path", "").icon("folder-open")),
                    )
                    .into_any_element(),
            );
        }
        let category = details.category.unwrap_or_default();
        let at = theme::CATEGORY_NAMES
            .iter()
            .position(|name| *name == category);
        let filed = put(&Sentence::said(category, Vec::new()));
        let filed = if details.category_chosen.is_empty() {
            filed
        } else {
            put(&say!("{} (chosen)", filed))
        };
        lines.push(
            div()
                .flex()
                .items_center()
                .gap(theme::GAP)
                .child(defined(theme, put(&say!("Filed under"))).w(label_width))
                .child(icon(
                    at.and_then(|at| theme::CATEGORY_ICONS.get(at))
                        .copied()
                        .unwrap_or("package"),
                    theme.icon(),
                    at.map_or(c.ink2, |at| theme.category_colour(at)),
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .sized_text(theme, theme.body())
                        .text_color(c.ink)
                        .child(filed),
                )
                .child(Btn::new("file-under", put(&say!("Change"))))
                .into_any_element(),
        );
        lines.push(
            div()
                .flex()
                .items_center()
                .gap(theme::GAP)
                .child(defined(theme, put(&say!("Group"))).w(label_width))
                .child(
                    field_look(
                        theme,
                        details.sub.clone(),
                        put(&say!("A tag to group under")),
                    )
                    .flex_1(),
                )
                .into_any_element(),
        );
        let facts = self.fold("about-facts", false, cx);
        shown.push(card(
            theme,
            "about-facts",
            put(&say!("Facts")),
            c.ink2,
            Some(facts),
            lines,
        ));
        let notes = vec![
            field_look(
                theme,
                details.tags.clone(),
                put(&say!("Tags, separated by commas")),
            )
            .into_any_element(),
            field_look(theme, details.note.clone(), put(&say!("Note")))
                .h(theme.sized(72.))
                .items_start()
                .pt(theme::GAP_SMALL)
                .into_any_element(),
        ];
        let notes_fold = self.fold("about-notes", false, cx);
        shown.push(card(
            theme,
            "about-notes",
            put(&say!("Tags and note")),
            c.ink2,
            Some(notes_fold),
            notes,
        ));
        shown
    }

    fn placed(&self, theme: &Theme, details: &Details) -> Vec<AnyElement> {
        let mut shown = Vec::new();
        if let Some(placement) = &details.placement {
            shown.push(
                words(theme, theme.body(), theme.colours.ink, put(placement)).into_any_element(),
            );
        }
        shown.extend(names(theme, put(&say!("Requires")), &details.requires));
        shown.extend(names(
            theme,
            put(&say!("Loads after")),
            &details.loads_after,
        ));
        shown.extend(names(
            theme,
            put(&say!("Loads before")),
            &details.loads_before,
        ));
        shown
    }

    fn files(&self, theme: &Theme, details: &Details) -> Vec<AnyElement> {
        let c = &theme.colours;
        let mut lines =
            vec![words(theme, theme.body(), c.ink, details.path.clone()).into_any_element()];
        lines.extend(
            details
                .load_folders
                .iter()
                .map(|folder| words(theme, theme.body(), c.ink2, put(folder)).into_any_element()),
        );
        vec![card(
            theme,
            "files-folders",
            put(&say!("Folders the game loads from")),
            c.ink2,
            None,
            lines,
        )]
    }
}
