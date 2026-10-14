//! What Downloads shows: updates Steam has, and what pasted links are.

use crate::workspace::now_secs;
use rimstack_core::base::locks::Shared;
use rimstack_core::kept::settings::Switch;
use rimstack_core::kept::updates::{LastCheck, Remote};
use rimstack_core::mods::library::{Library, ModId};
use rimstack_core::rimworld::{paths::Source, workshop};
use rimstack_core::said;
use rimstack_core::say;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;

/// The installed Workshop mods by Workshop id (their folder name).
pub(crate) fn steam_record(app: &crate::App) -> Result<String, rimstack_core::base::error::Error> {
    let record = app.paths().steam_record().ok_or_else(|| {
        rimstack_core::base::error::Error::refused(said!("The Workshop folder was not found."))
    })?;
    std::fs::read_to_string(&record)
        .map_err(|e| rimstack_core::base::error::Error::file(&record, e))
}

fn checked_file(app: &crate::App) -> PathBuf {
    app.data_dir().join("updates.json")
}

pub(crate) fn last_check(app: &crate::App) -> LastCheck {
    LastCheck::read(&checked_file(app))
}

fn keep_check(app: &crate::App, check: &LastCheck) {
    if let Err(problem) = check.write(&checked_file(app)) {
        rimstack_core::problem!("{problem}");
    }
}

/// Asks Steam and each checkout's remote, keeps what they said and
/// returns it. Steam not reached: what it said last time stays, and the
/// reason comes back beside it.
pub(crate) fn check(app: &crate::App, library: &Library) -> (LastCheck, Option<String>) {
    let mut kept = last_check(app);
    let ids: Vec<String> = library
        .workshop_mods()
        .keys()
        .map(|id| id.to_string())
        .collect();
    let problem = match workshop::fetch_details(&ids) {
        Ok(now) => {
            kept.checked = Some(now_secs());
            kept.asked = Some(ids.len() as u64);
            kept.now = now.iter().map(|d| (d.id.clone(), d.updated)).collect();
            None
        }
        Err(problem) => Some(problem.into()),
    };
    kept.checkouts = checkouts_asked(app, library);
    keep_check(app, &kept);
    (kept, problem)
}

/// A checkout just brought up to its remote is no longer behind: what
/// the last check said of it would otherwise stand until the next one.
pub(crate) fn checkout_updated(app: &crate::App, folder: &std::path::Path) {
    let mut kept = last_check(app);
    let caught_up = Remote {
        behind: Some(0),
        problem: None,
    };
    kept.checkouts
        .insert(folder.to_string_lossy().into_owned(), caught_up);
    keep_check(app, &kept);
}

/// What Downloads shows of a check, against the mods as they are on disk
/// now: a mod Steam has updated since is no longer behind.
pub(crate) fn updates_report(
    app: &crate::App,
    library: &Library,
    kept: &LastCheck,
) -> crate::reports::Updates {
    let on_disk = steam_record(app)
        .map(|acf| workshop::installed_times(&acf))
        .unwrap_or_default();
    let now: HashMap<String, u64> = kept.now.clone().into_iter().collect();
    let active = app.active_now(library);
    let items = rimstack_core::sources::downloads::items(library, &active);
    let behind = rimstack_core::sources::downloads::behind(&items, &on_disk, &now);
    if kept.checked.is_some() {
        *app.behind().writing() = Some(behind.iter().map(|row| row.id.clone()).collect());
    }
    crate::reports::Updates {
        checked: kept.checked,
        asked: kept.asked,
        answered: now.len(),
        items: behind,
        checkouts: checkouts_report(app, library, &kept.checkouts),
        problem: None,
    }
}

