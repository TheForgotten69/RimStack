//! Your own tags, a note, and whether you trust it on this game version,
//! per mod: kept by package id so they follow the mod wherever it is
//! installed.
//!
//! A mod's first tag is also its group: inside a category, mods with
//! the same first tag are kept side by side under that tag's name.

use crate::error::Error;
use crate::id;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Note {
    pub tags: Vec<String>,
    pub text: String,
    /// You said it works on your game version though it does not list it.
    pub version_ok: bool,
    /// The category you filed it under, by `Category::key`; empty to leave
    /// that to RimStack.
    pub category: String,
}

impl Note {
    /// The tag the mod is grouped under inside its category: its first.
    pub fn group(&self) -> &str {
        self.tags.first().map_or("", String::as_str)
    }

    /// Groups the mod under `tag`: it becomes the first tag, added if it
    /// was not one. An empty name takes the mod out of its group by
    /// removing that tag.
    pub fn set_group(&mut self, tag: &str) {
        let tag = tag.trim();
        if tag.is_empty() {
            if !self.tags.is_empty() {
                self.tags.remove(0);
            }
            return;
        }
        self.tags.retain(|t| !t.eq_ignore_ascii_case(tag));
        self.tags.insert(0, tag.to_string());
    }

    /// Takes `tag` off, in whatever case it was typed.
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| !t.eq_ignore_ascii_case(tag.trim()));
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Notes {
    by_id: BTreeMap<String, Note>,
}

/// Tags as typed in one box: separated by commas, trimmed, each once.
pub fn tags_in(text: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for tag in text.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        if !tags.iter().any(|seen| seen.eq_ignore_ascii_case(tag)) {
            tags.push(tag.to_string());
        }
    }
    tags
}

impl Notes {
    /// No file, or a damaged one, is no notes.
    pub fn read(path: &Path) -> Self {
        let Some(Value::Object(entries)) = crate::whole::read_json(path) else {
            return Self::default();
        };
        let note = |v: &Value| -> Note {
            let tags = v.get("tags").and_then(Value::as_array);
            let tag = |t: &Value| t.as_str().map(String::from);
            let mut read = Note {
                tags: tags.map_or(Vec::new(), |t| t.iter().filter_map(tag).collect()),
                text: v
                    .get("note")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                version_ok: v.get("versionOk").and_then(Value::as_bool).unwrap_or(false),
                category: v
                    .get("category")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            };
            // A sub-category of earlier versions is a tag now, and the first.
            if let Some(sub) = v.get("sub").and_then(Value::as_str) {
                read.set_group(sub);
            }
            read
        };
        Self {
            by_id: entries
                .iter()
                .map(|(k, v)| (id::fold(k), note(v)))
                .collect(),
        }
    }

    pub fn write(&self, path: &Path) -> Result<(), Error> {
        let entries: serde_json::Map<String, Value> = self
            .by_id
            .iter()
            .map(|(k, n)| {
                let mut entry = json!({ "tags": n.tags, "note": n.text });
                if n.version_ok {
                    entry["versionOk"] = json!(true);
                }
                if !n.category.is_empty() {
                    entry["category"] = json!(n.category);
                }
                (k.clone(), entry)
            })
            .collect();
        let text = serde_json::to_string_pretty(&Value::Object(entries))
            .map_err(|e| Error::format(format!("{}: {e}", path.display())))?;
        crate::whole::write(path, text)
    }

    pub fn get(&self, package_id: &str) -> Option<&Note> {
        self.by_id.get(&id::fold(package_id))
    }

    /// Replaces what is noted about a mod. A note that says nothing is
    /// forgotten.
    pub fn set(&mut self, package_id: &str, note: Note) {
        let key = id::fold(package_id);
        let says_nothing = note.tags.is_empty() && note.text.trim().is_empty();
        if says_nothing && !note.version_ok && note.category.is_empty() {
            self.by_id.remove(&key);
        } else {
            self.by_id.insert(key, note);
        }
    }

    /// The mods you said work on your game version, by folded package id.
    pub fn version_ok(&self) -> impl Iterator<Item = &str> {
        let trusted = self.by_id.iter().filter(|(_, n)| n.version_ok);
        trusted.map(|(k, _)| k.as_str())
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_split_on_commas_trimmed_and_kept_once() {
        assert_eq!(
            tags_in(" medieval, QoL ,, qol,keep "),
            ["medieval", "QoL", "keep"]
        );
        assert!(tags_in(" , ").is_empty());
    }

    #[test]
    fn a_note_is_kept_by_id_in_any_case_and_an_empty_one_forgotten() {
        let mut notes = Notes::default();
        let note = Note {
            tags: tags_in("keep"),
            text: "breaks on 1.5".into(),
            ..Note::default()
        };
        notes.set("Author.Mod", note.clone());
        assert_eq!(notes.get("author.mod"), Some(&note));
        notes.set("AUTHOR.MOD", Note::default());
        assert!(notes.is_empty());
    }

    #[test]
    fn notes_read_back_as_written_and_damage_reads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data").join("notes.json");
        let mut notes = Notes::default();
        notes.set(
            "a.a",
            Note {
                tags: tags_in("x, y"),
                text: "hello\nthere".into(),
                category: "Interface".into(),
                ..Note::default()
            },
        );
        let trusted = Note {
            version_ok: true,
            ..Note::default()
        };
        notes.set("B.B", trusted);
        assert_eq!(notes.version_ok().collect::<Vec<_>>(), ["b.b"]);
        notes.write(&path).unwrap();
        assert_eq!(Notes::read(&path), notes);
        std::fs::write(&path, "nope").unwrap();
        assert!(Notes::read(&path).is_empty());
    }

    #[test]
    fn the_first_tag_is_the_group_and_grouping_moves_a_tag_first() {
        let mut note = Note {
            tags: tags_in("keep, Genes"),
            ..Note::default()
        };
        assert_eq!(note.group(), "keep");
        note.set_group("genes");
        assert_eq!(note.tags, ["genes", "keep"]);
        note.set_group("Animals");
        assert_eq!(note.tags, ["Animals", "genes", "keep"]);
        note.set_group(" ");
        assert_eq!(note.group(), "genes");
        note.remove_tag("KEEP");
        assert_eq!(note.tags, ["genes"]);
        assert_eq!(Note::default().group(), "");
    }

    #[test]
    fn a_sub_category_of_an_earlier_version_is_read_as_the_first_tag() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        let old = r#"{ "a.a": { "tags": ["keep"], "note": "", "sub": "Genes" }, "b.b": { "tags": [], "note": "", "sub": "" } }"#;
        std::fs::write(&path, old).unwrap();
        let notes = Notes::read(&path);
        assert_eq!(notes.get("a.a").unwrap().tags, ["Genes", "keep"]);
        assert!(notes.get("b.b").unwrap().tags.is_empty());
        notes.write(&path).unwrap();
        assert!(!std::fs::read_to_string(&path).unwrap().contains("\"sub\""));
    }
}
