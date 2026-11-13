//! A size in a view follows the text size or a measure, never a fixed count of pixels: a
//! two-digit `px(…)` belongs in `theme.rs`, where the Theme's tokens are. A hairline (a border,
//! a bar) stays a pixel or three.

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

/// The whole numbers written inside each `px(…)` of `line`, as `px(x + 100.)` holds `100`.
fn pixels(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (at, _) in line.match_indices("px(") {
        let before = line[..at].chars().next_back().unwrap_or(' ');
        if before.is_alphanumeric() || matches!(before, '_' | '.') {
            continue;
        }
        let mut depth = 1;
        let inside: String = line[at + 3..]
            .chars()
            .take_while(|c| {
                depth += match c {
                    '(' => 1,
                    ')' => -1,
                    _ => 0,
                };
                depth > 0
            })
            .collect();
        let mut after = ' ';
        let mut run = String::new();
        for c in inside.chars().chain([' ']) {
            match c.is_ascii_digit() {
                true if run.is_empty()
                    && (after.is_alphanumeric() || matches!(after, '_' | '.')) => {}
                true => run.push(c),
                false if !run.is_empty() => found.push(std::mem::take(&mut run)),
                false => {}
            }
            if !c.is_ascii_digit() || run.is_empty() {
                after = c;
            }
        }
    }
    found
}

#[test]
fn no_view_sizes_a_thing_in_fixed_pixels() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    let mut files = Vec::new();
    views(&root, &mut files);
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        let shown = text
            .split("#[cfg(test)]\nmod tests {")
            .next()
            .unwrap_or_default();
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
    assert_eq!(pixels("px(width + 100.)"), ["100"]);
    assert_eq!(pixels("px(0.25).max(px(f(2) * 3.5))"), ["0", "2", "3"]);
    assert!(pixels("px(size_16)").is_empty());
    assert!(pixels(".px(theme.sized(40.)).mx_px(12)").is_empty());
}
