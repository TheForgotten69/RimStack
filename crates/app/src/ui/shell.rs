//! `Main.qml`: the top bar, the section shown, the status bar.

use super::Options;
use super::downloads::{BehindActive, Downloads, ShowMod};
use super::mods::{Mods, ShowSection};
use super::theme::Theme;
use super::top_bar::top_bar;
use crate::workspace::load_order::Side;
use gpui::{
    AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Window, div,
};

/// The sections made on their first visit, as `Theme.sections` numbers them.
const TEXTURES: usize = 2;
const WORKSHOP: usize = 3;
const SAVES: usize = 4;
const LOGS: usize = 5;
const FIND: usize = 6;
const SETTINGS: usize = 7;

pub struct Shell {
    section: usize,
    pub(crate) demo: bool,
    pub(crate) mods: Entity<Mods>,
    /// Made with the window, as Qt's `DownloadsData` was: it counts the updates the top bar and
    /// the Mods tab show. What it reads of the `App` it reads off the window's thread.
    pub(crate) downloads: Option<Entity<Downloads>>,
    /// The active mods with a newer Workshop version, on the Downloads tab.
    pub(crate) behind: usize,
    /// Made the first time Settings is shown; `part` is the one it opens on.
    pub(crate) settings: Option<Entity<super::settings::Settings>>,
    pub(crate) part: usize,
    /// Made the first time Logs is shown; `view` is the one it opens on.
    pub(crate) logs: Option<Entity<super::logs::Logs>>,
    /// Made the first time Saves is shown.
    pub(crate) saves: Option<Entity<super::saves::Saves>>,
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
        let mods = cx.new(|cx| Mods::new(demo, cx));
        let focus = mods.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        if let Some(text) = &options.filter {
            mods.update(cx, |mods, cx| mods.filter_by(text, cx));
        }
        if let Some(row) = options.pick {
            let tab = options.tab;
            mods.update(cx, |mods, cx| mods.pick_row(Side::Active, row, tab, cx));
        }
        if options.bench {
            let mods = mods.clone();
            cx.spawn_in(window, async move |_, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(300))
                    .await;
                let _ = cx.update(|_, cx| mods.update(cx, |mods, cx| mods.start_bench(cx)));
            })
            .detach();
        }
        if options.several {
            mods.update(cx, |mods, cx| mods.choose_several(cx));
        }
        if options.still {
            mods.update(cx, |mods, _| mods.leave_off());
        }
        if options.drawer {
            mods.update(cx, |mods, _| mods.drawer_open = true);
        }
        if options.comparing {
            mods.update(cx, |mods, cx| mods.compare_first(cx));
        }
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
            section: 0,
            demo,
            mods,
            downloads: None,
            behind: 0,
            settings: None,
            part: options.part,
            logs: None,
            textures: None,
            saves: None,
            find: None,
            workshop: None,
            view: options.view,
            game,
            lists,
            commands: None,
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
        };
        shell.downloads(cx);
        if !demo {
            keep_logs(cx);
        }
        shell.go(section, cx);
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
        if let Some(asked) = &options.browse {
            let workshop = shell.workshop(cx);
            workshop.update(cx, |workshop, cx| workshop.show_asked(asked, cx));
        }
        if let Some(text) = &options.find {
            let find = shell.find(cx);
            find.update(cx, |find, cx| find.search_for(text, Some(2), cx));
        }
        shell.show_asked(options, window, cx);
        shell
    }

    /// What `--notices` and `--open=` ask to be shown, as `Main.qml` showed them for pictures.
    fn show_asked(&mut self, options: &Options, window: &mut Window, cx: &mut Context<Self>) {
        use super::notices::{self, Tone};
        use super::prompt::{self, Line, Question};
        use crate::words::put;
        use rimstack_core::{say, say_n};
        if options.notices {
            notices::say(put(&say!("Saved")), Tone::Ok, cx);
            notices::offer(
                put(&say_n!(3, "Deactivated {} mod", "Deactivated {} mods")),
                put(&say!("Undo")),
                |_, _| {},
                cx,
            );
        }
        match options.open.as_deref() {
            Some("play") => self.before_play = true,
            Some("problems") => self.mods.update(cx, |mods, cx| mods.open_problems(cx)),
            Some("keys") => self.open_keys(window, cx),
            Some("commands") => self.open_commands(window, cx),
            Some("lists") => self.open_popup(super::lists::Which::Lists, window, cx),
            Some("history") => self.open_popup(super::lists::Which::History, window, cx),
            Some("view") => self
                .mods
                .update(cx, |mods, _| mods.menu_wanted = Some("view")),
            Some("order") => self
                .mods
                .update(cx, |mods, _| mods.menu_wanted = Some("order")),
            _ => {}
        }
        if options.open.as_deref() == Some("ask") {
            let question = Question {
                title: put(&say!("Set the mods' settings aside")),
                lines: vec![
                    Line::Text(put(&say!("Every Mod_*.xml is moved out of"))),
                    Line::Path("/home/example/.config/unity3d/Ludeon Studios/RimWorld by Ludeon Studios/Config".into()),
                    Line::Text(put(&say!("Mods start from their defaults. Moving the files back restores them."))),
                ],
                label: put(&say!("Set aside")),
                danger: false,
                run: Box::new(|_, _| {}),
            };
            prompt::ask(question, window, cx);
        }
    }

    pub(crate) fn downloads(&mut self, cx: &mut Context<Self>) -> Entity<Downloads> {
        if let Some(downloads) = &self.downloads {
            return downloads.clone();
        }
        let (demo, mods) = (self.demo, self.mods.clone());
        let downloads = cx.new(|cx| Downloads::new(demo, mods, cx));
        cx.subscribe(&downloads, |shell, _, ShowMod(mod_id), cx| {
            shell.go(0, cx);
            let mod_id = *mod_id;
            shell.mods.update(cx, |mods, cx| mods.show_mod(mod_id, cx));
        })
        .detach();
        cx.subscribe(&downloads, |shell, _, BehindActive(count), cx| {
            shell.behind = *count;
            let count = *count;
            shell.mods.update(cx, |mods, cx| mods.set_behind(count, cx));
            cx.notify();
        })
        .detach();
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
            shell.go(0, cx);
        })
        .detach();
        self.find = Some(find.clone());
        find
    }

    pub fn go(&mut self, section: usize, cx: &mut Context<Self>) {
        self.section = section;
        if section == WORKSHOP {
            self.workshop(cx);
        }
        if section == FIND {
            self.find(cx);
        }
        if section == 1 {
            self.downloads(cx);
        }
        if section == SAVES && self.saves.is_none() {
            let demo = self.demo;
            let saves = cx.new(|cx| super::saves::Saves::new(demo, cx));
            cx.subscribe(&saves, |shell, _, super::saves::UseList(ids), cx| {
                let ids = ids.clone();
                shell.mods.update(cx, |mods, cx| mods.use_list(ids, cx));
                shell.go(0, cx);
            })
            .detach();
            self.saves = Some(saves);
        }
        if section == TEXTURES && self.textures.is_none() {
            let demo = self.demo;
            self.textures = Some(cx.new(|cx| super::textures::Textures::new(demo, cx)));
        }
        if section == LOGS && self.logs.is_none() {
            let (demo, view) = (self.demo, self.view);
            let logs = cx.new(|cx| super::logs::Logs::new(demo, view, cx));
            cx.subscribe(&logs, |shell, _, super::logs::ShowMod(mod_id), cx| {
                let mod_id = *mod_id;
                shell.go(0, cx);
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
                shell.go(0, cx);
            })
            .detach();
            self.logs = Some(logs);
        }
        if section == SETTINGS && self.settings.is_none() {
            let (demo, part, mods) = (self.demo, self.part, self.mods.clone());
            let settings = cx.new(|cx| super::settings::Settings::new(demo, part, mods, cx));
            cx.subscribe(&settings, |shell, _, super::settings::UseList(ids), cx| {
                let ids = ids.clone();
                shell.mods.update(cx, |mods, cx| mods.use_list(ids, cx));
                shell.go(0, cx);
            })
            .detach();
            self.settings = Some(settings);
        }
        cx.notify();
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        div()
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
            .on_action(cx.listener(|shell, _: &super::keys::Section1, _, cx| shell.go(0, cx)))
            .on_action(cx.listener(|shell, _: &super::keys::Section2, _, cx| shell.go(1, cx)))
            .on_action(cx.listener(|shell, _: &super::keys::Section3, _, cx| shell.go(2, cx)))
            .on_action(cx.listener(|shell, _: &super::keys::Section4, _, cx| shell.go(3, cx)))
            .on_action(cx.listener(|shell, _: &super::keys::Section5, _, cx| shell.go(4, cx)))
            .on_action(cx.listener(|shell, _: &super::keys::Section6, _, cx| shell.go(5, cx)))
            .on_action(cx.listener(|shell, _: &super::keys::Section7, _, cx| shell.go(6, cx)))
            .on_action(cx.listener(|shell, _: &super::keys::Section8, _, cx| shell.go(7, cx)))
            .on_action(
                cx.listener(|shell, _: &super::find::FocusSearch, window, cx| {
                    shell.go(FIND, cx);
                    let find = shell.find(cx);
                    find.update(cx, |find, cx| find.focus_search(window, cx));
                }),
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
                    behind: self.behind,
                    running: self.game.read(cx).running(),
                    lists_view: self.lists.clone(),
                },
                window,
                cx,
            ))
            .children(self.hunt_bar(cx))
            .child(div().flex_1().min_h_0().children(self.shown()))
            .child(self.status_bar(cx))
            .children(self.before_play_popup(cx))
            .child(self.lists.clone())
            .children(self.keys_sheet(window, cx))
            .children(self.commands.clone())
            .child(self.notices.clone())
            .child(self.menu.clone())
            .child(self.prompt.clone())
    }
}

