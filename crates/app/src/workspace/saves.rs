//! What the Saves screen shows: each save with how its mods differ from
//! the load order being edited, and RimSaves' folders when it is used.

use super::App;
use rimstack_core::base::error::Error;
use rimstack_core::rimworld::{rimsaves, saves};
use rimstack_core::{say, say_n};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Default)]
pub struct SavesReport {
    /// Newest first.
    pub rows: Vec<saves::Row>,
    pub folders: Vec<String>,
    pub current_folder: Option<String>,
    /// RimSaves keeps folders of saves.
    pub rimsaves: bool,
    /// The game version every save that is not this game's shares, if one does.
    pub all_from: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveShown<'a> {
    #[serde(flatten)]
    pub row: &'a saves::Row,
    pub said: String,
    /// Its version, mod count and folder, as the row lists them under its name.
    pub about: String,
    pub dir: String,
    pub not_installed: usize,
    pub not_active: usize,
    /// Its mod list is one to take: read, and not the list already.
    pub usable: bool,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SavesShown<'a> {
    pub saves: Vec<SaveShown<'a>>,
    pub heading: String,
    /// Some save has a picture: every row keeps room for one.
    pub pictured: bool,
}

fn said(row: &saves::Row) -> String {
    match row.verdict {
        saves::Verdict::Unreadable => say!("Cannot be read"),
        saves::Verdict::Same => say!("Same mods as now"),
        saves::Verdict::Differs => say!("Loads with the current list"),
        saves::Verdict::Breaks => {
            let compared = row.compared.as_ref();
            let missing = compared.map_or(0, |c| c.missing.len());
            let mut parts = Vec::new();
            if missing > 0 {
                parts.push(say_n!(missing, "{} mod missing", "{} mods missing"));
            }
            if compared.is_some_and(|c| c.other_game) {
                parts.push(say!("other game version"));
            }
            parts.join(", ")
        }
    }
}

/// The saves in `folder`, every one for none.
pub fn shown<'a>(report: &'a SavesReport, folder: &str) -> SavesShown<'a> {
    let saves: Vec<SaveShown> = report
        .rows
        .iter()
        .filter(|row| folder.is_empty() || row.folder == folder)
        .map(|row| {
            let compared = row.compared.as_ref();
            let missing = compared.map_or(&[][..], |c| &c.missing[..]);
            let not_installed = missing.iter().filter(|m| !m.installed).count();
            let mut about = Vec::new();
            if let Some(compared) = compared {
                about.push(format!("RimWorld {}", compared.game_version));
                about.push(say_n!(compared.mods, "{} mod", "{} mods"));
            }
            if report.rimsaves && folder.is_empty() {
                about.push(row.folder.clone());
            }
            let dir = std::path::Path::new(&row.path).parent();
            SaveShown {
                row,
                said: said(row),
                about: about.join("   "),
                dir: dir.map_or(String::new(), |d| d.to_string_lossy().into_owned()),
                not_installed,
                not_active: missing.len() - not_installed,
                usable: !matches!(
                    row.verdict,
                    saves::Verdict::Unreadable | saves::Verdict::Same
                ),
            }
        })
        .collect();
    let count = saves.len();
    let mut heading = vec![match folder.is_empty() {
        true => count.to_string(),
        false => say!("{} in {}", count, folder),
    }];
    if let Some(version) = &report.all_from {
        heading.push(say!("all from RimWorld {}", version));
    }
    SavesShown {
        pictured: saves.iter().any(|s| s.row.preview.is_some()),
        heading: heading.join(", "),
        saves,
    }
}

impl App {
    pub fn saves_dir(&self) -> Option<PathBuf> {
        self.paths().config.as_ref().map(|c| c.join("Saves"))
    }

