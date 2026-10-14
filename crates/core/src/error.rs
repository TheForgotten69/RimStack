//! Why something asked of the files, of another program or of the web
//! could not be done: what kind of failure it was, for whoever must
//! tell them apart, and the sentence to show, in the chosen language.

use std::fmt::Display;
use std::path::Path;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: Kind,
    said: String,
}

impl Error {
    pub fn new(kind: Kind, said: impl Into<String>) -> Self {
        Self {
            kind,
            said: said.into(),
        }
    }

    /// `path: why`, as the system worded the failure.
    pub fn file(path: &Path, why: impl Display) -> Self {
        Self::new(Kind::File, format!("{}: {why}", path.display()))
    }

    pub fn format(said: impl Into<String>) -> Self {
        Self::new(Kind::Format, said)
    }

    pub fn program(said: impl Into<String>) -> Self {
        Self::new(Kind::Program, said)
    }

    pub fn network(said: impl Into<String>) -> Self {
        Self::new(Kind::Network, said)
    }

    pub fn refused(said: impl Into<String>) -> Self {
        Self::new(Kind::Refused, said)
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The same failure, said of what it stopped: `what: the failure`.
    pub fn of(self, what: impl Display) -> Self {
        Self::new(self.kind, format!("{what}: {}", self.said))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.said)
    }
}

impl std::error::Error for Error {}

/// As its sentence: what the interface is handed is what it shows.
impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, to: S) -> std::result::Result<S::Ok, S::Error> {
        to.serialize_str(&self.said)
    }
}

impl From<Error> for String {
    fn from(error: Error) -> Self {
        error.said
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_keeps_its_kind_whatever_is_said_around_it() {
        let denied = Error::file(Path::new("/a/notes.json"), "denied");
        assert_eq!(denied.to_string(), "/a/notes.json: denied");
        let said = denied.of("the notes were not kept");
        assert_eq!(said.kind(), Kind::File);
        assert_eq!(
            String::from(said),
            "the notes were not kept: /a/notes.json: denied"
        );
    }
}
