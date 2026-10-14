//! The community rules (RimSort's `communityRules.json`): load rules that
//! mod authors did not write themselves. Downloaded only when asked, kept
//! in RimStack's data folder, never shipped with RimStack.

use crate::error::Error;
use crate::say;

use crate::about::About;
use crate::id;
use crate::rules::{self, Kind, Rule};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const URL: &str =
    "https://raw.githubusercontent.com/RimSort/Community-Rules-Database/main/communityRules.json";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Entry {
    pub load_after: Vec<String>,
    pub load_before: Vec<String>,
    /// Belongs below every mod that is not itself marked this way.
    pub bottom: bool,
    /// Belongs near the top, with the libraries.
    pub top: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Community {
    /// When the file was published, in seconds since 1970.
    pub timestamp: Option<i64>,
    /// By folded package id.
    entries: HashMap<String, Entry>,
}

impl Community {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every "loads before" pair in the file, as (first, then) folded
    /// ids, in a fixed order. RimSort keeps your own rules in a file of
    /// this same format, which is how they are imported.
    pub fn pairs(&self) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = Vec::new();
        for (id, entry) in &self.entries {
            pairs.extend(
                entry
                    .load_after
                    .iter()
                    .map(|other| (id::fold(other), id.clone())),
            );
            pairs.extend(
                entry
                    .load_before
                    .iter()
                    .map(|other| (id.clone(), id::fold(other))),
            );
        }
        pairs.sort();
        pairs.dedup();
        pairs
    }

    /// Whether the community marks this mod to load near the top. As in
    /// RimSort, that means with the libraries, after the game; it is a
    /// matter of category and makes no rule.
    pub fn loads_at_top(&self, package_id: &str) -> bool {
        let entry = self.entries.get(&id::fold(package_id));
        entry.is_some_and(|e| e.top)
    }

    /// The community's rules between the listed mods, in list order.
    pub fn rules(&self, mods: &[Option<&About>]) -> Vec<Rule> {
        let position = rules::positions(mods);
        let entry_of =
            |about: &Option<&About>| about.and_then(|a| self.entries.get(&id::fold(&a.package_id)));
        let bottom: Vec<bool> = mods
            .iter()
            .map(|about| entry_of(about).is_some_and(|e| e.bottom))
            .collect();

        let mut found = Vec::new();
        for (me, about) in mods.iter().enumerate() {
            let Some(entry) = entry_of(about) else {
                continue;
            };
            let mut add = |targets: &[String], kind: Kind| {
                for target in targets {
                    let Some(&other) = position.get(&id::split_steam(target).0) else {
                        continue;
                    };
                    if other == me {
                        continue;
                    }
                    let (first, then) = match kind {
                        Kind::CommunityBefore => (me, other),
                        _ => (other, me),
                    };
                    found.push(Rule { first, then, kind });
                }
            };
            add(&entry.load_after, Kind::CommunityAfter);
            add(&entry.load_before, Kind::CommunityBefore);
            if entry.bottom {
                let above = (0..mods.len()).filter(|&other| !bottom[other]);
                found.extend(above.map(|other| Rule {
                    first: other,
                    then: me,
                    kind: Kind::CommunityBottom,
                }));
            }
        }
        found
    }
}

pub fn parse(text: &str) -> Result<Community, Error> {
    let root: Value = serde_json::from_str(text)
        .map_err(|e| Error::format(say!("community rules are not JSON: {}", e)))?;
    let rules = root
        .get("rules")
        .and_then(Value::as_object)
        .ok_or_else(|| Error::format(say!("community rules have no \"rules\" list")))?;

    let ids = |entry: &Value, key: &str| -> Vec<String> {
        let targets = entry.get(key).and_then(Value::as_object);
        targets.map_or(Vec::new(), |t| t.keys().cloned().collect())
    };
    // Written as `true` or as `{"value": true}`.
    let flag = |entry: &Value, key: &str| -> bool {
        let value = entry.get(key);
        let inner = value.and_then(|v| v.get("value")).or(value);
        inner.and_then(Value::as_bool).unwrap_or(false)
    };
    let entries = rules
        .iter()
        .map(|(package_id, entry)| {
            let entry = Entry {
                load_after: ids(entry, "loadAfter"),
                load_before: ids(entry, "loadBefore"),
                bottom: flag(entry, "loadBottom"),
                top: flag(entry, "loadTop"),
            };
            (id::fold(package_id), entry)
        })
        .collect();
    Ok(Community {
        timestamp: root.get("timestamp").and_then(Value::as_i64),
        entries,
    })
}

