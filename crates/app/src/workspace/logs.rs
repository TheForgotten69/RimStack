//! What the Logs screen shows: a log's errors with the mods they point
//! at, the launches on record, and what changed since the last clean one.

use super::App;
use rimstack_core::base::error::Error;
use rimstack_core::base::sentence::Sentence;
use rimstack_core::kept::launches::{self, LastStart, LogRead, Shown, SinceClean};
use rimstack_core::mods::blame::{Blamed, Pointed};
use rimstack_core::mods::library::ModId;
use rimstack_core::rimworld::{id, logs, paths::Source};
use rimstack_core::{said, say, say_n};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// How many lines of a log the screen is given to show.
const SHOWN_LINES: usize = 30_000;

/// A start of the game more than this long after the last launch from
/// here was not one of them: from Steam, or a round of the search.
const LAUNCH_SLACK_SECS: u64 = 120;

/// The longest kind of error: the part of its first line before a colon.
const LONGEST_KIND: usize = 90;

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
    /// The game's last start is none of the launches recorded here.
    pub outside: bool,
}

/// A launch's outcome in words: its kinds of error, or that its log was
/// not read.
pub fn outcome(errors: Option<usize>) -> Sentence {
    match errors {
        None => say!("Not read"),
        Some(0) => say!("Clean"),
        Some(n) => say_n!(n, "{} error", "{} errors"),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Head<'a> {
    pub previous: bool,
    pub has_other: bool,
    pub path: &'a str,
    pub written: u64,
    pub lines: usize,
    /// The game's version and the log's length.
    pub about: Sentence,
    /// Said when only the first lines are shown.
    pub cut: Sentence,
    pub outcome: Sentence,
    pub errors: usize,
    pub outside: bool,
}

pub fn head(report: &LogsReport) -> Head<'_> {
    let lines = say_n!(report.lines, "{} line", "{} lines");
    let version = report
        .version
        .as_ref()
        .map(|v| Sentence::Plain(format!("RimWorld {v}")));
    let about = Sentence::listed(version.into_iter().chain([lines]).collect());
    let shown = report.text.len();
    let cut = match report.lines > shown {
        true => say_n!(
            report.lines,
            "{} line, the first {} shown",
            "{} lines, the first {} shown",
            shown
        ),
        false => Sentence::default(),
    };
    Head {
        previous: report.previous,
        has_other: report.has_other,
        path: &report.path,
        written: report.written,
        lines: report.lines,
        about,
        cut,
        outcome: outcome(Some(report.errors.len())),
        errors: report.errors.len(),
        outside: report.outside,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchShown<'a> {
    #[serde(flatten)]
    pub launch: &'a Shown,
    pub outcome: Sentence,
}

pub fn launches(report: &LogsReport) -> Vec<LaunchShown<'_>> {
    report
        .launches
        .iter()
        .map(|launch| LaunchShown {
            launch,
            outcome: outcome(launch.errors),
        })
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Since<'a> {
    pub time: u64,
    pub ids: &'a [String],
    pub added: String,
    pub removed: String,
    pub reordered: bool,
    /// The load order is the clean launch's.
    pub same: bool,
}

