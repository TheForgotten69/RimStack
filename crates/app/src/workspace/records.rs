//! What RimStack keeps of what it did: the load order as it was before
//! something rearranged it, and a line in the history.

use super::App;
use crate::changes::Changes;
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
    /// "@sorting", "@list", "@hunt") and the local time to the second;
    /// gives the name, or nothing when it could not be kept.
    pub fn snapshot(&self, kind: &str, list: &[String]) -> Option<String> {
        let name = format!(
            "{kind} {}",
            chrono::Local::now().format("%Y-%m-%d %H.%M.%S")
        );
        let dir = self.snapshots_dir();
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

    /// A line in the history: `what` as `translate::kept` writes it, and
    /// the snapshot that takes it back, if there is one.
    pub fn note_change(&self, what: String, snapshot: Option<&str>) {
        let entry = journal::Entry {
            time: super::now_secs(),
            what,
            revert: snapshot.map_or(String::new(), |name| format!("list {name}")),
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
            rimstack_core::keep!("Saved the load order: {}", "x"),
            Some(&name),
        );
        let kept = journal::read(&app.journal_file());
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].revert, format!("list {name}"));
    }
}
