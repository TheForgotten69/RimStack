//! What Logs shows: the errors of a log with the mods they point at, the
//! launches on record, and what changed since the last clean one.

use super::*;
use rimstack_core::base::error::Error;
use rimstack_core::said;

/// How many lines of a log the Logs screen is given to show.
const SHOWN_LINES: usize = 30_000;

pub(crate) fn launches_file(app: &crate::App) -> PathBuf {
    app.data_dir().join("launches.json")
}

pub(super) fn kept_logs_dir(app: &crate::App) -> PathBuf {
    app.data_dir().join("logs")
}

/// How many of the game's logs are kept; 10 unless the settings say.
pub(super) fn logs_kept_limit(app: &crate::App) -> usize {
    let set = app.settings().logs_kept();
    set.unwrap_or(10)
}

/// Copies the game's two logs among the kept ones, as far as the limit goes.
pub(crate) fn keep_logs(app: &crate::App) {
    let paths = app.paths();
    let Some(config) = paths.config.as_ref() else {
        return;
    };
    for log in logs::files(config).iter().take(2).rev() {
        if let Err(problem) = logs::keep(log, &kept_logs_dir(app), logs_kept_limit(app)) {
            rimstack_core::problem!("log not kept: {problem}");
        }
    }
}

/// `kept`: a kept log to read in place of the game's last or previous one.
pub(crate) fn logs_report(
    app: &crate::App,
    library: &Library,
    previous: bool,
    kept: Option<PathBuf>,
) -> Result<crate::reports::LogsReport, Error> {
    let paths = app.paths();
    let config = paths
        .config
        .as_ref()
        .ok_or_else(|| Error::refused(said!("RimWorld's settings folder was not found.")))?;
    // Newest first: the last launch's, then the one before's.
    let found = logs::files(config);
    let path = kept
        .or_else(|| found.get(usize::from(previous)).cloned())
        .ok_or_else(|| Error::refused(said!("The game has not written a log yet.")))?;
    let text = logs::read(&path)?;
    let written = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    let written = written
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());

    let list = app.session().current().to_vec();
    let active = library.arrange(&list).active;
    let mut assemblies: HashMap<String, ModId> = HashMap::new();
    for i in active.iter().filter_map(|m| m.installed) {
        for name in library.get(i).into_iter().flat_map(|m| m.assemblies()) {
            assemblies.entry(name).or_insert(i);
        }
    }
    let ids: Vec<(String, ModId)> = library
        .ids()
        .filter(|(_, m)| m.source != rimstack_core::rimworld::paths::Source::Official)
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

    let file = launches_file(app);
    let mut record = launches::read(&file);
    // Only the current log says how the newest launch went.
    if !previous && launches::note_outcome(&mut record, written, groups.len()) {
        if let Err(problem) = launches::write(&file, &record) {
            rimstack_core::problem!("{problem}");
        }
    }
    let name = |written: &str| {
        let key = id::split_steam(written).0;
        library.name_of(&key).unwrap_or(key)
    };
    let since_clean = launches::since_clean(&record, &list, &name);
    let shown = launches::shown(&record, &list, &logs::kept(&kept_logs_dir(app)));
    Ok(crate::reports::LogsReport {
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_naming_an_active_mod_is_counted_against_it_and_the_launch_noted() {
        let (tmp, app) = crate::workspace::tests::installed();
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
        launches::record(&launches_file(&app), &["ludeon.rimworld".to_string()], 1).unwrap();

        let loaded = app.library();
        let (library, _) = loaded.as_ref().as_ref().unwrap();
        let report =
            serde_json::to_value(logs_report(&app, library, false, None).unwrap()).unwrap();
        assert_eq!(report["version"], "1.6.4871 rev600");
        assert_eq!(report["errors"][0]["count"], 2);
        assert_eq!(report["errors"][0]["mod"], "Ann.Roads");
        assert_eq!(report["culprits"][0]["times"], 2);
        // The launch on record now says how it went, and what the list gained since.
        assert_eq!(report["launches"][0]["errors"], 1);
        assert_eq!(report["launches"][0]["added"], 1);
        assert_eq!(launches::read(&launches_file(&app))[0].errors, Some(1));
    }

    #[test]
    fn without_a_log_the_report_says_so() {
        let (_tmp, app) = crate::workspace::tests::installed();
        let loaded = app.library();
        let (library, _) = loaded.as_ref().as_ref().unwrap();
        let problem = logs_report(&app, library, false, None).err().unwrap();
        assert_eq!(problem.kind(), rimstack_core::base::error::Kind::Refused);
    }
}
