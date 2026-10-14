//! The languages the app ships with, held to what a wording must keep
//! of its English. Their sense is not checked here: five of them were
//! translated by machine.

use rimstack_core::tongue::{misfit, PLURAL_RULE};
use serde_json::{Map, Value};
use std::path::Path;

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
