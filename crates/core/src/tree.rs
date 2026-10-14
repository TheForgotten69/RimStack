//! What is in a mod's folder, for showing.

use std::path::{Path, PathBuf};

/// A mod's folders two levels deep, each with how many files it holds
/// (counting everything below it): `("Textures/Things/", 612)`.
pub fn folder_tree(root: &Path) -> Vec<(String, usize)> {
    fn files_under(dir: &Path) -> usize {
        let entries = std::fs::read_dir(dir).into_iter().flatten().flatten();
        entries
            .map(|e| match e.path() {
                p if p.is_dir() => files_under(&p),
                _ => 1,
            })
            .sum()
    }
    fn folders(dir: &Path) -> Vec<(String, PathBuf)> {
        let entries = std::fs::read_dir(dir).into_iter().flatten().flatten();
        let mut found: Vec<(String, PathBuf)> = entries
            .filter(|e| e.path().is_dir())
            .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
            .filter(|(name, _)| !name.starts_with('.'))
            .collect();
        found.sort();
        found
    }
    let mut lines = Vec::new();
    for (name, path) in folders(root) {
        let inner = folders(&path);
        if inner.is_empty() || inner.len() > 12 {
            lines.push((format!("{name}/"), files_under(&path)));
        } else {
            for (inner_name, inner_path) in inner {
                lines.push((format!("{name}/{inner_name}/"), files_under(&inner_path)));
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn folders_are_listed_two_deep_with_their_files_counted() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        for file in [
            "About/About.xml",
            "Textures/Things/a.png",
            "Textures/Things/Deep/b.png",
            "Textures/UI/c.png",
            ".git/config",
            "readme.txt",
        ] {
            let path = root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        // Hidden folders and loose files are left out; a folder with no
        // folders in it is one line.
        assert_eq!(
            folder_tree(root),
            [
                ("About/".to_string(), 1),
                ("Textures/Things/".to_string(), 2),
                ("Textures/UI/".to_string(), 1)
            ]
        );
        assert!(folder_tree(&root.join("nowhere")).is_empty());
    }
}
