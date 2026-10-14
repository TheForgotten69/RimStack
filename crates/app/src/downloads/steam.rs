//! Steam, the Workshop, SteamCMD and Git: what is asked of them and
//! what comes back.

use super::*;

impl qobject::Downloads {
    pub(super) fn check_updates(mut self: Pin<&mut Self>) {
        if self.updates_busy {
            return;
        }
        self.as_mut().set_updates_busy(true);
        let asked = || match &*crate::app().library() {
            Ok((library, _)) => {
                let (kept, problem) = check(crate::app(), library);
                let mut report = updates_report(crate::app(), library, &kept);
                if let Some(problem) = problem {
                    report["problem"] = json!(problem);
                }
                report.to_string()
            }
            Err(problem) => json!({ "problem": problem }).to_string(),
        };
        crate::background::run(self.qt_thread(), asked, |mut downloads, report| {
            downloads.as_mut().set_updates(QString::from(&report));
            downloads.as_mut().set_updates_busy(false);
        });
    }

    pub(super) fn restore_updates(self: Pin<&mut Self>) {
        if self.updates_busy {
            return;
        }
        let kept_report = || {
            let kept = last_check(crate::app());
            let loaded = crate::app().library();
            let (library, _) = loaded.as_ref().as_ref().ok()?;
            let checked_before = kept.as_object().is_some_and(|kept| !kept.is_empty());
            checked_before.then(|| updates_report(crate::app(), library, &kept).to_string())
        };
        crate::background::run(self.qt_thread(), kept_report, |mut downloads, report| {
            // A check started meanwhile is about to say more.
            if let Some(report) = report.filter(|_| !downloads.updates_busy) {
                downloads.as_mut().set_updates(QString::from(&report));
            }
        });
    }

    pub(super) fn look_up(mut self: Pin<&mut Self>, pasted: &QString) {
        if self.queue_busy {
            return;
        }
        self.as_mut().set_queue_busy(true);
        let pasted = pasted.to_string();
        let looked_up = move || {
            let report = match &*crate::app().library() {
                Ok((library, _)) => queue_report(crate::app(), library, &pasted),
                Err(problem) => Err(problem.clone()),
            };
            let report = report.unwrap_or_else(|problem| json!({ "problem": problem }));
            report.to_string()
        };
        crate::background::run(self.qt_thread(), looked_up, |mut downloads, report| {
            downloads.as_mut().set_queue(QString::from(&report));
            downloads.as_mut().set_queue_busy(false);
        });
    }

    pub(super) fn update_from_git(self: Pin<&mut Self>, folder: &QString) {
        let folder = PathBuf::from(folder.to_string());
        self.task(&say!("Updating from Git…"), move || {
            let loaded = crate::app().library();
            let library = loaded.as_ref().as_ref().ok().map(|(library, _)| library);
            let name = library.and_then(|library| {
                let found = library.installed.iter().find(|m| m.path == folder);
                found.map(|m| m.display_name().to_string())
            });
            let updated = rimstack_core::git::update(&folder);
            if updated.is_ok() {
                checkout_updated(crate::app(), &folder);
            }
            Ended {
                wrote: updated.is_ok(),
                updates: library.map(|library| {
                    updates_report(crate::app(), library, &last_check(crate::app())).to_string()
                }),
                note: match updated {
                    Ok(commit) => match name {
                        Some(name) => Ok(keep!("Updated {}: {}", name, commit)),
                        None => Ok(keep!("Now at {}", commit)),
                    },
                    Err(problem) => Err(say!("Not updated: {}", problem)),
                },
            }
        });
    }

