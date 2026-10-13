//! What the details panel and the summary show: everything known about
//! one mod, and about the whole install.

use super::*;

/// How many patched methods the details list before saying how many more.
const MOST_PATCHES: usize = 200;

impl qobject::ModListModel {
    pub(crate) fn details(&self, mod_id: i32) -> VariantMap {
        let mut map = VariantMap::default();
        let Ok((library, _)) = crate::library() else {
            return map;
        };
        let Some(m) = usize::try_from(mod_id)
            .ok()
            .and_then(|i| library.installed.get(i))
        else {
            return map;
        };
        let mut put = |key: &str, value: &str| map.insert(QString::from(key), text(value));
        put("name", m.display_name());
        put("source", m.source_label());
        put("contains", &m.contains());
        put("path", &m.path.to_string_lossy());
        put(
            "repository",
            &rimstack_core::git::page(&m.path).unwrap_or_default(),
        );
        put("folder", &m.folder);
        if let Some(preview) = &m.preview {
            put("preview", &preview.to_string_lossy());
        }
        match &m.about {
            Ok(about) => {
                put("author", &about.authors.join(", "));
                put("packageId", &about.package_id);
                put("version", about.mod_version.as_deref().unwrap_or_default());
                put("gameVersions", &about.supported_versions.join(", "));
                put(
                    "description",
                    about.description.as_deref().unwrap_or_default(),
                );
                put(
                    "descriptionRich",
                    &rimstack_core::markup::rich(about.description.as_deref().unwrap_or_default()),
                );
                put("url", about.url.as_deref().unwrap_or_default());
                let took = rimstack_core::startup::in_words(&crate::STARTUP, &about.package_id);
                put("loadTime", &took);
            }
            Err(reason) => put("unreadable", reason),
        }
        if let Some(note) = m.key().and_then(|k| {
            let notes = crate::NOTES.read().unwrap_or_else(|e| e.into_inner());
            notes.get(&k).cloned()
        }) {
            put("tags", &note.tags.join(", "));
            put("note", &note.text);
            put("categoryChosen", &note.category);
            put("versionOk", if note.version_ok { "yes" } else { "" });
        }
        if let Ok(about) = &m.about {
            let supports = &about.supported_versions;
            let listed = supports.iter().any(|v| v.trim() == library.game_version);
            if !supports.is_empty() && !listed && !library.game_version.is_empty() {
                put("madeFor", &supports.join(", "));
            }
        }
        // A Workshop mod's folder is named by its Workshop id.
        if let Some(new) = crate::REPLACEMENTS
            .get(&m.folder)
            .filter(|_| m.source_label() == "Workshop")
        {
            put("replacedBy", &new.name);
            put("replacedById", &new.workshop_id);
            put("replacedByVersions", &new.versions.join(", "));
        }
        let (times, sizes) = &*crate::STEAM_RECORD;
        if let Some(when) = times
            .get(&m.folder)
            .filter(|_| m.source_label() == "Workshop")
        {
            map.insert(QString::from("updated"), QVariant::from(&(*when as f64)));
        }
        if let Some(bytes) = sizes
            .get(&m.folder)
            .filter(|_| m.source_label() == "Workshop")
        {
            map.insert(
                QString::from("megabytes"),
                QVariant::from(&(*bytes as f64 / 1_048_576.0)),
            );
        }
        let mut put = |key: &str, value: &str| map.insert(QString::from(key), text(value));
        put("tree", &folder_tree(&m.path).join("\n"));
        put("assemblies", &m.assemblies().join(", "));
        let textures = rimstack_core::textures::pngs(&m.path);
        if !textures.is_empty() {
            put("textureCount", &textures.len().to_string());
        }
        // Lists are one "name<tab>note" per line; a leading "!" marks trouble.
        let requires: Vec<String> = library
            .requirements(m)
            .into_iter()
            .map(|(name, installed)| match installed {
                true => format!("{name}\t"),
                false => format!("!{name}\t{}", say!("not installed")),
            })
            .collect();
        put("requires", &requires.join("\n"));
        put("loadFolders", &self.load_folders(library, m).join("\n"));
        put("code", &words::code(&m.code.facts));
        let listed = |x: &&Mod| x.installed == Some(mod_id as usize);
        put(
            "sub",
            self.list
                .mods()
                .iter()
                .find(listed)
                .map_or("", |x| x.sub.as_str()),
        );
        let active = library.arrange(crate::session().current()).active;
        let others: Vec<(&str, &[rimstack_core::code::Aim])> = active
            .iter()
            .filter_map(|other| other.installed.filter(|&i| i != mod_id as usize))
            .map(|i| &library.installed[i])
            .map(|other| (other.display_name(), other.code.aims.as_slice()))
            .collect();
        let also = words::also_patching(&m.code.aims, &others);
        put(
            "patches",
            &words::aims(&m.code.aims, &also, MOST_PATCHES).join("\n"),
        );
        let chosen = m.key().and_then(|k| {
            let notes = crate::NOTES.read().unwrap_or_else(|e| e.into_inner());
            Category::from_label(&notes.get(&k)?.category)
        });
        if let Some(category) = chosen.or(library.categories.get(mod_id as usize).copied()) {
            put("category", category.label());
        }
        if let Ok(at) = usize::try_from(self.index_of(mod_id)) {
            put("category", self.list.mods()[at].category.label());
            let own = self.troubles.get(at).map(Vec::as_slice).unwrap_or_default();
            let lines: Vec<String> = own
                .iter()
                .map(|t| {
                    let flag = if t.error { "!" } else { "" };
                    format!("{flag}{}\t{}\t{}", t.words, t.fix.code(), t.fix.label())
                })
                .collect();
            put("problems", &lines.join("\n"));
            let (after, before, bottom) = self.neighbours(library, at);
            put("yourRules", &self.own_rules(library, at).join("\n"));
            put("detected", &self.detected(library, at).join("\n"));
            let (loses, wins, marks) = self.texture_conflicts(mod_id as usize);
            put("conflictMarks", &marks);
            put("losesTo", &loses.join("\n"));
            put("winsOver", &wins.join("\n"));
            put("loadsAfter", &after.join("\n"));
            put("loadsBefore", &before.join("\n"));
            if bottom {
                put("placement", &say!("The community rules keep this mod near the bottom of the list, below everything that does not have to follow it."));
            }
        }
        map
    }

