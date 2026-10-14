//! The record as the app uses it: one for the process, so in a process
//! of its own.

use rimstack_core::base::error::Error;
use rimstack_core::{base::diagnostics, base::translate, note, problem, said};

#[test]
fn notes_problems_and_a_panic_on_another_thread_are_all_in_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("RimStack").join("diagnostics.log");
    note!("before the file is open: only printed");
    diagnostics::start(&file).unwrap();
    diagnostics::record_panics();
    note!("scan_ms={}", 12);
    problem!("{}: denied", "notes.json");
    // In French on screen, in English in the record.
    let french = [(
        "{} is not a setting".to_string(),
        "{} n'est pas un réglage".to_string(),
    )];
    translate::set(french.into_iter().collect());
    let refused = Error::refused(said!("{} is not a setting", "gameDir"));
    assert_eq!(refused.to_string(), "gameDir n'est pas un réglage");
    problem!("{refused}");
    translate::set(Default::default());
    let crashed = std::thread::spawn(|| {
        let rows: Vec<u8> = Vec::new();
        rows[3]
    })
    .join();
    assert!(crashed.is_err());

    let kept = std::fs::read_to_string(&file).unwrap();
    let lines: Vec<&str> = kept.lines().collect();
    assert!(lines[0].ends_with("Z note scan_ms=12"), "{kept}");
    assert!(lines[1].ends_with("Z problem notes.json: denied"), "{kept}");
    assert!(
        lines[2].ends_with("Z problem gameDir is not a setting"),
        "{kept}"
    );
    assert!(lines[3].contains("Z crash index out of bounds"), "{kept}");
    assert!(lines[3].contains("tests/diagnostics.rs:"), "{kept}");
    assert!(lines.len() > 5 && lines[4].starts_with("    "), "{kept}");
}