pub(crate) fn queue_report(
    app: &crate::App,
    library: &Library,
    pasted: &str,
) -> Result<crate::reports::Queue, rimstack_core::base::error::Error> {
    use rimstack_core::sources::downloads;
    let ids = workshop::ids_in(pasted);
    let local = app.paths().local_mods();
    let repositories = downloads::repositories(pasted, local.as_deref());
    if ids.is_empty() {
        let note = match repositories.is_empty() {
            true => say!("No Workshop link, mod id or Git link found"),
            false => String::new(),
        };
        return Ok(crate::reports::Queue {
            items: Vec::new(),
            repositories,
            note: Some(note),
        });
    }
    // A collection's link stands for the mods in it.
    let ids = workshop::expand_collections(&ids)?;
    let answered = workshop::fetch_details(&ids)?;
    let mods = library.workshop_mods();
    let items = downloads::asked(&ids, &answered, &|id| mods.contains_key(id));
    Ok(crate::reports::Queue {
        items,
        repositories,
        note: None,
    })
}

fn asked_of_git<'a>(
    app: &crate::App,
    library: &'a Library,
) -> Vec<(ModId, &'a rimstack_core::mods::library::Installed)> {
    let git_for_workshop = app.settings().switch(Switch::GitForWorkshop);
    let mods = library.ids();
    mods.filter(|(_, m)| rimstack_core::kept::updates::asks_git(m.source, m.git, git_for_workshop))
        .collect()
}

/// Asks the remote of each checkout whose updates come from Git: per
/// folder, how far it is behind or why that could not be told.
fn checkouts_asked(app: &crate::App, library: &Library) -> BTreeMap<String, Remote> {
    use rayon::prelude::*;
    let answers: Vec<(String, Remote)> = asked_of_git(app, library)
        .par_iter()
        .map(|(_, m)| {
            let said = match rimstack_core::sources::git::behind(&m.path) {
                Ok(count) => Remote {
                    behind: Some(count as u64),
                    problem: None,
                },
                Err(problem) => Remote {
                    behind: None,
                    problem: Some(problem.into()),
                },
            };
            (m.path.to_string_lossy().into_owned(), said)
        })
        .collect();
    answers.into_iter().collect()
}

/// The checkouts whose updates come from Git, each with what its remote
/// said at the last check; nothing yet for one not asked then.
fn checkouts_report(
    app: &crate::App,
    library: &Library,
    answers: &BTreeMap<String, Remote>,
) -> Vec<crate::reports::Checkout> {
    let rows = asked_of_git(app, library).into_iter().map(|(id, m)| {
        let path = m.path.to_string_lossy().into_owned();
        let said = answers.get(&path);
        crate::reports::Checkout {
            mod_id: id,
            name: m.display_name().to_string(),
            workshop: m.source == Source::Workshop,
            checked: said.is_some(),
            behind: said.and_then(|s| s.behind),
            problem: said.and_then(|s| s.problem.clone()),
            path,
        }
    });
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_local_checkout_is_listed_by_its_mod_with_what_its_remote_said_last() {
        let (tmp, app) = crate::workspace::tests::installed();
        let roads = tmp.path().join("game/Mods/roads");
        std::fs::create_dir(roads.join(".git")).unwrap();
        app.rescan();
        let loaded = app.library();
        let (library, _) = loaded.as_ref().as_ref().unwrap();
        let id = library.ids().find(|(_, m)| m.folder == "roads").unwrap().0;

        let report = updates_report(&app, library, &LastCheck::default());
        assert_eq!(report.checkouts.len(), 1);
        let row = &report.checkouts[0];
        assert_eq!((row.mod_id, row.checked, row.behind), (id, false, None));

        let mut kept = LastCheck::default();
        let remote = Remote {
            behind: Some(2),
            problem: None,
        };
        kept.checkouts
            .insert(roads.to_string_lossy().into_owned(), remote);
        let report = updates_report(&app, library, &kept);
        let row = &report.checkouts[0];
        assert_eq!(
            (row.checked, row.behind, row.workshop),
            (true, Some(2), false)
        );
        assert_eq!(serde_json::to_value(row).unwrap()["modId"], id.to_qml());
    }
}
