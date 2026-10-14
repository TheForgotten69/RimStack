//! Which mods a log's errors point at, as the Logs screen lists them:
//! each kind of error with the mods in it, and the mods by how much of
//! the log is theirs.
//!
//! A mod is its place in `Library::installed`.

use crate::logs::{self, ErrorGroup};
use serde::Serialize;
use std::collections::HashMap;

/// What tells that an error is a mod's, from the surest: its code in the
/// trace, then its name in the error's first line, then its package id
/// there (how the game names a mod it has not loaded).
pub struct Clues<'a> {
    /// Assembly names in lower case, of the active mods.
    pub assemblies: &'a HashMap<String, usize>,
    /// Display names of the active mods that are not the game's own.
    pub names: &'a [(String, usize)],
    /// Package ids of every installed mod that is not the game's own.
    pub ids: &'a [(String, usize)],
}

impl Clues<'_> {
    fn mods_of(&self, group: &ErrorGroup) -> Vec<usize> {
        let by_code = logs::mods_in(group, self.assemblies);
        if !by_code.is_empty() {
            return by_code;
        }
        let named = logs::mentioned(&group.head, self.names);
        named
            .or_else(|| logs::mentioned(&group.head, self.ids))
            .into_iter()
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Named {
    pub name: String,
    pub mod_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pointed {
    /// Every mod in the error, the one it is counted against first.
    pub mods: Vec<Named>,
    pub head: String,
    pub trace: Vec<String>,
    pub count: usize,
    pub line: usize,
    /// Its place among the kinds of error, and where its run of alike
    /// errors begins and how many are in it.
    pub at: usize,
    pub run: usize,
    pub alike: usize,
    #[serde(rename = "mod")]
    pub first: Option<String>,
    /// -1 for an error that points at no mod.
    pub mod_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Blamed {
    /// Empty for the errors that point at no mod.
    pub name: String,
    pub mod_id: i64,
    pub kinds: usize,
    pub times: usize,
}

/// The errors of a log with the mods each points at, and those mods by
/// their share of the errors. `name_of` gives a mod's display name.
pub fn of(
    groups: &[ErrorGroup],
    clues: &Clues,
    name_of: &dyn Fn(usize) -> String,
) -> (Vec<Pointed>, Vec<Blamed>) {
    let runs = logs::runs(groups);
    let numbered = |id: Option<usize>| id.map_or(-1, |id| id as i64);
    let mut counted: Vec<(Option<usize>, usize)> = Vec::with_capacity(groups.len());
    let errors = groups
        .iter()
        .enumerate()
        .map(|(at, group)| {
            let mods = clues.mods_of(group);
            let first = mods.first().copied();
            counted.push((first, group.count));
            Pointed {
                mods: mods
                    .iter()
                    .map(|&mod_id| Named {
                        name: name_of(mod_id),
                        mod_id,
                    })
                    .collect(),
                head: group.head.clone(),
                trace: group.trace.clone(),
                count: group.count,
                line: group.line,
                at,
                run: runs[at],
                alike: runs.iter().filter(|&&start| start == runs[at]).count(),
                first: first.map(name_of),
                mod_id: numbered(first),
            }
        })
        .collect();
    let blamed = logs::culprits(counted)
        .iter()
        .map(|culprit| Blamed {
            name: culprit.mod_id.map(name_of).unwrap_or_default(),
            mod_id: numbered(culprit.mod_id),
            kinds: culprit.kinds,
            times: culprit.times,
        })
        .collect();
    (errors, blamed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(head: &str, trace: &[&str], count: usize) -> ErrorGroup {
        ErrorGroup {
            head: head.to_string(),
            trace: trace.iter().map(|line| line.to_string()).collect(),
            count,
            line: 1,
        }
    }

    fn name_of(id: usize) -> String {
        ["Core", "Better Roads", "Hats Galore"][id].to_string()
    }

    #[test]
    fn an_error_is_a_mod_s_by_its_code_then_its_name_then_its_package_id() {
        let assemblies = HashMap::from([("betterroads".to_string(), 1)]);
        let names = [("Hats Galore".to_string(), 2)];
        let ids = [("ann.betterroads".to_string(), 1)];
        let clues = Clues {
            assemblies: &assemblies,
            names: &names,
            ids: &ids,
        };
        let groups = [
            // Its code is in the trace, though another mod is named.
            group(
                "Exception in Hats Galore",
                &["  at BetterRoads.Patch.Prefix ()"],
                3,
            ),
            group("Could not load texture for Hats Galore", &[], 2),
            group("Mod ann.betterroads has no About.xml", &[], 1),
            group(
                "Exception filling window",
                &["  at Verse.Window.Draw ()"],
                5,
            ),
        ];
        let (errors, blamed) = of(&groups, &clues, &name_of);
        let pointed: Vec<i64> = errors.iter().map(|error| error.mod_id).collect();
        assert_eq!(pointed, [1, 2, 1, -1]);
        assert_eq!(errors[0].first.as_deref(), Some("Better Roads"));
        assert_eq!(errors[3].mods, []);

        let roads = blamed.iter().find(|b| b.mod_id == 1).unwrap();
        assert_eq!((roads.kinds, roads.times), (2, 4));
        let nobody = blamed.iter().find(|b| b.mod_id == -1).unwrap();
        assert_eq!((nobody.name.as_str(), nobody.times), ("", 5));

        let said = serde_json::to_value(&errors[0]).unwrap();
        assert_eq!(said["mod"], "Better Roads");
        assert_eq!(said["mods"][0]["modId"], 1);
    }
}
