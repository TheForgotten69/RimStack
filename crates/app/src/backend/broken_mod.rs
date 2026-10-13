//! The search for the one mod that breaks the game: its state between
//! rounds, kept on disk so it outlives the app being closed to play.

use super::*;
use rimstack_core::bisect::{self, Bisect};

fn search_file() -> PathBuf {
    crate::data_dir().join("bisect.json")
}

/// `{ active, suspects, rounds, list, original, found, foundId, stuck }`:
/// `list` is the load order to try now; `found` names the mod once one
/// suspect is left; `stuck` names suspects that need each other.
fn state(search: Option<&Bisect>) -> QString {
    let Some(search) = search else {
        return QString::from(&json!({ "active": false }).to_string());
    };
    let name = |id: &str| {
        let key = rimstack_core::id::split_steam(id).0;
        let known = crate::library()
            .as_ref()
            .ok()
            .and_then(|l| l.0.name_of(&key));
        known.unwrap_or_else(|| id.to_string())
    };
    let stuck: Vec<String> = match search.is_stuck() {
        true => search
            .suspects
            .iter()
            .map(|at| name(&search.original[*at]))
            .collect(),
        false => Vec::new(),
    };
    let said = json!({
        "active": true,
        "suspects": search.suspects.len(),
        "rounds": search.rounds_left(),
        "list": search.list(),
        "original": search.original,
        "found": search.found().map(name).unwrap_or_default(),
        "foundId": search.found().unwrap_or_default(),
        "stuck": stuck,
    });
    QString::from(&said.to_string())
}

impl qobject::Backend {
    pub(super) fn hunt_state(&self) -> QString {
        state(Bisect::read(&search_file()).as_ref())
    }

    /// Begins a search over the load order as it is now.
    pub(super) fn hunt_start(&self) -> QString {
        let Ok((library, _)) = crate::library() else {
            return state(None);
        };
        // The newest save as it is before any round, apart from the
        // copies made before each launch: those are few and soon replaced.
        let newest = saves_dir().and_then(|dir| saves::list(&dir).into_iter().next());
        if let Some(save) = newest {
            let kept = crate::data_dir().join("save-before-search");
            if let Err(problem) = saves::back_up(&save.path, &kept, now(), 3) {
                eprintln!("{problem}");
            }
        }
        let now = library.arrange(crate::session().current()).active;
        let search = bisect::start(library, &now);
        if let Err(problem) = search.write(&search_file()) {
            eprintln!("{problem}");
        }
        state(Some(&search))
    }

    /// The game was tried with the round's list: still broken, or not.
    pub(super) fn hunt_answer(&self, broken: bool) -> QString {
        let Some(mut search) = Bisect::read(&search_file()) else {
            return state(None);
        };
        search.answer(broken);
        if let Err(problem) = search.write(&search_file()) {
            eprintln!("{problem}");
        }
        state(Some(&search))
    }

    pub(super) fn hunt_stop(&self) {
        let _ = std::fs::remove_file(search_file());
    }
}
