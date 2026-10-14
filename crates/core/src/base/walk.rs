//! Every file under a folder. A mod is a stranger's, and a link in it may
//! point back up: a folder is entered once however it is reached, so a
//! loop of links ends at once. The one walk every folder of a mod takes.

use std::path::{Path, PathBuf};

pub struct Entry {
    pub path: PathBuf,
    /// Its path from where the walk began.
    pub within: PathBuf,
    pub is_dir: bool,
}

/// What is under `dir`, folders and files, at most `deepest` folders
/// down, in no order. A link to a folder is followed (a mod being worked
/// on may keep its textures elsewhere) unless it leads into, or above, a
/// folder already walked.
/// A folder that cannot be read is passed over.
pub fn tree(dir: &Path, deepest: usize) -> Vec<Entry> {
    walk(dir, deepest, false).unwrap_or_default()
}

/// As `tree`, failing on a folder or entry that cannot be read: for a
/// copy, which must not be missing a part.
pub fn whole_tree(dir: &Path) -> std::io::Result<Vec<Entry>> {
    walk(dir, usize::MAX, true)
}

fn walk(dir: &Path, deepest: usize, whole: bool) -> std::io::Result<Vec<Entry>> {
    let mut found = Vec::new();
    let mut entered: Vec<PathBuf> = Vec::new();
    let mut waiting = vec![(dir.to_path_buf(), PathBuf::new(), 0)];
    while let Some((folder, within, depth)) = waiting.pop() {
        let entries = match std::fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(problem) if whole => return Err(problem),
            Err(_) => continue,
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(problem) if whole => return Err(problem),
                Err(_) => continue,
            };
            let (path, inside) = (entry.path(), within.join(entry.file_name()));
            let is_dir = match entry.file_type() {
                Ok(kind) if kind.is_dir() => true,
                Ok(kind) if kind.is_symlink() && path.is_dir() => {
                    if !may_enter(dir, &path, &mut entered) {
                        continue;
                    }
                    true
                }
                _ => false,
            };
            if is_dir && depth < deepest {
                waiting.push((path.clone(), inside.clone(), depth + 1));
            }
            found.push(Entry {
                path,
                within: inside,
                is_dir,
            });
        }
    }
    Ok(found)
}

/// The files under `dir`, at any depth, in no order.
pub fn files(dir: &Path) -> Vec<PathBuf> {
    let all = tree(dir, usize::MAX).into_iter();
    all.filter(|entry| !entry.is_dir)
        .map(|entry| entry.path)
        .collect()
}

/// Copies everything under `from` into `to`: what a link points at is
/// copied in its place, and a link back into what is copied is left out.
pub fn copy(from: &Path, to: &Path) -> std::io::Result<()> {
    let all = whole_tree(from)?;
    std::fs::create_dir_all(to)?;
    for entry in all {
        let target = to.join(&entry.within);
        match entry.is_dir {
            true => std::fs::create_dir_all(&target)?,
            false => drop(std::fs::copy(&entry.path, &target)?),
        }
    }
    Ok(())
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

    #[test]
    fn a_walk_stops_at_its_depth_and_names_what_it_found_from_where_it_began() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
        fs::write(tmp.path().join("a/b/c/deep.txt"), "").unwrap();
        let mut near: Vec<PathBuf> = tree(tmp.path(), 1).into_iter().map(|e| e.within).collect();
        near.sort();
        assert_eq!(near, [PathBuf::from("a"), PathBuf::from("a/b")]);
    }

    #[test]
    fn a_linked_folder_is_copied_as_what_it_points_at_and_a_loop_ends() {
        let tmp = tempfile::tempdir().unwrap();
        let (from, shared) = (tmp.path().join("mod"), tmp.path().join("shared"));
        std::fs::create_dir_all(&from).unwrap();
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::write(shared.join("a.png"), "png").unwrap();
        symlink(&shared, from.join("Textures")).unwrap();
        let to = tmp.path().join("copy");
        copy(&from, &to).unwrap();
        assert_eq!(
            std::fs::read_to_string(to.join("Textures/a.png")).unwrap(),
            "png"
        );
        assert!(!to.join("Textures").is_symlink());

        symlink(&from, from.join("itself")).unwrap();
        let again = tmp.path().join("again");
        copy(&from, &again).unwrap();
        assert!(again.join("Textures/a.png").is_file());
        assert!(!again.join("itself").exists());
    }
}
