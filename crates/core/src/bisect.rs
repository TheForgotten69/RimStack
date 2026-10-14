//! Finding the one mod that breaks the game, by halves: half of the
//! suspects are switched off, the game is tried, and the answer says
//! which half the mod is in.
//!
//! A mod that needs a switched-off mod is switched off with it, so a
//! round never fails for a missing requirement. It assumes one mod is at
//! fault; two that only break together are not found this way.

use crate::error::Error;
use crate::library::Library;
use crate::list::Mod;
use crate::paths::Source;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bisect {
    /// The load order when the search began, to go back to.
    pub original: Vec<String>,
    /// Places in `original` still suspected, in load order.
    pub suspects: Vec<usize>,
    /// For each place, the places it requires.
    requires: Vec<Vec<usize>>,
}

impl Bisect {
    /// `fixed` marks what is never suspected: the game and its expansions.
    pub fn new(original: Vec<String>, fixed: &[bool], requires: Vec<Vec<usize>>) -> Self {
        let suspect = |at: &usize| !fixed.get(*at).copied().unwrap_or(false);
        Self {
            suspects: (0..original.len()).filter(suspect).collect(),
            original,
            requires,
        }
    }

    /// `off` and whatever needs one of those, however indirectly.
    fn with_dependents(&self, mut off: Vec<bool>) -> Vec<bool> {
        // Requirements point at most a few levels deep; repeat until
        // nothing more goes off.
        loop {
            let needs_one = |at: usize| self.requires[at].iter().any(|r| off[*r]);
            let more: Vec<usize> = (0..off.len())
                .filter(|at| !off[*at] && needs_one(*at))
                .collect();
            if more.is_empty() {
                return off;
            }
            more.into_iter().for_each(|at| off[at] = true);
        }
    }

    /// Every place switched off this round: about half of the suspects,
    /// taken from the end of the list, each with whatever needs it. All
    /// `false` when the suspects need each other and cannot be told apart.
    pub fn off(&self) -> Vec<bool> {
        let none = vec![false; self.original.len()];
        let suspected = |off: &[bool]| self.suspects.iter().filter(|at| off[**at]).count();
        let half = self.suspects.len() / 2;
        let mut off = none.clone();
        let mut smallest: Option<Vec<bool>> = None;
        for at in self.suspects.iter().rev() {
            let mut with = off.clone();
            with[*at] = true;
            let with = self.with_dependents(with);
            let count = suspected(&with);
            if count <= half {
                off = with;
            } else if count < self.suspects.len()
                && smallest.as_ref().is_none_or(|s| count < suspected(s))
            {
                smallest = Some(with);
            }
            if suspected(&off) == half {
                break;
            }
        }
        match suspected(&off) {
            // No half could be cut cleanly: the smallest piece that can.
            0 => smallest.unwrap_or(none),
            _ => off,
        }
    }

    /// More than one suspect is left and none can be switched off
    /// without the others: they need each other.
    pub fn is_stuck(&self) -> bool {
        self.suspects.len() > 1 && self.off().iter().all(|off| !off)
    }

    /// The load order to try now.
    pub fn list(&self) -> Vec<String> {
        let off = self.off();
        let kept = self.original.iter().zip(off).filter(|(_, off)| !off);
        kept.map(|(id, _)| id.clone()).collect()
    }

    /// The game was tried with `list()`: still broken, or working. The
    /// mod at fault was on if it broke, and off if it did not.
    pub fn answer(&mut self, broken: bool) {
        let off = self.off();
        self.suspects.retain(|at| off[*at] != broken);
    }

    /// The mod at fault, once one suspect is left.
    pub fn found(&self) -> Option<&str> {
        match self.suspects.as_slice() {
            [one] => Some(self.original[*one].as_str()),
            _ => None,
        }
    }

    /// About how many more answers it takes; exact where no suspect
    /// needs another.
    pub fn rounds_left(&self) -> u32 {
        self.suspects
            .len()
            .max(1)
            .next_power_of_two()
            .trailing_zeros()
    }

    /// No file, or one that cannot be read, is no search under way.
    pub fn read(file: &Path) -> Option<Self> {
        serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()
    }

    pub fn write(&self, file: &Path) -> Result<(), Error> {
        let text = serde_json::to_string(self).map_err(|e| Error::format(e.to_string()))?;
        crate::whole::write(file, text)
    }
}

