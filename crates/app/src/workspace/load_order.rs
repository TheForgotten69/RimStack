//! The two lists of the Mods screen, the load order being edited and the
//! inactive mods, with what is wrong with each mod and a sort on show.
//! Every change ends in `settle`, which works out again what follows.

use crate::App;
use rimstack_core::base::locks::Shared;
use rimstack_core::base::sentence::Sentence;
use rimstack_core::mods::describe::Trouble;
use rimstack_core::mods::library::{Library, ModId};
use rimstack_core::mods::list::{flag, Category, GroupedList, Mod, SortKey};
use rimstack_core::mods::{demo, sort_reasons};
use rimstack_core::say;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

mod details;
mod edits;
mod rows;
mod rules;
mod sorting;

pub use details::{Details, Files, Overview};
pub use rows::RowView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Active,
    Inactive,
}

impl Side {
    pub fn of(active: bool) -> Self {
        if active {
            Side::Active
        } else {
            Side::Inactive
        }
    }

    fn other(self) -> Self {
        match self {
            Side::Active => Side::Inactive,
            Side::Inactive => Side::Active,
        }
    }
}

/// A sort on show, until it is kept or dropped.
pub struct Preview {
    before: Vec<Mod>,
    grouped_before: bool,
    /// Per mod in the previewed order: the number it had, or 0 if it stays.
    moved_from: Vec<usize>,
    /// For each new position that moved: why, and what that is filed under.
    why: Vec<Option<sort_reasons::Why>>,
}

pub struct LoadOrder {
    active: GroupedList,
    inactive: GroupedList,
    troubles: Vec<Vec<Trouble>>,
    version_ignored: Vec<bool>,
    /// The load order `troubles` was worked out for.
    checked_order: Vec<Option<ModId>>,
    twins: [Vec<bool>; 2],
    slowest: Option<u32>,
    preview: Option<Preview>,
    /// Changes to the load order count as one step for Undo meanwhile.
    batching: bool,
    /// Under category headings, not one numbered list.
    headings: bool,
    /// The inactive mods in one list by name, not by category.
    by_name: bool,
    /// Lists of the installed mods, not made-up ones.
    installed: bool,
    detected: rules::Detected,
    pub status: [Sentence; 2],
    /// Why the mods could not be read; empty when they were.
    pub problem: Sentence,
    /// What the last sort did, a sentence each: the first is the news.
    pub sort_note: Vec<Sentence>,
}

fn side_index(side: Side) -> usize {
    match side {
        Side::Active => 0,
        Side::Inactive => 1,
    }
}

/// The installed mods split by the load order being edited, each with
/// your tags, category and its load time.
fn arranged(app: &App, library: &Library) -> rimstack_core::mods::library::Arranged {
    let mut now = library.arrange(app.session().current());
    let measured = app.startup();
    for m in now.active.iter_mut() {
        let took = measured.of(&rimstack_core::rimworld::id::split_steam(&m.id).0);
        m.load_micros = took.map(|ms| (ms * 1000.0) as u32);
    }
    let notes = app.notes().reading();
    for m in now.active.iter_mut().chain(now.inactive.iter_mut()) {
        let key = m.installed.and_then(|i| library.get(i)?.key());
        if let Some(note) = key.and_then(|k| notes.get(&k)) {
            m.tags = note.tags.clone();
            m.sub = note.group().to_string();
            // Filed by hand: that stands over what RimStack worked out.
            if let Some(chosen) = Category::from_kept(&note.category) {
                m.category = chosen;
            }
        }
    }
    now
}

impl LoadOrder {
    fn empty(installed: bool) -> Self {
        Self {
            active: GroupedList::new(Vec::new()),
            inactive: GroupedList::new(Vec::new()),
            troubles: Vec::new(),
            version_ignored: Vec::new(),
            checked_order: Vec::new(),
            twins: [Vec::new(), Vec::new()],
            slowest: None,
            preview: None,
            batching: false,
            headings: true,
            by_name: false,
            installed,
            detected: rules::Detected::default(),
            status: [Sentence::default(), Sentence::default()],
            problem: Sentence::default(),
            sort_note: Vec::new(),
        }
    }

