//! Subscribing to Workshop mods and unsubscribing from them through
//! Steam itself, with the Steamworks library the game ships in its own
//! folder: RimStack carries no copy of it.
//!
//! While the library is in use Steam shows the game as running, so this
//! is done in a short-lived process of its own (`main` runs `act` for
//! the `--steamworks=` argument) that ends as soon as Steam has the
//! requests.

use crate::say;

use std::path::{Path, PathBuf};
use std::time::Duration;

const GAME: &str = "294100";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Subscribe,
    Unsubscribe,
    /// Fetch the current version of an item now, ahead of Steam's other downloads.
    Download,
}

/// The Steamworks library in the game's folder: in the folder itself from
/// 1.7; under Unity in `Plugins` of its data folder, or a folder below.
pub fn library(game_dir: &Path) -> Option<PathBuf> {
    const NAMES: [&str; 3] = ["libsteam_api.so", "steam_api64.dll", "libsteam_api.dylib"];
    fn search(dir: &Path, depth: usize) -> Option<PathBuf> {
        let entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .collect();
        let is_it = |p: &&PathBuf| {
            p.file_name()
                .is_some_and(|n| NAMES.iter().any(|name| n == *name))
        };
        let here = entries.iter().find(is_it).cloned();
        let below = || {
            entries
                .iter()
                .filter(|p| p.is_dir())
                .find_map(|p| search(p, depth - 1))
        };
        here.or_else(|| (depth > 0).then(below).flatten())
    }
    search(game_dir, 4)
}

/// What a request string means: `subscribe:1,2,3`, `unsubscribe:4` or `download:5`.
pub fn parse(request: &str) -> Option<(Action, Vec<u64>)> {
    let (what, ids) = request.split_once(':')?;
    let action = match what {
        "subscribe" => Action::Subscribe,
        "unsubscribe" => Action::Unsubscribe,
        "download" => Action::Download,
        _ => return None,
    };
    let ids: Option<Vec<u64>> = ids.split(',').map(|id| id.trim().parse().ok()).collect();
    ids.filter(|ids| !ids.is_empty()).map(|ids| (action, ids))
}

/// The library's name for its Workshop interface, whichever version the
/// game was built against: `SteamAPI_SteamUGC_v016` at the time of writing.
fn workshop_interface(
    steam: &libloading::Library,
) -> Option<unsafe extern "C" fn() -> *mut std::ffi::c_void> {
    (10..=40).rev().find_map(|version| {
        let name = format!("SteamAPI_SteamUGC_v0{version}\0");
        // SAFETY: the symbol, when present, is the flat API's accessor,
        // which takes nothing and returns the interface.
        unsafe { steam.get(name.as_bytes()).ok().map(|found| *found) }
    })
}

