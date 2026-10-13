//! The game's save files: which mods each was played with, and how that
//! compares with a load order.

use crate::say;

use crate::id;
use roxmltree::Document;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// What a save says about the game it came from, from its `<meta>` block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Header {
    /// As written: "1.6.4871 rev600".
    pub game_version: Option<String>,
    /// Package ids in load order, as written.
    pub mod_ids: Vec<String>,
    /// Display names, one per id; may be shorter in old saves.
    pub mod_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFile {
    pub path: PathBuf,
    pub name: String,
    /// When it was last written, in seconds since 1970.
    pub modified: u64,
    pub bytes: u64,
    /// The reason is short enough to show beside the save.
    pub header: Result<Header, String>,
}

/// A save is tens of megabytes and its header a few kilobytes at the
/// start, so reading stops at the end of `<meta>`, or here.
const HEADER_LIMIT: usize = 4 * 1024 * 1024;

pub fn read_header(path: &Path) -> Result<Header, String> {
    const END: &[u8] = b"</meta>";
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut start = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    let end = loop {
        let searched_from = start.len().saturating_sub(END.len());
        let read = file.read(&mut chunk).map_err(|e| e.to_string())?;
        start.extend_from_slice(&chunk[..read]);
        let found = start[searched_from..]
            .windows(END.len())
            .position(|w| w == END);
        if let Some(at) = found {
            break searched_from + at + END.len();
        }
        if read == 0 || start.len() > HEADER_LIMIT {
            return Err(say!("no mod list at the start of the file"));
        }
    };
    let text = std::str::from_utf8(&start[..end]).map_err(|_| say!("not UTF-8"))?;
    parse_header(&format!("{text}</savegame>"))
}

fn parse_header(text: &str) -> Result<Header, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let doc = Document::parse(text).map_err(|e| say!("malformed XML: {}", e))?;
    let meta = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("meta"))
        .ok_or_else(|| say!("no mod list at the start of the file"))?;
    let child = |name: &str| meta.children().find(|n| n.has_tag_name(name));
    let items = |name: &str| -> Vec<String> {
        let list = child(name).into_iter().flat_map(|n| n.children());
        list.filter(|n| n.has_tag_name("li"))
            .map(|n| n.text().unwrap_or("").trim().to_string())
            .collect()
    };
    Ok(Header {
        game_version: child("gameVersion")
            .and_then(|n| n.text())
            .map(|s| s.trim().to_string()),
        mod_ids: items("modIds"),
        mod_names: items("modNames"),
    })
}

/// Every `.rws` file in a `Saves` folder, newest first. A folder that does
/// not exist has no saves.
pub fn list(dir: &Path) -> Vec<SaveFile> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut saves: Vec<SaveFile> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("rws")))
        .map(|path| {
            let meta = std::fs::metadata(&path).ok();
            let modified = meta.as_ref().and_then(|m| m.modified().ok());
            let since = modified.and_then(|t| t.duration_since(UNIX_EPOCH).ok());
            SaveFile {
                name: path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                modified: since.map_or(0, |d| d.as_secs()),
                bytes: meta.map_or(0, |m| m.len()),
                header: read_header(&path),
                path,
            }
        })
        .collect();
    saves.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.name.cmp(&b.name)));
    saves
}

/// How a save's mods differ from a load order. Positions point into the
/// list each came from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Difference {
    /// In the save but not in the list: loading the save may break.
    pub missing: Vec<usize>,
    /// In the list but not in the save: usually harmless.
    pub added: Vec<usize>,
    /// The mods both have are not in the same order.
    pub reordered: bool,
}

impl Difference {
    pub fn is_none(&self) -> bool {
        self.missing.is_empty() && self.added.is_empty() && !self.reordered
    }
}

/// Compares by package id, ignoring case and the `_steam` suffix.
pub fn compare(save_ids: &[String], list_ids: &[String]) -> Difference {
    let key = |s: &String| id::split_steam(s).0;
    let in_save: Vec<String> = save_ids.iter().map(key).collect();
    let in_list: Vec<String> = list_ids.iter().map(key).collect();
    let save_set: HashSet<&String> = in_save.iter().collect();
    let list_set: HashSet<&String> = in_list.iter().collect();
    let positions = |of: &[String], other: &HashSet<&String>, wanted: bool| -> Vec<usize> {
        let keep = |(_, k): &(usize, &String)| other.contains(k) == wanted;
        of.iter().enumerate().filter(keep).map(|(i, _)| i).collect()
    };
    let shared = |of: &[String], other: &HashSet<&String>| -> Vec<String> {
        of.iter().filter(|k| other.contains(k)).cloned().collect()
    };
    Difference {
        missing: positions(&in_save, &list_set, false),
        added: positions(&in_list, &save_set, false),
        reordered: shared(&in_save, &list_set) != shared(&in_list, &save_set),
    }
}

/// Copies `save` into `backups` under a name starting with `stamp`, then
/// deletes all but the newest `keep` copies there. Returns the copy.
pub fn back_up(save: &Path, backups: &Path, stamp: u64, keep: usize) -> Result<PathBuf, String> {
    let failed = |e: std::io::Error| format!("{}: {e}", backups.display());
    std::fs::create_dir_all(backups).map_err(failed)?;
    let name = save.file_name().unwrap_or_default().to_string_lossy();
    let copy = backups.join(format!("{stamp:012}-{name}"));
    std::fs::copy(save, &copy).map_err(failed)?;

    // Names start with the time, so name order is age order.
    let mut copies: Vec<PathBuf> = std::fs::read_dir(backups)
        .map_err(failed)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("rws")))
        .collect();
    copies.sort();
    let extra = copies.len().saturating_sub(keep);
    for old in &copies[..extra] {
        std::fs::remove_file(old).map_err(failed)?;
    }
    Ok(copy)
}

