//! What every part of the app shares: where its files are, what was read
//! from disk, and the load order being edited. Each is read on first
//! use; what follows the folders is read again when they change.

use rimstack_core::base::derived::Derived;
use rimstack_core::base::locks::{Held, Shared};
use rimstack_core::base::reread::Reread;
use rimstack_core::{
    kept::notes::Notes, kept::settings::Settings, mods::community, mods::library,
    mods::session::Session, mods::userrules::UserRules, rimworld::paths,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, RwLock, RwLockReadGuard};
use std::time::{Duration, Instant};

pub mod changed;
pub mod downloads;
pub mod fetch;
mod game;
pub mod lists;
pub mod load_order;
pub mod logs;
mod notes;
mod records;
pub mod saves;
pub mod search;
mod settings;
pub mod textures;
mod upkeep;
pub use settings::{newest_release, short_contrast};

pub static STARTED: OnceLock<Instant> = OnceLock::new();

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The language a setting names; none chosen is English, as before it had a file.
pub fn language_name(chosen: &str) -> &str {
    if chosen.trim().is_empty() {
        "English"
    } else {
        chosen
    }
}

/// What reading the active mods' files found: which provide the same
/// textures, and which point at each other. Worked out in the background
/// whenever the set of active mods changes.
pub struct Conflicts {
    /// The active mods it was worked out for, ascending.
    pub members: Vec<library::ModId>,
    /// By place in `members`.
    pub overlaps: rimstack_core::mods::conflicts::Overlaps,
    /// What each of those mods defines and points at; the rules detected
    /// from files are worked out from these.
    pub scanned: HashMap<library::ModId, rimstack_core::mods::infer::Scanned>,
}

/// The installed mods and how long reading them took, or why they could
/// not be read.
pub type Loaded = Result<(library::Library, Duration), rimstack_core::base::error::Error>;

/// When each Workshop item on disk was last updated, and how big it is,
/// by Workshop id.
type SteamRecord = (HashMap<String, u64>, HashMap<String, u64>);

type Replacements = HashMap<String, rimstack_core::sources::replacements::Replacement>;

pub struct App {
    home: PathBuf,
    settings_dir: PathBuf,
    data_dir: PathBuf,
    /// Folders set in the environment: they win over those set in
    /// Settings, which win over what detection finds.
    given: paths::Overrides,
    settings: RwLock<Settings>,
    paths: Reread<paths::Paths>,
    library: OnceLock<RwLock<Arc<Loaded>>>,
    session: OnceLock<Mutex<Session>>,
    notes: OnceLock<RwLock<Notes>>,
    user_rules: OnceLock<RwLock<UserRules>>,
    community: OnceLock<RwLock<Option<community::Community>>>,
    works_on: OnceLock<RwLock<HashSet<String>>>,
    replacements: Derived<(PathBuf, u64), Replacements>,
    workshop_names: OnceLock<RwLock<rimstack_core::rimworld::workshopdb::Names>>,
    seen: OnceLock<RwLock<rimstack_core::kept::new_mods::Seen>>,
    steam_record: Reread<SteamRecord>,
    startup: Reread<rimstack_core::rimworld::startup::Measured>,
    newest_save: Reread<Option<HashSet<String>>>,
    /// Where SteamCMD is: found by looking through `PATH`, which several
    /// screens ask after on every change.
    steamcmd: Reread<Option<PathBuf>>,
    conflicts: RwLock<Option<Conflicts>>,
    behind: RwLock<Option<HashSet<String>>>,
    /// Counted when first asked for: walking a mod of 10,935 textures
    /// took 85 ms, and the details ask each time the mod is selected.
    texture_counts: Mutex<BTreeMap<PathBuf, usize>>,
    /// When ModsConfig.xml was last looked at, and whether it then held
    /// another load order than the one saved here.
    list_on_disk: Mutex<(u64, bool)>,
    /// What `git --version` said: the program does not appear or leave
    /// while RimStack runs.
    git: OnceLock<Option<String>>,
    /// The game's newest log as the Logs screen last read it.
    log_read: Mutex<Option<rimstack_core::kept::launches::LogRead>>,
    bus: crate::changes::Bus,
}

