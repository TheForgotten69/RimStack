//! What Downloads shows: updates Steam has, and what pasted links are.

use super::*;

/// The installed Workshop mods by Workshop id (their folder name).
pub(crate) fn steam_record(app: &crate::App) -> Result<String, rimstack_core::error::Error> {
    let record = app.paths().steam_record().ok_or_else(|| {
        rimstack_core::error::Error::refused(say!("The Workshop folder was not found."))
    })?;
    std::fs::read_to_string(&record).map_err(|e| rimstack_core::error::Error::file(&record, e))
}

fn checked_file(app: &crate::App) -> PathBuf {
    app.data_dir().join("updates.json")
}

/// What the last check learnt, kept between runs: `{ checked, asked,
/// now: { Workshop id: time updated }, checkouts: { folder: { behind,
/// problem } } }`. An empty object before the first check.
pub(crate) fn last_check(app: &crate::App) -> Value {
    let text = std::fs::read_to_string(checked_file(app)).unwrap_or_default();
    let kept = serde_json::from_str::<Value>(&text).ok();
    kept.filter(Value::is_object).unwrap_or_else(|| json!({}))
}

/// Asks Steam and each checkout's remote, keeps what they said and
/// returns it. Steam not reached: what it said last time stays, and the
/// reason comes back beside it.
pub(crate) fn check(app: &crate::App, library: &Library) -> (Value, Option<String>) {
    let mut kept = last_check(app);
    let ids: Vec<String> = library
        .workshop_mods()
        .keys()
        .map(|id| id.to_string())
        .collect();
    let problem = match workshop::fetch_details(&ids) {
        Ok(now) => {
            let times: serde_json::Map<String, Value> = now
                .iter()
                .map(|d| (d.id.clone(), json!(d.updated)))
                .collect();
            kept["checked"] = json!(now_secs());
            kept["asked"] = json!(ids.len());
            kept["now"] = Value::Object(times);
            None
        }
        Err(problem) => Some(problem.into()),
    };
    kept["checkouts"] = checkouts_asked(app, library);
    if let Err(e) = std::fs::write(checked_file(app), kept.to_string()) {
        rimstack_core::problem!("{}: {e}", checked_file(app).display());
    }
    (kept, problem)
}

/// A checkout just brought up to its remote is no longer behind: what
/// the last check said of it would otherwise stand until the next one.
pub(crate) fn checkout_updated(app: &crate::App, folder: &std::path::Path) {
    let mut kept = last_check(app);
    kept["checkouts"][folder.to_string_lossy().as_ref()] = json!({ "behind": 0 });
    if let Err(e) = std::fs::write(checked_file(app), kept.to_string()) {
        rimstack_core::problem!("{}: {e}", checked_file(app).display());
    }
}

/// What Downloads shows of a check, against the mods as they are on disk
/// now: a mod Steam has updated since is no longer behind.
pub(crate) fn updates_report(app: &crate::App, library: &Library, kept: &Value) -> Value {
    let on_disk = steam_record(app)
        .map(|acf| workshop::installed_times(&acf))
        .unwrap_or_default();
    let now: HashMap<String, u64> = kept["now"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(id, updated)| Some((id.clone(), updated.as_u64()?)))
        .collect();
    let active = app.active_now(library);
    let items = rimstack_core::downloads::items(library, &active);
    let behind = rimstack_core::downloads::behind(&items, &on_disk, &now);
    if kept["checked"].is_u64() {
        *app.behind().writing() = Some(behind.iter().map(|row| row.id.clone()).collect());
    }
    json!({
        "checked": kept["checked"], "asked": kept["asked"], "answered": now.len(),
        "items": behind, "checkouts": checkouts_report(app, library, &kept["checkouts"]),
    })
}

pub(crate) fn queue_report(
    app: &crate::App,
    library: &Library,
    pasted: &str,
) -> Result<Value, rimstack_core::error::Error> {
    use rimstack_core::downloads;
    let ids = workshop::ids_in(pasted);
    let local = app.paths().local_mods();
    let repositories = downloads::repositories(pasted, local.as_deref());
    if ids.is_empty() {
        let note = match repositories.is_empty() {
            true => say!("No Workshop link, mod id or Git link found"),
            false => String::new(),
        };
        return Ok(json!({ "items": [], "repositories": repositories, "note": note }));
    }
    // A collection's link stands for the mods in it.
    let ids = workshop::expand_collections(&ids)?;
    let answered = workshop::fetch_details(&ids)?;
    let mods = library.workshop_mods();
    let items = downloads::asked(&ids, &answered, &|id| mods.contains_key(id));
    Ok(json!({ "items": items, "repositories": repositories }))
}

fn asked_of_git<'a>(
    app: &crate::App,
    library: &'a Library,
) -> Vec<(usize, &'a rimstack_core::library::Installed)> {
    let git_for_workshop = app.settings().get("gitForWorkshop") == Some("on");
    let mods = library.installed.iter().enumerate();
    mods.filter(|(_, m)| rimstack_core::updates::asks_git(m.source, m.git, git_for_workshop))
        .collect()
}

/// Asks the remote of each checkout whose updates come from Git: per
/// folder, how far it is behind or why that could not be told.
fn checkouts_asked(app: &crate::App, library: &Library) -> Value {
    use rayon::prelude::*;
    let answers: Vec<(String, Value)> = asked_of_git(app, library)
        .par_iter()
        .map(|(_, m)| {
            let said = match rimstack_core::git::behind(&m.path) {
                Ok(count) => json!({ "behind": count }),
                Err(problem) => json!({ "problem": problem }),
            };
            (m.path.to_string_lossy().into_owned(), said)
        })
        .collect();
    Value::Object(answers.into_iter().collect())
}

/// The checkouts whose updates come from Git, each with what its remote
/// said at the last check; nothing yet for one not asked then.
fn checkouts_report(app: &crate::App, library: &Library, answers: &Value) -> Value {
    let rows = asked_of_git(app, library).into_iter().map(|(id, m)| {
        let path = m.path.to_string_lossy();
        let said = &answers[path.as_ref()];
        json!({
            "modId": id,
            "name": m.display_name(),
            "path": path,
            "workshop": m.source == Source::Workshop,
            "checked": said.is_object(),
            "behind": said["behind"],
            "problem": said["problem"],
        })
    });
    Value::Array(rows.collect())
}
