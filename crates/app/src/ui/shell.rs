//! The top bar, the section shown, the status bar.

use super::Options;
use super::downloads::{Downloads, ShowMod};
use super::mods::{Mods, ShowSection};
use super::section::Section;
use super::theme::Theme;
use super::top_bar::top_bar;
use gpui::{
    AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Window, div,
};

pub struct Shell {
    section: Section,
    /// The keys' handles of the sections and of the search box.
    bar_keys: [gpui::FocusHandle; 2],
    pub(crate) demo: bool,
    pub(crate) mods: Entity<Mods>,
    /// Made with the window, as Qt's `DownloadsData` was: it counts the updates the top bar and
    /// the Mods tab show. What it reads of the `App` it reads off the window's thread.
    pub(crate) downloads: Option<Entity<Downloads>>,
    /// Made the first time Settings is shown; `part` is the one it opens on.
    pub(crate) settings: Option<Entity<super::settings::Settings>>,
    pub(crate) part: usize,
    /// Made the first time Logs is shown; `view` is the one it opens on.
    pub(crate) logs: Option<Entity<super::logs::Logs>>,
    /// Made the first time Saves is shown.
    pub(crate) saves: Option<Entity<super::saves::Saves>>,
    /// The Background work popup open over the status bar.
    pub(crate) tray_open: bool,
    pub(crate) tray_turn: usize,
    /// The Background work popup closed, while it fades out.
    pub(crate) tray_fading: bool,
    /// The turn of the section shown last, which fades it in; none for the first.
    shown_turn: usize,
    /// The turning arrows beside the work under way, a view of its own so that only it is drawn
    /// again while it turns.
    pub(crate) spinner: Entity<super::status_bar::Spinner>,
    /// A newer RimStack found: its version and its page.
    pub(crate) release: Option<(String, String)>,
    /// Made the first time Workshop is shown.
    pub(crate) workshop: Option<Entity<super::workshop::Workshop>>,
    /// Made the first time Search is shown.
    pub(crate) find: Option<Entity<super::find::Find>>,
    /// Made the first time Textures is shown.
    pub(crate) textures: Option<Entity<super::textures::Textures>>,
    pub(crate) view: usize,
    pub(crate) game: Entity<super::game::Game>,
    /// The search for the broken mod, as `bisect.json` keeps it between starts.
    pub(crate) hunt: crate::workspace::search::Hunt,
    /// The saved lists and the history, over the window when open.
    pub(crate) lists: Entity<super::lists::Lists>,
    /// The first start's setup, in place of everything until Start.
    pub(crate) first_run: Option<Entity<super::first_run::FirstRun>>,
    /// Ctrl + K, open over the window.
    pub(crate) commands: Option<Entity<super::commands::Commands>>,
    /// Ctrl + K just closed, while it fades out.
    pub(crate) closing_commands: Option<Entity<super::commands::Commands>>,
    /// The keys' sheet, open over the window.
    pub(crate) keys_open: bool,
    pub(crate) keys_focus: gpui::FocusHandle,
    /// `beforePlay`: what stands in Play's way, open under it.
    pub(crate) before_play: bool,
    prompt: Entity<super::prompt::Prompt>,
    menu: Entity<super::menu::Menu>,
    notices: Entity<super::notices::Notices>,
    drawn: bool,
}

