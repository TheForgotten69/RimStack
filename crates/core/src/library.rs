//! The installed mods and which of them are active.

use crate::say;
use crate::tongue::named;

use crate::about::{self, About};
use crate::id;
use crate::list::{Category, Mod};
use crate::paths::{ModRoot, Paths, Source};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use crate::classify::Kinds;
use crate::classify::{classify, Standing};
use crate::code::Code;
use crate::game::Game;

#[derive(Debug, Clone, PartialEq)]
pub struct Installed {
    pub path: PathBuf,
    pub source: Source,
    /// Folder name; the Workshop id for Workshop mods.
    pub folder: String,
    /// Why the mod could not be read, if it could not.
    pub about: Result<About, String>,
    pub kinds: Kinds,
    /// What its code is about and patches; nothing until `read_code`.
    pub code: Code,
    pub preview: Option<PathBuf>,
    /// The folder is a Git checkout.
    pub git: bool,
}

impl Installed {
    pub fn key(&self) -> Option<String> {
        self.about.as_ref().ok().map(|a| id::fold(&a.package_id))
    }

    /// The names of the mod's assemblies, lower case, without `.dll`:
    /// those in `Assemblies` at its root and one folder down (a game
    /// version or an optional part). Reads the disk.
    pub fn assemblies(&self) -> Vec<String> {
        let dlls_in = |dir: PathBuf| -> Vec<String> {
            let entries = std::fs::read_dir(dir.join("Assemblies"))
                .into_iter()
                .flatten();
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("dll")))
                .filter_map(|p| Some(p.file_stem()?.to_string_lossy().to_lowercase()))
                .collect()
        };
        let mut names = dlls_in(self.path.clone());
        for entry in std::fs::read_dir(&self.path)
            .into_iter()
            .flatten()
            .flatten()
        {
            names.extend(dlls_in(entry.path()));
        }
        names.sort_unstable();
        names.dedup();
        names
    }

    pub fn source_label(&self) -> &'static str {
        match self.source {
            Source::Official => "Ludeon",
            Source::Local => "Local",
            Source::Workshop => "Workshop",
        }
    }

    /// "Defs, patches, code, textures": what the mod ships, in words.
    pub fn contains(&self) -> String {
        let k = self.kinds;
        [
            (k.defs, named("defs")),
            (k.patches, named("patches")),
            (k.assemblies, named("code")),
            (k.textures, named("textures")),
        ]
        .iter()
        .filter(|(present, _)| *present)
        .map(|(_, word)| crate::tongue::say(word))
        .collect::<Vec<_>>()
        .join(", ")
    }

    pub fn display_name(&self) -> &str {
        match &self.about {
            Ok(About {
                name: Some(name), ..
            }) => name,
            // The game and its expansions carry no name; their folders do.
            Ok(a) if self.source != Source::Official => &a.package_id,
            _ => &self.folder,
        }
    }
}

/// Content folders can sit in the mod's root or one level down (`1.6/`,
/// `Common/`, or wherever `LoadFolders.xml` points), so look in both.
fn kinds(mod_root: &Path) -> Kinds {
    let mut found = Kinds::default();
    let mut look = |dir: &Path, descend: bool| -> Vec<PathBuf> {
        let mut below = Vec::new();
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            if !entry.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            match entry.file_name().to_string_lossy().to_lowercase().as_str() {
                "defs" => found.defs = true,
                "patches" => found.patches = true,
                "assemblies" => found.assemblies = true,
                "textures" => found.textures = true,
                "about" | "source" | "languages" | "sounds" | ".git" => {}
                _ if descend => below.push(entry.path()),
                _ => {}
            }
        }
        below
    };
    for dir in look(mod_root, true) {
        look(&dir, false);
    }
    found
}

