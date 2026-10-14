//! Every answer the window is handed as JSON, each a type of its own.
//! QML reads these fields by name: renaming one here renames what QML
//! must read, so each type's test lists its fields, and the doc comment
//! names who reads them.

use cxx_qt_lib::QString;
use rimstack_core::kept::launches::{Shown, SinceClean};
use rimstack_core::mods::blame::{Blamed, Pointed};
use rimstack_core::mods::library::ModId;
use rimstack_core::rimworld::saves;
use rimstack_core::sources::changes::Note;
use rimstack_core::sources::downloads::{Asked, Behind, Changed};
use serde::Serialize;

/// The answer as the window reads it.
pub(crate) fn said<T: Serialize>(value: &T) -> QString {
    QString::from(&serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()))
}

/// What was asked, done, or why not: `{ …what came of it }` or `{ problem }`.
#[derive(Serialize)]
#[serde(untagged)]
pub enum Answer<T> {
    Done(T),
    Failed { problem: String },
}

impl<T> From<Result<T, rimstack_core::base::error::Error>> for Answer<T> {
    fn from(outcome: Result<T, rimstack_core::base::error::Error>) -> Self {
        match outcome {
            Ok(done) => Answer::Done(done),
            Err(problem) => Answer::Failed {
                problem: problem.into(),
            },
        }
    }
}

/// `Backend.settings()`: Settings, FirstRun, Main and the Mods screen.
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettingsReport {
    pub game: Option<String>,
    pub config: Option<String>,
    pub workshop: Option<String>,
    pub game_version: Option<String>,
    pub mods_config: Option<String>,
    pub backup: Option<String>,
    pub set_game: Option<String>,
    pub set_config: Option<String>,
    pub set_workshop: Option<String>,
    pub launch_arguments: Option<String>,
    pub launch_directly: bool,
    pub language: Option<String>,
    pub steamcmd: bool,
    pub interface_scale: String,
    pub font: Option<String>,
    pub community_rules: bool,
    pub version_warnings: bool,
    pub version_aside_for: Option<String>,
    pub file_hints: bool,
    pub git_for_workshop: bool,
    pub git: Option<String>,
    pub self_check: bool,
    pub logs_kept: usize,
    pub logs_kept_now: usize,
    pub replacements_ignored: Vec<String>,
    pub setup_done: bool,
    pub rimsort_rules: bool,
    pub data_folder: String,
    pub settings_file: String,
    pub diagnostics_file: String,
    pub version: &'static str,
}

/// `Backend.summary()`: the Settings screen's counts.
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub saves: usize,
    pub folders: usize,
    pub snapshots: usize,
    pub replacements: usize,
    pub rules: usize,
    pub last_launch: Option<LastLaunch>,
}

#[derive(Serialize, Default)]
pub struct LastLaunch {
    pub time: u64,
    pub errors: Option<usize>,
    pub mods: usize,
}

/// `ModListModel.findMods()`: the Ctrl+K finder.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundMod<'a> {
    pub name: &'a str,
    pub mod_id: ModId,
    pub active: bool,
}

/// `ModListModel.sortMoves()`: SortMoves, and the count kept in the history.
#[derive(Serialize, Default)]
pub struct SortGroup {
    pub name: &'static str,
    pub moves: Vec<SortMove>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SortMove {
    pub name: String,
    pub from: usize,
    pub to: usize,
    /// -1 for a mod that is not installed.
    pub mod_id: i32,
    pub why: String,
}

/// A file written on request: Settings' languages and own colours.
#[derive(Serialize)]
pub struct Written {
    pub file: String,
}

/// `Backend.shortContrast()`: the colour pairs that read too faintly.
#[derive(Serialize, Default)]
pub struct ShortContrast {
    pub what: &'static str,
    pub found: f64,
    pub needed: f64,
}

/// `Backend.release`: a newer RimStack, `{}` when there is none, or why
/// GitHub could not be asked. Read by StatusBar and Settings.
#[derive(Serialize)]
#[serde(untagged)]
pub enum ReleaseSaid {
    Newer { version: String, page: String },
    Latest {},
    Failed { problem: String },
}

/// `SaveReader.saves`: the Saves screen.
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SavesReport {
    pub saves: Vec<saves::Row>,
    pub all_from: Option<String>,
    pub rimsaves: bool,
    pub folders: Vec<String>,
    pub current_folder: Option<String>,
}

/// `BrokenModSearch.huntState()`: HuntBar and the search's dialog.
#[derive(Serialize, Default)]
pub struct HuntState {
    pub active: bool,
    #[serde(flatten)]
    pub search: Option<HuntSearch>,
}

/// `list` is the load order to try now; `found` names the mod once one
/// suspect is left; `stuck` names suspects that need each other.
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HuntSearch {
    pub suspects: usize,
    pub rounds: u32,
    pub list: Vec<String>,
    pub original: Vec<String>,
    pub found: String,
    pub found_id: String,
    pub stuck: Vec<String>,
}

/// `TextureTools.ddsAudit()`: the Textures screen's check of DDS files.
#[derive(Serialize, Default)]
pub struct DdsProblem<'a> {
    #[serde(rename = "mod")]
    pub of_mod: &'a str,
    pub file: String,
    pub why: String,
}

