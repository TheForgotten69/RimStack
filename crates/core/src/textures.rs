//! PNG textures and the DDS copies RimStack makes of them. The game loads
//! a `.dds` beside a `.png` instead of it, and faster.
//!
//! Every file RimStack makes is recorded in a manifest kept in RimStack's
//! own data folder, so that later only those files are ever deleted, and
//! only while they are still exactly what RimStack wrote. Nothing but the
//! `.dds` itself is put in a mod's folder.

use crate::say;

use image_dds::{ImageFormat, Mipmaps, Quality};
use rayon::prelude::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// A DDS RimStack made, and the PNG it was made from as it was then.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Made {
    pub source_len: u64,
    /// Seconds since 1970.
    pub source_modified: u64,
    pub output_hash: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifest {
    made: BTreeMap<PathBuf, Made>,
}

impl Manifest {
    /// No file, or a damaged one, is an empty manifest: RimStack then
    /// treats every DDS as someone else's and deletes none.
    pub fn read(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(Value::Object(entries)) = serde_json::from_str(&text) else {
            return Self::default();
        };
        let made = entries
            .iter()
            .filter_map(|(dds, v)| {
                let made = Made {
                    source_len: v.get("len")?.as_u64()?,
                    source_modified: v.get("modified")?.as_u64()?,
                    output_hash: u64::from_str_radix(v.get("output")?.as_str()?, 16).ok()?,
                };
                Some((PathBuf::from(dds), made))
            })
            .collect();
        Self { made }
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        let failed = |e: std::io::Error| format!("{}: {e}", path.display());
        let entries: serde_json::Map<String, Value> = self
            .made
            .iter()
            .map(|(dds, m)| {
                let entry = json!({ "len": m.source_len, "modified": m.source_modified, "output": format!("{:016x}", m.output_hash) });
                (dds.to_string_lossy().into_owned(), entry)
            })
            .collect();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(failed)?;
        }
        let partial = path.with_extension("json.part");
        std::fs::write(&partial, Value::Object(entries).to_string()).map_err(failed)?;
        std::fs::rename(&partial, path).map_err(failed)
    }

    pub fn len(&self) -> usize {
        self.made.len()
    }

    pub fn is_empty(&self) -> bool {
        self.made.is_empty()
    }
}

/// FNV-1a: enough to tell whether a file is still the one RimStack wrote.
fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn fingerprint(path: &Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some((meta.len(), modified.as_secs()))
}

fn has_extension(path: &Path, wanted: &str) -> bool {
    path.extension()
        .is_some_and(|x| x.eq_ignore_ascii_case(wanted))
}

fn pngs_under(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            pngs_under(&path, into);
        } else if has_extension(&path, "png") {
            into.push(path);
        }
    }
}

/// Every PNG in the mod's `Textures` folders: the one at its root and
/// those one folder down (a game version or an optional part). Sorted.
pub fn pngs(mod_root: &Path) -> Vec<PathBuf> {
    let is_textures = |p: &Path| {
        p.file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case("textures"))
    };
    let mut found = Vec::new();
    for entry in std::fs::read_dir(mod_root).into_iter().flatten().flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if is_textures(&path) {
            pngs_under(&path, &mut found);
            continue;
        }
        for inner in std::fs::read_dir(&path).into_iter().flatten().flatten() {
            if inner.path().is_dir() && is_textures(&inner.path()) {
                pngs_under(&inner.path(), &mut found);
            }
        }
    }
    found.sort();
    found
}

/// The textures of one mod, counted by what stands beside each PNG.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    pub textures: usize,
    /// A DDS RimStack made from the PNG as it is now.
    pub converted: usize,
    /// A DDS RimStack made from an earlier version of the PNG.
    pub out_of_date: usize,
    /// A DDS that RimStack did not make: the author's, or another tool's.
    pub shipped: usize,
    /// Bytes of all the PNG files, and of all the DDS files beside them.
    pub png_bytes: u64,
    pub dds_bytes: u64,
    /// Bytes of the DDS files RimStack made.
    pub made_bytes: u64,
}

impl Summary {
    pub fn not_converted(&self) -> usize {
        self.textures - self.converted - self.out_of_date - self.shipped
    }
}

