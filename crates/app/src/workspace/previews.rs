use super::App;
use rimstack_core::base::error::Error;
use rimstack_core::base::picture_pack::Pack;
use std::path::{Path, PathBuf};

impl App {
    /// The previews of every mod read, active or not.
    pub fn mod_previews(&self) -> Vec<PathBuf> {
        match &*self.library() {
            Ok((library, _)) => library
                .installed
                .iter()
                .filter_map(|installed| installed.preview.clone())
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// The pack of `set`'s pictures at `size`, holding `wanted`; the packs of `set` at other
    /// sizes go.
    pub fn fill_pictures(
        &self,
        set: &str,
        (width, height): (u32, u32),
        wanted: &[PathBuf],
    ) -> Result<Pack, Error> {
        let dir = self.data_dir().join("cache");
        let file = dir.join(format!("pictures-{set}-{width}x{height}.pack"));
        let pack = Pack::fill(&file, (width, height), wanted)?;
        remove_other_sizes(&dir, set, &file);
        Ok(pack)
    }
}

fn remove_other_sizes(dir: &Path, set: &str, kept: &Path) {
    let prefix = format!("pictures-{set}-");
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&prefix) && name.ends_with(".pack") && entry.path() != kept {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rimstack_core::base::picture_pack::Found;

    #[test]
    fn a_set_keeps_one_pack_the_size_last_shown() {
        let tmp = tempfile::tempdir().unwrap();
        let app = App::new(
            tmp.path().to_path_buf(),
            &tmp.path().join("settings"),
            &tmp.path().join("data"),
            Default::default(),
        );
        let preview = tmp.path().join("Preview.png");
        image::RgbaImage::from_pixel(8, 8, image::Rgba([9, 9, 9, 255]))
            .save(&preview)
            .unwrap();
        let wanted = [preview.clone()];
        app.fill_pictures("mods", (4, 2), &wanted).unwrap();
        app.fill_pictures("saves", (4, 2), &wanted).unwrap();
        let pack = app.fill_pictures("mods", (6, 3), &wanted).unwrap();
        assert!(
            matches!(pack.found(&preview.to_string_lossy()), Found::Pixels(p) if p.len() == 6 * 3 * 4)
        );
        let mut packs: Vec<String> = std::fs::read_dir(app.data_dir().join("cache"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        packs.sort();
        assert_eq!(packs, ["pictures-mods-6x3.pack", "pictures-saves-4x2.pack"]);
    }
}
