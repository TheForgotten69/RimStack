//! What the Lists and History menus show: the lists kept under a name,
//! the snapshots taken before a change, and the history of changes.

use super::App;
use crate::changes::Changes;
use rimstack_core::base::error::Error;
use rimstack_core::base::translate::in_words;
use rimstack_core::kept::{journal, lists};
use rimstack_core::rimworld::modsconfig;
use rimstack_core::say;
use serde::Serialize;

/// How many snapshots the Lists menu offers; the rest stay in the folder.
const SNAPSHOTS_SHOWN: usize = 5;

/// How many lines of the history are shown.
const HISTORY_SHOWN: usize = 40;

/// The English an earlier version named a snapshot by, before its kind.
const OLDER_NAMES: [(&str, &str); 4] = [
    ("Before saving, ", "@saving"),
    ("Before sorting, ", "@sorting"),
    ("Before using another list, ", "@list"),
    ("Before finding the broken mod, ", "@hunt"),
];

/// The month names an earlier version wrote in the system's language.
const MONTHS: [&[&str]; 12] = [
    &["jan", "janv"],
    &["feb", "fév", "févr"],
    &["mar", "mars", "mär"],
    &["apr", "avr"],
    &["may", "mai"],
    &["jun", "juin"],
    &["jul", "juil"],
    &["aug", "août"],
    &["sep", "sept"],
    &["oct", "okt"],
    &["nov"],
    &["dec", "déc", "dez"],
];

fn before(kind: &str, when: &str) -> Option<String> {
    Some(match kind {
        "@saving" => say!("Before saving, {}", when),
        "@sorting" => say!("Before sorting, {}", when),
        "@list" => say!("Before using another list, {}", when),
        "@hunt" => say!("Before finding the broken mod, {}", when),
        _ => return None,
    })
}

/// A snapshot's moment as its name holds it, to the minute:
/// "2026-10-13 02.56.55", or from an earlier version the system's own
/// form, "12 oct. 2026 20.17.39".
fn stamp(text: &str) -> String {
    let parts: Vec<&str> = text.split(' ').collect();
    let clock = |time: &str| -> Option<String> {
        let numbers: Vec<&str> = time.split('.').collect();
        let digits = |s: &&str| s.len() == 2 && s.bytes().all(|b| b.is_ascii_digit());
        (numbers.len() == 3 && numbers.iter().all(digits))
            .then(|| format!("{}:{}", numbers[0], numbers[1]))
    };
    let dated = |day: &str| {
        let b = day.as_bytes();
        day.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && day
                .bytes()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    };
    match parts.as_slice() {
        [day, time] if dated(day) => clock(time).map(|t| format!("{day} {t}")),
        [day, month, year, time] => {
            let month = month.trim_end_matches('.').to_lowercase();
            let at = MONTHS
                .iter()
                .position(|names| names.contains(&month.as_str()));
            let day = day.parse::<u8>().ok().filter(|_| day.len() <= 2);
            let year = Some(*year).filter(|y| y.len() == 4 && y.parse::<u16>().is_ok());
            match (at, day, year, clock(time)) {
                (Some(at), Some(day), Some(year), Some(t)) => {
                    Some(format!("{year}-{:02}-{day:02} {t}", at + 1))
                }
                _ => None,
            }
        }
        _ => None,
    }
    .unwrap_or_else(|| text.to_string())
}