    /// The library id of the installed mod with this folded package id
    /// that is in this list; -1 for none.
    pub(crate) fn package_mod(&self, key: &QString) -> i32 {
        let key = key.to_string();
        let Ok((library, _)) = crate::library() else {
            return -1;
        };
        let has = |m: &&Mod| {
            let installed = m.installed.and_then(|i| library.installed.get(i));
            installed.and_then(|i| i.key()).is_some_and(|k| k == key)
        };
        let found = self.list.mods().iter().find(has);
        found.and_then(|m| m.installed).map_or(-1, |i| i as i32)
    }

    /// The mods the one at `at` must load after and before, as details
    /// lines, each with who says so; and whether it belongs at the bottom.
    pub(crate) fn neighbours(
        &self,
        library: &Library,
        at: usize,
    ) -> (Vec<String>, Vec<String>, bool) {
        let mods = self.list.mods();
        let found = all_rules(library, mods);
        let mut after: Vec<(usize, String)> = Vec::new();
        let mut before: Vec<(usize, String)> = Vec::new();
        // "At the bottom" is one fact about a mod, not a rule per other mod.
        let pairwise = found.iter().filter(|r| r.kind != Kind::CommunityBottom);
        for rule in pairwise.filter(|r| r.first == at || r.then == at) {
            let (list, other) = match rule.then == at {
                true => (&mut after, rule.first),
                false => (&mut before, rule.then),
            };
            if list.iter().any(|(seen, _)| *seen == other) {
                continue;
            }
            let why = match (rule.broken(), rule.kind, rule.declared_by()) {
                (true, ..) => say!("out of order"),
                (_, Kind::Yours, _) => say!("your rule"),
                (_, Kind::Inferred, _) => say!("detected from files"),
                (_, _, None) => say!("community rule"),
                (_, Kind::Requires, Some(by)) if by == at => say!("required"),
                (_, Kind::Requires, _) => say!("requires this mod"),
                (_, _, Some(by)) if by == at => say!("this mod's rule"),
                _ => say!("that mod's rule"),
            };
            let flag = if rule.broken() { "!" } else { "" };
            list.push((other, format!("{flag}{}\t{why}", mods[other].name)));
        }
        let lines = |list: Vec<(usize, String)>| list.into_iter().map(|(_, line)| line).collect();
        let bottom = found
            .iter()
            .any(|r| r.kind == Kind::CommunityBottom && r.then == at);
        (lines(after), lines(before), bottom)
    }

