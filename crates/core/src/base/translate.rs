//! What languages have in common: which form a count takes, which
//! language a system's settings ask for, and whether a language file's
//! wording can stand where its English does. The words themselves are
//! given by whoever shows a sentence: see `sentence::Language`.

use icu_plurals::{PluralCategory, PluralRules};

/// The key under which a language file names its locale ("fr", "pt-BR"):
/// its plural rules come from it, and the system's settings are matched
/// against it. A file without one counts as English does.
pub const LOCALE: &str = "@locale";

/// How a language picks a count's form, by the CLDR plural rules of its
/// locale. A counted text is given in two: the singular's, for the
/// counts CLDR calls "one", and the plural's, which holds a form for each
/// of the language's other categories, in CLDR's order, with a bar
/// between ("{} mody|{} modów" in Polish: few, then many). A category
/// without its own form takes the last one.
pub struct Counting {
    rules: PluralRules,
    /// The categories after "one", in CLDR's order.
    after_one: Vec<PluralCategory>,
}

impl Counting {
    /// The rules of `locale`; English's for none, or one that cannot be read.
    pub fn of(locale: &str) -> Self {
        let parsed = icu_locale_core::Locale::try_from_str(locale.trim());
        let rules = parsed
            .ok()
            .and_then(|locale| PluralRules::try_new_cardinal((&locale).into()).ok())
            .unwrap_or_else(english_rules);
        let after_one = rules
            .categories()
            .filter(|category| *category != PluralCategory::One)
            .collect();
        Self { rules, after_one }
    }

    /// The form `n` takes: 0 for the singular's text, then 1 and on for
    /// the forms of the plural's.
    pub fn form(&self, n: usize) -> usize {
        let category = self.rules.category_for(n);
        if category == PluralCategory::One {
            return 0;
        }
        let at = self.after_one.iter().position(|c| *c == category);
        at.map_or(self.after_one.len(), |at| at + 1)
    }

    /// How many forms the plural's text may hold.
    pub fn plural_forms(&self) -> usize {
        self.after_one.len()
    }

    /// The text `n` takes, of a counted text's two.
    pub fn pick<'a>(&self, n: usize, one: &'a str, many: &'a str) -> &'a str {
        match self.form(n) {
            0 => one,
            at => {
                let forms: Vec<&str> = many.split('|').collect();
                forms.get(at - 1).or(forms.last()).copied().unwrap_or(many)
            }
        }
    }
}

fn english_rules() -> PluralRules {
    let english = icu_locale_core::locale!("en");
    PluralRules::try_new_cardinal((&english).into()).expect("CLDR's English rules are built in")
}

/// The language to start in, of those `offered` as (name, locale): the
/// first of the system's `preferred` locales, most wanted first, that one
/// of them is in, or failing that whose language one of them speaks.
pub fn detected<'a>(preferred: &[String], offered: &[(&'a str, &str)]) -> Option<&'a str> {
    let tag = |locale: &str| {
        let locale = locale.split(['.', '@']).next().unwrap_or_default();
        locale.replace('_', "-").to_lowercase()
    };
    let language = |tag: &str| tag.split('-').next().unwrap_or_default().to_string();
    preferred.iter().map(|p| tag(p)).find_map(|wanted| {
        let same = |(_, locale): &&(&str, &str)| tag(locale) == wanted;
        let speaks = |(_, locale): &&(&str, &str)| {
            !locale.is_empty() && language(&tag(locale)) == language(&wanted)
        };
        let found = offered.iter().find(same).or(offered.iter().find(speaks));
        found.map(|(name, _)| *name)
    })
}

/// Why a wording cannot stand where its English does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Misfit {
    /// Not as many `{}` as the English: a value would be dropped, or a
    /// place left bare.
    Places,
    /// More forms, between bars, than the language's plural rules have.
    Forms,
    /// A space at either end, or a final `…`, that only one of the two has:
    /// such a text is joined to what stands beside it.
    Edges,
}

