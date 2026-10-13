//! `ModsConfig.xml`: the game's own record of which mods are active.
//! Writing changes only the list of active mods and leaves the rest of the
//! file exactly as the game wrote it.

use crate::say;

use roxmltree::{Document, Node};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModsConfig {
    /// As the game wrote it: "1.6.4871 rev598".
    pub version: Option<String>,
    /// Package ids in load order, as written.
    pub active: Vec<String>,
    pub known_expansions: Vec<String>,
}

pub fn read(path: &Path) -> Result<ModsConfig, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn parse(text: &str) -> Result<ModsConfig, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let doc = Document::parse(text).map_err(|e| say!("malformed XML: {}", e))?;
    let child = |name: &str| {
        doc.root_element()
            .children()
            .find(|n| n.tag_name().name().eq_ignore_ascii_case(name))
    };
    let items = |node: Node| -> Vec<String> {
        node.children()
            .filter(|n| n.tag_name().name().eq_ignore_ascii_case("li"))
            .filter_map(|n| n.text())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    let active = child("activeMods").ok_or_else(|| say!("no activeMods element"))?;
    Ok(ModsConfig {
        version: child("version")
            .and_then(|n| n.text())
            .map(|s| s.trim().to_string()),
        active: items(active),
        known_expansions: child("knownExpansions").map(items).unwrap_or_default(),
    })
}

/// `text` with its active mods replaced by `ids`, everything else byte for
/// byte as it was.
pub fn with_active(text: &str, ids: &[String]) -> Result<String, String> {
    const OPEN: &str = "<activeMods>";
    const CLOSE: &str = "</activeMods>";
    let (start, end) = match (text.find(OPEN), text.find(CLOSE)) {
        (Some(open), Some(close)) if open < close => (open, close + CLOSE.len()),
        _ => {
            // An empty list may be written as one self-closing tag.
            let start = text
                .find("<activeMods")
                .ok_or_else(|| say!("no activeMods element"))?;
            let length = text[start..]
                .find("/>")
                .ok_or_else(|| say!("no activeMods element"))?
                + 2;
            (start, start + length)
        }
    };
    // Keep the file's own line ends and indentation.
    let line_end = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let indent = &text[line_start..start];
    let indent = if indent.trim().is_empty() { indent } else { "" };
    let step = if indent.starts_with('\t') { "\t" } else { "  " };

    let mut block = String::from(OPEN);
    for id in ids {
        let escaped = id.replace('&', "&amp;").replace('<', "&lt;");
        block += &format!("{line_end}{indent}{step}<li>{escaped}</li>");
    }
    block += &format!("{line_end}{indent}{CLOSE}");
    let changed = format!("{}{block}{}", &text[..start], &text[end..]);

    // Never hand back a file the reader would not give the same list for.
    if parse(&changed)?.active != ids {
        return Err(say!("the new list would not read back the same"));
    }
    Ok(changed)
}

fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

/// Writes `ids` as the active mods of the `ModsConfig.xml` at `path`.
///
/// The file as it was is first copied to `ModsConfig.xml.rimstack-backup`
/// beside it, and the very first time also to
/// `ModsConfig.xml.rimstack-original`, which is never overwritten. The new
/// file is written whole under another name and then moved into place, so
/// a crash cannot leave half a file.
pub fn write(path: &Path, ids: &[String]) -> Result<(), String> {
    let failed = |e: std::io::Error| format!("{}: {e}", path.display());
    let before = std::fs::read_to_string(path).map_err(failed)?;
    let after = with_active(&before, ids).map_err(|e| format!("{}: {e}", path.display()))?;

    let original = beside(path, ".rimstack-original");
    if !original.exists() {
        std::fs::write(&original, &before).map_err(failed)?;
    }
    std::fs::write(beside(path, ".rimstack-backup"), &before).map_err(failed)?;
    let partial = beside(path, ".rimstack-part");
    std::fs::write(&partial, &after).map_err(failed)?;
    std::fs::rename(&partial, path).map_err(failed)
}

