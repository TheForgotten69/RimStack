use super::*;
use crate::workspace::tests::installed;

/// Core with a xenotype, `c` replacing the gene `i` adds: `c` fails when it loads first.
fn reported_pair() -> (tempfile::TempDir, App, Vec<Mod>) {
    let (tmp, app) = installed();
    let root = tmp.path();
    let write = |file: &str, xml: &str| {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, xml).unwrap();
    };
    write(
        "game/Data/Core/Defs/x.xml",
        "<Defs><XenotypeDef><defName>X</defName><genes><li>Base</li></genes></XenotypeDef></Defs>",
    );
    let about = |id: &str| {
        format!("<ModMetaData><packageId>t.{id}</packageId><name>{id}</name></ModMetaData>")
    };
    let genes = "Defs/XenotypeDef[defName=\"X\"]/genes";
    write("game/Mods/c/About/About.xml", &about("c"));
    write("game/Mods/i/About/About.xml", &about("i"));
    write(
        "game/Mods/c/Patches/p.xml",
        &format!(
            "<Patch><Operation Class=\"PatchOperationReplace\"><xpath>{genes}/li[text()=\"Added\"]</xpath>\
             <value><li>Replaced</li></value></Operation></Patch>"
        ),
    );
    write(
        "game/Mods/i/Patches/p.xml",
        &format!(
            "<Patch><Operation Class=\"PatchOperationAdd\"><xpath>{genes}</xpath>\
             <value><li>Added</li></value></Operation></Patch>"
        ),
    );
    write(
        "config/Config/ModsConfig.xml",
        "<ModsConfigData><activeMods><li>ludeon.rimworld</li><li>t.c</li><li>t.i</li></activeMods></ModsConfigData>",
    );
    let order = LoadOrder::read(&app, false, None);
    let mods = order.list(Side::Active).mods().to_vec();
    (tmp, app, mods)
}

fn place_of(mods: &[Mod], id: &str) -> usize {
    mods.iter().position(|m| m.id == id).unwrap()
}

fn sorted(app: &App, mods: &[Mod]) -> Vec<Mod> {
    let loaded = app.library();
    let Ok((library, _)) = &*loaded else { panic!() };
    let (mods, sorted) =
        rules::sorted_by_category(app, &mut rules::Detected::default(), library, mods.to_vec());
    sorted.order.iter().map(|&at| mods[at].clone()).collect()
}

fn a_turn() -> crate::jobs::Turn {
    crate::jobs::Lane::default().next()
}

#[test]
fn simulated_hints_reach_sort() {
    let (_tmp, app, mods) = reported_pair();
    let before = sorted(&app, &mods);
    assert!(place_of(&before, "t.c") < place_of(&before, "t.i"));

    let found = app.simulate_patches(mods.clone(), &a_turn()).unwrap();
    assert_eq!(found.hints.len(), 1);
    assert!(found.unfixed.is_empty());
    *app.patched().writing() = Some(found);

    let after = sorted(&app, &mods);
    assert!(place_of(&after, "t.i") < place_of(&after, "t.c"));
}

#[test]
fn a_result_for_another_list_is_not_used() {
    let (_tmp, app, mods) = reported_pair();
    let found = app.simulate_patches(mods.clone(), &a_turn()).unwrap();
    *app.patched().writing() = Some(found);
    let loaded = app.library();
    let Ok((library, _)) = &*loaded else { panic!() };
    let patches = |mods: &[Mod]| {
        let found = rules::all_rules(&app, &mut rules::Detected::default(), library, mods);
        found
            .iter()
            .filter(|r| r.kind == rimstack_core::mods::rules::Kind::Patches)
            .count()
    };
    assert_eq!(patches(&mods), 1);
    let without_c: Vec<Mod> = mods.iter().filter(|m| m.id != "t.c").cloned().collect();
    assert_eq!(patches(&without_c), 0);
}

#[test]
fn no_game_defs_no_simulation() {
    let (_tmp, app, mods) = reported_pair();
    std::fs::remove_dir_all(_tmp.path().join("game/Data/Core/Defs")).unwrap();
    app.rescan();
    assert!(app.simulate_patches(mods, &a_turn()).is_none());
}

#[test]
fn a_result_made_before_a_rescan_is_not_kept() {
    let (_tmp, app, mods) = reported_pair();
    let generation = app.conflicts_generation();
    let found = app.simulate_patches(mods.clone(), &a_turn());
    let running = app.simulation_started(&mods);
    app.textures_changed();
    app.keep_patched(generation, found, running);
    assert!(app.patched().reading().is_none());
}

#[test]
fn a_running_simulation_is_told_until_its_result_is_kept() {
    let (_tmp, app, mods) = reported_pair();
    assert!(!app.simulation_running());
    let generation = app.conflicts_generation();
    let guard = app.simulation_started(&mods);
    assert!(app.simulation_running());
    let found = app.simulate_patches(mods, &a_turn());
    assert!(app.simulation_running());
    app.keep_patched(generation, found, guard);
    assert!(app.patched().reading().is_some());
    assert!(!app.simulation_running());
}

#[test]
fn a_stale_result_still_ends_the_running_simulation() {
    let (_tmp, app, mods) = reported_pair();
    let generation = app.conflicts_generation();
    let guard = app.simulation_started(&mods);
    app.textures_changed();
    app.keep_patched(generation, None, guard);
    assert!(!app.simulation_running());
}

