use crate::workspace::browse::{Asked, Part, Related, Shown};
use rimstack_core::base::sentence::Sentence;
use rimstack_core::say;
use serde::Serialize;

const RELATED: [(&str, Related); 4] = [
    ("requires", Related::Requires),
    ("neededBy", Related::NeededBy),
    ("sameAuthor", Related::SameAuthor),
    ("collection", Related::Collection),
];

pub fn related_of(name: &str) -> Option<Related> {
    RELATED
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, which)| *which)
}

fn related_name(which: Related) -> &'static str {
    RELATED
        .iter()
        .find(|(_, known)| *known == which)
        .map_or("", |(name, _)| name)
}

#[derive(Clone, Debug, PartialEq)]
pub enum View {
    Search(Asked),
    Related(Related, String),
}

/// `which` is empty and `active` false while the results are a search's.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RelatedShown {
    pub active: bool,
    pub which: &'static str,
    pub id: String,
    pub title: String,
    /// Empty while `active` is false.
    pub said: Option<Sentence>,
}

impl RelatedShown {
    pub fn none() -> Self {
        Self {
            active: false,
            which: "",
            id: String::new(),
            title: String::new(),
            said: None,
        }
    }

    /// `title` is empty until the mod's name is known; the id stands in for it.
    pub fn of(which: Related, id: &str, title: &str) -> Self {
        let name = if title.is_empty() { id } else { title };
        let said = match which {
            Related::Requires => say!("Requirements of {}", name),
            Related::NeededBy => say!("Mods that need {}", name),
            Related::SameAuthor => say!("More by the author of {}", name),
            Related::Collection => say!("In the collection {}", name),
        };
        Self {
            active: true,
            which: related_name(which),
            id: id.to_string(),
            title: title.to_string(),
            said: Some(said),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailsShown<'a> {
    #[serde(flatten)]
    pub shown: &'a Shown,
    pub preview_path: &'a str,
    pub description: Vec<Part>,
    pub screenshots: &'a [String],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_related_list_is_read_from_its_name() {
        for (name, which) in RELATED {
            assert_eq!(related_of(name), Some(which));
            assert_eq!(RelatedShown::of(which, "1", "T").which, name);
        }
        assert_eq!(related_of("requiredBy"), None);
    }

    #[test]
    fn nothing_related_is_inactive_and_a_list_names_its_subject() {
        let none = RelatedShown::none();
        assert!(!none.active && none.which.is_empty());
        let some = RelatedShown::of(Related::NeededBy, "7", "Harmony");
        assert!(some.active && some.id == "7" && some.title == "Harmony");
        let said = |which, title| {
            RelatedShown::of(which, "7", title)
                .said
                .unwrap()
                .to_string()
        };
        assert_eq!(said(Related::NeededBy, "Harmony"), "Mods that need Harmony");
        assert_eq!(said(Related::Requires, ""), "Requirements of 7");
        assert_eq!(none.said, None);
    }
}
