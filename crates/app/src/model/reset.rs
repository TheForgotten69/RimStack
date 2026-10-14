//! What follows every change to a list: its problems found again, the
//! step noted for Undo, and the counts the window shows.

use super::*;

impl qobject::ModListModel {
    /// Change the list in a way that may touch any row.
    pub(crate) fn reset_with<T>(
        mut self: Pin<&mut Self>,
        change: impl FnOnce(&mut GroupedList) -> T,
    ) -> T {
        unsafe { self.as_mut().begin_reset_model() };
        let result = change(&mut self.as_mut().rust_mut().list);
        let twins = rimstack_core::list::twins(self.list.mods());
        let slowest = self.list.mods().iter().filter_map(|m| m.load_micros).max();
        self.as_mut().rust_mut().twins = twins;
        self.as_mut().rust_mut().slowest = slowest;
        // Before the view re-reads its rows, so the markers are right.
        self.as_mut().check_mods();
        // Before the flags, which say what changed since the save.
        let step = self.as_mut().record_step();
        if self.checked || self.installed {
            let flags = self.flags();
            self.as_mut().rust_mut().list.set_flags(flags);
        }
        unsafe { self.as_mut().end_reset_model() };
        self.as_mut().publish_counts();
        if step {
            self.as_mut().session_changed();
        }
        result
    }

    /// Finds the problems of the load order again, when its mods changed:
    /// not on a filter or a collapse.
    fn check_mods(mut self: Pin<&mut Self>) {
        let now: Vec<Option<usize>> = self.list.mods().iter().map(|m| m.installed).collect();
        if !self.checked || now == self.checked_order {
            return;
        }
        let loaded = crate::app().library();
        let Ok((library, _)) = &*loaded else {
            return;
        };
        let (found, ignored) = troubles(library, self.list.mods());
        self.as_mut().rust_mut().troubles = found;
        self.as_mut().rust_mut().version_ignored = ignored;
        self.as_mut().rust_mut().checked_order = now;
        self.as_mut().find_conflicts(loaded.clone());
    }

    /// Every change to the load order is a step that can be undone; a
    /// sort on show is not one until it is applied. Returns whether one
    /// was noted.
    fn record_step(self: Pin<&mut Self>) -> bool {
        let step = self.checked && self.preview.is_none() && !self.batching;
        if step {
            let ids = self.list.mods().iter().map(|m| m.id.clone()).collect();
            crate::app().session().record(ids);
        }
        step
    }

    fn publish_counts(mut self: Pin<&mut Self>) {
        let in_conflict = self.in_conflict().len() as i32;
        self.as_mut().set_conflicts(in_conflict);
        let with = |error: bool| {
            let has = |own: &&Vec<Trouble>| own.iter().any(|t| t.error == error);
            self.troubles.iter().filter(has).count() as i32
        };
        let (errors, warnings) = (with(true), with(false));
        self.as_mut().set_errors(errors);
        self.as_mut().set_warnings(warnings);
        let count = self.list.mods().len() as i32;
        self.as_mut().set_count(count);
        let matching = self.list.matching() as i32;
        self.as_mut().set_matching(matching);
    }
}
