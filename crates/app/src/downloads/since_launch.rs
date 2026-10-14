//! What changed in the installed mods since the game was last started
//! from here, and what their authors wrote about it.

use super::*;
use rimstack_core::sources::changes::{self, AtLaunch};

/// How many mods' change notes are asked for in one go.
const MOST_ASKED: usize = 40;
/// How many entries of one mod's notes are shown.
const MOST_NOTES: usize = 4;

fn at_launch_file(app: &crate::App) -> PathBuf {
    app.data_dir().join("at-launch.json")
}

/// Notes the versions on disk now, for the next comparison.
pub(crate) fn record_versions(app: &crate::App) {
    let record = AtLaunch {
        time: now_secs(),
        workshop: steam_record(app)
            .map(|acf| workshop::installed_times(&acf))
            .unwrap_or_default(),
    };
    if let Err(problem) = record.write(&at_launch_file(app)) {
        rimstack_core::problem!("{problem}");
    }
}

/// The first few of a mod's notes: all the screen has room for.
fn shown(found: &[changes::Note]) -> &[changes::Note] {
    &found[..found.len().min(MOST_NOTES)]
}

/// When the game's newest log was written: it writes one at every start.
fn logged(app: &crate::App) -> Option<u64> {
    let paths = app.paths();
    let newest = rimstack_core::rimworld::logs::files(paths.config.as_ref()?)
        .into_iter()
        .next()?;
    let written = std::fs::metadata(newest).ok()?.modified().ok()?;
    Some(
        written
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs(),
    )
}

impl qobject::Downloads {
    /// The time the game was last seen starting, and each mod that
    /// changed since. Read from disk only; a Workshop mod's notes come
    /// with `fetchChangeNotes`.
    pub(super) fn since_launch(&self) -> QString {
        let none = || crate::reports::said(&crate::reports::SinceLaunch::default());
        let loaded = crate::app().library();
        let Ok((library, _)) = &*loaded else {
            return none();
        };
        let now = steam_record(crate::app())
            .map(|acf| workshop::installed_times(&acf))
            .unwrap_or_default();
        let launched = launches::read(&crate::app().launches_file())
            .pop()
            .map(|l| l.time);
        let record = AtLaunch::read(&at_launch_file(crate::app()));
        let last = changes::counted_from(launched, logged(crate::app()));
        let Some((time, updated)) = changes::since(record.as_ref(), last, &now) else {
            return none();
        };

        use rimstack_core::sources::downloads::{changed, items, newest_first, Changed};
        let active = crate::app().active_now(library);
        let mut rows = changed(&items(library, &active), &updated, &now);
        for (mod_id, m) in library.ids().filter(|(_, m)| m.git) {
            let commits = changes::git_notes(&m.path, time);
            if let Some(newest) = commits.first() {
                rows.push(Changed {
                    kind: "git",
                    id: m.path.to_string_lossy().into_owned(),
                    mod_id,
                    name: m.display_name().to_string(),
                    active: active.contains(&mod_id),
                    was: time,
                    now: Some(newest.time),
                    notes: shown(&commits).to_vec(),
                });
            }
        }
        crate::reports::said(&crate::reports::SinceLaunch {
            time,
            items: newest_first(rows),
        })
    }

    /// The newest commits of the Git checkout in this folder, as a JSON
    /// array of `{ time, text }`. Read from the checkout; no network.
    pub(super) fn git_notes(&self, folder: &QString) -> QString {
        let commits = changes::git_notes(std::path::Path::new(&folder.to_string()), 0);
        crate::reports::said(&shown(&commits))
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
        let known = serde_json::from_str(&self.change_notes.to_string());
        let mut found: crate::reports::ChangeNotes = known.unwrap_or_default();
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
                let state = crate::reports::NotesState {
                    done,
                    asked: asked.len(),
                    resume,
                    refused,
                };
                let text = crate::reports::said(&state).to_string();
                let _ = qt_thread.queue(move |mut backend| {
                    backend
                        .as_mut()
                        .set_change_notes_state(QString::from(&text));
                });
            };
            let mut tell = |fetching| match fetching {
                changes::Fetching::Under { done } => state(done, 0, false),
                changes::Fetching::Waiting { done, resume } => state(done, resume, false),
                changes::Fetching::GaveUp { done } => state(done, 0, true),
                changes::Fetching::Fetched { id, notes } => {
                    found.insert(id, notes);
                    let text = crate::reports::said(&found).to_string();
                    let _ = qt_thread.queue(move |mut backend| {
                        backend.as_mut().set_change_notes(QString::from(&text));
                    });
                }
            };
            changes::fetch_notes(
                &asked,
                MOST_NOTES,
                &changes::fetch_steam_notes,
                &std::thread::sleep,
                &now_secs,
                &mut tell,
            );
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_change_notes_busy(false);
            });
        });
    }
}
