//! Steam, the Workshop, SteamCMD and Git: what is asked of them and
//! what comes back.

use super::*;

impl qobject::Backend {
    pub(super) fn check_updates(mut self: Pin<&mut Self>) {
        if self.updates_busy {
            return;
        }
        self.as_mut().set_updates_busy(true);
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let report = match crate::library() {
                Ok((library, _)) => {
                    let (kept, problem) = check(library);
                    let mut report = updates_report(library, &kept);
                    if let Some(problem) = problem {
                        report["problem"] = json!(problem);
                    }
                    report
                }
                Err(problem) => json!({ "problem": problem }),
            };
            let text = report.to_string();
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_updates(QString::from(&text));
                backend.as_mut().set_updates_busy(false);
            });
        });
    }

    pub(super) fn restore_updates(self: Pin<&mut Self>) {
        if self.updates_busy {
            return;
        }
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let kept = last_check();
            let Ok((library, _)) = crate::library() else {
                return;
            };
            if kept.as_object().is_some_and(|k| k.is_empty()) {
                return;
            }
            let text = updates_report(library, &kept).to_string();
            let _ = qt_thread.queue(move |mut backend| {
                if !backend.updates_busy {
                    backend.as_mut().set_updates(QString::from(&text));
                }
            });
        });
    }

    pub(super) fn look_up(mut self: Pin<&mut Self>, pasted: &QString) {
        if self.queue_busy {
            return;
        }
        self.as_mut().set_queue_busy(true);
        let pasted = pasted.to_string();
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let report = match crate::library() {
                Ok((library, _)) => queue_report(library, &pasted),
                Err(problem) => Err(problem.clone()),
            };
            let text = report
                .unwrap_or_else(|problem| json!({ "problem": problem }))
                .to_string();
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_queue(QString::from(&text));
                backend.as_mut().set_queue_busy(false);
            });
        });
    }

    pub(super) fn update_from_git(mut self: Pin<&mut Self>, folder: &QString) {
        let folder = PathBuf::from(folder.to_string());
        self.as_mut()
            .set_task_note(QString::from(&say!("Updating from Git…")));
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let name = crate::library().as_ref().ok().and_then(|(library, _)| {
                let found = library.installed.iter().find(|m| m.path == folder);
                found.map(|m| m.display_name().to_string())
            });
            let updated = rimstack_core::git::update(&folder);
            let wrote = updated.is_ok();
            if wrote {
                checkout_updated(&folder);
            }
            let report = crate::library()
                .as_ref()
                .ok()
                .map(|(library, _)| updates_report(library, &last_check()).to_string());
            let note = match updated {
                Ok(commit) => match name {
                    Some(name) => keep!("Updated {}: {}", name, commit),
                    None => keep!("Now at {}", commit),
                },
                Err(problem) => format!("!{}", say!("Not updated: {}", problem)),
            };
            let _ = qt_thread.queue(move |mut backend| {
                if let Some(report) = report {
                    backend.as_mut().set_updates(QString::from(&report));
                }
                if wrote {
                    let written = *backend.mods_written() + 1;
                    backend.as_mut().set_mods_written(written);
                }
                backend.as_mut().set_task_note(QString::from(&note));
            });
        });
    }

    pub(super) fn start_game_directly(&self) -> QString {
        let Some(game) = crate::PATHS.game.as_ref() else {
            return QString::from(&say!("The game's folder is not known"));
        };
        let arguments = crate::settings()
            .get("launchArguments")
            .unwrap_or_default()
            .to_string();
        match rimstack_core::upkeep::start_game(game, &arguments) {
            Ok(()) => QString::default(),
            Err(problem) => QString::from(&problem),
        }
    }

    pub(super) fn ask_steam(mut self: Pin<&mut Self>, what: &QString, ids: &QString) {
        let ids: Vec<String> = serde_json::from_str(&ids.to_string()).unwrap_or_default();
        let request = format!("--steamworks={what}:{}", ids.join(","));
        self.as_mut()
            .set_task_note(QString::from(&say!("Asking Steam…")));
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            // A process of its own: Steam shows the game as running for
            // as long as its library is in use.
            // On Linux through /proc: the path of a program rebuilt or
            // updated while it runs no longer leads to it.
            let own = std::path::Path::new("/proc/self/exe");
            let program = match own.exists() {
                true => Ok(own.to_path_buf()),
                false => std::env::current_exe(),
            };
            let ran = program
                .and_then(|program| std::process::Command::new(program).arg(&request).output());
            let note = match ran {
                Ok(out) if out.status.success() && request.contains("=download:") => {
                    keep!("Steam was asked to update {}", ids.len())
                }
                Ok(out) if out.status.success() => keep!("Steam was asked about {}", ids.len()),
                Ok(out) => format!(
                    "!{}",
                    say!(
                        "Steam was not asked: {}",
                        String::from_utf8_lossy(&out.stderr).trim()
                    )
                ),
                Err(problem) => format!("!{}", say!("Steam was not asked: {}", problem)),
            };
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_task_note(QString::from(&note));
            });
        });
    }

    pub(super) fn clone_mod(mut self: Pin<&mut Self>, link: &QString) {
        let link = link.to_string();
        let Some(mods) = crate::PATHS.game.as_ref().map(|game| game.join("Mods")) else {
            let note = format!("!{}", say!("The game's folder is not known"));
            self.as_mut().set_task_note(QString::from(&note));
            return;
        };
        self.as_mut()
            .set_task_note(QString::from(&say!("Cloning {}…", link)));
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let cloned = rimstack_core::git::clone(&link, &mods);
            let wrote = cloned.is_ok();
            let note = match cloned {
                Ok(into) => keep!("Cloned {} into {}", link, into.display()),
                Err(problem) => format!("!{}", say!("Not cloned: {}", problem)),
            };
            let _ = qt_thread.queue(move |mut backend| {
                if wrote {
                    let written = *backend.mods_written() + 1;
                    backend.as_mut().set_mods_written(written);
                }
                backend.as_mut().set_task_note(QString::from(&note));
            });
        });
    }

    pub(super) fn install_steamcmd(mut self: Pin<&mut Self>) {
        self.as_mut()
            .set_task_note(QString::from(&say!("Installing SteamCMD…")));
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let note = match rimstack_core::redownload::install(&crate::data_dir()) {
                Ok(_) => keep!("SteamCMD installed"),
                Err(problem) => format!("!{}", say!("SteamCMD not installed: {}", problem)),
            };
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_task_note(QString::from(&note));
            });
        });
    }

    pub(super) fn download_with_steamcmd(mut self: Pin<&mut Self>, ids: &QString) {
        let ids: Vec<String> = serde_json::from_str(&ids.to_string()).unwrap_or_default();
        let note = say!("Downloading {} with SteamCMD…", ids.len());
        self.as_mut().set_task_note(QString::from(&note));
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            use rimstack_core::redownload;
            let data = crate::data_dir();
            let local = crate::PATHS.game.as_ref().map(|game| game.join("Mods"));
            let (note, wrote) = match (redownload::steamcmd(&data), local) {
                (Some(steamcmd), Some(local)) => {
                    let fetched =
                        redownload::fetch_all(&steamcmd, &ids, &data.join("steamcmd-downloads"));
                    let placed = fetched.into_iter().map(|(id, fresh)| {
                        fresh.and_then(|fresh| redownload::put_in_local(&fresh, &id, &local))
                    });
                    let outcomes: Vec<Result<PathBuf, String>> = placed.collect();
                    let failed: Vec<&String> =
                        outcomes.iter().filter_map(|o| o.as_ref().err()).collect();
                    let note = match failed.first() {
                        None => keep!("{} downloaded", outcomes.len()),
                        Some(first) => format!(
                            "!{}",
                            say!(
                                "{} of {} not downloaded: {}",
                                failed.len(),
                                outcomes.len(),
                                first
                            )
                        ),
                    };
                    (note, failed.len() < outcomes.len())
                }
                (None, _) => (format!("!{}", say!("SteamCMD is not installed")), false),
                (_, None) => (
                    format!("!{}", say!("The game's folder is not known")),
                    false,
                ),
            };
            let _ = qt_thread.queue(move |mut backend| {
                if wrote {
                    let written = *backend.mods_written() + 1;
                    backend.as_mut().set_mods_written(written);
                }
                backend.as_mut().set_task_note(QString::from(&note));
            });
        });
    }

    pub(super) fn workshop_id_of(&self, name: &QString) -> QString {
        let name = name.to_string();
        let names = crate::WORKSHOP_NAMES
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let by_id = names.get(&rimstack_core::id::fold(&name));
        let found = by_id.or_else(|| names.values().find(|item| item.name == name));
        QString::from(found.map_or("", |item| item.workshop_id.as_str()))
    }

    pub(super) fn redownload(mut self: Pin<&mut Self>, workshop_id: &QString, folder: &QString) {
        let (id, folder) = (workshop_id.to_string(), PathBuf::from(folder.to_string()));
        self.as_mut()
            .set_task_note(QString::from(&say!("Downloading again with SteamCMD…")));
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            use rimstack_core::redownload;
            let scratch = crate::data_dir().join("steamcmd");
            let done = redownload::steamcmd(&crate::data_dir())
                .ok_or(say!("SteamCMD is not installed"))
                .and_then(|steamcmd| redownload::fetch(&steamcmd, &id, &scratch))
                .and_then(|fresh| redownload::replace(&folder, &fresh));
            let wrote = done.is_ok();
            let note = match done {
                Ok(()) => keep!("Downloaded again"),
                Err(problem) => format!("!{}", say!("Not downloaded again: {}", problem)),
            };
            let _ = qt_thread.queue(move |mut backend| {
                if wrote {
                    let written = *backend.mods_written() + 1;
                    backend.as_mut().set_mods_written(written);
                }
                backend.as_mut().set_task_note(QString::from(&note));
            });
        });
    }

    pub(super) fn replacements(&self) -> QString {
        let Ok((library, _)) = crate::library() else {
            return QString::from("[]");
        };
        let active: std::collections::HashSet<usize> = {
            let list = crate::session().current().to_vec();
            let arranged = library.arrange(&list);
            arranged.active.iter().filter_map(|m| m.installed).collect()
        };
        let installed = workshop_mods(library);
        let ignored = crate::replacements_ignored();
        let mut rows: Vec<Value> = installed
            .iter()
            .filter(|(id, _)| !ignored.contains(**id))
            .filter_map(|(id, &i)| {
                let new = crate::REPLACEMENTS.get(*id)?;
                Some(json!({
                    "id": id,
                    "modId": i,
                    "name": library.installed[i].display_name(),
                    "active": active.contains(&i),
                    "newName": new.name,
                    "newId": new.workshop_id,
                    "newVersions": new.versions.join(", "),
                    "haveNew": installed.contains_key(new.workshop_id.as_str()),
                }))
            })
            .collect();
        let key = |v: &Value| {
            (
                !v["active"].as_bool().unwrap_or(false),
                v["name"].as_str().unwrap_or("").to_lowercase(),
            )
        };
        rows.sort_by_key(key);
        QString::from(&Value::Array(rows).to_string())
    }
}
