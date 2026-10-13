//! What the list model holds besides its rows.

use super::*;

pub(super) struct Preview {
    pub(super) before: Vec<Mod>,
    pub(super) grouped_before: bool,
    /// Per mod in the previewed order: the number it had, or 0 if it stays.
    pub(super) moved_from: Vec<usize>,
    /// For each new position that moved: what its reason is filed under,
    /// and the reason.
    pub(super) why: Vec<Option<rimstack_core::why::Why>>,
}

pub struct ModListModelRust {
    pub(super) demo_count: i32,
    pub(super) status: QString,
    pub(super) problem: QString,
    pub(super) count: i32,
    pub(super) matching: i32,
    pub(super) previewing: bool,
    pub(super) sort_note: QString,
    pub(super) dirty: bool,
    /// What changed since the list was saved, in words; empty for nothing.
    pub(super) unsaved: QString,
    pub(super) can_undo: bool,
    pub(super) can_redo: bool,
    pub(super) errors: i32,
    pub(super) conflicts: i32,
    pub(super) warnings: i32,
    /// Goes up each time the texture conflicts have been worked out again.
    pub(super) conflicts_version: i32,
    /// Whether the files of the mods in this list have been read: Sort
    /// follows what they say, so it waits for this.
    pub(super) files_read: bool,
    /// For the inactive mods: in one list by name, not by category under
    /// headings. Call `reshape` after changing it.
    pub(super) by_name: bool,
    /// For the load order: 1 always under category headings, 0 always
    /// one numbered list, -1 headings when the order is close to
    /// category order. Call `reshape` after changing it.
    pub(super) headings: i32,
    /// Goes up each time a rule of your own is added or removed.
    pub(super) rules_version: i32,
    /// Goes up each time the mod folders have been read again.
    pub(super) library_version: i32,
    pub(super) rescanning: bool,
    pub(super) rules_busy: bool,
    /// The community rules were downloaded while this window was open,
    /// so they are known to be the latest.
    pub(super) rules_current: bool,
    pub(super) rules_problem: QString,
    /// Whether this is the load order of the installed mods: the list
    /// that is checked for problems.
    pub(super) checked: bool,
    pub(super) installed: bool,
    /// Inside `beginBatch`…`endBatch`: steps are not noted one by one.
    pub(super) batching: bool,
    pub(super) troubles: Vec<Vec<Trouble>>,
    pub(super) version_ignored: Vec<bool>,
    /// The load order `troubles` was worked out for.
    pub(super) checked_order: Vec<Option<usize>>,
    pub(super) list: GroupedList,
    pub(super) preview: Option<Preview>,
}

impl Default for ModListModelRust {
    fn default() -> Self {
        Self {
            demo_count: 0,
            status: QString::default(),
            problem: QString::default(),
            count: 0,
            matching: 0,
            previewing: false,
            sort_note: QString::default(),
            dirty: false,
            unsaved: QString::default(),
            can_undo: false,
            can_redo: false,
            errors: 0,
            conflicts: 0,
            warnings: 0,
            conflicts_version: 0,
            files_read: false,
            by_name: false,
            headings: -1,
            rules_version: 0,
            library_version: 0,
            rescanning: false,
            rules_busy: false,
            rules_current: false,
            rules_problem: QString::default(),
            checked: false,
            installed: false,
            batching: false,
            troubles: Vec::new(),
            version_ignored: Vec::new(),
            checked_order: Vec::new(),
            list: GroupedList::new(Vec::new()),
            preview: None,
        }
    }
}
