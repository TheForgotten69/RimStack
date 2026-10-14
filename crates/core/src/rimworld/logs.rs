//! The game's `Player.log`: the errors in it, grouped, and which mod each
//! most likely comes from.
//!
//! Up to 1.6 the game ran on Unity, whose log has no level markers:
//! errors are recognised by how they start, which misses unusual
//! wordings and is a best guess. From 1.7 the game runs on its own
//! engine (Waylith), which starts every entry with a time and a level
//! and writes the log to `Logs/Player.log`.

use crate::base::error::Error;
use std::collections::HashMap;
use std::path::Path;

/// One kind of error, however many times it was logged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorGroup {
    pub head: String,
    /// The lines under the first occurrence: stack trace and patches.
    pub trace: Vec<String>,
    pub count: usize,
    /// Line number of the first occurrence, from 1.
    pub line: usize,
}

const HEADS: [&str; 9] = [
    "Error",
    "Exception",
    "XML error",
    "Config error",
    "Could not",
    "Failed to",
    "Cannot ",
    "Tried to",
    "Root level exception",
];

fn starts_an_error(line: &str) -> bool {
    !continues(line) && (HEADS.iter().any(|h| line.starts_with(h)) || line.contains("Exception"))
}

/// Whether a line belongs to the entry above it: indented, a stack
/// reference, or a bare `Type.Method()` frame.
fn continues(line: &str) -> bool {
    let bare_frame = || {
        let before_args = line.split('(').next().unwrap_or("");
        line.ends_with(')') && !before_args.is_empty() && !before_args.contains(' ')
    };
    line.starts_with([' ', '\t'])
        || line.starts_with("[Ref ")
        || line.starts_with("(wrapper")
        || bare_frame()
}

/// Two errors are the same kind when they differ only in numbers.
fn kind(head: &str) -> String {
    head.chars().filter(|c| !c.is_ascii_digit()).collect()
}

/// What follows the time on a line that starts an entry of a 1.7 log:
/// "2026-10-12 19:37:35.060 ERROR [Verse] Tried loading…" gives
/// "ERROR [Verse] Tried loading…".
fn stamped(line: &str) -> Option<&str> {
    let b = line.as_bytes();
    let digits = |from: usize, to: usize| {
        b.get(from..to)
            .is_some_and(|d| d.iter().all(u8::is_ascii_digit))
    };
    let shaped = digits(0, 4) && digits(5, 7) && digits(8, 10) && digits(11, 13) && digits(20, 23);
    let marks = b.get(4) == Some(&b'-') && b.get(10) == Some(&b' ') && b.get(13) == Some(&b':');
    (shaped && marks).then(|| line[23..].trim_start())
}

/// The frames every entry of a 1.7 log starts its trace with: the
/// logging itself, which says nothing about where the error is from.
fn logging_frame(line: &str) -> bool {
    let frame = line.trim_start();
    frame.starts_with("at Brutal.Logging.") || frame.starts_with("at Verse.Log.")
}

fn errors_stamped(text: &str) -> Vec<ErrorGroup> {
    let mut groups: Vec<ErrorGroup> = Vec::new();
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    // The group the lines being read belong to, if their entry is an error.
    let mut open: Option<usize> = None;
    for (number, line) in text.lines().enumerate() {
        let Some(entry) = stamped(line) else {
            // Only the first of a kind keeps its trace.
            if let Some(at) = open.filter(|at| groups[*at].count == 1) {
                if !line.is_empty() && !logging_frame(line) {
                    groups[at].trace.push(line.trim_end().to_string());
                }
            }
            continue;
        };
        let message = ["ERROR", "FATAL"]
            .iter()
            .find_map(|level| entry.strip_prefix(level));
        open = message.map(|message| {
            // "[Verse] Tried…": the part of the game that says it is
            // nearly always the same and not the news.
            let message = message.trim_start();
            let said = match message.split_once("] ") {
                Some((category, rest)) if category.starts_with('[') => rest,
                _ => message,
            };
            let at = *by_kind.entry(kind(said)).or_insert(groups.len());
            match groups.get_mut(at) {
                Some(group) => group.count += 1,
                None => groups.push(ErrorGroup {
                    head: said.trim_end().to_string(),
                    trace: Vec::new(),
                    count: 1,
                    line: number + 1,
                }),
            }
            at
        });
    }
    groups
}