impl App {
    /// With the folders this system and environment give.
    pub fn from_environment() -> Self {
        let env = &paths::from_environment;
        let home = paths::home(env);
        let (settings, data) = paths::app_dirs(paths::Os::running(), &home, env);
        Self::new(home, &settings, &data, paths::Overrides::from_env())
    }

    /// `settings_root` and `data_root` are the folders programs keep
    /// theirs in; RimStack's own are made inside each.
    pub fn new(
        home: PathBuf,
        settings_root: &Path,
        data_root: &Path,
        given: paths::Overrides,
    ) -> Self {
        let settings_dir = settings_root.join("RimStack");
        Self {
            settings: RwLock::new(Settings::read(&settings_dir.join("settings.json"))),
            home,
            settings_dir,
            data_dir: data_root.join("RimStack"),
            given,
            paths: Reread::new(),
            library: OnceLock::new(),
            session: OnceLock::new(),
            notes: OnceLock::new(),
            user_rules: OnceLock::new(),
            community: OnceLock::new(),
            works_on: OnceLock::new(),
            replacements: Derived::default(),
            workshop_names: OnceLock::new(),
            seen: OnceLock::new(),
            steam_record: Reread::new(),
            startup: Reread::new(),
            newest_save: Reread::new(),
            steamcmd: Reread::new(),
            conflicts: RwLock::new(None),
            behind: RwLock::new(None),
            texture_counts: Mutex::new(BTreeMap::new()),
            list_on_disk: Mutex::new((0, false)),
            git: OnceLock::new(),
            log_read: Mutex::new(None),
            bus: crate::changes::Bus::default(),
        }
    }

    /// Tells every listener what changed.
    pub fn publish(&self, changes: crate::changes::Changes) {
        self.bus.publish(changes);
    }

    pub fn listen(&self, listener: impl Fn(crate::changes::Changes) + Send + Sync + 'static) {
        self.bus.listen(listener);
    }

    /// `$XDG_DATA_HOME/RimStack` on Linux, the system's own place for a
    /// program's data elsewhere.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// In `$XDG_CONFIG_HOME/RimStack` on Linux, the system's own place
    /// for a program's settings elsewhere.
    pub fn settings_file(&self) -> PathBuf {
        self.settings_dir.join("settings.json")
    }

    pub fn diagnostics_file(&self) -> PathBuf {
        self.data_dir.join("diagnostics.log")
    }

    /// RimStack's record of the DDS files it made.
    pub fn texture_manifest_file(&self) -> PathBuf {
        self.data_dir.join("textures.json")
    }

    pub fn launches_file(&self) -> PathBuf {
        self.data_dir.join("launches.json")
    }

    pub fn kept_logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    pub fn git(&self) -> &Option<String> {
        self.git.get_or_init(rimstack_core::sources::git::installed)
    }

    pub fn notes_file(&self) -> PathBuf {
        self.data_dir.join("notes.json")
    }

    pub fn user_rules_file(&self) -> PathBuf {
        self.data_dir.join("rules.json")
    }

    pub fn seen_file(&self) -> PathBuf {
        self.data_dir.join("seen.txt")
    }