impl Shell {
    /// The view of the section shown, once it is made.
    fn shown(&self) -> Option<gpui::AnyView> {
        match self.section {
            0 => Some(self.mods.clone().into()),
            1 => self.downloads.clone().map(Into::into),
            TEXTURES => self.textures.clone().map(Into::into),
            SAVES => self.saves.clone().map(Into::into),
            LOGS => self.logs.clone().map(Into::into),
            FIND => self.find.clone().map(Into::into),
            WORKSHOP => self.workshop.clone().map(Into::into),
            SETTINGS => self.settings.clone().map(Into::into),
            _ => None,
        }
    }
}

#[cfg(test)]
impl Shell {
    pub(crate) fn section(&self) -> usize {
        self.section
    }

    pub(crate) fn behind(&self) -> usize {
        self.behind
    }
}

/// The game's logs copied among the kept ones at the start and each time they change, as Qt's
/// `LogsData` did from the window's start.
fn keep_logs(cx: &mut Context<Shell>) {
    let app = super::app(cx);
    let keeping = app.clone();
    crate::jobs::spawn("keep logs", move || keeping.keep_logs());
    let kept = app.clone();
    app.listen(move |changes| {
        if changes.contains(crate::changes::Changes::LOGS) {
            let kept = kept.clone();
            crate::jobs::spawn("keep logs", move || kept.keep_logs());
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
                |_, found, cx| {
                    if let crate::reports::ReleaseSaid::Newer { version, page } = found {
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
        cx.notify();
    }
}