/// The errors in a log, in the order each kind first appears.
pub fn errors(text: &str) -> Vec<ErrorGroup> {
    if text.lines().take(40).any(|line| stamped(line).is_some()) {
        return errors_stamped(text);
    }
    let mut groups: Vec<ErrorGroup> = Vec::new();
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    let mut lines = text.lines().enumerate().peekable();
    while let Some((number, line)) = lines.next() {
        if !starts_an_error(line) {
            continue;
        }
        let mut trace = Vec::new();
        while let Some((_, next)) = lines.peek() {
            if next.is_empty() || !continues(next) {
                break;
            }
            trace.push(next.trim_end().to_string());
            lines.next();
        }
        match by_kind.get(&kind(line)) {
            Some(&at) => groups[at].count += 1,
            None => {
                by_kind.insert(kind(line), groups.len());
                groups.push(ErrorGroup {
                    head: line.trim_end().to_string(),
                    trace,
                    count: 1,
                    line: number + 1,
                });
            }
        }
    }
    groups
}

/// Reads a log file; one that does not exist has no errors. Bytes that
/// are not UTF-8 are replaced, not refused.
pub fn read(path: &Path) -> Result<String, Error> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(Error::file(path, e)),
    }
}

/// Code that is part of the game or its runtime, never a mod's.
const NOT_A_MOD: [&str; 13] = [
    "Brutal",
    "Waylith",
    "RimWorldWaylith",
    "Microsoft",
    "System",
    "Verse",
    "RimWorld",
    "UnityEngine",
    "HarmonyLib",
    "Mono",
    "MonoMod",
    "Unity",
    "LudeonTK",
];

/// Every mod whose code appears in an error, in the order met, each
/// once: a stack frame whose namespace is the name of a mod's assembly.
/// `assemblies` maps lower-case assembly names (no `.dll`) to whatever
/// names the mod. The first is the one the error most likely comes from.
pub fn mods_in<T: Copy + PartialEq>(group: &ErrorGroup, assemblies: &HashMap<String, T>) -> Vec<T> {
    let mut found: Vec<T> = Vec::new();
    let lines = std::iter::once(&group.head).chain(&group.trace);
    for line in lines {
        for word in line.split(|c: char| !(c.is_alphanumeric() || c == '.' || c == '_')) {
            let mut parts: Vec<&str> = word.split('.').filter(|p| !p.is_empty()).collect();
            if parts.len() < 2 || NOT_A_MOD.contains(&parts[0]) {
                continue;
            }
            // "Author.Mod.Type.Method": try the longest namespace first.
            parts.pop();
            while !parts.is_empty() {
                if let Some(owner) = assemblies.get(&parts.join(".").to_lowercase()) {
                    if !found.contains(owner) {
                        found.push(*owner);
                    }
                    break;
                }
                parts.pop();
            }
        }
    }
    found
}

/// The game's version as it writes it near the top of its log:
/// `RimWorld 1.6.4871 rev600`.
pub fn game_version(text: &str) -> Option<String> {
    let lines = text.lines().take(400);
    // A 1.7 log has the time, the level and "[Verse]" before it.
    let said = |line| {
        stamped(line)
            .and_then(|entry| entry.split_once("] "))
            .map_or(line, |(_, rest)| rest)
    };
    let mut found = lines.filter_map(|line| said(line).strip_prefix("RimWorld "));
    let version = found.find(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))?;
    Some(version.trim_end().to_string())
}

