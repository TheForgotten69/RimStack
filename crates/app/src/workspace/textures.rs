//! Textures: how far each mod's are converted to DDS, converting them,
//! the DDS files the game would refuse, and choosing between two mods'
//! versions of one texture.

use super::App;
use rayon::prelude::*;
use rimstack_core::base::error::Error;
use rimstack_core::base::sentence::Sentence;
use rimstack_core::mods::hidden::{self, Hidden};
use rimstack_core::mods::library::{Library, ModId};
use rimstack_core::mods::{conflicts, ddsaudit, texdiff, textures};
use rimstack_core::{said, say_n};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// How often a conversion says how far it is, in textures.
const PROGRESS_EVERY: usize = 20;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowShown<'a> {
    #[serde(flatten)]
    pub row: &'a textures::Row,
    pub said: Sentence,
}

/// The rows on show and what the screen says of them all.
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TexturesShown<'a> {
    pub rows: Vec<RowShown<'a>>,
    /// Mods with textures, on show or not.
    pub all: usize,
    /// The state most rows are in, not repeated on each; empty for none.
    pub usual: &'static str,
    pub total: usize,
    /// Textures that have a DDS.
    pub have: usize,
    pub png_mb: f64,
    pub dds_mb: f64,
    /// What "convert the rest", "rebuild" and "remove" work on.
    pub remaining: Vec<ModId>,
    pub stale: Vec<ModId>,
    pub made: Vec<ModId>,
    pub made_files: usize,
    pub out_of_date: usize,
    /// Mods that converting would change, and the first of them.
    pub waiting: usize,
    pub first_waiting: Option<ModId>,
}

