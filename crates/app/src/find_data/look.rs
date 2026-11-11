use super::{int, qobject};
use crate::qml::values::to_qml;
use crate::workspace::find::Look;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QVariant;
use rimstack_core::base::sentence::Sentence;
use rimstack_core::{say, say_n};
use std::path::PathBuf;

#[derive(Default)]
pub(super) struct Viewing {
    pub file: PathBuf,
    pub hit: usize,
    pub found: Vec<usize>,
    pub at: Option<usize>,
    pub said: Option<Sentence>,
}

fn said_of(text: &str, found: usize, at: Option<usize>) -> Option<Sentence> {
    match (text.is_empty(), at) {
        (true, _) => None,
        (false, _) if found == 0 => Some(say!("No matches")),
        (false, Some(at)) => Some(say!("{} of {}", at + 1, found)),
        (false, None) => Some(say_n!(found, "{} match", "{} matches")),
    }
}

impl qobject::FindData {
    pub(super) fn show_inside(mut self: Pin<&mut Self>) {
        let said = self
            .viewing
            .said
            .as_ref()
            .map(crate::words::said)
            .unwrap_or_default();
        let found = int(self.viewing.found.len());
        self.as_mut().set_inside_found(found);
        self.set_inside_said(said);
    }

    pub(super) fn select(mut self: Pin<&mut Self>, row: i32) {
        let located = self
            .row(row)
            .filter(|at| !at.is_mod())
            .and_then(|at| self.app().find_located(at));
        let Some((file, hit)) = located else {
            self.previews.cancel();
            self.as_mut().set_current(-1);
            self.as_mut().rust_mut().get_mut().viewing = Viewing::default();
            self.as_mut().show_inside();
            return self.set_preview(QVariant::default());
        };
        self.as_mut().set_current(row);
        let me = self.as_mut().rust_mut().get_mut();
        me.viewing = Viewing {
            file,
            hit,
            ..Viewing::default()
        };
        self.look(0, false);
    }

    /// `again` keeps the lines found instead of searching the file for them.
    fn look(self: Pin<&mut Self>, step: i32, again: bool) {
        let app = self.app();
        let viewing = &self.viewing;
        let (file, hit) = (viewing.file.clone(), viewing.hit);
        let look = Look {
            text: self.inside_text.to_string(),
            found: again.then(|| viewing.found.clone()),
            at: viewing.at,
            step,
        };
        self.previews.run(
            "find preview",
            self.qt_thread(),
            move |_| app.find_preview(&file, hit, look),
            |mut me, looked| {
                let text = me.inside_text.to_string();
                let viewing = &mut me.as_mut().rust_mut().get_mut().viewing;
                viewing.said = said_of(&text, looked.found.len(), looked.at);
                viewing.found = looked.found;
                viewing.at = looked.at;
                me.as_mut().show_inside();
                me.set_preview(to_qml(&looked.shown));
            },
        );
    }

    pub(super) fn inside_typed(mut self: Pin<&mut Self>) {
        if self.viewing.file.as_os_str().is_empty() {
            return;
        }
        self.as_mut().rust_mut().get_mut().viewing.at = None;
        self.look(0, false);
    }

    pub(super) fn inside_step(self: Pin<&mut Self>, by: i32) {
        if self.viewing.file.as_os_str().is_empty() || self.viewing.found.is_empty() {
            return;
        }
        self.look(by, true);
    }

    pub(super) fn show_mods(self: Pin<&mut Self>) {
        let mods = self.app().find_results(|r| r.found_mods());
        self.set_mods(to_qml(&mods));
    }

    pub(super) fn pick_mod(mut self: Pin<&mut Self>, group: i32) {
        let rows = self
            .app()
            .find_change(|results| results.set_pick(usize::try_from(group).ok()));
        self.as_mut().reach(rows);
        self.show_mods();
    }
}
