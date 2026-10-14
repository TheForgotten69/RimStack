//! The one object QML imports: it is handed the `App`, makes one object
//! per part of the window on it, and owns them for as long as it lives.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("rimstack-app/src/settings_data.cxxqt.h");
        type SettingsData = crate::settings_data::qobject::SettingsData;
        include!("rimstack-app/src/mods_data.cxxqt.h");
        type ModsData = crate::mods_data::qobject::ModsData;
        include!("rimstack-app/src/saves_data.cxxqt.h");
        type SavesData = crate::saves_data::qobject::SavesData;
        include!("rimstack-app/src/logs_data.cxxqt.h");
        type LogsData = crate::logs_data::qobject::LogsData;
        include!("rimstack-app/src/game_data.cxxqt.h");
        type GameData = crate::game_data::qobject::GameData;
        include!("rimstack-app/src/lists_data.cxxqt.h");
        type ListsData = crate::lists_data::qobject::ListsData;
        include!("rimstack-app/src/search_data.cxxqt.h");
        type SearchData = crate::search_data::qobject::SearchData;
        include!("rimstack-app/src/textures_data.cxxqt.h");
        type TexturesData = crate::textures_data::qobject::TexturesData;
        include!("rimstack-app/src/downloads_data.cxxqt.h");
        type DownloadsData = crate::downloads_data::qobject::DownloadsData;

        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("rimstack-app/src/application.h");
        #[rust_name = "set_window_icon"]
        fn setWindowIcon(resource: &QString);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(*mut SettingsData, settings, READ, CONSTANT)]
        #[qproperty(*mut ModsData, mods, READ, CONSTANT)]
        #[qproperty(*mut SavesData, saves, READ, CONSTANT)]
        #[qproperty(*mut LogsData, logs, READ, CONSTANT)]
        #[qproperty(*mut GameData, game, READ, CONSTANT)]
        #[qproperty(*mut ListsData, lists, READ, CONSTANT)]
        #[qproperty(*mut SearchData, search, READ, CONSTANT)]
        #[qproperty(*mut TexturesData, textures, READ, CONSTANT)]
        #[qproperty(*mut DownloadsData, downloads, READ, CONSTANT)]
        type Hub = super::HubRust;
    }

    impl cxx_qt::Initialize for Hub {}
}

use core::pin::Pin;
use cxx::UniquePtr;
use cxx_qt::CxxQtType;
use qobject::{
    DownloadsData, GameData, ListsData, LogsData, ModsData, SavesData, SearchData, SettingsData,
    TexturesData,
};
use std::time::Duration;

pub struct HubRust {
    settings: *mut SettingsData,
    mods: *mut ModsData,
    saves: *mut SavesData,
    logs: *mut LogsData,
    game: *mut GameData,
    lists: *mut ListsData,
    search: *mut SearchData,
    textures: *mut TexturesData,
    downloads: *mut DownloadsData,
    owned_settings: UniquePtr<SettingsData>,
    owned_mods: UniquePtr<ModsData>,
    owned_saves: UniquePtr<SavesData>,
    owned_logs: UniquePtr<LogsData>,
    owned_game: UniquePtr<GameData>,
    owned_lists: UniquePtr<ListsData>,
    owned_search: UniquePtr<SearchData>,
    owned_textures: UniquePtr<TexturesData>,
    owned_downloads: UniquePtr<DownloadsData>,
}

impl Default for HubRust {
    fn default() -> Self {
        Self {
            settings: std::ptr::null_mut(),
            mods: std::ptr::null_mut(),
            saves: std::ptr::null_mut(),
            logs: std::ptr::null_mut(),
            game: std::ptr::null_mut(),
            lists: std::ptr::null_mut(),
            search: std::ptr::null_mut(),
            textures: std::ptr::null_mut(),
            downloads: std::ptr::null_mut(),
            owned_settings: UniquePtr::null(),
            owned_mods: UniquePtr::null(),
            owned_saves: UniquePtr::null(),
            owned_logs: UniquePtr::null(),
            owned_game: UniquePtr::null(),
            owned_lists: UniquePtr::null(),
            owned_search: UniquePtr::null(),
            owned_textures: UniquePtr::null(),
            owned_downloads: UniquePtr::null(),
        }
    }
}

