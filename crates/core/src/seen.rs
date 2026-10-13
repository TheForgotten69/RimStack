//! Which mod folders were installed when the game was last started from
//! RimStack, so the ones that came since can be shown as new.

use std::collections::HashSet;
use std::path::Path;

/// The folders on record; `None` when nothing was ever recorded, which
/// makes no mod new.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seen(Option<HashSet<String>>);

impl Seen {
    /// One folder per line.
    pub fn read(file: &Path) -> Self {
        let text = std::fs::read_to_string(file).ok();
        Self(text.map(|t| t.lines().map(String::from).collect()))
    }

    pub fn of(folders: impl IntoIterator<Item = String>) -> Self {
        Self(Some(folders.into_iter().collect()))
    }

    pub fn write(&self, file: &Path) -> Result<(), String> {
        let failed = |e: std::io::Error| format!("{}: {e}", file.display());
        let mut lines: Vec<&str> = self.0.iter().flatten().map(String::as_str).collect();
        lines.sort_unstable();
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(failed)?;
        }
        std::fs::write(file, lines.join("\n")).map_err(failed)
    }

    pub fn is_recorded(&self) -> bool {
        self.0.is_some()
    }

    pub fn is_new(&self, folder: &str) -> bool {
        self.0.as_ref().is_some_and(|seen| !seen.contains(folder))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_new_before_a_first_record() {
        let dir = tempfile::tempdir().unwrap();
        let seen = Seen::read(&dir.path().join("seen.txt"));
        assert!(!seen.is_recorded());
        assert!(!seen.is_new("/mods/a"));
    }

    #[test]
    fn a_folder_that_came_after_the_record_is_new() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("kept/seen.txt");
        let now = Seen::of(["/mods/a".to_string(), "/mods/b".to_string()]);
        now.write(&file).unwrap();
        let seen = Seen::read(&file);
        assert_eq!(seen, now);
        assert!(!seen.is_new("/mods/a"));
        assert!(seen.is_new("/mods/c"));
    }
}
