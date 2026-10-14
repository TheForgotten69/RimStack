//! What the Textures screen still reaches outside the `App`.

use rimstack_core::mods::library::{Library, ModId};
use rimstack_core::rimworld::loadfolders;
use std::path::PathBuf;

pub(crate) fn hidden_file(app: &crate::App) -> PathBuf {
    app.data_dir().join("hidden.json")
}

/// The folders the game loads a mod from with the load order being edited.
pub(crate) fn loaded_folders(app: &crate::App, library: &Library, mod_id: ModId) -> Vec<PathBuf> {
    let list = app.session().current().to_vec();
    let arranged = library.arrange(&list);
    let active: std::collections::HashSet<String> = arranged
        .active
        .iter()
        .filter_map(|m| library.get(m.installed?)?.key())
        .collect();
    let Some(root) = library.get(mod_id).map(|m| &m.path) else {
        return Vec::new();
    };
    loadfolders::folders(root, &library.game_version, &|k| active.contains(k))
}

pub(crate) fn ids_from(json: &cxx_qt_lib::QString) -> Vec<ModId> {
    let json = json.to_string();
    serde_json::from_str(&json).unwrap_or_else(|e| {
        rimstack_core::problem!("mod ids given as {json:?}: {e}");
        Vec::new()
    })
}
