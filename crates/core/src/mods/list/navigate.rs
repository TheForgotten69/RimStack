//! Finding one's way about a list: which mod is on a row, which row
//! shows a mod, and where a step or a move from there leads.

use super::{GroupedList, Row};
use crate::mods::library::ModId;

impl GroupedList {
    /// The mod on a row; nothing for a heading or a mod that is not
    /// installed.
    pub fn installed_on(&self, row: usize) -> Option<ModId> {
        match self.rows.get(row)? {
            Row::Mod { index } => self.mods[*index].installed,
            _ => None,
        }
    }

    /// The nearest row holding an installed mod from `from`, going down
    /// (`by` 1) or up (-1). `from` may be one above the first row or one
    /// below the last, to start at an end.
    pub fn step(&self, from: isize, by: isize) -> Option<usize> {
        let mut row = from.checked_add(by)?;
        while let Some(at) = usize::try_from(row).ok().filter(|at| *at < self.rows.len()) {
            if self.installed_on(at).is_some() {
                return Some(at);
            }
            row = row.checked_add(by)?;
        }
        None
    }

    /// The installed mods on the rows from one to the other, whichever
    /// is the lower, both included.
    pub fn installed_between(&self, a: usize, b: usize) -> Vec<ModId> {
        let rows = a.min(b)..=a.max(b);
        rows.filter_map(|row| self.installed_on(row)).collect()
    }

    /// The mod the selection goes to when the one on `row` leaves the
    /// list: the row below, the one above, then two away either side.
    pub fn beside(&self, row: usize) -> Option<ModId> {
        let near = [
            row.checked_add(1),
            row.checked_sub(1),
            row.checked_add(2),
            row.checked_sub(2),
        ];
        near.into_iter()
            .flatten()
            .find_map(|at| self.installed_on(at))
    }

    /// Where the mod is in the list, by place among its mods.
    pub fn position_of(&self, id: ModId) -> Option<usize> {
        self.mods.iter().position(|m| m.installed == Some(id))
    }

    /// The row showing the mod at this place; nothing when it is
    /// filtered out or under a folded heading.
    pub fn row_of(&self, position: usize) -> Option<usize> {
        let shows = |row: &Row| matches!(row, Row::Mod { index } if *index == position);
        self.rows.iter().position(shows)
    }

    fn row_of_mod(&self, id: ModId) -> Option<usize> {
        self.row_of(self.position_of(id)?)
    }

    /// The row a step of `by` from the mod's row leads to, past headings;
    /// from no mod, or one not shown, a step starts at an end.
    pub fn step_from(&self, id: Option<ModId>, by: isize) -> Option<usize> {
        let at_end = if by > 0 { -1 } else { self.rows.len() as isize };
        let from = id.and_then(|id| self.row_of_mod(id));
        self.step(from.map_or(at_end, |row| row as isize), by)
    }

    /// The mod the selection goes to when this one leaves the list.
    pub fn beside_mod(&self, id: ModId) -> Option<ModId> {
        self.beside(self.row_of_mod(id)?)
    }

    /// The other mods on the rows from one mod to another, both shown.
    pub fn others_through(&self, from: ModId, to: ModId) -> Option<Vec<ModId>> {
        let (a, b) = (self.row_of_mod(from)?, self.row_of_mod(to)?);
        let between = self.installed_between(a, b).into_iter();
        Some(between.filter(|id| *id != from).collect())
    }

    /// The two mods, the one that loads first first; nothing unless both
    /// are in the list.
    pub fn in_load_order(&self, a: ModId, b: ModId) -> Option<(ModId, ModId)> {
        match self.position_of(a)? < self.position_of(b)? {
            true => Some((a, b)),
            false => Some((b, a)),
        }
    }

    /// The mods of the list each key names, in the order of the keys; a
    /// key no mod of the list has is left out. `key_of` gives a mod's.
    pub fn with_keys(
        &self,
        keys: &[String],
        key_of: &dyn Fn(ModId) -> Option<String>,
    ) -> Vec<ModId> {
        let keyed: Vec<(ModId, String)> = self
            .mods
            .iter()
            .filter_map(|m| m.installed)
            .filter_map(|id| Some((id, key_of(id)?)))
            .collect();
        let named = |key: &String| keyed.iter().find(|(_, k)| k == key).map(|(id, _)| *id);
        keys.iter().filter_map(named).collect()
    }

    /// Where the mod is and the place of `other`, when `other` loads after
    /// it: a move there puts the mod just after `other`.
    pub fn below(&self, id: ModId, other: ModId) -> Option<(usize, usize)> {
        let (at, theirs) = (self.position_of(id)?, self.position_of(other)?);
        (theirs > at).then_some((at, theirs))
    }

    /// Where the mod is and where a move of `by` places takes it; nothing
    /// when that would leave the list.
    pub fn nudged(&self, id: ModId, by: isize) -> Option<(usize, usize)> {
        let at = self.position_of(id)?;
        let to = at
            .checked_add_signed(by)
            .filter(|to| *to < self.mods.len())?;
        Some((at, to))
    }
}
