//! What Downloads shows: updates Steam has, and what pasted links are.

use super::*;

/// The installed Workshop mods by Workshop id (their folder name).
pub(super) fn workshop_mods(library: &Library) -> HashMap<&str, usize> {
    let workshop = library.installed.iter().enumerate();
    workshop
        .filter(|(_, m)| m.source == Source::Workshop)
        .map(|(i, m)| (m.folder.as_str(), i))
        .collect()
}

pub(super) fn steam_record() -> Result<String, String> {
    let content = crate::PATHS
        .workshop
        .as_ref()
        .ok_or_else(|| say!("The Workshop folder was not found."))?;
    // <library>/steamapps/workshop/content/294100 -> .../workshop/appworkshop_294100.acf
    let record = content
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("appworkshop_294100.acf"))
        .ok_or_else(|| say!("The Workshop folder was not found."))?;
    std::fs::read_to_string(&record).map_err(|e| format!("{}: {e}", record.display()))
}

fn checked_file() -> PathBuf {
    crate::data_dir().join("updates.json")
}

/// What the last check learnt, kept between runs: `{ checked, asked,
/// now: { Workshop id: time updated }, checkouts: { folder: { behind,
/// problem } } }`. An empty object before the first check.
pub(super) fn last_check() -> Value {
    let text = std::fs::read_to_string(checked_file()).unwrap_or_default();
    let kept = serde_json::from_str::<Value>(&text).ok();
    kept.filter(Value::is_object).unwrap_or_else(|| json!({}))
}

/// Asks Steam and each checkout's remote, keeps what they said and
/// returns it. Steam not reached: what it said last time stays, and the
/// reason comes back beside it.
pub(super) fn check(library: &Library) -> (Value, Option<String>) {
    let mut kept = last_check();
    let ids: Vec<String> = workshop_mods(library)
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
        Err(problem) => Some(problem),
    };
    kept["checkouts"] = checkouts_asked(library);
    if let Err(e) = std::fs::write(checked_file(), kept.to_string()) {
        eprintln!("{}: {e}", checked_file().display());
    }
    (kept, problem)
}

/// A checkout just brought up to its remote is no longer behind: what
/// the last check said of it would otherwise stand until the next one.
pub(super) fn checkout_updated(folder: &std::path::Path) {
    let mut kept = last_check();
    kept["checkouts"][folder.to_string_lossy().as_ref()] = json!({ "behind": 0 });
    if let Err(e) = std::fs::write(checked_file(), kept.to_string()) {
        eprintln!("{}: {e}", checked_file().display());
    }
}

/// What Downloads shows of a check, against the mods as they are on disk
/// now: a mod Steam has updated since is no longer behind.
pub(super) fn updates_report(library: &Library, kept: &Value) -> Value {
    let on_disk = steam_record()
        .map(|acf| workshop::installed_times(&acf))
        .unwrap_or_default();
    let mods = workshop_mods(library);
    let times = kept["now"].as_object();
    let now: Vec<workshop::Details> = times
        .into_iter()
        .flatten()
        .filter_map(|(id, updated)| {
            Some(workshop::Details {
                id: id.clone(),
                title: String::new(),
                updated: updated.as_u64()?,
                size: 0,
            })
        })
        .collect();
    let active: std::collections::HashSet<usize> = {
        let list = crate::session().current().to_vec();
        let arranged = library.arrange(&list);
        arranged.active.iter().filter_map(|m| m.installed).collect()
    };
    let behind = workshop::updates(&on_disk, &now);
    if kept["checked"].is_u64() {
        *crate::BEHIND.write().unwrap_or_else(|e| e.into_inner()) =
            Some(behind.iter().map(|d| d.id.clone()).collect());
    }
    let mut items: Vec<Value> = behind
        .into_iter()
        .filter_map(|d| {
            let i = *mods.get(d.id.as_str())?;
            Some(json!({
                "id": d.id,
                "modId": i,
                "name": library.installed[i].display_name(),
                "active": active.contains(&i),
                "have": on_disk.get(&d.id),
                "now": d.updated,
            }))
        })
        .collect();
    items.sort_by_key(|v| {
        (
            !v["active"].as_bool().unwrap_or(false),
            std::cmp::Reverse(v["now"].as_u64()),
        )
    });
    json!({
        "checked": kept["checked"], "asked": kept["asked"], "answered": now.len(),
        "items": items, "checkouts": checkouts_report(library, &kept["checkouts"]),
    })
}

pub(super) fn queue_report(library: &Library, pasted: &str) -> Result<Value, String> {
    let ids = workshop::ids_in(pasted);
    let local = crate::PATHS.game.as_ref().map(|game| game.join("Mods"));
    let repositories: Vec<Value> = rimstack_core::git::links_in(pasted)
        .iter()
        .map(|link| {
            let into = local
                .as_ref()
                .map(|mods| rimstack_core::git::folder_for(link, mods));
            json!({
                "link": link,
                "name": link.split('/').skip(3).collect::<Vec<_>>().join("/"),
                "into": into.as_ref().map(|p| p.to_string_lossy()),
                "installed": into.is_some_and(|p| p.exists()),
            })
        })
        .collect();
    if ids.is_empty() {
        let note = match repositories.is_empty() {
            true => say!("No Workshop link, mod id or Git link found"),
            false => String::new(),
        };
        return Ok(json!({ "items": [], "repositories": repositories, "note": note }));
    }
    // A collection's link stands for the mods in it.
    let ids = workshop::expand_collections(&ids)?;
    let found = workshop::fetch_details(&ids)?;
    let mods = workshop_mods(library);
    let items: Vec<Value> = ids
        .iter()
        .map(|id| {
            let details = found.iter().find(|d| &d.id == id);
            json!({
                "id": id,
                                "title": details.map(|d| d.title.as_str()),
                "updated": details.map(|d| d.updated),
                "megabytes": details.map(|d| d.size as f64 / 1_048_576.0),
                "installed": mods.contains_key(id.as_str()),
            })
        })
        .collect();
    Ok(json!({ "items": items, "repositories": repositories }))
}

fn asked_of_git(library: &Library) -> Vec<(usize, &rimstack_core::library::Installed)> {
    let git_for_workshop = crate::settings().get("gitForWorkshop") == Some("on");
    let mods = library.installed.iter().enumerate();
    mods.filter(|(_, m)| rimstack_core::updates::asks_git(m.source, m.git, git_for_workshop))
        .collect()
}

/// Asks the remote of each checkout whose updates come from Git: per
/// folder, how far it is behind or why that could not be told.
fn checkouts_asked(library: &Library) -> Value {
    use rayon::prelude::*;
    let answers: Vec<(String, Value)> = asked_of_git(library)
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
fn checkouts_report(library: &Library, answers: &Value) -> Value {
    let rows = asked_of_git(library).into_iter().map(|(id, m)| {
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
