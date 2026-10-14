//! The two lists on a made-up install: Core active, one local mod beside it.

use super::*;
use crate::workspace::tests::installed;

/// The local mod made for an older game version than the one installed.
fn roads_made_for_an_older_game(tmp: &tempfile::TempDir) {
    let about = "<ModMetaData><packageId>Ann.Roads</packageId><name>Ann.Roads</name><supportedVersions><li>1.5</li></supportedVersions></ModMetaData>";
    std::fs::write(tmp.path().join("game/Mods/roads/About/About.xml"), about).unwrap();
}

fn roads(order: &LoadOrder, side: Side) -> ModId {
    let list = order.list(side);
    let m = list.mods().iter().find(|m| m.id == "ann.roads").unwrap();
    m.installed.unwrap()
}

#[test]
fn the_two_lists_are_the_load_order_and_the_rest() {
    let (_tmp, app) = installed();
    let order = LoadOrder::read(&app, true, false);
    assert!(order.problem.is_empty() && order.is_installed());
    let ids = |side| -> Vec<String> {
        order
            .list(side)
            .mods()
            .iter()
            .map(|m| m.id.clone())
            .collect()
    };
    assert_eq!(ids(Side::Active), ["ludeon.rimworld"]);
    assert_eq!(ids(Side::Inactive), ["ann.roads"]);
    assert!(order.status[0].contains('1'));
}

#[test]
fn mods_that_could_not_be_read_give_empty_lists_and_the_reason() {
    let (tmp, app) = installed();
    std::fs::remove_dir_all(tmp.path().join("game")).unwrap();
    let order = LoadOrder::read(&app, true, false);
    assert!(!order.problem.is_empty() && !order.is_installed());
    assert!(order.list(Side::Active).mods().is_empty());
}

#[test]
fn a_mod_taken_across_is_one_step_that_undo_takes_back() {
    let (_tmp, app) = installed();
    let mut order = LoadOrder::read(&app, true, false);
    let id = roads(&order, Side::Inactive);
    order.take_all(&app, Side::Inactive, &[id]);
    assert_eq!(app.session().current(), ["ludeon.rimworld", "ann.roads"]);
    assert!(order.list(Side::Inactive).mods().is_empty());
    assert!(app.session().is_dirty());
    assert!(order.undo(&app));
    assert_eq!(app.session().current(), ["ludeon.rimworld"]);
    assert_eq!(order.list(Side::Inactive).mods().len(), 1);
    assert!(order.redo(&app));
    assert_eq!(order.list(Side::Active).mods().len(), 2);
}

#[test]
fn a_mod_made_for_another_game_version_offers_to_trust_it_once_active() {
    let (tmp, app) = installed();
    roads_made_for_an_older_game(&tmp);
    let mut order = LoadOrder::read(&app, true, false);
    let id = roads(&order, Side::Inactive);
    let shown = |order: &mut LoadOrder| serde_json::to_value(order.details(&app, id)).unwrap();
    assert_eq!(shown(&mut order)["offerTrust"], false);
    order.take(&app, Side::Inactive, 0, None);
    let details = shown(&mut order);
    assert_eq!(details["offerTrust"], true, "{}", details["problems"]);
    assert_eq!(details["name"], "Ann.Roads");
}

#[test]
fn saving_writes_the_load_order_and_marks_it_saved() {
    let (tmp, app) = installed();
    let mut order = LoadOrder::read(&app, true, false);
    order.take(&app, Side::Inactive, 0, None);
    order.save(&app).unwrap();
    let written = std::fs::read_to_string(tmp.path().join("config/Config/ModsConfig.xml")).unwrap();
    assert!(written.contains("<li>ann.roads</li>"));
    assert!(!app.session().is_dirty());
}

#[test]
fn every_row_has_a_key_of_its_own_and_a_mod_row_its_id() {
    let (tmp, app) = installed();
    roads_made_for_an_older_game(&tmp);
    let mut order = LoadOrder::read(&app, true, false);
    order.take(&app, Side::Inactive, 0, None);
    let rows = order.rows(&app, Side::Active);
    let keys: std::collections::HashSet<_> = rows.iter().map(|r| r.key.clone()).collect();
    assert_eq!(keys.len(), rows.len());
    let roads = rows.iter().find(|r| r.name == "Ann.Roads").unwrap();
    assert!(roads.mod_id >= 0 && roads.number == 2 && roads.trouble > 0);
}

#[test]
fn a_sort_of_a_list_in_order_moves_nothing_and_says_so() {
    let (_tmp, app) = installed();
    let mut order = LoadOrder::read(&app, true, false);
    order.preview_sort(&app);
    assert!(!order.previewing());
    assert!(!order.sort_note.is_empty());
    assert_eq!(order.sort_pending(&app), 0);
}

#[test]
fn made_up_lists_have_rows_and_no_problems() {
    let (_tmp, app) = installed();
    let order = LoadOrder::demo(40, 10);
    assert_eq!(order.list(Side::Active).mods().len(), 40);
    assert!(order.rows(&app, Side::Active).len() >= 40);
    assert_eq!(order.errors_and_warnings(), (0, 0));
}
