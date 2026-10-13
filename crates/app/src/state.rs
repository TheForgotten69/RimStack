//! What every part of the app shares: where its files are, what was read
//! from disk, and the load order being edited. Each is read on first
//! use; what follows the folders is read again when they change.

use rimstack_core::{
    community, library, notes::Notes, paths, session::Session, settings::Settings,
    userrules::UserRules,
};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock, RwLock};
use std::time::{Duration, Instant};

pub static STARTED: OnceLock<Instant> = OnceLock::new();

/// The application object, for what Qt only lets it change. Set once in
/// `main`, where it lives until the app ends.
static APP: std::sync::atomic::AtomicPtr<cxx_qt_lib::QGuiApplication> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
/// The font the system gave the app at its start, as Qt writes a font
/// in text: what "System font" goes back to.
static SYSTEM_FONT: Mutex<String> = Mutex::new(String::new());

pub fn keep_app(app: std::pin::Pin<&mut cxx_qt_lib::QGuiApplication>) {
    *SYSTEM_FONT.lock().unwrap_or_else(|e| e.into_inner()) = app.font().to_qstring().to_string();
    // SAFETY: the pointer is only stored; `use_font` says when it is used.
    let app = unsafe { app.get_unchecked_mut() };
    APP.store(app, std::sync::atomic::Ordering::Relaxed);
}

/// New texts take this font family from now on; an empty name gives the
/// system's own. Returns the family in use. Texts already on screen
/// keep theirs until told (`Theme.refont`). Call on the window's thread
/// only.
pub fn use_font(family: &str) -> String {
    use cxx_qt_lib::{QFont, QString};
    let app = APP.load(std::sync::atomic::Ordering::Relaxed);
    if app.is_null() {
        return String::new();
    }
    let system = SYSTEM_FONT
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let mut font = QFont::default();
    font.from_string(&QString::from(&system));
    if !family.trim().is_empty() {
        font.set_family(&QString::from(family.trim()));
    }
    // SAFETY: `main` owns the application for the whole run and nothing
    // else holds it while an invokable runs on its thread.
    let app = unsafe { std::pin::Pin::new_unchecked(&mut *app) };
    app.set_application_font(&font);
    font.family().map(|f| f.to_string()).unwrap_or_default()
}

/// Something read from disk on first use, that can be read again. An
/// earlier reading is kept in memory, not freed: other threads may still
/// hold it, and reading again is rare.
pub struct Reread<T: 'static> {
    now: RwLock<Option<&'static T>>,
    read: fn() -> T,
}

impl<T> Reread<T> {
    pub const fn new(read: fn() -> T) -> Self {
        Self {
            now: RwLock::new(None),
            read,
        }
    }

    pub fn get(&self) -> &'static T {
        let known = *self.now.read().unwrap_or_else(|e| e.into_inner());
        if let Some(known) = known {
            return known;
        }
        let fresh: &'static T = Box::leak(Box::new((self.read)()));
        let mut now = self.now.write().unwrap_or_else(|e| e.into_inner());
        // Two threads may both have read: the first one's stands.
        now.get_or_insert(fresh)
    }

    pub fn again(&self) {
        let fresh: &'static T = Box::leak(Box::new((self.read)()));
        *self.now.write().unwrap_or_else(|e| e.into_inner()) = Some(fresh);
    }
}

impl<T> std::ops::Deref for Reread<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.get()
    }
}

pub static PATHS: Reread<paths::Paths> = Reread::new(|| {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    // A folder set in the environment wins over one set in Settings,
    // which wins over what detection finds.
    let mut given = paths::Overrides::from_env();
    let settings = settings();
    let set = |name: &str| settings.get(name).map(PathBuf::from);
    given.game = given.game.or(set("game"));
    given.config = given.config.or(set("config"));
    given.workshop = given.workshop.or(set("workshop"));
    paths::detect(&home, &given)
});

fn home_dir() -> PathBuf {
    paths::home(&paths::from_environment)
}

fn app_dirs() -> (PathBuf, PathBuf) {
    paths::app_dirs(paths::Os::running(), &home_dir(), &paths::from_environment)
}

/// Where RimStack keeps its settings: `$XDG_CONFIG_HOME/RimStack` on
/// Linux, the system's own place for a program's settings elsewhere.
/// The language a setting names; none chosen is English, as before it had a file.
pub fn language_name(chosen: &str) -> &str {
    if chosen.trim().is_empty() {
        "English"
    } else {
        chosen
    }
}

