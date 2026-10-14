//! The record as the app uses it: one for the process, so in a process
//! of its own.

use rimstack_core::{diagnostics, note, problem};

#[test]
fn notes_problems_and_a_panic_on_another_thread_are_all_in_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("RimStack").join("diagnostics.log");
    note!("before the file is open: only printed");
    diagnostics::start(&file).unwrap();
    diagnostics::record_panics();
    note!("scan_ms={}", 12);
    problem!("{}: denied", "notes.json");
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
    assert!(lines[2].contains("Z crash index out of bounds"), "{kept}");
    assert!(lines[2].contains("tests/diagnostics.rs:"), "{kept}");
    assert!(lines.len() > 4 && lines[3].starts_with("    "), "{kept}");
}
