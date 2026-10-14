//! What Textures shows: per mod, how many textures have a DDS.

use super::*;

/// The mods you asked not to convert, by folded package id.
pub(super) fn skipped_mods(app: &crate::App) -> std::collections::HashSet<String> {
    let settings = app.settings();
    settings.listed("textureSkip").map(String::from).collect()
}

pub(crate) fn manifest_file(app: &crate::App) -> PathBuf {
    app.data_dir().join("textures.json")
}

pub(super) fn textures_report(app: &crate::App, library: &Library) -> Vec<textures::Row> {
    let manifest = textures::Manifest::read(&manifest_file(app));
    let skipped = skipped_mods(app);
    let active = app.active_now(library);
    let all: Vec<(ModId, &rimstack_core::mods::library::Installed)> = library.ids().collect();
    let rows: Vec<textures::Row> = all
        .par_iter()
        .filter_map(|&(i, m)| {
            let counted = textures::summarize(&m.path, &manifest);
            let skipped = m.key().is_some_and(|k| skipped.contains(&k));
            textures::Row::of(
                i,
                m.display_name(),
                &m.path,
                counted,
                skipped,
                active.contains(&i),
            )
        })
        .collect();
    textures::in_order(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_mod_with_textures_is_listed_until_you_ask_to_skip_it() {
        let (tmp, app) = crate::workspace::tests::installed();
        let textures = tmp.path().join("game/Mods/roads/Textures");
        std::fs::create_dir_all(&textures).unwrap();
        std::fs::write(textures.join("road.png"), [0u8; 16]).unwrap();
        let loaded = app.library();
        let (library, _) = loaded.as_ref().as_ref().unwrap();

        let rows = serde_json::to_value(textures_report(&app, library)).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 1);
        assert_eq!(rows[0]["name"], "Ann.Roads");
        assert_eq!(
            (&rows[0]["state"], &rows[0]["waits"]),
            (&json!("Not converted"), &json!(true))
        );

        app.set_setting("textureSkip", "ann.roads").unwrap();
        let rows = serde_json::to_value(textures_report(&app, library)).unwrap();
        assert_eq!(
            (&rows[0]["state"], &rows[0]["waits"]),
            (&json!("Skipped"), &json!(false))
        );
    }
}
