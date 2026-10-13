//! Load orders kept under a name, to go back to later: one small file
//! each in RimStack's data folder.

use crate::say;

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    pub name: String,
    pub mods: usize,
    /// When it was last saved, in seconds since 1970.
    pub modified: u64,
}

/// The file a name is kept in. Characters a file name cannot hold become
/// `_`, so two names that differ only in those share a file.
fn file_for(dir: &Path, name: &str) -> PathBuf {
    let safe: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_.()".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    dir.join(format!("{safe}.json"))
}

pub fn save(dir: &Path, name: &str, ids: &[String]) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err(say!("Give the list a name."));
    }
    let path = file_for(dir, name);
    let failed = |e: std::io::Error| format!("{}: {e}", path.display());
    std::fs::create_dir_all(dir).map_err(failed)?;
    let text = json!({ "name": name.trim(), "ids": ids }).to_string();
    std::fs::write(&path, text).map_err(failed)
}

fn read(path: &Path) -> Option<(String, Vec<String>)> {
    let root: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    let ids = root.get("ids")?.as_array()?;
    let ids = ids
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    Some((root.get("name")?.as_str()?.to_string(), ids))
}

/// Every kept list, by name. Files that are not lists are skipped.
pub fn all(dir: &Path) -> Vec<Saved> {
    let mut found: Vec<Saved> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let (name, ids) = read(&entry.path())?;
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some(Saved {
                name,
                mods: ids.len(),
                modified: modified.duration_since(UNIX_EPOCH).ok()?.as_secs(),
            })
        })
        .collect();
    found.sort_by_key(|s| s.name.to_lowercase());
    found
}

/// Deletes all but the `keep` most recently saved lists in a folder.
/// For snapshots, which would otherwise pile up.
pub fn prune(dir: &Path, keep: usize) {
    let mut found = all(dir);
    found.sort_by_key(|s| std::cmp::Reverse(s.modified));
    for old in found.iter().skip(keep) {
        // Best effort: a snapshot that cannot be deleted is only clutter.
        let _ = delete(dir, &old.name);
    }
}

pub fn load(dir: &Path, name: &str) -> Result<Vec<String>, String> {
    let path = file_for(dir, name);
    read(&path)
        .map(|(_, ids)| ids)
        .ok_or_else(|| say!("{} is not a saved list", path.display()))
}

pub fn delete(dir: &Path, name: &str) -> Result<(), String> {
    let path = file_for(dir, name);
    std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_list_is_kept_listed_loaded_and_deleted_by_name() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("lists");
        assert!(all(&dir).is_empty());
        save(&dir, "Medieval run", &ids(&["a", "b"])).unwrap();
        save(&dir, "base", &ids(&["a"])).unwrap();
        let found = all(&dir);
        let names: Vec<&str> = found.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["base", "Medieval run"]);
        assert_eq!(found[1].mods, 2);
        assert_eq!(load(&dir, "Medieval run").unwrap(), ids(&["a", "b"]));
        delete(&dir, "base").unwrap();
        assert_eq!(all(&dir).len(), 1);
        assert!(load(&dir, "base").is_err());
    }

    #[test]
    fn saving_under_the_same_name_replaces_the_list() {
        let tmp = tempfile::tempdir().unwrap();
        save(tmp.path(), "run", &ids(&["a"])).unwrap();
        save(tmp.path(), " run ", &ids(&["b", "c"])).unwrap();
        assert_eq!(all(tmp.path()).len(), 1);
        assert_eq!(load(tmp.path(), "run").unwrap(), ids(&["b", "c"]));
    }

    #[test]
    fn a_name_cannot_leave_the_folder_or_be_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("lists");
        save(&dir, "../../escape/it", &ids(&["a"])).unwrap();
        assert!(!tmp.path().join("escape").exists());
        assert_eq!(all(&dir)[0].name, "../../escape/it");
        assert!(save(&dir, "  ", &ids(&["a"])).is_err());
    }

    #[test]
    fn pruning_keeps_the_given_number_of_lists() {
        let tmp = tempfile::tempdir().unwrap();
        for n in 0..5 {
            save(tmp.path(), &format!("snapshot {n}"), &ids(&["a"])).unwrap();
        }
        prune(tmp.path(), 3);
        assert_eq!(all(tmp.path()).len(), 3);
        prune(tmp.path(), 10);
        assert_eq!(all(tmp.path()).len(), 3);
    }

    #[test]
    fn other_files_in_the_folder_are_not_lists() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("notes.json"), "[1,2]").unwrap();
        std::fs::write(tmp.path().join("x.txt"), "hello").unwrap();
        assert!(all(tmp.path()).is_empty());
    }
}