    /// Made-up mods: the window measured on a list of its own.
    pub fn demo(active: usize, inactive: usize) -> Self {
        let mut order = Self::empty(false);
        order.active = GroupedList::new(demo::generate(active));
        order.inactive = GroupedList::new(demo::generate(inactive));
        order.twins = [
            rimstack_core::mods::list::twins(order.active.mods()),
            rimstack_core::mods::list::twins(order.inactive.mods()),
        ];
        order
    }

    /// The installed mods as the app has read them; the load order is
    /// the one being edited. Reads the mods first if no one has.
    pub fn read(app: &App, headings: bool, by_name: bool) -> Self {
        let loaded = app.library();
        let (library, took) = match &*loaded {
            Ok(read) => read,
            Err(problem) => {
                let mut order = Self::empty(false);
                order.problem = problem.sentence().clone();
                return order;
            }
        };
        let mut order = Self::empty(true);
        order.headings = headings;
        order.by_name = by_name;
        let now = arranged(app, library);
        let (mods, _) = rules::sorted_by_category(app, &mut order.detected, library, now.active);
        order.active = GroupedList::flat(mods);
        order.active.set_grouped(headings);
        order.inactive = GroupedList::new(now.inactive);
        order.inactive.arrange(by_name);
        order.status = [
            match library.missing {
                0 => say!("{} active", library.active.len()),
                n => say!("{} active, {} not installed", library.active.len(), n),
            },
            say!(
                "{} inactive, read in {} ms",
                library.inactive.len(),
                took.as_millis()
            ),
        ];
        order.settle(app);
        order
    }

    pub fn is_installed(&self) -> bool {
        self.installed
    }

    pub fn list(&self, side: Side) -> &GroupedList {
        match side {
            Side::Active => &self.active,
            Side::Inactive => &self.inactive,
        }
    }

    fn list_mut(&mut self, side: Side) -> &mut GroupedList {
        match side {
            Side::Active => &mut self.active,
            Side::Inactive => &mut self.inactive,
        }
    }

    pub fn troubles(&self) -> &[Vec<Trouble>] {
        &self.troubles
    }

    /// The folded package ids of the mods the load order needs that are
    /// installed and only switched off.
    pub fn needed(&self) -> Vec<String> {
        rimstack_core::mods::describe::tally(&self.troubles).needed
    }

    pub fn previewing(&self) -> bool {
        self.preview.is_some()
    }

    pub fn headings(&self) -> bool {
        self.headings
    }

    pub fn by_name(&self) -> bool {
        self.by_name
    }

    /// Counts the window shows: mods with an error, with a warning only.
    pub fn errors_and_warnings(&self) -> (usize, usize) {
        let with = |error: bool| {
            let has = |own: &&Vec<Trouble>| own.iter().any(|t| t.error == error);
            self.troubles.iter().filter(has).count()
        };
        (with(true), with(false))
    }

    /// The active mods whose files must be read for the texture conflicts
    /// and the rules they hint at: `None` when what was read still holds.
    pub fn files_to_read(&self, app: &App) -> Option<Vec<ModId>> {
        if !self.installed {
            return None;
        }
        let mut members: Vec<ModId> = self.checked_order.iter().flatten().copied().collect();
        members.sort_unstable();
        members.dedup();
        let known = app.conflicts().reading();
        let read = known.as_ref().is_some_and(|c| c.members == members);
        (!read).then_some(members)
    }

    /// Everything that follows a change to the lists: the problems found
    /// again when the load order's mods changed, the step noted for Undo,
    /// the markers the rows show, and what the filter terms ask about.
    pub fn settle(&mut self, app: &App) {
        self.twins = [
            rimstack_core::mods::list::twins(self.active.mods()),
            rimstack_core::mods::list::twins(self.inactive.mods()),
        ];
        self.slowest = self
            .active
            .mods()
            .iter()
            .filter_map(|m| m.load_micros)
            .max();
        if !self.installed {
            return;
        }
        let loaded = app.library();
        let Ok((library, _)) = &*loaded else {
            return;
        };
        let now: Vec<Option<ModId>> = self.active.mods().iter().map(|m| m.installed).collect();
        if now != self.checked_order {
            let (found, ignored) = rules::troubles(app, library, self.active.mods());
            self.troubles = found;
            self.version_ignored = ignored;
            self.checked_order = now;
        }
        if self.preview.is_none() && !self.batching {
            let ids = self.active.mods().iter().map(|m| m.id.clone()).collect();
            app.session().record(ids);
        }
        for side in [Side::Active, Side::Inactive] {
            let flags = self.flags(app, library, side);
            self.list_mut(side).set_flags(flags);
        }
    }

