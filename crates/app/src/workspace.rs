//! What every part of the app shares: where its files are, what was read
//! from disk, and the load order being edited. Each is read on first
//! use. What follows the folders is an `Install`, replaced whole; what is
//! read from one file is `Derived` from its time of writing.

use rimstack_core::base::derived::Derived;
use rimstack_core::base::locks::{Held, Shared};
use rimstack_core::base::reread::Reread;
use rimstack_core::{
    kept::notes::Notes, kept::settings::Settings, mods::community, mods::library,
    mods::session::Session, mods::userrules::UserRules, rimworld::paths,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, RwLock, RwLockReadGuard};
use std::time::{Duration, Instant};

pub mod changed;
pub mod downloads;
pub mod fetch;
mod game;
mod install;
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
pub type SteamRecord = (HashMap<String, u64>, HashMap<String, u64>);

type Replacements = HashMap<String, rimstack_core::sources::replacements::Replacement>;

/// A file and when it was written, in milliseconds since 1970.
type Written = (PathBuf, u64);

fn written(file: PathBuf) -> Written {
    let at = rimstack_core::base::watch::changed_at(&file);
    (file, at)
}

pub struct App {
    home: PathBuf,
    settings_dir: PathBuf,
    data_dir: PathBuf,
    /// Folders set in the environment: they win over those set in
    /// Settings, which win over what detection finds.
    given: paths::Overrides,
    /// The languages the system is set to, most wanted first, as
    /// "fr-FR": where none is chosen, the first offered is used.
    system_languages: Vec<String>,
    /// That language, found again once a language file comes or goes.
    detected_language: Derived<u64, String>,
    settings: RwLock<Settings>,
    install: Reread<install::Install>,
    session: OnceLock<Mutex<Session>>,
    notes: OnceLock<RwLock<Notes>>,
    user_rules: OnceLock<RwLock<UserRules>>,
    seen: OnceLock<RwLock<rimstack_core::kept::new_mods::Seen>>,
    community: Derived<Written, Option<community::Community>>,
    works_on: Derived<Written, HashSet<String>>,
    replacements: Derived<Written, Replacements>,
    workshop_names: Derived<Written, rimstack_core::rimworld::workshopdb::Names>,
    texture_manifest: Derived<Written, rimstack_core::mods::textures::Manifest>,
    /// Where SteamCMD is: found by looking through `PATH`, which several
    /// screens ask after on every change.
    steamcmd: Reread<Option<PathBuf>>,
    conflicts: RwLock<Option<Conflicts>>,
    behind: RwLock<Option<HashSet<String>>>,
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
    pub fn from_environment() -> Self {
        let env = &paths::from_environment;
        let home = paths::home(env);
        let (settings, data) = paths::app_dirs(paths::Os::running(), &home, env);
        Self::new(home, &settings, &data, paths::Overrides::from_env())
            .speaking(sys_locale::get_locales().collect())
    }

