//! What Logs shows: the errors of a log with the mods they point at, the
//! launches on record, and what changed since the last clean one.

use super::*;

/// How many lines of a log the Logs screen is given to show.
const SHOWN_LINES: usize = 30_000;

pub(super) fn launches_file() -> PathBuf {
    crate::data_dir().join("launches.json")
}

pub(super) fn kept_logs_dir() -> PathBuf {
    crate::data_dir().join("logs")
}

/// How many of the game's logs are kept; 10 unless the settings say.
pub(super) fn logs_kept_limit() -> usize {
    let set = crate::settings()
        .get("logsKept")
        .and_then(|n| n.parse().ok());
    set.unwrap_or(10)
}

/// Copies the game's two logs among the kept ones, as far as the limit goes.
pub(super) fn keep_logs() {
    let Some(config) = crate::PATHS.config.as_ref() else {
        return;
    };
    for log in logs::files(config).iter().take(2).rev() {
        if let Err(problem) = logs::keep(log, &kept_logs_dir(), logs_kept_limit()) {
            eprintln!("log not kept: {problem}");
        }
    }
}

/// `kept`: a kept log to read in place of the game's last or previous one.
pub(super) fn logs_report(library: &Library, previous: bool, kept: Option<PathBuf>) -> Value {
    let Some(config) = crate::PATHS.config.as_ref() else {
        return json!({ "problem": "RimWorld's settings folder was not found." });
    };
    // Newest first: the last launch's, then the one before's.
    let found = logs::files(config);
    let Some(path) = kept.or_else(|| found.get(usize::from(previous)).cloned()) else {
        return json!({ "problem": "The game has not written a log yet." });
    };
    let text = match logs::read(&path) {
        Ok(text) => text,
        Err(problem) => return json!({ "problem": problem }),
    };
    let written = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    let written = written
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());

    let list = crate::session().current().to_vec();
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
    let runs = logs::runs(&groups);
    let errors: Vec<Value> = groups
        .iter()
        .enumerate()
        .map(|(at, group)| {
            let mut all = logs::mods_in(group, &assemblies);
            if all.is_empty() {
                all.extend(logs::mentioned(&group.head, &names));
            }
            // The game names a mod by its package id where it has not loaded it.
            if all.is_empty() {
                all.extend(logs::mentioned(&group.head, &ids));
            }
            let from = all.first().copied();
            let named: Vec<Value> = all
                .iter()
                .map(|&i| json!({ "name": library.installed[i].display_name(), "modId": i }))
                .collect();
            json!({
                "mods": named,
                "head": group.head,
                "trace": group.trace,
                "count": group.count,
                "line": group.line,
                // Where its run of alike errors begins, and how long that is.
                "at": at,
                "run": runs[at],
                "alike": runs.iter().filter(|&&start| start == runs[at]).count(),
                "mod": from.map(|i| library.installed[i].display_name()),
                "modId": from.map_or(-1, |i| i as i64),
            })
        })
        .collect();

    let file = launches_file();
    let mut record = launches::read(&file);
    // Only the current log says how the newest launch went.
    if !previous && launches::note_outcome(&mut record, written, groups.len()) {
        if let Err(problem) = launches::write(&file, &record) {
            eprintln!("{problem}");
        }
    }
    let name = |written: &String| {
        let key = id::split_steam(written).0;
        library.name_of(&key).unwrap_or(key)
    };
    let since_clean = launches::last_clean(&record).map(|clean| {
        let difference = saves::compare(&clean.ids, &list);
        json!({
            "time": clean.time,
            "ids": clean.ids,
            "removed": difference.missing.iter().map(|&i| name(&clean.ids[i])).collect::<Vec<_>>(),
            "added": difference.added.iter().map(|&i| name(&list[i])).collect::<Vec<_>>(),
            "reordered": difference.reordered,
        })
    });
    let kept_logs = logs::kept(&kept_logs_dir());
    let shown: Vec<Value> = record
        .iter()
        .enumerate()
        .rev()
        .map(|(at, l)| {
            let difference = saves::compare(&l.ids, &list);
            let next = record.get(at + 1).map(|after| after.time);
            let log = logs::kept_for(&kept_logs, l.time, next);
            json!({
                "log": log.map(|path| path.to_string_lossy()),
                "time": l.time,
                "mods": l.ids.len(),
                "errors": l.errors,
                "ids": l.ids,
                "removed": difference.missing.len(),
                "added": difference.added.len(),
                "reordered": difference.reordered,
            })
        })
        .collect();
    json!({
        "previous": previous,
        "hasOther": found.len() > 1,
        "path": path.to_string_lossy(),
        "written": written,
                "lines": text.lines().count(),
        "version": logs::game_version(&text),
        "text": text.lines().take(SHOWN_LINES).collect::<Vec<_>>(),
        "errors": errors,
        "launches": shown,
        "sinceClean": since_clean,
    })
}
