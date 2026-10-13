//! RimSort's Steam Workshop Database: every Workshop mod's package id
//! and name, to name a mod that is required and not installed.
//!
//! The database is 49 MB of JSON. It is read once when downloaded and
//! kept as a small file of `package id<tab>Workshop id<tab>name` lines.

use crate::say;

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

const URL: &str =
    "https://raw.githubusercontent.com/RimSort/Steam-Workshop-Database/main/steamDB.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub workshop_id: String,
    pub name: String,
}

/// Workshop mods by folded package id.
pub type Names = HashMap<String, Item>;

#[derive(Deserialize)]
struct Database {
    database: HashMap<String, Entry>,
}

#[derive(Deserialize)]
struct Entry {
    #[serde(rename = "packageId")]
    package_id: Option<String>,
    name: Option<String>,
    #[serde(rename = "steamName")]
    steam_name: Option<String>,
}

pub fn parse(json: impl std::io::Read) -> Result<Names, String> {
    let all: Database = serde_json::from_reader(json)
        .map_err(|e| say!("the database is not as expected: {}", e))?;
    let item = |(workshop_id, entry): (String, Entry)| {
        let name = entry.steam_name.or(entry.name)?;
        Some((
            crate::id::fold(&entry.package_id?),
            Item { workshop_id, name },
        ))
    };
    Ok(all.database.into_iter().filter_map(item).collect())
}

pub fn file_in(data_dir: &Path) -> PathBuf {
    data_dir.join("workshopNames.tsv")
}

pub fn store(names: &Names, to: &Path) -> Result<(), String> {
    let mut lines: Vec<String> = names
        .iter()
        .map(|(id, item)| {
            format!(
                "{id}\t{}\t{}",
                item.workshop_id,
                item.name.replace(['\t', '\n'], " ")
            )
        })
        .collect();
    lines.sort();
    std::fs::write(to, lines.join("\n")).map_err(|e| format!("{}: {e}", to.display()))
}

/// No file is no names.
pub fn read(path: &Path) -> Names {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let item = |line: &str| {
        let mut parts = line.splitn(3, '\t');
        let (id, workshop_id, name) = (parts.next()?, parts.next()?, parts.next()?);
        Some((
            id.to_string(),
            Item {
                workshop_id: workshop_id.into(),
                name: name.into(),
            },
        ))
    };
    text.lines().filter_map(item).collect()
}

pub fn download(to: &Path) -> Result<Names, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .into();
    let failed = |e: ureq::Error| say!("the download failed: {}", e);
    let mut response = agent.get(URL).call().map_err(failed)?;
    let names = parse(response.body_mut().with_config().limit(200 << 20).reader())?;
    store(&names, to)?;
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"version": 1, "database": {
        "111": {"url": "u", "steamName": "Big Mod (Continued)", "name": "Big Mod", "packageId": "Author.BigMod", "dependencies": {"222": ["x", "y"]}},
        "222": {"name": "Only In About", "packageId": "a.b"},
        "333": {"steamName": "No id"},
        "294100": {"appid": true, "packageid": "ludeon.rimworld", "name": "RimWorld", "packageId": "ludeon.rimworld"}
    }}"#;

    #[test]
    fn mods_are_indexed_by_package_id_under_their_workshop_name() {
        let names = parse(SAMPLE.as_bytes()).unwrap();
        assert_eq!(names.len(), 3);
        let big = &names["author.bigmod"];
        assert_eq!(
            (big.workshop_id.as_str(), big.name.as_str()),
            ("111", "Big Mod (Continued)")
        );
        assert_eq!(names["a.b"].name, "Only In About");
        assert!(parse("[]".as_bytes()).is_err());
    }

    #[test]
    fn the_names_read_back_as_stored() {
        let dir = tempfile::tempdir().unwrap();
        let path = file_in(dir.path());
        assert!(read(&path).is_empty());
        let mut names = parse(SAMPLE.as_bytes()).unwrap();
        names.get_mut("a.b").unwrap().name = "Tab\there".into();
        store(&names, &path).unwrap();
        names.get_mut("a.b").unwrap().name = "Tab here".into();
        assert_eq!(read(&path), names);
    }

    #[test]
    #[ignore = "needs RIMSTACK_STEAMDB: a downloaded steamDB.json"]
    fn the_real_database_is_read() {
        let file = std::fs::File::open(std::env::var("RIMSTACK_STEAMDB").unwrap()).unwrap();
        let started = std::time::Instant::now();
        let names = parse(std::io::BufReader::new(file)).unwrap();
        println!("{} names in {:?}", names.len(), started.elapsed());
        assert!(names.len() > 10_000);
    }
}