    /// The problems worked out anew: something outside the lists changed
    /// what the game will load.
    pub fn recheck(&mut self, app: &App) {
        self.checked_order = Vec::new();
        self.settle(app);
    }

    /// The lists arranged anew from the load order being edited and what
    /// is noted about each mod: after Undo, a list taken whole, a note.
    pub fn sync(&mut self, app: &App) {
        let loaded = app.library();
        let Ok((library, _)) = &*loaded else {
            return;
        };
        if !self.installed {
            return;
        }
        self.preview = None;
        self.sort_note.clear();
        let now = arranged(app, library);
        let (mods, _) = rules::sorted_by_category(app, &mut self.detected, library, now.active);
        self.active.set_grouped(self.headings);
        self.active.set_mods(mods);
        self.inactive.set_mods(now.inactive);
        self.inactive.arrange(self.by_name);
        self.settle(app);
    }

    /// Each mod filed again without moving any: the category a mod shows
    /// under follows from the rules known, and more become known once the
    /// mods' files are read and the community rules arrive.
    pub fn refile(&mut self, app: &App) {
        let loaded = app.library();
        let (Ok((library, _)), None) = (&*loaded, &self.preview) else {
            return;
        };
        if !self.installed {
            return;
        }
        let mods = self.active.mods().to_vec();
        let (mods, _) = rules::sorted_by_category(app, &mut self.detected, library, mods);
        self.active.set_mods(mods);
        self.settle(app);
    }

    pub fn set_headings(&mut self, app: &App, headings: bool) {
        self.headings = headings;
        if self.active.is_grouped() != headings {
            self.active.set_grouped(headings);
            self.settle(app);
        }
    }

    pub fn set_by_name(&mut self, app: &App, by_name: bool) {
        self.by_name = by_name;
        self.inactive.arrange(by_name);
        self.settle(app);
    }

    pub fn set_filter(&mut self, text: &str) {
        self.active.set_filter(text);
        self.inactive.set_filter(text);
    }

    /// The load order shown by a column ("name", "author", "version",
    /// "load"), or by anything else in load order again.
    pub fn show_by(&mut self, column: &str, descending: bool) {
        let key = match column {
            "name" => Some(SortKey::Name),
            "author" => Some(SortKey::Author),
            "version" => Some(SortKey::Version),
            "load" => Some(SortKey::Load),
            _ => None,
        };
        self.active.show_by(key.map(|key| (key, descending)));
    }

    /// Per mod of a list, what the `is:` filter terms can ask about.
    fn flags(&self, app: &App, library: &Library, side: Side) -> Vec<u16> {
        let in_conflict = self.in_conflict(app);
        let behind = app.behind().reading();
        let mut copies: HashMap<String, usize> = HashMap::new();
        for key in library.installed.iter().filter_map(|m| m.key()) {
            *copies.entry(key).or_default() += 1;
        }
        let week_ago = super::now_secs().saturating_sub(7 * 24 * 3600);
        let newest_save = app.newest_save();
        let in_save = newest_save
            .as_ref()
            .as_ref()
            .filter(|_| side == Side::Active);
        let steam_record = app.steam_record();
        let seen = app.seen().reading();
        let session = app.session();
        let changes = session.changes();
        let changed: HashSet<&str> = match side {
            Side::Active => changes
                .moved
                .iter()
                .chain(&changes.activated)
                .copied()
                .collect(),
            Side::Inactive => HashSet::new(),
        };
        let troubles: &[Vec<Trouble>] = match side {
            Side::Active => &self.troubles,
            Side::Inactive => &[],
        };
        let mods = self.list(side).mods().iter().enumerate();
        mods.map(|(at, m)| {
            let own = troubles.get(at).map(Vec::as_slice).unwrap_or_default();
            let installed = m.installed.and_then(|i| library.get(i));
            let folder = installed.map(|i| &i.folder);
            let key = rimstack_core::rimworld::id::split_steam(&m.id).0;
            let marks = [
                (own.iter().any(|t| t.error), flag::ERROR),
                (own.iter().any(|t| !t.error), flag::WARNING),
                (
                    m.installed.is_some_and(|i| in_conflict.contains(&i)),
                    flag::CONFLICT,
                ),
                (
                    folder.is_some_and(|f| behind.as_ref().is_some_and(|b| b.contains(f))),
                    flag::UPDATED,
                ),
                (
                    in_save.is_some_and(|ids| !ids.contains(&key)),
                    flag::NOT_IN_SAVE,
                ),
                (
                    folder.is_some_and(|f| steam_record.0.get(f).is_some_and(|at| *at >= week_ago)),
                    flag::RECENT,
                ),
                (copies.get(&key).is_some_and(|n| *n > 1), flag::DUPLICATE),
                (
                    installed.is_some_and(|i| seen.is_new(&i.path.to_string_lossy())),
                    flag::NEW,
                ),
                (changed.contains(m.id.as_str()), flag::CHANGED),
            ];
            marks
                .iter()
                .filter(|(on, _)| *on)
                .fold(0, |flags, (_, mark)| flags | mark)
        })
        .collect()
    }

