//! What changed in the installed mods since the game was last started
//! from here, and what their authors wrote about it.

use super::*;
use rimstack_core::changes::{self, AtLaunch};

/// How many mods' change notes are asked for in one go.
const MOST_ASKED: usize = 40;
/// How many entries of one mod's notes are shown.
const MOST_NOTES: usize = 4;

fn at_launch_file() -> PathBuf {
    crate::data_dir().join("at-launch.json")
}

/// Notes the versions on disk now, for the next comparison.
pub(super) fn record_versions() {
    let record = AtLaunch {
        time: now(),
        workshop: steam_record()
            .map(|acf| workshop::installed_times(&acf))
            .unwrap_or_default(),
    };
    if let Err(problem) = record.write(&at_launch_file()) {
        eprintln!("{problem}");
    }
}

fn notes(found: &[changes::Note]) -> Vec<Value> {
    let shown = found.iter().take(MOST_NOTES);
    shown
        .map(|n| json!({ "time": n.time, "text": n.text }))
        .collect()
}

impl qobject::Backend {
    /// The game was seen starting, whoever started it: what it loads now
    /// is what later changes are counted from.
    pub(super) fn note_game_started(&self) {
        record_versions();
    }

    /// `{ time, items }`: the time the game was last seen starting, and each mod that
    /// changed since as `{ kind: "workshop" or "git", id, modId, name,
    /// active, was, now, notes }`. Read from disk only; a Workshop mod's
    /// notes come with `fetchChangeNotes`.
    pub(super) fn since_launch(&self) -> QString {
        let none = || QString::from(&json!({ "time": 0, "items": [] }).to_string());
        let Ok((library, _)) = crate::library() else {
            return none();
        };
        let now = steam_record()
            .map(|acf| workshop::installed_times(&acf))
            .unwrap_or_default();
        // Without a record of the versions, the time of the launch says
        // which were published since.
        let launched = launches::read(&launches_file()).pop().map(|l| l.time);
        let logged = crate::PATHS.config.as_ref().and_then(|config| {
            let newest = rimstack_core::logs::files(config).into_iter().next()?;
            let written = std::fs::metadata(newest).ok()?.modified().ok()?;
            Some(
                written
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?
                    .as_secs(),
            )
        });
        let last = changes::counted_from(launched, logged);
        let (time, updated) = match (AtLaunch::read(&at_launch_file()), last) {
            (Some(record), _) => (record.time, record.updated(&now)),
            (None, Some(time)) => {
                let since = now.iter().filter(|(_, at)| **at > time);
                (time, since.map(|(id, _)| (id.clone(), time)).collect())
            }
            (None, None) => return none(),
        };

        let active: std::collections::HashSet<usize> = {
            let list = crate::session().current().to_vec();
            let arranged = library.arrange(&list);
            arranged.active.iter().filter_map(|m| m.installed).collect()
        };
        let mods = workshop_mods(library);
        let mut items: Vec<Value> = updated
            .iter()
            .filter_map(|(id, was)| {
                let i = *mods.get(id.as_str())?;
                Some(json!({
                    "kind": "workshop", "id": id, "modId": i,
                    "name": library.installed[i].display_name(),
                    "active": active.contains(&i), "was": was, "now": now.get(id), "notes": [],
                }))
            })
            .collect();
        for (i, m) in library.installed.iter().enumerate().filter(|(_, m)| m.git) {
            let commits = changes::git_notes(&m.path, time);
            if let Some(newest) = commits.first() {
                items.push(json!({
                    "kind": "git", "id": m.path.to_string_lossy(), "modId": i,
                    "name": m.display_name(), "active": active.contains(&i),
                    "was": time, "now": newest.time, "notes": notes(&commits),
                }));
            }
        }
        items.sort_by_key(|v| {
            (
                !v["active"].as_bool().unwrap_or(false),
                std::cmp::Reverse(v["now"].as_u64()),
            )
        });
        QString::from(&json!({ "time": time, "items": items }).to_string())
    }

    /// The newest commits of the Git checkout in this folder, as a JSON
    /// array of `{ time, text }`. Read from the checkout; no network.
    pub(super) fn git_notes(&self, folder: &QString) -> QString {
        let commits = changes::git_notes(std::path::Path::new(&folder.to_string()), 0);
        QString::from(&Value::Array(notes(&commits)).to_string())
    }

    /// Asks Steam's site for the change notes of these Workshop mods, in
    /// the background: `asked` is a JSON array of `[id, was]`, and
    /// `changeNotes` gains `{ id: [{ time, text }] }` with each mod's
    /// entries newer than `was`, or its newest one. What was fetched
    /// before for other mods stays.
    pub(super) fn fetch_change_notes(mut self: Pin<&mut Self>, asked: &QString) {
        if self.change_notes_busy {
            return;
        }
        let asked: Vec<(String, u64)> =
            serde_json::from_str(&asked.to_string()).unwrap_or_default();
        self.as_mut().set_change_notes_busy(true);
        let known = serde_json::from_str::<Value>(&self.change_notes.to_string()).ok();
        let mut found = match known {
            Some(Value::Object(known)) => known,
            _ => serde_json::Map::new(),
        };
        // Asked for many at once, what this run already read is not asked again.
        let several = asked.len() > 1;
        let asked: Vec<(String, u64)> = asked
            .into_iter()
            .filter(|(id, _)| !(several && found.contains_key(id)))
            .take(MOST_ASKED)
            .collect();
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let state = |done: usize, resume: u64, refused: bool| {
                let text = json!({ "done": done, "asked": asked.len(), "resume": resume, "refused": refused }).to_string();
                let _ = qt_thread.queue(move |mut backend| {
                    backend
                        .as_mut()
                        .set_change_notes_state(QString::from(&text));
                });
            };
            let mut refusals = 0;
            let mut at = 0;
            state(0, 0, false);
            while let Some((id, was)) = asked.get(at) {
                let said = match changes::fetch_steam_notes(id) {
                    Ok(all) => {
                        let newer: Vec<changes::Note> =
                            all.iter().filter(|n| n.time > *was).cloned().collect();
                        let shown = if newer.is_empty() {
                            &all[..all.len().min(1)]
                        } else {
                            &newer[..]
                        };
                        json!(notes(shown))
                    }
                    Err(changes::NotesProblem::Refused { wait }) => {
                        refusals += 1;
                        let Some(pause) = changes::pause_after_refusal(refusals, wait) else {
                            state(at, 0, true);
                            break;
                        };
                        state(at, now_secs() + pause, false);
                        std::thread::sleep(std::time::Duration::from_secs(pause));
                        continue;
                    }
                    Err(changes::NotesProblem::Failed(problem)) => {
                        json!([{ "time": 0, "text": problem }])
                    }
                };
                refusals = 0;
                at += 1;
                found.insert(id.clone(), said);
                let text = Value::Object(found.clone()).to_string();
                let _ = qt_thread.queue(move |mut backend| {
                    backend.as_mut().set_change_notes(QString::from(&text));
                });
                state(at, 0, false);
                // One at a time with a pause: Steam's site answers 429 to a
                // handful of requests within a few seconds (seen 2026-10-12).
                if at < asked.len() {
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                }
            }
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_change_notes_busy(false);
            });
        });
    }
}
