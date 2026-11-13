//! What the other sections read of the load order: filled from the Mods tab each time it
//! changes, never while drawn, and told only when it changed. A section reads it while drawn, or
//! observes it, and needs no handle on the Mods tab.

use super::mods::Mods;
use crate::workspace::load_order::Overview;
use gpui::{App as Cx, AppContext, Entity, Global};
use rimstack_core::mods::library::ModId;
use std::rc::Rc;

#[derive(Default, Clone, PartialEq)]
pub struct Order {
    pub overview: Option<Rc<Overview>>,
    /// The mod chosen first.
    pub selected: Option<ModId>,
    /// The active mods installed, in their order.
    pub active: Rc<Vec<ModId>>,
}

struct Held(Entity<Order>);

impl Global for Held {}

pub fn install(cx: &mut Cx) {
    let order = cx.new(|_| Order::default());
    cx.set_global(Held(order));
}

pub fn order(cx: &Cx) -> Entity<Order> {
    cx.global::<Held>().0.clone()
}

pub fn read(cx: &Cx) -> Order {
    order(cx).read(cx).clone()
}

/// What the Mods tab shows the other sections, read again after it changed.
pub fn from_mods(mods: &Entity<Mods>, cx: &mut Cx) {
    let mods = mods.read(cx);
    let fresh = Order {
        overview: mods.overview(),
        selected: mods.selected_id(),
        active: Rc::new(mods.active_installed()),
    };
    order(cx).update(cx, |order, cx| {
        if *order != fresh {
            *order = fresh;
            cx.notify();
        }
    });
}
