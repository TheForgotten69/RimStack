//! A size in a view follows the text size or a measure, never a fixed count of pixels: a
//! two-digit `px(…)` belongs in `theme.rs`, where the Theme's tokens are, as `gates.rs` held it
//! for the QML. A hairline (a border, a bar) stays a pixel or three.

use std::path::Path;

fn views(dir: &Path, into: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if path.is_dir() {
            views(&path, into);
        } else if name.ends_with(".rs")
            && !name.ends_with("tests.rs")
            && !matches!(name.as_str(), "theme.rs" | "gates.rs" | "layout_checks.rs")
        {
            into.push(path);
        }
    }
}

/// The numbers written right after `px(` in `line`.
fn pixels(line: &str) -> Vec<String> {
    line.match_indices("px(")
        .map(|(at, _)| line[at + 3..].trim_start().trim_start_matches('-'))
        .map(|rest| rest.chars().take_while(|c| c.is_ascii_digit()).collect())
        .collect()
}

#[test]
fn no_view_sizes_a_thing_in_fixed_pixels() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    let mut files = Vec::new();
    views(&root, &mut files);
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        let shown = text.split("#[cfg(test)]").next().unwrap_or_default();
        for (n, line) in shown.lines().enumerate() {
            if pixels(line).iter().any(|digits| digits.len() >= 2) {
                found.push(format!(
                    "{}:{}: {}",
                    file.strip_prefix(&root).unwrap().display(),
                    n + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        found.is_empty(),
        "sizes in fixed pixels; use a Theme token or a measure:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_check_sees_a_fixed_size() {
    assert_eq!(pixels("div().w(px(120.)).h(px(2.))"), ["120", "2"]);
    assert_eq!(pixels(".top(px(-12.))"), ["12"]);
}