pub fn settings_file() -> PathBuf {
    app_dirs().0.join("RimStack").join("settings.json")
}

static SETTINGS: LazyLock<RwLock<Settings>> =
    LazyLock::new(|| RwLock::new(Settings::read(&settings_file())));

pub fn settings() -> std::sync::RwLockReadGuard<'static, Settings> {
    SETTINGS.read().unwrap_or_else(|e| e.into_inner())
}

/// Whether a community database ("rules", "worksOn", "replacements",
/// "names") is downloaded and used: all are, but those the settings list
/// under `databasesOff`.
pub fn database_on(id: &str) -> bool {
    let off = settings()
        .get("databasesOff")
        .map(String::from)
        .unwrap_or_default();
    !off.split(',').any(|other| other == id)
}

pub fn set_setting(name: &str, value: &str) -> Result<(), String> {
    let mut settings = SETTINGS.write().unwrap_or_else(|e| e.into_inner());
    settings.set(name, value);
    settings.write(&settings_file())
}

/// The Workshop ids of the mods whose suggested replacement was set aside.
pub fn replacements_ignored() -> std::collections::HashSet<String> {
    let kept = settings()
        .get("replacementsIgnored")
        .unwrap_or_default()
        .to_string();
    let ids = kept.split(',').filter(|id| !id.is_empty());
    ids.map(String::from).collect()
}

/// Steam's record of the Workshop items it has downloaded: when each was
/// last updated and how big it is, by Workshop id. Empty without Steam.
pub static STEAM_RECORD: Reread<(
    std::collections::HashMap<String, u64>,
    std::collections::HashMap<String, u64>,
)> = Reread::new(|| {
    let record = PATHS
        .workshop
        .as_ref()
        .and_then(|content| Some(content.parent()?.parent()?.join("appworkshop_294100.acf")))
        .and_then(|file| std::fs::read_to_string(file).ok())
        .unwrap_or_default();
    (
        rimstack_core::workshop::installed_times(&record),
        rimstack_core::workshop::installed_sizes(&record),
    )
});

/// The Workshop ids whose Workshop version is newer than the one on
/// disk, once Downloads has asked Steam.
pub static BEHIND: RwLock<Option<std::collections::HashSet<String>>> = RwLock::new(None);

pub fn notes_file() -> PathBuf {
    data_dir().join("notes.json")
}

/// Your tags and notes, read from disk by whoever asks first.
pub static NOTES: LazyLock<RwLock<Notes>> =
    LazyLock::new(|| RwLock::new(Notes::read(&notes_file())));

pub fn user_rules_file() -> PathBuf {
    data_dir().join("rules.json")
}

/// The rules of your own, read from disk by whoever asks first.
pub static USER_RULES: LazyLock<RwLock<UserRules>> =
    LazyLock::new(|| RwLock::new(UserRules::read(&user_rules_file())));

/// What reading the active mods' files found: which provide the same
/// textures, and which point at each other. Worked out in the background
/// whenever the set of active mods changes.
pub struct Conflicts {
    /// The active mods it was worked out for, by library id, ascending.
    pub members: Vec<usize>,
    /// By place in `members`.
    pub overlaps: rimstack_core::conflicts::Overlaps,
    /// What each of those mods defines and points at, by library id; the
    /// rules detected from files are worked out from these.
    pub scanned: std::collections::HashMap<usize, rimstack_core::infer::Scanned>,
}

pub static CONFLICTS: RwLock<Option<Conflicts>> = RwLock::new(None);

/// The load order being edited, starting from the one on disk.
static SESSION: LazyLock<Mutex<Session>> = LazyLock::new(|| {
    let on_disk = match library() {
        Ok((library, _)) => library.active.iter().map(|m| m.id.clone()).collect(),
        Err(_) => Vec::new(),
    };
    Mutex::new(Session::new(on_disk))
});

pub fn session() -> MutexGuard<'static, Session> {
    SESSION.lock().unwrap_or_else(|e| e.into_inner())
}

/// The installed mods and how long reading them took, or why they could
/// not be read.
pub type Loaded = Result<(library::Library, Duration), String>;

fn load_library() -> Loaded {
    let started = Instant::now();
    let loaded = library::load(&PATHS)?;
    let took = started.elapsed();
    eprintln!("scan_ms={}", took.as_millis());
    Ok((loaded, took))
}

