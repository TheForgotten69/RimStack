//! `FirstRun.qml`: the first start's five steps; nothing changes until Start but what is chosen
//! here, which is kept as it is chosen.

use super::button::Btn;
use super::controls::Choice;
use super::theme::{self, Theme, ThemeName};
use super::widgets::{TextSized, elide_middle_in, icon, label, measure};
use crate::changes::Changes;
use crate::reports::{Database, SettingsReport};
use crate::words::{count, put};
use gpui::{
    AnyElement, Context, EventEmitter, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Pixels, Render, Role, SharedString, StatefulInteractiveElement, Styled, StyledImage, Toggled,
    Window, div, img, prelude::FluentBuilder, px,
};
use rimstack_core::base::sentence::Sentence;
use rimstack_core::say;
use std::rc::Rc;

/// Asked of the window: the setup ends, the community data fetched and RimSort's rules read
/// when asked.
pub struct Started {
    pub with_data: bool,
    pub with_rimsort: bool,
}

pub struct FirstRun {
    step: usize,
    report: Rc<SettingsReport>,
    databases: Rc<Vec<Database>>,
    languages: Rc<Vec<String>>,
    with_rimsort: bool,
    /// The load order as it was read: its active and inactive mods, or why it was not.
    mods: (usize, usize, String),
}

impl EventEmitter<Started> for FirstRun {}

const STEPS: usize = 5;

impl FirstRun {
    pub fn new(step: usize, cx: &mut Context<Self>) -> Self {
        let mut first = Self {
            step: step.min(STEPS - 1),
            report: Rc::default(),
            databases: Rc::default(),
            languages: Rc::default(),
            with_rimsort: true,
            mods: (0, 0, String::new()),
        };
        first.read(cx);
        let (tell, mut told) = futures::channel::mpsc::unbounded();
        super::app(cx).listen(move |changes| {
            let _ = tell.unbounded_send(changes);
        });
        cx.spawn(async move |first, cx| {
            use futures::StreamExt;
            while let Some(changes) = told.next().await {
                let read =
                    Changes::SETTINGS | Changes::COMMUNITY | Changes::LANGUAGE | Changes::FOLDERS;
                let going = first.update(cx, |first, cx| {
                    if changes.contains(read) {
                        first.read(cx);
                    }
                });
                if going.is_err() {
                    return;
                }
            }
        })
        .detach();
        first
    }

    fn read(&mut self, cx: &mut Context<Self>) {
        let app = super::app(cx);
        self.report = Rc::new(app.settings_report());
        self.databases = Rc::new(app.databases());
        self.languages = Rc::new(app.languages());
        cx.notify();
    }

    /// What the Mods tab read, for the first step.
    pub fn set_mods(
        &mut self,
        active: usize,
        inactive: usize,
        problem: String,
        cx: &mut Context<Self>,
    ) {
        self.mods = (active, inactive, problem);
        cx.notify();
    }

    fn keep(&mut self, name: &str, value: &str, cx: &mut Context<Self>) {
        if let Err(problem) = super::app(cx).keep_setting(name, value) {
            super::notices::say(
                crate::words::said_of(&problem),
                super::notices::Tone::Err,
                cx,
            );
        }
        self.read(cx);
    }

    #[cfg(test)]
    pub(crate) fn press_start(&mut self, cx: &mut Context<Self>) {
        self.start(cx);
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        let with_data = self.databases.iter().any(|d| d.on);
        let with_rimsort = self.with_rimsort && self.report.rimsort_rules;
        cx.emit(Started {
            with_data,
            with_rimsort,
        });
    }

    fn steps() -> [Sentence; STEPS] {
        [
            say!("Found"),
            say!("Community data"),
            say!("Sorting"),
            say!("Appearance"),
            say!("Start"),
        ]
    }

    fn block(theme: &Theme, title: String, lines: Vec<AnyElement>) -> AnyElement {
        let c = &theme.colours;
        let count = lines.len();
        let lines = lines.into_iter().enumerate().map(|(at, line)| {
            div()
                .when(at + 1 < count, |d| d.border_b_1().border_color(c.line))
                .child(line)
        });
        div()
            .flex()
            .flex_col()
            .pt(theme::GAP_WIDE)
            .child(
                label(theme, theme.small(), c.ink2, title)
                    .font_weight(theme.semibold)
                    .pl(theme::GAP_LARGE)
                    .pb(theme::GAP_SMALL),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .rounded(theme::CORNER_LARGE)
                    .bg(c.card)
                    .children(lines),
            )
            .into_any_element()
    }

