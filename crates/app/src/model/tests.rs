//! What the model gathers from an `App`, on a made-up install.

use super::*;
use crate::state::tests::installed;
use rimstack_core::mods::list::Category;

fn library_of(app: &crate::App) -> std::sync::Arc<crate::Loaded> {
    let loaded = app.library();
    assert!(loaded.is_ok());
    loaded
}

#[test]
fn the_two_lists_are_the_load_order_and_the_rest() {
    let (_tmp, app) = installed();
    let loaded = library_of(&app);
    let (active, status, problem) = listed(&app, &loaded, true, false);
    assert_eq!((active.mods().len(), problem.as_str()), (1, ""));
    assert_eq!(active.mods()[0].id, "ludeon.rimworld");
    assert!(status.contains('1'));
    let (inactive, _, _) = listed(&app, &loaded, false, false);
    let ids: Vec<&str> = inactive.mods().iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["ann.roads"]);
}

#[test]
fn mods_that_could_not_be_read_give_an_empty_list_and_the_reason() {
    let (tmp, app) = installed();
    std::fs::remove_dir_all(tmp.path().join("game")).unwrap();
    let (list, status, problem) = listed(&app, &app.library(), true, true);
    assert!(list.mods().is_empty() && status.is_empty());
    assert!(!problem.is_empty());
}

#[test]
fn a_note_is_kept_on_disk_and_shows_on_the_mod_s_row() {
    let (_tmp, app) = installed();
    let loaded = library_of(&app);
    let (library, _) = loaded.as_ref().as_ref().unwrap();
    let roads = library
        .ids()
        .find(|(_, m)| m.folder == "roads")
        .map(|(id, _)| id);
    crate::mod_edits::change_note(&app, roads.unwrap(), |note| {
        note.tags = vec!["travel".to_string()];
        note.category = Category::Tweaks.label().to_string();
    });
    // An id no mod has, or one from an earlier reading, changes nothing.
    let unknown = rimstack_core::mods::library::ModId::new(0, 99);
    crate::mod_edits::change_note(&app, unknown, |note| note.text = "lost".to_string());
    app.rescan();
    crate::mod_edits::change_note(&app, roads.unwrap(), |note| note.text = "lost".to_string());

    let kept = rimstack_core::kept::notes::Notes::read(&app.notes_file());
    assert_eq!(kept.len(), 1);
    assert_eq!(kept.get("ann.roads").unwrap().text, "");
    let row = arranged(&app, library).inactive.remove(0);
    assert_eq!(
        (row.tags, row.sub.as_str()),
        (vec!["travel".to_string()], "travel")
    );
    assert_eq!(row.category, Category::Tweaks);
}

#[test]
fn a_mod_s_folders_are_asked_by_its_id_and_an_id_no_mod_has_gives_none() {
    let (tmp, app) = installed();
    let loaded = library_of(&app);
    let (library, _) = loaded.as_ref().as_ref().unwrap();
    let roads = library
        .ids()
        .find(|(_, m)| m.folder == "roads")
        .map(|(id, _)| id);
    assert_eq!(
        crate::backend::loaded_folders(&app, library, roads.unwrap()),
        [tmp.path().join("game/Mods/roads")]
    );
    let unknown = rimstack_core::mods::library::ModId::new(0, 99);
    assert!(crate::backend::loaded_folders(&app, library, unknown).is_empty());
}

#[test]
fn every_listed_mod_has_its_troubles_worked_out() {
    let (_tmp, app) = installed();
    let loaded = library_of(&app);
    let (library, _) = loaded.as_ref().as_ref().unwrap();
    let mods = arranged(&app, library).active;
    let (found, ignored) = troubles(&app, library, &mods);
    assert_eq!((found.len(), ignored.len()), (mods.len(), mods.len()));
    assert!(all_rules(&app, library, &mods).is_empty());
}
