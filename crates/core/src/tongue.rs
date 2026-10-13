//! The language of the sentences written here. They are written in
//! English; the interface gives the chosen language's table once, and
//! `say!` then answers in it. A text the table lacks stays in English.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::RwLock;

static WORDS: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

/// From English to the chosen language; an empty table is English.
pub fn set(words: HashMap<String, String>) {
    *WORDS.write().unwrap_or_else(|e| e.into_inner()) = Some(words);
}

pub fn say(english: &str) -> String {
    let words = WORDS.read().unwrap_or_else(|e| e.into_inner());
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
        let words = WORDS.read().unwrap_or_else(|e| e.into_inner());
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

/// Marks a text that is kept in English here, as a key, and put in the
/// chosen language where it is shown.
pub const fn named(english: &'static str) -> &'static str {
    english
}

#[macro_export]
macro_rules! say {
    ($text:literal) => { $crate::tongue::say($text) };
    ($text:literal, $($part:expr),+ $(,)?) => {
        $crate::tongue::fill($text, &[$(&$part as &dyn std::fmt::Display),+])
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

/// `keep!("Updated {}: {}", name, commit)`: see `kept`.
#[macro_export]
macro_rules! keep {
    ($text:literal $(, $part:expr)* $(,)?) => {
        $crate::tongue::kept($text, &[$(serde_json::Value::from($part.to_string())),*])
    };
}

/// `say_n!(3, "{} mod", "{} mods")`: the count and the form that goes with it.
#[macro_export]
macro_rules! say_n {
    ($n:expr, $one:literal, $many:literal $(, $part:expr)* $(,)?) => {
        $crate::tongue::several($n, $one, $many, &[$(&$part as &dyn std::fmt::Display),*])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

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
