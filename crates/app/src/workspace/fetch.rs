//! What Downloads asks of Steam, Git and SteamCMD. Each answers with what
//! it did, as a sentence for the history, or why it changed nothing.

use super::App;
use rimstack_core::base::error::{Error, Result};
use rimstack_core::mods::library::ModId;
use rimstack_core::sources::{git, redownload};
use rimstack_core::{keep, said};
use std::path::PathBuf;

/// How a task ended.
pub struct Done {
    /// What it did, kept as `translate::kept` writes it, or why it changed nothing.
    pub note: Result<String>,
    /// It changed a mod's folder on disk.
    pub wrote: bool,
}

impl Done {
    fn told(note: Result<String>) -> Self {
        Self { note, wrote: false }
    }

    /// For a task that changes a mod's folder unless it fails.
    fn of_a_write(note: Result<String>) -> Self {
        Self {
            wrote: note.is_ok(),
            note,
        }
    }
}

/// What Steam is asked to do with Workshop items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteamAsk {
    Subscribe,
    Unsubscribe,
    Download,
}

impl SteamAsk {
    fn word(self) -> &'static str {
        match self {
            SteamAsk::Subscribe => "subscribe",
            SteamAsk::Unsubscribe => "unsubscribe",
            SteamAsk::Download => "download",
        }
    }
}

impl App {
    fn mod_folder(&self, id: ModId) -> Result<(PathBuf, String)> {
        let loaded = self.library();
        let (library, _) = loaded.as_ref().as_ref().map_err(Clone::clone)?;
        let m = library.get(id).ok_or_else(|| {
            Error::refused(said!(
                "That mod is no longer where it was read; read the folders again"
            ))
        })?;
        Ok((m.path.clone(), m.display_name().to_string()))
    }