    pub(super) fn ask_steam(self: Pin<&mut Self>, what: &QString, ids: &QString) {
        let ids: Vec<String> = serde_json::from_str(&ids.to_string()).unwrap_or_default();
        let request = format!("--steamworks={what}:{}", ids.join(","));
        self.task(&say!("Asking Steam…"), move || {
            // A process of its own: Steam shows the game as running for
            // as long as its library is in use.
            // On Linux through /proc: the path of a program rebuilt or
            // updated while it runs no longer leads to it.
            let own = std::path::Path::new("/proc/self/exe");
            let program = match own.exists() {
                true => Ok(own.to_path_buf()),
                false => std::env::current_exe(),
            };
            let ran = program.and_then(|program| {
                rimstack_core::program::run(
                    std::process::Command::new(program).arg(&request),
                    rimstack_core::program::NETWORK,
                )
            });
            Ended::told(match ran {
                Ok(out) if out.status.success() && request.contains("=download:") => {
                    Ok(keep!("Steam was asked to update {}", ids.len()))
                }
                Ok(out) if out.status.success() => Ok(keep!("Steam was asked about {}", ids.len())),
                Ok(out) => Err(say!(
                    "Steam was not asked: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                )),
                Err(problem) => Err(say!("Steam was not asked: {}", problem)),
            })
        });
    }

    pub(super) fn clone_mod(self: Pin<&mut Self>, link: &QString) {
        let link = link.to_string();
        let mods = crate::app().paths().local_mods();
        self.task(&say!("Cloning {}…", link), move || {
            let Some(mods) = mods else {
                return Ended::told(Err(say!("The game's folder is not known")));
            };
            Ended::of_a_write(match rimstack_core::git::clone(&link, &mods) {
                Ok(into) => Ok(keep!("Cloned {} into {}", link, into.display())),
                Err(problem) => Err(say!("Not cloned: {}", problem)),
            })
        });
    }

    pub(super) fn install_steamcmd(self: Pin<&mut Self>) {
        self.task(&say!("Installing SteamCMD…"), || {
            Ended::told(
                match rimstack_core::redownload::install(crate::app().data_dir()) {
                    Ok(_) => Ok(keep!("SteamCMD installed")),
                    Err(problem) => Err(say!("SteamCMD not installed: {}", problem)),
                },
            )
        });
    }

    pub(super) fn download_with_steamcmd(self: Pin<&mut Self>, ids: &QString) {
        let ids: Vec<String> = serde_json::from_str(&ids.to_string()).unwrap_or_default();
        let doing = say!("Downloading {} with SteamCMD…", ids.len());
        self.task(&doing, move || {
            use rimstack_core::redownload;
            let data = crate::app().data_dir();
            let local = crate::app().paths().local_mods();
            let (steamcmd, local) = match (redownload::steamcmd(data), local) {
                (Some(steamcmd), Some(local)) => (steamcmd, local),
                (None, _) => return Ended::told(Err(say!("SteamCMD is not installed"))),
                (_, None) => return Ended::told(Err(say!("The game's folder is not known"))),
            };
            let fetched = redownload::fetch_all(&steamcmd, &ids, &data.join("steamcmd-downloads"));
            let placed = fetched.into_iter().map(|(id, fresh)| {
                fresh.and_then(|fresh| redownload::put_in_local(&fresh, &id, &local))
            });
            let outcomes: Vec<rimstack_core::error::Result<PathBuf>> = placed.collect();
            let failed: Vec<_> = outcomes.iter().filter_map(|o| o.as_ref().err()).collect();
            Ended {
                wrote: failed.len() < outcomes.len(),
                updates: None,
                note: match failed.first() {
                    None => Ok(keep!("{} downloaded", outcomes.len())),
                    Some(first) => Err(say!(
                        "{} of {} not downloaded: {}",
                        failed.len(),
                        outcomes.len(),
                        first
                    )),
                },
            }
        });
    }

    pub(super) fn workshop_id_of(&self, name: &QString) -> QString {
        let name = name.to_string();
        let names = crate::app().workshop_names().reading();
        let by_id = names.get(&rimstack_core::id::fold(&name));
        let found = by_id.or_else(|| names.values().find(|item| item.name == name));
        QString::from(found.map_or("", |item| item.workshop_id.as_str()))
    }

    pub(super) fn redownload(self: Pin<&mut Self>, workshop_id: &QString, folder: &QString) {
        let (id, folder) = (workshop_id.to_string(), PathBuf::from(folder.to_string()));
        self.task(&say!("Downloading again with SteamCMD…"), move || {
            use rimstack_core::redownload;
            let scratch = crate::app().data_dir().join("steamcmd");
            let done = redownload::steamcmd(crate::app().data_dir())
                .ok_or_else(|| {
                    rimstack_core::error::Error::refused(say!("SteamCMD is not installed"))
                })
                .and_then(|steamcmd| redownload::fetch(&steamcmd, &id, &scratch))
                .and_then(|fresh| redownload::replace(&folder, &fresh));
            Ended::of_a_write(match done {
                Ok(()) => Ok(keep!("Downloaded again")),
                Err(problem) => Err(say!("Not downloaded again: {}", problem)),
            })
        });
    }

    pub(super) fn replacements(&self) -> QString {
        let loaded = crate::app().library();
        let Ok((library, _)) = &*loaded else {
            return QString::from("[]");
        };
        let active = crate::app().active_now(library);
        let items = rimstack_core::downloads::items(library, &active);
        let rows = rimstack_core::downloads::replaceable(
            &items,
            crate::app().replacements(),
            &crate::app().replacements_ignored(),
        );
        QString::from(&json!(rows).to_string())
    }
}
