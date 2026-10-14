//! Where a mod is in a list and where a step or a move takes it, for
//! the keyboard and the selection; the arithmetic is the list's own.

use super::*;

/// A row, a place or a mod as the window holds it; -1 for none.
fn held(found: Option<usize>) -> i32 {
    found.map_or(-1, |at| at as i32)
}

impl qobject::ModListModel {
    pub(super) fn index_of(&self, mod_id: i32) -> i32 {
        held(ModId::from_qml(mod_id).and_then(|id| self.list.position_of(id)))
    }

    /// Where the mod is in this list.
    pub(super) fn position_of(&self, id: ModId) -> Option<usize> {
        self.list.position_of(id)
    }

    pub(super) fn row_of(&self, position: i32) -> i32 {
        held(
            usize::try_from(position)
                .ok()
                .and_then(|at| self.list.row_of(at)),
        )
    }

    pub(super) fn mod_id_at(&self, row: i32) -> i32 {
        let on_row = usize::try_from(row)
            .ok()
            .and_then(|row| self.list.installed_on(row));
        on_row.map_or(-1, ModId::to_qml)
    }

    pub(super) fn step_from(&self, mod_id: i32, by: i32) -> i32 {
        held(
            self.list
                .step_from(ModId::from_qml(mod_id), by.signum() as isize),
        )
    }

    pub(super) fn beside_mod(&self, mod_id: i32) -> i32 {
        let beside = ModId::from_qml(mod_id).and_then(|id| self.list.beside_mod(id));
        beside.map_or(-1, ModId::to_qml)
    }

    pub(super) fn others_through(&self, from: i32, to: i32) -> QString {
        let (Some(from), Some(to)) = (ModId::from_qml(from), ModId::from_qml(to)) else {
            return crate::reports::said(&None::<Vec<ModId>>);
        };
        crate::reports::said(&self.list.others_through(from, to))
    }

    pub(super) fn in_load_order(&self, a: i32, b: i32) -> QString {
        let ordered = match (ModId::from_qml(a), ModId::from_qml(b)) {
            (Some(a), Some(b)) => self.list.in_load_order(a, b),
            _ => None,
        };
        crate::reports::said(&ordered.map(|(earlier, later)| [earlier, later]))
    }

    pub(super) fn place_below(&self, mod_id: i32, other: i32) -> i32 {
        let below = match (ModId::from_qml(mod_id), ModId::from_qml(other)) {
            (Some(id), Some(other)) => self.list.below(id, other),
            _ => None,
        };
        held(below.map(|(_, to)| to))
    }

    pub(super) fn nudge_to(&self, mod_id: i32, by: i32) -> i32 {
        let nudged = ModId::from_qml(mod_id).and_then(|id| self.list.nudged(id, by as isize));
        held(nudged.map(|(_, to)| to))
    }

    pub(super) fn version_warned(&self, ids: &QString) -> QString {
        let warned = |id: &ModId| {
            let at = self.list.position_of(*id);
            let own = at.and_then(|at| self.troubles.get(at));
            own.is_some_and(|own| describe::warned_for_version(own))
        };
        let ids = crate::backend::ids_from(ids);
        crate::reports::said(&ids.into_iter().filter(warned).collect::<Vec<_>>())
    }

    pub(super) fn package_mods(&self, keys: &QString) -> QString {
        let keys: Vec<String> = serde_json::from_str(&keys.to_string()).unwrap_or_default();
        let loaded = crate::app().library();
        let Ok((library, _)) = &*loaded else {
            return crate::reports::said(&Vec::<ModId>::new());
        };
        let key_of = |id: ModId| library.get(id)?.key();
        crate::reports::said(&self.list.with_keys(&keys, &key_of))
    }
}