    /// Asks the running Steam about these Workshop items, through this
    /// program started again: Steam shows the game as running for as long
    /// as its library is in use.
    pub fn ask_steam(&self, what: SteamAsk, ids: &[String]) -> Done {
        let request = format!("--steamworks={}:{}", what.word(), ids.join(","));
        // On Linux through /proc: the path of a program rebuilt or
        // updated while it runs no longer leads to it.
        let own = std::path::Path::new("/proc/self/exe");
        let program = match own.exists() {
            true => Ok(own.to_path_buf()),
            false => std::env::current_exe(),
        };
        let ran = program.and_then(|program| {
            rimstack_core::base::program::run(
                std::process::Command::new(program).arg(&request),
                rimstack_core::base::program::NETWORK,
            )
        });
        Done::told(match ran {
            Ok(out) if out.status.success() && what == SteamAsk::Download => {
                Ok(keep!("Steam was asked to update {}", ids.len()))
            }
            Ok(out) if out.status.success() => Ok(keep!("Steam was asked about {}", ids.len())),
            Ok(out) => Err(Error::program(said!(
                "Steam was not asked: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))),
            Err(problem) => Err(Error::program(said!("Steam was not asked: {}", problem))),
        })
    }

    /// Brings a mod kept as a Git checkout up to its remote.
    pub fn update_checkout(&self, id: ModId) -> Done {
        let (folder, name) = match self.mod_folder(id) {
            Ok(found) => found,
            Err(problem) => return Done::told(Err(problem)),
        };
        let updated = git::update(&folder);
        if updated.is_ok() {
            self.checkout_updated(&folder);
        }
        Done::of_a_write(match updated {
            Ok(commit) => Ok(keep!("Updated {}: {}", name, commit)),
            Err(problem) => Err(Error::new(
                problem.kind(),
                said!("Not updated: {}", problem.within()),
            )),
        })
    }

    /// Clones a repository into the game's own `Mods` folder.
    pub fn clone_repository(&self, link: &str) -> Done {
        let Some(mods) = self.paths().local_mods() else {
            return Done::told(Err(Error::refused(said!("The game's folder is not known"))));
        };
        Done::of_a_write(match git::clone(link, &mods) {
            Ok(into) => Ok(keep!("Cloned {} into {}", link, into.display())),
            Err(problem) => Err(Error::new(
                problem.kind(),
                said!("Not cloned: {}", problem.within()),
            )),
        })
    }

    /// Downloads Valve's SteamCMD into RimStack's data folder.
    pub fn install_steamcmd(&self) -> Done {
        Done::told(match redownload::install(self.data_dir()) {
            Ok(_) => {
                self.look_for_steamcmd();
                Ok(keep!("SteamCMD installed"))
            }
            Err(problem) => Err(Error::new(
                problem.kind(),
                said!("SteamCMD not installed: {}", problem.within()),
            )),
        })
    }

    /// Downloads these Workshop items with SteamCMD into the game's own
    /// `Mods` folder.
    pub fn download_with_steamcmd(&self, ids: &[String]) -> Done {
        let steamcmd = (*self.steamcmd()).clone();
        let (steamcmd, local) = match (steamcmd, self.paths().local_mods()) {
            (Some(steamcmd), Some(local)) => (steamcmd, local),
            (None, _) => {
                return Done::told(Err(Error::refused(said!("SteamCMD is not installed"))))
            }
            (_, None) => {
                return Done::told(Err(Error::refused(said!("The game's folder is not known"))))
            }
        };
        let scratch = self.data_dir().join("steamcmd-downloads");
        let fetched = redownload::fetch_all(&steamcmd, ids, &scratch);
        let outcomes: Vec<Result<PathBuf>> = fetched
            .into_iter()
            .map(|(id, fresh)| {
                fresh.and_then(|fresh| redownload::put_in_local(&fresh, &id, &local))
            })
            .collect();
        let failed: Vec<&Error> = outcomes.iter().filter_map(|o| o.as_ref().err()).collect();
        Done {
            wrote: failed.len() < outcomes.len(),
            note: match failed.first() {
                None => Ok(keep!("{} downloaded", outcomes.len())),
                Some(first) => Err(Error::new(
                    first.kind(),
                    said!(
                        "{} of {} not downloaded: {}",
                        failed.len(),
                        outcomes.len(),
                        first.within()
                    ),
                )),
            },
        }
    }

    /// Downloads a Workshop mod again with SteamCMD and puts the fresh
    /// copy in the place of its folder.
    pub fn redownload(&self, id: ModId) -> Done {
        let found = self.mod_folder(id).and_then(|(folder, _)| {
            let workshop = self
                .workshop_id(id)
                .ok_or_else(|| Error::refused(said!("Not a Workshop mod")))?;
            Ok((folder, workshop))
        });
        let scratch = self.data_dir().join("steamcmd");
        let done = found.and_then(|(folder, workshop)| {
            let steamcmd = (*self.steamcmd())
                .clone()
                .ok_or_else(|| Error::refused(said!("SteamCMD is not installed")))?;
            let fresh = redownload::fetch(&steamcmd, &workshop, &scratch)?;
            redownload::replace(&folder, &fresh)
        });
        Done::of_a_write(match done {
            Ok(()) => Ok(keep!("Downloaded again")),
            Err(problem) => Err(Error::new(
                problem.kind(),
                said!("Not downloaded again: {}", problem.within()),
            )),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::workspace::tests::installed;
    use rimstack_core::base::error::Kind;
    use rimstack_core::mods::library::ModId;

    #[test]
    fn a_local_mod_is_not_downloaded_again_and_a_gone_one_is_not_updated() {
        let (_tmp, app) = installed();
        let loaded = app.library();
        let (library, _) = loaded.as_ref().as_ref().unwrap();
        let roads = library.ids().find(|(_, m)| m.folder == "roads").unwrap().0;
        drop(loaded);
        let done = app.redownload(roads);
        assert_eq!(done.note.unwrap_err().kind(), Kind::Refused);
        assert!(!done.wrote);
        let gone = app.update_checkout(ModId::new(0, 99));
        assert_eq!(gone.note.unwrap_err().kind(), Kind::Refused);
        let not_git = app.update_checkout(roads);
        assert!(not_git.note.is_err() && !not_git.wrote);
    }
}
