//! The language of the sentences written here. They are written in
//! English; the interface gives the chosen language's table once, and
//! `say!` then answers in it. A text the table lacks stays in English.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::RwLock;

use crate::base::locks::Shared;

static WORDS: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

/// From English to the chosen language; an empty table is English.
pub fn set(words: HashMap<String, String>) {
    *WORDS.writing() = Some(words);
}

/// Held by a test that changes the table, which the whole process shares.
#[cfg(test)]
pub(crate) static TABLE_IN_USE: std::sync::Mutex<()> = std::sync::Mutex::new(());

thread_local! {
    static IN_ENGLISH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `put` with every sentence said in English on this thread: what
/// is written down for whoever reads the diagnostics later.
pub fn in_english<R>(put: impl FnOnce() -> R) -> R {
    let before = IN_ENGLISH.with(|english| english.replace(true));
    let said = put();
    IN_ENGLISH.with(|english| english.set(before));
    said
}

pub fn say(english: &str) -> String {
    if IN_ENGLISH.with(std::cell::Cell::get) {
        return english.to_string();
    }
    let words = WORDS.reading();
    let said = words.as_ref().and_then(|w| w.get(english));
    said.filter(|s| !s.is_empty())
        .map_or(english, String::as_str)
        .to_string()
}

/// `say`, with each `{}` of the text replaced by the next part: the
/// places can stand in another order in another language only by number
/// of appearance, so a sentence keeps its parts in the English order.
pub fn fill(english: &str, parts: &[&dyn Display]) -> String {
    let said = say(english);
    let mut pieces = said.split("{}");
    let mut out = pieces.next().unwrap_or_default().to_string();
    for (piece, part) in pieces.zip(parts) {
        out.push_str(&part.to_string());
        out.push_str(piece);
    }
    out
}

/// The key under which a language says which counts take the singular,
/// separated by spaces: "1" in English, "0 1" in French. After a slash,
/// the last digits that take a third form, as Polish has for 2, 3 and 4
/// outside the teens: "1 / 2 3 4". Such a language writes that form
/// before the plural's, with a bar between: "{} mody|{} modów".
pub const PLURAL_RULE: &str = "@plural";

fn takes_singular(rule: &str, n: usize) -> bool {
    let singular = rule.split('/').next().unwrap_or_default();
    singular.split_whitespace().any(|one| one.parse() == Ok(n))
}

fn takes_few(rule: &str, n: usize) -> bool {
    let few = rule.split('/').nth(1).unwrap_or_default();
    let teen = (11..=19).contains(&(n % 100));
    !teen
        && few
            .split_whitespace()
            .any(|digit| digit.parse() == Ok(n % 10))
}

fn form_for<'a>(rule: &str, n: usize, one: &'a str, many: &'a str) -> &'a str {
    if takes_singular(rule, n) {
        return one;
    }
    match many.split_once('|') {
        Some((few, _)) if takes_few(rule, n) => few,
        Some((_, many)) => many,
        None => many,
    }
}

/// `fill` with the count first, in the form the language uses for it.
pub fn several(n: usize, one: &str, many: &str, parts: &[&dyn Display]) -> String {
    let rule = {
        let words = WORDS.reading();
        let said = words.as_ref().and_then(|w| w.get(PLURAL_RULE));
        said.filter(|s| !s.is_empty()).cloned()
    };
    let (one, many) = (say(one), say(many));
    let form = form_for(rule.as_deref().unwrap_or("1"), n, &one, &many);
    let mut pieces = form.split("{}");
    let mut out = pieces.next().unwrap_or_default().to_string();
    let all = std::iter::once(&n as &dyn Display).chain(parts.iter().copied());
    for (piece, part) in pieces.zip(all) {
        out.push_str(&part.to_string());
        out.push_str(piece);
    }
    out
}

/// Why a wording cannot stand where its English does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Misfit {
    /// Not as many `{}` as the English: a value would be dropped, or a
    /// place left bare.
    Places,
    /// A bar between forms in a language whose `@plural` names no third form.
    Forms,
    /// A space at either end, or a final `…`, that only one of the two has:
    /// such a text is joined to what stands beside it.
    Edges,
}

/// `third_form` is whether the language's `@plural` has a part after a slash.
pub fn misfit(english: &str, said: &str, third_form: bool) -> Option<Misfit> {
    let places = |text: &str| text.matches("{}").count();
    let forms: Vec<&str> = said.split('|').collect();
    if forms.len() > 1 && !third_form {
        return Some(Misfit::Forms);
    }
    if forms.iter().any(|form| places(form) != places(english)) {
        return Some(Misfit::Places);
    }
    let edges = |text: &str| {
        (
            text.starts_with(' '),
            text.ends_with(' '),
            text.ends_with('…'),
        )
    };
    forms
        .iter()
        .any(|form| edges(form) != edges(english))
        .then_some(Misfit::Edges)
}

