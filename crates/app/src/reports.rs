//! Answers the window reads field by field: renaming one here renames
//! what QML must read, so each type's test lists its fields.

use serde::Serialize;

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

/// `Hub.settings.report`: Settings, FirstRun, Main and the Mods screen.
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

/// `Hub.mods.summary`: the counts beside the load order.
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

/// A file written on request: Settings' languages and own colours.
#[derive(Serialize)]
pub struct Written {
    pub file: String,
}

/// `Hub.settings.shortContrast()`: the colour pairs that read too faintly.
#[derive(Serialize, Default)]
pub struct ShortContrast {
    pub what: &'static str,
    pub found: f64,
    pub needed: f64,
}

/// `Hub.settings.release`: a newer RimStack, `{}` when there is none, or why
/// GitHub could not be asked. Read by StatusBar and Settings.
#[derive(Serialize)]
#[serde(untagged)]
pub enum ReleaseSaid {
    Newer { version: String, page: String },
    Latest {},
    Failed { problem: String },
}

/// `Hub.settings.databases`: Settings' list of community databases.
#[derive(Serialize, Default)]
pub struct Database {
    pub id: &'static str,
    pub name: &'static str,
    #[serde(rename = "use")]
    pub used_for: &'static str,
    pub updated: u64,
    pub on: bool,
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
    }
}
