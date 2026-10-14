//! RimStack's own settings: a small file of names and values.

use crate::base::error::Error;
use crate::said;
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
        let Some(Value::Object(entries)) = crate::base::atomic_file::read_json(path) else {
            return Self::default();
        };
        let values = entries
            .into_iter()
            .filter_map(|(k, v)| Some((k, v.as_str()?.to_string())))
            .collect();
        Self { values }
    }

    pub fn write(&self, path: &Path) -> Result<(), Error> {
        let entries: serde_json::Map<String, Value> = self
            .values
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();
        let text = serde_json::to_string_pretty(&Value::Object(entries))
            .map_err(|e| Error::format(format!("{}: {e}", path.display())))?;
        crate::base::atomic_file::write(path, text)
    }

    /// The value, or nothing when it was never set or was set to empty.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values
            .get(name)
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }

    /// A value that is several, kept with commas between: each of them.
    pub fn listed(&self, name: &str) -> impl Iterator<Item = &str> {
        let kept = self.get(name).unwrap_or_default();
        kept.split(',').map(str::trim).filter(|one| !one.is_empty())
    }

    pub fn switch(&self, switch: Switch) -> bool {
        match self.get(switch.name()) {
            Some("on") => true,
            Some("off") => false,
            _ => switch.on_at_first(),
        }
    }

    /// Play starts the game's own program rather than asking Steam.
    pub fn launch_directly(&self) -> bool {
        self.get("launch") == Some("direct")
    }

    /// How many of the game's logs are kept, when one was chosen.
    pub fn logs_kept(&self) -> Option<usize> {
        self.get("logsKept")?.parse().ok()
    }

    /// Setting a value to empty forgets it. Only a setting RimStack
    /// knows is kept, and only a value it can hold.
    pub fn set(&mut self, name: &str, value: &str) -> Result<(), Error> {
        let value = value.trim();
        let Some((_, holds)) = KNOWN.iter().find(|(known, _)| *known == name) else {
            return Err(Error::refused(said!("{} is not a setting", name)));
        };
        let fits = match holds {
            _ if value.is_empty() => true,
            Holds::Choice(allowed) => allowed.contains(&value),
            Holds::Count => value.parse::<usize>().is_ok(),
            Holds::Text | Holds::List => true,
        };
        if !fits {
            return Err(Error::refused(said!("{} cannot be {}", name, value)));
        }
        match value.is_empty() {
            true => drop(self.values.remove(name)),
            false => drop(self.values.insert(name.to_string(), value.to_string())),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum Holds {
    Text,
    /// Several, with commas between.
    List,
    Count,
    Choice(&'static [&'static str]),
}

const SWITCH: Holds = Holds::Choice(&["on", "off"]);

const KNOWN: [(&str, Holds); 20] = [
    ("game", Holds::Text),
    ("config", Holds::Text),
    ("workshop", Holds::Text),
    ("launch", Holds::Choice(&["steam", "direct"])),
    ("launchArguments", Holds::Text),
    ("language", Holds::Text),
    ("font", Holds::Text),
    ("interfaceScale", Holds::Text),
    ("ownColoursFor", Holds::Text),
    ("versionAsideFor", Holds::Text),
    ("setupDone", Holds::Choice(&["yes"])),
    ("logsKept", Holds::Count),
    ("databasesOff", Holds::List),
    ("replacementsIgnored", Holds::List),
    ("textureSkip", Holds::List),
    ("communityRules", SWITCH),
    ("versionWarnings", SWITCH),
    ("fileHints", SWITCH),
    ("gitForWorkshop", SWITCH),
    ("selfCheck", SWITCH),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    /// Sort follows the community's rules.
    CommunityRules,
    /// Mods that do not list the game's version are warned about.
    VersionWarnings,
    /// Sort follows what the mods' files say of each other.
    FileHints,
    /// A Workshop mod that is a Git checkout is updated with Git.
    GitForWorkshop,
    /// RimStack asks whether a newer RimStack is published.
    SelfCheck,
}

impl Switch {
    pub fn name(self) -> &'static str {
        match self {
            Switch::CommunityRules => "communityRules",
            Switch::VersionWarnings => "versionWarnings",
            Switch::FileHints => "fileHints",
            Switch::GitForWorkshop => "gitForWorkshop",
            Switch::SelfCheck => "selfCheck",
        }
    }

    fn on_at_first(self) -> bool {
        match self {
            Switch::CommunityRules | Switch::VersionWarnings | Switch::SelfCheck => true,
            Switch::FileHints | Switch::GitForWorkshop => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_of_several_is_read_one_by_one() {
        let mut s = Settings::default();
        assert_eq!(s.listed("databasesOff").count(), 0);
        s.set("databasesOff", "rules, names,,").unwrap();
        assert_eq!(
            s.listed("databasesOff").collect::<Vec<_>>(),
            ["rules", "names"]
        );
    }

    #[test]
    fn values_are_kept_trimmed_and_empty_forgets() {
        let mut s = Settings::default();
        assert_eq!(s.get("game"), None);
        s.set("game", "  /games/RimWorld ").unwrap();
        assert_eq!(s.get("game"), Some("/games/RimWorld"));
        s.set("game", "  ").unwrap();
        assert_eq!(s.get("game"), None);
    }

    #[test]
    fn a_switch_is_as_it_was_set_and_otherwise_as_it_is_at_first() {
        let mut s = Settings::default();
        assert!(s.switch(Switch::CommunityRules) && !s.switch(Switch::FileHints));
        s.set("communityRules", "off").unwrap();
        s.set("fileHints", "on").unwrap();
        assert!(!s.switch(Switch::CommunityRules) && s.switch(Switch::FileHints));
        s.set("communityRules", "").unwrap();
        assert!(s.switch(Switch::CommunityRules));
    }

    #[test]
    fn only_a_known_setting_with_a_value_it_can_hold_is_kept() {
        let mut s = Settings::default();
        assert!(s.set("gameDir", "/x").is_err());
        assert!(s.set("fileHints", "yes").is_err());
        assert!(s.set("launch", "now").is_err());
        assert!(s.set("logsKept", "ten").is_err());
        assert_eq!(s, Settings::default());
        s.set("launch", "direct").unwrap();
        s.set("logsKept", "10").unwrap();
        assert!(s.launch_directly());
        assert_eq!(s.logs_kept(), Some(10));
        let refused = s.set("selfCheck", "maybe").unwrap_err();
        assert_eq!(refused.kind(), crate::base::error::Kind::Refused);
    }

    #[test]
    fn settings_read_back_as_written_and_damage_reads_as_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("RimStack").join("settings.json");
        let mut s = Settings::default();
        s.set("game", "/games/RimWorld").unwrap();
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