impl Shell {
    pub fn new(options: &Options, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let demo = options.demo;
        cx.observe_global::<super::tip::Hint>(|_, cx| cx.notify())
            .detach();
        let mods = cx.new(|cx| Mods::new(demo, cx));
        let focus = mods.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.subscribe(&mods, |shell, _, ShowSection(section), cx| {
            shell.go(*section, cx)
        })
        .detach();
        cx.subscribe(&mods, |shell, _, wanted: &super::mods::Wanted, cx| {
            use super::mods::Wanted;
            match wanted {
                Wanted::FindBroken => shell.start_hunt(cx),
                Wanted::Browse(text) => {
                    let (text, workshop) = (text.clone(), shell.workshop(cx));
                    workshop.update(cx, |workshop, cx| workshop.browse(text, cx));
                }
                wanted => {
                    let downloads = shell.downloads(cx);
                    downloads.update(cx, |downloads, cx| match wanted {
                        Wanted::Subscribe(id) => downloads.subscribe(id.clone(), cx),
                        Wanted::Redownload(id) => downloads.redownload(*id, cx),
                        Wanted::Unsubscribe(id) => downloads.unsubscribe(*id, cx),
                        Wanted::UpdateFromGit(id) => downloads.update_from_git(*id, cx),
                        Wanted::Browse(_) | Wanted::FindBroken => {}
                    });
                }
            }
        })
        .detach();
        let held_by = mods.downgrade();
        let game = cx.new(|cx| {
            super::game::Game::new(
                demo,
                options.still,
                move |cx| held_by.upgrade().is_some_and(|mods| mods.read(cx).held()),
                cx,
            )
        });
        cx.subscribe(&game, |shell, _, told, cx| shell.heard_game(told, cx))
            .detach();
        cx.observe(&game, |_, _, cx| cx.notify()).detach();
        let section = options.section;
        let lists = {
            let mods = mods.clone();
            cx.new(|cx| super::lists::Lists::new(demo, mods, cx))
        };
        let mut shell = Self {
            section: Section::Mods,
            demo,
            mods,
            downloads: None,
            settings: None,
            part: options.part,
            logs: None,
            textures: None,
            saves: None,
            tray_open: false,
            tray_turn: 0,
            tray_fading: false,
            shown_turn: 0,
            spinner: cx.new(|_| super::status_bar::Spinner),
            release: None,
            find: None,
            workshop: None,
            view: options.view,
            game,
            lists,
            commands: None,
            closing_commands: None,
            first_run: None,
            keys_open: false,
            keys_focus: cx.focus_handle(),
            hunt: match demo {
                true => Default::default(),
                false => super::app(cx).hunt(),
            },
            before_play: false,
            prompt: super::prompt::install(cx),
            menu: super::menu::install(cx),
            notices: super::notices::install(cx),
            drawn: false,
            bar_keys: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
        };
        shell.downloads(cx);
        if !demo {
            keep_logs(cx);
        }
        shell.go(section, cx);
        shell.shown_turn = 0;
        shell.tell_hunting(cx);
        let unset = !demo && !options.still && !super::app(cx).settings_report().setup_done;
        if options.first || unset {
            shell.start_first_run(options.step, cx);
        }
        if !demo && !options.still {
            let left = shell.mods.downgrade();
            window.on_window_should_close(cx, move |window, cx| {
                super::keep_window(window, cx);
                let _ = left.update(cx, |mods, cx| mods.keep_left_scroll(cx));
                true
            });
            check_release(cx);
        }
        super::scenes::stage(&mut shell, options, window, cx);
        shell
    }

    pub(crate) fn downloads(&mut self, cx: &mut Context<Self>) -> Entity<Downloads> {
        if let Some(downloads) = &self.downloads {
            return downloads.clone();
        }
        let (demo, mods) = (self.demo, self.mods.clone());
        let downloads = cx.new(|cx| Downloads::new(demo, mods, cx));
        cx.subscribe(&downloads, |shell, _, ShowMod(mod_id), cx| {
            shell.go(Section::Mods, cx);
            let mod_id = *mod_id;
            shell.mods.update(cx, |mods, cx| mods.show_mod(mod_id, cx));
        })
        .detach();
        self.mods
            .update(cx, |mods, cx| mods.set_downloads(&downloads, cx));
        self.downloads = Some(downloads.clone());
        downloads
    }

    pub(crate) fn workshop(&mut self, cx: &mut Context<Self>) -> Entity<super::workshop::Workshop> {
        if let Some(workshop) = &self.workshop {
            return workshop.clone();
        }
        let (demo, downloads) = (self.demo, self.downloads(cx));
        let workshop = cx.new(|cx| super::workshop::Workshop::new(demo, downloads, cx));
        self.workshop = Some(workshop.clone());
        if self.section != Section::Workshop {
            self.tell_shown(Section::Workshop, false, cx);
        }
        workshop
    }

    pub(crate) fn find(&mut self, cx: &mut Context<Self>) -> Entity<super::find::Find> {
        if let Some(find) = &self.find {
            return find.clone();
        }
        let (demo, mods) = (self.demo, self.mods.clone());
        let find = cx.new(|cx| super::find::Find::new(demo, mods, cx));
        cx.subscribe(&find, |shell, _, super::find::ShowInMods, cx| {
            shell
                .mods
                .update(cx, |mods, cx| mods.filter_by("in:search", cx));
            shell.go(Section::Mods, cx);
        })
        .detach();
        self.find = Some(find.clone());
        if self.section != Section::Find {
            self.tell_shown(Section::Find, false, cx);
        }
        find
    }