    /// The folders of a mod that the game loads from with this load
    /// order, relative to the mod; "(the mod's own folder)" for its root.
    pub(crate) fn load_folders(
        &self,
        library: &Library,
        m: &rimstack_core::library::Installed,
    ) -> Vec<String> {
        let active: std::collections::HashSet<String> = self
            .list
            .mods()
            .iter()
            .filter_map(|x| library.installed.get(x.installed?)?.key())
            .collect();
        let found = loadfolders::folders(&m.path, &library.game_version, &|k| active.contains(k));
        found
            .iter()
            .map(|folder| match folder.strip_prefix(&m.path) {
                Ok(inside) if !inside.as_os_str().is_empty() => {
                    inside.to_string_lossy().into_owned()
                }
                _ => say!("(the mod's own folder)"),
            })
            .collect()
    }

    /// What the files of the mod at `at` say about other active mods, as
    /// details lines: "<name><tab><reason><tab>code|hint".
    pub(crate) fn detected(&self, library: &Library, at: usize) -> Vec<String> {
        let mods = self.list.mods();
        detected(library, mods)
            .iter()
            .filter(|d| d.then == at)
            .map(|d| {
                let kind = if d.code { "code" } else { "hint" };
                format!("{}\t{}\t{kind}", mods[d.first].name, d.why)
            })
            .collect()
    }

    /// Your own rules about the mod at `at`, as details lines:
    /// "<other's library id><tab><other's name><tab>after|before", where
    /// the last word says how this mod loads relative to the other.
    pub(crate) fn own_rules(&self, library: &Library, at: usize) -> Vec<String> {
        let mods = self.list.mods();
        let abouts = library.abouts(mods);
        let yours = crate::USER_RULES.read().unwrap_or_else(|e| e.into_inner());
        yours
            .rules(&abouts)
            .iter()
            .filter(|r| r.first == at || r.then == at)
            .filter_map(|r| {
                let (other, how) = if r.then == at {
                    (r.first, "after")
                } else {
                    (r.then, "before")
                };
                Some(format!(
                    "{}\t{}\t{how}",
                    mods[other].installed?, mods[other].name
                ))
            })
            .collect()
    }

    /// The mods that provide some of the same textures as the one with
    /// this library id, as details lines: those it loses to (they load
    /// later) and those it wins over. Last, their library ids for marking
    /// the rows: "12:-" for a mod this one loses to, "40:+" for one it
    /// wins over, joined by spaces.
    pub(crate) fn texture_conflicts(&self, mod_id: usize) -> (Vec<String>, Vec<String>, String) {
        let known = crate::CONFLICTS.read().unwrap_or_else(|e| e.into_inner());
        let (Some(known), Ok(mine)) = (
            known.as_ref(),
            usize::try_from(self.index_of(mod_id as i32)),
        ) else {
            return Default::default();
        };
        let Ok(place) = known.members.binary_search(&mod_id) else {
            return Default::default();
        };
        let (mut loses, mut wins, mut marks) = (Vec::new(), Vec::new(), Vec::new());
        for (other, count) in known.overlaps.with(place) {
            let other_id = known.members[other];
            let Ok(theirs) = usize::try_from(self.index_of(other_id as i32)) else {
                continue;
            };
            // Last, the other mod's library id, for the Compare button.
            let line = format!(
                "{}\t{}\t{other_id}",
                self.list.mods()[theirs].name,
                plural(count, &say!("texture"), &say!("textures"))
            );
            if theirs > mine {
                loses.push(line);
                marks.push(format!("{other_id}:-"));
            } else {
                wins.push(line);
                marks.push(format!("{other_id}:+"));
            }
        }
        (loses, wins, marks.join(" "))
    }

