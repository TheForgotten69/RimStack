//! This install's rules, gathered from the shared state for the core's
//! ordering, and what the mods' files say, read in the background.

use super::*;

/// What the listed mods' files say about each other, by position in the
/// list. Uses what the background reading kept; a mod it has not read
/// (one just activated) is read here, which takes a few milliseconds.
/// Nothing until the first reading is done.
pub(super) fn detected(library: &Library, mods: &[Mod]) -> Vec<infer::Inferred> {
    // Asked several times for the same list (each click on a mod asks
    // twice), so the last answer is kept: the list it was for, the mods
    // whose files had been read, and the rules.
    type Kept = (Vec<Option<usize>>, Vec<usize>, Vec<infer::Inferred>);
    static LAST: std::sync::Mutex<Option<Kept>> = std::sync::Mutex::new(None);

    let known = crate::app().conflicts().reading();
    let Some(read) = known.as_ref() else {
        return Vec::new();
    };
    let list: Vec<Option<usize>> = mods.iter().map(|m| m.installed).collect();
    let mut last = LAST.held();
    if let Some((for_list, for_read, rules)) = last.as_ref() {
        if *for_list == list && *for_read == read.members {
            return rules.clone();
        }
    }
    let active: std::collections::HashSet<String> = mods
        .iter()
        .filter_map(|m| library.installed.get(m.installed?)?.key())
        .collect();
    let installed = |m: &Mod| m.installed.and_then(|i| library.installed.get(i));
    let nothing = infer::Scanned::default();
    // Only a few: reading a whole list here would freeze the window.
    let mut read_now: Vec<(usize, infer::Scanned)> = Vec::new();
    for (at, m) in mods.iter().enumerate() {
        let unread = m.installed.is_some_and(|i| !read.scanned.contains_key(&i));
        if let (true, Some(found), true) = (unread, installed(m), read_now.len() < 3) {
            let loaded =
                loadfolders::folders(&found.path, &library.game_version, &|k| active.contains(k));
            read_now.push((at, infer::scan(&loaded)));
        }
    }
    let scanned: Vec<&infer::Scanned> = mods
        .iter()
        .enumerate()
        .map(|(at, m)| {
            let fresh = read_now
                .iter()
                .find(|(place, _)| *place == at)
                .map(|(_, s)| s);
            let kept = m.installed.and_then(|i| read.scanned.get(&i));
            fresh.or(kept).unwrap_or(&nothing)
        })
        .collect();
    let names: Vec<String> = mods.iter().map(|m| m.name.clone()).collect();
    let official: Vec<bool> = mods
        .iter()
        .map(|m| installed(m).is_some_and(|i| i.source == rimstack_core::paths::Source::Official))
        .collect();
    let roots: Vec<std::path::PathBuf> = mods
        .iter()
        .map(|m| installed(m).map(|i| i.path.clone()).unwrap_or_default())
        .collect();
    let rules = infer::infer(&scanned, &names, &official, &roots);
    // Not kept when a mod had to be read here: the background reading
    // will have it next time, and may see more.
    if read_now.is_empty() {
        *last = Some((list, read.members.clone(), rules.clone()));
    }
    rules
}

/// Runs `with` on where this install's rules come from: your own, the
/// community's if they are switched on, and what the mods' files say.
fn with_sources<T>(from_files: &[infer::Inferred], with: impl FnOnce(Sources) -> T) -> T {
    let yours = crate::app().user_rules().reading();
    let community = crate::app().community().reading();
    let use_community = crate::app().settings().is_on("communityRules");
    with(Sources {
        yours: &yours,
        community: community.as_ref().filter(|_| use_community),
        from_files,
        hints: crate::app().settings().get("fileHints") == Some("on"),
    })
}

pub(super) fn all_rules(library: &Library, mods: &[Mod]) -> Vec<Rule> {
    with_sources(&detected(library, mods), |sources| {
        ordering::all_rules(library, mods, sources)
    })
}

/// Moves the mod just added at `at` to where its rules allow.
pub(super) fn settle(list: &mut GroupedList, at: usize) {
    let loaded = crate::app().library();
    let Ok((library, _)) = &*loaded else {
        return;
    };
    let found = all_rules(library, list.mods());
    let to = ordering::place(&found, at, list.mods().len());
    list.move_mod(at, to);
}

pub(super) fn sorted_by_category(library: &Library, mods: Vec<Mod>) -> (Vec<Mod>, sort::Sorted) {
    with_sources(&detected(library, &mods), |sources| {
        ordering::sorted_by_category(library, mods, sources)
    })
}
