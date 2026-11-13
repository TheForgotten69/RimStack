//! Where a view asks the window to go: another section, a mod in the load order, a list used.
//! Told to one router the shell hears, so a view needs no handle on the others and is never
//! updated by them while it updates.

use super::section::Section;
use gpui::{App as Cx, AppContext, Entity, EventEmitter, Global};
use rimstack_core::mods::library::ModId;

#[derive(Clone, Debug, PartialEq)]
pub enum Go {
    Section(Section),
    /// The mod chosen in the load order, on the Mods tab.
    ShowMod(ModId),
    /// The load order becomes these mods, unsaved.
    UseList(Vec<String>),
    /// The Mods tab filtered to what Search found.
    SearchedInMods,
    /// The search for the mod that breaks the game started.
    FindBroken,
    /// The Workshop searched for this, as a fix that names a page asks.
    Browse(String),
    Subscribe(String),
    Redownload(ModId),
    Unsubscribe(ModId),
    UpdateFromGit(ModId),
}

pub struct Router;

impl EventEmitter<Go> for Router {}

struct Held(Entity<Router>);

impl Global for Held {}

pub fn install(cx: &mut Cx) {
    let router = cx.new(|_| Router);
    cx.set_global(Held(router));
}

pub fn router(cx: &Cx) -> Entity<Router> {
    cx.global::<Held>().0.clone()
}

/// Heard once the update asking it is done, as an event is.
pub fn go(to: Go, cx: &mut Cx) {
    router(cx).update(cx, |_, cx| cx.emit(to));
}
