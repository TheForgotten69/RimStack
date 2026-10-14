//! Every file under a folder of a mod. A mod is a stranger's, and a link
//! in it may point back up: a folder is walked once however it is reached.

use std::path::{Path, PathBuf};

/// The files under `dir`, at any depth, in no order. A link to a folder
/// is followed (a mod being worked on may keep its textures elsewhere)
/// unless it leads into, or above, a folder already walked.
pub fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut entered: Vec<PathBuf> = Vec::new();
    let mut waiting = vec![dir.to_path_buf()];
    while let Some(folder) = waiting.pop() {
        for entry in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => waiting.push(path),
                Ok(kind) if kind.is_symlink() && path.is_dir() => {
                    if may_enter(dir, &path, &mut entered) {
                        waiting.push(path);
                    }
                }
                _ => found.push(path),
            }
        }
    }
    found
}

/// `entered` holds where the walk began and where each link followed
/// leads, with no link left in the path; filled when the first link is met.
fn may_enter(began: &Path, link: &Path, entered: &mut Vec<PathBuf>) -> bool {
    if entered.is_empty() {
        entered.extend(std::fs::canonicalize(began));
    }
    let Ok(leads_to) = std::fs::canonicalize(link) else {
        return false;
    };
    let apart = |walked: &PathBuf| !leads_to.starts_with(walked) && !walked.starts_with(&leads_to);
    let new = !entered.is_empty() && entered.iter().all(apart);
    if new {
        entered.push(leads_to);
    }
    new
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    fn names(dir: &Path) -> Vec<String> {
        let mut found: Vec<String> = files(dir)
            .iter()
            .map(|file| {
                file.strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        found.sort();
        found
    }

    #[test]
    fn files_are_found_at_any_depth_and_a_missing_folder_has_none() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("a/b")).unwrap();
        fs::write(tmp.path().join("top.txt"), "").unwrap();
        fs::write(tmp.path().join("a/b/deep.txt"), "").unwrap();
        assert_eq!(names(tmp.path()), ["a/b/deep.txt", "top.txt"]);
        assert!(files(&tmp.path().join("absent")).is_empty());
    }

    #[test]
    fn links_back_up_and_to_each_other_are_not_followed() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir_all(root.join("a")).unwrap();
        fs::write(root.join("a/one.txt"), "").unwrap();
        symlink(&root, root.join("a/up")).unwrap();
        symlink(root.join("a"), root.join("beside")).unwrap();
        symlink(tmp.path(), root.join("above")).unwrap();
        assert_eq!(names(&root), ["a/one.txt"]);
    }

    #[test]
    fn a_link_to_a_folder_elsewhere_is_followed_once() {
        let tmp = tempfile::tempdir().unwrap();
        let (root, elsewhere) = (tmp.path().join("root"), tmp.path().join("elsewhere"));
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(elsewhere.join("inner")).unwrap();
        fs::write(elsewhere.join("inner/far.txt"), "").unwrap();
        symlink(&elsewhere, root.join("first")).unwrap();
        symlink(elsewhere.join("inner"), root.join("second")).unwrap();
        symlink(&elsewhere, elsewhere.join("inner/round")).unwrap();
        assert_eq!(files(&root).len(), 1);
    }
}