pub fn summarize(mod_root: &Path, manifest: &Manifest) -> Summary {
    let mut summary = Summary::default();
    for png in pngs(mod_root) {
        summary.textures += 1;
        let source = fingerprint(&png);
        summary.png_bytes += source.map_or(0, |(len, _)| len);
        let dds = png.with_extension("dds");
        let Ok(dds_len) = std::fs::metadata(&dds).map(|m| m.len()) else {
            continue;
        };
        summary.dds_bytes += dds_len;
        match manifest.made.get(&dds) {
            None => summary.shipped += 1,
            Some(made) => {
                summary.made_bytes += dds_len;
                if source == Some((made.source_len, made.source_modified)) {
                    summary.converted += 1
                } else {
                    summary.out_of_date += 1
                }
            }
        }
    }
    summary
}

/// Encodes PNG bytes as the bytes of a DDS the game can load: flipped top
/// to bottom as the game expects, sides rounded up to a multiple of four,
/// with mipmaps. A picture with transparency becomes BC7 and one without
/// becomes DXT1, each under the header the DDS files already shipped for
/// the game use (19,770 looked at: BC7 in the newer "DX10" header, DXT1
/// in the older one, and no DXT1 in a DX10 header at all).
pub fn encode(png: &[u8]) -> Result<Vec<u8>, String> {
    let picture = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .map_err(|e| say!("not a readable PNG: {}", e))?;
    let mut picture = image::imageops::flip_vertical(&picture.to_rgba8());
    let round = |n: u32| n.div_ceil(4).max(1) * 4;
    let (wide, high) = (round(picture.width()), round(picture.height()));
    if (wide, high) != picture.dimensions() {
        let filter = image::imageops::FilterType::Triangle;
        picture = image::imageops::resize(&picture, wide, high, filter);
    }
    let failed = |e: &dyn std::fmt::Display| say!("could not be encoded: {}", e);
    let opaque = picture.pixels().all(|p| p.0[3] == 255);
    let format = if opaque {
        ImageFormat::BC1RgbaUnorm
    } else {
        ImageFormat::BC7RgbaUnorm
    };
    let surface = image_dds::SurfaceRgba8::from_image(&picture)
        .encode(format, Quality::Normal, Mipmaps::GeneratedAutomatic)
        .map_err(|e| failed(&e))?;
    if opaque {
        let mut bytes = dxt1_header(wide, high, surface.mipmaps);
        bytes.extend_from_slice(&surface.data);
        return Ok(bytes);
    }
    let mut bytes = Vec::new();
    let dds = surface.to_dds().map_err(|e| failed(&e))?;
    dds.write(&mut bytes).map_err(|e| failed(&e))?;
    Ok(bytes)
}