/// A snapshot's name in words: what it was taken before, and when.
pub fn snapshot_title(name: &str) -> String {
    let (kind, rest) = name.split_once(' ').unwrap_or((name, ""));
    if let Some(title) = before(kind, &stamp(rest)) {
        return title;
    }
    OLDER_NAMES
        .iter()
        .find_map(|(english, kind)| {
            let rest = name.strip_prefix(english)?;
            before(kind, &stamp(rest))
        })
        .unwrap_or_else(|| name.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub name: String,
    pub title: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryLine {
    pub time: u64,
    pub said: String,
    /// The snapshot that takes it back, if it is still kept.
    pub restore: String,
    /// The mod folder whose textures it set aside.
    pub bring_back: String,
}

impl App {
    pub fn saved_lists(&self) -> Vec<lists::Saved> {
        lists::all(&self.lists_dir())
    }

    /// The newest snapshots, newest first.
    pub fn recent_snapshots(&self) -> Vec<Snapshot> {
        let mut found = lists::all(&self.snapshots_dir());
        found.sort_by_key(|l| std::cmp::Reverse(l.modified));
        found
            .into_iter()
            .take(SNAPSHOTS_SHOWN)
            .map(|l| Snapshot {
                title: snapshot_title(&l.name),
                name: l.name,
            })
            .collect()
    }

    /// The history, newest first, in words.
    pub fn history(&self) -> Vec<HistoryLine> {
        let snapshots: Vec<String> = lists::all(&self.snapshots_dir())
            .into_iter()
            .map(|l| l.name)
            .collect();
        journal::read(&self.journal_file())
            .into_iter()
            .rev()
            .take(HISTORY_SHOWN)
            .map(|entry| {
                let (how, about) = entry.revert.split_once(' ').unwrap_or_default();
                let kept = |name: &str| snapshots.iter().any(|s| s == name);
                HistoryLine {
                    time: entry.time,
                    said: in_words(&entry.what),
                    restore: match how == "list" && kept(about) {
                        true => about.to_string(),
                        false => String::new(),
                    },
                    bring_back: match how {
                        "unhide" => about.to_string(),
                        _ => String::new(),
                    },
                }
            })
            .collect()
    }

    /// Keeps `ids`, or the load order being edited, under a name.
    pub fn keep_list(&self, name: &str, ids: Option<Vec<String>>) -> Result<(), Error> {
        let ids = ids.unwrap_or_else(|| self.session().current().to_vec());
        lists::save(&self.lists_dir(), name, &ids)?;
        self.publish(Changes::LISTS);
        Ok(())
    }

    pub fn delete_list(&self, name: &str) {
        match lists::delete(&self.lists_dir(), name) {
            Ok(()) => self.publish(Changes::LISTS),
            Err(problem) => rimstack_core::problem!("{problem}"),
        }
    }

    pub fn list_named(&self, name: &str) -> Option<Vec<String>> {
        lists::load(&self.lists_dir(), name).ok()
    }

    pub fn snapshot_named(&self, name: &str) -> Option<Vec<String>> {
        lists::load(&self.snapshots_dir(), name).ok()
    }

    /// The load order as it was before RimStack first saved.
    pub fn original_list(&self) -> Option<Vec<String>> {
        let config = self.paths().mods_config()?;
        let original = config.with_file_name("ModsConfig.xml.rimstack-original");
        modsconfig::read(&original).ok().map(|config| config.active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::tests::installed;

    #[test]
    fn a_snapshot_is_titled_by_its_kind_and_moment_whatever_version_named_it() {
        assert_eq!(
            snapshot_title("@sorting 2026-10-13 02.56.55"),
            "Before sorting, 2026-10-13 02:56"
        );
        assert_eq!(
            snapshot_title("Before saving, 12 oct. 2026 20.17.39"),
            "Before saving, 2026-10-12 20:17"
        );
        assert_eq!(
            snapshot_title("@hunt 3 Mär 2025 08.05.00"),
            "Before finding the broken mod, 2025-03-03 08:05"
        );
        assert_eq!(
            snapshot_title("@list someday"),
            "Before using another list, someday"
        );
        assert_eq!(snapshot_title("My own list"), "My own list");
    }

    #[test]
    fn a_list_is_kept_found_and_deleted_and_the_history_says_what_takes_it_back() {
        let (_tmp, app) = installed();
        app.keep_list("Mine", Some(vec!["a.b".into()])).unwrap();
        assert!(app.keep_list(" ", None).is_err());
        assert_eq!(app.saved_lists()[0].mods, 1);
        assert_eq!(app.list_named("Mine").unwrap(), ["a.b"]);
        app.delete_list("Mine");
        assert!(app.list_named("Mine").is_none());

        let snapshot = app.snapshot("@saving", &["a.b".to_string()]).unwrap();
        app.note_change(
            rimstack_core::keep!("Saved the load order: {}", "x"),
            Some(&snapshot),
        );
        app.note_change("[\"Set aside\"]".into(), None);
        let history = app.history();
        assert_eq!(history[0].said, "Set aside");
        assert_eq!(history[1].said, "Saved the load order: x");
        assert_eq!(history[1].restore, snapshot);
        assert_eq!(app.recent_snapshots()[0].name, snapshot);
        assert!(app.recent_snapshots()[0]
            .title
            .starts_with("Before saving, 2"));
        assert_eq!(app.snapshot_named(&snapshot).unwrap(), ["a.b"]);
        assert_eq!(app.original_list(), None);
    }
}
