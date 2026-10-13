//! Package ids. `ModsConfig.xml` writes them in lower case while `About.xml`
//! keeps the author's spelling, so they are only ever compared folded.

/// The form package ids are compared in: trimmed and lower-cased.
pub fn fold(id: &str) -> String {
    id.trim().to_lowercase()
}

/// Splits the `_steam` suffix the game adds to a Workshop copy whose id is
/// also installed locally. Returns the folded id and whether it had the suffix.
pub fn split_steam(id: &str) -> (String, bool) {
    let folded = fold(id);
    match folded.strip_suffix("_steam") {
        Some(base) if !base.is_empty() => (base.to_string(), true),
        _ => (folded, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_trims_and_lowercases() {
        assert_eq!(fold(" Ludeon.RimWorld "), "ludeon.rimworld");
    }

    #[test]
    fn steam_suffix_is_split_off() {
        assert_eq!(split_steam("Foo.Bar_steam"), ("foo.bar".into(), true));
        assert_eq!(split_steam("foo.bar"), ("foo.bar".into(), false));
    }

    #[test]
    fn steam_inside_an_id_is_left_alone() {
        assert_eq!(split_steam("my_steam.mod"), ("my_steam.mod".into(), false));
    }
}