fn third_form(rule: Option<&serde_json::Value>) -> bool {
    rule.and_then(|rule| rule.as_str())
        .is_some_and(|rule| rule.contains('/'))
}

/// A language's texts, from English to it: those it ships with (the text
/// of its file), under those of the user's own file of the same name.
/// Of the user's, an empty wording and one that does not fit its English
/// leave the shipped one standing.
pub fn table(shipped: &str, own: &str) -> serde_json::Map<String, serde_json::Value> {
    let read = |text: &str| match serde_json::from_str(text) {
        Ok(serde_json::Value::Object(words)) => words,
        _ => Default::default(),
    };
    let (mut words, own) = (read(shipped), read(own));
    let third = third_form(own.get(PLURAL_RULE).or(words.get(PLURAL_RULE)));
    for (english, said) in own {
        let Some(wording) = said.as_str().filter(|wording| !wording.is_empty()) else {
            continue;
        };
        let why = misfit(&english, wording, third).filter(|_| english != PLURAL_RULE);
        match why {
            // Said in the record: on screen the shipped wording just stays.
            Some(why) => {
                crate::problem!("your wording {wording:?} for {english:?} is not used: {why:?}")
            }
            None => drop(words.insert(english, said)),
        }
    }
    words
}

/// Marks a text that is kept in English here, as a key, and put in the
/// chosen language where it is shown.
pub const fn named(english: &'static str) -> &'static str {
    english
}

#[macro_export]
macro_rules! say {
    ($text:literal) => { $crate::base::translate::say($text) };
    ($text:literal, $($part:expr),+ $(,)?) => {
        $crate::base::translate::fill($text, &[$(&$part as &dyn std::fmt::Display),+])
    };
}

/// A sentence kept for later, as JSON: its English and its parts, so that
/// it is put in words in the language chosen when it is read, not the one
/// it was written in. `["Updated {}: {}", "Harmony", "35a8963"]`; a part
/// may be such a sentence itself, and `[",", a, b]` is a list of them.
pub fn kept(english: &str, parts: &[serde_json::Value]) -> String {
    let mut all = vec![serde_json::Value::from(english)];
    all.extend_from_slice(parts);
    serde_json::Value::Array(all).to_string()
}

/// A sentence `kept` wrote, in the chosen language. `["#", n, one,
/// many, part…]` is one with a count.
pub fn put(sentence: &serde_json::Value) -> String {
    use serde_json::Value;
    let all = match sentence {
        Value::Array(all) => all,
        Value::String(text) => return text.clone(),
        other => return other.to_string(),
    };
    let Some(Value::String(head)) = all.first() else {
        return sentence.to_string();
    };
    let parts: Vec<String> = all[1..].iter().map(put).collect();
    let shown = |from: usize| -> Vec<&dyn Display> {
        parts.iter().skip(from).map(|p| p as &dyn Display).collect()
    };
    match head.as_str() {
        "," => parts.join(", "),
        "#" if parts.len() >= 3 => {
            let n = parts[0].parse().unwrap_or(0);
            several(n, &parts[1], &parts[2], &shown(3))
        }
        english => fill(english, &shown(0)),
    }
}

/// What a record holds, in words: a sentence `kept` wrote, or the plain
/// English an older record holds.
pub fn in_words(record: &str) -> String {
    if !record.starts_with('[') {
        return say(record);
    }
    serde_json::from_str(record).map_or_else(|_| record.to_string(), |sentence| put(&sentence))
}

/// `keep!("Updated {}: {}", name, commit)`: see `kept`.
#[macro_export]
macro_rules! keep {
    ($text:literal $(, $part:expr)* $(,)?) => {
        $crate::base::translate::kept($text, &[$(serde_json::Value::from($part.to_string())),*])
    };
}

/// `say_n!(3, "{} mod", "{} mods")`: the count and the form that goes with it.
#[macro_export]
macro_rules! say_n {
    ($n:expr, $one:literal, $many:literal $(, $part:expr)* $(,)?) => {
        $crate::base::translate::several($n, $one, $many, &[$(&$part as &dyn std::fmt::Display),*])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_english_a_sentence_is_said_in_english_whatever_the_language() {
        let _alone = TABLE_IN_USE.lock();
        set(HashMap::from([(
            "Kept for the test {}".to_string(),
            "Gardé {}".to_string(),
        )]));
        assert_eq!(fill("Kept for the test {}", &[&1]), "Gardé 1");
        assert_eq!(
            in_english(|| fill("Kept for the test {}", &[&1])),
            "Kept for the test 1"
        );
        assert_eq!(fill("Kept for the test {}", &[&2]), "Gardé 2");
        set(HashMap::new());
    }

    #[test]
    fn a_kept_sentence_is_put_in_words_with_its_parts_lists_and_counts() {
        let _alone = TABLE_IN_USE.lock();
        set(HashMap::from([
            (
                "Kept, updated {}: {}".to_string(),
                "Gardé, mis à jour {} : {}".to_string(),
            ),
            ("{} kept mods".to_string(), "{} mods gardés".to_string()),
        ]));
        let kept = kept(
            "Kept, updated {}: {}",
            &["Harmony".into(), "35a8963".into()],
        );
        assert_eq!(in_words(&kept), "Gardé, mis à jour Harmony : 35a8963");
        let counted = serde_json::json!(["#", "1", "{} kept mod", "{} kept mods"]);
        assert_eq!(put(&counted), "1 kept mod");
        let listed = serde_json::json!([",", "a", ["#", 2, "{} kept mod", "{} kept mods"]]);
        assert_eq!(put(&listed), "a, 2 mods gardés");
        assert_eq!(in_words("Not kept as a sentence"), "Not kept as a sentence");
        assert_eq!(in_words("[broken"), "[broken");
        set(HashMap::new());
    }

    #[test]
    fn a_sentence_keeps_its_parts_and_a_missing_text_stays_english() {
        assert_eq!(
            fill("Needs {}, which is {}.", &[&"Lib", &2]),
            "Needs Lib, which is 2."
        );
        assert_eq!(fill("{} {}", &[&"a{}", &"b"]), "a{} b");
        assert_eq!(say("Not in any table"), "Not in any table");
    }

    #[test]
    fn a_wording_fits_when_it_keeps_the_places_and_the_edges_of_its_english() {
        assert_eq!(misfit("Needs {}", "Requiert {}", false), None);
        assert_eq!(misfit("Needs {}", "Requis", false), Some(Misfit::Places));
        assert_eq!(
            misfit("{} of {}", "{} sur {} {}", false),
            Some(Misfit::Places)
        );
        assert_eq!(misfit("{} mods", "{} mody|{} modów", true), None);
        assert_eq!(
            misfit("{} mods", "{} mody|modów", true),
            Some(Misfit::Places)
        );
        assert_eq!(
            misfit("{} mods", "{} mody|{} modów", false),
            Some(Misfit::Forms)
        );
        assert_eq!(misfit("Reading…", "Lecture", false), Some(Misfit::Edges));
        assert_eq!(misfit(" and ", "et", false), Some(Misfit::Edges));
    }

    #[test]
    fn your_own_wording_stands_over_the_shipped_one_unless_it_does_not_fit() {
        let shipped = r#"{ "@plural": "0 1", "Needs {}": "Requiert {}", "Save": "Enregistrer" }"#;
        let own = r#"{ "Needs {}": "Il faut", "Save": "Sauver", "Play": "", "New": "Neuf" }"#;
        let words = table(shipped, own);
        assert_eq!(words["Needs {}"], "Requiert {}");
        assert_eq!(words["Save"], "Sauver");
        assert_eq!(words["New"], "Neuf");
        assert!(!words.contains_key("Play"));
        assert_eq!(table("not json", "[1]").len(), 0);
        // A third form is allowed once your own rule names one.
        let polish = table(
            r#"{ "{} mods": "x {}" }"#,
            r#"{ "@plural": "1 / 2 3 4", "{} mods": "{} mody|{} modów" }"#,
        );
        assert_eq!(polish["{} mods"], "{} mody|{} modów");
    }

    #[test]
    fn a_kept_sentence_holds_its_english_and_its_parts() {
        assert_eq!(
            keep!("Updated {}: {}", "Lib", 7),
            r#"["Updated {}: {}","Lib","7"]"#
        );
        assert_eq!(kept("Saved", &[]), r#"["Saved"]"#);
    }

    #[test]
    fn a_count_takes_the_form_its_language_gives_it() {
        let said = |n| form_for("1 / 2 3 4", n, "mod", "mody|modów");
        assert_eq!(
            [said(1), said(3), said(5), said(12), said(22)],
            ["mod", "mody", "modów", "modów", "mody"]
        );
        assert_eq!(form_for("1", 2, "mod", "mods"), "mods");
        assert!(takes_singular("1", 1) && !takes_singular("1", 0) && !takes_singular("1", 2));
        assert!(takes_singular("0 1", 0) && takes_singular("0 1", 1) && !takes_singular("0 1", 11));
        assert_eq!(
            several(1, "{} mod of {}", "{} mods of {}", &[&7]),
            "1 mod of 7"
        );
        assert_eq!(several(0, "{} mod", "{} mods", &[]), "0 mods");
    }
}
