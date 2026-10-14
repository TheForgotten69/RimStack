//! Why something asked of the files, of another program or of the web
//! could not be done: what kind of failure it was, for whoever must
//! tell them apart, and the sentence that says so. The sentence is kept
//! as its English and its parts, and put in words when it is shown: in
//! the chosen language on screen, in English in the diagnostics.

use std::fmt::Display;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A file or folder could not be read, written, moved or deleted.
    File,
    /// What a file or an answer holds is not what it should be.
    Format,
    /// Another program could not be run, or ended saying it failed.
    Program,
    /// The web was not reached, or did not answer.
    Network,
    /// Nothing failed: what was asked is not done, to keep something safe.
    Refused,
}

/// A sentence as written in English, with `{}` for each part, put in
/// words only when it is shown. Text from elsewhere (the system's own
/// wording of a failure) is kept as it came.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    english: String,
    parts: Vec<Part>,
    /// Looked up in the language's table; text from elsewhere is not.
    translated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    Text(String),
    /// Another failure said inside this one, put in words with it.
    Said(Box<Sentence>),
}

impl<T: Display> From<&T> for Part {
    fn from(part: &T) -> Self {
        Part::Text(part.to_string())
    }
}

/// An error given as a part of a sentence, to be put in words with it.
pub struct Within<'a>(&'a Error);

impl From<&Within<'_>> for Part {
    fn from(within: &Within<'_>) -> Self {
        Part::Said(Box::new(within.0.said.clone()))
    }
}

impl Sentence {
    pub fn new(english: &str, parts: Vec<Part>) -> Self {
        Self {
            english: english.to_string(),
            parts,
            translated: true,
        }
    }

    fn plain(text: String) -> Self {
        Self {
            english: text,
            parts: Vec::new(),
            translated: false,
        }
    }

    /// In the chosen language, or in English within `translate::in_english`.
    fn put(&self) -> String {
        if !self.translated {
            return self.english.clone();
        }
        let parts: Vec<String> = self.parts.iter().map(Part::put).collect();
        let parts: Vec<&dyn Display> = parts.iter().map(|p| p as &dyn Display).collect();
        crate::base::translate::fill(&self.english, &parts)
    }
}

impl Part {
    fn put(&self) -> String {
        match self {
            Part::Text(text) => text.clone(),
            Part::Said(sentence) => sentence.put(),
        }
    }
}

impl From<String> for Sentence {
    fn from(text: String) -> Self {
        Sentence::plain(text)
    }
}

impl From<&str> for Sentence {
    fn from(text: &str) -> Self {
        Sentence::plain(text.to_string())
    }
}

/// `said!("Needs {}", name)`: a sentence for an `Error`, kept in English
/// and put in words when shown. An error inside is given as
/// `problem.within()`.
#[macro_export]
macro_rules! said {
    ($text:literal $(, $part:expr)* $(,)?) => {
        $crate::base::error::Sentence::new($text, vec![$($crate::base::error::Part::from(&$part)),*])
    };
}

type Cause = Arc<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
pub struct Error {
    kind: Kind,
    said: Sentence,
    /// What failed underneath, as the system or a library gave it.
    cause: Option<Cause>,
}

/// Two failures are the same when they say the same of the same kind.
impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.said == other.said
    }
}

impl Eq for Error {}

impl Error {
    pub fn new(kind: Kind, said: impl Into<Sentence>) -> Self {
        Self {
            kind,
            said: said.into(),
            cause: None,
        }
    }

    /// `said: cause`, the cause kept as this failure's source.
    pub fn caused(
        kind: Kind,
        said: Sentence,
        cause: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        let said = Sentence::new(
            "{}: {}",
            vec![Part::Said(Box::new(said)), Part::from(&cause)],
        );
        Self {
            kind,
            said,
            cause: Some(Arc::new(cause)),
        }
    }

    /// `path: cause`, as the system worded the failure.
    pub fn file(path: &Path, cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            kind: Kind::File,
            said: Sentence::plain(format!("{}: {cause}", path.display())),
            cause: Some(Arc::new(cause)),
        }
    }

    pub fn format(said: impl Into<Sentence>) -> Self {
        Self::new(Kind::Format, said)
    }

    pub fn program(said: impl Into<Sentence>) -> Self {
        Self::new(Kind::Program, said)
    }

    pub fn network(said: impl Into<Sentence>) -> Self {
        Self::new(Kind::Network, said)
    }

    pub fn refused(said: impl Into<Sentence>) -> Self {
        Self::new(Kind::Refused, said)
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The same failure, said of what it stopped: `what: the failure`.
    pub fn of(self, what: impl Display) -> Self {
        let said = Sentence {
            english: "{}: {}".to_string(),
            parts: vec![
                Part::Text(what.to_string()),
                Part::Said(Box::new(self.said)),
            ],
            translated: true,
        };
        Self { said, ..self }
    }

    /// This failure as a part of another one's sentence.
    pub fn within(&self) -> Within<'_> {
        Within(self)
    }
}

/// In the chosen language: what the window shows.
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.said.put())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        let cause = self.cause.as_deref()?;
        Some(cause)
    }
}

/// As its sentence: what the interface is handed is what it shows.
impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, to: S) -> std::result::Result<S::Ok, S::Error> {
        to.serialize_str(&self.said.put())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_is_put_in_words_when_shown_and_in_english_for_the_diagnostics() {
        let _alone = crate::base::translate::TABLE_IN_USE.lock();
        crate::base::translate::set(std::collections::HashMap::from([
            (
                "For the error test {}".to_string(),
                "Pour le test {}".to_string(),
            ),
            ("Inside the error test".to_string(), "Dedans".to_string()),
        ]));
        let inner = Error::refused(said!("Inside the error test"));
        let outer = Error::program(said!("For the error test {}", inner.within()));
        assert_eq!(outer.to_string(), "Pour le test Dedans");
        assert_eq!(
            crate::base::translate::in_english(|| outer.to_string()),
            "For the error test Inside the error test"
        );
        crate::base::translate::set(std::collections::HashMap::new());
        // Text from elsewhere is kept as it came, even with braces in it.
        assert_eq!(
            Error::format("bad {} value".to_string()).to_string(),
            "bad {} value"
        );
    }

    #[test]
    fn a_failure_keeps_its_kind_and_its_cause_whatever_is_said_around_it() {
        use std::error::Error as _;
        let denied = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let denied = Error::file(Path::new("/a/notes.json"), denied);
        assert_eq!(denied.to_string(), "/a/notes.json: denied");
        let said = denied.of("the notes were not kept");
        assert_eq!(said.kind(), Kind::File);
        assert_eq!(
            said.to_string(),
            "the notes were not kept: /a/notes.json: denied"
        );
        let cause = said
            .source()
            .and_then(|c| c.downcast_ref::<std::io::Error>());
        assert_eq!(
            cause.map(|c| c.kind()),
            Some(std::io::ErrorKind::PermissionDenied)
        );
        let missing = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let caused = Error::caused(Kind::File, said!("Not kept"), missing);
        assert_eq!(caused.to_string(), "Not kept: gone");
        assert!(caused.source().is_some());
    }
}
