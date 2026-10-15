//! Starting the game, and what is known of it while RimStack runs.

use super::App;
use rimstack_core::base::error::Error;
use rimstack_core::kept::launches;
use rimstack_core::rimworld::{modsconfig, paths::Source};
use rimstack_core::said;

const APP_ID: u32 = 294_100;

/// How many copies of the newest save are kept from before a launch.
const SAVE_BACKUPS: usize = 5;

/// The text as JavaScript's `encodeURIComponent` gives it: Steam reads
/// the launch arguments of a `steam://run` address that way.
fn uri_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' => out.push(char::from(byte)),
            b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => {
                out.push(char::from(byte))
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The address that has Steam start the game with these arguments.
pub fn steam_address(arguments: &str) -> String {
    match arguments.trim() {
        "" => format!("steam://rungameid/{APP_ID}"),
        given => format!("steam://run/{APP_ID}//{}/", uri_component(given)),
    }
}

impl App {
    pub fn game_running(&self) -> bool {
        let paths = self.paths();
        let game = paths.game.as_deref();
        game.is_some_and(rimstack_core::sources::maintenance::game_running)
    }

    /// Before a launch from here: the newest save copied, the launch
    /// recorded with its list, and every mod seen.
    pub fn note_launch(&self) {
        if let Err(problem) = self.back_up_newest_save("save-backups", SAVE_BACKUPS) {
            rimstack_core::problem!("save not backed up: {problem}");
        }
        let list = self.session().current().to_vec();
        if let Err(problem) = launches::record(&self.launches_file(), &list, super::now_secs()) {
            rimstack_core::problem!("{problem}");
        }
        self.mark_all_seen();
        self.record_versions();
    }

    /// Starts the game's own program with the launch arguments from the
    /// settings, or gives the address that has Steam start it.
    pub fn start_game(&self) -> Result<Option<String>, Error> {
        let (directly, arguments) = {
            let settings = self.settings();
            let arguments = settings.get("launchArguments").unwrap_or_default();
            (settings.launch_directly(), arguments.to_string())
        };
        if !directly {
            return Ok(Some(steam_address(&arguments)));
        }
        let paths = self.paths();
        let game = paths
            .game
            .as_ref()
            .ok_or_else(|| Error::refused(said!("The game's folder is not known")))?;
        rimstack_core::sources::maintenance::start_game(game, &arguments)?;
        Ok(None)
    }

    /// The load order `ModsConfig.xml` holds now.
    pub fn mods_config_list(&self) -> Option<Vec<String>> {
        let file = self.paths().mods_config()?;
        modsconfig::read(&file).ok().map(|config| config.active)
    }

    /// The last launch's list, when the game has emptied its own to its
    /// official mods since, as it does when it first starts on a new version.
    pub fn list_reset_by_game(&self) -> Option<Vec<String>> {
        let loaded = self.library();
        let (library, _) = loaded.as_ref().as_ref().ok()?;
        let last = launches::read(&self.launches_file()).pop()?;
        let saved = self.session().saved().to_vec();
        let official = |m: &rimstack_core::mods::list::Mod| {
            let installed = m.installed.and_then(|i| library.get(i));
            installed.is_some_and(|i| i.source == Source::Official)
        };
        let emptied = library.arrange(&saved).active.iter().all(official);
        (emptied && last.ids.len() > saved.len()).then_some(last.ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::tests::installed;

    #[test]
    fn steam_is_given_the_arguments_as_a_browser_would_encode_them() {
        assert_eq!(steam_address("  "), "steam://rungameid/294100");
        assert_eq!(
            steam_address("-logfile /tmp/a b.txt é"),
            "steam://run/294100//-logfile%20%2Ftmp%2Fa%20b.txt%20%C3%A9/"
        );
    }

    #[test]
    fn a_launch_is_recorded_with_its_list_and_steam_starts_the_game_by_default() {
        let (_tmp, app) = installed();
        app.note_launch();
        let record = launches::read(&app.launches_file());
        assert_eq!(record.len(), 1);
        assert_eq!(record[0].ids, app.session().current().to_vec());
        assert_eq!(
            app.start_game().unwrap().as_deref(),
            Some("steam://rungameid/294100")
        );
    }

    #[test]
    fn a_list_emptied_to_the_official_mods_offers_the_last_launch_s() {
        let both = vec!["ludeon.rimworld".to_string(), "ann.roads".to_string()];
        let (_tmp, app) = installed();
        assert_eq!(app.list_reset_by_game(), None);
        launches::record(&app.launches_file(), &both, 1).unwrap();
        assert_eq!(app.list_reset_by_game(), Some(both.clone()));
        assert_eq!(
            app.mods_config_list(),
            Some(vec!["ludeon.rimworld".to_string()])
        );

        let (tmp, app) = installed();
        std::fs::write(
            tmp.path().join("config/Config/ModsConfig.xml"),
            "<ModsConfigData><activeMods><li>ludeon.rimworld</li><li>ann.roads</li></activeMods></ModsConfigData>",
        )
        .unwrap();
        launches::record(&app.launches_file(), &both, 1).unwrap();
        assert_eq!(app.list_reset_by_game(), None);
    }
}