    pub fn go(&mut self, section: Section, cx: &mut Context<Self>) {
        if section != self.section {
            self.shown_turn = super::motion::opening();
            self.tell_shown(self.section, false, cx);
        }
        self.section = section;
        if section == Section::Workshop {
            self.workshop(cx);
        }
        if section == Section::Find {
            self.find(cx);
        }
        if section == Section::Downloads {
            self.downloads(cx);
        }
        if section == Section::Saves && self.saves.is_none() {
            let demo = self.demo;
            let saves = cx.new(|cx| super::saves::Saves::new(demo, cx));
            cx.subscribe(&saves, |shell, _, super::saves::UseList(ids), cx| {
                let ids = ids.clone();
                shell.mods.update(cx, |mods, cx| mods.use_list(ids, cx));
                shell.go(Section::Mods, cx);
            })
            .detach();
            self.saves = Some(saves);
        }
        if section == Section::Textures && self.textures.is_none() {
            let demo = self.demo;
            self.textures = Some(cx.new(|cx| super::textures::Textures::new(demo, cx)));
        }
        if section == Section::Logs && self.logs.is_none() {
            let (demo, view) = (self.demo, self.view);
            let logs = cx.new(|cx| super::logs::Logs::new(demo, view, cx));
            cx.subscribe(&logs, |shell, _, super::logs::ShowMod(mod_id), cx| {
                let mod_id = *mod_id;
                shell.go(Section::Mods, cx);
                shell.mods.update(cx, |mods, cx| mods.show_mod(mod_id, cx));
            })
            .detach();
            cx.subscribe(&logs, |shell, _, _: &super::logs::FindBroken, cx| {
                shell.start_hunt(cx)
            })
            .detach();
            cx.subscribe(&logs, |shell, _, super::logs::UseList(ids), cx| {
                let ids = ids.clone();
                shell.mods.update(cx, |mods, cx| mods.use_list(ids, cx));
                shell.go(Section::Mods, cx);
            })
            .detach();
            self.logs = Some(logs);
        }
        if section == Section::Settings && self.settings.is_none() {
            let (demo, part, mods) = (self.demo, self.part, self.mods.clone());
            let settings = cx.new(|cx| super::settings::Settings::new(demo, part, mods, cx));
            cx.subscribe(&settings, |shell, _, super::settings::UseList(ids), cx| {
                let ids = ids.clone();
                shell.mods.update(cx, |mods, cx| mods.use_list(ids, cx));
                shell.go(Section::Mods, cx);
            })
            .detach();
            self.settings = Some(settings);
        }
        self.tell_shown(section, true, cx);
        cx.notify();
    }

