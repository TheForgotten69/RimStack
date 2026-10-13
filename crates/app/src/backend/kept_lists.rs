//! Load orders kept aside: snapshots, named lists, launches and the
//! save copied before one.

use super::*;

impl qobject::Backend {
    pub(super) fn ids_in_text(&self, text: &QString) -> QString {
        let ids = rimstack_core::share::ids_in(&text.to_string());
        QString::from(&json!(ids).to_string())
    }

    pub(super) fn import_rimsort_rules(&self) -> QString {
        let said = match rimsort_rules() {
            Err(problem) => problem,
            Ok(pairs) => {
                let mut yours = crate::USER_RULES.write().unwrap_or_else(|e| e.into_inner());
                let added = yours.import(&pairs);
                match yours.write(&crate::user_rules_file()) {
                    Err(problem) => problem,
                    Ok(()) if pairs.is_empty() => say!("RimSort has no rules of your own."),
                    Ok(()) => say!(
                        "{} of RimSort's {} rules were new and are now yours here.",
                        added,
                        pairs.len()
                    ),
                }
            }
        };
        QString::from(&said)
    }

    pub(super) fn snapshot(&self, name: &QString) {
        let dir = snapshots_dir();
        let list = crate::session().current().to_vec();
        match lists::save(&dir, &name.to_string(), &list) {
            Ok(()) => lists::prune(&dir, 30),
            Err(problem) => eprintln!("{problem}"),
        }
    }

    /// Keeps the list as it is on disk among the snapshots; returns the
    /// name it was kept under, or nothing if it could not be.
    pub(super) fn snapshot_saved(&self, name: &QString) -> QString {
        let dir = snapshots_dir();
        let list = crate::session().saved().to_vec();
        match lists::save(&dir, &name.to_string(), &list) {
            Ok(()) => {
                lists::prune(&dir, 30);
                name.clone()
            }
            Err(problem) => {
                eprintln!("{problem}");
                QString::default()
            }
        }
    }

    pub(super) fn snapshots(&self) -> QString {
        let mut found = lists::all(&snapshots_dir());
        found.sort_by_key(|l| std::cmp::Reverse(l.modified));
        let rows: Vec<Value> = found
            .iter()
            .map(|l| json!({ "name": l.name, "mods": l.mods, "modified": l.modified }))
            .collect();
        QString::from(&Value::Array(rows).to_string())
    }

    pub(super) fn snapshot_list(&self, name: &QString) -> QString {
        match lists::load(&snapshots_dir(), &name.to_string()) {
            Ok(ids) => QString::from(&json!(ids).to_string()),
            Err(_) => QString::default(),
        }
    }

    pub(super) fn save_ids_as(&self, name: &QString, ids: &QString) -> QString {
        let Ok(ids) = serde_json::from_str::<Vec<String>>(&ids.to_string()) else {
            return QString::from(&say!("That is not a list of mods."));
        };
        match lists::save(&lists_dir(), &name.to_string(), &ids) {
            Ok(()) => QString::default(),
            Err(problem) => QString::from(&problem),
        }
    }

    pub(super) fn list_on_disk(&self) -> QString {
        let on_disk = crate::PATHS.mods_config();
        match on_disk.and_then(|path| modsconfig::read(&path).ok()) {
            Some(config) => QString::from(&json!(config.active).to_string()),
            None => QString::default(),
        }
    }

    pub(super) fn original_list(&self) -> QString {
        let original = crate::PATHS
            .mods_config()
            .map(|c| c.with_file_name("ModsConfig.xml.rimstack-original"));
        match original.and_then(|path| modsconfig::read(&path).ok()) {
            Some(config) => QString::from(&json!(config.active).to_string()),
            None => QString::default(),
        }
    }

    pub(super) fn saved_lists(&self) -> QString {
        let found = lists::all(&lists_dir());
        let rows: Vec<Value> = found
            .iter()
            .map(|l| json!({ "name": l.name, "mods": l.mods, "modified": l.modified }))
            .collect();
        QString::from(&Value::Array(rows).to_string())
    }

    pub(super) fn save_list_as(&self, name: &QString) -> QString {
        let list = crate::session().current().to_vec();
        match lists::save(&lists_dir(), &name.to_string(), &list) {
            Ok(()) => QString::default(),
            Err(problem) => QString::from(&problem),
        }
    }

    pub(super) fn saved_list(&self, name: &QString) -> QString {
        match lists::load(&lists_dir(), &name.to_string()) {
            Ok(ids) => QString::from(&json!(ids).to_string()),
            Err(_) => QString::default(),
        }
    }

    pub(super) fn delete_list(&self, name: &QString) {
        if let Err(problem) = lists::delete(&lists_dir(), &name.to_string()) {
            eprintln!("{problem}");
        }
    }

    pub(super) fn record_launch(&self) {
        let list = crate::session().current().to_vec();
        if let Err(problem) = launches::record(&launches_file(), &list, now()) {
            eprintln!("{problem}");
        }
        crate::mark_all_seen();
        since_launch::record_versions();
    }

    pub(super) fn back_up_newest_save(&self) -> QString {
        let Some(dir) = saves_dir() else {
            return QString::default();
        };
        // `list` is newest first.
        let Some(newest) = saves::list(&dir).into_iter().next() else {
            return QString::default();
        };
        let backups = crate::data_dir().join("save-backups");
        match saves::back_up(&newest.path, &backups, now(), 5) {
            Ok(_) => QString::default(),
            Err(problem) => QString::from(&problem),
        }
    }

    /// The list of the last launch from RimStack, as a JSON array, when
    /// the list on disk now holds nothing but the game and its
    /// expansions and that launch had more: the game empties the list
    /// when it starts on a new version. Otherwise nothing.
    pub(super) fn list_reset_by_game(&self) -> QString {
        let Ok((library, _)) = crate::library() else {
            return QString::default();
        };
        let Some(last) = launches::read(&launches_file()).pop() else {
            return QString::default();
        };
        let saved = crate::session().saved().to_vec();
        let on_disk = library.arrange(&saved).active;
        let official = |m: &rimstack_core::list::Mod| {
            let installed = m.installed.and_then(|i| library.installed.get(i));
            installed.is_some_and(|i| i.source == rimstack_core::paths::Source::Official)
        };
        match on_disk.iter().all(official) && last.ids.len() > saved.len() {
            true => QString::from(&json!(last.ids).to_string()),
            false => QString::default(),
        }
    }

    pub(super) fn game_running(&self) -> bool {
        let game = crate::PATHS.game.as_deref();
        game.is_some_and(|dir| {
            rimstack_core::upkeep::game_running(std::path::Path::new("/proc"), dir)
        })
    }
}