fn read_mod(path: PathBuf, source: Source) -> Installed {
    let about = match about::locate(&path) {
        None => Err("no About.xml".to_string()),
        Some(file) => std::fs::read(&file)
            .map_err(|e| format!("About.xml: {e}"))
            .and_then(|bytes| about::parse(&bytes)),
    };
    Installed {
        folder: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        kinds: kinds(&path),
        code: Code::default(),
        preview: about::preview(&path),
        git: path.join(".git").exists(),
        about,
        source,
        path,
    }
}

/// Reads every mod folder under the roots, on all cores. Sorted by source,
/// then folder name.
pub fn scan(roots: &[ModRoot]) -> Vec<Installed> {
    let folders: Vec<(PathBuf, Source)> = roots
        .iter()
        .flat_map(|root| {
            std::fs::read_dir(&root.path)
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| (e.path(), root.source))
        })
        .collect();
    let mut mods: Vec<Installed> = folders
        .into_par_iter()
        .map(|(path, source)| read_mod(path, source))
        .collect();
    mods.sort_by(|a, b| (a.source, &a.folder).cmp(&(b.source, &b.folder)));
    mods
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resolved<'a> {
    Found(&'a Installed),
    /// Active in `ModsConfig.xml` but not installed; the id as written.
    Missing(&'a str),
}

/// One entry per active id, in order. When several installed mods share an
/// id, the official one wins, then local, then Workshop; an id ending in
/// `_steam` picks the Workshop copy.
pub fn resolve_active<'a>(active: &'a [String], mods: &'a [Installed]) -> Vec<Resolved<'a>> {
    let mut by_key: HashMap<String, Vec<&Installed>> = HashMap::new();
    for m in mods {
        if let Some(key) = m.key() {
            by_key.entry(key).or_default().push(m);
        }
    }
    active
        .iter()
        .map(|written| {
            let (key, steam) = id::split_steam(written);
            let copies = by_key.get(&key).map(Vec::as_slice).unwrap_or_default();
            let chosen = if steam {
                copies.iter().find(|m| m.source == Source::Workshop)
            } else {
                copies.iter().min_by_key(|m| m.source)
            };
            match chosen {
                Some(m) => Resolved::Found(m),
                // "x_steam" may also simply be a mod's real id.
                None => match by_key.get(&id::fold(written)).and_then(|c| c.first()) {
                    Some(m) => Resolved::Found(m),
                    None => Resolved::Missing(written),
                },
            }
        })
        .collect()
}

/// Reads what the code of every mod with assemblies is about, on all
/// cores. Without the game's own code to hold it against, nothing is read.
pub fn read_code(mods: &mut [Installed], game: Option<&Game>, game_version: &str) {
    let Some(game) = game else {
        return;
    };
    mods.par_iter_mut()
        .filter(|m| m.kinds.assemblies && m.source != Source::Official)
        .for_each(|m| m.code = crate::code::read(&m.path, game_version, game));
}

/// The mods that ask to load before the game or one of its expansions,
/// and those that ask to load before one of them, by folded package id.
fn before_game(mods: &[Installed], game_version: &str) -> std::collections::HashSet<String> {
    let official = |m: &&Installed| m.source == Source::Official;
    let mut ahead: std::collections::HashSet<String> = mods
        .iter()
        .filter(official)
        .filter_map(Installed::key)
        .collect();
    let game: std::collections::HashSet<String> = ahead.clone();
    loop {
        let more: Vec<String> = mods
            .iter()
            .filter_map(|m| Some((m.key()?, m.about.as_ref().ok()?)))
            .filter(|(key, _)| !ahead.contains(key))
            .filter(|(_, about)| {
                let asks = about.load_before.for_version(game_version);
                let asks = asks.iter().chain(&about.force_load_before);
                asks.into_iter().any(|id| ahead.contains(&id::fold(id)))
            })
            .map(|(key, _)| key)
            .collect();
        if more.is_empty() {
            break;
        }
        ahead.extend(more);
    }
    ahead.retain(|key| !game.contains(key));
    ahead
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Library {
    /// Every mod folder found. `list::Mod::installed` points in here.
    pub installed: Vec<Installed>,
    /// "1.6".
    pub game_version: String,
    /// As on disk when the library was read, in load order.
    pub active: Vec<Mod>,
    pub inactive: Vec<Mod>,
    pub missing: usize,
    /// Installed folders that could not be read as a mod.
    pub unreadable: usize,
    /// Display name by folded package id.
    names: HashMap<String, String>,
    /// Each installed mod's own category, before sorting moves it.
    pub categories: Vec<Category>,
}

impl Library {
    pub fn name_of(&self, key: &str) -> Option<String> {
        self.names.get(key).cloned()
    }

    /// Each listed mod's `About.xml`, in list order, for `rules::among`.
    pub fn abouts(&self, mods: &[Mod]) -> Vec<Option<&About>> {
        mods.iter()
            .map(|m| self.installed.get(m.installed?)?.about.as_ref().ok())
            .collect()
    }

    /// The mods this one requires for the current game version: display
    /// name (or the id, when not installed) and whether it is installed.
    pub fn requirements(&self, m: &Installed) -> Vec<(String, bool)> {
        let Ok(about) = &m.about else {
            return Vec::new();
        };
        about
            .dependencies
            .for_version(&self.game_version)
            .iter()
            .map(|dep| {
                let key = id::fold(dep);
                match self
                    .installed
                    .iter()
                    .find(|x| x.key().as_deref() == Some(&key))
                {
                    Some(found) => (found.display_name().to_string(), true),
                    None => (dep.clone(), false),
                }
            })
            .collect()
    }
}

fn patched(aim: &crate::code::Aim) -> String {
    format!("{}.{}", aim.target, aim.method).to_lowercase()
}

fn row(m: &Installed, id: String, category: Category, installed: usize) -> Mod {
    let about = m.about.as_ref().ok();
    Mod {
        id,
        tags: Vec::new(),
        local: m.source == Source::Local,
        code: m.kinds.assemblies,
        patched: m.code.aims.iter().map(patched).collect(),
        sub: String::new(),
        load_micros: None,
        name: m.display_name().to_string(),
        author: about.map(|a| a.authors.join(", ")).unwrap_or_default(),
        version: about
            .and_then(|a| {
                a.mod_version
                    .clone()
                    .or(a.supported_versions.last().cloned())
            })
            .unwrap_or_default(),
        category,
        installed: Some(installed),
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Arranged {
    pub active: Vec<Mod>,
    pub inactive: Vec<Mod>,
    /// Ids with no installed mod; they stay in `active` as placeholders.
    pub missing: usize,
}

impl Library {
    /// The installed mods whose name holds every word of `text`, or whose
    /// package id holds it, as indexes into `installed`: a name starting
    /// with the text first, then the shorter name. At most `most`.
    pub fn find(&self, text: &str, most: usize) -> Vec<usize> {
        let text = text.trim().to_lowercase();
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return Vec::new();
        }
        let mut found: Vec<(bool, usize, String, usize)> = Vec::new();
        for (i, m) in self.installed.iter().enumerate() {
            let name = m.display_name().to_lowercase();
            let by_id = m.key().is_some_and(|key| key.contains(&text));
            if words.iter().all(|w| name.contains(w)) || by_id {
                found.push((!name.starts_with(&text), name.chars().count(), name, i));
            }
        }
        found.sort();
        found.into_iter().take(most).map(|f| f.3).collect()
    }

    /// Splits the installed mods into those named by `ids` (package ids as
    /// `ModsConfig.xml` writes them, in load order) and the rest.
    pub fn arrange(&self, ids: &[String]) -> Arranged {
        let index_of: HashMap<&Path, usize> = self
            .installed
            .iter()
            .enumerate()
            .map(|(i, m)| (m.path.as_path(), i))
            .collect();
        let mut arranged = Arranged::default();
        let mut is_active = vec![false; self.installed.len()];
        for (written, entry) in ids.iter().zip(resolve_active(ids, &self.installed)) {
            match entry {
                Resolved::Found(m) => {
                    let i = index_of[m.path.as_path()];
                    is_active[i] = true;
                    arranged
                        .active
                        .push(row(m, written.clone(), self.categories[i], i));
                }
                Resolved::Missing(written) => {
                    arranged.missing += 1;
                    arranged.active.push(Mod {
                        id: written.to_string(),
                        tags: Vec::new(),
                        local: false,
                        code: false,
                        patched: Default::default(),
                        sub: String::new(),
                        load_micros: None,
                        name: say!("{} (not installed)", written),
                        author: String::new(),
                        version: String::new(),
                        category: Category::Content,
                        installed: None,
                    });
                }
            }
        }
        for (i, m) in self.installed.iter().enumerate() {
            // The other copy of an active mod stays listed as inactive: the
            // game lets only one of the two be active.
            if !is_active[i] {
                arranged
                    .inactive
                    .push(row(m, self.written_id(i), self.categories[i], i));
            }
        }
        arranged
            .inactive
            .sort_by_cached_key(|m| (m.category, m.name.to_lowercase()));
        arranged
    }

    /// The id that makes the game load exactly this folder: its package id,
    /// with `_steam` for a Workshop copy of a mod that is also installed
    /// elsewhere. Empty for a folder that cannot be read as a mod.
    fn written_id(&self, i: usize) -> String {
        let m = &self.installed[i];
        let Some(key) = m.key() else {
            return String::new();
        };
        let also_elsewhere = || {
            let same =
                |x: &&Installed| x.source != Source::Workshop && x.key().as_ref() == Some(&key);
            self.installed.iter().any(|x| same(&x))
        };
        if m.source == Source::Workshop && also_elsewhere() {
            format!("{key}_steam")
        } else {
            key
        }
    }
}

pub fn build(mods: Vec<Installed>, active: &[String], game_version: &str) -> Library {
    let mut dependents: HashMap<String, usize> = HashMap::new();
    for about in mods.iter().filter_map(|m| m.about.as_ref().ok()) {
        for dep in about.dependencies.for_version(game_version) {
            *dependents.entry(id::fold(dep)).or_default() += 1;
        }
    }
    let ahead = before_game(&mods, game_version);
    let category = |m: &Installed| {
        let key = m.key();
        let standing = Standing {
            official: m.source == Source::Official,
            before_game: key.as_ref().is_some_and(|k| ahead.contains(k)),
            dependents: key.and_then(|k| dependents.get(&k).copied()).unwrap_or(0),
        };
        classify(m.kinds, &m.code.facts, standing)
    };
    let mut library = Library {
        game_version: game_version.to_string(),
        categories: mods.iter().map(category).collect(),
        unreadable: mods.iter().filter(|m| m.about.is_err()).count(),
        ..Library::default()
    };
    for m in &mods {
        if let Some(key) = m.key() {
            // The first copy found names the id.
            let name = || m.display_name().to_string();
            library.names.entry(key).or_insert_with(name);
        }
    }
    library.installed = mods;
    let arranged = library.arrange(active);
    library.active = arranged.active;
    library.inactive = arranged.inactive;
    library.missing = arranged.missing;
    library
}

pub fn load(paths: &Paths) -> Result<Library, String> {
    let roots = paths.roots();
    if roots.is_empty() {
        return Err(say!(
            "RimWorld was not found. Set RIMSTACK_GAME to its folder."
        ));
    }
    let config = paths.mods_config().ok_or_else(|| {
        say!("RimWorld's settings folder was not found. Set RIMSTACK_CONFIG to it.")
    })?;
    let active = crate::modsconfig::read(&config)?.active;
    let version = paths.game_version.as_deref().unwrap_or_default();
    // The game's own code is read while the mods' folders are.
    let (mut mods, game) = rayon::join(
        || scan(&roots),
        || paths.game.as_deref().and_then(Game::find),
    );
    read_code(&mut mods, game.as_ref(), version);
    Ok(build(mods, &active, version))
}

#[cfg(test)]
mod real;
#[cfg(test)]
mod tests;
