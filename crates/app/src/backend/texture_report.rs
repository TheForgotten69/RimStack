//! What Textures shows: per mod, how many textures have a DDS.

use super::*;

/// The mods you asked not to convert, by folded package id; kept in the
/// settings as one line.
pub(super) fn skipped_mods() -> std::collections::HashSet<String> {
    let settings = crate::settings();
    let kept = settings.get("textureSkip").unwrap_or_default();
    kept.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

pub(super) fn manifest_file() -> PathBuf {
    crate::data_dir().join("textures.json")
}

/// One line per mod that has textures, in name order.
pub(super) fn textures_report(library: &Library) -> Value {
    let manifest = textures::Manifest::read(&manifest_file());
    let skipped = skipped_mods();
    let active: std::collections::HashSet<usize> = {
        let list = crate::session().current().to_vec();
        let arranged = library.arrange(&list);
        arranged.active.iter().filter_map(|m| m.installed).collect()
    };
    // What still waits to be converted comes first, then by name.
    let mut rows: Vec<((bool, String), Value)> = library
        .installed
        .par_iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let s = textures::summarize(&m.path, &manifest);
            if s.textures == 0 {
                return None;
            }
            let skipped = m.key().is_some_and(|k| skipped.contains(&k));
            let state = if skipped {
                named("Skipped")
            } else if s.out_of_date > 0 {
                named("Out of date")
            } else if s.not_converted() == s.textures {
                named("Not converted")
            } else if s.not_converted() > 0 {
                named("Partly converted")
            } else if s.converted == 0 {
                named("Already has DDS")
            } else {
                named("Converted")
            };
            let waits = !skipped && (s.out_of_date > 0 || s.not_converted() > 0);
            let row = json!({
                "id": i,
                "waits": waits,
                "name": m.display_name(),
                "active": active.contains(&i),
                "textures": s.textures,
                "converted": s.converted,
                "outOfDate": s.out_of_date,
                "shipped": s.shipped,
                "notConverted": s.not_converted(),
                                "state": state,
                "skipped": skipped,
                "pngMb": s.png_bytes as f64 / 1_048_576.0,
                "ddsMb": s.dds_bytes as f64 / 1_048_576.0,
                "madeMb": s.made_bytes as f64 / 1_048_576.0,
                "path": m.path.to_string_lossy(),
            });
            Some(((!waits, m.display_name().to_lowercase()), row))
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    Value::Array(rows.into_iter().map(|(_, row)| row).collect())
}