/// Read once at start; whoever asks first does the work and the others
/// wait for it.
static FIRST: LazyLock<Loaded> = LazyLock::new(load_library);

static RESCANNED: RwLock<Option<&'static Loaded>> = RwLock::new(None);

pub fn library() -> &'static Loaded {
    let rescanned = *RESCANNED.read().unwrap_or_else(|e| e.into_inner());
    rescanned.unwrap_or(&FIRST)
}

/// Reads the mod folders again. The earlier reading is kept in memory,
/// not freed: lists on screen may still point into it, and a rescan is
/// something done by hand now and then.
pub fn rescan() {
    STEAM_RECORD.again();
    STARTUP.again();
    NEWEST_SAVE.again();
    let fresh: &'static Loaded = Box::leak(Box::new(load_library()));
    *RESCANNED.write().unwrap_or_else(|e| e.into_inner()) = Some(fresh);
    *CONFLICTS.write().unwrap_or_else(|e| e.into_inner()) = None;
}

/// The folders were changed in Settings: finds them again, reads the
/// mods there, and starts from the load order on disk there.
pub fn reread_folders() {
    PATHS.again();
    rescan();
    let on_disk = match library() {
        Ok((library, _)) => library.active.iter().map(|m| m.id.clone()).collect(),
        Err(_) => Vec::new(),
    };
    *session() = Session::new(on_disk);
}

/// When ModsConfig.xml was written with another load order than the one
/// last saved or read here; 0 while it holds that one. The game writes
/// the file at every start, mostly unchanged.
fn list_changed_outside() -> u64 {
    static LAST: Mutex<(u64, bool)> = Mutex::new((0, false));
    let Some(file) = PATHS.mods_config() else {
        return 0;
    };
    let written = rimstack_core::watch::changed_at(&file);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if last.0 != written {
        let on_disk = rimstack_core::modsconfig::read(&file).map(|config| config.active);
        let same = |a: &[String], b: &[String]| {
            let ids = |list: &[String]| -> Vec<String> {
                let named = list.iter().filter(|id| !id.is_empty());
                named.map(|id| id.to_lowercase()).collect()
            };
            ids(a) == ids(b)
        };
        let differs = on_disk.is_ok_and(|on_disk| !same(&on_disk, session().saved()));
        *last = (written, differs);
    }
    if last.1 {
        written
    } else {
        0
    }
}

/// When each thing RimStack shows was last changed on disk, by name:
/// compared with the time before, it says what to read again.
pub fn stamps() -> serde_json::Value {
    use rimstack_core::watch::{changed_at, newest};
    let paths = &*PATHS;
    let record = paths
        .workshop
        .as_ref()
        .and_then(|content| Some(content.parent()?.parent()?.join("appworkshop_294100.acf")));
    let local = paths.game.as_ref().map(|game| game.join("Mods"));
    let mods = [paths.workshop.clone(), record, local];
    // A checkout pulled outside RimStack: Git notes every move of HEAD.
    // And a mod changed where it lies: its folder and its About.xml.
    let inside: Vec<PathBuf> = match library() {
        Ok((library, _)) => {
            let git = library.installed.iter().filter(|m| m.git);
            let heads = git.map(|m| m.path.join(".git").join("logs").join("HEAD"));
            let abouts = library
                .installed
                .iter()
                .flat_map(|m| [m.path.clone(), m.path.join("About").join("About.xml")]);
            heads.chain(abouts).collect()
        }
        Err(_) => Vec::new(),
    };
    let config = paths.config.as_ref();
    let saves = config.map(|c| c.join("Saves"));
    let logs = [
        config.map(|c| c.join("Player.log")),
        config.map(|c| c.join("Logs").join("Player.log")),
    ];
    let language = settings().get("language").map(|name| {
        settings_file()
            .with_file_name("languages")
            .join(format!("{}.json", language_name(name)))
    });
    let one = |path: Option<PathBuf>| path.map_or(0, |p| changed_at(&p));
    serde_json::json!({
        "mods": newest(mods.iter().flatten().chain(&inside).map(PathBuf::as_path)),
        "saves": one(saves),
        "log": newest(logs.iter().flatten().map(PathBuf::as_path)),
        "modsConfig": list_changed_outside(),
        "theme": one(Some(settings_file().with_file_name("theme.json"))),
        "language": one(language),
    })
}

