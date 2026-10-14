//! The bridges QML reaches only gather what the workspace gives: no file,
//! no program and no download is touched from them, and no thread is
//! started but through `jobs`.

use std::path::Path;

const REFUSED: [&str; 6] = [
    "std::fs",
    "fs::",
    "std::process",
    "rimstack_core::sources",
    "thread::spawn",
    "thread::Builder",
];

/// The bridge files, as `build.rs` names them.
pub(crate) fn bridges() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let build = std::fs::read_to_string(root.join("build.rs")).unwrap();
    let named: Vec<String> = build
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"')?.strip_suffix("\","))
        .filter(|file| file.starts_with("src/") && file.ends_with(".rs"))
        .map(String::from)
        .collect();
    assert!(named.len() > 10, "the bridges were not found in build.rs");
    named
}

#[test]
fn a_bridge_reaches_files_programs_and_threads_only_through_the_workspace_and_jobs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for file in bridges() {
        let text = std::fs::read_to_string(root.join(&file)).unwrap();
        for (at, line) in text.lines().enumerate() {
            for refused in REFUSED.iter().filter(|refused| line.contains(*refused)) {
                found.push(format!("{file}:{}: {refused}", at + 1));
            }
        }
    }
    assert!(found.is_empty(), "{found:#?}");
}
