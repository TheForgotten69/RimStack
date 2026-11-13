//! Small copies of pictures, cut to one size, kept together in one file so a list shows each
//! in the frame its row appears.
//!
//! The file, all numbers little-endian:
//! - header, 32 bytes: `RSPP`, version (u32), width (u32), height (u32), stamp (u64), where the
//!   index starts (u64);
//! - each copy's pixels, width × height × 4 bytes, BGRA;
//! - the index: a count (u32), then per picture its path's length (u32) and UTF-8 bytes, the
//!   original's byte size (u64) and modification time in ms (u64), and where its pixels start
//!   (u64), `u64::MAX` for a picture that could not be read.
//!
//! The stamp changes at each write: a reader holding an older index reads nothing from a newer
//! file rather than another picture's pixels.

use crate::base::error::Error;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 4] = b"RSPP";
const VERSION: u32 = 1;
const HEADER: u64 = 32;
const UNMADE: u64 = u64::MAX;
/// Copies made at once while writing: a first fill of 2,000 previews stays a few MB.
const BATCH: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Entry {
    bytes: u64,
    modified: u64,
    at: Option<u64>,
}

pub struct Pack {
    file: PathBuf,
    size: (u32, u32),
    stamp: u64,
    entries: HashMap<String, Entry>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Found {
    /// BGRA, width × height × 4 bytes.
    Pixels(Vec<u8>),
    /// The picture could not be read when the pack was filled.
    Unmade,
    /// Not in this pack, or the file was written again since it was opened.
    Absent,
}

impl Pack {
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// `None` when there is no pack at `file`, or one of another size or version.
    pub fn open(file: &Path, size: (u32, u32)) -> Option<Self> {
        let mut read = std::fs::File::open(file).ok()?;
        let header = Header::read(&mut read)?;
        if header.size != size {
            return None;
        }
        read.seek(SeekFrom::Start(header.index_at)).ok()?;
        let mut index = Vec::new();
        read.read_to_end(&mut index).ok()?;
        Some(Self {
            file: file.to_path_buf(),
            size,
            stamp: header.stamp,
            entries: parse_index(&index)?,
        })
    }

    pub fn found(&self, path: &str) -> Found {
        match self.entries.get(path) {
            None => Found::Absent,
            Some(Entry { at: None, .. }) => Found::Unmade,
            Some(Entry { at: Some(at), .. }) => match self.pixels_at(*at) {
                Some(pixels) => Found::Pixels(pixels),
                None => Found::Absent,
            },
        }
    }

    fn pixels_at(&self, at: u64) -> Option<Vec<u8>> {
        let mut read = std::fs::File::open(&self.file).ok()?;
        let header = Header::read(&mut read)?;
        if header.stamp != self.stamp {
            return None;
        }
        read.seek(SeekFrom::Start(at)).ok()?;
        let mut pixels = vec![0; pixel_len(self.size)];
        read.read_exact(&mut pixels).ok()?;
        Some(pixels)
    }