    /// `Found`: a thing found and where, or a sentence; `note` for words that only tell.
    fn found(
        theme: &Theme,
        window: &Window,
        what: String,
        place: Option<String>,
        sentence: bool,
        note: bool,
        width: Pixels,
    ) -> AnyElement {
        let c = &theme.colours;
        let (mark, tone) = match (note, place.is_some()) {
            (true, _) => ("•", c.ink3),
            (false, true) => ("✓", c.ok),
            (false, false) => ("!", c.err),
        };
        let row = div()
            .flex()
            .items_center()
            .gap(theme::GAP)
            .min_h(theme.sized(36.))
            .py(theme::GAP_SMALL)
            .px(theme::GAP_LARGE)
            .child(
                label(theme, theme.body(), tone, mark.to_string())
                    .font_weight(theme.semibold)
                    .w(theme.sized(18.)),
            );
        match sentence {
            true => {
                let said = match &place {
                    Some(place) => format!("{what}, {place}"),
                    None => what,
                };
                row.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .sized_text(theme, theme.body())
                        .text_color(if note || place.is_some() {
                            c.ink
                        } else {
                            c.err
                        })
                        .child(said),
                )
            }
            false => {
                let room = width
                    - theme::GAP_LARGE * 2.
                    - theme.sized(18.)
                    - theme.sized(220.)
                    - theme::GAP * 2.;
                let path = place.clone().unwrap_or_else(|| put(&say!("Not found")));
                let shown = elide_middle_in(
                    window,
                    theme,
                    theme.mono.clone(),
                    &path,
                    theme.small(),
                    room,
                );
                row.child(
                    div()
                        .w(theme.sized(220.))
                        .flex_none()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .sized_text(theme, theme.body())
                        .text_color(c.ink)
                        .child(what),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .sized_text(theme, theme.small())
                        .font_family(theme.mono.clone())
                        .text_color(if place.is_some() { c.ink2 } else { c.err })
                        .child(shown),
                )
            }
        }
        .into_any_element()
    }

    /// `Tick`: a box to tick, its words and, dimmer, what they mean.
    fn tick(
        theme: &Theme,
        id: SharedString,
        text: String,
        detail: String,
        on: bool,
        toggled: impl Fn(&mut Window, &mut gpui::App) + 'static,
    ) -> AnyElement {
        let c = &theme.colours;
        let size = theme.sized(15.);
        div()
            .id(id)
            .role(Role::CheckBox)
            .aria_label(text.clone())
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .flex()
            .items_center()
            .gap(theme::GAP)
            .min_h(theme.sized(36.))
            .py(theme::GAP_SMALL)
            .px(theme::GAP_LARGE)
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(size)
                    .rounded(px(3.))
                    .bg(if on { c.accent } else { c.well })
                    .border_1()
                    .border_color(if on { c.accent } else { c.line })
                    .when(on, |d| {
                        d.child(
                            label(theme, theme.caption(), c.accent_ink, "✓".to_string())
                                .font_weight(theme.semibold),
                        )
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .sized_text(theme, theme.body())
                    .text_color(c.ink)
                    .child(text)
                    .when(!detail.is_empty(), |d| {
                        d.child(div().text_color(c.ink2).child(format!(" {detail}")))
                    }),
            )
            .on_click(move |_, window, cx| toggled(window, cx))
            .into_any_element()
    }

    /// `Pick`: a name and the controls that choose it.
    fn pick(
        theme: &Theme,
        window: &Window,
        what: String,
        controls: Vec<AnyElement>,
        width: Pixels,
    ) -> AnyElement {
        let c = &theme.colours;
        let named = measure(window, theme, &what, theme.body(), FontWeight::NORMAL)
            .max(theme.sized(190.))
            .min(width / 2.);
        div()
            .flex()
            .items_center()
            .gap(theme::GAP_LARGE)
            .min_h(theme.sized(44.))
            .py(theme::GAP)
            .px(theme::GAP_LARGE)
            .child(
                div()
                    .w(named)
                    .flex_none()
                    .sized_text(theme, theme.body())
                    .text_color(c.ink)
                    .child(what),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .flex_1()
                    .min_w_0()
                    .gap(theme::GAP_SMALL)
                    .children(controls),
            )
            .into_any_element()
    }

    fn content(
        &self,
        theme: &Theme,
        window: &Window,
        width: Pixels,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let report = self.report.clone();
        let found = |what: String, place: Option<String>, sentence: bool, note: bool| {
            Self::found(theme, window, what, place, sentence, note, width)
        };
        let mut blocks = Vec::new();
        match self.step {
            0 => {
                let languages = self.languages.iter().map(|name| {
                    let chosen = report.language_in_use == *name;
                    let held = name.clone();
                    Btn::new(SharedString::from(format!("language-{name}")), name.clone())
                        .primary_if(chosen)
                        .on_click(cx.listener(move |first, _, _, cx| {
                            first.keep("language", &held, cx);
                            let app = super::app(cx);
                            crate::words::show_in(crate::words::Words::of(&app.speak()));
                            cx.refresh_windows();
                        }))
                        .into_any_element()
                });
                blocks.push(Self::block(
                    theme,
                    put(&say!("Interface")),
                    vec![Self::pick(
                        theme,
                        window,
                        put(&say!("Language")),
                        languages.collect(),
                        width,
                    )],
                ));
                let game = match &report.game_version {
                    Some(version) => format!("RimWorld {version}"),
                    None => "RimWorld".to_string(),
                };
                blocks.push(Self::block(
                    theme,
                    put(&say!("Game")),
                    vec![
                        found(game, report.game.clone(), false, false),
                        found(
                            put(&say!("Workshop mods")),
                            report.workshop.clone(),
                            false,
                            false,
                        ),
                        found(
                            put(&say!("Config and saves")),
                            report.config.clone(),
                            false,
                            false,
                        ),
                    ],
                ));
                let (active, inactive, problem) = &self.mods;
                let line = match problem.is_empty() {
                    true => found(
                        put(&say!(
                            "{} active mods, {} inactive",
                            count(*active),
                            count(*inactive)
                        )),
                        Some(put(&say!("kept as they are"))),
                        true,
                        false,
                    ),
                    false => found(
                        put(&say!("Mods could not be read: {}", problem.clone())),
                        None,
                        true,
                        false,
                    ),
                };
                blocks.push(Self::block(
                    theme,
                    put(&say!("Current mod list")),
                    vec![line],
                ));
                let mut programs = Vec::new();
                match &report.git {
                    Some(version) => programs.push(found("Git".into(), Some(put(&say!("version {}", version.clone()))), true, false)),
                    None => programs.push(found(put(&say!("Git is not installed: a mod kept as a Git checkout cannot be checked or updated")), None, true, true)),
                }
                match (&report.steamcmd_refused, report.steamcmd) {
                    (_, true) => programs.push(found("SteamCMD".into(), Some(put(&say!("installed"))), true, false)),
                    (Some(refused), false) => programs.push(found(put(refused), None, true, true)),
                    (None, false) => programs.push(found(put(&say!("SteamCMD is not installed: Downloads can install it, to fetch a mod without subscribing")), None, true, true)),
                }
                blocks.push(Self::block(theme, put(&say!("Programs")), programs));
            }
            1 => {
                if report.rimsort_rules {
                    let on = self.with_rimsort;
                    let me = cx.entity().downgrade();
                    blocks.push(Self::block(
                        theme,
                        put(&say!("Import from RimSort")),
                        vec![Self::tick(
                            theme,
                            "rimsort".into(),
                            put(&say!("Your load order rules")),
                            put(&say!("(RimSort's file is only read)")),
                            on,
                            move |_, cx| {
                                let _ = me.update(cx, |first, cx| {
                                    first.with_rimsort = !first.with_rimsort;
                                    cx.notify();
                                });
                            },
                        )],
                    ));
                }
                let ticks = self.databases.iter().map(|database| {
                    let mut detail = put(&database.used_for);
                    if database.updated > 0 {
                        detail = format!(
                            "{detail}, {}",
                            put(&say!(
                                "downloaded {}",
                                crate::workspace::logs::day(database.updated)
                            ))
                        );
                    }
                    let (id, on) = (database.id, database.on);
                    let me = cx.entity().downgrade();
                    Self::tick(
                        theme,
                        SharedString::from(format!("database-{id}")),
                        database.name.to_string(),
                        detail,
                        on,
                        move |_, cx| {
                            let _ = me.update(cx, |first, cx| {
                                if let Err(problem) = super::app(cx).use_database(id, !on) {
                                    super::notices::say(
                                        crate::words::said_of(&problem),
                                        super::notices::Tone::Err,
                                        cx,
                                    );
                                }
                                first.read(cx);
                            });
                        },
                    )
                });
                blocks.push(Self::block(
                    theme,
                    put(&say!("Community data")),
                    ticks.collect(),
                ));
            }
            2 => {
                let me = cx.entity().downgrade();
                let flip = |name: &'static str, on: bool| {
                    let me = me.clone();
                    move |_: &mut Window, cx: &mut gpui::App| {
                        let _ = me.update(cx, |first, cx| {
                            first.keep(name, if on { "off" } else { "on" }, cx)
                        });
                    }
                };
                blocks.push(Self::block(
                    theme,
                    put(&say!("Sort follows")),
                    vec![
                        found(put(&say!("Each mod's own rules, and the code it uses or patches in another mod: always")), None, true, true),
                        Self::tick(theme, "community-rules".into(), put(&say!("Community rules")), put(&say!("load-order rules written by players, for mods whose files do not say enough")), report.community_rules, flip("communityRules", report.community_rules)),
                        Self::tick(theme, "file-hints".into(), put(&say!("Patch and def hints")), put(&say!("a patch is placed after what it patches, read from the files")), report.file_hints, flip("fileHints", report.file_hints)),
                    ],
                ));
                blocks.push(Self::block(
                    theme,
                    put(&say!("Either way")),
                    vec![
                        found(put(&say!("Sort shows every move and the rule behind it before anything changes")), None, true, true),
                        found(put(&say!("Hints that Sort does not use still show in a mod's Load order tab")), None, true, true),
                    ],
                ));
            }
            3 => {
                blocks.extend(self.glance(theme, &report));
                let look = cx.global::<Theme>().appearance.clone();
                let names: Vec<SharedString> =
                    [say!("Rim"), say!("Dark"), say!("OLED"), say!("Light")]
                        .iter()
                        .map(|name| put(name).into())
                        .collect();
                let themes = [
                    ThemeName::Rim,
                    ThemeName::Dark,
                    ThemeName::Oled,
                    ThemeName::Light,
                ];
                let current = themes.iter().position(|t| *t == look.theme).unwrap_or(0);
                let sizes = [1.0_f32, 1.1, 1.25, 1.5];
                let size_at = sizes
                    .iter()
                    .position(|s| (s - look.text_scale).abs() < 0.03)
                    .unwrap_or(0);
                let rows = if look.comfortable {
                    2
                } else if look.large_rows {
                    1
                } else {
                    0
                };
                let in_steam = super::app(cx)
                    .prefs()
                    .flag("Appearance", "workshopInSteam")
                    .unwrap_or(false);
                let choice = |id: &'static str,
                              title: String,
                              options: Vec<SharedString>,
                              at: usize,
                              picked: fn(usize, &mut gpui::App)| {
                    Choice::new(id, options, at)
                        .title(title)
                        .on_pick(move |at, _, cx| picked(at, cx))
                        .into_any_element()
                };
                blocks.push(Self::block(
                    theme,
                    put(&say!("Appearance")),
                    vec![
                        Self::pick(
                            theme,
                            window,
                            put(&say!("Theme")),
                            vec![choice(
                                "first-theme",
                                put(&say!("Theme")),
                                names,
                                current,
                                |at, cx| {
                                    let theme = [
                                        ThemeName::Rim,
                                        ThemeName::Dark,
                                        ThemeName::Oled,
                                        ThemeName::Light,
                                    ][at];
                                    super::keep_appearance("theme", theme.name(), cx)
                                },
                            )],
                            width,
                        ),
                        Self::pick(
                            theme,
                            window,
                            put(&say!("Text size")),
                            vec![choice(
                                "first-size",
                                put(&say!("Text size")),
                                ["100%", "110%", "125%", "150%"]
                                    .map(SharedString::from)
                                    .to_vec(),
                                size_at,
                                |at, cx| {
                                    super::keep_appearance(
                                        "textScale",
                                        ["1", "1.1", "1.25", "1.5"][at],
                                        cx,
                                    )
                                },
                            )],
                            width,
                        ),
                        Self::pick(
                            theme,
                            window,
                            put(&say!("Mod rows")),
                            vec![choice(
                                "first-rows",
                                put(&say!("Mod rows")),
                                ["Compact", "Larger", "Comfortable"]
                                    .map(SharedString::from)
                                    .to_vec(),
                                rows,
                                |at, cx| {
                                    super::keep_appearance(
                                        "comfortable",
                                        if at == 2 { "true" } else { "false" },
                                        cx,
                                    );
                                    super::keep_appearance(
                                        "largeRows",
                                        if at == 1 { "true" } else { "false" },
                                        cx,
                                    );
                                },
                            )],
                            width,
                        ),
                        Self::pick(
                            theme,
                            window,
                            put(&say!("Workshop pages open in")),
                            vec![choice(
                                "first-pages",
                                put(&say!("Workshop pages open in")),
                                ["Browser", "Steam"].map(SharedString::from).to_vec(),
                                usize::from(in_steam),
                                |at, cx| {
                                    super::keep_appearance(
                                        "workshopInSteam",
                                        if at == 1 { "true" } else { "false" },
                                        cx,
                                    )
                                },
                            )],
                            width,
                        ),
                    ],
                ));
            }
            _ => {
                blocks.push(Self::block(
                    theme,
                    put(&say!("After Start")),
                    vec![
                        found(
                            put(&say!(
                                "Problems are counted above the load order, each with its fix"
                            )),
                            None,
                            true,
                            true,
                        ),
                        found(
                            put(&say!(
                                "Sort shows every move, and why, before anything changes"
                            )),
                            None,
                            true,
                            true,
                        ),
                        found(
                            put(&say!(
                                "Save writes ModsConfig.xml and copies the file as it was first"
                            )),
                            None,
                            true,
                            true,
                        ),
                    ],
                ));
            }
        }
        blocks
    }

    /// The game and its expansions as the load order will show them, the second one chosen.
    fn glance(&self, theme: &Theme, report: &SettingsReport) -> Option<AnyElement> {
        if report.official.is_empty() {
            return None;
        }
        let c = &theme.colours;
        let comfortable = theme.appearance.comfortable;
        let version = report.game_version.clone().unwrap_or_default();
        let rows = report.official.iter().enumerate().map(|(at, one)| {
            let chosen = at == 1;
            let (ink, dim) = match chosen {
                true => (c.on_selected, c.on_selected_dim),
                false => (c.ink, c.ink2),
            };
            div()
                .flex()
                .items_center()
                .gap(theme::GAP)
                .h(if comfortable {
                    theme.sized(46.)
                } else {
                    theme.row()
                })
                .px(theme::GAP_MEDIUM)
                .rounded(theme::CORNER)
                .when(chosen, |d| d.bg(c.chosen))
                .child(label(
                    theme,
                    theme.small(),
                    if chosen { dim } else { c.ink3 },
                    (at + 1).to_string(),
                ))
                .child(icon(
                    theme::CATEGORY_ICONS[1],
                    theme.sized(13.),
                    if chosen {
                        ink
                    } else {
                        theme.category_colour(1)
                    },
                ))
                .when(comfortable, |d| {
                    d.child(
                        div()
                            .flex_none()
                            .w(theme.sized(64.))
                            .h(theme.sized(40.))
                            .rounded(theme::CORNER)
                            .bg(c.ctl)
                            .overflow_hidden()
                            .p(theme::GAP_LINE)
                            .when_some(one.picture.clone(), |d, picture| {
                                d.child(
                                    img(std::path::PathBuf::from(picture))
                                        .size_full()
                                        .object_fit(gpui::ObjectFit::Cover),
                                )
                            }),
                    )
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .sized_text(theme, theme.body())
                        .text_color(ink)
                        .child(one.name.clone()),
                )
                .child(label(theme, theme.body(), dim, one.author.clone()))
                .child(label(theme, theme.body(), dim, version.clone()))
                .into_any_element()
        });
        Some(Self::block(
            theme,
            put(&say!("As it will look")),
            vec![div().p(theme::GAP).children(rows).into_any_element()],
        ))
    }
}

