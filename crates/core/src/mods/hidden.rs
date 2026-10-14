//! Hiding a texture so the game skips it: the file is renamed inside its
//! mod's folder, never edited or deleted. This is the one case where
//! RimStack changes files of a mod, and every rename is recorded so it
//! can be put back.

use crate::base::error::Error;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const SUFFIX: &str = ".rimstack-hidden";

/// What is hidden: each original path, with the length the file had.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hidden {
    files: BTreeMap<PathBuf, u64>,
}

fn hidden_name(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(SUFFIX);
    PathBuf::from(name)
}

impl Hidden {
    /// No file, or a damaged one, is nothing hidden. Files already renamed
    /// then stay renamed; `strays` finds them.
    pub fn read(path: &Path) -> Self {
        let Some(Value::Object(entries)) = crate::base::atomic_file::read_json(path) else {
            return Self::default();
        };
        let files = entries
            .iter()
            .filter_map(|(k, v)| Some((PathBuf::from(k), v.as_u64()?)))
            .collect();
        Self { files }
    }

    pub fn write(&self, path: &Path) -> Result<(), Error> {
        let entries: serde_json::Map<String, Value> = self
            .files
            .iter()
            .map(|(k, len)| (k.to_string_lossy().into_owned(), json!(len)))
            .collect();
        crate::base::atomic_file::write(path, Value::Object(entries).to_string())
    }

    #[cfg(test)]
    fn is_hidden(&self, texture: &Path) -> bool {
        self.files.contains_key(texture)
    }

    /// The hidden files inside one mod's folder, by original path.
    pub fn in_mod(&self, mod_root: &Path) -> Vec<&Path> {
        let inside = |p: &&PathBuf| p.starts_with(mod_root);
        self.files
            .keys()
            .filter(inside)
            .map(PathBuf::as_path)
            .collect()
    }

    /// Hides a texture: the PNG, and the DDS beside it if there is one.
    /// Hiding one that is already hidden changes nothing.
    pub fn hide(&mut self, png: &Path) -> Result<(), Error> {
        for file in [png.to_path_buf(), png.with_extension("dds")] {
            if self.files.contains_key(&file) || !file.is_file() {
                continue;
            }
            let failed = |e: std::io::Error| Error::file(&file, e);
            let len = std::fs::metadata(&file).map_err(failed)?.len();
            std::fs::rename(&file, hidden_name(&file)).map_err(failed)?;
            self.files.insert(file, len);
        }
        Ok(())
    }

    /// Puts a hidden texture back under its own name. If a file of that
    /// name exists again (Steam restored it), the hidden copy is left
    /// where it is and the record dropped: nothing is overwritten.
    pub fn unhide(&mut self, png: &Path) -> Result<(), Error> {
        for file in [png.to_path_buf(), png.with_extension("dds")] {
            if self.files.remove(&file).is_none() {
                continue;
            }
            let renamed = hidden_name(&file);
            if renamed.is_file() && !file.exists() {
                std::fs::rename(&renamed, &file).map_err(|e| Error::file(&file, e))?;
            }
        }
        Ok(())
    }

    /// Unhides everything in one mod's folder. Returns how many textures.
    pub fn unhide_all(&mut self, mod_root: &Path) -> Result<usize, Error> {
        let pngs: Vec<PathBuf> = self
            .in_mod(mod_root)
            .into_iter()
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")))
            .map(Path::to_path_buf)
            .collect();
        for png in &pngs {
            self.unhide(png)?;
        }
        Ok(pngs.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn textures() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("mod/Textures");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("wall.png"), "png bytes").unwrap();
        fs::write(dir.join("wall.dds"), "dds").unwrap();
        fs::write(dir.join("door.png"), "door").unwrap();
        (tmp, dir)
    }

    #[test]
    fn hiding_renames_the_png_and_its_dds_and_unhiding_puts_them_back() {
        let (_tmp, dir) = textures();
        let mut hidden = Hidden::default();
        hidden.hide(&dir.join("wall.png")).unwrap();
        assert!(!dir.join("wall.png").exists() && !dir.join("wall.dds").exists());
        assert_eq!(
            fs::read_to_string(dir.join("wall.png.rimstack-hidden")).unwrap(),
            "png bytes"
        );
        assert!(dir.join("wall.dds.rimstack-hidden").exists());
        assert!(hidden.is_hidden(&dir.join("wall.png")));
        assert!(dir.join("door.png").exists());

        hidden.hide(&dir.join("wall.png")).unwrap(); // again: nothing changes
        hidden.unhide(&dir.join("wall.png")).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("wall.png")).unwrap(),
            "png bytes"
        );
        assert_eq!(fs::read_to_string(dir.join("wall.dds")).unwrap(), "dds");
        assert!(!dir.join("wall.png.rimstack-hidden").exists());
        assert_eq!(hidden, Hidden::default());
    }

    #[test]
    fn a_whole_mod_is_unhidden_at_once_and_other_mods_are_left() {
        let (_tmp, dir) = textures();
        let (_tmp2, other) = textures();
        let mut hidden = Hidden::default();
        hidden.hide(&dir.join("wall.png")).unwrap();
        hidden.hide(&dir.join("door.png")).unwrap();
        hidden.hide(&other.join("door.png")).unwrap();
        assert_eq!(hidden.in_mod(dir.parent().unwrap()).len(), 3);
        assert_eq!(hidden.unhide_all(dir.parent().unwrap()).unwrap(), 2);
        assert!(dir.join("wall.png").exists() && dir.join("door.png").exists());
        assert!(hidden.is_hidden(&other.join("door.png")));
    }

    #[test]
    fn a_file_steam_restored_is_never_overwritten() {
        let (_tmp, dir) = textures();
        let mut hidden = Hidden::default();
        hidden.hide(&dir.join("door.png")).unwrap();
        fs::write(dir.join("door.png"), "restored by Steam").unwrap();
        hidden.unhide(&dir.join("door.png")).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("door.png")).unwrap(),
            "restored by Steam"
        );
        assert!(dir.join("door.png.rimstack-hidden").exists());
        assert!(!hidden.is_hidden(&dir.join("door.png")));
    }

    #[test]
    fn the_record_reads_back_as_written() {
        let (tmp, dir) = textures();
        let mut hidden = Hidden::default();
        hidden.hide(&dir.join("wall.png")).unwrap();
        let file = tmp.path().join("data/hidden.json");
        hidden.write(&file).unwrap();
        assert_eq!(Hidden::read(&file), hidden);
        fs::write(&file, "x").unwrap();
        assert_eq!(Hidden::read(&file), Hidden::default());
    }
}
