//! The game's saves, newest first, each against the load order now; the chosen
//! one's colony, what it lacks, and its list to use or keep.

mod screen;

use super::rows::Scrolling;
use super::text_field::{Edited, Entered, TextField};
use super::theme::Theme;
use crate::changes::Changes;
use crate::jobs::Lane;
use crate::workspace::saves::SavesReport;
use gpui::{AppContext, Context, Entity, EventEmitter, FocusHandle};
use std::rc::Rc;

/// Asked of the window: the load order becomes these mods, unsaved.
pub struct UseList(pub Vec<String>);

pub struct Saves {
    demo: bool,
    report: Rc<SavesReport>,
    busy: bool,
    stale: bool,
    reading: Lane,
    /// With the RimSaves mod, the folder shown; empty for every one.
    folder: String,
    filter: Entity<TextField>,
    filter_text: String,
    selected: usize,
    all_missing: bool,
    all_added: bool,
    list_name: Entity<TextField>,
    list_problem: String,
    scroll: Scrolling,
    side_paging: super::page::Paging,
    thumbs: super::thumbs::Shared,
    focus: FocusHandle,
}

impl EventEmitter<UseList> for Saves {}

impl Saves {
    pub fn new(demo: bool, cx: &mut Context<Self>) -> Self {
        let theme = cx.global::<Theme>().clone();
        let filter = cx.new(|cx| TextField::new(theme.body(), cx));
        cx.subscribe(&filter, |saves, field, _: &Edited, cx| {
            saves.filter_text = field.read(cx).text().to_string();
            saves.selected = 0;
            cx.notify();
        })
        .detach();
        let list_name = cx.new(|cx| TextField::new(theme.body(), cx));
        cx.observe(&list_name, |_, _, cx| cx.notify()).detach();
        cx.subscribe(&list_name, |saves, _, _: &Entered, cx| saves.keep_list(cx))
            .detach();
        let scroll = Scrolling::default();
        scroll
            .borrow_mut()
            .set_reduce_motion(theme.appearance.reduce_motion);
        let mut saves = Self {
            demo,
            report: Rc::default(),
            busy: false,
            stale: true,
            reading: Lane::default(),
            folder: String::new(),
            filter,
            filter_text: String::new(),
            selected: 0,
            all_missing: false,
            all_added: false,
            list_name,
            list_problem: String::new(),
            scroll,
            side_paging: super::page::Paging::new(theme.appearance.reduce_motion),
            thumbs: super::thumbs::Shared::default(),
            focus: cx.focus_handle(),
        };
        if !demo {
            saves.hear(cx);
            saves.read(cx);
        }
        saves
    }

    fn hear(&self, cx: &mut Context<Self>) {
        super::hear(cx, Self::heard);
    }

    fn heard(&mut self, changes: Changes, cx: &mut Context<Self>) {
        if changes.contains(Changes::LIBRARY | Changes::LOAD_ORDER | Changes::SAVES) {
            self.stale = true;
            self.read(cx);
        } else if changes.contains(Changes::LANGUAGE) {
            cx.notify();
        }
    }

    /// Read again now, as the command list asks.
    pub fn reread(&mut self, cx: &mut Context<Self>) {
        self.stale = true;
        self.read(cx);
    }

    fn read(&mut self, cx: &mut Context<Self>) {
        if !self.stale || self.demo {
            return;
        }
        self.stale = false;
        self.busy = true;
        let app = super::app(cx);
        self.reading.run(
            "saves",
            cx,
            move |_| app.saves_report(),
            |saves, report, cx| {
                saves.report = Rc::new(report);
                saves.busy = false;
                cx.notify();
            },
        );
        cx.notify();
    }

    /// The chosen save's list kept under the name typed.
    fn keep_list(&mut self, cx: &mut Context<Self>) {
        let name = self.list_name.read(cx).text().trim().to_string();
        let shown = crate::workspace::saves::shown(&self.report, &self.folder, &self.filter_text);
        let Some(ids) = shown
            .saves
            .get(self.selected)
            .and_then(|s| s.row.compared.as_ref())
            .map(|c| c.ids.clone())
        else {
            return;
        };
        if name.is_empty() {
            return;
        }
        match super::app(cx).keep_list(&name, Some(ids)) {
            Ok(()) => {
                self.list_problem.clear();
                self.list_name
                    .update(cx, |field, cx| field.set_text("", cx));
            }
            Err(problem) => self.list_problem = crate::words::put(problem.sentence()),
        }
        cx.notify();
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