impl Render for FirstRun {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let c = theme.colours;
        let viewport = window.viewport_size();
        let width = theme.sized(940.).min(viewport.width - theme::GAP_PAGE * 4.);
        let height = theme
            .sized(580.)
            .min(viewport.height - theme::GAP_PAGE * 2.);
        let side = theme.sized(112.).max(measure(
            window,
            &theme,
            &put(&say!("A mod manager for RimWorld")),
            theme.title(),
            FontWeight::NORMAL,
        ));
        let content_width = width - side - theme::GAP_WIDE * 3.;
        let steps = Self::steps();
        let marks = steps.iter().enumerate().map(|(at, name)| {
            let (ground, ink) = match at.cmp(&self.step) {
                std::cmp::Ordering::Equal => (c.accent, c.accent_ink),
                std::cmp::Ordering::Less => (c.ok, c.accent_ink),
                std::cmp::Ordering::Greater => (c.ctl, c.ink2),
            };
            let mark = if at < self.step {
                "✓".to_string()
            } else {
                (at + 1).to_string()
            };
            let on = at == self.step;
            div()
                .id(("step", at))
                .flex()
                .items_center()
                .gap(theme::GAP)
                .cursor_pointer()
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .size(theme.sized(22.))
                        .rounded(theme.sized(11.))
                        .bg(ground)
                        .child(label(&theme, theme.small(), ink, mark).font_weight(theme.semibold)),
                )
                .child(
                    label(
                        &theme,
                        theme.body(),
                        if on { c.ink } else { c.ink2 },
                        put(name),
                    )
                    .when(on, |d| d.font_weight(theme.semibold)),
                )
                .on_click(cx.listener(move |first, _, _, cx| {
                    first.step = at;
                    cx.notify();
                }))
        });
        let meanings = [
            say!("Found automatically; changeable in Settings"),
            say!("Fetched from GitHub on Start, and again only on request"),
            say!("What Sort follows besides each mod's own rules"),
            say!("Changeable in Settings at any time"),
            say!("Nothing is written to the game before Save"),
        ];
        let last = self.step == STEPS - 1;
        let buttons =
            div()
                .flex()
                .justify_end()
                .gap(theme::GAP)
                .when(self.step > 0, |d| {
                    d.child(Btn::new("first-back", put(&say!("Back"))).large().on_click(
                        cx.listener(|first, _, _, cx| {
                            first.step -= 1;
                            cx.notify();
                        }),
                    ))
                })
                .when(!last, |d| {
                    d.child(
                        Btn::new("first-skip", put(&say!("Skip to Start")))
                            .large()
                            .on_click(cx.listener(|first, _, _, cx| first.start(cx))),
                    )
                    .child(
                        Btn::new("first-next", put(&say!("Next")))
                            .large()
                            .primary()
                            .on_click(cx.listener(|first, _, _, cx| {
                                first.step += 1;
                                cx.notify();
                            })),
                    )
                })
                .when(last, |d| {
                    d.child(
                        Btn::new("first-start", put(&say!("Start")))
                            .large()
                            .primary()
                            .icon("check")
                            .on_click(cx.listener(|first, _, _, cx| first.start(cx))),
                    )
                });
        div()
            .id("first-run")
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(c.win)
            .child(
                div()
                    .flex()
                    .gap(theme::GAP_WIDE * 3.)
                    .w(width)
                    .h(height)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_none()
                            .gap(theme::GAP_SMALL)
                            .child(img("rimstack.svg").size(theme.sized(112.)))
                            .child(
                                label(&theme, theme.display(), c.brand, put(&say!("RimStack")))
                                    .font_weight(theme.semibold)
                                    .pt(theme::GAP),
                            )
                            .child(label(
                                &theme,
                                theme.title(),
                                c.ink,
                                put(&say!("A mod manager for RimWorld")),
                            ))
                            .child(div().h(theme::GAP_PAGE))
                            .children(marks),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap(theme::GAP_WIDE)
                            .child(
                                div()
                                    .id("first-content")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                label(
                                                    &theme,
                                                    theme.heading(),
                                                    c.ink,
                                                    put(&steps[self.step]),
                                                )
                                                .font_weight(theme.semibold),
                                            )
                                            .child(
                                                label(
                                                    &theme,
                                                    theme.body(),
                                                    c.ink2,
                                                    put(&meanings[self.step]),
                                                )
                                                .pt(theme::GAP_HAIR),
                                            )
                                            .children(self.content(
                                                &theme,
                                                window,
                                                content_width,
                                                cx,
                                            )),
                                    ),
                            )
                            .child(buttons),
                    ),
            )
    }
}
