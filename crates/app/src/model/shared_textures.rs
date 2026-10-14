//! Which mods share textures, read in the background and marked on the
//! rows.

use super::*;

impl qobject::ModListModel {
    /// The library ids of the active mods that share a texture with
    /// another active mod.
    pub(crate) fn in_conflict(&self) -> std::collections::HashSet<usize> {
        let known = crate::app().conflicts().reading();
        let Some(known) = known.as_ref() else {
            return Default::default();
        };
        let involved = (0..known.members.len()).filter(|&at| !known.overlaps.with(at).is_empty());
        involved.map(|at| known.members[at]).collect()
    }

    pub(crate) fn is_new(&self, m: &Mod) -> bool {
        let loaded = crate::app().library();
        let installed = match (&*loaded, m.installed) {
            (Ok((library, _)), Some(i)) => library.installed.get(i),
            _ => None,
        };
        let seen = crate::app().seen().reading();
        installed.is_some_and(|i| seen.is_new(&i.path.to_string_lossy()))
    }

    /// The library ids of this list's mods that are new, as a JSON array.
    pub(crate) fn fresh_ids(&self) -> QString {
        let mods = self.list.mods().iter().filter(|m| self.is_new(m));
        let ids: Vec<usize> = mods.filter_map(|m| m.installed).collect();
        QString::from(&serde_json::json!(ids).to_string())
    }

    /// Per mod in load order, what the `is:` filter terms can ask about.
    pub(crate) fn flags(&self) -> Vec<u16> {
        use rimstack_core::list::flag;
        let conflict = self.in_conflict();
        let behind = crate::app().behind().reading();
        let loaded = crate::app().library();
        let library = loaded.as_ref().as_ref().ok();
        let installed = library
            .map(|l| l.0.installed.as_slice())
            .unwrap_or_default();
        let mut copies: std::collections::HashMap<String, usize> = Default::default();
        for key in installed.iter().filter_map(|m| m.key()) {
            *copies.entry(key).or_default() += 1;
        }
        let week_ago = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |now| now.as_secs().saturating_sub(7 * 24 * 3600));
        let newest_save = crate::app().newest_save();
        let in_save = newest_save.as_ref().as_ref().filter(|_| self.checked);
        let steam_record = crate::app().steam_record();
        let session = crate::app().session();
        let changes = session.changes();
        let changed: std::collections::HashSet<&str> = match self.checked {
            true => changes
                .moved
                .iter()
                .chain(&changes.activated)
                .copied()
                .collect(),
            false => Default::default(),
        };
        let mods = self.list.mods().iter().enumerate();
        mods.map(|(at, m)| {
            let own = self.troubles.get(at).map(Vec::as_slice).unwrap_or_default();
            let mut flags = 0;
            if own.iter().any(|t| t.error) {
                flags |= flag::ERROR;
            }
            if own.iter().any(|t| !t.error) {
                flags |= flag::WARNING;
            }
            if m.installed.is_some_and(|i| conflict.contains(&i)) {
                flags |= flag::CONFLICT;
            }
            let folder = m
                .installed
                .and_then(|i| Some(&library?.0.installed.get(i)?.folder));
            if folder.is_some_and(|f| behind.as_ref().is_some_and(|b| b.contains(f))) {
                flags |= flag::UPDATED;
            }
            let key = rimstack_core::id::split_steam(&m.id).0;
            if in_save.is_some_and(|ids| !ids.contains(&key)) {
                flags |= flag::NOT_IN_SAVE;
            }
            if folder.is_some_and(|f| steam_record.0.get(f).is_some_and(|at| *at >= week_ago)) {
                flags |= flag::RECENT;
            }
            if copies.get(&key).is_some_and(|n| *n > 1) {
                flags |= flag::DUPLICATE;
            }
            if self.is_new(m) {
                flags |= flag::NEW;
            }
            if changed.contains(m.id.as_str()) {
                flags |= flag::CHANGED;
            }
            flags
        })
        .collect()
    }

    /// Works out, in the background, which active mods provide the same
    /// textures; unless that is already known for this set of mods.
    pub(crate) fn find_conflicts(mut self: Pin<&mut Self>, loaded: std::sync::Arc<crate::Loaded>) {
        let mut members: Vec<usize> = self.checked_order.iter().flatten().copied().collect();
        members.sort_unstable();
        members.dedup();
        let known = crate::app().conflicts().reading();
        let already = known.as_ref().is_some_and(|c| c.members == members);
        drop(known);
        self.as_mut().set_files_read(already);
        if already {
            return;
        }
        let read = move || {
            let Ok((library, _)) = &*loaded else {
                return false;
            };
            *crate::app().conflicts().writing() = Some(read_files(library, members));
            true
        };
        crate::background::run(self.qt_thread(), read, |mut model, read| {
            if read {
                model.as_mut().set_files_read(true);
                model.as_mut().refile();
                let next = model.conflicts_version + 1;
                model.as_mut().set_conflicts_version(next);
            }
        });
    }
}

/// What the files of these mods (library ids, ascending) provide and
/// point at, each read from the folders the game would load it from.
fn read_files(library: &rimstack_core::library::Library, members: Vec<usize>) -> crate::Conflicts {
    let active: std::collections::HashSet<String> = members
        .iter()
        .filter_map(|&i| library.installed[i].key())
        .collect();
    let read: Vec<(Vec<String>, infer::Scanned)> = members
        .par_iter()
        .map(|&i| {
            let root = &library.installed[i].path;
            let loaded = loadfolders::folders(root, &library.game_version, &|m| active.contains(m));
            (conflicts::provided(&loaded), infer::scan(&loaded))
        })
        .collect();
    let (provided, scanned): (Vec<_>, Vec<_>) = read.into_iter().unzip();
    crate::Conflicts {
        overlaps: conflicts::overlaps(&provided),
        scanned: members.iter().copied().zip(scanned).collect(),
        members,
    }
}