    pub fn in_conflict(&self, app: &App) -> HashSet<ModId> {
        let known = app.conflicts().reading();
        let Some(known) = known.as_ref() else {
            return HashSet::new();
        };
        (0..known.members.len())
            .filter(|&at| !known.overlaps.with(at).is_empty())
            .map(|at| known.members[at])
            .collect()
    }

    /// The installed mods of a list that are new since the game was last
    /// started from RimStack.
    pub fn fresh(&self, app: &App, side: Side) -> Vec<ModId> {
        let loaded = app.library();
        let Ok((library, _)) = &*loaded else {
            return Vec::new();
        };
        let seen = app.seen().reading();
        let mods = self.list(side).mods().iter();
        mods.filter_map(|m| m.installed)
            .filter(|&i| {
                library
                    .get(i)
                    .is_some_and(|m| seen.is_new(&m.path.to_string_lossy()))
            })
            .collect()
    }
}

impl App {
    /// Reads what the files of these active mods provide and point at,
    /// each from the folders the game would load it from, and keeps it.
    /// Slow: off the window's thread.
    pub fn read_active_files(&self, members: Vec<ModId>) {
        use rayon::prelude::*;
        use rimstack_core::mods::{conflicts, infer};
        let loaded = self.library();
        let Ok((library, _)) = &*loaded else {
            return;
        };
        let active: HashSet<String> = members
            .iter()
            .filter_map(|&i| library.get(i)?.key())
            .collect();
        let read: Vec<(Vec<String>, infer::Scanned)> = members
            .par_iter()
            .map(|&i| {
                let Some(root) = library.get(i).map(|m| &m.path) else {
                    return Default::default();
                };
                let loaded = rimstack_core::rimworld::loadfolders::folders(
                    root,
                    &library.game_version,
                    &|m| active.contains(m),
                );
                (conflicts::provided(&loaded), infer::scan(&loaded))
            })
            .collect();
        let (provided, scanned): (Vec<_>, Vec<_>) = read.into_iter().unzip();
        *self.conflicts().writing() = Some(super::Conflicts {
            overlaps: conflicts::overlaps(&provided),
            scanned: members.iter().copied().zip(scanned).collect(),
            members,
        });
        self.publish(crate::changes::Changes::FILES_READ);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundMod {
    pub name: String,
    pub mod_id: i32,
    pub active: bool,
}

impl LoadOrder {
    /// Installed mods whose name holds `text`, closest first, at most `most`.
    pub fn found(&self, app: &App, text: &str, most: usize) -> Vec<FoundMod> {
        let loaded = app.library();
        let Ok((library, _)) = &*loaded else {
            return Vec::new();
        };
        let active = self.list(Side::Active);
        library
            .find(text, most)
            .into_iter()
            .filter_map(|id| {
                Some(FoundMod {
                    name: library.get(id)?.display_name().to_string(),
                    mod_id: id.to_qml(),
                    active: active.position_of(id).is_some(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