    pub(crate) fn overview(&self) -> VariantMap {
        let mut map = VariantMap::default();
        if let Ok((library, took)) = crate::library() {
            map.insert(QString::from("gameVersion"), text(&library.game_version));
            // The mods with an error, for the list of what to fix before
            // playing: "<library id><tab>name<tab>problem in a few words".
            let lines: Vec<String> = self
                .list
                .mods()
                .iter()
                .zip(&self.troubles)
                .filter_map(|(m, own)| {
                    let worst = own.iter().find(|t| t.error)?;
                    Some(format!("{}\t{}\t{}", m.installed?, m.name, worst.short))
                })
                .collect();
            map.insert(QString::from("problemLines"), text(&lines.join("\n")));
            // Every problem of every mod, for the list of them all:
            // "<library id or -1><tab>name<tab>1 for an error<tab>kind
            // <tab>sentence<tab>fix<tab>the fix's button".
            let each = self.list.mods().iter().zip(&self.troubles);
            let all: Vec<String> = each
                .flat_map(|(m, own)| own.iter().map(move |t| (m, t)))
                .map(|(m, t)| {
                    let id = m.installed.map_or(-1, |i| i as i64);
                    let error = u8::from(t.error);
                    let (group, code, label) = (t.fix.group(), t.fix.code(), t.fix.label());
                    format!(
                        "{id}\t{}\t{error}\t{group}\t{}\t{code}\t{label}",
                        m.name, t.words
                    )
                })
                .collect();
            map.insert(QString::from("troubleLines"), text(&all.join("\n")));
            // The five mods that took longest at the game's last start:
            // "<library id><tab>name<tab>time<tab>part of the slowest".
            let mut timed: Vec<(&Mod, u32)> = self
                .list
                .mods()
                .iter()
                .filter_map(|m| Some((m, m.load_micros.filter(|micros| *micros >= 1000)?)))
                .collect();
            timed.sort_by_key(|(_, micros)| std::cmp::Reverse(*micros));
            let most = timed.first().map_or(1, |(_, micros)| *micros);
            let slowest: Vec<String> = timed
                .iter()
                .take(5)
                .map(|(m, micros)| {
                    let id = m.installed.map_or(-1, |i| i as i64);
                    let took = rimstack_core::startup::seconds(f64::from(*micros) / 1000.0);
                    let part = f64::from(*micros) / f64::from(most);
                    format!("{id}\t{}\t{took}\t{part:.3}", m.name)
                })
                .collect();
            map.insert(QString::from("slowest"), text(&slowest.join("\n")));
            let any_timed = self.list.mods().iter().any(|m| m.load_micros.is_some());
            map.insert(QString::from("loadTimes"), QVariant::from(&any_timed));
            let mut number = |key: &str, n: usize| {
                map.insert(QString::from(key), QVariant::from(&(n as i32)));
            };
            number("installed", library.installed.len());
            number("missing", library.missing);
            number("unreadable", library.unreadable);
            number("readMs", took.as_millis() as usize);
            number(
                "worksOn",
                crate::WORKS_ON
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .len(),
            );
            // The mods you said work on this game version: name, nothing,
            // library id.
            let notes = crate::NOTES.read().unwrap_or_else(|e| e.into_inner());
            let trusted: Vec<String> = notes
                .version_ok()
                .map(|key| {
                    let found = library
                        .installed
                        .iter()
                        .position(|m| m.key().as_deref() == Some(key));
                    match found {
                        Some(i) => {
                            let name = library.name_of(key).unwrap_or_else(|| key.to_string());
                            format!("{name}\t\t{i}")
                        }
                        None => format!("{key}\tnot installed\t"),
                    }
                })
                .collect();
            drop(notes);
            map.insert(QString::from("trusted"), text(&trusted.join("\n")));
            let mut number = |key: &str, n: usize| {
                map.insert(QString::from(key), QVariant::from(&(n as i32)));
            };
            if let Some(community) = &*crate::COMMUNITY.read().unwrap_or_else(|e| e.into_inner()) {
                number("communityMods", community.len());
                let published = community.timestamp.unwrap_or(0) as f64;
                map.insert(QString::from("communityTime"), QVariant::from(&published));
            }
        }
        map
    }
}
