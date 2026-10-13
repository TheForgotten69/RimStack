//! Noticing that something on disk changed while RimStack was open,
//! without being told: by when it was last written.

use std::path::Path;

/// When the file was last written, or an entry was added to or removed
/// from the folder, in milliseconds since 1970; 0 when it is not there.
pub fn changed_at(path: &Path) -> u64 {
    let written = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let since = written.and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok());
    since.map_or(0, |d| d.as_millis() as u64)
}

/// The newest of several, so that a change to any one shows.
pub fn newest<'a>(paths: impl IntoIterator<Item = &'a Path>) -> u64 {
    paths.into_iter().map(changed_at).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_has_no_time_and_a_new_entry_changes_its_folder() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(changed_at(&tmp.path().join("none")), 0);
        let before = changed_at(tmp.path());
        assert!(before > 0);
        // A file system keeps times to the millisecond at best.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::create_dir(tmp.path().join("mod")).unwrap();
        let after = changed_at(tmp.path());
        assert!(after > before);
        assert_eq!(
            newest([tmp.path(), tmp.path().join("none").as_path()]),
            after
        );
        assert_eq!(newest([]), 0);
    }
}