/// The game version every save was written with, as "1.6", when there is
/// more than one save and none is from the installed game: a warning all
/// of them share tells none apart. Each save: its version text, and
/// whether that is another game version than the installed one.
pub fn version_all_share(saves: &[(&str, bool)]) -> Option<String> {
    let short = |version: &str| version.split('.').take(2).collect::<Vec<_>>().join(".");
    let first = short(saves.first()?.0);
    let shared = saves.len() > 1 && saves.iter().all(|(v, other)| *other && short(v) == first);
    shared.then_some(first).filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_version_is_shared_only_by_several_saves_all_from_another_game() {
        let other = [("1.6.4871 rev600", true), ("1.6.4500 rev1", true)];
        assert_eq!(super::version_all_share(&other), Some("1.6".to_string()));
        assert_eq!(super::version_all_share(&other[..1]), None);
        assert_eq!(
            super::version_all_share(&[("1.6.1", true), ("1.5.2", true)]),
            None
        );
        assert_eq!(
            super::version_all_share(&[("1.6.1", true), ("1.7.2", false)]),
            None
        );
    }

    use super::*;

    const START: &str = "\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>
<savegame>
\t<meta>
\t\t<gameVersion>1.6.4871 rev600</gameVersion>
\t\t<modIds>
\t\t\t<li>brrainz.harmony</li>
\t\t\t<li>ludeon.rimworld</li>
\t\t</modIds>
\t\t<modSteamIds><li>2009463077</li><li>0</li></modSteamIds>
\t\t<modNames>
\t\t\t<li>Harmony</li>
\t\t\t<li>Core</li>
\t\t</modNames>
\t</meta>
\t<game><lots>";

    fn ids(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn save_in(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn the_header_is_read_without_the_rest_of_the_file() {
        let dir = tempfile::tempdir().unwrap();
        // The rest is not valid XML and is far larger than one read.
        let body = format!("{START}{}", "<junk".repeat(100_000));
        let header = read_header(&save_in(dir.path(), "a.rws", &body)).unwrap();
        assert_eq!(header.game_version.as_deref(), Some("1.6.4871 rev600"));
        assert_eq!(header.mod_ids, ["brrainz.harmony", "ludeon.rimworld"]);
        assert_eq!(header.mod_names, ["Harmony", "Core"]);
    }

    #[test]
    fn a_header_that_ends_across_two_reads_is_still_found() {
        let dir = tempfile::tempdir().unwrap();
        let at = START.find("</meta>").unwrap();
        // Pad so that "</meta>" straddles the 64 KiB read boundary.
        let padding = " ".repeat(64 * 1024 - at - 3);
        let body = START.replacen("<modIds>", &format!("{padding}<modIds>"), 1);
        let header = read_header(&save_in(dir.path(), "a.rws", &body)).unwrap();
        assert_eq!(header.mod_ids.len(), 2);
    }

    #[test]
    fn a_file_that_is_not_a_save_gives_a_reason() {
        let dir = tempfile::tempdir().unwrap();
        let reason = read_header(&save_in(dir.path(), "a.rws", "hello")).unwrap_err();
        assert_eq!(reason, "no mod list at the start of the file");
    }

    #[test]
    fn saves_are_listed_newest_first_and_other_files_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        save_in(dir.path(), "notes.txt", "x");
        save_in(dir.path(), "broken.rws", "x");
        save_in(dir.path(), "good.rws", START);
        let saves = list(dir.path());
        let mut names: Vec<&str> = saves.iter().map(|s| s.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, ["broken", "good"]);
        assert!(saves.iter().all(|s| s.modified > 0 && s.bytes > 0));
        assert!(saves.iter().any(|s| s.header.is_err()));
        assert!(list(&dir.path().join("nowhere")).is_empty());
    }

    #[test]
    fn the_same_mods_in_the_same_order_are_no_difference() {
        let d = compare(&ids(&["A.a", "b.b_steam"]), &ids(&["a.a", "b.b"]));
        assert!(d.is_none());
    }

    #[test]
    fn missing_added_and_reordered_are_told_apart() {
        let d = compare(&ids(&["a", "gone", "b"]), &ids(&["b", "new", "a"]));
        assert_eq!(d.missing, [1]);
        assert_eq!(d.added, [1]);
        assert!(d.reordered);
        let d = compare(&ids(&["a", "b"]), &ids(&["a", "new", "b"]));
        assert!(!d.reordered && d.missing.is_empty());
    }

    #[test]
    fn backing_up_keeps_only_the_newest_copies() {
        let dir = tempfile::tempdir().unwrap();
        let save = save_in(dir.path(), "colony.rws", START);
        let backups = dir.path().join("backups");
        for stamp in 1..=7 {
            back_up(&save, &backups, stamp, 5).unwrap();
        }
        let mut kept: Vec<String> = std::fs::read_dir(&backups)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        kept.sort();
        assert_eq!(kept.len(), 5);
        assert_eq!(kept[0], "000000000003-colony.rws");
        assert_eq!(
            std::fs::read_to_string(backups.join(&kept[4])).unwrap(),
            START
        );
    }
}