    fn tell_shown(&self, section: Section, shown: bool, cx: &mut Context<Self>) {
        use super::section::Shown;
        fn tell<V: Shown + 'static>(view: &Option<Entity<V>>, shown: bool, cx: &mut gpui::App) {
            if let Some(view) = view {
                view.update(cx, |view, cx| view.set_shown(shown, cx));
            }
        }
        match section {
            Section::Mods | Section::Downloads => {}
            Section::Textures => tell(&self.textures, shown, cx),
            Section::Workshop => tell(&self.workshop, shown, cx),
            Section::Saves => tell(&self.saves, shown, cx),
            Section::Logs => tell(&self.logs, shown, cx),
            Section::Find => tell(&self.find, shown, cx),
            Section::Settings => tell(&self.settings, shown, cx),
        }
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(test)]
        RENDERS.with(|renders| renders.set(renders.get() + 1));
        if !self.drawn {
            self.drawn = true;
            window.on_next_frame(|_, _| {
                if let Some(started) = crate::STARTED.get() {
                    rimstack_core::note!("first_frame_ms={}", started.elapsed().as_millis());
                }
            });
        }
        let in_the_way = self.mods.read(cx).in_the_way(cx);
        let theme = cx.global::<Theme>();
        let (win, ink, family, body) = (
            theme.colours.win,
            theme.colours.ink,
            theme.family.clone(),
            theme.body(),
        );
        if let Some(first) = self.first_run.clone() {
            let theme = cx.global::<Theme>();
            return div()
                .relative()
                .size_full()
                .bg(theme.colours.win)
                .text_color(theme.colours.ink)
                .font_family(theme.family.clone())
                .text_size(theme.body())
                .on_action(|_: &super::focus::Next, window, cx| window.focus_next(cx))
                .on_action(|_: &super::focus::Previous, window, cx| window.focus_prev(cx))
                .child(first)
                .child(self.notices.clone())
                .child(self.menu.clone())
                .child(self.prompt.clone());
        }
        super::keys::developer_tools(div())
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .on_action(|_: &super::focus::Next, window, cx| window.focus_next(cx))
            .on_action(|_: &super::focus::Previous, window, cx| window.focus_prev(cx))
            .on_action(cx.listener(|shell, _: &super::keys::ShowKeys, window, cx| {
                shell.open_keys(window, cx)
            }))
            .on_action(
                cx.listener(|shell, _: &super::keys::OpenCommands, window, cx| {
                    shell.open_commands(window, cx)
                }),
            )
            .on_action(
                cx.listener(|shell, _: &super::keys::Section1, _, cx| shell.go(Section::Mods, cx)),
            )
            .on_action(cx.listener(|shell, _: &super::keys::Section2, _, cx| {
                shell.go(Section::Downloads, cx)
            }))
            .on_action(cx.listener(|shell, _: &super::keys::Section3, _, cx| {
                shell.go(Section::Textures, cx)
            }))
            .on_action(cx.listener(|shell, _: &super::keys::Section4, _, cx| {
                shell.go(Section::Workshop, cx)
            }))
            .on_action(
                cx.listener(|shell, _: &super::keys::Section5, _, cx| shell.go(Section::Saves, cx)),
            )
            .on_action(
                cx.listener(|shell, _: &super::keys::Section6, _, cx| shell.go(Section::Logs, cx)),
            )
            .on_action(
                cx.listener(|shell, _: &super::keys::Section7, _, cx| shell.go(Section::Find, cx)),
            )
            .on_action(cx.listener(|shell, _: &super::keys::Section8, _, cx| {
                shell.go(Section::Settings, cx)
            }))
            .on_action(
                cx.listener(|shell, _: &super::find::FocusSearch, window, cx| {
                    shell.go(Section::Find, cx);
                    let find = shell.find(cx);
                    find.update(cx, |find, cx| find.focus_search(window, cx));
                }),
            )
            .capture_any_mouse_down(|_, _, cx| super::tip::forget_hint(cx))
            .capture_key_down(|_, _, cx| super::tip::forget_hint(cx))
            .child(
                gpui::canvas(
                    |_, _, _| (),
                    |_, _, window, _| {
                        window.on_mouse_event(|_: &gpui::ScrollEvent, phase, _, cx| {
                            if phase.capture() {
                                super::tip::forget_hint(cx);
                            }
                        })
                    },
                )
                .absolute()
                .size_0(),
            )
            .bg(win)
            .text_color(ink)
            .font_family(family)
            .text_size(body)
            .child(top_bar(
                super::top_bar::TopState {
                    section: self.section,
                    demo: self.demo,
                    in_the_way,
                    behind: super::counts::read(cx).behind,
                    running: self.game.read(cx).running(),
                    lists_view: self.lists.clone(),
                    keys: self.bar_keys.clone(),
                },
                window,
                cx,
            ))
            .children(self.hunt_bar(cx))
            .child(self.shown_faded())
            .child(self.status_bar(cx))
            .children(self.before_play_popup(cx))
            .child(self.lists.clone())
            .children(self.keys_sheet(window, cx))
            .children(self.closing_commands.clone())
            .children(self.commands.clone())
            .child(self.notices.clone())
            .child(self.menu.clone())
            .child(self.prompt.clone())
    }
}

impl Shell {
    fn shown_faded(&self) -> gpui::AnyElement {
        let shown = div().flex_1().min_h_0().children(self.shown());
        match self.shown_turn {
            0 => shown.into_any_element(),
            turn => super::motion::appear(shown, ("appear", turn)).into_any_element(),
        }
    }

