//! The window's sections: Mods and Downloads made with the window, the others the first time
//! they are asked for; each told when it is shown or hidden, and what the bars show of it put in
//! `around` as it changes.

use super::downloads::Downloads;
use super::find::Find;
use super::logs::Logs;
use super::mods::Mods;
use super::saves::Saves;
use super::section::{Section, Shown};
use super::settings::Settings;
use super::textures::Textures;
use super::workshop::Workshop;
use gpui::{App as Cx, AppContext, Entity};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub struct Sections {
    demo: bool,
    shown: Section,
    pub(crate) mods: Entity<Mods>,
    pub(crate) downloads: Option<Entity<Downloads>>,
    pub(crate) settings: Option<Entity<Settings>>,
    /// The part Settings opens on.
    pub(crate) part: usize,
    pub(crate) logs: Option<Entity<Logs>>,
    /// The view Logs opens on.
    pub(crate) view: usize,
    pub(crate) saves: Option<Entity<Saves>>,
    pub(crate) workshop: Option<Entity<Workshop>>,
    pub(crate) find: Option<Entity<Find>>,
    pub(crate) textures: Option<Entity<Textures>>,
    /// How the textures' work under way is stopped.
    pub(crate) textures_stop: Option<Arc<AtomicBool>>,
}

impl Sections {
    pub fn new(demo: bool, part: usize, view: usize, cx: &mut Cx) -> Self {
        let mods = cx.new(|cx| Mods::new(demo, cx));
        cx.observe(&mods, |mods, cx| super::around::from_mods(&mods, cx))
            .detach();
        super::around::from_mods(&mods, cx);
        let mut sections = Self {
            demo,
            shown: Section::Mods,
            mods,
            downloads: None,
            settings: None,
            part,
            logs: None,
            view,
            saves: None,
            workshop: None,
            find: None,
            textures: None,
            textures_stop: None,
        };
        sections.downloads(cx);
        sections
    }

    pub fn shown(&self) -> Section {
        self.shown
    }

    /// The view of the section shown, once it is made.
    pub fn view_shown(&self) -> Option<gpui::AnyView> {
        match self.shown {
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

    /// `section` shown, made if it was not, and the one shown before told it is hidden.
    pub fn show(&mut self, section: Section, cx: &mut Cx) {
        if section != self.shown {
            self.tell_shown(self.shown, false, cx);
        }
        self.shown = section;
        self.make(section, cx);
        self.tell_shown(section, true, cx);
    }

    fn make(&mut self, section: Section, cx: &mut Cx) {
        let demo = self.demo;
        match section {
            Section::Mods => {}
            Section::Downloads => {
                self.downloads(cx);
            }
            Section::Workshop => {
                self.workshop(cx);
            }
            Section::Find => {
                self.find(cx);
            }
            Section::Saves if self.saves.is_none() => {
                self.saves = Some(cx.new(|cx| Saves::new(demo, cx)));
            }
            Section::Textures if self.textures.is_none() => {
                let textures = cx.new(|cx| Textures::new(demo, cx));
                cx.observe(&textures, |textures, cx| {
                    let work = textures.read(cx).task_said();
                    super::around::change(cx, |around| around.textures_work = work);
                })
                .detach();
                self.textures_stop = Some(textures.read(cx).stop.clone());
                self.textures = Some(textures);
            }
            Section::Logs if self.logs.is_none() => {
                let view = self.view;
                self.logs = Some(cx.new(|cx| Logs::new(demo, view, cx)));
            }
            Section::Settings if self.settings.is_none() => {
                let (part, mods) = (self.part, self.mods.clone());
                let settings = cx.new(|cx| Settings::new(demo, part, mods, cx));
                cx.observe(&settings, |settings, cx| {
                    let busy = settings.read(cx).community_busy();
                    super::around::change(cx, |around| around.community_busy = busy);
                })
                .detach();
                self.settings = Some(settings);
            }
            Section::Saves | Section::Textures | Section::Logs | Section::Settings => {}
        }
    }

    pub fn downloads(&mut self, cx: &mut Cx) -> Entity<Downloads> {
        if let Some(downloads) = &self.downloads {
            return downloads.clone();
        }
        let (demo, mods) = (self.demo, self.mods.clone());
        let downloads = cx.new(|cx| Downloads::new(demo, mods, cx));
        cx.observe(&downloads, |downloads, cx| {
            let work = downloads.read(cx).tasks_said();
            super::around::change(cx, |around| around.downloads_work = work);
        })
        .detach();
        self.mods
            .update(cx, |mods, cx| mods.set_downloads(&downloads, cx));
        self.downloads = Some(downloads.clone());
        downloads
    }

    /// Workshop, made hidden when it is not the section shown.
    pub fn workshop(&mut self, cx: &mut Cx) -> Entity<Workshop> {
        if let Some(workshop) = &self.workshop {
            return workshop.clone();
        }
        let (demo, downloads) = (self.demo, self.downloads(cx));
        let workshop = cx.new(|cx| Workshop::new(demo, downloads, cx));
        self.workshop = Some(workshop.clone());
        if self.shown != Section::Workshop {
            self.tell_shown(Section::Workshop, false, cx);
        }
        workshop
    }

    /// Search, made hidden when it is not the section shown.
    pub fn find(&mut self, cx: &mut Cx) -> Entity<Find> {
        if let Some(find) = &self.find {
            return find.clone();
        }
        let (demo, mods) = (self.demo, self.mods.clone());
        let find = cx.new(|cx| Find::new(demo, mods, cx));
        self.find = Some(find.clone());
        if self.shown != Section::Find {
            self.tell_shown(Section::Find, false, cx);
        }
        find
    }

    fn tell_shown(&self, section: Section, shown: bool, cx: &mut Cx) {
        fn tell<V: Shown + 'static>(view: &Option<Entity<V>>, shown: bool, cx: &mut Cx) {
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