    pub fn speaking(self, system_languages: Vec<String>) -> Self {
        Self {
            system_languages,
            ..self
        }
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
            system_languages: Vec::new(),
            detected_language: Derived::default(),
            install: Reread::new(),
            session: OnceLock::new(),
            notes: OnceLock::new(),
            user_rules: OnceLock::new(),
            seen: OnceLock::new(),
            community: Derived::default(),
            works_on: Derived::default(),
            replacements: Derived::default(),
            workshop_names: Derived::default(),
            texture_manifest: Derived::default(),
            steamcmd: Reread::new(),
            conflicts: RwLock::new(None),
            behind: RwLock::new(None),
            list_on_disk: Mutex::new((0, false)),
            git: OnceLock::new(),
            log_read: Mutex::new(None),
            bus: crate::changes::Bus::default(),
        }
    }

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

    fn install(&self) -> Arc<install::Install> {
        self.install
            .get(|| install::Install::of(Arc::new(self.find_paths())))
    }

    pub fn paths(&self) -> Arc<paths::Paths> {
        self.install().paths().clone()
    }

    /// Steam's record of the Workshop items it has downloaded. Empty
    /// without Steam.
    pub fn steam_record(&self) -> Arc<SteamRecord> {
        self.install().steam_record()
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

    /// The load order being edited, starting from the one on disk.
    pub fn session(&self) -> MutexGuard<'_, Session> {
        self.session
            .get_or_init(|| Mutex::new(Session::new(self.install().on_disk())))
            .held()
    }

    /// The mods in the load order being edited.
    pub fn active_now(&self, library: &library::Library) -> HashSet<library::ModId> {
        let list = self.session().current().to_vec();
        library.active_ids(&list)
    }

    pub fn library(&self) -> Arc<Loaded> {
        self.install().library()
    }

    pub fn texture_count(&self, mod_root: &Path) -> usize {
        self.install().texture_count(mod_root)
    }

    /// Reads the mod folders again. Whoever still shows the earlier
    /// reading holds it until done.
    pub fn rescan(&self) {
        let fresh = install::Install::of(self.paths()).read_all();
        self.install.put(fresh);
        self.textures_changed();
        self.publish(crate::changes::Changes::LIBRARY);
    }

    /// A texture was hidden, shown again or read anew: what was worked
    /// out from the textures on disk no longer holds.
    pub fn textures_changed(&self) {
        *self.conflicts.writing() = None;
        self.install().forget_texture_counts();
        self.publish(crate::changes::Changes::TEXTURES);
    }

    /// The folders were changed in Settings: finds them again, reads the
    /// mods there, and starts from the load order on disk there. All of it
    /// is in place before anyone is told.
    pub fn reread_folders(&self) {
        let fresh = install::Install::of(Arc::new(self.find_paths())).read_all();
        let on_disk = fresh.on_disk();
        self.install.put(fresh);
        *self.session() = Session::new(on_disk);
        *self.behind.writing() = None;
        *self.list_on_disk.held() = (0, false);
        *self.log_read.held() = None;
        self.textures_changed();
        self.publish(crate::changes::Changes::LIBRARY);
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

    /// Where the game writes its log: up to 1.6 in the settings folder,
    /// from 1.7 in `Logs` there.
    fn game_logs(&self) -> Vec<PathBuf> {
        let paths = self.paths();
        let Some(config) = paths.config.as_ref() else {
            return Vec::new();
        };
        vec![
            config.join("Player.log"),
            config.join("Logs").join("Player.log"),
        ]
    }

    /// When the game last wrote a log, in milliseconds since 1970; 0 for
    /// never.
    pub fn log_written(&self) -> u64 {
        let logs = self.game_logs();
        rimstack_core::base::watch::newest(logs.iter().map(PathBuf::as_path))
    }

    /// The log the game wrote last, if it wrote one.
    pub fn game_log(&self) -> Option<PathBuf> {
        let logs = self.game_logs().into_iter();
        logs.filter(|log| log.is_file())
            .max_by_key(|log| rimstack_core::base::watch::changed_at(log))
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
        let language = Some(
            self.languages_dir()
                .join(format!("{}.json", self.language_in_use())),
        );
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
    pub fn community(&self) -> Arc<Option<community::Community>> {
        let file = community::file_in(&self.data_dir);
        self.community.get(written(file.clone()), || {
            community::read(&file).unwrap_or_else(|problem| {
                rimstack_core::problem!("{problem}");
                None
            })
        })
    }

    fn works_on_file(&self) -> PathBuf {
        let version = self.paths().game_version.clone().unwrap_or_default();
        community::works_on_file(&self.data_dir, &version)
    }

    /// Mods known to work on the installed game version without listing
    /// it (the "No Version Warning" list), if it was ever downloaded.
    pub fn works_on(&self) -> Arc<HashSet<String>> {
        let file = self.works_on_file();
        self.works_on
            .get(written(file.clone()), || community::read_works_on(&file))
    }

    /// Workshop mods with a maintained replacement, by Workshop id: its
    /// own download of the Use This Instead list, or RimSort's copy for
    /// an install that has none yet.
    pub fn replacements(&self) -> Arc<Replacements> {
        let own = rimstack_core::sources::replacements::file_in(&self.data_dir);
        let rimsort = self
            .data_dir
            .with_file_name("RimSort")
            .join("dbs/UseThisInstead/replacements.json.gz");
        let file = if own.is_file() { own } else { rimsort };
        self.replacements.get(written(file.clone()), || {
            rimstack_core::sources::replacements::read(&file)
        })
    }

    pub fn workshop_names(&self) -> Arc<rimstack_core::rimworld::workshopdb::Names> {
        let file = rimstack_core::rimworld::workshopdb::file_in(&self.data_dir);
        self.workshop_names.get(written(file.clone()), || {
            rimstack_core::rimworld::workshopdb::read(&file)
        })
    }

    /// The record of the DDS files RimStack made: 141 ms to read at
    /// 150,000 of them, so read again only once written.
    pub fn texture_manifest(&self) -> Arc<rimstack_core::mods::textures::Manifest> {
        let file = self.texture_manifest_file();
        self.texture_manifest.get(written(file.clone()), || {
            rimstack_core::mods::textures::Manifest::read(&file)
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

    /// What Loading Progress measured at the game's last start.
    pub fn startup(&self) -> Arc<rimstack_core::rimworld::startup::Measured> {
        self.install().startup()
    }

    /// The mods of the newest save, by folded package id; `None` without a save.
    pub fn newest_save(&self) -> Arc<Option<HashSet<String>>> {
        self.install().newest_save()
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
    fn a_game_folder_given_by_the_environment_wins_over_one_set_in_settings() {
        let (tmp, app) = installed();
        let other = tmp.path().join("other-game");
        fs::create_dir_all(other.join("Data")).unwrap();
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
