//! What Logs shows: the errors of a log with the mods they point at, the
//! launches on record, and what changed since the last clean one.

use super::*;

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
    let set = app.settings().get("logsKept").and_then(|n| n.parse().ok());
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
) -> Value {
    let paths = app.paths();
    let Some(config) = paths.config.as_ref() else {
        return json!({ "problem": say!("RimWorld's settings folder was not found.") });
    };
    // Newest first: the last launch's, then the one before's.
    let found = logs::files(config);
    let Some(path) = kept.or_else(|| found.get(usize::from(previous)).cloned()) else {
        return json!({ "problem": say!("The game has not written a log yet.") });
    };
    let text = match logs::read(&path) {
        Ok(text) => text,
        Err(problem) => return json!({ "problem": problem }),
    };
    let written = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    let written = written
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());

    let list = app.session().current().to_vec();
    let active = library.arrange(&list).active;
    let mut assemblies: HashMap<String, usize> = HashMap::new();
    for i in active.iter().filter_map(|m| m.installed) {
        for name in library.installed[i].assemblies() {
            assemblies.entry(name).or_insert(i);
        }
    }
    let names: Vec<(String, usize)> = active
        .iter()
        .filter_map(|m| m.installed)
        .filter(|&i| library.installed[i].source != rimstack_core::paths::Source::Official)
        .map(|i| (library.installed[i].display_name().to_string(), i))
        .collect();
    let ids: Vec<(String, usize)> = library
        .installed
        .iter()
        .enumerate()
        .filter(|(_, m)| m.source != rimstack_core::paths::Source::Official)
        .filter_map(|(i, m)| Some((m.about.as_ref().ok()?.package_id.clone(), i)))
        .collect();
    let groups = logs::errors(&text);
    let clues = rimstack_core::blame::Clues {
        assemblies: &assemblies,
        names: &names,
        ids: &ids,
    };
    let name_of_mod = |i: usize| library.installed[i].display_name().to_string();
    let (errors, culprits) = rimstack_core::blame::of(&groups, &clues, &name_of_mod);

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
    json!({
        "previous": previous,
        "hasOther": found.len() > 1,
        "path": path.to_string_lossy(),
        "written": written,
        "lines": text.lines().count(),
        "version": logs::game_version(&text),
        "text": text.lines().take(SHOWN_LINES).collect::<Vec<_>>(),
        "errors": errors,
        "culprits": culprits,
        "launches": shown,
        "sinceClean": since_clean,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_naming_an_active_mod_is_counted_against_it_and_the_launch_noted() {
        let (tmp, app) = crate::state::tests::installed();
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
        let report = logs_report(&app, library, false, None);
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
        let (_tmp, app) = crate::state::tests::installed();
        let loaded = app.library();
        let (library, _) = loaded.as_ref().as_ref().unwrap();
        assert!(logs_report(&app, library, false, None)["problem"].is_string());
    }
}