/// The "No Version Warning" list: mods known to work on a game version
/// they do not list. One file per game version, from the mod of that name
/// by Mlie (github.com/emipa606/NoVersionWarning).
pub fn works_on_url(game_version: &str) -> String {
    format!("https://raw.githubusercontent.com/emipa606/NoVersionWarning/main/{game_version}/ModIdsToFix.xml")
}

pub fn works_on_file(data_dir: &Path, game_version: &str) -> PathBuf {
    data_dir.join(format!("worksOn-{game_version}.xml"))
}

/// The folded package ids in the text of a `ModIdsToFix.xml`.
pub fn works_on(xml: &str) -> Result<std::collections::HashSet<String>, Error> {
    let xml = xml.strip_prefix('\u{feff}').unwrap_or(xml);
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| Error::format(say!("the list is not XML: {}", e)))?;
    if !doc.root_element().has_tag_name("ModIdsToFix") {
        return Err(Error::format(say!("the list is not a ModIdsToFix file")));
    }
    let items = doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("li"));
    Ok(items
        .filter_map(|n| n.text())
        .map(id::fold)
        .filter(|s| !s.is_empty())
        .collect())
}

/// Reads the kept list; none kept, or one that cannot be read, is empty.
pub fn read_works_on(path: &Path) -> std::collections::HashSet<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    works_on(&text).unwrap_or_default()
}

/// Downloads the list for a game version and keeps it at `to`. A file
/// that does not parse never replaces a good one.
pub fn download_works_on(
    game_version: &str,
    to: &Path,
) -> Result<std::collections::HashSet<String>, Error> {
    let agent = crate::web::agent(30);
    // No list is published for a game version until someone starts one:
    // that is an empty list, not a failure.
    let text = match agent.get(works_on_url(game_version)).call() {
        Ok(mut response) => response
            .body_mut()
            .read_to_string()
            .map_err(crate::web::failed)?,
        Err(ureq::Error::StatusCode(404)) => "<ModIdsToFix></ModIdsToFix>".to_string(),
        Err(problem) => return Err(crate::web::failed(problem)),
    };
    let parsed = works_on(&text)?;
    crate::whole::write(to, &text)?;
    Ok(parsed)
}

pub fn file_in(data_dir: &Path) -> PathBuf {
    data_dir.join("communityRules.json")
}

/// Reads the kept file. `Ok(None)` when it was never downloaded.
pub fn read(path: &Path) -> Result<Option<Community>, Error> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::file(path, e)),
    }
}

/// Checks `text` and keeps it at `to`. A file that does not parse never
/// replaces a good one, and neither does half a file.
pub fn store(text: &str, to: &Path) -> Result<Community, Error> {
    let parsed = parse(text)?;
    crate::whole::write(to, text)?;
    Ok(parsed)
}

