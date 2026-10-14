//! Run against the real install with `cargo test -p rimstack-core real -- --ignored --nocapture`.
//! Read-only.

use super::*;
use crate::rimworld::paths::{detect, Overrides};

/// What every installed mod's code is about: one line per mod, then
/// one per method it patches, to the file named by RIMSTACK_OUT.
#[test]
#[ignore = "needs RimWorld installed"]
fn real_install_code() {
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    let paths = detect(&home, &Overrides::from_env());
    let started = std::time::Instant::now();
    let game = Game::find(paths.game.as_ref().unwrap()).unwrap();
    println!("game read in {:?}", started.elapsed());
    let mut mods = scan(&paths.roots());
    let started = std::time::Instant::now();
    read_code(
        &mut mods,
        Some(&game),
        paths.game_version.as_deref().unwrap(),
    );
    println!("code read in {:?}", started.elapsed());
    let mut out = String::new();
    for m in mods.iter().filter(|m| m.code.facts.types > 0) {
        out += &format!(
            "# {}\t{:?}\t{:?}\n",
            m.display_name(),
            m.kinds,
            m.code.facts
        );
        for aim in &m.code.aims {
            out += &format!(
                "{}.{}\t{:?}\t{:?}\n",
                aim.target, aim.method, aim.when, aim.does
            );
        }
    }
    std::fs::write(std::env::var("RIMSTACK_OUT").unwrap(), out).unwrap();
}

/// Where the installed mods land. Prints the Loaders, Interface and
/// Performance lists.
#[test]
#[ignore = "needs RimWorld installed"]
fn real_install_categories() {
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    let paths = detect(&home, &Overrides::from_env());
    let started = std::time::Instant::now();
    let library = load(&paths).unwrap();
    println!("loaded in {:?}", started.elapsed());
    let all = || library.active.iter().chain(&library.inactive);
    for category in [Category::Loaders, Category::Interface, Category::Tweaks] {
        let names: Vec<&str> = all()
            .filter(|m| m.category == category)
            .map(|m| m.name.as_str())
            .collect();
        println!("{category:?}: {} {names:?}", names.len());
    }
    assert!(all().any(|m| m.category == Category::Loaders));
}

#[test]
#[ignore = "needs RimWorld installed"]
fn real_mods_config_copy_is_unchanged_by_writing_the_same_list() {
    use std::fs;
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    let paths = detect(&home, &Overrides::from_env());
    let before = fs::read_to_string(paths.mods_config().unwrap()).unwrap();
    // Only ever a copy: the real file is never written by a test.
    let dir = tempfile::tempdir().unwrap();
    let copy = dir.path().join("ModsConfig.xml");
    fs::write(&copy, &before).unwrap();
    let ids = crate::rimworld::modsconfig::parse(&before).unwrap().active;
    crate::rimworld::modsconfig::write(&copy, &ids).unwrap();
    assert_eq!(fs::read_to_string(&copy).unwrap(), before);
    println!("{} ids written back, file identical", ids.len());
}

