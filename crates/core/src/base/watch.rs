//! Noticing that something on disk changed while RimStack was open,
//! without being told: by when it was last written.

use serde::Serialize;
use std::path::Path;

/// When the file was last written, or an entry was added to or removed
/// from the folder, in milliseconds since 1970; 0 when it is not there.
pub fn changed_at(path: &Path) -> u64 {
    let written = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let since = written.and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok());
    since.map_or(0, |d| d.as_millis() as u64)
}

/// The newest of several, so that a change to any one shows.
pub fn newest<'a>(paths: impl IntoIterator<Item = &'a Path>) -> u64 {
    paths.into_iter().map(changed_at).max().unwrap_or(0)
}

/// When each thing RimStack shows was last changed on disk, as
/// `changed_at` or `newest` gives it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stamps {
    /// The mod folders, Steam's record of its downloads, and each mod's
    /// folder, `About.xml` and Git history.
    pub mods: u64,
    pub saves: u64,
    pub log: u64,
    /// `ModsConfig.xml`, and only while it holds another load order than
    /// the one last saved or read here: 0 while it holds that one.
    pub mods_config: u64,
    pub theme: u64,
    /// The chosen language's own file.
    pub language: u64,
}

/// What a look found changed since the one before, to be read again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Noticed {
    /// Read the mod folders again: they changed, and have been quiet
    /// since for a look, and nothing was in the way.
    pub mods: bool,
    pub saves: bool,
    /// The game wrote a log, and has stopped.
    pub log: bool,
    /// `ModsConfig.xml` holds another load order, written outside
    /// RimStack while the game was not running.
    pub mods_config: bool,
    pub theme: bool,
    pub language: bool,
}

/// Looks at the stamps again and again, and says what changed between
/// two looks. The first look only takes note.
#[derive(Debug, Default)]
pub struct Watcher {
    before: Option<Stamps>,
    mods_pending: bool,
}

impl Watcher {
    /// `in_the_way`: the mods cannot be read again now (a reading under
    /// way, a sort on show, a drag, a comparison); they are once it is not.
    pub fn look(&mut self, mut now: Stamps, game_running: bool, in_the_way: bool) -> Noticed {
        let Some(before) = self.before else {
            self.before = Some(now);
            return Noticed::default();
        };
        let mut noticed = Noticed::default();
        // Steam writes its record again and again while it downloads:
        // the folders are read once they have been quiet for a look.
        if now.mods != before.mods {
            self.mods_pending = true;
        } else if self.mods_pending && !in_the_way {
            self.mods_pending = false;
            noticed.mods = true;
        }
        noticed.saves = now.saves != before.saves;
        noticed.theme = now.theme != before.theme;
        noticed.language = now.language != before.language;
        // The game writes its log all the time it runs, and writes
        // ModsConfig.xml while it starts: both wait until it has stopped.
        if now.log != before.log {
            match game_running {
                true => now.log = before.log,
                false => noticed.log = true,
            }
        }
        if now.mods_config != before.mods_config && now.mods_config != 0 {
            match game_running {
                true => now.mods_config = before.mods_config,
                false => noticed.mods_config = true,
            }
        }
        self.before = Some(now);
        noticed
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn at(mods: u64, log: u64, mods_config: u64) -> Stamps {
        Stamps {
            mods,
            log,
            mods_config,
            ..Stamps::default()
        }
    }

    #[test]
    fn the_first_look_only_takes_note_and_an_unchanged_one_finds_nothing() {
        let mut watcher = Watcher::default();
        assert_eq!(watcher.look(at(1, 1, 0), false, false), Noticed::default());
        assert_eq!(watcher.look(at(1, 1, 0), false, false), Noticed::default());
    }

    #[test]
    fn changed_mods_are_read_once_quiet_for_a_look_and_nothing_is_in_the_way() {
        let mut watcher = Watcher::default();
        watcher.look(at(1, 0, 0), false, false);
        // Steam still writing: not yet.
        assert!(!watcher.look(at(2, 0, 0), false, false).mods);
        assert!(!watcher.look(at(3, 0, 0), false, false).mods);
        // Quiet, but a drag is under way: still waiting.
        assert!(!watcher.look(at(3, 0, 0), false, true).mods);
        assert!(watcher.look(at(3, 0, 0), false, false).mods);
        // Once read, not again.
        assert!(!watcher.look(at(3, 0, 0), false, false).mods);
    }

    #[test]
    fn the_log_and_the_list_written_while_the_game_runs_wait_until_it_stops() {
        let mut watcher = Watcher::default();
        watcher.look(at(0, 10, 0), false, false);
        assert!(!watcher.look(at(0, 11, 5), true, false).log);
        assert!(!watcher.look(at(0, 12, 5), true, false).mods_config);
        let stopped = watcher.look(at(0, 12, 5), false, false);
        assert!(stopped.log && stopped.mods_config);
        assert_eq!(watcher.look(at(0, 12, 5), false, false), Noticed::default());
    }

    #[test]
    fn a_list_back_to_the_saved_one_is_no_change_to_offer() {
        let mut watcher = Watcher::default();
        watcher.look(at(0, 0, 5), false, false);
        assert!(!watcher.look(at(0, 0, 0), false, false).mods_config);
        assert!(watcher.look(at(0, 0, 9), false, false).mods_config);
    }

    #[test]
    fn saves_the_theme_and_the_language_are_noticed_at_once() {
        let mut watcher = Watcher::default();
        watcher.look(Stamps::default(), true, true);
        let changed = Stamps {
            saves: 1,
            theme: 2,
            language: 3,
            ..Stamps::default()
        };
        let noticed = watcher.look(changed, true, true);
        assert!(noticed.saves && noticed.theme && noticed.language);
    }

    #[test]
    fn a_missing_file_has_no_time_and_a_new_entry_changes_its_folder() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(changed_at(&tmp.path().join("none")), 0);
        let before = changed_at(tmp.path());
        assert!(before > 0);
        // A file system keeps times to the millisecond at best.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::create_dir(tmp.path().join("mod")).unwrap();
        let after = changed_at(tmp.path());
        assert!(after > before);
        assert_eq!(
            newest([tmp.path(), tmp.path().join("none").as_path()]),
            after
        );
        assert_eq!(newest([]), 0);
    }
}
