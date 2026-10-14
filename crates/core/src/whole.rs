//! A file written so that a crash, or the power going, leaves the one
//! before: whole under another name and on the disk, then moved into
//! place.

use crate::error::Error;
use std::path::Path;

/// Makes the file's folder if need be. The error names the file.
pub fn write(path: &Path, contents: impl AsRef<[u8]>) -> Result<(), Error> {
    let failed = |e: std::io::Error| Error::file(path, e);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(failed)?;
    }
    let mut partial = path.as_os_str().to_os_string();
    partial.push(".part");
    put(Path::new(&partial), path, contents.as_ref()).map_err(failed)
}

/// Writes `partial`, waits for it to be on the disk, then gives it the
/// name `path`. Without the wait a rename can outlive a power cut that
/// the bytes did not: the file would be there, and empty.
pub(crate) fn put(partial: &Path, path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(partial)?;
    file.write_all(contents)?;
    file.sync_all()?;
    std::fs::rename(partial, path)
}

fn damaged(path: &Path) -> std::path::PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".damaged");
    name.into()
}

/// The JSON a file of RimStack's own holds; nothing for a file that is
/// not there. One that is there and is not JSON is first copied to
/// `<name>.damaged`, because whoever reads nothing starts afresh and the
/// next write would replace what the user had; the record says so.
pub fn read_json(path: &Path) -> Option<serde_json::Value> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            crate::problem!("{}: {e}", path.display());
            return None;
        }
    };
    match serde_json::from_str(&text) {
        Ok(value) => Some(value),
        Err(e) => {
            let kept = damaged(path);
            match std::fs::copy(path, &kept) {
                Ok(_) => crate::problem!("{}: {e}; kept as {}", path.display(), kept.display()),
                Err(not_kept) => crate::problem!("{}: {e}; not kept: {not_kept}", path.display()),
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_that_is_not_json_reads_as_nothing_and_is_kept_beside_itself() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        assert_eq!(read_json(&path), None);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);

        write(&path, r#"{ "a.b": 1 }"#).unwrap();
        assert_eq!(read_json(&path), Some(serde_json::json!({ "a.b": 1 })));
        assert!(!damaged(&path).exists());

        std::fs::write(&path, r#"{ "a.b": { "note": "two years of no"#).unwrap();
        assert_eq!(read_json(&path), None);
        write(&path, "{}").unwrap();
        let kept = std::fs::read_to_string(damaged(&path)).unwrap();
        assert!(kept.ends_with("two years of no"));
    }

    #[test]
    fn the_file_and_its_folder_are_made_and_nothing_else_is_left() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("RimStack").join("notes.v2.json");
        write(&path, "first").unwrap();
        write(&path, "second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1
        );
    }

    #[test]
    fn a_file_that_cannot_be_written_keeps_what_it_held() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        write(&path, "kept").unwrap();
        // A folder in the way of the partial file.
        std::fs::create_dir(dir.path().join("settings.json.part")).unwrap();
        let lost = write(&path, "lost").unwrap_err();
        assert_eq!(lost.kind(), crate::error::Kind::File);
        assert!(lost.to_string().contains("settings.json"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "kept");
    }
}