    /// The pack at `file` holding `wanted` and nothing else. A copy whose original kept its
    /// size and time is taken from the pack there; the rest are made again. Nothing is written
    /// when every copy is current.
    pub fn fill(file: &Path, size: (u32, u32), wanted: &[PathBuf]) -> Result<Self, Error> {
        let old = Self::open(file, size);
        let mut seen = std::collections::HashSet::new();
        let now: Vec<Original> = wanted
            .iter()
            .filter(|path| seen.insert(path.to_string_lossy().into_owned()))
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|path| Original::of(path))
            .collect();
        let kept = |original: &Original| {
            old.as_ref()?
                .entries
                .get(&original.key)
                .filter(|e| e.bytes == original.bytes && e.modified == original.modified)
                .copied()
        };
        if let Some(old) = &old
            && old.entries.len() == now.len()
            && now.iter().all(|original| kept(original).is_some())
        {
            return Ok(Self {
                file: old.file.clone(),
                size,
                stamp: old.stamp,
                entries: old.entries.clone(),
            });
        }
        let failed = |e: std::io::Error| Error::file(file, e);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(failed)?;
        }
        let real = crate::base::atomic_file::through_links(file);
        let partial = crate::base::atomic_file::partial_beside(&real);
        let written = (|| {
            let mut out = BufWriter::new(std::fs::File::create(&partial)?);
            out.write_all(&[0; HEADER as usize])?;
            let mut at = HEADER;
            let mut entries = HashMap::with_capacity(now.len());
            for batch in now.chunks(BATCH) {
                let made: Vec<Option<Vec<u8>>> = batch
                    .par_iter()
                    .map(|original| match kept(original) {
                        Some(Entry { at: Some(from), .. }) => old
                            .as_ref()
                            .and_then(|old| old.pixels_at(from))
                            .or_else(|| crop(&original.path, size)),
                        Some(Entry { at: None, .. }) => None,
                        None => crop(&original.path, size),
                    })
                    .collect();
                for (original, pixels) in batch.iter().zip(made) {
                    let place = pixels.map(|pixels| {
                        let here = at;
                        at += pixels.len() as u64;
                        out.write_all(&pixels).map(|()| here)
                    });
                    let entry = Entry {
                        bytes: original.bytes,
                        modified: original.modified,
                        at: place.transpose()?,
                    };
                    entries.insert(original.key.clone(), entry);
                }
            }
            out.write_all(&index_bytes(&entries))?;
            let stamp = new_stamp(old.as_ref().map(|old| old.stamp));
            out.seek(SeekFrom::Start(0))?;
            out.write_all(
                &Header {
                    size,
                    stamp,
                    index_at: at,
                }
                .bytes(),
            )?;
            let file = out.into_inner().map_err(|e| e.into_error())?;
            crate::base::atomic_file::settle(file, &partial, &real)?;
            Ok((stamp, entries))
        })();
        match written {
            Ok((stamp, entries)) => Ok(Self {
                file: file.to_path_buf(),
                size,
                stamp,
                entries,
            }),
            Err(problem) => {
                let _ = std::fs::remove_file(&partial);
                Err(failed(problem))
            }
        }
    }
}

struct Original {
    key: String,
    path: PathBuf,
    bytes: u64,
    modified: u64,
}

impl Original {
    fn of(path: &Path) -> Self {
        let bytes = std::fs::metadata(path).map_or(0, |m| m.len());
        Self {
            key: path.to_string_lossy().into_owned(),
            path: path.to_path_buf(),
            bytes,
            modified: crate::base::watch::changed_at(path),
        }
    }
}

/// The picture at `path` cut to fill `width` by `height` from its middle, as BGRA.
pub fn crop(path: &Path, (width, height): (u32, u32)) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    let picture = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let mut shown = picture
        .resize_to_fill(width, height, image::imageops::FilterType::Triangle)
        .to_rgba8();
    for pixel in shown.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    Some(shown.into_raw())
}

fn pixel_len((width, height): (u32, u32)) -> usize {
    width as usize * height as usize * 4
}

fn new_stamp(old: Option<u64>) -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64);
    match old {
        Some(old) if old == now => now.wrapping_add(1),
        _ => now,
    }
}

struct Header {
    size: (u32, u32),
    stamp: u64,
    index_at: u64,
}

impl Header {
    fn read(file: &mut std::fs::File) -> Option<Self> {
        let mut bytes = [0; HEADER as usize];
        file.read_exact(&mut bytes).ok()?;
        let mut at = Cursor(&bytes);
        if at.take(4)? != MAGIC || at.u32()? != VERSION {
            return None;
        }
        Some(Self {
            size: (at.u32()?, at.u32()?),
            stamp: at.u64()?,
            index_at: at.u64()?,
        })
    }

    fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER as usize);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&self.size.0.to_le_bytes());
        out.extend_from_slice(&self.size.1.to_le_bytes());
        out.extend_from_slice(&self.stamp.to_le_bytes());
        out.extend_from_slice(&self.index_at.to_le_bytes());
        out
    }
}

fn index_bytes(entries: &HashMap<String, Entry>) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (key, entry) in entries {
        out.extend_from_slice(&(key.len() as u32).to_le_bytes());
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(&entry.bytes.to_le_bytes());
        out.extend_from_slice(&entry.modified.to_le_bytes());
        out.extend_from_slice(&entry.at.unwrap_or(UNMADE).to_le_bytes());
    }
    out
}

