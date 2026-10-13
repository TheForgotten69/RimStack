//! RimStack's own settings: a small file of names and values.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    values: BTreeMap<String, String>,
}

impl Settings {
    /// No file, or a damaged one, is the default settings.
    pub fn read(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(Value::Object(entries)) = serde_json::from_str(&text) else {
            return Self::default();
        };
        let values = entries
            .into_iter()
            .filter_map(|(k, v)| Some((k, v.as_str()?.to_string())))
            .collect();
        Self { values }
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        let failed = |e: std::io::Error| format!("{}: {e}", path.display());
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(failed)?;
        }
        let entries: serde_json::Map<String, Value> = self
            .values
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();
        let text = serde_json::to_string_pretty(&Value::Object(entries)).unwrap_or_default();
        std::fs::write(path, text).map_err(failed)
    }

    /// The value, or nothing when it was never set or was set to empty.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values
            .get(name)
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }

    /// A switch that is on unless it was turned off.
    pub fn is_on(&self, name: &str) -> bool {
        self.get(name) != Some("off")
    }

    /// Setting a value to empty forgets it.
    pub fn set(&mut self, name: &str, value: &str) {
        if value.trim().is_empty() {
            self.values.remove(name);
        } else {
            self.values
                .insert(name.to_string(), value.trim().to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_kept_trimmed_and_empty_forgets() {
        let mut s = Settings::default();
        assert_eq!(s.get("game"), None);
        s.set("game", "  /games/RimWorld ");
        assert_eq!(s.get("game"), Some("/games/RimWorld"));
        s.set("game", "  ");
        assert_eq!(s.get("game"), None);
    }

    #[test]
    fn a_switch_is_on_until_turned_off() {
        let mut s = Settings::default();
        assert!(s.is_on("communityRules"));
        s.set("communityRules", "off");
        assert!(!s.is_on("communityRules"));
        s.set("communityRules", "on");
        assert!(s.is_on("communityRules"));
    }

    #[test]
    fn settings_read_back_as_written_and_damage_reads_as_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("RimStack").join("settings.json");
        let mut s = Settings::default();
        s.set("game", "/games/RimWorld");
        s.write(&path).unwrap();
        assert_eq!(Settings::read(&path), s);
        std::fs::write(&path, "[1,2").unwrap();
        assert_eq!(Settings::read(&path), Settings::default());
        assert_eq!(
            Settings::read(&dir.path().join("none.json")),
            Settings::default()
        );
    }
}