pub fn shown(rows: &[textures::Row], active_only: bool) -> TexturesShown<'_> {
    let on_show: Vec<&textures::Row> = rows.iter().filter(|r| !active_only || r.active).collect();
    let ids = |keep: &dyn Fn(&textures::Row) -> bool| -> Vec<ModId> {
        on_show.iter().filter(|r| keep(r)).map(|r| r.id).collect()
    };
    let mut counts: Vec<(&'static str, usize)> = Vec::new();
    for row in &on_show {
        match counts.iter_mut().find(|(state, _)| *state == row.state) {
            Some((_, n)) => *n += 1,
            None => counts.push((row.state, 1)),
        }
    }
    let usual = counts
        .iter()
        .max_by_key(|(_, n)| *n)
        .filter(|(_, n)| *n * 2 > on_show.len())
        .map_or("", |(state, _)| state);
    let made: Vec<&&textures::Row> = on_show
        .iter()
        .filter(|r| r.converted + r.out_of_date > 0)
        .collect();
    let total: usize = on_show.iter().map(|r| r.textures).sum();
    let waiting: Vec<ModId> = ids(&|r| r.waits);
    TexturesShown {
        all: rows.len(),
        usual,
        total,
        have: total - on_show.iter().map(|r| r.not_converted).sum::<usize>(),
        png_mb: on_show.iter().map(|r| r.png_mb).sum(),
        dds_mb: on_show.iter().map(|r| r.dds_mb).sum(),
        remaining: ids(&|r| !r.skipped && r.not_converted > 0),
        stale: ids(&|r| !r.skipped && r.out_of_date > 0),
        made_files: made.iter().map(|r| r.converted + r.out_of_date).sum(),
        made: made.iter().map(|r| r.id).collect(),
        out_of_date: on_show
            .iter()
            .filter(|r| !r.skipped)
            .map(|r| r.out_of_date)
            .sum(),
        waiting: waiting.len(),
        first_waiting: waiting.first().copied(),
        rows: on_show
            .into_iter()
            .map(|row| RowShown {
                row,
                said: Sentence::named(row.state),
            })
            .collect(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Refused {
    #[serde(rename = "mod")]
    pub of_mod: String,
    pub file: String,
    /// The file's own name.
    pub name: String,
    pub why: Sentence,
}

#[derive(Serialize, Clone)]
pub struct Side {
    pub id: ModId,
    pub name: String,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Shared {
    pub name: String,
    /// The last part of `name`.
    pub file: String,
    /// Its place among all the textures the two mods share.
    pub at: usize,
    #[serde(skip)]
    pub earlier: PathBuf,
    #[serde(skip)]
    pub later: PathBuf,
    /// The files as they are on disk: a hidden one under its renamed name.
    pub earlier_shown: String,
    pub later_shown: String,
    pub earlier_bytes: u64,
    pub later_bytes: u64,
    /// The later mod wins unless its file is hidden.
    pub keeps_earlier: bool,
    pub your_choice: bool,
    /// The mod whose file the game uses, and whether that was your choice.
    pub used: Sentence,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub earlier: Side,
    pub later: Side,
    #[serde(skip)]
    pub later_folder: PathBuf,
    #[serde(skip)]
    pub textures: Vec<Shared>,
    pub total: usize,
    pub kept_earlier: usize,
    /// How many were your choice.
    pub yours: usize,
}

impl Comparison {
    /// The shared textures whose name holds `filter`, whatever the case.
    pub fn named(&self, filter: &str) -> Vec<&Shared> {
        let filter = filter.trim().to_lowercase();
        self.textures
            .iter()
            .filter(|t| t.name.to_lowercase().contains(&filter))
            .collect()
    }
}

#[derive(Serialize)]
pub struct Difference {
    pub file: String,
    pub differing: u64,
    pub total: u64,
}

impl App {
    pub fn hidden_file(&self) -> PathBuf {
        self.data_dir().join("hidden.json")
    }

    /// The mods you asked not to convert, by folded package id.
    fn skipped_textures(&self) -> HashSet<String> {
        let settings = self.settings();
        settings.listed("textureSkip").map(String::from).collect()
    }

    /// Every installed mod's textures, counted, the mods that wait first.
    pub fn texture_rows(&self) -> Vec<textures::Row> {
        let loaded = self.library();
        let Ok((library, _)) = &*loaded else {
            return Vec::new();
        };
        let manifest = textures::Manifest::read(&self.texture_manifest_file());
        let skipped = self.skipped_textures();
        let active = self.active_now(library);
        let all: Vec<_> = library.ids().collect();
        let rows = all
            .par_iter()
            .filter_map(|&(i, m)| {
                let counted = textures::summarize(&m.path, &manifest);
                let skipped = m.key().is_some_and(|k| skipped.contains(&k));
                let active = active.contains(&i);
                textures::Row::of(i, m.display_name(), &m.path, counted, skipped, active)
            })
            .collect();
        textures::in_order(rows)
    }

    pub fn skip_textures(&self, id: ModId, skip: bool) -> Result<(), Error> {
        let key = {
            let loaded = self.library();
            let (library, _) = loaded.as_ref().as_ref().map_err(Clone::clone)?;
            library.get(id).and_then(|m| m.key())
        };
        let Some(key) = key else {
            return Ok(());
        };
        let mut skipped = self.skipped_textures();
        match skip {
            true => skipped.insert(key),
            false => skipped.remove(&key),
        };
        let mut kept: Vec<String> = skipped.into_iter().collect();
        kept.sort();
        self.set_setting("textureSkip", &kept.join(","))?;
        self.textures_changed();
        Ok(())
    }

    /// Makes DDS files for these mods, or with `remove` deletes those
    /// RimStack made, until `stop`; `progress` is told how far it is.
    /// Gives what to say of the textures that could not be converted.
    pub fn convert_textures(
        &self,
        ids: &[ModId],
        remove: bool,
        stop: &AtomicBool,
        progress: &(dyn Fn(Sentence) + Sync),
    ) -> Sentence {
        let loaded = self.library();
        let Ok((library, _)) = &*loaded else {
            return Sentence::default();
        };
        let roots: Vec<&Path> = ids
            .iter()
            .filter_map(|&i| library.get(i))
            .map(|m| m.path.as_path())
            .collect();
        let file = self.texture_manifest_file();
        let mut manifest = textures::Manifest::read(&file);
        let total: usize = roots
            .iter()
            .map(|root| {
                let counted = textures::summarize(root, &manifest);
                counted.not_converted() + counted.out_of_date
            })
            .sum();
        let done = AtomicUsize::new(0);
        let mut failed = 0;
        for root in roots {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            if remove {
                textures::remove(root, &mut manifest);
            } else {
                let outcome = textures::convert(root, &mut manifest, &|| {
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if n % PROGRESS_EVERY == 0 || n == total {
                        progress(say_n!(
                            total,
                            "{} texture: {} converted",
                            "{} textures: {} converted",
                            n
                        ));
                    }
                });
                failed += outcome.failed.len();
                for (png, reason) in outcome.failed {
                    rimstack_core::problem!("{}: {reason}", png.display());
                }
            }
            // After every mod, so a crash loses track of little.
            if let Err(problem) = manifest.write(&file) {
                rimstack_core::problem!("{problem}");
            }
        }
        self.textures_changed();
        match failed {
            0 => Sentence::default(),
            n => say_n!(
                n,
                "{} texture could not be converted; it keeps using its PNG.",
                "{} textures could not be converted; they keep using their PNG."
            ),
        }
    }

    /// The DDS files of the installed mods the game would refuse.
    pub fn refused_dds(&self) -> Vec<Refused> {
        let loaded = self.library();
        let Ok((library, _)) = &*loaded else {
            return Vec::new();
        };
        library
            .installed
            .par_iter()
            .flat_map_iter(|m| {
                ddsaudit::audit(&m.path).into_iter().map(|bad| Refused {
                    of_mod: m.display_name().to_string(),
                    name: bad
                        .file
                        .file_name()
                        .map_or(String::new(), |n| n.to_string_lossy().into_owned()),
                    file: bad.file.to_string_lossy().into_owned(),
                    why: bad.why,
                })
            })
            .collect()
    }

    /// Renames each of these refused files out of the game's way; gives
    /// how many could not be.
    pub fn set_dds_aside(&self, refused: &[Refused]) -> usize {
        let failed = refused
            .iter()
            .filter(|bad| ddsaudit::set_aside(Path::new(&bad.file)).is_err())
            .count();
        self.textures_changed();
        failed
    }

    fn loaded_folders(&self, library: &Library, id: ModId) -> Vec<PathBuf> {
        let list = self.session().current().to_vec();
        let active: HashSet<String> = library
            .arrange(&list)
            .active
            .iter()
            .filter_map(|m| library.get(m.installed?)?.key())
            .collect();
        let Some(root) = library.get(id).map(|m| &m.path) else {
            return Vec::new();
        };
        rimstack_core::rimworld::loadfolders::folders(root, &library.game_version, &|k| {
            active.contains(k)
        })
    }

    /// The textures two mods share; `earlier` loads first.
    pub fn compare_textures(&self, earlier: ModId, later: ModId) -> Option<Comparison> {
        let loaded = self.library();
        let (library, _) = loaded.as_ref().as_ref().ok()?;
        let (one, other) = (library.get(earlier)?, library.get(later)?);
        let (one_name, other_name) = (one.display_name(), other.display_name());
        let first = conflicts::files_with_hidden(&self.loaded_folders(library, earlier));
        let second = conflicts::files_with_hidden(&self.loaded_folders(library, later));
        let on_disk = |(path, hidden): &(PathBuf, bool)| match hidden {
            true => format!("{}{}", path.to_string_lossy(), hidden::SUFFIX),
            false => path.to_string_lossy().into_owned(),
        };
        let size = |file: &str| std::fs::metadata(file).map_or(0, |m| m.len());
        let textures: Vec<Shared> = first
            .iter()
            .filter_map(|(name, one)| Some((name, one, second.get(name)?)))
            .enumerate()
            .map(|(at, (name, one, other))| {
                let keeps_earlier = other.1 && !one.1;
                let your_choice = one.1 || other.1;
                let user = match keeps_earlier {
                    true => one_name,
                    false => other_name,
                };
                Shared {
                    file: name.rsplit('/').next().unwrap_or(name).to_string(),
                    name: name.clone(),
                    at,
                    earlier: one.0.clone(),
                    later: other.0.clone(),
                    earlier_bytes: size(&on_disk(one)),
                    later_bytes: size(&on_disk(other)),
                    earlier_shown: on_disk(one),
                    later_shown: on_disk(other),
                    keeps_earlier,
                    your_choice,
                    used: match your_choice {
                        true => said!("{}, chosen", user),
                        false => Sentence::Plain(user.to_string()),
                    },
                }
            })
            .collect();
        Some(Comparison {
            earlier: Side {
                id: earlier,
                name: one_name.to_string(),
            },
            later: Side {
                id: later,
                name: other_name.to_string(),
            },
            later_folder: other.path.clone(),
            total: textures.len(),
            kept_earlier: textures.iter().filter(|t| t.keeps_earlier).count(),
            yours: textures.iter().filter(|t| t.your_choice).count(),
            textures,
        })
    }

    /// Which of two mods' versions of a texture the game uses: keeping
    /// the earlier mod's hides the later one's file by renaming it;
    /// keeping the later one's puts every file back.
    pub fn keep_textures(&self, chosen: &[&Shared], keep_earlier: bool) -> Result<(), Error> {
        let file = self.hidden_file();
        let mut hidden = Hidden::read(&file);
        let done = chosen.iter().try_for_each(|shared| {
            hidden.unhide(&shared.earlier)?;
            hidden.unhide(&shared.later)?;
            match keep_earlier {
                true => hidden.hide(&shared.later),
                false => Ok(()),
            }
        });
        let written = hidden.write(&file);
        self.textures_changed();
        done.and(written)
    }

    /// Keeps every shared texture from one side; keeping the earlier
    /// mod's is told in the history, with the way to take it back.
    /// Gives what to say of it.
    pub fn keep_all_textures(
        &self,
        comparison: &Comparison,
        keep_earlier: bool,
    ) -> Result<Sentence, Error> {
        let all: Vec<&Shared> = comparison.textures.iter().collect();
        self.keep_textures(&all, keep_earlier)?;
        if !keep_earlier {
            return Ok(Sentence::default());
        }
        let (n, name) = (all.len(), comparison.later.name.as_str());
        let aside = say_n!(
            n,
            "Set aside {} texture of {}",
            "Set aside {} textures of {}",
            name
        );
        let folder = comparison.later_folder.to_string_lossy();
        self.note(aside.clone(), format!("unhide {folder}"));
        Ok(aside)
    }

    /// How many PNG textures RimStack has set aside in a mod.
    pub fn hidden_in(&self, id: ModId) -> usize {
        let loaded = self.library();
        let Some(m) = loaded.as_ref().as_ref().ok().and_then(|(l, _)| l.get(id)) else {
            return 0;
        };
        let hidden = Hidden::read(&self.hidden_file());
        let pngs = hidden.in_mod(&m.path).into_iter();
        pngs.filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")))
            .count()
    }

    /// Puts back every texture RimStack set aside in an installed mod's folder.
    pub fn unhide_folder(&self, folder: &Path) -> Result<(), Error> {
        let installed = {
            let loaded = self.library();
            let library = loaded.as_ref().as_ref().map_err(Clone::clone)?;
            library.0.installed.iter().any(|m| m.path == folder)
        };
        if !installed {
            return Err(Error::refused(said!(
                "{} is not an installed mod's folder",
                folder.display()
            )));
        }
        let file = self.hidden_file();
        let mut hidden = Hidden::read(&file);
        let done = hidden.unhide_all(folder).and_then(|_| hidden.write(&file));
        self.textures_changed();
        done
    }

    pub fn unhide_mod(&self, id: ModId) -> Result<(), Error> {
        let folder = {
            let loaded = self.library();
            let library = loaded.as_ref().as_ref().map_err(Clone::clone)?;
            library.0.get(id).map(|m| m.path.clone())
        };
        match folder {
            Some(folder) => self.unhide_folder(&folder),
            None => Ok(()),
        }
    }

    /// Writes the picture of where two versions of a texture differ into
    /// RimStack's cache; one picture at a time is kept.
    pub fn texture_difference(&self, shared: &Shared) -> Result<Difference, Error> {
        use std::hash::{Hash, Hasher};
        let dir = self.data_dir().join("cache");
        for old in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            if old.file_name().to_string_lossy().starts_with("difference-") {
                let _ = std::fs::remove_file(old.path());
            }
        }
        let (from, to) = (&shared.earlier_shown, &shared.later_shown);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (from, to).hash(&mut hasher);
        let file = dir.join(format!("difference-{:016x}.png", hasher.finish()));
        let (differing, total) = texdiff::write(Path::new(from), Path::new(to), &file)?;
        Ok(Difference {
            file: file.to_string_lossy().into_owned(),
            differing,
            total,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::tests::installed;

    fn texture(at: &Path) {
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, [0u8; 16]).unwrap();
    }

    #[test]
    fn a_mod_with_textures_is_listed_until_you_ask_to_skip_it() {
        let (tmp, app) = installed();
        texture(&tmp.path().join("game/Mods/roads/Textures/road.png"));
        let rows = app.texture_rows();
        let on_show = crate::qml::values::as_shown(&shown(&rows, false));
        assert_eq!(on_show["rows"][0]["name"], "Ann.Roads");
        assert_eq!(on_show["rows"][0]["state"], "Not converted");
        assert_eq!(on_show["usual"], "Not converted");
        assert_eq!(on_show["total"], 1);
        assert_eq!(on_show["have"], 0);
        assert_eq!(on_show["waiting"], 1);
        assert_eq!(on_show["remaining"].as_array().unwrap().len(), 1);
        assert!(shown(&rows, true).rows.is_empty(), "Roads is not active");

        app.skip_textures(rows[0].id, true).unwrap();
        let rows = app.texture_rows();
        assert_eq!((rows[0].state, rows[0].waits), ("Skipped", false));
        assert!(shown(&rows, false).remaining.is_empty());
    }

    #[test]
    fn two_mods_sharing_a_texture_keep_either_version_and_the_history_can_undo_it() {
        let (tmp, app) = installed();
        let mods = tmp.path().join("game/Mods");
        for (folder, id) in [("a", "Ann.A"), ("b", "Ann.B")] {
            let about = mods.join(folder).join("About");
            std::fs::create_dir_all(&about).unwrap();
            std::fs::write(
                about.join("About.xml"),
                format!("<ModMetaData><packageId>{id}</packageId><name>{id}</name></ModMetaData>"),
            )
            .unwrap();
            texture(&mods.join(folder).join("Textures/Things/rock.png"));
        }
        app.rescan();
        app.session().record(vec![
            "ludeon.rimworld".into(),
            "ann.a".into(),
            "ann.b".into(),
        ]);
        let id_of = |folder: &str| {
            let loaded = app.library();
            let (library, _) = loaded.as_ref().as_ref().unwrap();
            let found = library.ids().find(|(_, m)| m.folder == folder).unwrap().0;
            found
        };
        let (a, b) = (id_of("a"), id_of("b"));
        let compared = app.compare_textures(a, b).unwrap();
        assert_eq!((compared.total, compared.kept_earlier), (1, 0));
        assert_eq!(compared.named(" ROCK").len(), 1);
        assert!(compared.named("tree").is_empty());

        let said = app.keep_all_textures(&compared, true).unwrap();
        assert_eq!(said.to_string(), "Set aside 1 texture of Ann.B");
        assert_eq!((app.hidden_in(b), app.hidden_in(a)), (1, 0));
        let again = app.compare_textures(a, b).unwrap();
        assert_eq!((again.kept_earlier, again.yours), (1, 1));
        let history = app.history();
        assert_eq!(history[0].said, said);
        assert_eq!(Path::new(&history[0].bring_back), mods.join("b"));

        assert!(app.unhide_folder(tmp.path()).is_err());
        app.unhide_folder(Path::new(&history[0].bring_back))
            .unwrap();
        assert_eq!(app.hidden_in(b), 0);
        assert!(mods.join("b/Textures/Things/rock.png").is_file());
    }
}