/// The rules detected from files, judged against the order the game
/// was happily running with before RimStack sorted it (kept as
/// ModsConfig.xml.rimstack-original): few of them should be broken.
#[test]
#[ignore = "needs RimWorld installed"]
fn real_install_inferred_rules_mostly_agree_with_a_working_order() {
    use rayon::prelude::*;
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    let paths = detect(&home, &Overrides::from_env());
    let config = paths.mods_config().unwrap();
    let original = config.with_file_name("ModsConfig.xml.rimstack-original");
    let baseline = if original.exists() { original } else { config };
    let ids = crate::rimworld::modsconfig::read(&baseline).unwrap().active;
    let library = build(
        scan(&paths.roots()),
        &ids,
        paths.game_version.as_deref().unwrap(),
    );
    let active: Vec<&Installed> = library
        .active
        .iter()
        .filter_map(|m| library.get(m.installed?))
        .collect();
    let keys: std::collections::HashSet<String> = active.iter().filter_map(|m| m.key()).collect();

    let started = std::time::Instant::now();
    let scanned: Vec<crate::mods::infer::Scanned> = active
        .par_iter()
        .map(|m| {
            crate::mods::infer::scan(&crate::rimworld::loadfolders::folders(
                &m.path,
                &library.game_version,
                &|k| keys.contains(k),
            ))
        })
        .collect();
    let names: Vec<String> = active
        .iter()
        .map(|m| m.display_name().to_string())
        .collect();
    let skip: Vec<bool> = active
        .iter()
        .map(|m| m.source == Source::Official)
        .collect();
    let roots: Vec<std::path::PathBuf> = active.iter().map(|m| m.path.clone()).collect();
    let borrowed: Vec<&crate::mods::infer::Scanned> = scanned.iter().collect();
    let found = crate::mods::infer::infer(&borrowed, &names, &skip, &roots);
    println!(
        "{} mods scanned, {} rules inferred in {:?}",
        active.len(),
        found.len(),
        started.elapsed()
    );
    let defs: usize = scanned.iter().map(|s| s.defines.len()).sum();
    println!(
        "{defs} defs, {} patched names",
        scanned.iter().map(|s| s.patched.len()).sum::<usize>()
    );
    let broken: Vec<&crate::mods::infer::Inferred> =
        found.iter().filter(|r| r.first > r.then).collect();
    println!("{} broken by the baseline order", broken.len());
    for r in broken.iter().take(15) {
        println!(
            "  {} should load after {}: {}",
            names[r.then], names[r.first], r.why
        );
    }
    for r in found.iter().take(8) {
        println!(
            "  ok: {} after {}: {}",
            names[r.then], names[r.first], r.why
        );
    }
}

#[test]
#[ignore = "needs RimWorld installed"]
fn real_install_sort_explains_every_move() {
    use crate::mods::rules::{self, Kind};
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    let library = load(&detect(&home, &Overrides::from_env())).unwrap();
    let started = std::time::Instant::now();
    let abouts = library.abouts(&library.active);
    let mut found = rules::among(&abouts, &library.game_version);
    // With the community rules too, when RimStack has downloaded them.
    let kept = home.join(".local/share/RimStack/communityRules.json");
    if let Ok(Some(community)) = crate::mods::community::read(&kept) {
        found.extend(community.rules(&abouts));
    }
    let sorted = crate::mods::sort::sort(library.active.len(), &found);
    println!("sorted in {:?}", started.elapsed());

    let runs = |order: &[usize]| {
        let cats: Vec<Category> = order.iter().map(|&i| library.active[i].category).collect();
        1 + cats.windows(2).filter(|w| w[0] != w[1]).count()
    };
    let as_is: Vec<usize> = (0..library.active.len()).collect();
    println!(
        "category runs: now {}, sorted {}",
        runs(&as_is),
        runs(&sorted.order)
    );
    let own: Vec<usize> = library.active.iter().map(|m| m.category as usize).collect();
    let grouped = crate::mods::sort::sort_grouped(&own, Category::Last as usize, &found);
    let after: Vec<usize> = grouped.order.iter().map(|&m| grouped.groups[m]).collect();
    let changed = (0..own.len())
        .filter(|&m| own[m] != grouped.groups[m])
        .count();
    println!(
        "grouped: {} runs, {} mods moved, {} mods changed category",
        1 + after.windows(2).filter(|w| w[0] != w[1]).count(),
        grouped.moved().iter().filter(|m| **m).count(),
        changed
    );
    for (to, &m) in grouped.order.iter().enumerate() {
        let c = Category::ALL[grouped.groups[m]];
        let was = library.active[m].category;
        let note = if c != was {
            format!("  (was {})", was.label())
        } else {
            String::new()
        };
        println!(
            "    {:>3} {:<10} {}{}",
            to + 1,
            c.label(),
            library.active[m].name,
            note
        );
    }
    let mut per: std::collections::BTreeMap<Category, usize> = Default::default();
    for m in &library.active {
        *per.entry(m.category).or_default() += 1;
    }
    println!("active per category: {per:?}");
    let name = |i: usize| library.active[i].name.as_str();
    let checked =
        crate::mods::checks::check(&abouts, &library.game_version, |k| library.name_of(k));
    for (i, problems) in checked.iter().enumerate() {
        for p in problems {
            println!("  problem {}: {p:?}", name(i));
        }
    }
    let mut per_kind: std::collections::BTreeMap<Kind, (usize, usize)> = Default::default();
    for r in &found {
        let entry = per_kind.entry(r.kind).or_default();
        entry.0 += 1;
        entry.1 += usize::from(r.broken());
    }
    println!("rules (count, broken by the current order): {per_kind:?}");
    for r in found.iter().filter(|r| r.broken()) {
        println!(
            "  broken {:?}: {} before {}",
            r.kind,
            name(r.first),
            name(r.then)
        );
    }
    for r in &sorted.dropped {
        println!(
            "  dropped {:?}: {} before {}",
            r.kind,
            name(r.first),
            name(r.then)
        );
    }
    for (to, moved) in sorted.moved().into_iter().enumerate() {
        if moved {
            let from = sorted.order[to];
            println!("  move {} from {} to {}", name(from), from + 1, to + 1);
        }
    }
}

