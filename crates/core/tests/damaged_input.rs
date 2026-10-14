//! What strangers' files can do to the readers: made-up and damaged
//! input, in bulk. Nothing here runs SteamCMD, Steam or the game.
//!
//! `RIMSTACK_DLL` may name any mod's .dll for the assembly fuzz; without
//! it that probe says so and passes.

use rimstack_core::mods::list::{Category, GroupedList, Mod};
use rimstack_core::mods::rules::{Kind, Rule};
use rimstack_core::{
    base::markup, base::translate, base::vdf, kept::notes, kept::owntheme, mods::community,
    mods::ddsaudit, mods::share, mods::sort, rimworld::about, rimworld::dotnet,
    rimworld::loadfolders, rimworld::logs, rimworld::modsconfig, rimworld::rimsaves,
    rimworld::steamworks, rimworld::workshop, sources::changes,
};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::{Duration, Instant};

struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Pieces the parsers here look for, with text that is not one byte per
/// letter between them.
const PIECES: &[&str] = &[
    "<",
    ">",
    "</",
    "/>",
    "[",
    "]",
    "[/",
    "=",
    "\"",
    "'",
    "&",
    "&amp;",
    "#",
    ":",
    "/",
    "\\",
    ".",
    ",",
    " ",
    "\n",
    "\r\n",
    "\t",
    "\u{feff}",
    "é",
    "ß",
    "İ",
    "日本",
    "𝔘",
    "\u{202e}",
    "0",
    "7",
    "1.6",
    "42",
    "activeMods",
    "<activeMods>",
    "</activeMods>",
    "<activeMods/>",
    "li",
    "<li>",
    "</li>",
    "url",
    "[url=",
    "[/url]",
    "img",
    "[img]",
    "[/img]",
    "b",
    "i",
    "h1",
    "list",
    "*",
    "https://",
    "http://",
    "steam://",
    "javascript:",
    "github.com/a/b",
    "?id=",
    "<p id=\"",
    "\">",
    "</p>",
    "<br>",
    "RimWorld ",
    "ERROR",
    "Exception",
    "  at ",
    "(wrapper",
    "[Ref ",
    "2026-10-12 19:37:35.060 ",
    "[Verse] ",
    "\"path\"",
    "\"apps\"",
    "{",
    "}",
    "loadFolders",
    "<v1.6>",
    "IfModActive",
    "tag:",
    "is:",
    "author:",
    "patches:",
    "subscribe:",
    "_steam",
    "packageId",
    "<ModMetaData>",
    "<name>",
    "modDependencies",
    "\0",
];

fn text(random: &mut Random) -> String {
    let pieces = random.below(40);
    (0..pieces)
        .map(|_| PIECES[random.below(PIECES.len())])
        .collect()
}

/// Runs `probe` on `rounds` made-up texts; the texts it panicked on.
fn panics_on(rounds: usize, seed: u64, probe: impl Fn(&str)) -> Vec<String> {
    let mut random = Random(seed);
    let quiet = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let failed: Vec<String> = (0..rounds)
        .map(|_| text(&mut random))
        .filter(|made| catch_unwind(AssertUnwindSafe(|| probe(made))).is_err())
        .collect();
    std::panic::set_hook(quiet);
    failed
}

fn a_mod(name: &str, category: Category) -> Mod {
    Mod {
        id: name.to_lowercase(),
        name: name.to_string(),
        author: "someone".to_string(),
        version: "1.0".to_string(),
        category,
        installed: None,
        tags: Vec::new(),
        local: false,
        code: false,
        patched: std::sync::Arc::from(Vec::<String>::new()),
        sub: String::new(),
        load_micros: None,
    }
}

#[test]
fn no_text_parser_panics_on_thirty_thousand_made_up_texts() {
    let failed = panics_on(30_000, 0x5eed, |made| {
        let _ = modsconfig::parse(made);
        let _ = modsconfig::with_active(made, &["a.b".to_string()]);
        let _ = modsconfig::short_version(made);
        let _ = markup::rich(made);
        let _ = vdf::libraries(made);
        let _ = about::parse(made.as_bytes());
        let _ = logs::errors(made);
        let _ = logs::game_version(made);
        let _ = logs::mentioned(
            made,
            &[("İstanbul mod".to_string(), 1), ("日本 mod".into(), 2)],
        );
        let _ = changes::steam_notes(made);
        let _ = workshop::ids_in(made);
        let _ = workshop::parse_details(made);
        let _ = workshop::parse_collections(made);
        let _ = workshop::installed_times(made);
        let _ = workshop::installed_sizes(made);
        let _ = share::ids_in(made);
        let _ = loadfolders::declared(made, "1.6");
        let _ = rimsaves::parse_settings(made);
        let _ = rimsaves::parse_meta(made);
        let _ = rimsaves::split(made);
        let _ = community::parse(made);
        let _ = community::works_on(made);
        let _ = notes::tags_in(made);
        let _ = steamworks::parse(made);
        let _ = owntheme::contrast(made, "#000000");
        let _ = owntheme::contrast(&format!("#{made}"), "#000000");
        let _ = ddsaudit::problem(made.as_bytes(), made.len() as u64);
    });
    assert!(failed.is_empty(), "panicked on {:?}", failed.first());
}

#[test]
fn the_filter_box_takes_any_text() {
    let mods: Vec<Mod> = ["Harmony", "İstanbul", "日本 mod", "Core"]
        .iter()
        .map(|name| a_mod(name, Category::Content))
        .collect();
    let failed = panics_on(20_000, 0xf117e4, |made| {
        let mut list = GroupedList::new(mods.clone());
        list.set_filter(made);
        let _ = list.matching();
        let _ = list.rows().len();
    });
    assert!(failed.is_empty(), "panicked on {:?}", failed.first());
}

