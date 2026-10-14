//! The words for a load order's problems, with what this install trusts.

use super::*;
use std::collections::HashSet;

/// What is wrong with each listed mod, in words, and per mod whether a
/// version warning was left out for it. No version warning for a mod
/// the community lists as working on this game version, for one you
/// excused, or for any when the check is switched off.
pub(super) fn troubles(library: &Library, mods: &[Mod]) -> (Vec<Vec<Trouble>>, Vec<bool>) {
    let mut trusted = crate::app().works_on().reading().clone();
    let notes = crate::app().notes().reading();
    let excused: HashSet<String> = notes.version_ok().map(String::from).collect();
    drop(notes);
    trusted.extend(excused.iter().cloned());
    let off = {
        let settings = crate::app().settings();
        !rimstack_core::checks::version_warnings_shown(
            settings.get("versionWarnings") == Some("off"),
            settings.get("versionAsideFor"),
            &library.game_version,
        )
    };
    let versions = match off {
        true => words::Versions::Ignored,
        false => words::Versions::CheckedBut(&trusted),
    };
    let workshop = crate::app().workshop_names().reading();
    // The community's word is not an ignored warning; yours is.
    let by_you = match off {
        true => words::Versions::Ignored,
        false => words::Versions::CheckedBut(&excused),
    };
    (
        words::troubles(library, mods, versions, &workshop),
        words::version_ignored(library, mods, by_you),
    )
}