    /// The view of the section shown, once it is made.
    fn shown(&self) -> Option<gpui::AnyView> {
        match self.section {
            Section::Mods => Some(self.mods.clone().into()),
            Section::Downloads => self.downloads.clone().map(Into::into),
            Section::Textures => self.textures.clone().map(Into::into),
            Section::Saves => self.saves.clone().map(Into::into),
            Section::Logs => self.logs.clone().map(Into::into),
            Section::Find => self.find.clone().map(Into::into),
            Section::Workshop => self.workshop.clone().map(Into::into),
            Section::Settings => self.settings.clone().map(Into::into),
        }
    }
}

#[cfg(test)]
thread_local! {
    /// How often the shell was rendered, as a test of what motion draws again.
    pub(crate) static RENDERS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
impl Shell {
    pub(crate) fn section(&self) -> Section {
        self.section
    }

    pub(crate) fn behind(&self, cx: &gpui::App) -> usize {
        super::counts::read(cx).behind
    }
}

/// The game's logs copied among the kept ones at the start and each time they change.
fn keep_logs(cx: &mut Context<Shell>) {
    super::app(cx).keep_logs_soon();
    super::hear(cx, |_: &mut Shell, changes, cx| {
        if changes.contains(crate::changes::Changes::LOGS) {
            super::app(cx).keep_logs_soon();
        }
    });
}

/// A newer RimStack told once the window has settled, when Settings asks for the check.
fn check_release(cx: &mut Context<Shell>) {
    use super::notices;
    use crate::words::put;
    use rimstack_core::say;
    const SETTLED: std::time::Duration = std::time::Duration::from_secs(4);
    let app = super::app(cx);
    if !app.settings_report().self_check {
        return;
    }
    cx.spawn(async move |shell, cx| {
        cx.background_executor().timer(SETTLED).await;
        let _ = shell.update(cx, |_, cx| {
            let nightly = app.settings().nightly_updates();
            crate::jobs::run(
                "release check",
                cx,
                move || crate::workspace::newest_release(nightly),
                |shell, found, cx| {
                    if let crate::reports::ReleaseSaid::Newer { version, page } = found {
                        shell.release = Some((version.clone(), page.clone()));
                        cx.notify();
                        let running = super::app(cx).settings_report().version;
                        notices::offer(
                            put(&say!(
                                "RimStack {} is published; this is {}",
                                version,
                                running
                            )),
                            put(&say!("Open its page")),
                            move |_, cx| cx.open_url(&page),
                            cx,
                        );
                    }
                },
            );
        });
    })
    .detach();
}

impl Shell {
    fn start_first_run(&mut self, step: usize, cx: &mut Context<Self>) {
        let first = cx.new(|cx| super::first_run::FirstRun::new(step, cx));
        cx.subscribe(
            &first,
            |shell, _, started: &super::first_run::Started, cx| {
                shell.end_first_run(started.with_data, started.with_rimsort, cx)
            },
        )
        .detach();
        cx.observe(&self.mods, |shell, mods, cx| {
            let Some(first) = shell.first_run.clone() else {
                return;
            };
            let (active, inactive, problem) = mods.read(cx).read_as();
            first.update(cx, |first, cx| {
                first.set_mods(active, inactive, problem, cx)
            });
        })
        .detach();
        self.first_run = Some(first);
        cx.notify();
    }

    /// Start: the setup is done, the community data fetched and RimSort's rules read when asked.
    fn end_first_run(&mut self, with_data: bool, with_rimsort: bool, cx: &mut Context<Self>) {
        use super::notices;
        use crate::words::put;
        let app = super::app(cx);
        if let Err(problem) = app.keep_setting("setupDone", "yes") {
            notices::say(crate::words::said_of(&problem), notices::Tone::Err, cx);
        }
        let overview = self.mods.read(cx).overview();
        let missing = overview.is_none_or(|o| o.community_mods.is_none() || o.works_on == 0);
        if with_data && missing {
            let fetching = app.clone();
            crate::jobs::run(
                "community download",
                cx,
                move || fetching.download_databases(),
                |_, done, cx| {
                    if let Err(problem) = done {
                        notices::say(crate::words::said_of(&problem), notices::Tone::Err, cx);
                    }
                },
            );
        }
        if with_rimsort {
            let said = app.import_rimsort_rules();
            notices::say(put(&said), notices::Tone::Ok, cx);
        }
        self.first_run = None;
        self.mods.update(cx, |mods, cx| mods.start_hints(cx));
        cx.notify();
    }
}