#[test]
fn a_simulation_is_needed_for_each_new_list() {
    let (_tmp, app, mods) = reported_pair();
    assert!(app.simulation_needed(&mods));
    let _guard = app.simulation_started(&mods);
    assert!(!app.simulation_needed(&mods));
    let without_c: Vec<Mod> = mods.iter().filter(|m| m.id != "t.c").cloned().collect();
    assert!(app.simulation_needed(&without_c));
    let _again = app.simulation_started(&without_c);
    assert!(app.simulation_needed(&mods));
    app.textures_changed();
    assert!(app.simulation_needed(&without_c));
}

#[test]
fn a_rescan_forgets_the_patched_result() {
    let (_tmp, app, mods) = reported_pair();
    *app.patched().writing() = app.simulate_patches(mods, &a_turn());
    app.textures_changed();
    assert!(app.patched().reading().is_none());
}

#[test]
fn sort_waits_for_a_running_simulation_then_sorts_once() {
    let (_tmp, app, mods) = reported_pair();
    let mut order = LoadOrder::read(&app, true, None);
    let generation = app.conflicts_generation();
    let running = app.simulation_started(&mods);
    order.preview_sort(&app);
    assert!(!order.previewing() && order.sort_waiting());
    order.preview_sort(&app);
    assert!(!order.previewing() && order.sort_waiting());
    order.sort_when_patched(&app);
    assert!(!order.previewing() && order.sort_waiting());

    let found = app.simulate_patches(mods, &a_turn());
    app.keep_patched(generation, found, running);
    order.sort_when_patched(&app);
    assert!(order.previewing() && !order.sort_waiting());
    let shown = order.list(Side::Active).mods().to_vec();
    order.sort_when_patched(&app);
    assert_eq!(order.list(Side::Active).mods(), shown);
}

#[test]
fn a_stale_result_still_ends_the_wait() {
    let (_tmp, app, mods) = reported_pair();
    let mut order = LoadOrder::read(&app, true, None);
    let generation = app.conflicts_generation();
    let running = app.simulation_started(&mods);
    order.preview_sort(&app);
    app.textures_changed();
    app.keep_patched(generation, None, running);
    order.sort_when_patched(&app);
    assert!(!order.sort_waiting());
}

#[test]
fn a_list_changed_while_waiting_cancels_the_wait() {
    let (_tmp, app, mods) = reported_pair();
    let mut order = LoadOrder::read(&app, true, None);
    let running = app.simulation_started(&mods);
    order.preview_sort(&app);
    assert!(order.sort_waiting());
    let at = place_of(order.list(Side::Active).mods(), "t.i");
    order.take(&app, Side::Active, at, None);
    assert!(!order.sort_waiting());
    drop(running);
    order.sort_when_patched(&app);
    assert!(!order.previewing());
}

#[test]
fn no_simulation_no_wait() {
    let (_tmp, app, _mods) = reported_pair();
    let mut order = LoadOrder::read(&app, true, None);
    order.preview_sort(&app);
    assert!(!order.sort_waiting());
    assert!(!order.sort_note.is_empty());
}

/// Empties the kept result's hints: only a reuse gives a result without them.
fn emptied_in_the_record(app: &App) {
    let file = app.patches_file();
    let mut record = rimstack_core::kept::patched::Results::read(&file);
    record.results[0].hints.clear();
    record.write(&file).unwrap();
}

fn kept_results(app: &App) -> usize {
    rimstack_core::kept::patched::Results::read(&app.patches_file())
        .results
        .len()
}

fn hints_of(app: &App, mods: &[Mod]) -> usize {
    app.simulate_patches(mods.to_vec(), &a_turn())
        .unwrap()
        .hints
        .len()
}

#[test]
fn a_result_is_kept_and_reused_while_nothing_it_was_made_from_changed() {
    let (_tmp, app, mods) = reported_pair();
    assert_eq!(hints_of(&app, &mods), 1);
    emptied_in_the_record(&app);
    let again = app.simulate_patches(mods, &a_turn()).unwrap();
    assert!(again.hints.is_empty());
}

#[test]
fn a_reused_result_is_the_one_made() {
    let (_tmp, app, mods) = reported_pair();
    let made = app.simulate_patches(mods.clone(), &a_turn()).unwrap();
    let reused = app.simulate_patches(mods, &a_turn()).unwrap();
    assert_eq!(reused.members, made.members);
    assert_eq!(reused.hints, made.hints);
    assert_eq!(reused.unfixed, made.unfixed);
}

#[test]
fn a_patch_file_touched_makes_the_simulation_run_again() {
    let (tmp, app, mods) = reported_pair();
    assert_eq!(hints_of(&app, &mods), 1);
    emptied_in_the_record(&app);
    let patch = std::fs::File::options()
        .write(true)
        .open(tmp.path().join("game/Mods/c/Patches/p.xml"))
        .unwrap();
    patch
        .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(60))
        .unwrap();
    assert_eq!(hints_of(&app, &mods), 1);
}

#[test]
fn another_order_makes_the_simulation_run_again() {
    let (_tmp, app, mods) = reported_pair();
    assert_eq!(hints_of(&app, &mods), 1);
    emptied_in_the_record(&app);
    let mut other = mods.clone();
    other.swap(1, 2);
    app.simulate_patches(other, &a_turn());
    assert_eq!(kept_results(&app), 2);
}

#[test]
fn another_rule_makes_the_simulation_run_again() {
    let (_tmp, app, mods) = reported_pair();
    assert_eq!(hints_of(&app, &mods), 1);
    emptied_in_the_record(&app);
    app.user_rules().writing().add("t.i", "t.c");
    app.simulate_patches(mods, &a_turn());
    assert_eq!(kept_results(&app), 2);
}
