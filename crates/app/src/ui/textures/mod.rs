//! How many textures have a DDS beside them, per mod; converting, removing
//! and skipping; and the DDS files of every mod the game would refuse.

mod screen;

use super::rows::Scrolling;
use super::theme::Theme;
use crate::changes::Changes;
use crate::words::put;
use crate::workspace::textures::Refused;
use gpui::{Context, Window};
use rimstack_core::base::sentence::Sentence;
use rimstack_core::mods::library::ModId;
use rimstack_core::mods::textures::Row;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const READS: Changes = Changes::LIBRARY
    .or(Changes::TEXTURES)
    .or(Changes::LOAD_ORDER)
    .or(Changes::LANGUAGE);

pub struct Textures {
    unseen: crate::ui::section::Unseen,
    demo: bool,
    rows: Rc<Vec<Row>>,
    busy: bool,
    /// Counted again once the count under way ends.
    stale: bool,
    progress: String,
    found: Option<Rc<Vec<Refused>>>,
    auditing: bool,
    active_only: bool,
    selected: Option<ModId>,
    confirm_remove: bool,
    pub(crate) stop: Arc<AtomicBool>,
    scroll: Scrolling,
    focus: gpui::FocusHandle,
}

impl Textures {
    pub fn new(demo: bool, cx: &mut Context<Self>) -> Self {
        let theme = cx.global::<Theme>().clone();
        let scroll: Scrolling = Scrolling::default();
        scroll
            .borrow_mut()
            .set_reduce_motion(theme.appearance.reduce_motion);
        let mut textures = Self {
            unseen: Default::default(),
            demo,
            rows: Rc::default(),
            busy: false,
            stale: true,
            progress: String::new(),
            found: None,
            auditing: false,
            active_only: true,
            selected: None,
            confirm_remove: false,
            stop: Arc::default(),
            scroll,
            focus: cx.focus_handle(),
        };
        if !demo {
            textures.hear(cx);
            textures.count(cx);
        }
        textures
    }

    fn hear(&self, cx: &mut Context<Self>) {
        super::hear(cx, READS, Self::heard);
    }

    fn heard(&mut self, changes: Changes, cx: &mut Context<Self>) {
        let Some(changes) = self.unseen.pass(changes) else {
            return;
        };
        if changes.contains(Changes::LIBRARY | Changes::TEXTURES | Changes::LOAD_ORDER) {
            self.stale = true;
            self.count(cx);
        } else if changes.contains(Changes::LANGUAGE) {
            cx.notify();
        }
    }

    /// Every mod's textures counted, off the window's thread; one count at a time.
    fn count(&mut self, cx: &mut Context<Self>) {
        if !self.stale || self.busy || self.demo {
            return;
        }
        self.stale = false;
        self.busy = true;
        let app = super::app(cx);
        crate::jobs::run(
            "textures count",
            cx,
            move || app.texture_rows(),
            |textures, rows, cx| {
                textures.rows = Rc::new(rows);
                textures.busy = false;
                textures.count(cx);
                cx.notify();
            },
        );
        cx.notify();
    }

    /// Converts the mods `ids`, or removes what RimStack made in them, telling how far it got.
    fn work(&mut self, ids: Vec<ModId>, remove: bool, cx: &mut Context<Self>) {
        if self.busy || ids.is_empty() || self.demo {
            return;
        }
        self.busy = true;
        self.stop.store(false, Ordering::Relaxed);
        let (app, stop) = (super::app(cx), self.stop.clone());
        let (tell, mut told) =
            futures::channel::mpsc::unbounded::<Result<Sentence, (Vec<Row>, Sentence)>>();
        crate::jobs::spawn("textures convert", move || {
            let progress = |text: Sentence| {
                let _ = tell.unbounded_send(Ok(text));
            };
            let note = app.convert_textures(&ids, remove, &stop, &progress);
            let _ = tell.unbounded_send(Err((app.texture_rows(), note)));
        });
        cx.spawn(async move |textures, cx| {
            use futures::StreamExt;
            while let Some(heard) = told.next().await {
                let going = textures.update(cx, |textures, cx| {
                    match heard {
                        Ok(progress) => textures.progress = put(&progress),
                        Err((rows, note)) => {
                            textures.rows = Rc::new(rows);
                            textures.stale = false;
                            textures.progress = put(&note);
                            textures.busy = false;
                        }
                    }
                    cx.notify();
                });
                if going.is_err() {
                    return;
                }
            }
        })
        .detach();
        cx.notify();
    }

    /// What the status bar says of the count or the conversion under way.
    pub(crate) fn task_said(&self) -> Option<String> {
        use rimstack_core::say;
        self.busy.then(|| match self.progress.is_empty() {
            true => put(&say!("Counting textures…")),
            false => self.progress.clone(),
        })
    }

    fn skip(&mut self, id: ModId, skip: bool, cx: &mut Context<Self>) {
        if let Err(problem) = super::app(cx).skip_textures(id, skip) {
            rimstack_core::problem!("{problem}");
        }
    }

    fn audit(&mut self, cx: &mut Context<Self>) {
        if self.auditing || self.demo {
            return;
        }
        self.auditing = true;
        let app = super::app(cx);
        crate::jobs::run(
            "dds audit",
            cx,
            move || app.refused_dds(),
            |textures, found, cx| {
                textures.found = Some(Rc::new(found));
                textures.auditing = false;
                cx.notify();
            },
        );
        cx.notify();
    }

    fn set_refused_aside(&mut self, cx: &mut Context<Self>) {
        let Some(found) = self.found.take() else {
            return;
        };
        let failed = super::app(cx).set_dds_aside(&found);
        if failed > 0 {
            rimstack_core::problem!("{failed} DDS files could not be set aside");
        }
        self.audit(cx);
    }

    fn ask_refused_aside(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use super::prompt::{self, Line, Question};
        use rimstack_core::say;
        let Some(found) = self.found.clone().filter(|f| !f.is_empty()) else {
            return;
        };
        let me = cx.entity().downgrade();
        let question = Question {
            title: put(&say!("Set {} DDS files aside", found.len())),
            lines: vec![
                Line::Text(put(&say!(
                    "Each is renamed to .dds.rimstack-bad inside its own mod's folder, so the game loads the PNG beside it. The first of them:"
                ))),
                Line::Path(found[0].file.clone()),
            ],
            label: put(&say!("Set aside")),
            danger: false,
            run: Box::new(move |_, cx| {
                let _ = me.update(cx, |textures, cx| textures.set_refused_aside(cx));
            }),
        };
        prompt::ask(question, window, cx);
    }
}

/// `megabytes(n)`: KB under a tenth of a megabyte, one decimal under ten, whole above.
fn megabytes(n: f64) -> String {
    use rimstack_core::say;
    if n <= 0.0 {
        return String::new();
    }
    if n < 0.1 {
        return put(&say!("{} KB", ((n * 1024.0).round() as usize).max(1)));
    }
    match n < 10.0 {
        true => put(&say!("{} MB", format!("{n:.1}"))),
        false => put(&say!("{} MB", crate::words::count(n.round() as usize))),
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests;

impl crate::ui::section::Shown for Textures {
    fn set_shown(&mut self, shown: bool, cx: &mut Context<Self>) {
        if let Some(missed) = self.unseen.shown(shown) {
            self.heard(missed, cx);
        }
    }
}
