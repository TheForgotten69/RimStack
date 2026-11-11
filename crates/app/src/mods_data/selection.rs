use super::{id, qobject};
use crate::qml::values::to_qml;
use crate::workspace::load_order::Side;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QVariant;
use rimstack_core::mods::library::ModId;

/// The first mod chosen, then the others by id.
#[derive(Default, PartialEq)]
pub(super) struct Selection {
    ids: Vec<ModId>,
    active: bool,
}

impl Selection {
    pub(super) fn of(ids: Vec<ModId>, active: bool) -> Selection {
        let mut ids = ids.into_iter();
        let first = ids.next();
        let mut others: Vec<ModId> = ids.filter(|id| Some(*id) != first).collect();
        others.sort_by_key(|id| id.to_qml());
        others.dedup();
        Selection {
            ids: first.into_iter().chain(others).collect(),
            active,
        }
    }

    pub(super) fn ids(&self) -> &[ModId] {
        &self.ids
    }

    fn first(&self) -> Option<ModId> {
        self.ids.first().copied()
    }

    fn alongside(&self, in_active: bool) -> Option<ModId> {
        self.first().filter(|_| self.active == in_active)
    }
}

impl qobject::ModsData {
    pub(super) fn select(self: Pin<&mut Self>, mod_id: i32, in_active: bool) {
        self.keep(Selection::of(id(mod_id).into_iter().collect(), in_active));
    }

    pub(super) fn select_also(self: Pin<&mut Self>, mod_id: i32, in_active: bool) {
        let Some(first) = self.chosen.alongside(in_active) else {
            return self.select(mod_id, in_active);
        };
        let Some(toggled) = id(mod_id).filter(|toggled| *toggled != first) else {
            return;
        };
        let mut ids = self.chosen.ids().to_vec();
        match ids.iter().position(|held| *held == toggled) {
            Some(at) => drop(ids.remove(at)),
            None => ids.push(toggled),
        }
        self.keep(Selection::of(ids, in_active));
    }

    pub(super) fn select_through(self: Pin<&mut Self>, mod_id: i32, in_active: bool) {
        let Some(first) = self.chosen.alongside(in_active) else {
            return self.select(mod_id, in_active);
        };
        let list = self.order.list(Side::of(in_active));
        let Some(others) = id(mod_id).and_then(|to| list.others_through(first, to)) else {
            return;
        };
        let ids = std::iter::once(first).chain(others).collect();
        self.keep(Selection::of(ids, in_active));
    }

    pub(super) fn invert_selection(self: Pin<&mut Self>, in_active: bool) {
        let (side, chosen_on) = (Side::of(in_active), Side::of(self.chosen.active));
        let inverted = self.order.inverted(side, chosen_on, self.chosen.ids());
        self.keep(Selection::of(inverted.ids, in_active));
    }

    pub(super) fn follow_selection(self: Pin<&mut Self>) {
        let Some(first) = self.chosen.first() else {
            return;
        };
        let on = |side| self.order.list(side).position_of(first).is_some();
        let active = match (on(Side::Active), on(Side::Inactive)) {
            (true, _) => true,
            (_, true) => false,
            _ => return,
        };
        if active != self.chosen.active {
            let ids = self.chosen.ids().to_vec();
            self.keep(Selection::of(ids, active));
        }
    }

    pub(super) fn keep(mut self: Pin<&mut Self>, next: Selection) {
        if next == self.chosen {
            return;
        }
        let held: Vec<i32> = next.ids().iter().map(|id| id.to_qml()).collect();
        self.as_mut().set_selected_active(next.active);
        self.as_mut().set_selected_count(held.len() as i32);
        self.as_mut().set_selected_ids(to_qml(&held));
        self.as_mut().rust_mut().get_mut().chosen = next;
        for side in [Side::Active, Side::Inactive] {
            if let Some(model) = self.as_mut().rows(side) {
                model.choose(&held);
            }
        }
        self.as_mut().summarize_selection();
        self.set_selected(held.first().copied().unwrap_or(-1));
    }

    pub(super) fn summarize_selection(self: Pin<&mut Self>) {
        let many = self.chosen.ids().len() > 1;
        if !many && self.selection_summary.is_null() {
            return;
        }
        let said = if many {
            let (active, ids) = (self.chosen.active, self.chosen.ids());
            to_qml(&self.order.selection_summary(active, ids))
        } else {
            QVariant::default()
        };
        self.set_selection_summary(said);
    }
}