/// `TextureTools.compare()`: CompareView.
#[derive(Serialize)]
pub struct Comparison<'a> {
    pub earlier: Side<'a>,
    pub later: Side<'a>,
    pub textures: Vec<SharedTexture>,
}

#[derive(Serialize)]
pub struct Side<'a> {
    pub id: ModId,
    pub name: &'a str,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SharedTexture {
    pub name: String,
    pub earlier: String,
    pub later: String,
    pub earlier_shown: String,
    pub later_shown: String,
    pub earlier_bytes: u64,
    pub later_bytes: u64,
    /// The later mod wins unless its file is hidden.
    pub keeps_earlier: bool,
    pub your_choice: bool,
}

/// `TextureTools.textureDifference()`: the picture of where two differ.
#[derive(Serialize, Default)]
pub struct Difference {
    pub file: String,
    pub differing: u64,
    pub total: u64,
}

/// `CommunityData.databases()`: Settings' list of community databases.
#[derive(Serialize, Default)]
pub struct Database {
    pub id: &'static str,
    pub name: &'static str,
    #[serde(rename = "use")]
    pub used_for: &'static str,
    pub updated: u64,
    pub on: bool,
}

/// `KeptLists.journal()`: HistoryMenu. `revert` is empty where the
/// entry can no longer be taken back.
#[derive(Serialize, Default)]
pub struct JournalRow<'a> {
    pub time: u64,
    pub what: &'a str,
    pub revert: &'a str,
}

/// `Downloads.updates`: the Downloads screen's updates.
#[derive(Serialize, Default)]
pub struct Updates {
    pub checked: Option<u64>,
    pub asked: Option<u64>,
    pub answered: usize,
    pub items: Vec<Behind>,
    pub checkouts: Vec<Checkout>,
    /// Why Steam was not asked this time; what it said before stands.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

/// A checkout whose updates come from Git, with what its remote said at
/// the last check; `checked` is false for one not asked then.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkout {
    pub mod_id: ModId,
    pub name: String,
    pub path: String,
    pub workshop: bool,
    pub checked: bool,
    pub behind: Option<u64>,
    pub problem: Option<String>,
}

