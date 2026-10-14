//! A theme and a language of the user's own.

use super::*;

/// The languages RimStack comes with, each a JSON object from English
/// to it. A file of the same name in the settings folder stands over
/// one, text by text.
const SHIPPED: &[(&str, &str)] = &[
    (
        "English",
        include_str!("../../assets/languages/English.json"),
    ),
    (
        "Deutsch",
        include_str!("../../assets/languages/Deutsch.json"),
    ),
    (
        "Español",
        include_str!("../../assets/languages/Español.json"),
    ),
    (
        "Français",
        include_str!("../../assets/languages/Français.json"),
    ),
    (
        "Italiano",
        include_str!("../../assets/languages/Italiano.json"),
    ),
    ("Polski", include_str!("../../assets/languages/Polski.json")),
    (
        "Português",
        include_str!("../../assets/languages/Português.json"),
    ),
];

fn shipped(name: &str) -> Option<&'static str> {
    SHIPPED
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, words)| *words)
}

/// A language's texts: those it ships with, under those of the file of
/// its name in the settings folder.
fn language_table(name: &str) -> Value {
    let name = crate::language_name(name);
    let file = languages_dir().join(format!("{name}.json"));
    let own = std::fs::read_to_string(file).unwrap_or_default();
    Value::Object(rimstack_core::tongue::table(
        shipped(name).unwrap_or_default(),
        &own,
    ))
}

/// Has the sentences written in Rust said in this language from now on.
pub fn speak(name: &str) -> Value {
    let words = language_table(name);
    let said = words.as_object().into_iter().flatten();
    let table =
        said.filter_map(|(english, said)| Some((english.clone(), said.as_str()?.to_string())));
    rimstack_core::tongue::set(table.collect());
    words
}

/// A file written on request, as `{ file }`, or `{ problem }`.
fn made(file: Result<&std::path::Path, String>) -> QString {
    let said = match file {
        Ok(file) => json!({ "file": file.to_string_lossy() }),
        Err(problem) => json!({ "problem": problem }),
    };
    QString::from(&said.to_string())
}

impl qobject::Backend {
    pub(super) fn languages(&self) -> QString {
        let files = std::fs::read_dir(languages_dir())
            .into_iter()
            .flatten()
            .flatten();
        let mut names: Vec<String> = files
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|x| x == "json"))
            .filter_map(|path| Some(path.file_stem()?.to_string_lossy().into_owned()))
            .collect();
        names.sort();
        names.retain(|name| shipped(name).is_none());
        let all: Vec<&str> = SHIPPED
            .iter()
            .map(|(name, _)| *name)
            .chain(names.iter().map(String::as_str))
            .collect();
        QString::from(&json!(all).to_string())
    }

    pub(super) fn language_words(&self, name: &QString) -> QString {
        QString::from(&speak(&name.to_string()).to_string())
    }

    pub(super) fn start_language(&self, name: &QString) -> QString {
        let name = name.to_string();
        if name.trim().is_empty() || name.contains(['/', '\\']) {
            return made(Err(say!("A language needs a name without slashes")));
        }
        let file = languages_dir().join(format!("{}.json", name.trim()));
        let write = || {
            std::fs::create_dir_all(languages_dir())?;
            let start = shipped(name.trim()).unwrap_or(include_str!("../../assets/words.json"));
            std::fs::write(&file, start)
        };
        if !file.exists() {
            if let Err(problem) = write() {
                return made(Err(format!("{}: {problem}", file.display())));
            }
        }
        made(Ok(&file))
    }

    pub(super) fn stamps(&self) -> QString {
        QString::from(&crate::app().stamps().to_string())
    }

    pub(super) fn use_font(&self, family: &QString) -> QString {
        QString::from(&crate::use_font(&family.to_string()))
    }

    pub(super) fn short_contrast(&self, colours: &QString) -> QString {
        let colours = serde_json::from_str::<Value>(&colours.to_string()).unwrap_or(Value::Null);
        let short = colours.as_object().map(owntheme::short_pairs);
        let listed: Vec<Value> = short
            .unwrap_or_default()
            .into_iter()
            .map(|(what, found, needed)| json!({ "what": what, "found": found, "needed": needed }))
            .collect();
        QString::from(&Value::Array(listed).to_string())
    }

    pub(super) fn own_theme(&self, theme: &QString) -> QString {
        let theme = theme.to_string();
        let text = std::fs::read_to_string(theme_file()).unwrap_or_default();
        let file = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
        // An older file belongs to the theme in use when it is first met.
        let known = crate::app()
            .settings()
            .get("ownColoursFor")
            .map(String::from);
        let flat_for = match known {
            Some(known) => known,
            None if owntheme::is_flat(&file) => {
                if let Err(problem) = crate::app().set_setting("ownColoursFor", &theme) {
                    rimstack_core::problem!("{problem}");
                }
                theme.clone()
            }
            None => String::new(),
        };
        let colours = owntheme::colours_for(&file, &theme, &flat_for);
        QString::from(&Value::Object(colours).to_string())
    }

    pub(super) fn start_own_theme(&self, theme: &QString, colours: &QString) -> QString {
        let file = theme_file();
        let colours = serde_json::from_str::<Value>(&colours.to_string()).unwrap_or(Value::Null);
        let theme = theme.to_string();
        let now = std::fs::read_to_string(&file)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok());
        let wanted = match now {
            None => Some(json!({ theme: colours })),
            Some(now) => owntheme::with_theme(&now, &theme, &colours),
        };
        let pretty = wanted.and_then(|w| serde_json::to_string_pretty(&w).ok());
        if let Some(pretty) = pretty {
            if let Err(problem) = std::fs::write(&file, pretty) {
                return made(Err(format!("{}: {problem}", file.display())));
            }
        }
        made(Ok(&file))
    }
}
