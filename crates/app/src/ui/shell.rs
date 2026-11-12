//! `Main.qml`: the top bar, the section shown, the status bar.

use super::Options;
use super::downloads::{BehindActive, Downloads, ShowMod};
use super::mods::{Mods, ShowSection};
use super::status_bar::status_bar;
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
    demo: bool,
    pub(crate) mods: Entity<Mods>,
    /// Made with the window, as Qt's `DownloadsData` was: it counts the updates the top bar and
    /// the Mods tab show. What it reads of the `App` it reads off the window's thread.
    downloads: Option<Entity<Downloads>>,
    /// The active mods with a newer Workshop version, on the Downloads tab.
    behind: usize,
    /// Made the first time Settings is shown; `part` is the one it opens on.
    pub(crate) settings: Option<Entity<super::settings::Settings>>,
    part: usize,
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
    view: usize,
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
            mods.update(cx, |mods, cx| {
                mods.choose_nth(Side::Active, 14, false, cx);
                for at in [15, 16, 19] {
                    mods.choose_nth(Side::Active, at, true, cx);
                }
            });
        }
        cx.subscribe(&mods, |shell, _, ShowSection(section), cx| {
            shell.go(*section, cx)
        })
        .detach();
        let section = options.section;
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
        let (in_the_way, last_start) = {
            let mods = self.mods.read(cx);
            (mods.in_the_way(cx), mods.last_start())
        };
        let theme = cx.global::<Theme>();
        let (win, ink, family, body) = (
            theme.colours.win,
            theme.colours.ink,
            theme.family.clone(),
            theme.body(),
        );
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .on_action(|_: &super::focus::Next, window, cx| window.focus_next(cx))
            .on_action(|_: &super::focus::Previous, window, cx| window.focus_prev(cx))
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
                self.section,
                self.demo,
                in_the_way,
                self.behind,
                window,
                cx,
            ))
            .child(div().flex_1().min_h_0().children(self.shown()))
            .child(status_bar(self.demo, last_start, cx))
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
