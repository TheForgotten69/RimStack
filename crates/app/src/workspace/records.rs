//! What RimStack keeps of what it did: the load order as it was before
//! something rearranged it, and a line in the history.

use super::App;
use crate::changes::Changes;
use rimstack_core::base::sentence::Sentence;
use rimstack_core::kept::{journal, lists};
use std::path::PathBuf;

/// How many snapshots are kept; the oldest give way.
const SNAPSHOTS_KEPT: usize = 30;

impl App {
    pub fn snapshots_dir(&self) -> PathBuf {
        self.data_dir().join("snapshots")
    }

    pub fn lists_dir(&self) -> PathBuf {
        self.data_dir().join("lists")
    }

    pub fn journal_file(&self) -> PathBuf {
        self.data_dir().join("journal.json")
    }

    /// Keeps `list` among the snapshots, named by its kind ("@saving",
    /// "@sorting", "@list", "@hunt") and the local time to the second,
    /// and a number after it for a second one in that second; gives the
    /// name, or nothing when it could not be kept.
    pub fn snapshot(&self, kind: &str, list: &[String]) -> Option<String> {
        let dir = self.snapshots_dir();
        let at = format!(
            "{kind} {}",
            chrono::Local::now().format("%Y-%m-%d %H.%M.%S")
        );
        let numbered = (2..).map(|n| format!("{at} {n}"));
        let name = std::iter::once(at.clone())
            .chain(numbered)
            .find(|name| !lists::taken(&dir, name))?;
        match lists::save(&dir, &name, list) {
            Ok(()) => {
                lists::prune(&dir, SNAPSHOTS_KEPT);
                self.publish(Changes::LISTS);
                Some(name)
            }
            Err(problem) => {
                rimstack_core::problem!("{problem}");
                None
            }
        }
    }

    /// A line in the history, and the snapshot that takes it back, if
    /// there is one.
    pub fn note_change(&self, what: Sentence, snapshot: Option<&str>) {
        let revert = snapshot.map_or(String::new(), |name| format!("list {name}"));
        self.note(what, revert);
    }

    /// A line in the history; `revert` is "list <snapshot>", "unhide
    /// <mod folder>" or nothing.
    pub fn note(&self, what: Sentence, revert: String) {
        let entry = journal::Entry {
            time: super::now_secs(),
            what: what.to_kept().to_string(),
            revert,
        };
        match journal::add(&self.journal_file(), entry) {
            Ok(()) => self.publish(Changes::LISTS),
            Err(problem) => rimstack_core::problem!("{problem}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::workspace::tests::installed;
    use rimstack_core::kept::{journal, lists};

    #[test]
    fn a_snapshot_is_kept_by_its_kind_and_named_in_the_history() {
        let (_tmp, app) = installed();
        let name = app.snapshot("@saving", &["a.b".to_string()]).unwrap();
        assert!(name.starts_with("@saving 2"), "{name}");
        assert_eq!(lists::load(&app.snapshots_dir(), &name).unwrap(), ["a.b"]);
        app.note_change(
            rimstack_core::say!("Saved the load order: {}", "x"),
            Some(&name),
        );
        let kept = journal::read(&app.journal_file());
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].revert, format!("list {name}"));
    }

    #[test]
    fn two_snapshots_in_one_second_are_both_kept() {
        let (_tmp, app) = installed();
        let first = app.snapshot("@saving", &["a".to_string()]).unwrap();
        let second = app.snapshot("@saving", &["b".to_string()]).unwrap();
        let dir = app.snapshots_dir();
        assert_ne!(first, second);
        assert_eq!(lists::load(&dir, &first).unwrap(), ["a"]);
        assert_eq!(lists::load(&dir, &second).unwrap(), ["b"]);
    }
}