/// `Downloads.queue`: what pasted links are.
#[derive(Serialize, Default)]
pub struct Queue {
    pub items: Vec<Asked>,
    pub repositories: Vec<rimstack_core::sources::downloads::Repository>,
    /// Said when nothing in the text was a Workshop link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// `Downloads.sinceLaunch()`: what changed since the game last started.
#[derive(Serialize, Default)]
pub struct SinceLaunch {
    pub time: u64,
    pub items: Vec<Changed>,
}

/// `Downloads.changeNotesState`: how far fetching change notes got;
/// `resume` is when it goes on after Steam asked it to wait.
#[derive(Serialize, Default)]
pub struct NotesState {
    pub done: usize,
    pub asked: usize,
    pub resume: u64,
    pub refused: bool,
}

/// `Downloads.changeNotes`: per Workshop id, the notes fetched so far.
pub(crate) type ChangeNotes = std::collections::BTreeMap<String, Vec<Note>>;

/// `LogReader.logs`: the Logs screen.
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogsReport {
    pub previous: bool,
    pub has_other: bool,
    pub path: String,
    pub written: u64,
    pub lines: usize,
    pub version: Option<String>,
    pub text: Vec<String>,
    pub errors: Vec<Pointed>,
    pub culprits: Vec<Blamed>,
    pub launches: Vec<Shown>,
    pub since_clean: Option<SinceClean>,
}

/// The fields an answer has, as QML reads them, sorted.
#[cfg(test)]
pub(crate) fn fields<T: Serialize>(value: &T) -> Vec<String> {
    let said = serde_json::to_value(value).unwrap();
    let mut keys: Vec<String> = said.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> ModId {
        ModId::new(0, 1)
    }

    #[test]
    fn settings_keep_their_fields() {
        assert_eq!(
            fields(&SettingsReport::default()),
            [
                "backup",
                "communityRules",
                "config",
                "dataFolder",
                "diagnosticsFile",
                "fileHints",
                "font",
                "game",
                "gameVersion",
                "git",
                "gitForWorkshop",
                "interfaceScale",
                "language",
                "launchArguments",
                "launchDirectly",
                "logsKept",
                "logsKeptNow",
                "modsConfig",
                "replacementsIgnored",
                "rimsortRules",
                "selfCheck",
                "setConfig",
                "setGame",
                "setWorkshop",
                "settingsFile",
                "setupDone",
                "steamcmd",
                "version",
                "versionAsideFor",
                "versionWarnings",
                "workshop",
            ]
        );
    }

    #[test]
    fn the_summary_and_its_last_launch_keep_their_fields() {
        let summary = Summary {
            last_launch: Some(LastLaunch::default()),
            ..Summary::default()
        };
        assert_eq!(
            fields(&summary),
            [
                "folders",
                "lastLaunch",
                "replacements",
                "rules",
                "saves",
                "snapshots"
            ]
        );
        assert_eq!(fields(&LastLaunch::default()), ["errors", "mods", "time"]);
    }

    #[test]
    fn the_mods_screen_answers_keep_their_fields() {
        let found = FoundMod {
            name: "Roads",
            mod_id: id(),
            active: true,
        };
        assert_eq!(fields(&found), ["active", "modId", "name"]);
        assert_eq!(fields(&SortGroup::default()), ["moves", "name"]);
        assert_eq!(
            fields(&SortMove::default()),
            ["from", "modId", "name", "to", "why"]
        );
    }

    #[test]
    fn a_failure_is_its_problem_and_nothing_else() {
        let failed: Answer<Written> = Err(rimstack_core::base::error::Error::refused("no")).into();
        assert_eq!(
            serde_json::to_value(&failed).unwrap(),
            serde_json::json!({ "problem": "no" })
        );
        let done: Answer<Written> = Ok(Written { file: "/a".into() }).into();
        assert_eq!(fields(&done), ["file"]);
    }

    #[test]
    fn appearance_answers_keep_their_fields() {
        assert_eq!(
            fields(&ShortContrast::default()),
            ["found", "needed", "what"]
        );
        let newer = ReleaseSaid::Newer {
            version: "1.1".into(),
            page: "p".into(),
        };
        assert_eq!(fields(&newer), ["page", "version"]);
        assert!(fields(&ReleaseSaid::Latest {}).is_empty());
        let failed = ReleaseSaid::Failed {
            problem: "x".into(),
        };
        assert_eq!(fields(&failed), ["problem"]);
    }

    #[test]
    fn saves_and_the_broken_mod_search_keep_their_fields() {
        assert_eq!(
            fields(&SavesReport::default()),
            ["allFrom", "currentFolder", "folders", "rimsaves", "saves"]
        );
        assert_eq!(fields(&HuntState::default()), ["active"]);
        let searching = HuntState {
            active: true,
            search: Some(HuntSearch::default()),
        };
        assert_eq!(
            fields(&searching),
            ["active", "found", "foundId", "list", "original", "rounds", "stuck", "suspects"]
        );
    }

    #[test]
    fn texture_answers_keep_their_fields() {
        assert_eq!(fields(&DdsProblem::default()), ["file", "mod", "why"]);
        let side = || Side {
            id: id(),
            name: "a",
        };
        let comparison = Comparison {
            earlier: side(),
            later: side(),
            textures: Vec::new(),
        };
        assert_eq!(fields(&comparison), ["earlier", "later", "textures"]);
        assert_eq!(fields(&side()), ["id", "name"]);
        assert_eq!(
            fields(&SharedTexture::default()),
            [
                "earlier",
                "earlierBytes",
                "earlierShown",
                "keepsEarlier",
                "later",
                "laterBytes",
                "laterShown",
                "name",
                "yourChoice"
            ]
        );
        assert_eq!(
            fields(&Difference::default()),
            ["differing", "file", "total"]
        );
    }

    #[test]
    fn lists_and_databases_keep_their_fields() {
        let saved = rimstack_core::kept::lists::Saved {
            name: String::new(),
            mods: 0,
            modified: 0,
        };
        assert_eq!(fields(&saved), ["modified", "mods", "name"]);
        assert_eq!(
            fields(&Database::default()),
            ["id", "name", "on", "updated", "use"]
        );
        assert_eq!(fields(&JournalRow::default()), ["revert", "time", "what"]);
    }

    #[test]
    fn download_answers_keep_their_fields() {
        assert_eq!(
            fields(&Updates::default()),
            ["answered", "asked", "checked", "checkouts", "items"]
        );
        let failed_once = Updates {
            problem: Some("x".into()),
            ..Updates::default()
        };
        assert!(fields(&failed_once).contains(&"problem".to_string()));
        let checkout = Checkout {
            mod_id: id(),
            name: String::new(),
            path: String::new(),
            workshop: false,
            checked: false,
            behind: None,
            problem: None,
        };
        assert_eq!(
            fields(&checkout),
            ["behind", "checked", "modId", "name", "path", "problem", "workshop"]
        );
        assert_eq!(fields(&Queue::default()), ["items", "repositories"]);
        assert_eq!(fields(&SinceLaunch::default()), ["items", "time"]);
        assert_eq!(
            fields(&NotesState::default()),
            ["asked", "done", "refused", "resume"]
        );
    }

    #[test]
    fn the_logs_report_keeps_its_fields() {
        assert_eq!(
            fields(&LogsReport::default()),
            [
                "culprits",
                "errors",
                "hasOther",
                "launches",
                "lines",
                "path",
                "previous",
                "sinceClean",
                "text",
                "version",
                "written"
            ]
        );
    }
}