/// The mod a line names: the longest of `names` that stands in it as
/// whole words, in any case. Names under five letters say too little.
pub fn mentioned<T: Copy>(line: &str, names: &[(String, T)]) -> Option<T> {
    let line = line.to_lowercase();
    let whole = |name: &str| {
        line.match_indices(name).any(|(at, _)| {
            let before = line[..at].chars().next_back();
            let after = line[at + name.len()..].chars().next();
            !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
        })
    };
    names
        .iter()
        .filter(|(name, _)| name.chars().count() >= 5 && whole(&name.to_lowercase()))
        .max_by_key(|(name, _)| name.len())
        .map(|(_, owner)| *owner)
}

/// The log of the last launch and of the one before, newest first, among
/// the places the game has written logs to: its settings folder up to
/// 1.6, `Logs` inside it from 1.7. After an update both hold a log, and
/// only the dates say which is the last.
pub fn files(config: &Path) -> Vec<std::path::PathBuf> {
    let places = [
        "Logs/Player.log",
        "Logs/Player-prev.log",
        "Player.log",
        "Player-prev.log",
    ];
    let mut found: Vec<(std::time::SystemTime, std::path::PathBuf)> = places
        .iter()
        .map(|place| config.join(place))
        .filter_map(|file| Some((std::fs::metadata(&file).ok()?.modified().ok()?, file)))
        .collect();
    found.sort_by_key(|(written, _)| std::cmp::Reverse(*written));
    found.into_iter().map(|(_, file)| file).collect()
}

/// Copies a log into `dir` under the second it was last written, unless
/// that copy is there, and deletes the oldest copies beyond `limit`. The
/// game keeps two logs; this is how an older launch can still be read.
pub fn keep(log: &Path, dir: &Path, limit: usize) -> std::io::Result<()> {
    let written = std::fs::metadata(log)?.modified()?;
    let stamp = written
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let copy = dir.join(format!("{stamp}.log"));
    if limit > 0 && !copy.exists() {
        std::fs::create_dir_all(dir)?;
        std::fs::copy(log, &copy)?;
    }
    let kept = kept(dir);
    for (_, old) in kept.iter().rev().skip(limit) {
        std::fs::remove_file(old)?;
    }
    Ok(())
}

/// The logs kept in `dir`, oldest first, each with the second it was written.
pub fn kept(dir: &Path) -> Vec<(u64, std::path::PathBuf)> {
    let entries = std::fs::read_dir(dir).into_iter().flatten().flatten();
    let mut found: Vec<(u64, std::path::PathBuf)> = entries
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|x| x == "log"))
        .filter_map(|path| Some((path.file_stem()?.to_str()?.parse().ok()?, path)))
        .collect();
    found.sort();
    found
}

/// The kept log of a launch started at `from`, before the next one at
/// `until`: the last written in between.
pub fn kept_for(
    kept: &[(u64, std::path::PathBuf)],
    from: u64,
    until: Option<u64>,
) -> Option<&Path> {
    let mut within = kept
        .iter()
        .filter(|(at, _)| *at >= from && until.is_none_or(|next| *at < next));
    within.next_back().map(|(_, path)| path.as_path())
}

/// For each group, where its run begins: a run is three or more
/// neighbours that are one error about different things, their first
/// lines the same but for the last word. A group in no run begins its own.
pub fn runs(groups: &[ErrorGroup]) -> Vec<usize> {
    fn stem(group: &ErrorGroup) -> Option<&str> {
        let head = group.head.trim_end();
        head.rsplit_once(' ').map(|(stem, _)| stem)
    }
    let mut starts: Vec<usize> = (0..groups.len()).collect();
    let mut at = 0;
    while at < groups.len() {
        let alike = |other: &ErrorGroup| stem(other).is_some() && stem(other) == stem(&groups[at]);
        let length = groups[at..].iter().take_while(|g| alike(g)).count().max(1);
        if length >= 3 {
            starts[at..at + length].fill(at);
        }
        at += length;
    }
    starts
}

/// A mod the errors point at, or `None` for the errors pointing at no mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Culprit<T> {
    pub mod_id: Option<T>,
    /// Different errors pointing at it; `times` counts every logging of them.
    pub kinds: usize,
    pub times: usize,
}