pub fn download(to: &Path) -> Result<Community, Error> {
    let agent = crate::web::agent(30);
    let mut response = agent.get(URL).call().map_err(crate::web::failed)?;
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(crate::web::failed)?;
    store(&text, to)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "timestamp": 1789239946,
        "rules": {
            "Author.Patch": {
                "loadAfter": {"big.mod": {"name": ["Big Mod"]}, "not.installed": {"name": ["x"]}},
                "loadBefore": {"late.mod_steam": {"name": ["Late"], "comment": "why"}}
            },
            "krkr.rocketman": {"loadBottom": {"value": true, "comment": "last"}},
            "old.style": {"loadBottom": true},
            "top.mod": {"loadTop": {"value": true}}
        }
    }"#;

    fn about(id: &str) -> About {
        About {
            package_id: id.to_string(),
            ..About::default()
        }
    }

    fn rules_for(ids: &[&str]) -> Vec<Rule> {
        let abouts: Vec<About> = ids.iter().map(|id| about(id)).collect();
        let refs: Vec<Option<&About>> = abouts.iter().map(Some).collect();
        parse(SAMPLE).unwrap().rules(&refs)
    }

    #[test]
    fn parses_the_timestamp_and_counts_the_mods() {
        let community = parse(SAMPLE).unwrap();
        assert_eq!(community.timestamp, Some(1789239946));
        assert_eq!(community.len(), 4);
    }

    #[test]
    fn load_after_and_before_become_rules_between_listed_mods() {
        let found = rules_for(&["author.patch", "Big.Mod", "late.mod"]);
        assert_eq!(
            found,
            vec![
                Rule {
                    first: 1,
                    then: 0,
                    kind: Kind::CommunityAfter
                },
                Rule {
                    first: 0,
                    then: 2,
                    kind: Kind::CommunityBefore
                },
            ]
        );
        assert_eq!(found[0].declared_by(), None);
    }

    #[test]
    fn a_bottom_mod_follows_every_mod_that_is_not_one() {
        let found = rules_for(&["krkr.rocketman", "a.a", "old.style", "b.b"]);
        let pairs: Vec<(usize, usize)> = found.iter().map(|r| (r.first, r.then)).collect();
        assert_eq!(pairs, vec![(1, 0), (3, 0), (1, 2), (3, 2)]);
        assert!(found.iter().all(|r| r.kind == Kind::CommunityBottom));
    }

    #[test]
    fn every_pair_in_the_file_can_be_listed() {
        let pairs = parse(SAMPLE).unwrap().pairs();
        let pair = |a: &str, b: &str| (a.to_string(), b.to_string());
        assert_eq!(
            pairs,
            [
                pair("author.patch", "late.mod_steam"),
                pair("big.mod", "author.patch"),
                pair("not.installed", "author.patch"),
            ]
        );
    }

    #[test]
    fn load_top_marks_the_mod_and_makes_no_rule() {
        assert!(rules_for(&["a.a", "top.mod"]).is_empty());
        let community = parse(SAMPLE).unwrap();
        assert!(community.loads_at_top("Top.Mod"));
        assert!(!community.loads_at_top("krkr.rocketman"));
        assert!(!community.loads_at_top("unknown.mod"));
    }

    #[test]
    fn text_that_is_not_the_rules_file_is_refused() {
        let said = |text| parse(text).unwrap_err().to_string();
        assert!(said("<html>").contains("not JSON"));
        assert!(said("{}").contains("no \"rules\""));
    }

    #[test]
    fn a_file_never_downloaded_reads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read(&file_in(dir.path())), Ok(None));
    }

    #[test]
    fn storing_keeps_the_file_and_reads_back_the_same() {
        let dir = tempfile::tempdir().unwrap();
        let path = file_in(&dir.path().join("RimStack"));
        let stored = store(SAMPLE, &path).unwrap();
        assert_eq!(read(&path), Ok(Some(stored)));
    }

    #[test]
    fn a_bad_download_does_not_replace_a_good_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = file_in(dir.path());
        store(SAMPLE, &path).unwrap();
        assert!(store("404: Not Found", &path).is_err());
        assert_eq!(read(&path).unwrap().unwrap().len(), 4);
    }

    #[test]
    fn the_works_on_list_gives_folded_ids_and_refuses_other_files() {
        let xml = "\u{feff}<?xml version=\"1.0\"?>\n<ModIdsToFix>\n  <!-- A mod -->\n  <li>Baby.Muffalump</li>\n  <li> zilla.fertility </li>\n  <li></li>\n</ModIdsToFix>";
        let found = works_on(xml).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.contains("baby.muffalump") && found.contains("zilla.fertility"));
        assert!(works_on("404: Not Found").is_err());
        assert!(works_on("<other><li>a.b</li></other>").is_err());
        let dir = tempfile::tempdir().unwrap();
        assert!(read_works_on(&works_on_file(dir.path(), "1.6")).is_empty());
    }

    #[test]
    #[ignore = "needs the network"]
    fn the_real_works_on_list_downloads() {
        let dir = tempfile::tempdir().unwrap();
        let path = works_on_file(dir.path(), "1.6");
        let found = download_works_on("1.6", &path).unwrap();
        println!("{} mods", found.len());
        assert!(found.len() > 50);
        assert_eq!(read_works_on(&path), found);
    }

    #[test]
    #[ignore = "needs the network"]
    fn the_real_file_downloads_and_parses() {
        let dir = tempfile::tempdir().unwrap();
        let community = download(&file_in(dir.path())).unwrap();
        println!(
            "{} mods, timestamp {:?}",
            community.len(),
            community.timestamp
        );
        assert!(community.len() > 100);
    }
}