/// Asks Steam, which must be running, to subscribe to, unsubscribe from
/// or download these Workshop items. Steam then downloads or removes them.
pub fn act(library: &Path, action: Action, ids: &[u64]) -> Result<(), String> {
    type Item = unsafe extern "C" fn(*mut std::ffi::c_void, u64) -> u64;
    // ISteamUGC::DownloadItem(id, high priority): true when Steam took it.
    type Fetch = unsafe extern "C" fn(*mut std::ffi::c_void, u64, bool) -> bool;
    std::env::set_var("SteamAppId", GAME);
    std::env::set_var("SteamGameId", GAME);
    let missing = |what: &str| say!("the game's Steam library has no {}", what);
    // SAFETY: the game's own Steamworks library, called as its flat C
    // interface is declared: Init before anything else, the Workshop
    // interface only while initialised, Shutdown last.
    unsafe {
        let steam =
            libloading::Library::new(library).map_err(|e| format!("{}: {e}", library.display()))?;
        let init: libloading::Symbol<unsafe extern "C" fn() -> bool> = steam
            .get(b"SteamAPI_Init\0")
            .map_err(|_| missing("SteamAPI_Init"))?;
        let run: libloading::Symbol<unsafe extern "C" fn()> = steam
            .get(b"SteamAPI_RunCallbacks\0")
            .map_err(|_| missing("SteamAPI_RunCallbacks"))?;
        let shutdown: libloading::Symbol<unsafe extern "C" fn()> = steam
            .get(b"SteamAPI_Shutdown\0")
            .map_err(|_| missing("SteamAPI_Shutdown"))?;
        let name: &[u8] = match action {
            Action::Subscribe => b"SteamAPI_ISteamUGC_SubscribeItem\0",
            Action::Unsubscribe => b"SteamAPI_ISteamUGC_UnsubscribeItem\0",
            Action::Download => b"SteamAPI_ISteamUGC_DownloadItem\0",
        };
        // Looked up before Init so a library without it leaves Steam alone.
        let ask: libloading::Symbol<Item> =
            steam.get(name).map_err(|_| missing("way to do that"))?;
        let fetch: libloading::Symbol<Fetch> =
            steam.get(name).map_err(|_| missing("way to do that"))?;
        let interface = workshop_interface(&steam).ok_or_else(|| missing("Workshop interface"))?;
        if !init() {
            return Err(say!("Steam is not running, or does not own the game"));
        }
        let workshop = interface();
        if workshop.is_null() {
            shutdown();
            return Err(say!("Steam gave no Workshop interface"));
        }
        let mut refused = 0;
        for id in ids {
            match action {
                Action::Download => refused += usize::from(!fetch(workshop, *id, true)),
                Action::Subscribe | Action::Unsubscribe => {
                    ask(workshop, *id);
                }
            }
        }
        // Steam takes the requests as its callbacks run.
        for _ in 0..30 {
            run();
            std::thread::sleep(Duration::from_millis(100));
        }
        shutdown();
        if refused > 0 {
            return Err(say!("Steam refused {} of {}", refused, ids.len()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_names_what_to_do_and_to_which_items() {
        assert_eq!(
            parse("subscribe:1,22, 333"),
            Some((Action::Subscribe, vec![1, 22, 333]))
        );
        assert_eq!(parse("unsubscribe:7"), Some((Action::Unsubscribe, vec![7])));
        assert_eq!(parse("download:7,8"), Some((Action::Download, vec![7, 8])));
        assert_eq!(parse("delete:7"), None);
        assert_eq!(parse("subscribe:"), None);
        assert_eq!(parse("subscribe:1,x"), None);
    }

    #[test]
    fn the_library_is_found_in_the_game_s_plugins() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(library(tmp.path()), None);
        let plugins = tmp.path().join("RimWorldLinux_Data/Plugins");
        std::fs::create_dir_all(&plugins).unwrap();
        std::fs::write(plugins.join("libsteam_api.so"), "").unwrap();
        assert_eq!(library(tmp.path()), Some(plugins.join("libsteam_api.so")));
    }

    /// Loads the real library and finds what `act` calls, without
    /// starting Steam's interface: nothing is asked of Steam.
    #[test]
    #[ignore = "needs RimWorld installed"]
    fn real_library_has_what_is_called() {
        let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
        let paths = crate::paths::detect(&home, &crate::paths::Overrides::from_env());
        let file = library(paths.game.as_ref().unwrap()).unwrap();
        // SAFETY: loading runs no Steam code; symbols are only looked up.
        let steam = unsafe { libloading::Library::new(&file).unwrap() };
        assert!(workshop_interface(&steam).is_some());
        for name in [
            &b"SteamAPI_Init\0"[..],
            b"SteamAPI_ISteamUGC_SubscribeItem\0",
            b"SteamAPI_ISteamUGC_UnsubscribeItem\0",
        ] {
            // SAFETY: looked up, never called.
            assert!(unsafe { steam.get::<unsafe extern "C" fn()>(name).is_ok() });
        }
    }
}