/// A search over this load order: the game's own parts are not
/// suspected, and each mod's requirements are those of its `About.xml`
/// that are in the list.
pub fn start(library: &Library, mods: &[Mod]) -> Bisect {
    let abouts = library.abouts(mods);
    let position = crate::rules::positions(&abouts);
    let installed = |m: &Mod| library.installed.get(m.installed?);
    let fixed: Vec<bool> = mods
        .iter()
        .map(|m| installed(m).is_some_and(|i| i.source == Source::Official))
        .collect();
    let requires = abouts
        .iter()
        .map(|about| {
            let needed = about.map(|a| a.dependencies.for_version(&library.game_version));
            let keys = needed.unwrap_or_default().iter();
            keys.filter_map(|id| position.get(&crate::id::split_steam(id).0).copied())
                .collect()
        })
        .collect();
    Bisect::new(
        mods.iter().map(|m| m.id.clone()).collect(),
        &fixed,
        requires,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_search_knows_what_each_listed_mod_requires() {
        use crate::library::{build, scan};
        use crate::paths::ModRoot;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Mods");
        let abouts = [
            ("lib", "<ModMetaData><packageId>a.lib</packageId><name>Lib</name></ModMetaData>"),
            ("user", "<ModMetaData><packageId>a.user</packageId><name>User</name><modDependencies><li><packageId>A.Lib</packageId></li><li><packageId>not.listed</packageId></li></modDependencies></ModMetaData>"),
        ];
        for (folder, about) in abouts {
            std::fs::create_dir_all(root.join(folder).join("About")).unwrap();
            std::fs::write(root.join(folder).join("About/About.xml"), about).unwrap();
        }
        let roots = [ModRoot {
            path: root,
            source: Source::Local,
        }];
        let active = ["a.user".to_string(), "a.lib".to_string()];
        let library = build(scan(&roots), &active, "1.6");
        let search = start(&library, &library.active);
        assert_eq!(search.original, active);
        assert_eq!(search.suspects, [0, 1]);
        assert_eq!(search.requires, [vec![1], vec![]]);
        // The library cannot go off without its user: the user goes off alone.
        assert_eq!(search.list(), ["a.lib"]);
    }

    fn ids(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("mod.{i}")).collect()
    }

    /// Answers as a game would in which `culprit` alone breaks things,
    /// and returns the mod found and the rounds it took.
    fn hunt(mut search: Bisect, culprit: &str) -> (String, u32) {
        let mut rounds = 0;
        while search.found().is_none() {
            let broken = search.list().iter().any(|id| id == culprit);
            search.answer(broken);
            rounds += 1;
            assert!(rounds < 64, "the search does not end");
        }
        (search.found().unwrap().to_string(), rounds)
    }

    #[test]
    fn every_mod_is_found_in_as_many_rounds_as_promised() {
        for n in [1, 2, 3, 8, 13, 100] {
            for culprit in 0..n {
                let search = Bisect::new(ids(n), &[], vec![Vec::new(); n]);
                let promised = search.rounds_left();
                let (found, rounds) = hunt(search, &format!("mod.{culprit}"));
                assert_eq!(found, format!("mod.{culprit}"));
                assert!(
                    rounds <= promised,
                    "{n} mods: {rounds} rounds, {promised} promised"
                );
            }
        }
    }

    #[test]
    fn the_game_and_its_expansions_are_never_suspected_or_switched_off() {
        let fixed = [true, true, false, false, false];
        let search = Bisect::new(ids(5), &fixed, vec![Vec::new(); 5]);
        assert_eq!(search.suspects, [2, 3, 4]);
        assert_eq!(search.list(), ["mod.0", "mod.1", "mod.2", "mod.3"]);
        let (found, _) = hunt(search, "mod.4");
        assert_eq!(found, "mod.4");
    }

    #[test]
    fn a_mod_goes_off_with_the_mod_it_needs() {
        // 1 needs 3, 0 needs 1: switching 3 off takes both along, so the
        // round cuts elsewhere.
        let requires = vec![vec![1], vec![3], vec![], vec![]];
        let search = Bisect::new(ids(4), &[], requires.clone());
        assert_eq!(search.off(), [true, false, true, false]);
        assert_eq!(search.list(), ["mod.1", "mod.3"]);
        for culprit in 0..4 {
            let (found, _) = hunt(
                Bisect::new(ids(4), &[], requires.clone()),
                &format!("mod.{culprit}"),
            );
            assert_eq!(found, format!("mod.{culprit}"));
        }
    }

    #[test]
    fn a_mod_everything_needs_is_still_told_from_the_rest() {
        // Each of 1, 2 and 3 needs 0.
        let requires = vec![vec![], vec![0], vec![0], vec![0]];
        for culprit in 0..4 {
            let (found, _) = hunt(
                Bisect::new(ids(4), &[], requires.clone()),
                &format!("mod.{culprit}"),
            );
            assert_eq!(found, format!("mod.{culprit}"));
        }
    }

    #[test]
    fn mods_that_need_each_other_cannot_be_told_apart() {
        let search = Bisect::new(ids(2), &[], vec![vec![1], vec![0]]);
        assert!(search.is_stuck());
        assert_eq!(search.found(), None);
        assert_eq!(search.list(), ids(2));
        assert!(!Bisect::new(ids(2), &[], vec![vec![], vec![]]).is_stuck());
    }

    #[test]
    fn a_search_survives_being_written_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("kept/bisect.json");
        assert_eq!(Bisect::read(&file), None);
        let mut search = Bisect::new(ids(6), &[true], vec![vec![]; 6]);
        search.answer(false);
        search.write(&file).unwrap();
        assert_eq!(Bisect::read(&file), Some(search));
    }
}