pub fn since(report: &LogsReport) -> Option<Since<'_>> {
    report.since_clean.as_ref().map(|since| Since {
        time: since.time,
        ids: &since.ids,
        added: since.added.join(", "),
        removed: since.removed.join(", "),
        reordered: since.reordered,
        same: since.added.is_empty() && since.removed.is_empty() && !since.reordered,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorShown<'a> {
    #[serde(flatten)]
    pub error: &'a Pointed,
    /// The part of the first line before its colon, or all of it.
    pub kind: &'a str,
    /// The rest of the first line, after a kind.
    pub detail: &'a str,
    /// How many more alike errors its run hides or shows; 0 for none.
    pub more: usize,
    pub opened: bool,
    /// The stack trace, with a place to break after each separator.
    pub trace_text: String,
}

fn kind_of(head: &str) -> (&str, &str) {
    match head.find(':') {
        Some(at) if (1..=LONGEST_KIND).contains(&head[..at].chars().count()) => {
            (&head[..at], head[at + 1..].trim())
        }
        _ => (head, ""),
    }
}

fn breakable(trace: &[String]) -> String {
    let lines: Vec<String> = trace
        .iter()
        .map(|line| {
            let mut out = String::with_capacity(line.len());
            for c in line.trim_end().chars() {
                out.push(c);
                if matches!(c, '.' | '/' | '(' | ',') {
                    out.push('\u{200b}');
                }
            }
            out
        })
        .collect();
    lines.join("\n")
}

/// The errors this filter text lets through; a run of alike errors
/// shows as its first unless its start is among `open_runs`.
pub fn listed<'a>(
    errors: &'a [Pointed],
    filter: &str,
    open_runs: &HashSet<usize>,
) -> Vec<ErrorShown<'a>> {
    errors
        .iter()
        .filter(|error| {
            logs::listed(
                filter,
                &error.head,
                error.first.as_deref(),
                error.run == error.at,
                open_runs.contains(&error.run),
            )
        })
        .map(|error| {
            let (kind, detail) = kind_of(&error.head);
            let leads = error.alike > 1 && error.run == error.at && filter.is_empty();
            ErrorShown {
                error,
                kind,
                detail,
                more: if leads { error.alike - 1 } else { 0 },
                opened: open_runs.contains(&error.run),
                trace_text: breakable(&error.trace),
            }
        })
        .collect()
}

/// A line of the log as the screen lists it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line<'a> {
    pub number: usize,
    pub text: &'a str,
    /// A kind of error is first logged on it.
    pub error_starts: bool,
}

/// The log's lines that hold the filter text, numbered from 1.
pub fn lines<'a>(report: &'a LogsReport, filter: &str) -> Vec<Line<'a>> {
    let starts: HashSet<usize> = report.errors.iter().map(|e| e.line).collect();
    report
        .text
        .iter()
        .enumerate()
        .filter(|(_, text)| filter.is_empty() || text.to_lowercase().contains(filter))
        .map(|(at, text)| Line {
            number: at + 1,
            text,
            error_starts: starts.contains(&(at + 1)),
        })
        .collect()
}

/// The errors counted against the mod of this name; the empty name for
/// those that point at none.
pub fn errors_of<'a>(report: &'a LogsReport, name: &str) -> Vec<&'a Pointed> {
    report
        .errors
        .iter()
        .filter(|e| e.first.as_deref().unwrap_or_default() == name)
        .collect()
}

/// An error as text for asking for help: kept in English, as whoever
/// reads it may not read the user's language.
pub fn help_text(report: &LogsReport, error: &Pointed) -> String {
    let from = error
        .first
        .as_ref()
        .map_or(String::new(), |m| format!("Likely mod: {m}\n"));
    let game = report
        .version
        .as_ref()
        .map_or(String::new(), |v| format!("RimWorld {v}\n"));
    format!(
        "{}\n{}\n\n{game}{from}Logged {} times, first at line {} of the log.",
        error.head,
        error.trace.join("\n"),
        error.count,
        error.line
    )
}

impl App {
    /// How many of the game's logs are kept; 10 unless the settings say.
    pub fn logs_kept_limit(&self) -> usize {
        self.settings().logs_kept().unwrap_or(10)
    }

    /// Copies the game's two logs among the kept ones, as far as the limit goes.
    pub fn keep_logs(&self) {
        let paths = self.paths();
        let Some(config) = paths.config.as_ref() else {
            return;
        };
        for log in logs::files(config).iter().take(2).rev() {
            if let Err(problem) = logs::keep(log, &self.kept_logs_dir(), self.logs_kept_limit()) {
                rimstack_core::problem!("log not kept: {problem}");
            }
        }
    }