/// Who the errors point at, from each error's mod and how often it was
/// logged: the errors pointing at no mod first, then the mod logged
/// most; ties stay in the log's order.
pub fn culprits<T: Copy + PartialEq>(
    errors: impl IntoIterator<Item = (Option<T>, usize)>,
) -> Vec<Culprit<T>> {
    let mut found: Vec<Culprit<T>> = Vec::new();
    for (mod_id, count) in errors {
        let at = found.iter().position(|c| c.mod_id == mod_id);
        let at = at.unwrap_or_else(|| {
            found.push(Culprit {
                mod_id,
                kinds: 0,
                times: 0,
            });
            found.len() - 1
        });
        found[at].kinds += 1;
        found[at].times += count;
    }
    found.sort_by_key(|c| (c.mod_id.is_some(), std::cmp::Reverse(c.times)));
    found
}

/// What the filter box holds to list the errors that point at no mod.
pub const NO_MOD: &str = "mod:none";

/// Whether an error is listed under this filter. With none, a run of
/// alike errors shows as its first until it is opened; with a text, the
/// errors whose first line or mod holds it, whatever the case.
pub fn listed(
    filter: &str,
    head: &str,
    mod_name: Option<&str>,
    first_of_run: bool,
    run_open: bool,
) -> bool {
    let wanted = filter.trim().to_lowercase();
    match wanted.as_str() {
        "" => first_of_run || run_open,
        NO_MOD => mod_name.is_none(),
        text => {
            let holds = |said: &str| said.to_lowercase().contains(text);
            holds(head) || mod_name.is_some_and(holds)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_filter_lists_errors_by_text_or_by_having_no_mod() {
        use super::{listed, NO_MOD};
        assert!(listed("", "Null reference", None, true, false));
        assert!(!listed("", "Null reference", None, false, false));
        assert!(listed("", "Null reference", None, false, true));
        assert!(listed(" NULL ", "Null reference", None, false, false));
        assert!(listed(
            "harm",
            "Could not patch",
            Some("Harmony"),
            false,
            false
        ));
        assert!(!listed("harm", "Could not patch", None, false, false));
        assert!(listed(NO_MOD, "Could not patch", None, false, false));
        assert!(!listed(
            NO_MOD,
            "Could not patch",
            Some("Harmony"),
            false,
            false
        ));
    }

    #[test]
    fn errors_are_counted_by_the_mod_they_point_at() {
        let found = super::culprits([(Some(4), 2), (None, 1), (Some(7), 9), (Some(4), 3)]);
        let said: Vec<(Option<usize>, usize, usize)> =
            found.iter().map(|c| (c.mod_id, c.kinds, c.times)).collect();
        assert_eq!(said, [(None, 1, 1), (Some(7), 1, 9), (Some(4), 2, 5)]);
        assert!(super::culprits::<usize>([]).is_empty());
    }

    #[test]
    fn a_log_is_kept_once_and_the_oldest_give_way() {
        let tmp = tempfile::tempdir().unwrap();
        let (log, dir) = (tmp.path().join("Player.log"), tmp.path().join("kept"));
        for stamp in [100, 200, 300] {
            std::fs::write(dir_file(&dir, stamp), "old").unwrap();
        }
        std::fs::write(&log, "now").unwrap();
        super::keep(&log, &dir, 3).unwrap();
        super::keep(&log, &dir, 3).unwrap();
        let kept = super::kept(&dir);
        assert_eq!(kept.len(), 3);
        assert_eq!(kept[0].0, 200);
        assert_eq!(std::fs::read_to_string(&kept[2].1).unwrap(), "now");
        assert_eq!(
            super::kept_for(&kept, 150, Some(250)),
            Some(kept[0].1.as_path())
        );
        assert_eq!(super::kept_for(&kept, 301, Some(302)), None);
        super::keep(&log, &dir, 0).unwrap();
        assert!(super::kept(&dir).is_empty());
    }

    fn dir_file(dir: &std::path::Path, stamp: u64) -> std::path::PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        dir.join(format!("{stamp}.log"))
    }

    use super::*;

    const WAYLITH: &str = "=== RimWorld by Ludeon Studios log ===
Runtime:   .NET 10.0.10-waylithclr.dev

2026-10-12 19:37:34.278  INFO [Verse] RimWorld 1.7.4976 rev1043
    at Brutal.Logging.LogCategory.Info(LogString`1 message) [0x0]
2026-10-12 19:37:35.060 ERROR [Verse] Tried loading mod with the same packageId multiple times: A.B. Ignoring the duplicates.
/mods/3296654393
    at Brutal.Logging.LogCategory.Error(LogString`1 message) [0x0]
    at Verse.Log.Error(String text) [0x1C]
    at Verse.ModLister.TryAddMod(ModMetaData mod) [0xBC]
2026-10-12 19:37:35.100  WARN [Verse] Mod X dependency needs a downloadUrl. Could not be helped.
2026-10-12 19:37:58.850 ERROR [Verse] Could not load reference to Verse.ResearchProjectDef named Gunlink1
    at Verse.Log.Error(String text) [0x1C]
    at MyMod.Patches.Postfix() [0x2]
2026-10-12 19:37:58.851 ERROR [Verse] Could not load reference to Verse.ResearchProjectDef named Gunlink2
    at SomethingElse.Run() [0x2]
";

    #[test]
    fn a_log_of_the_new_engine_is_read_by_its_levels() {
        let found = errors(WAYLITH);
        let heads: Vec<(&str, usize, usize)> = found
            .iter()
            .map(|g| (g.head.as_str(), g.count, g.line))
            .collect();
        assert_eq!(
            heads,
            [
                ("Tried loading mod with the same packageId multiple times: A.B. Ignoring the duplicates.", 1, 6),
                ("Could not load reference to Verse.ResearchProjectDef named Gunlink1", 2, 12),
            ]
        );
        // What the entry holds besides its first line, less the logging's own frames.
        assert_eq!(
            found[0].trace,
            [
                "/mods/3296654393",
                "    at Verse.ModLister.TryAddMod(ModMetaData mod) [0xBC]"
            ]
        );
        assert_eq!(found[1].trace, ["    at MyMod.Patches.Postfix() [0x2]"]);
        assert_eq!(game_version(WAYLITH).as_deref(), Some("1.7.4976 rev1043"));
        let assemblies: HashMap<String, u8> = [("mymod".to_string(), 7)].into();
        assert_eq!(mods_in(&found[1], &assemblies), [7]);
        assert!(mods_in(&found[0], &assemblies).is_empty());
    }

    #[test]
    fn the_last_log_is_the_newest_wherever_the_game_wrote_it() {
        let dir = tempfile::tempdir().unwrap();
        assert!(files(dir.path()).is_empty());
        std::fs::write(dir.path().join("Player.log"), "old engine").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::create_dir(dir.path().join("Logs")).unwrap();
        std::fs::write(dir.path().join("Logs/Player.log"), "new engine").unwrap();
        assert_eq!(
            files(dir.path()),
            [
                dir.path().join("Logs/Player.log"),
                dir.path().join("Player.log")
            ]
        );
    }

    const LOG: &str = "Mono path[0] = '/game/Managed'
[SizedApparel]: RimJobWorld Found
Error in static constructor of DynamicAnimationOffsets.ModInit: System.TypeInitializationException: Invalid path
[Ref A0F2FD20]
  at System.IO.Path.GetDirectoryName (System.String path) [0x0000d] in <e3b>:0 
  at DynamicAnimationOffsets.OffsetPaths.Initialise () [0x00001] in D:\\Mods\\Source\\OffsetPaths.cs:80 
DynamicAnimationOffsets.ModInit..cctor()
   --- End of inner exception stack trace ---
[BetterTrees] Finished applying tree overrides.
Error in static constructor of DynamicAnimationOffsets.ModInit: System.TypeInitializationException: Invalid path
[Ref A0F2FD20] Duplicate stacktrace, see ref for original

Could not resolve cross-reference to Verse.ThingDef named Steel7 (wanter=thingDef)
Could not resolve cross-reference to Verse.ThingDef named Steel9 (wanter=thingDef)
Loaded 5 things
";

    #[test]
    fn errors_are_grouped_with_their_first_trace_and_counted() {
        let found = errors(LOG);
        assert_eq!(found.len(), 2);
        assert!(found[0].head.starts_with("Error in static constructor"));
        assert_eq!(found[0].count, 2);
        assert_eq!(found[0].line, 3);
        assert_eq!(found[0].trace.len(), 5);
        assert_eq!(
            found[0].trace[4],
            "   --- End of inner exception stack trace ---"
        );
    }

    #[test]
    fn errors_that_differ_only_in_numbers_are_one_kind() {
        let found = errors(LOG);
        assert!(found[1].head.contains("Steel7"));
        assert_eq!(found[1].count, 2);
        assert!(found[1].trace.is_empty());
    }

    #[test]
    fn the_game_version_is_the_line_the_game_writes_about_itself() {
        let log =
            "Setting breakpad minidump AppID = 294100\nRimWorld 1.6.4871 rev600\nRimWorld is fun\n";
        assert_eq!(game_version(log).as_deref(), Some("1.6.4871 rev600"));
        assert_eq!(game_version(LOG), None);
    }

    #[test]
    fn a_mod_is_recognised_by_its_name_as_a_whole_word_the_longest_first() {
        let names = [
            ("Animation".to_string(), 1),
            ("Dynamic Animation Offsets".to_string(), 2),
            ("Core".to_string(), 3),
            ("HAR".to_string(), 4),
        ];
        let by = |line: &str| mentioned(line, &names);
        let line =
            "Mod Dynamic Animation Offsets dependency (c0ffee.rimworld.animations) needs a url";
        assert_eq!(by(line), Some(2));
        assert_eq!(by("mod dynamic animation offsets broke"), Some(2));
        // Not inside another word, and not a name too short to mean anything.
        assert_eq!(by("Reanimation failed"), None);
        assert_eq!(by("Could not load Core textures"), None);
        assert_eq!(by("CHARGE"), None);
    }

    #[test]
    fn ordinary_lines_are_not_errors() {
        assert!(errors("[Mod] active.\nLoaded 5 things\n  at Some.Frame ()\n").is_empty());
        assert!(errors("").is_empty());
    }

    #[test]
    fn the_mod_is_found_from_the_first_frame_that_is_not_the_game_s() {
        let assemblies: HashMap<String, usize> = [
            ("dynamicanimationoffsets".to_string(), 7),
            ("ilyvion.loadingprogress".to_string(), 9),
        ]
        .into();
        let found = errors(LOG);
        // Loading Progress is in the same trace, further down.
        assert_eq!(mods_in(&found[0], &assemblies), [7]);
        assert!(mods_in(&found[1], &assemblies).is_empty());
    }

    #[test]
    fn a_dotted_assembly_name_is_matched_by_its_longest_namespace() {
        let assemblies: HashMap<String, usize> = [("author.mod".to_string(), 1)].into();
        let group = ErrorGroup {
            head: "Exception in thing".into(),
            trace: vec!["  at Author.Mod.Things.Type.Run () [0x1] in <a>:0".into()],
            count: 1,
            line: 1,
        };
        assert_eq!(mods_in(&group, &assemblies), [1]);
    }

    #[test]
    fn a_missing_log_reads_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read(&dir.path().join("Player.log")), Ok(String::new()));
    }

    #[test]
    fn three_neighbours_that_differ_in_their_last_word_are_one_run() {
        let group = |head: &str| ErrorGroup {
            head: head.to_string(),
            trace: Vec::new(),
            count: 1,
            line: 1,
        };
        let groups = [
            group("Harmony failed"),
            group("Could not load reference to Def named A"),
            group("Could not load reference to Def named B"),
            group("Could not load reference to Def named C"),
            group("Could not load reference to Thing named D"),
            group("Could not load reference to Thing named E"),
            group("Oops"),
        ];
        assert_eq!(runs(&groups), [0, 1, 1, 1, 4, 5, 6]);
        assert!(runs(&[]).is_empty());
    }
}
