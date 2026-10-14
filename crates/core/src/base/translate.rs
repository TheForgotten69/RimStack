//! What languages have in common: which counts take the singular, and
//! whether a language file's wording can stand where its English does.
//! The words themselves are given by whoever shows a sentence: see
//! `sentence::Language`.

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

pub(crate) fn form_for<'a>(rule: &str, n: usize, one: &'a str, many: &'a str) -> &'a str {
    if takes_singular(rule, n) {
        return one;
    }
    match many.split_once('|') {
        Some((few, _)) if takes_few(rule, n) => few,
        Some((_, many)) => many,
        None => many,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn a_count_takes_the_form_its_language_gives_it() {
        let said = |n| form_for("1 / 2 3 4", n, "mod", "mody|modów");
        assert_eq!(
            [said(1), said(3), said(5), said(12), said(22)],
            ["mod", "mody", "modów", "modów", "mody"]
        );
        assert_eq!(form_for("1", 2, "mod", "mods"), "mods");
        assert!(takes_singular("1", 1) && !takes_singular("1", 0) && !takes_singular("1", 2));
        assert!(takes_singular("0 1", 0) && takes_singular("0 1", 1) && !takes_singular("0 1", 11));
    }
}