fn parse_index(bytes: &[u8]) -> Option<HashMap<String, Entry>> {
    let mut at = Cursor(bytes);
    let count = at.u32()? as usize;
    let mut entries = HashMap::with_capacity(count.min(bytes.len() / 28));
    for _ in 0..count {
        let len = at.u32()? as usize;
        let key = String::from_utf8(at.take(len)?.to_vec()).ok()?;
        let entry = Entry {
            bytes: at.u64()?,
            modified: at.u64()?,
            at: Some(at.u64()?).filter(|&at| at != UNMADE),
        };
        entries.insert(key, entry);
    }
    Some(entries)
}

struct Cursor<'a>(&'a [u8]);

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.0.len() < n {
            return None;
        }
        let (taken, rest) = self.0.split_at(n);
        self.0 = rest;
        Some(taken)
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(path: &Path, width: u32, height: u32, rgb: [u8; 3]) {
        let shown =
            image::RgbaImage::from_pixel(width, height, image::Rgba([rgb[0], rgb[1], rgb[2], 255]));
        shown.save(path).unwrap();
    }

    #[test]
    fn copies_are_cut_to_the_size_and_read_back_in_bgra() {
        let tmp = tempfile::tempdir().unwrap();
        let red = tmp.path().join("red.png");
        picture(&red, 64, 32, [255, 0, 0]);
        let file = tmp.path().join("cache/previews.pack");
        let pack = Pack::fill(&file, (8, 4), std::slice::from_ref(&red)).unwrap();
        let Found::Pixels(pixels) = pack.found(&red.to_string_lossy()) else {
            panic!("no copy");
        };
        assert_eq!(pixels.len(), 8 * 4 * 4);
        assert_eq!(&pixels[..4], &[0, 0, 255, 255]);
        let again = Pack::open(&file, (8, 4)).unwrap();
        assert_eq!(again.found(&red.to_string_lossy()), Found::Pixels(pixels));
        assert!(Pack::open(&file, (9, 4)).is_none());
    }

    #[test]
    fn a_picture_that_cannot_be_read_is_kept_as_unmade_until_it_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let broken = tmp.path().join("Preview.png");
        std::fs::write(&broken, b"not a picture").unwrap();
        let file = tmp.path().join("previews.pack");
        let wanted = [broken.clone(), tmp.path().join("gone.png")];
        let pack = Pack::fill(&file, (4, 4), &wanted).unwrap();
        assert_eq!(pack.found(&broken.to_string_lossy()), Found::Unmade);
        assert_eq!(pack.found("elsewhere.png"), Found::Absent);
        let stamp = pack.stamp;
        assert_eq!(Pack::fill(&file, (4, 4), &wanted).unwrap().stamp, stamp);
        picture(&broken, 4, 4, [0, 255, 0]);
        let filetime = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        std::fs::File::options()
            .write(true)
            .open(&broken)
            .unwrap()
            .set_modified(filetime)
            .unwrap();
        let refilled = Pack::fill(&file, (4, 4), &wanted).unwrap();
        assert!(matches!(
            refilled.found(&broken.to_string_lossy()),
            Found::Pixels(_)
        ));
    }

    #[test]
    fn a_fill_keeps_current_copies_drops_what_is_no_longer_wanted_and_voids_older_readers() {
        let tmp = tempfile::tempdir().unwrap();
        let (one, two) = (tmp.path().join("one.png"), tmp.path().join("two.png"));
        picture(&one, 4, 4, [1, 2, 3]);
        picture(&two, 4, 4, [4, 5, 6]);
        let file = tmp.path().join("previews.pack");
        let first = Pack::fill(&file, (2, 2), &[one.clone(), two.clone()]).unwrap();
        let second = Pack::fill(&file, (2, 2), std::slice::from_ref(&two)).unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second.found(&one.to_string_lossy()), Found::Absent);
        assert!(
            matches!(second.found(&two.to_string_lossy()), Found::Pixels(p) if p[..3] == [6, 5, 4])
        );
        assert_eq!(first.found(&two.to_string_lossy()), Found::Absent);
    }
}
