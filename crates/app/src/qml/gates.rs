//! What the QML may not do, checked on its files: it hands Rust no JSON
//! text and reads no command line, and its functions and timers, where
//! logic hides, only grow fewer.

use std::path::{Path, PathBuf};

/// The most there may be; lower them as QML loses some, never raise them.
const MOST_FUNCTIONS: usize = 189;
const MOST_TIMERS: usize = 19;

fn qml_files(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        match path.is_dir() {
            true => qml_files(&path, into),
            false if path.extension().is_some_and(|x| x == "qml") => into.push(path),
            false => {}
        }
    }
}

fn every_qml() -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    qml_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("qml"),
        &mut found,
    );
    assert!(found.len() > 40, "the QML was not found");
    found
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).unwrap();
            (path, text)
        })
        .collect()
}

/// Lines that are not comments, with their file and number.
fn code_lines(all: &[(PathBuf, String)]) -> Vec<(String, String)> {
    let mut lines = Vec::new();
    for (path, text) in all {
        for (at, line) in text.lines().enumerate() {
            if !line.trim_start().starts_with("//") {
                lines.push((format!("{}:{}", path.display(), at + 1), line.to_string()));
            }
        }
    }
    lines
}

#[test]
fn the_qml_hands_over_no_json_and_reads_no_command_line() {
    let lines = code_lines(&every_qml());
    let found: Vec<&String> = lines
        .iter()
        .filter(|(_, line)| line.contains("JSON.") || line.contains("Qt.application.arguments"))
        .map(|(at, _)| at)
        .collect();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_qml_s_functions_and_timers_only_grow_fewer() {
    let lines = code_lines(&every_qml());
    let count = |word: &str| {
        lines
            .iter()
            .filter(|(_, line)| {
                let line = line.trim_start();
                line.starts_with(word) || line.contains(&format!(" {word}"))
            })
            .count()
    };
    let (functions, timers) = (count("function "), count("Timer {"));
    assert!(
        functions <= MOST_FUNCTIONS,
        "{functions} functions, at most {MOST_FUNCTIONS}"
    );
    assert!(
        timers <= MOST_TIMERS,
        "{timers} timers, at most {MOST_TIMERS}"
    );
}