/// A real assembly with bytes changed at random, counts made huge, or cut
/// short: read in full each time. Reports the slowest reading.
#[test]
fn a_damaged_assembly_is_refused_or_read_and_never_panics() {
    let Some(path) = std::env::var_os("RIMSTACK_DLL") else {
        println!("RIMSTACK_DLL not set: assembly fuzz skipped");
        return;
    };
    let sound = std::fs::read(path).unwrap();
    assert!(dotnet::Assembly::read(&sound).is_some());
    let mut random = Random(0xd11);
    let quiet = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let (mut panicked, mut read, mut slowest) = (0, 0, Duration::ZERO);
    for round in 0..4_000 {
        let mut dll = sound.clone();
        match round % 4 {
            0 => {
                let cut = random.below(dll.len());
                dll.truncate(cut);
            }
            1 => {
                // Counts and offsets are four bytes: make some of them huge.
                for _ in 0..1 + random.below(4) {
                    let at = random.below(dll.len() - 4);
                    dll[at..at + 4].copy_from_slice(&[0xff, 0xff, 0xff, 0x7f]);
                }
            }
            _ => {
                // Mostly in the first sixteenth, where the headers and tables are.
                for _ in 0..1 + random.below(24) {
                    let span = if random.below(3) == 0 {
                        dll.len()
                    } else {
                        dll.len() / 16
                    };
                    let at = random.below(span.max(1));
                    dll[at] = random.next() as u8;
                }
            }
        }
        let started = Instant::now();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let Some(assembly) = dotnet::Assembly::read(&dll) else {
                return false;
            };
            let _ = assembly.references();
            let _ = assembly.types();
            let _ = assembly.patches();
            let _ = assembly.hand_applied();
            let _ = assembly.looked_up();
            true
        }));
        slowest = slowest.max(started.elapsed());
        match outcome {
            Ok(true) => read += 1,
            Ok(false) => {}
            Err(_) => panicked += 1,
        }
    }
    std::panic::set_hook(quiet);
    println!("assembly fuzz: 4000 damaged files, {read} still read, {panicked} panics, slowest {slowest:?}");
    assert_eq!(panicked, 0);
    assert!(
        slowest < Duration::from_secs(2),
        "one reading took {slowest:?}"
    );
}

/// How many of the assemblies named in the file `RIMSTACK_DLLS` (a path
/// per line) are read, and the first few that are refused.
#[test]
fn installed_assemblies_are_read() {
    let Some(listing) = std::env::var_os("RIMSTACK_DLLS") else {
        println!("RIMSTACK_DLLS not set: skipped");
        return;
    };
    let listing = std::fs::read_to_string(listing).unwrap();
    let (mut read, mut refused, mut panicked) = (0, Vec::new(), Vec::new());
    let started = Instant::now();
    for path in listing.lines().filter(|line| !line.is_empty()) {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let assembly = dotnet::Assembly::read(&bytes)?;
            Some((assembly.types().len(), assembly.patches().len()))
        }));
        match outcome {
            Ok(Some(_)) => read += 1,
            // Not managed code at all (a native library) has no .NET header to find.
            Ok(None) => refused.push(path),
            Err(_) => panicked.push(path),
        }
    }
    println!(
        "assemblies: {read} read, {} refused, {} panicked, in {:?}",
        refused.len(),
        panicked.len(),
        started.elapsed()
    );
    for path in refused.iter().take(8) {
        println!("  refused: {path}");
    }
    assert!(panicked.is_empty(), "panicked on {:?}", panicked.first());
}

/// The size of the user's own list (2,300 mods), with what the community
/// rules add: five mods "at the bottom" are 11,000 rules. One pair of
/// rules that contradict sends the sorter down its loop-breaking path.
#[test]
fn a_contradiction_among_many_rules_still_sorts_in_reasonable_time() {
    let count = 2_300;
    let mut rules: Vec<Rule> = (0..count - 1)
        .map(|m| Rule {
            first: m,
            then: m + 1,
            kind: Kind::LoadAfter,
        })
        .collect();
    for bottom in 0..5 {
        rules.extend((5..count).map(|above| Rule {
            first: above,
            then: bottom,
            kind: Kind::CommunityBottom,
        }));
    }
    let started = Instant::now();
    let sorted = sort::sort(count, &rules);
    let took = started.elapsed();
    println!(
        "sort with a loop: {count} mods, {} rules, {} dropped, {took:?}",
        rules.len(),
        sorted.dropped.len()
    );
    assert_eq!(sorted.order.len(), count);
    assert!(took < Duration::from_secs(5), "took {took:?}");
}

/// Not a defect by itself: what the design is. An error of the core is a
/// sentence already in the chosen language, so nothing can tell one
/// error from another but its wording, and the wording follows a global.
/// It sets that global, which the other probes here share while they
/// run: none of them reads a sentence, so it is left to run with them.
#[test]
fn an_error_is_a_sentence_in_whatever_language_is_set_for_the_process() {
    let not_dds = b"PNG....";
    assert_eq!(
        ddsaudit::problem(not_dds, 7).as_deref(),
        Some("not a DDS file")
    );
    let french = [(
        "not a DDS file".to_string(),
        "pas un fichier DDS".to_string(),
    )];
    translate::set(french.into_iter().collect());
    assert_eq!(
        ddsaudit::problem(not_dds, 7).as_deref(),
        Some("pas un fichier DDS")
    );
    translate::set(Default::default());
}