    /// The last start of the game, from the launches on record, the log
    /// last read, and when the game last wrote one.
    pub fn last_start(&self) -> Option<LastStart> {
        let recorded = launches::read(&self.launches_file()).pop();
        let read = *self.log_read.lock().unwrap_or_else(|e| e.into_inner());
        launches::last_start(recorded.as_ref(), read, self.log_written() / 1000)
    }

    /// A log as the screen shows it: the game's last (or, with `previous`,
    /// the one before), or `kept`, a log RimStack kept. Also says whether
    /// the launches on record, or what is known of the last start, changed.
    pub fn logs_report(
        &self,
        previous: bool,
        kept: Option<&Path>,
    ) -> Result<(LogsReport, bool), Error> {
        let loaded = self.library();
        let (library, _) = loaded.as_ref().as_ref().map_err(Clone::clone)?;
        let paths = self.paths();
        let config = paths
            .config
            .as_ref()
            .ok_or_else(|| Error::refused(said!("RimWorld's settings folder was not found.")))?;
        // Newest first: the last launch's, then the one before's.
        let found = logs::files(config);
        let kept = kept.map(|kept| self.kept_log(kept)).transpose()?;
        let on_last = !previous && kept.is_none();
        let path = kept
            .or_else(|| found.get(usize::from(previous)).cloned())
            .ok_or_else(|| Error::refused(said!("The game has not written a log yet.")))?;
        let text = logs::read(&path)?;
        let written = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());
        let list = self.session().current().to_vec();
        let active = library.arrange(&list).active;
        let mut assemblies: HashMap<String, ModId> = HashMap::new();
        for i in active.iter().filter_map(|m| m.installed) {
            for name in library.get(i).into_iter().flat_map(|m| m.assemblies()) {
                assemblies.entry(name).or_insert(i);
            }
        }
        let ids: Vec<(String, ModId)> = library
            .ids()
            .filter(|(_, m)| m.source != Source::Official)
            .filter_map(|(i, m)| Some((m.about.as_ref().ok()?.package_id.clone(), i)))
            .collect();
        let groups = logs::errors(&text);
        let clues = rimstack_core::mods::blame::Clues {
            assemblies: &assemblies,
            ids: &ids,
        };
        let name_of_mod = |i: ModId| {
            let named = library.get(i).map(|m| m.display_name());
            named.unwrap_or_default().to_string()
        };
        let (errors, culprits) = rimstack_core::mods::blame::of(&groups, &clues, &name_of_mod);
        let file = self.launches_file();
        let mut record = launches::read(&file);
        let mut changed = false;
        // Only the current log says how the newest launch went.
        if !previous && launches::note_outcome(&mut record, written, groups.len()) {
            changed = true;
            if let Err(problem) = launches::write(&file, &record) {
                rimstack_core::problem!("{problem}");
            }
        }
        if on_last {
            let read = Some(LogRead {
                written,
                previous: false,
                errors: groups.len(),
            });
            let mut last = self.log_read.lock().unwrap_or_else(|e| e.into_inner());
            changed |= *last != read;
            *last = read;
        }
        let name = |written: &str| {
            let key = id::split_steam(written).0;
            library.name_of(&key).unwrap_or(key)
        };
        let since_clean = launches::since_clean(&record, &list, &name);
        let shown = launches::shown(&record, &list, &logs::kept(&self.kept_logs_dir()));
        let last_launch = shown.first().map_or(0, |launch| launch.time);
        let report = LogsReport {
            previous,
            has_other: found.len() > 1,
            path: path.to_string_lossy().into_owned(),
            written,
            lines: text.lines().count(),
            version: logs::game_version(&text),
            text: text.lines().take(SHOWN_LINES).map(String::from).collect(),
            errors,
            culprits,
            launches: shown,
            since_clean,
            outside: on_last && written > last_launch + LAUNCH_SLACK_SECS,
        };
        Ok((report, changed))
    }

    /// A kept log, by its path: only one inside the folder of kept logs.
    fn kept_log(&self, path: &Path) -> Result<PathBuf, Error> {
        let dir = self.kept_logs_dir();
        match path.parent() == Some(dir.as_path()) && path.is_file() {
            true => Ok(path.to_path_buf()),
            false => Err(Error::refused(said!(
                "{} is not a kept log",
                path.display()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::tests::installed;

    fn with_log(tmp: &tempfile::TempDir) {
        let config = tmp.path().join("config");
        std::fs::write(
            config.join("Config/ModsConfig.xml"),
            "<ModsConfigData><activeMods><li>ludeon.rimworld</li><li>ann.roads</li></activeMods></ModsConfigData>",
        )
        .unwrap();
        std::fs::write(
            config.join("Player.log"),
            "RimWorld 1.6.4871 rev600\nCould not load texture for Ann.Roads\nCould not load texture for Ann.Roads\nall fine\n",
        )
        .unwrap();
    }

    #[test]
    fn an_error_naming_an_active_mod_is_counted_against_it_and_the_launch_noted() {
        let (tmp, app) = installed();
        with_log(&tmp);
        launches::record(&app.launches_file(), &["ludeon.rimworld".to_string()], 1).unwrap();
        let (report, changed) = app.logs_report(false, None).unwrap();
        assert!(changed);
        let errors = crate::qml::values::as_shown(&listed(&report.errors, "", &HashSet::new()));
        assert_eq!(errors[0]["count"], 2);
        assert_eq!(errors[0]["mod"], "Ann.Roads");
        assert_eq!(errors[0]["kind"], "Could not load texture for Ann.Roads");
        assert_eq!(
            crate::qml::values::as_shown(&report.culprits)[0]["times"],
            2
        );
        let shown = crate::qml::values::as_shown(&launches(&report));
        assert_eq!(shown[0]["errors"], 1);
        assert_eq!(shown[0]["added"], 1);
        assert_eq!(shown[0]["outcome"], "1 error");
        let said = crate::qml::values::as_shown(&head(&report));
        assert_eq!(said["about"], "RimWorld 1.6.4871 rev600, 4 lines");
        assert_eq!(said["outside"], true);
        assert_eq!(said["cut"], "");
        assert_eq!(launches::read(&app.launches_file())[0].errors, Some(1));
        let found = lines(&report, "texture");
        assert_eq!(found.len(), 2);
        assert!(found[0].error_starts);
        assert_eq!(errors_of(&report, "Ann.Roads").len(), 1);
        assert!(help_text(&report, &report.errors[0]).contains("Logged 2 times, first at line 2"));
        assert_eq!(app.last_start().unwrap().errors, Some(1));
        let (_, again) = app.logs_report(false, None).unwrap();
        assert!(!again, "a second read changes nothing");
    }

    #[test]
    fn without_a_log_the_report_says_so_and_only_kept_logs_are_read() {
        let (tmp, app) = installed();
        let problem = app.logs_report(false, None).err().unwrap();
        assert_eq!(problem.kind(), rimstack_core::base::error::Kind::Refused);
        with_log(&tmp);
        let elsewhere = tmp.path().join("config/Player.log");
        let refused = app.logs_report(false, Some(&elsewhere)).err().unwrap();
        assert_eq!(refused.kind(), rimstack_core::base::error::Kind::Refused);
    }

    #[test]
    fn a_kind_is_the_first_line_before_a_short_colon() {
        assert_eq!(kind_of("Exception: boom"), ("Exception", "boom"));
        assert_eq!(kind_of(": none"), (": none", ""));
        let long = format!("{}: tail", "x".repeat(91));
        assert_eq!(kind_of(&long), (long.as_str(), ""));
        assert_eq!(
            breakable(&["a.b(c) ".into(), "d/e".into()]),
            "a.\u{200b}b(\u{200b}c)\nd/\u{200b}e"
        );
    }
}