pub fn use_window_icon() {
    qobject::set_window_icon(&cxx_qt_lib::QString::from(
        ":/qt/qml/RimStack/Ui/assets/rimstack.svg",
    ));
}

/// What the command line gives for `flag`: "" for the flag alone.
fn value(flag: &str) -> Option<String> {
    std::env::args().find_map(|a| match a.strip_prefix(flag)? {
        "" => Some(String::new()),
        rest => rest.strip_prefix('=').map(String::from),
    })
}

fn given(flag: &str) -> bool {
    value(flag).is_some()
}

/// How often the game and the disk are looked at: never for made-up
/// mods, a check or a picture, unless `--check=live` or `--live` asks.
fn watch_every(demo: bool) -> Option<Duration> {
    let check = value("--check");
    let live = check.as_deref() == Some("live");
    let still = demo || (check.is_some() && !live) || (given("--shot") && !given("--live"));
    match (still, live) {
        (true, _) => None,
        (false, true) => Some(Duration::from_millis(500)),
        (false, false) => Some(Duration::from_secs(2)),
    }
}

/// What QML holds of an object the Hub owns: its address, valid for as
/// long as the Hub lives.
pub(crate) fn address<T>(owned: &mut UniquePtr<T>) -> *mut T
where
    T: cxx::memory::UniquePtrTarget,
{
    owned.as_mut().map_or(std::ptr::null_mut(), |pinned| {
        // SAFETY: the address is only handed to QML; the object stays
        // pinned inside the `UniquePtr` the Hub keeps.
        unsafe { Pin::get_unchecked_mut(pinned) as *mut T }
    })
}

impl cxx_qt::Initialize for qobject::Hub {
    fn initialize(mut self: Pin<&mut Self>) {
        let app = crate::workspace::shared();
        let mut settings = crate::settings_data::qobject::make_settings_data();
        if let Some(settings) = settings.as_mut() {
            settings.connect(app.clone());
        }
        // Made-up mods are shown and nothing is written for them; while
        // checked, no snapshot or history is kept.
        let demo = given("--demo") || given("--selftest");
        let mut mods = crate::mods_data::qobject::make_mods_data();
        if let Some(mods) = mods.as_mut() {
            mods.connect(app.clone(), demo, !given("--check"));
        }
        let mut saves = crate::saves_data::qobject::make_saves_data();
        if let Some(saves) = saves.as_mut() {
            saves.connect(app.clone());
        }
        let mut logs = crate::logs_data::qobject::make_logs_data();
        if let Some(logs) = logs.as_mut() {
            logs.connect(app.clone(), !demo);
        }
        let mut game = crate::game_data::qobject::make_game_data();
        if let Some(game) = game.as_mut() {
            game.connect(app.clone(), demo, watch_every(demo));
        }
        let mut lists = crate::lists_data::qobject::make_lists_data();
        if let Some(lists) = lists.as_mut() {
            lists.connect(app.clone(), demo);
        }
        let mut search = crate::search_data::qobject::make_search_data();
        if let Some(search) = search.as_mut() {
            search.connect(app.clone(), demo, !given("--check"));
        }
        let mut textures = crate::textures_data::qobject::make_textures_data();
        if let Some(textures) = textures.as_mut() {
            textures.connect(app.clone());
        }
        let mut downloads = crate::downloads_data::qobject::make_downloads_data();
        if let Some(downloads) = downloads.as_mut() {
            downloads.connect(app.clone(), demo);
        }
        let mut hub = self.as_mut().rust_mut();
        hub.settings = address(&mut settings);
        hub.owned_settings = settings;
        hub.mods = address(&mut mods);
        hub.owned_mods = mods;
        hub.saves = address(&mut saves);
        hub.owned_saves = saves;
        hub.logs = address(&mut logs);
        hub.owned_logs = logs;
        hub.game = address(&mut game);
        hub.owned_game = game;
        hub.lists = address(&mut lists);
        hub.owned_lists = lists;
        hub.search = address(&mut search);
        hub.owned_search = search;
        hub.textures = address(&mut textures);
        hub.owned_textures = textures;
        hub.downloads = address(&mut downloads);
        hub.owned_downloads = downloads;
    }
}
