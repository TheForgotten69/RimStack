//! What the window around the sections shows of them: what stands in Play's way, the list's name
//! and whether it is saved, and the work under way. Filled from the sections each time one of
//! them changes, never while drawn; the bars read it while drawn, so they are drawn again only
//! when what they show changed.

use super::mods::Mods;
use gpui::{App as Cx, AppContext, Context, Entity, Global};

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Around {
    /// What stands in Play's way, worded, in the order the popup lists it.
    pub in_the_way: Vec<String>,
    /// Whether a mod has a problem that causes errors in the game.
    pub errors: bool,
    pub sort_pending: bool,
    /// The list has changes not saved, and these in words.
    pub dirty: bool,
    pub unsaved: Option<String>,
    /// The kept list the load order came from, empty when none: read at the start, then told
    /// by `Lists` as it keeps it.
    pub list_name: String,
    /// The Mods tab's work under way, worded.
    pub mods_work: Vec<String>,
    pub downloads_work: Vec<String>,
    pub community_busy: bool,
    pub textures_work: Option<String>,
}

struct Held(Entity<Around>);

impl Global for Held {}

pub fn install(cx: &mut Cx) {
    let list_name = super::app(cx)
        .prefs()
        .get("Mods", "listName")
        .unwrap_or_default();
    let around = cx.new(|_| Around {
        list_name,
        ..Around::default()
    });
    cx.set_global(Held(around));
}

pub fn around(cx: &Cx) -> Entity<Around> {
    cx.global::<Held>().0.clone()
}

pub fn read(cx: &Cx) -> Around {
    around(cx).read(cx).clone()
}

/// Told only when it changes, so nothing is drawn again for the same.
pub fn change<V>(cx: &mut Context<V>, change: impl FnOnce(&mut Around)) {
    around(cx).update(cx, |around, cx| {
        let was = around.clone();
        change(around);
        if *around != was {
            cx.notify();
        }
    });
}

/// What the Mods tab shows the window, read again after it changed.
pub fn from_mods<V>(mods: &Entity<Mods>, cx: &mut Context<V>) {
    let mods = mods.read(cx);
    let (errors, sort_pending, dirty, unsaved) = mods.standing();
    let in_the_way = mods.in_the_way_said(errors, sort_pending, dirty);
    let work = mods.work_said();
    change(cx, |around| {
        around.in_the_way = in_the_way;
        around.errors = errors;
        around.sort_pending = sort_pending;
        around.dirty = dirty;
        around.unsaved = unsaved;
        around.mods_work = work;
    });
}