/// "1.6.4871 rev598" -> "1.6". `None` unless it starts with two numbers.
pub fn short_version(version: &str) -> Option<String> {
    let mut parts = version.trim().split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts
        .next()?
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    Some(format!("{major}.{minor}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPICAL: &str = r#"<?xml version="1.0" ?>
<ModsConfigData>
  <version>1.6.4871 rev598</version>
  <activeMods>
    <li>brrainz.harmony</li>
    <li>ludeon.rimworld</li>
    <li>ludeon.rimworld.royalty</li>
  </activeMods>
  <knownExpansions>
    <li>ludeon.rimworld.royalty</li>
  </knownExpansions>
</ModsConfigData>"#;

    #[test]
    fn active_mods_keep_their_order_and_exclude_expansions() {
        let c = parse(TYPICAL).unwrap();
        assert_eq!(c.version.as_deref(), Some("1.6.4871 rev598"));
        assert_eq!(
            c.active,
            [
                "brrainz.harmony",
                "ludeon.rimworld",
                "ludeon.rimworld.royalty"
            ]
        );
        assert_eq!(c.known_expansions, ["ludeon.rimworld.royalty"]);
    }

    #[test]
    fn empty_and_missing_lists() {
        let c = parse("<ModsConfigData><activeMods/></ModsConfigData>").unwrap();
        assert!(c.active.is_empty() && c.known_expansions.is_empty());
        assert_eq!(
            parse("<ModsConfigData/>").unwrap_err(),
            "no activeMods element"
        );
        assert!(parse("<ModsConfigData>").is_err());
    }

    fn ids(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_new_list_changes_only_the_active_mods() {
        let list = ids(&["ludeon.rimworld", "a.b"]);
        let changed = with_active(TYPICAL, &list).unwrap();
        assert_eq!(
            changed,
            TYPICAL.replace(
                "    <li>brrainz.harmony</li>\n    <li>ludeon.rimworld</li>\n    <li>ludeon.rimworld.royalty</li>",
                "    <li>ludeon.rimworld</li>\n    <li>a.b</li>"
            )
        );
    }

    #[test]
    fn tabs_windows_line_ends_and_a_byte_order_mark_are_kept() {
        let text = "\u{feff}<ModsConfigData>\r\n\t<activeMods>\r\n\t\t<li>a.a</li>\r\n\t</activeMods>\r\n</ModsConfigData>";
        let changed = with_active(text, &ids(&["b.b", "c.c"])).unwrap();
        assert_eq!(
            changed,
            "\u{feff}<ModsConfigData>\r\n\t<activeMods>\r\n\t\t<li>b.b</li>\r\n\t\t<li>c.c</li>\r\n\t</activeMods>\r\n</ModsConfigData>"
        );
    }

    #[test]
    fn an_empty_self_closing_list_can_be_filled_and_a_list_emptied() {
        let filled = with_active(
            "<ModsConfigData><activeMods /></ModsConfigData>",
            &ids(&["a.a"]),
        );
        assert_eq!(parse(&filled.unwrap()).unwrap().active, ["a.a"]);
        let emptied = with_active(TYPICAL, &[]).unwrap();
        assert!(parse(&emptied).unwrap().active.is_empty());
        assert_eq!(
            parse(&emptied).unwrap().known_expansions,
            ["ludeon.rimworld.royalty"]
        );
    }

    #[test]
    fn a_file_without_the_list_is_refused() {
        assert!(with_active("<ModsConfigData/>", &ids(&["a.a"])).is_err());
    }

    #[test]
    fn writing_keeps_the_first_file_and_the_one_before() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ModsConfig.xml");
        std::fs::write(&path, TYPICAL).unwrap();

        write(&path, &ids(&["a.a"])).unwrap();
        assert_eq!(read(&path).unwrap().active, ["a.a"]);
        let first = std::fs::read_to_string(&path).unwrap();
        write(&path, &ids(&["b.b"])).unwrap();

        assert_eq!(read(&path).unwrap().active, ["b.b"]);
        let kept = |name: &str| std::fs::read_to_string(dir.path().join(name)).unwrap();
        assert_eq!(kept("ModsConfig.xml.rimstack-original"), TYPICAL);
        assert_eq!(kept("ModsConfig.xml.rimstack-backup"), first);
        assert!(!dir.path().join("ModsConfig.xml.rimstack-part").exists());
    }

    #[test]
    fn writing_to_a_missing_file_fails_and_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ModsConfig.xml");
        assert!(write(&path, &ids(&["a.a"])).is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn short_version_takes_two_numbers() {
        assert_eq!(short_version("1.6.4871 rev598").as_deref(), Some("1.6"));
        assert_eq!(short_version("1.5").as_deref(), Some("1.5"));
        assert_eq!(short_version("garbage"), None);
        assert_eq!(short_version("1"), None);
    }
}
