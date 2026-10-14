//! The words of the language chosen, for the sentences written in Rust.
//! `core` hands its sentences over in English and parts; they are put
//! in words here, on the window's thread, as they are handed to QML.

use cxx_qt_lib::QString;
use rimstack_core::base::error::Error;
use rimstack_core::base::sentence::{Language, Sentence};
use rimstack_core::base::translate::PLURAL_RULE;
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
pub struct Words {
    table: HashMap<String, String>,
    rule: String,
}

impl Words {
    /// A language's table, as `App::language_table` gives it.
    pub fn of(table: &Map<String, Value>) -> Self {
        let said = |(english, said): (&String, &Value)| {
            let said = said.as_str().filter(|said| !said.is_empty())?;
            Some((english.clone(), said.to_string()))
        };
        let table: HashMap<String, String> = table.iter().filter_map(said).collect();
        Self {
            rule: table.get(PLURAL_RULE).cloned().unwrap_or_default(),
            table,
        }
    }
}

impl Language for Words {
    fn word<'a>(&'a self, english: &'a str) -> &'a str {
        self.table.get(english).map_or(english, String::as_str)
    }

    fn plural_rule(&self) -> &str {
        match self.rule.is_empty() {
            true => "1",
            false => &self.rule,
        }
    }
}

thread_local! {
    /// Set on the window's thread, the only one that hands sentences to QML.
    static ON_SCREEN: RefCell<Rc<Words>> = RefCell::new(Rc::new(Words::default()));
}

/// From now on, what is shown is put in these words.
pub fn show_in(words: Words) {
    ON_SCREEN.with(|on_screen| *on_screen.borrow_mut() = Rc::new(words));
}

pub fn put(sentence: &Sentence) -> String {
    let words = ON_SCREEN.with(|on_screen| on_screen.borrow().clone());
    sentence.put(words.as_ref())
}

pub fn said(sentence: &Sentence) -> QString {
    QString::from(&put(sentence))
}

/// A failure as shown on screen.
pub fn said_of(problem: &Error) -> QString {
    said(problem.sentence())
}

/// Why something was not done, as shown on screen; nothing when it was.
pub fn why_not<T>(done: &Result<T, Error>) -> QString {
    done.as_ref().err().map_or_else(QString::default, said_of)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rimstack_core::say_n;

    #[test]
    fn a_sentence_is_shown_in_the_words_set_and_english_before_any() {
        let counted = say_n!(0, "{} mod", "{} mods");
        assert_eq!(put(&counted), "0 mods");
        let table = serde_json::json!({ "@plural": "0 1", "{} mod": "{} mod", "{} mods": "{} mods", "Play": "" });
        let words = Words::of(table.as_object().unwrap());
        assert_eq!(
            words.word("Play"),
            "Play",
            "an empty wording leaves the English"
        );
        show_in(words);
        assert_eq!(put(&counted), "0 mod");
        show_in(Words::default());
    }
}