    /// Every save, each compared with the load order being edited.
    pub fn saves_report(&self) -> SavesReport {
        let config = self.paths().config.clone();
        let list = self.session().current().to_vec();
        let loaded = self.library();
        let rows: Vec<saves::Row> = match (&*loaded, self.saves_dir()) {
            (Ok((library, _)), Some(dir)) => saves::list(&dir)
                .iter()
                .map(|save| {
                    let config = config.clone().unwrap_or_default();
                    let compared = save.header.as_ref().ok().map(|header| {
                        let name_of = |key: &str| library.name_of(key);
                        saves::Compared::of(header, &list, &library.game_version, &name_of)
                    });
                    saves::Row::of(
                        save,
                        rimsaves::facts(&rimsaves::meta(&config, &save.name)),
                        rimsaves::preview(&config, &save.name),
                        compared,
                    )
                })
                .collect(),
            _ => Vec::new(),
        };
        // RimSaves' folders, when the mod has been used: those it lists,
        // and any that only show up in a file name.
        let kept = config.as_deref().and_then(rimsaves::settings);
        let mut folders = kept.as_ref().map_or(Vec::new(), |s| s.folders.clone());
        for row in &rows {
            if !folders.contains(&row.folder) {
                folders.push(row.folder.clone());
            }
        }
        let versions: Vec<(&str, bool)> = rows
            .iter()
            .map(|row| match &row.compared {
                Some(compared) => (compared.game_version.as_str(), compared.other_game),
                None => ("", false),
            })
            .collect();
        SavesReport {
            all_from: saves::version_all_share(&versions),
            rimsaves: kept.is_some(),
            folders,
            current_folder: kept.map(|s| s.current),
            rows,
        }
    }

    /// Copies the newest save among `kept` copies in RimStack's data
    /// folder under `into`; nothing without a save.
    pub fn back_up_newest_save(&self, into: &str, kept: usize) -> Result<(), Error> {
        let Some(newest) = self
            .saves_dir()
            .and_then(|dir| saves::list(&dir).into_iter().next())
        else {
            return Ok(());
        };
        let backups = self.data_dir().join(into);
        saves::back_up(&newest.path, &backups, super::now_secs(), kept).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::shown;
    use crate::workspace::tests::installed;

    const SAVE: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<savegame><meta><gameVersion>1.6.4871 rev600</gameVersion>
<modIds><li>ludeon.rimworld</li><li>ann.roads</li><li>gone.mod</li></modIds>
<modNames><li>Core</li><li>Ann.Roads</li><li>Gone</li></modNames></meta><game/></savegame>";

    #[test]
    fn a_save_is_compared_with_the_load_order_and_said_in_words() {
        let (tmp, app) = installed();
        let saves = tmp.path().join("config/Saves");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("Colony.rws"), SAVE).unwrap();
        let report = app.saves_report();
        let all = serde_json::to_value(shown(&report, "")).unwrap();
        let save = &all["saves"][0];
        assert_eq!(save["name"], "Colony");
        assert_eq!(save["verdict"], "breaks");
        assert_eq!(save["said"], "2 mods missing");
        assert_eq!(save["about"], "RimWorld 1.6.4871 rev600   3 mods");
        assert_eq!(save["notInstalled"], 1);
        assert_eq!(save["notActive"], 1);
        assert_eq!(save["usable"], true);
        assert_eq!(save["dir"], saves.to_string_lossy().as_ref());
        assert_eq!(all["heading"], "1");
        assert_eq!(all["pictured"], false);
        assert!(!report.rimsaves);
        let elsewhere = serde_json::to_value(shown(&report, "Other")).unwrap();
        assert_eq!(elsewhere["saves"].as_array().unwrap().len(), 0);
        assert_eq!(elsewhere["heading"], "0 in Other");
    }

    #[test]
    fn the_newest_save_is_copied_and_none_is_no_failure() {
        let (tmp, app) = installed();
        app.back_up_newest_save("save-backups", 5).unwrap();
        let saves = tmp.path().join("config/Saves");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("Colony.rws"), SAVE).unwrap();
        app.back_up_newest_save("save-backups", 5).unwrap();
        let kept = std::fs::read_dir(app.data_dir().join("save-backups"))
            .unwrap()
            .count();
        assert_eq!(kept, 1);
    }
}
