//! The words for a load order's problems, with what this install trusts.

use super::*;
use rimstack_core::kept::settings::Switch;
use std::collections::HashSet;

/// What is wrong with each listed mod, in words, and per mod whether a
/// version warning was left out for it. No version warning for a mod
/// the community lists as working on this game version, for one you
/// excused, or for any when the check is switched off.
pub(super) fn troubles(
    app: &crate::App,
    library: &Library,
    mods: &[Mod],
) -> (Vec<Vec<Trouble>>, Vec<bool>) {
    let mut trusted = app.works_on().reading().clone();
    let notes = app.notes().reading();
    let excused: HashSet<String> = notes.version_ok().map(String::from).collect();
    drop(notes);
    trusted.extend(excused.iter().cloned());
    let off = {
        let settings = app.settings();
        !rimstack_core::mods::checks::version_warnings_shown(
            !settings.switch(Switch::VersionWarnings),
            settings.get("versionAsideFor"),
            &library.game_version,
        )
    };
    let versions = match off {
        true => describe::Versions::Ignored,
        false => describe::Versions::CheckedBut(&trusted),
    };
    let workshop = app.workshop_names().reading();
    // The community's word is not an ignored warning; yours is.
    let by_you = match off {
        true => describe::Versions::Ignored,
        false => describe::Versions::CheckedBut(&excused),
    };
    (
        describe::troubles(library, mods, versions, &workshop),
        describe::version_ignored(library, mods, by_you),
    )
}