/// The 128 bytes that start a DXT1 file in the older DDS layout.
fn dxt1_header(wide: u32, high: u32, mipmaps: u32) -> Vec<u8> {
    const CAPS_HEIGHT_WIDTH_FORMAT_MIPS_SIZE: u32 = 0x000A_1007;
    const FOUR_CC: u32 = 0x4;
    const TEXTURE: u32 = 0x1000;
    const COMPLEX_WITH_MIPMAPS: u32 = 0x0040_0008;
    let top_level_bytes = (wide / 4) * (high / 4) * 8;
    let mut words: Vec<u32> = vec![
        124,
        CAPS_HEIGHT_WIDTH_FORMAT_MIPS_SIZE,
        high,
        wide,
        top_level_bytes,
        1,
        mipmaps,
    ];
    words.extend([0; 11]);
    words.extend([32, FOUR_CC, u32::from_le_bytes(*b"DXT1"), 0, 0, 0, 0, 0]);
    let caps = if mipmaps > 1 {
        TEXTURE | COMPLEX_WITH_MIPMAPS
    } else {
        TEXTURE
    };
    words.extend([caps, 0, 0, 0, 0]);
    let mut bytes = b"DDS ".to_vec();
    bytes.extend(words.iter().flat_map(|w| w.to_le_bytes()));
    bytes
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Converted {
    pub made: usize,
    pub failed: Vec<(PathBuf, String)>,
}

/// Makes a DDS for every PNG of the mod that has none, and again for
/// those whose PNG changed since. A DDS RimStack did not make is left
/// alone. `step` is called after each texture, from any thread.
pub fn convert(mod_root: &Path, manifest: &mut Manifest, step: &(dyn Fn() + Sync)) -> Converted {
    let todo: Vec<PathBuf> = pngs(mod_root)
        .into_iter()
        .filter(|png| {
            let dds = png.with_extension("dds");
            match manifest.made.get(&dds) {
                None => !dds.exists(),
                Some(made) => {
                    !dds.exists()
                        || fingerprint(png) != Some((made.source_len, made.source_modified))
                }
            }
        })
        .collect();
    let results: Vec<(PathBuf, Result<Made, String>)> = todo
        .into_par_iter()
        .map(|png| {
            let result = convert_one(&png);
            step();
            (png, result)
        })
        .collect();
    let mut outcome = Converted::default();
    for (png, result) in results {
        match result {
            Ok(made) => {
                manifest.made.insert(png.with_extension("dds"), made);
                outcome.made += 1;
            }
            Err(reason) => outcome.failed.push((png, reason)),
        }
    }
    outcome
}

fn convert_one(png: &Path) -> Result<Made, String> {
    let (source_len, source_modified) = fingerprint(png).ok_or_else(|| say!("cannot be read"))?;
    let bytes = std::fs::read(png).map_err(|e| e.to_string())?;
    let encoded = encode(&bytes)?;
    // Written whole under another name first: the game must never find
    // half a texture.
    let dds = png.with_extension("dds");
    let partial = png.with_extension("dds.rimstack-part");
    std::fs::write(&partial, &encoded).map_err(|e| e.to_string())?;
    std::fs::rename(&partial, &dds).map_err(|e| e.to_string())?;
    Ok(Made {
        source_len,
        source_modified,
        output_hash: hash(&encoded),
    })
}

/// Deletes the DDS files RimStack made in the mod, and only those that
/// are still byte for byte what it wrote. One that was changed since is
/// kept and forgotten. Returns how many were deleted.
pub fn remove(mod_root: &Path, manifest: &mut Manifest) -> usize {
    let ours: Vec<PathBuf> = manifest
        .made
        .keys()
        .filter(|dds| dds.starts_with(mod_root))
        .cloned()
        .collect();
    let mut removed = 0;
    for dds in ours {
        let made = manifest.made.remove(&dds).expect("listed above");
        let unchanged = std::fs::read(&dds).is_ok_and(|bytes| hash(&bytes) == made.output_hash);
        if unchanged && std::fs::remove_file(&dds).is_ok() {
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A small PNG; `alpha` below 255 makes it transparent.
    fn png(wide: u32, high: u32, alpha: u8) -> Vec<u8> {
        let picture = image::RgbaImage::from_fn(wide, high, |x, y| {
            image::Rgba([(x * 40) as u8, (y * 40) as u8, 90, alpha])
        });
        let mut bytes = std::io::Cursor::new(Vec::new());
        picture
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    fn a_mod() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("mod");
        for (file, alpha) in [
            ("Textures/Things/a.png", 255),
            ("1.6/Textures/b.PNG", 100),
            ("Textures/shipped.png", 255),
        ] {
            let path = root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, png(6, 5, alpha)).unwrap();
        }
        fs::write(root.join("Textures/shipped.dds"), "the author's own").unwrap();
        // Not textures: outside a Textures folder, or too deep.
        fs::create_dir_all(root.join("About")).unwrap();
        fs::write(root.join("About/Preview.png"), png(4, 4, 255)).unwrap();
        fs::create_dir_all(root.join("a/b/Textures")).unwrap();
        fs::write(root.join("a/b/Textures/deep.png"), png(4, 4, 255)).unwrap();
        (dir, root)
    }

    #[test]
    fn textures_are_found_at_the_root_and_one_folder_down_only() {
        let (_dir, root) = a_mod();
        let found = pngs(&root);
        let names: Vec<String> = found
            .iter()
            .map(|p| {
                p.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(
            names,
            [
                "1.6/Textures/b.PNG",
                "Textures/Things/a.png",
                "Textures/shipped.png"
            ]
        );
    }

    #[test]
    fn a_dds_has_the_format_for_its_transparency_and_sides_in_fours() {
        let side =
            |bytes: &[u8], at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let opaque = encode(&png(6, 5, 255)).unwrap();
        assert_eq!(&opaque[..4], b"DDS ");
        assert_eq!((side(&opaque, 16), side(&opaque, 12)), (8, 8));
        let transparent = encode(&png(8, 4, 100)).unwrap();
        assert_eq!((side(&transparent, 16), side(&transparent, 12)), (8, 4));
        // Opaque: the older header with "DXT1". Transparent: the DX10
        // header with format 98, BC7.
        assert_eq!(&opaque[84..88], b"DXT1");
        assert_eq!(
            (&transparent[84..88], side(&transparent, 128)),
            (&b"DX10"[..], 98)
        );
        // Header, then every mipmap of an 8 by 8 picture: 4 + 1 + 1 + 1 blocks of 8 bytes.
        assert_eq!(opaque.len(), 128 + 7 * 8);
        assert_eq!(side(&opaque, 28), 4);
        // What we write reads back as the picture's size with a standard reader.
        let read = image_dds::ddsfile::Dds::read(&opaque[..]).unwrap();
        assert_eq!((read.get_width(), read.get_height()), (8, 8));
        assert!(image_dds::image_from_dds(&read, 0).is_ok());
        assert!(encode(b"not a png").is_err());
    }

    #[test]
    fn an_opaque_picture_survives_dxt1_with_no_holes_and_close_colours() {
        // Gradients with a hard edge and a black patch: the kind of block
        // a careless DXT1 encoder turns transparent.
        let source = image::RgbaImage::from_fn(32, 32, |x, y| {
            if x < 8 && y < 8 {
                image::Rgba([0, 0, 0, 255])
            } else if x > 20 {
                image::Rgba([250, (y * 8) as u8, 30, 255])
            } else {
                image::Rgba([(x * 8) as u8, (y * 8) as u8, 128, 255])
            }
        });
        let mut png = std::io::Cursor::new(Vec::new());
        source.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let dds = encode(&png.into_inner()).unwrap();
        assert_eq!(&dds[84..88], b"DXT1");

        let read = image_dds::ddsfile::Dds::read(&dds[..]).unwrap();
        let decoded = image::imageops::flip_vertical(&image_dds::image_from_dds(&read, 0).unwrap());
        assert_eq!(decoded.dimensions(), (32, 32));
        assert!(
            decoded.pixels().all(|p| p.0[3] == 255),
            "a pixel became transparent"
        );
        let mut worst = 0;
        let mut total: u64 = 0;
        for (before, after) in source.pixels().zip(decoded.pixels()) {
            for channel in 0..3 {
                let off = before.0[channel].abs_diff(after.0[channel]);
                worst = worst.max(off);
                total += u64::from(off);
            }
        }
        let mean = total as f64 / (32.0 * 32.0 * 3.0);
        assert!(mean < 6.0, "mean colour error {mean}");
        // The hard edges fall on block borders, so no block has to blur them.
        assert!(worst < 48, "worst colour error {worst}");
    }

    #[test]
    fn converting_makes_what_is_missing_and_leaves_the_author_s_dds() {
        let (_dir, root) = a_mod();
        let mut manifest = Manifest::default();
        let before = summarize(&root, &manifest);
        assert_eq!(
            (before.textures, before.shipped, before.converted),
            (3, 1, 0)
        );
        assert_eq!(before.dds_bytes, "the author's own".len() as u64);

        let steps = std::sync::atomic::AtomicUsize::new(0);
        let outcome = convert(&root, &mut manifest, &|| {
            steps.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });
        assert_eq!((outcome.made, outcome.failed.len()), (2, 0));
        assert_eq!(steps.into_inner(), 2);
        assert_eq!(
            fs::read_to_string(root.join("Textures/shipped.dds")).unwrap(),
            "the author's own"
        );
        let after = summarize(&root, &manifest);
        assert_eq!(
            (after.converted, after.shipped, after.not_converted()),
            (2, 1, 0)
        );
        assert!(after.png_bytes > 0 && after.made_bytes > 0);
        assert_eq!(
            after.dds_bytes,
            after.made_bytes + "the author's own".len() as u64
        );
        // Nothing but the DDS files was added to the mod.
        assert!(!root.join("Textures/Things/a.dds.rimstack-part").exists());

        // A second run has nothing to do.
        assert_eq!(convert(&root, &mut manifest, &|| ()).made, 0);
    }

    #[test]
    fn a_changed_png_is_out_of_date_and_converted_again() {
        let (_dir, root) = a_mod();
        let mut manifest = Manifest::default();
        convert(&root, &mut manifest, &|| ());
        fs::write(root.join("Textures/Things/a.png"), png(12, 12, 255)).unwrap();
        assert_eq!(summarize(&root, &manifest).out_of_date, 1);
        assert_eq!(convert(&root, &mut manifest, &|| ()).made, 1);
        assert_eq!(summarize(&root, &manifest).out_of_date, 0);
    }

    #[test]
    fn a_bad_png_is_reported_and_the_rest_still_convert() {
        let (_dir, root) = a_mod();
        fs::write(root.join("Textures/broken.png"), "junk").unwrap();
        let mut manifest = Manifest::default();
        let outcome = convert(&root, &mut manifest, &|| ());
        assert_eq!(outcome.made, 2);
        assert_eq!(outcome.failed.len(), 1);
        assert!(outcome.failed[0].1.contains("not a readable PNG"));
        assert!(!root.join("Textures/broken.dds").exists());
    }

    #[test]
    fn removing_deletes_only_unchanged_files_rimstack_made() {
        let (_dir, root) = a_mod();
        let mut manifest = Manifest::default();
        convert(&root, &mut manifest, &|| ());
        // Someone replaced one of them since.
        fs::write(root.join("1.6/Textures/b.dds"), "replaced by hand").unwrap();

        assert_eq!(remove(&root, &mut manifest), 1);
        assert!(!root.join("Textures/Things/a.dds").exists());
        assert!(root.join("1.6/Textures/b.dds").exists());
        assert!(root.join("Textures/shipped.dds").exists());
        assert!(manifest.is_empty());
        assert!(root.join("Textures/Things/a.png").exists());
    }

    #[test]
    fn removing_from_one_mod_leaves_another_mod_s_files() {
        let (_dir, root) = a_mod();
        let (_other_dir, other) = a_mod();
        let mut manifest = Manifest::default();
        convert(&root, &mut manifest, &|| ());
        convert(&other, &mut manifest, &|| ());
        assert_eq!(remove(&root, &mut manifest), 2);
        assert_eq!(manifest.len(), 2);
        assert!(other.join("Textures/Things/a.dds").exists());
    }

    /// Checks the flip against DDS files other tools made for the game:
    /// decoded, ours must look like theirs, not like theirs upside down.
    #[test]
    #[ignore = "needs RIMSTACK_TEXTURE_PAIRS: a folder with .png files that have a .dds beside them"]
    fn our_dds_is_the_same_way_up_as_one_made_for_the_game() {
        let dir = PathBuf::from(std::env::var_os("RIMSTACK_TEXTURE_PAIRS").unwrap());
        let mut found = Vec::new();
        pngs_under(&dir, &mut found);
        let mut checked = 0;
        for png in found
            .iter()
            .filter(|p| p.with_extension("dds").is_file())
            .take(12)
        {
            let decode = |bytes: &[u8]| {
                let dds = image_dds::ddsfile::Dds::read(bytes).ok()?;
                image_dds::image_from_dds(&dds, 0).ok()
            };
            let Some(theirs) = decode(&fs::read(png.with_extension("dds")).unwrap()) else {
                continue;
            };
            let ours = decode(&encode(&fs::read(png).unwrap()).unwrap()).unwrap();
            if ours.dimensions() != theirs.dimensions() {
                continue;
            }
            let distance = |a: &image::RgbaImage, b: &image::RgbaImage| -> u64 {
                a.pixels()
                    .zip(b.pixels())
                    .map(|(p, q)| {
                        u64::from(p.0[3].abs_diff(q.0[3])) + u64::from(p.0[0].abs_diff(q.0[0]))
                    })
                    .sum()
            };
            let same = distance(&ours, &theirs);
            let upside_down = distance(&image::imageops::flip_vertical(&ours), &theirs);
            println!(
                "{}: same way up {same}, upside down {upside_down}",
                png.display()
            );
            assert!(same <= upside_down, "{}", png.display());
            checked += 1;
        }
        assert!(checked > 0, "no pairs found");
    }

    #[test]
    fn the_manifest_reads_back_as_written_and_damage_reads_as_empty() {
        let (dir, root) = a_mod();
        let mut manifest = Manifest::default();
        convert(&root, &mut manifest, &|| ());
        let path = dir.path().join("data").join("textures.json");
        manifest.write(&path).unwrap();
        assert_eq!(Manifest::read(&path), manifest);
        fs::write(&path, "{broken").unwrap();
        assert!(Manifest::read(&path).is_empty());
        assert!(Manifest::read(&dir.path().join("none.json")).is_empty());
    }
}
