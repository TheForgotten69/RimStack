//! What Saves shows: each save with how its mods differ from the load
//! order.

use super::*;
use rimstack_core::tongue::named;

pub(super) fn saves_dir() -> Option<PathBuf> {
    crate::PATHS.config.as_ref().map(|c| c.join("Saves"))
}

pub(super) fn save_row(save: &saves::SaveFile, library: &Library, list: &[String]) -> Value {
    // With the RimSaves mod, a save's folder is part of its file name,
    // and a preview and some facts are kept beside the saves.
    let (folder, shown) = rimsaves::split(&save.name);
    let config = crate::PATHS.config.clone().unwrap_or_default();
    let noted = rimsaves::meta(&config, &save.name);
    let fact = |label: &str, key: &str| -> Option<Value> {
        let value = noted.get(key).filter(|v| !v.is_empty())?;
        Some(json!([label, value]))
    };
    let facts: Vec<Value> = [
        fact(named("Colony"), "faction"),
        fact(named("Colonists"), "nbColonist"),
        fact(named("Settlements"), "nbColony"),
        fact(named("Storyteller"), "storyteller"),
        fact(named("Difficulty"), "storytellerDifficulty"),
        fact(named("Scenario"), "scenario"),
        fact(named("Seed"), "seed"),
    ]
    .into_iter()
    .flatten()
    .collect();
    let preview = rimsaves::preview(&config, &save.name);
    let mut row = json!({
        "file": save.name,
        "folder": folder,
        "facts": facts,
        "preview": preview.map(|p| p.to_string_lossy().into_owned()),
        "name": shown,
        "path": save.path.to_string_lossy(),
        "modified": save.modified,
        "megabytes": save.bytes as f64 / 1_048_576.0,
    });
    let header = match &save.header {
        Ok(header) => header,
        Err(reason) => {
            row["problem"] = json!(reason);
            row["verdict"] = json!("unreadable");
            return row;
        }
    };
    let difference = saves::compare(&header.mod_ids, list);
    let missing: Vec<Value> = difference
        .missing
        .iter()
        .map(|&i| {
            let key = id::split_steam(&header.mod_ids[i]).0;
            let installed = library.name_of(&key);
            let name = installed
                .clone()
                .or_else(|| header.mod_names.get(i).cloned());
            json!({ "name": name.unwrap_or(key), "installed": installed.is_some() })
        })
        .collect();
    let added: Vec<String> = difference
        .added
        .iter()
        .map(|&i| {
            let key = id::split_steam(&list[i]).0;
            library.name_of(&key).unwrap_or(key)
        })
        .collect();
    let version = header.game_version.as_deref().unwrap_or_default();
    let short = modsconfig::short_version(version).unwrap_or_default();
    let other_game = !short.is_empty() && short != library.game_version;
    // Only what can break a save counts as trouble.
    let verdict = if !missing.is_empty() || other_game {
        "breaks"
    } else if difference.is_none() {
        "same"
    } else {
        "differs"
    };
    row["gameVersion"] = json!(version);
    row["otherGame"] = json!(other_game);
    row["mods"] = json!(header.mod_ids.len());
    row["ids"] = json!(header.mod_ids);
    row["missing"] = json!(missing);
    row["added"] = json!(added);
    row["reordered"] = json!(difference.reordered);
    row["verdict"] = json!(verdict);
    row
}
