//! The languages the app ships with, held to what a wording must keep
//! of its English. Their sense is not checked here: five of them were
//! translated by machine.

use rimstack_core::base::translate::{misfit, PLURAL_RULE};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn read(file: &Path) -> Map<String, Value> {
    let text = std::fs::read_to_string(file).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn every_language_has_every_text_and_each_wording_fits_its_english() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/assets");
    let english = read(&assets.join("words.json"));
    let mut faults = Vec::new();
    let mut languages = 0;
    for entry in std::fs::read_dir(assets.join("languages")).unwrap() {
        let file = entry.unwrap().path();
        let name = file.file_stem().unwrap().to_string_lossy().into_owned();
        let words = read(&file);
        languages += 1;
        let rule = words.get(PLURAL_RULE).and_then(Value::as_str);
        let third = rule.is_some_and(|rule| rule.contains('/'));
        if rule.is_none_or(|rule| rule.trim().is_empty()) {
            faults.push(format!("{name}: no {PLURAL_RULE}"));
        }
        for text in english.keys().filter(|text| !words.contains_key(*text)) {
            faults.push(format!("{name}: lacks {text:?}"));
        }
        for (text, said) in words.iter().filter(|(text, _)| *text != PLURAL_RULE) {
            let said = said.as_str().unwrap_or_default();
            if !english.contains_key(text) {
                faults.push(format!("{name}: {text:?} is no longer a text"));
            } else if said.is_empty() {
                faults.push(format!("{name}: {text:?} has no wording"));
            } else if let Some(why) = misfit(text, said, third) {
                faults.push(format!("{name}: {text:?} as {said:?}: {why:?}"));
            }
        }
    }
    assert!(languages >= 7, "only {languages} language files were found");
    assert!(faults.is_empty(), "{}", faults.join("\n"));
}

fn files_under(dir: &Path, extension: &str, into: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files_under(&path, extension, into);
        } else if path.extension().is_some_and(|x| x == extension) {
            into.push(path);
        }
    }
}

/// A count before a plural noun through `Theme.fill` or `say!` reads
/// "Tagged 1 mods", and never takes a language's own plural forms.
#[test]
fn every_counted_sentence_has_its_singular() {
    const NOUNS: [&str; 17] = [
        "mods",
        "files",
        "textures",
        "lines",
        "minutes",
        "hours",
        "days",
        "months",
        "years",
        "seconds",
        "rules",
        "errors",
        "saves",
        "problems",
        "warnings",
        "conflicts",
        "commits",
    ];
    let app = Path::new(env!("CARGO_MANIFEST_DIR")).join("../app");
    let mut sources = Vec::new();
    files_under(&app.join("qml"), "qml", &mut sources);
    files_under(&app.join("src"), "rs", &mut sources);
    files_under(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        "rs",
        &mut sources,
    );
    let mut found = Vec::new();
    for file in sources {
        let text = fs::read_to_string(&file).unwrap();
        // Rust texts below the tests are not shown (tools/words.py stops there too).
        let shown = text.split("#[cfg(test)]").next().unwrap_or_default();
        for (n, line) in shown.lines().enumerate() {
            if counts_without_singular(line, &NOUNS) {
                let short = file
                    .strip_prefix(&app)
                    .unwrap_or(&file)
                    .display()
                    .to_string();
                found.push(format!("{short}:{}", n + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "{} counted sentences without a singular:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// `fill("…{} mods…"` or `say!("…{} mods…"`: a literal that puts a count
/// before one of `nouns`.
fn counts_without_singular(line: &str, nouns: &[&str]) -> bool {
    ["fill(\"", "say!(\""].iter().any(|opener| {
        line.match_indices(opener).any(|(at, _)| {
            let literal = line[at + opener.len()..]
                .split('"')
                .next()
                .unwrap_or_default();
            nouns.iter().any(|noun| {
                let counted = format!("{{}} {noun}");
                literal.match_indices(&counted).any(|(i, _)| {
                    let after = &literal[i + counted.len()..];
                    !after.starts_with(|c: char| c.is_alphanumeric())
                })
            })
        })
    })
}
