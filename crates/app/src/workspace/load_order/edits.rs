//! Moves within a list and between the two, folding a category or a group
//! away, and the load order's history: Undo, Redo, a list taken whole,
//! the saved one.

use super::{rules, LoadOrder, Side};
use crate::App;
use rimstack_core::base::error::Error;
use rimstack_core::mods::library::ModId;
use rimstack_core::mods::list::{edge_of_run, Category, Mod, Row};
use rimstack_core::rimworld::modsconfig;

impl LoadOrder {
    /// A mod from one place of a list to another, by load-order position.
    pub fn move_mod(&mut self, app: &App, side: Side, from: usize, to: usize) {
        let list = self.list(side);
        if from >= list.mods().len() || from == to {
            return;
        }
        self.end_preview();
        self.list_mut(side).move_mod(from, to);
        self.settle(app);
    }

    /// The mod at `index` of one list into the other: where its category
    /// and, in the load order, its rules put it; or at `to`.
    pub fn take(&mut self, app: &App, from: Side, index: usize, to: Option<usize>) {
        self.end_preview();
        let Some(m) = self.list_mut(from).remove(index) else {
            return;
        };
        let into = from.other();
        self.put(app, into, m, to);
        self.settle(app);
    }

    fn put(&mut self, app: &App, into: Side, m: Mod, to: Option<usize>) {
        let at = self.list_mut(into).insert_in_category(m);
        match (into, to) {
            (_, Some(to)) => {
                let last = self.list(into).mods().len() - 1;
                self.list_mut(into).move_mod(at, to.min(last));
            }
            (Side::Active, None) => self.settle_one(app, at),
            (Side::Inactive, None) => {
                let by_name = self.by_name;
                self.inactive.arrange(by_name);
            }
        }
    }

    /// Moves the mod just added to the load order at `at` to where its
    /// rules allow.
    fn settle_one(&mut self, app: &App, at: usize) {
        let loaded = app.library();
        let Ok((library, _)) = &*loaded else {
            return;
        };
        let found = rules::all_rules(app, &mut self.detected, library, self.active.mods());
        let to = rimstack_core::mods::ordering::place(&found, at, self.active.mods().len());
        self.active.move_mod(at, to);
    }

    /// These mods (by library id) into the other list, in the order they
    /// have, as one step for Undo.
    pub fn take_all(&mut self, app: &App, from: Side, ids: &[ModId]) {
        self.end_preview();
        let mut ids: Vec<ModId> = ids.to_vec();
        ids.sort_by_key(|&id| self.list(from).position_of(id));
        self.batching = true;
        for id in ids {
            if let Some(at) = self.list(from).position_of(id) {
                if let Some(m) = self.list_mut(from).remove(at) {
                    self.put(app, from.other(), m, None);
                }
            }
        }
        self.batching = false;
        self.settle(app);
    }

    /// Each of these mods to the top or bottom of its category's stretch,
    /// keeping their order among themselves, as one step for Undo.
    pub fn to_edge(&mut self, app: &App, ids: &[ModId], top: bool) {
        self.end_preview();
        let mut ids: Vec<ModId> = ids.to_vec();
        ids.sort_by_key(|&id| self.active.position_of(id));
        // The last one moved ends up outermost.
        if top {
            ids.reverse();
        }
        for id in ids {
            if let Some(at) = self.active.position_of(id) {
                let to = edge_of_run(self.active.mods(), at, top);
                self.active.move_mod(at, to);
            }
        }
        self.settle(app);
    }

    pub fn toggle_category(&mut self, app: &App, side: Side, category: Category) {
        self.list_mut(side).toggle(category);
        self.settle(app);
    }

    fn sub_on(&self, side: Side, row: usize) -> Option<usize> {
        match self.list(side).rows().get(row)? {
            Row::Sub { first, .. } => Some(*first),
            _ => None,
        }
    }

    pub fn toggle_sub(&mut self, app: &App, side: Side, row: usize) {
        if let Some(first) = self.sub_on(side, row) {
            self.list_mut(side).toggle_sub(first);
            self.settle(app);
        }
    }

    /// The group on this row, with the tag that makes it: its mods, and
    /// every other mod with that tag.
    pub fn group_on(&self, side: Side, row: usize) -> Option<(String, Vec<ModId>)> {
        let first = self.sub_on(side, row)?;
        let list = self.list(side);
        let tag = list.mods().get(first)?.sub.clone();
        let members = list.in_sub_of(first).into_iter();
        let ids = members
            .filter_map(|at| list.mods().get(at)?.installed)
            .collect();
        Some((tag, ids))
    }

    pub fn undo(&mut self, app: &App) -> bool {
        let stepped = app.session().undo().is_some();
        if stepped {
            self.sync(app);
        }
        stepped
    }

    pub fn redo(&mut self, app: &App) -> bool {
        let stepped = app.session().redo().is_some();
        if stepped {
            self.sync(app);
        }
        stepped
    }

    /// Back to the list on disk, as one step that Undo takes back.
    pub fn discard_changes(&mut self, app: &App) {
        let saved = app.session().saved().to_vec();
        app.session().record(saved);
        self.sync(app);
    }

    pub fn use_list(&mut self, app: &App, ids: Vec<String>) {
        app.session().record(ids);
        self.sync(app);
    }

    /// Writes the load order to the game's `ModsConfig.xml`.
    pub fn save(&mut self, app: &App) -> Result<(), Error> {
        let saving = self.to_save(app)?;
        modsconfig::write(&saving.file, &saving.written)?;
        self.saved(app, saving.list);
        Ok(())
    }

    /// What a save writes, and where: the sort on show kept first.
    pub fn to_save(&mut self, app: &App) -> Result<Saving, Error> {
        self.keep_sort(app);
        let file = app.paths().mods_config().ok_or_else(|| {
            Error::refused(rimstack_core::said!(
                "RimWorld's settings folder was not found."
            ))
        })?;
        let list = app.session().current().to_vec();
        // An id is empty for a folder that is not a readable mod.
        let written = list.iter().filter(|id| !id.is_empty()).cloned().collect();
        Ok(Saving {
            file,
            list,
            written,
        })
    }

    /// `list` was written: it is the saved load order now.
    pub fn saved(&mut self, app: &App, list: Vec<String>) {
        app.session().mark_saved_as(list);
        self.settle(app);
    }
}

/// A save under way: the load order as it was asked for, and as written.
pub struct Saving {
    pub file: std::path::PathBuf,
    pub list: Vec<String>,
    pub written: Vec<String>,
}