    pub fn settings(&self) -> RwLockReadGuard<'_, Settings> {
        self.settings.reading()
    }

    pub fn set_setting(
        &self,
        name: &str,
        value: &str,
    ) -> Result<(), rimstack_core::base::error::Error> {
        let mut settings = self.settings.writing();
        settings.set(name, value)?;
        settings.write(&self.settings_file())
    }

    /// Whether a community database ("rules", "worksOn", "replacements",
    /// "names") is downloaded and used: all are, but those the settings
    /// list under `databasesOff`.
    pub fn database_on(&self, id: &str) -> bool {
        !self.settings().listed("databasesOff").any(|off| off == id)
    }

    /// The Workshop ids of the mods whose suggested replacement was set aside.
    pub fn replacements_ignored(&self) -> HashSet<String> {
        let settings = self.settings();
        let ids = settings.listed("replacementsIgnored");
        ids.map(String::from).collect()
    }

    fn find_paths(&self) -> paths::Paths {
        let mut given = self.given.clone();
        let settings = self.settings();
        let set = |name: &str| settings.get(name).map(PathBuf::from);
        given.game = given.game.or(set("game"));
        given.config = given.config.or(set("config"));
        given.workshop = given.workshop.or(set("workshop"));
        paths::detect(&self.home, &given)
    }

    pub fn paths(&self) -> Arc<paths::Paths> {
        self.paths.get(|| self.find_paths())
    }

    fn read_steam_record(&self) -> SteamRecord {
        let file = self.paths().steam_record();
        let record = file.and_then(|file| std::fs::read_to_string(file).ok());
        let record = record.unwrap_or_default();
        (
            rimstack_core::rimworld::workshop::installed_times(&record),
            rimstack_core::rimworld::workshop::installed_sizes(&record),
        )
    }

    /// Steam's record of the Workshop items it has downloaded. Empty
    /// without Steam.
    pub fn steam_record(&self) -> Arc<SteamRecord> {
        self.steam_record.get(|| self.read_steam_record())
    }

    /// The Workshop ids whose Workshop version is newer than the one on
    /// disk, once Downloads has asked Steam.
    pub fn behind(&self) -> &RwLock<Option<HashSet<String>>> {
        &self.behind
    }

    pub fn notes(&self) -> &RwLock<Notes> {
        self.notes
            .get_or_init(|| RwLock::new(Notes::read(&self.notes_file())))
    }

    pub fn user_rules(&self) -> &RwLock<UserRules> {
        self.user_rules
            .get_or_init(|| RwLock::new(UserRules::read(&self.user_rules_file())))
    }

    pub fn conflicts(&self) -> &RwLock<Option<Conflicts>> {
        &self.conflicts
    }

    fn on_disk(&self) -> Vec<String> {
        match &*self.library() {
            Ok((library, _)) => library.active.iter().map(|m| m.id.clone()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// The load order being edited, starting from the one on disk.
    pub fn session(&self) -> MutexGuard<'_, Session> {
        self.session
            .get_or_init(|| Mutex::new(Session::new(self.on_disk())))
            .held()
    }

    /// The mods in the load order being edited.
    pub fn active_now(&self, library: &library::Library) -> HashSet<library::ModId> {
        let list = self.session().current().to_vec();
        library.active_ids(&list)
    }

    fn load_library(&self) -> Loaded {
        let started = Instant::now();
        let loaded = library::load(&self.paths())?;
        let took = started.elapsed();
        rimstack_core::note!("scan_ms={}", took.as_millis());
        Ok((loaded, took))
    }

    /// Read once at start; whoever asks first does the work and the
    /// others wait for it.
    pub fn library(&self) -> Arc<Loaded> {
        let read = || RwLock::new(Arc::new(self.load_library()));
        self.library.get_or_init(read).reading().clone()
    }

    pub fn texture_count(&self, mod_root: &Path) -> usize {
        if let Some(known) = self.texture_counts.held().get(mod_root) {
            return *known;
        }
        let counted = rimstack_core::mods::textures::pngs(mod_root).len();
        let mut counts = self.texture_counts.held();
        counts.insert(mod_root.to_path_buf(), counted);
        counted
    }

    /// Reads the mod folders again. Whoever still shows the earlier
    /// reading holds it until done.
    pub fn rescan(&self) {
        self.steam_record.put(self.read_steam_record());
        self.startup.put(self.read_startup());
        self.newest_save.put(self.read_newest_save());
        // Read before the lock is taken: readers go on meanwhile.
        let fresh = Arc::new(self.load_library());
        match self.library.get() {
            Some(library) => *library.writing() = fresh,
            None => drop(self.library.set(RwLock::new(fresh))),
        }
        self.textures_changed();
        self.publish(crate::changes::Changes::LIBRARY);
    }

    /// A texture was hidden, shown again or read anew: what was worked
    /// out from the textures on disk no longer holds.
    pub fn textures_changed(&self) {
        *self.conflicts.writing() = None;
        self.texture_counts.held().clear();
        self.publish(crate::changes::Changes::TEXTURES);
    }

    /// The folders were changed in Settings: finds them again, reads the
    /// mods there, and starts from the load order on disk there.
    pub fn reread_folders(&self) {
        self.paths.put(self.find_paths());
        self.rescan();
        let on_disk = self.on_disk();
        *self.session() = Session::new(on_disk);
    }

    /// When ModsConfig.xml was written with another load order than the
    /// one last saved or read here; 0 while it holds that one. The game
    /// writes the file at every start, mostly unchanged.
    fn list_changed_outside(&self) -> u64 {
        let Some(file) = self.paths().mods_config() else {
            return 0;
        };
        let written = rimstack_core::base::watch::changed_at(&file);
        let mut last = self.list_on_disk.held();
        if last.0 != written {
            let on_disk =
                rimstack_core::rimworld::modsconfig::read(&file).map(|config| config.active);
            let saved = self.session().saved().to_vec();
            let differs = on_disk.is_ok_and(|on_disk| {
                !rimstack_core::rimworld::modsconfig::same_order(&on_disk, &saved)
            });
            *last = (written, differs);
        }
        if last.1 {
            written
        } else {
            0
        }
    }

    /// When the game last wrote a log, in milliseconds since 1970; 0 for
    /// never. Up to 1.6 in the settings folder, from 1.7 in `Logs` there.
    pub fn log_written(&self) -> u64 {
        let paths = self.paths();
        let Some(config) = paths.config.as_ref() else {
            return 0;
        };
        let logs = [
            config.join("Player.log"),
            config.join("Logs").join("Player.log"),
        ];
        rimstack_core::base::watch::newest(logs.iter().map(PathBuf::as_path))
    }

    /// When each thing RimStack shows was last changed on disk: compared
    /// with the time before, it says what to read again.
    pub fn stamps(&self) -> rimstack_core::base::watch::Stamps {
        use rimstack_core::base::watch::{changed_at, newest};
        let paths = self.paths();
        let (record, local) = (paths.steam_record(), paths.local_mods());
        let mods = [paths.workshop.clone(), record, local];
        // A checkout pulled outside RimStack: Git notes every move of HEAD.
        // And a mod changed where it lies: its folder and its About.xml.
        let inside: Vec<PathBuf> = match &*self.library() {
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
        let saves = paths.config.as_ref().map(|c| c.join("Saves"));
        let language = self.settings().get("language").map(|name| {
            self.settings_file()
                .with_file_name("languages")
                .join(format!("{}.json", language_name(name)))
        });
        let one = |path: Option<PathBuf>| path.map_or(0, |p| changed_at(&p));
        rimstack_core::base::watch::Stamps {
            mods: newest(mods.iter().flatten().chain(&inside).map(PathBuf::as_path)),
            saves: one(saves),
            log: self.log_written(),
            mods_config: self.list_changed_outside(),
            theme: one(Some(self.settings_file().with_file_name("theme.json"))),
            language: one(language),
        }
    }

    /// The community rules, if they were ever downloaded.
    pub fn community(&self) -> &RwLock<Option<community::Community>> {
        self.community.get_or_init(|| {
            let kept = community::read(&community::file_in(&self.data_dir));
            RwLock::new(kept.unwrap_or_else(|problem| {
                rimstack_core::problem!("{problem}");
                None
            }))
        })
    }

    /// Mods known to work on the installed game version without listing
    /// it (the "No Version Warning" list), if it was ever downloaded.
    pub fn works_on(&self) -> &RwLock<HashSet<String>> {
        self.works_on.get_or_init(|| {
            let version = self.paths().game_version.clone().unwrap_or_default();
            let file = community::works_on_file(&self.data_dir, &version);
            RwLock::new(community::read_works_on(&file))
        })
    }

    /// Workshop mods with a maintained replacement, by Workshop id: its
    /// own download of the Use This Instead list, or RimSort's copy for
    /// an install that has none yet. Read again when the file changes.
    pub fn replacements(&self) -> Arc<Replacements> {
        let own = rimstack_core::sources::replacements::file_in(&self.data_dir);
        let rimsort = self
            .data_dir
            .with_file_name("RimSort")
            .join("dbs/UseThisInstead/replacements.json.gz");
        let file = if own.is_file() { own } else { rimsort };
        let written = rimstack_core::base::watch::changed_at(&file);
        self.replacements.get((file.clone(), written), || {
            rimstack_core::sources::replacements::read(&file)
        })
    }

    pub fn workshop_names(&self) -> &RwLock<rimstack_core::rimworld::workshopdb::Names> {
        self.workshop_names.get_or_init(|| {
            let file = rimstack_core::rimworld::workshopdb::file_in(&self.data_dir);
            RwLock::new(rimstack_core::rimworld::workshopdb::read(&file))
        })
    }

    /// SteamCMD, RimStack's own or the system's; see `redownload::steamcmd`.
    pub fn steamcmd(&self) -> Arc<Option<PathBuf>> {
        self.steamcmd
            .get(|| rimstack_core::sources::redownload::steamcmd(&self.data_dir))
    }

    /// SteamCMD was installed or removed: look for it again.
    pub fn look_for_steamcmd(&self) {
        self.steamcmd
            .put(rimstack_core::sources::redownload::steamcmd(&self.data_dir));
    }

    fn read_startup(&self) -> rimstack_core::rimworld::startup::Measured {
        let paths = self.paths();
        let config = paths.config.as_deref();
        // Measured before the game was last updated, the times are of
        // another game: the mod that measures them may not run on this one.
        let written = |file: PathBuf| std::fs::metadata(file).and_then(|m| m.modified()).ok();
        let measured = config.and_then(|c| written(c.join("StartupImpactData.xml")));
        let updated = paths
            .game
            .as_ref()
            .and_then(|g| written(g.join("Version.txt")));
        if matches!((measured, updated), (Some(measured), Some(updated)) if measured < updated) {
            return Default::default();
        }
        config
            .map(rimstack_core::rimworld::startup::read)
            .unwrap_or_default()
    }

    /// What Loading Progress measured at the game's last start.
    pub fn startup(&self) -> Arc<rimstack_core::rimworld::startup::Measured> {
        self.startup.get(|| self.read_startup())
    }

    fn read_newest_save(&self) -> Option<HashSet<String>> {
        let saves =
            rimstack_core::rimworld::saves::list(&self.paths().config.as_ref()?.join("Saves"));
        let header = saves.into_iter().next()?.header.ok()?;
        let ids = header.mod_ids.iter();
        Some(
            ids.map(|id| rimstack_core::rimworld::id::split_steam(id).0)
                .collect(),
        )
    }

    /// The mods of the newest save, by folded package id; `None` without a save.
    pub fn newest_save(&self) -> Arc<Option<HashSet<String>>> {
        self.newest_save.get(|| self.read_newest_save())
    }

    fn all_seen(&self) -> rimstack_core::kept::new_mods::Seen {
        let loaded = self.library();
        let installed = loaded.as_ref().as_ref().ok();
        let installed = installed.map(|(library, _)| library.installed.as_slice());
        let folders = installed.unwrap_or_default().iter();
        let now = rimstack_core::kept::new_mods::Seen::of(
            folders.map(|m| m.path.to_string_lossy().into_owned()),
        );
        if let Err(problem) = now.write(&self.seen_file()) {
            rimstack_core::problem!("{problem}");
        }
        now
    }

    /// The mod folders there at the last start of the game from RimStack.
    /// On a first run every installed mod counts as seen.
    pub fn seen(&self) -> &RwLock<rimstack_core::kept::new_mods::Seen> {
        self.seen.get_or_init(|| {
            let kept = rimstack_core::kept::new_mods::Seen::read(&self.seen_file());
            RwLock::new(if kept.is_recorded() {
                kept
            } else {
                self.all_seen()
            })
        })
    }

    pub fn mark_all_seen(&self) {
        let now = self.all_seen();
        *self.seen().writing() = now;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::fs;

    /// A game with Core and one local mod, its settings listing Core, and
    /// an `App` whose own folders are under the same scratch folder.
    pub(crate) fn installed() -> (tempfile::TempDir, App) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let add = |folder: &str, id: &str| {
            let about = root.join(folder).join("About");
            fs::create_dir_all(&about).unwrap();
            let xml =
                format!("<ModMetaData><packageId>{id}</packageId><name>{id}</name></ModMetaData>");
            fs::write(about.join("About.xml"), xml).unwrap();
        };
        add("game/Data/Core", "Ludeon.RimWorld");
        add("game/Mods/roads", "Ann.Roads");
        fs::write(root.join("game/Version.txt"), "1.6.4871 rev600").unwrap();
        fs::create_dir_all(root.join("config/Config")).unwrap();
        fs::write(
            root.join("config/Config/ModsConfig.xml"),
            "<ModsConfigData><activeMods><li>ludeon.rimworld</li></activeMods></ModsConfigData>",
        )
        .unwrap();
        let given = paths::Overrides {
            game: Some(root.join("game")),
            config: Some(root.join("config")),
            workshop: Some(root.join("workshop")),
        };
        let app = App::new(
            root.join("home"),
            &root.join("settings"),
            &root.join("data"),
            given,
        );
        (tmp, app)
    }

    #[test]
    fn its_own_files_lie_under_the_folders_it_was_given() {
        let (tmp, app) = installed();
        assert_eq!(app.data_dir(), tmp.path().join("data/RimStack"));
        assert_eq!(
            app.settings_file(),
            tmp.path().join("settings/RimStack/settings.json")
        );
        assert!(app.notes_file().starts_with(app.data_dir()));
    }

    #[test]
    fn a_setting_is_kept_on_disk_and_read_by_whoever_asks_next() {
        let (_tmp, app) = installed();
        assert!(app.database_on("rules"));
        app.set_setting("databasesOff", "rules,names").unwrap();
        assert!(!app.database_on("rules") && app.database_on("worksOn"));
        let kept = Settings::read(&app.settings_file());
        assert_eq!(kept.get("databasesOff"), Some("rules,names"));
    }

    #[test]
    fn the_mods_are_read_once_and_again_only_when_asked() {
        let (tmp, app) = installed();
        let first = app.library();
        let (library, _) = first.as_ref().as_ref().unwrap();
        assert_eq!((library.installed.len(), library.active.len()), (2, 1));
        assert_eq!(app.session().current(), ["ludeon.rimworld"]);
        assert_eq!(app.active_now(library).len(), 1);

        let about = tmp.path().join("game/Mods/hats/About");
        fs::create_dir_all(&about).unwrap();
        fs::write(
            about.join("About.xml"),
            "<ModMetaData><packageId>Ann.Hats</packageId></ModMetaData>",
        )
        .unwrap();
        assert!(Arc::ptr_eq(&first, &app.library()));
        app.rescan();
        let again = app.library();
        assert_eq!(again.as_ref().as_ref().unwrap().0.installed.len(), 3);
    }

    #[test]
    fn a_list_written_outside_is_noticed_and_one_of_the_same_order_is_not() {
        let (tmp, app) = installed();
        let file = tmp.path().join("config/Config/ModsConfig.xml");
        assert_eq!(app.stamps().mods_config, 0);
        let write = |ids: &str, at: u64| {
            fs::write(
                &file,
                format!("<ModsConfigData><activeMods>{ids}</activeMods></ModsConfigData>"),
            )
            .unwrap();
            let when = std::time::UNIX_EPOCH + Duration::from_secs(at);
            fs::File::options()
                .write(true)
                .open(&file)
                .unwrap()
                .set_modified(when)
                .unwrap();
        };
        // As the game writes it at a start: the same mods, another case.
        write("<li>Ludeon.RimWorld</li>", 2_000_000_000);
        assert_eq!(app.stamps().mods_config, 0);
        write("<li>ludeon.rimworld</li><li>ann.roads</li>", 2_000_000_100);
        assert_ne!(app.stamps().mods_config, 0);
    }

    #[test]
    fn a_folder_set_in_settings_is_used_once_the_folders_are_read_again() {
        let (tmp, app) = installed();
        let other = tmp.path().join("other-game");
        fs::create_dir_all(other.join("Data")).unwrap();
        // The environment's wins while it names one.
        app.set_setting("game", &other.to_string_lossy()).unwrap();
        app.reread_folders();
        assert_eq!(app.paths().game, Some(tmp.path().join("game")));
    }

    #[cfg(unix)]
    #[test]
    fn a_replacements_list_written_now_is_the_one_shown() {
        let (_tmp, app) = installed();
        assert!(app.replacements().is_empty());

        let file = rimstack_core::sources::replacements::file_in(app.data_dir());
        std::fs::create_dir_all(app.data_dir()).unwrap();
        let json = r#"{"rules":[{"oldWorkshopId":"111","newWorkshopId":"222","newName":"Roads Continued"}]}"#;
        let packed = std::process::Command::new("gzip")
            .arg("-c")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut gzip| {
                use std::io::Write;
                gzip.stdin.take().unwrap().write_all(json.as_bytes())?;
                gzip.wait_with_output()
            })
            .unwrap();
        std::fs::write(&file, packed.stdout).unwrap();
        assert!(
            rimstack_core::sources::replacements::read(&file).contains_key("111"),
            "control"
        );

        assert!(
            app.replacements().contains_key("111"),
            "a list downloaded now is shown at once"
        );
    }
}
