//! What Saves shows: each save with how its mods differ from the load
//! order.

use super::*;

pub(crate) fn saves_dir(app: &crate::App) -> Option<PathBuf> {
    app.paths().config.as_ref().map(|c| c.join("Saves"))
}

pub(crate) fn save_row(
    app: &crate::App,
    save: &saves::SaveFile,
    library: &Library,
    list: &[String],
) -> saves::Row {
    let config = app.paths().config.clone().unwrap_or_default();
    let noted = rimsaves::meta(&config, &save.name);
    let compared = save.header.as_ref().ok().map(|header| {
        let name_of = |key: &str| library.name_of(key);
        saves::Compared::of(header, list, &library.game_version, &name_of)
    });
    saves::Row::of(
        save,
        rimsaves::facts(&noted),
        rimsaves::preview(&config, &save.name),
        compared,
    )
}