/// Where RimStack keeps what it downloads: `$XDG_DATA_HOME/RimStack` on
/// Linux, the system's own place for a program's data elsewhere.
pub fn data_dir() -> PathBuf {
    app_dirs().1.join("RimStack")
}

/// The community rules, if they were ever downloaded. Read from disk by
/// whoever asks first; replaced when the user downloads them again.
pub static COMMUNITY: LazyLock<RwLock<Option<community::Community>>> = LazyLock::new(|| {
    let kept = community::read(&community::file_in(&data_dir()));
    RwLock::new(kept.unwrap_or_else(|problem| {
        eprintln!("{problem}");
        None
    }))
});

/// Mods known to work on the installed game version without listing it
/// (the "No Version Warning" list), if it was ever downloaded.
pub static WORKS_ON: LazyLock<RwLock<std::collections::HashSet<String>>> = LazyLock::new(|| {
    let version = PATHS.game_version.clone().unwrap_or_default();
    RwLock::new(community::read_works_on(&community::works_on_file(
        &data_dir(),
        &version,
    )))
});

/// Workshop mods with a maintained replacement, by Workshop id; from
/// RimSort's copy of the Use This Instead list, when RimSort is installed.
pub static REPLACEMENTS: LazyLock<
    std::collections::HashMap<String, rimstack_core::replacements::Replacement>,
> = LazyLock::new(|| {
    // Its own download, or RimSort's copy for an install that has none yet.
    let own = rimstack_core::replacements::file_in(&data_dir());
    let rimsort = data_dir()
        .with_file_name("RimSort")
        .join("dbs/UseThisInstead/replacements.json.gz");
    rimstack_core::replacements::read(if own.is_file() { &own } else { &rimsort })
});

pub static WORKSHOP_NAMES: LazyLock<RwLock<rimstack_core::workshopdb::Names>> =
    LazyLock::new(|| {
        let file = rimstack_core::workshopdb::file_in(&data_dir());
        RwLock::new(rimstack_core::workshopdb::read(&file))
    });

/// What Loading Progress measured at the game's last start, read once.
pub static STARTUP: Reread<rimstack_core::startup::Measured> = Reread::new(|| {
    let config = PATHS.config.as_deref();
    // Measured before the game was last updated, the times are of
    // another game: the mod that measures them may not run on this one.
    let written = |file: PathBuf| std::fs::metadata(file).and_then(|m| m.modified()).ok();
    let measured = config.and_then(|c| written(c.join("StartupImpactData.xml")));
    let updated = PATHS
        .game
        .as_ref()
        .and_then(|g| written(g.join("Version.txt")));
    if matches!((measured, updated), (Some(measured), Some(updated)) if measured < updated) {
        return Default::default();
    }
    config.map(rimstack_core::startup::read).unwrap_or_default()
});

/// The mods of the newest save, by folded package id; `None` without a save.
pub static NEWEST_SAVE: Reread<Option<std::collections::HashSet<String>>> = Reread::new(|| {
    let saves = rimstack_core::saves::list(&PATHS.config.as_ref()?.join("Saves"));
    let header = saves.into_iter().next()?.header.ok()?;
    let ids = header.mod_ids.iter();
    Some(ids.map(|id| rimstack_core::id::split_steam(id).0).collect())
});

pub fn seen_file() -> PathBuf {
    data_dir().join("seen.txt")
}

/// The mod folders there at the last start of the game from RimStack. On
/// a first run every installed mod counts as seen.
pub static SEEN: LazyLock<RwLock<rimstack_core::seen::Seen>> = LazyLock::new(|| {
    let kept = rimstack_core::seen::Seen::read(&seen_file());
    RwLock::new(if kept.is_recorded() { kept } else { all_seen() })
});

fn all_seen() -> rimstack_core::seen::Seen {
    let installed = library().as_ref().ok().map(|l| l.0.installed.as_slice());
    let folders = installed.unwrap_or_default().iter();
    let now = rimstack_core::seen::Seen::of(folders.map(|m| m.path.to_string_lossy().into_owned()));
    if let Err(problem) = now.write(&seen_file()) {
        eprintln!("{problem}");
    }
    now
}

pub fn mark_all_seen() {
    let now = all_seen();
    *SEEN.write().unwrap_or_else(|e| e.into_inner()) = now;
}