#[test]
#[ignore = "needs RimWorld installed"]
fn real_install_loads_and_matches_the_game_s_list() {
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    let paths = detect(&home, &Overrides::from_env());
    println!("{paths:#?}");
    let started = std::time::Instant::now();
    let mods = scan(&paths.roots());
    println!("scanned {} mods in {:?}", mods.len(), started.elapsed());

    let mut reasons: HashMap<String, usize> = HashMap::new();
    for m in &mods {
        if let Err(e) = &m.about {
            let said = e.to_string();
            *reasons
                .entry(said.split(':').next().unwrap().to_string())
                .or_default() += 1;
            println!("  {}: {e}", m.path.display());
        }
    }
    println!("unreadable by reason: {reasons:?}");
    let mut copies: HashMap<String, Vec<Source>> = HashMap::new();
    for m in &mods {
        if let Some(k) = m.key() {
            copies.entry(k).or_default().push(m.source);
        }
    }
    copies.retain(|_, v| v.len() > 1);
    println!("ids installed more than once: {copies:?}");

    let active = crate::rimworld::modsconfig::read(&paths.mods_config().unwrap())
        .unwrap()
        .active;
    let resolved = resolve_active(&active, &mods);
    assert_eq!(resolved.len(), active.len());
    for (written, entry) in active.iter().zip(&resolved) {
        match entry {
            Resolved::Found(m) => assert_eq!(m.key().unwrap(), id::split_steam(written).0),
            Resolved::Missing(id) => println!("active but not installed: {id}"),
        }
    }
    let total = mods.len();
    let library = build(
        mods.clone(),
        &active,
        paths.game_version.as_deref().unwrap(),
    );
    let mut per_category: std::collections::BTreeMap<Category, usize> = Default::default();
    for m in &library.inactive {
        *per_category.entry(m.category).or_default() += 1;
    }
    println!(
        "active {}, inactive {} {per_category:?}, missing {}, unreadable {}",
        library.active.len(),
        library.inactive.len(),
        library.missing,
        library.unreadable
    );
    assert!(
        reasons.values().sum::<usize>() * 100 <= total,
        "more than 1% unreadable"
    );
}

#[test]
#[ignore = "needs RimWorld installed"]
fn real_install_dds_audit() {
    use rayon::prelude::*;
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    let paths = detect(&home, &Overrides::from_env());
    let mods = scan(&paths.roots());
    let started = std::time::Instant::now();
    let bad: Vec<crate::mods::ddsaudit::Bad> = mods
        .par_iter()
        .flat_map(|m| crate::mods::ddsaudit::audit(&m.path))
        .collect();
    println!("{} refused in {:?}", bad.len(), started.elapsed());
    for b in bad.iter().take(12) {
        println!("  {} — {}", b.file.display(), b.why);
    }
}
