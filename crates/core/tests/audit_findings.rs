//! What the audit of 2026-10-14 (`docs/audit/2026-10-14-code-audit.md`)
//! found broken, each as what must hold. Everything runs in a scratch folder.

use rimstack_core::{
    base::atomic_file, kept::lists, mods::blame, mods::textures, mods::tree, rimworld::loadfolders,
    rimworld::logs, rimworld::modsconfig,
};
use std::fs;
use std::path::Path;
use std::time::Duration;

fn a_mod(root: &Path) {
    fs::create_dir_all(root.join("About")).unwrap();
    fs::write(
        root.join("About/About.xml"),
        "<ModMetaData><packageId>a.b</packageId></ModMetaData>",
    )
    .unwrap();
}

#[cfg(unix)]
fn a_mod_with_one_texture_and_links_back(root: &Path, links: &[&str]) {
    a_mod(root);
    let textures_dir = root.join("Textures");
    fs::create_dir_all(&textures_dir).unwrap();
    fs::write(textures_dir.join("one.png"), b"png").unwrap();
    for link in links {
        std::os::unix::fs::symlink(&textures_dir, textures_dir.join(link)).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn a_texture_is_found_once_whatever_links_lie_beside_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("mod");
    a_mod_with_one_texture_and_links_back(&root, &["again"]);

    let found = textures::pngs(&root).len();
    let counted: usize = tree::folder_tree(&root)
        .iter()
        .filter(|(name, _)| name.starts_with("Textures"))
        .map(|(_, files)| *files)
        .sum();
    println!("F2: 1 PNG on disk, pngs() finds {found}, folder_tree() counts {counted} files");
    assert_eq!((found, counted), (1, 1));
}

#[cfg(unix)]
#[test]
fn two_links_back_do_not_keep_the_walk_from_ending() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("mod");
    a_mod_with_one_texture_and_links_back(&root, &["again", "and-again"]);

    let (done, waiting) = std::sync::mpsc::channel();
    std::thread::spawn(move || done.send(textures::pngs(&root).len()));
    let found = waiting.recv_timeout(Duration::from_secs(3));
    println!("F2: after three seconds: {found:?}");
    assert_eq!(found, Ok(1));
}

#[test]
fn a_mod_is_only_read_from_folders_inside_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("mods/evil");
    a_mod(&root);
    fs::create_dir_all(tmp.path().join("mods/victim/Textures")).unwrap();
    fs::write(
        root.join("LoadFolders.xml"),
        "<loadFolders><v1.6><li>../victim</li></v1.6></loadFolders>",
    )
    .unwrap();

    let loaded = loadfolders::folders(&root, "1.6", &|_| true);
    println!("F3: folders the mod is read from: {loaded:?}");
    let root = fs::canonicalize(&root).unwrap();
    let inside = |folder: &std::path::PathBuf| fs::canonicalize(folder).unwrap().starts_with(&root);
    assert!(loaded.iter().all(inside));
}

#[test]
fn saving_a_list_never_replaces_another_list() {
    let tmp = tempfile::tempdir().unwrap();
    let ids = |id: &str| vec![id.to_string()];
    lists::save(tmp.path(), "war_crimes", &ids("first.list")).unwrap();
    let second = lists::save(tmp.path(), "war/crimes", &ids("second.list"));
    let all = lists::all(tmp.path());
    println!(
        "F4: second save {}; lists kept: {:?}",
        if second.is_ok() {
            "accepted"
        } else {
            "refused"
        },
        all.iter().map(|l| &l.name).collect::<Vec<_>>()
    );
    assert_eq!(
        lists::load(tmp.path(), "war_crimes").unwrap(),
        ids("first.list")
    );
}

#[test]
fn two_writers_of_one_file_both_succeed_and_one_text_is_kept_whole() {
    let tmp = tempfile::tempdir().unwrap();
    let file = &tmp.path().join("notes.json");
    let texts = ["a".repeat(256 * 1024), "b".repeat(64 * 1024)];
    let (mut failed, mut mixed) = (0, 0);
    for _ in 0..200 {
        let outcomes: Vec<bool> = std::thread::scope(|scope| {
            let writers: Vec<_> = texts
                .iter()
                .map(|text| scope.spawn(move || atomic_file::write(file, text).is_ok()))
                .collect();
            writers.into_iter().map(|w| w.join().unwrap()).collect()
        });
        failed += outcomes.iter().filter(|ok| !**ok).count();
        let kept = fs::read_to_string(file).unwrap();
        mixed += usize::from(!texts.contains(&kept));
    }
    println!("F1: of 200 rounds, {failed} writes failed, {mixed} files held neither text whole");
    assert_eq!((failed, mixed), (0, 0));
}

#[test]
fn an_error_is_not_counted_against_a_mod_for_a_word_it_is_named_by() {
    let group = logs::ErrorGroup {
        head: "Exception loading def from file Things.xml: loading failed".to_string(),
        trace: Vec::new(),
        count: 1,
        line: 1,
    };
    let clues = blame::Clues {
        assemblies: &std::collections::HashMap::new(),
        ids: &[(
            "ann.loading".to_string(),
            rimstack_core::mods::library::ModId::new(0, 7),
        )],
    };
    let (errors, _) = blame::of(&[group], &clues, &|_| "Loading".to_string());
    println!("F5: counted against mod {}", errors[0].mod_id);
    assert_eq!(errors[0].mod_id, -1);
}

#[cfg(unix)]
#[test]
fn saving_writes_through_a_linked_modsconfig_and_keeps_its_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let real = tmp.path().join("dotfiles/ModsConfig.xml");
    fs::create_dir_all(real.parent().unwrap()).unwrap();
    let before = "<ModsConfigData>\n  <activeMods>\n    <li>ludeon.rimworld</li>\n  </activeMods>\n</ModsConfigData>";
    fs::write(&real, before).unwrap();
    fs::set_permissions(&real, fs::Permissions::from_mode(0o600)).unwrap();
    let link = tmp.path().join("ModsConfig.xml");
    std::os::unix::fs::symlink(&real, &link).unwrap();

    modsconfig::write(&link, &["ludeon.rimworld".into(), "a.b".into()]).unwrap();

    let still_a_link = fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink();
    let mode = fs::metadata(&link).unwrap().permissions().mode() & 0o777;
    println!("F6: still a link: {still_a_link}; mode was 600, is {mode:o}");
    assert!(still_a_link);
    assert_eq!(mode, 0o600);
    assert_ne!(fs::read_to_string(&real).unwrap(), before);
}