/// `plural_forms` is how many forms the language's plural text may hold:
/// `Counting::plural_forms`.
pub fn misfit(english: &str, said: &str, plural_forms: usize) -> Option<Misfit> {
    let places = |text: &str| text.matches("{}").count();
    let forms: Vec<&str> = said.split('|').collect();
    if forms.len() > plural_forms.max(1) {
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
    let locale = own.get(LOCALE).or(words.get(LOCALE));
    let forms = Counting::of(locale.and_then(|l| l.as_str()).unwrap_or_default()).plural_forms();
    for (english, said) in own {
        let Some(wording) = said.as_str().filter(|wording| !wording.is_empty()) else {
            continue;
        };
        let why = misfit(&english, wording, forms).filter(|_| english != LOCALE);
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
        assert_eq!(misfit("Needs {}", "Requiert {}", 1), None);
        assert_eq!(misfit("Needs {}", "Requis", 1), Some(Misfit::Places));
        assert_eq!(misfit("{} of {}", "{} sur {} {}", 1), Some(Misfit::Places));
        assert_eq!(misfit("{} mods", "{} mody|{} modów", 3), None);
        assert_eq!(misfit("{} mods", "{} mody|modów", 3), Some(Misfit::Places));
        assert_eq!(
            misfit("{} mods", "{} mody|{} modów", 1),
            Some(Misfit::Forms)
        );
        assert_eq!(misfit("Reading…", "Lecture", 1), Some(Misfit::Edges));
        assert_eq!(misfit(" and ", "et", 1), Some(Misfit::Edges));
    }

    #[test]
    fn your_own_wording_stands_over_the_shipped_one_unless_it_does_not_fit() {
        let shipped = r#"{ "@locale": "fr", "Needs {}": "Requiert {}", "Save": "Enregistrer" }"#;
        let own = r#"{ "Needs {}": "Il faut", "Save": "Sauver", "Play": "", "New": "Neuf" }"#;
        let words = table(shipped, own);
        assert_eq!(words["Needs {}"], "Requiert {}");
        assert_eq!(words["Save"], "Sauver");
        assert_eq!(words["New"], "Neuf");
        assert!(!words.contains_key("Play"));
        assert_eq!(table("not json", "[1]").len(), 0);
        // More forms are allowed once your own file names a locale with more.
        let polish = table(
            r#"{ "{} mods": "x {}" }"#,
            r#"{ "@locale": "pl", "{} mods": "{} mody|{} modów" }"#,
        );
        assert_eq!(polish["{} mods"], "{} mody|{} modów");
        let english = table(r#"{ "{} mods": "x {}" }"#, r#"{ "{} mods": "{} a|{} b" }"#);
        assert_eq!(english["{} mods"], "x {}");
    }

    #[test]
    fn a_count_takes_the_form_cldr_gives_it_in_its_language() {
        let polish = Counting::of("pl");
        let said = |n| polish.pick(n, "mod", "mody|modów");
        assert_eq!(
            [said(1), said(2), said(5), said(12), said(22), said(25)],
            ["mod", "mody", "modów", "modów", "mody", "modów"]
        );
        let french = Counting::of("fr");
        let said = |n| french.pick(n, "mod", "mods");
        assert_eq!(
            [said(0), said(1), said(2), said(1_000_000)],
            ["mod", "mod", "mods", "mods"]
        );
        let russian = Counting::of("ru");
        let said = |n| russian.pick(n, "мод", "мода|модов");
        assert_eq!(
            [said(21), said(11), said(3), said(5)],
            ["мод", "модов", "мода", "модов"]
        );
        for english in [
            Counting::of("en"),
            Counting::of(""),
            Counting::of("not a locale!"),
        ] {
            let said = |n| english.pick(n, "mod", "mods");
            assert_eq!([said(0), said(1), said(2)], ["mods", "mod", "mods"]);
        }
    }

    #[test]
    fn the_language_to_start_in_is_the_first_the_system_asks_for_that_is_offered() {
        let offered = [
            ("English", "en"),
            ("Français", "fr"),
            ("Português", "pt"),
            ("Klingon", ""),
        ];
        let start = |preferred: &[&str]| {
            let preferred: Vec<String> = preferred.iter().map(|p| p.to_string()).collect();
            detected(&preferred, &offered)
        };
        assert_eq!(start(&["fr_FR.UTF-8"]), Some("Français"));
        assert_eq!(start(&["pt-BR", "en-US"]), Some("Português"));
        assert_eq!(start(&["de-DE", "en-GB"]), Some("English"));
        assert_eq!(start(&["ja-JP", "C"]), None);
        assert_eq!(start(&[]), None);
        let regional = [("Português", "pt-PT"), ("Português do Brasil", "pt-BR")];
        let preferred = vec!["pt-BR".to_string()];
        assert_eq!(detected(&preferred, &regional), Some("Português do Brasil"));
    }
}
