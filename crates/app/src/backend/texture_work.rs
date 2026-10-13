//! Textures: converting them, auditing them, and choosing between two
//! mods' versions of one.

use super::*;

impl qobject::Backend {
    pub(super) fn skip_textures(mut self: Pin<&mut Self>, mod_id: i32, skip: bool) {
        let Ok((library, _)) = crate::library() else {
            return;
        };
        let key = usize::try_from(mod_id)
            .ok()
            .and_then(|i| library.installed.get(i)?.key());
        let Some(key) = key else {
            return;
        };
        let mut skipped = skipped_mods();
        if skip {
            skipped.insert(key);
        } else {
            skipped.remove(&key);
        }
        let mut kept: Vec<String> = skipped.into_iter().collect();
        kept.sort();
        if let Err(problem) = crate::set_setting("textureSkip", &kept.join(",")) {
            eprintln!("{problem}");
        }
        self.as_mut().load_textures();
    }

    pub(super) fn load_textures(mut self: Pin<&mut Self>) {
        if self.textures_busy {
            return;
        }
        self.as_mut().set_textures_busy(true);
        self.as_mut().texture_work(Vec::new(), false);
    }

    pub(super) fn convert_textures(mut self: Pin<&mut Self>, mod_ids: &QString) {
        if self.textures_busy {
            return;
        }
        self.as_mut().set_textures_busy(true);
        self.as_mut().texture_work(ids_from(mod_ids), false);
    }

    pub(super) fn remove_textures(mut self: Pin<&mut Self>, mod_ids: &QString) {
        if self.textures_busy {
            return;
        }
        self.as_mut().set_textures_busy(true);
        self.as_mut().texture_work(ids_from(mod_ids), true);
    }

    /// Converts (or, with `remove`, un-converts) the given mods on a
    /// background thread, then counts everything again.
    pub(super) fn texture_work(self: Pin<&mut Self>, mods: Vec<usize>, remove: bool) {
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let Ok((library, _)) = crate::library() else {
                let _ = qt_thread.queue(|mut b| b.as_mut().set_textures_busy(false));
                return;
            };
            let roots: Vec<&std::path::Path> = mods
                .iter()
                .filter_map(|&i| library.installed.get(i))
                .map(|m| m.path.as_path())
                .collect();
            let mut failed = 0;
            if !roots.is_empty() {
                let file = manifest_file();
                let mut manifest = textures::Manifest::read(&file);
                let total: usize = roots
                    .iter()
                    .map(|root| {
                        let s = textures::summarize(root, &manifest);
                        s.not_converted() + s.out_of_date
                    })
                    .sum();
                let done = AtomicUsize::new(0);
                let say = |text: String| {
                    let _ = qt_thread.queue(move |mut b| {
                        b.as_mut().set_textures_progress(QString::from(&text));
                    });
                };
                history::STOP_TEXTURES.store(false, Ordering::Relaxed);
                for root in roots {
                    if history::STOP_TEXTURES.load(Ordering::Relaxed) {
                        break;
                    }
                    if remove {
                        textures::remove(root, &mut manifest);
                    } else {
                        let outcome = textures::convert(root, &mut manifest, &|| {
                            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                            if n % 20 == 0 || n == total {
                                say(say!("Converted {} of {} textures", n, total));
                            }
                        });
                        failed += outcome.failed.len();
                        for (png, reason) in outcome.failed {
                            eprintln!("{}: {reason}", png.display());
                        }
                    }
                    // After every mod, so a crash loses track of little.
                    if let Err(problem) = manifest.write(&file) {
                        eprintln!("{problem}");
                    }
                }
            }
            let text = textures_report(library).to_string();
            let note = match failed {
                0 => String::new(),
                1 => say!("1 texture could not be converted; it keeps using its PNG."),
                n => say!(
                    "{} textures could not be converted; they keep using their PNG.",
                    n
                ),
            };
            let _ = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_textures(QString::from(&text));
                backend.as_mut().set_textures_progress(QString::from(&note));
                backend.as_mut().set_textures_busy(false);
            });
        });
    }

    pub(super) fn dds_audit(&self) -> QString {
        use rayon::prelude::*;
        let Ok((library, _)) = crate::library() else {
            return QString::from("[]");
        };
        let found: Vec<Value> = library
            .installed
            .par_iter()
            .flat_map_iter(|m| {
                let bad = rimstack_core::ddsaudit::audit(&m.path).into_iter();
                bad.map(|b| json!({ "mod": m.display_name(), "file": b.file.to_string_lossy(), "why": b.why }))
            })
            .collect();
        QString::from(&json!(found).to_string())
    }

    pub(super) fn set_dds_aside(&self, files: &QString) -> i32 {
        let files: Vec<String> = serde_json::from_str(&files.to_string()).unwrap_or_default();
        let aside = |file: &&String| {
            rimstack_core::ddsaudit::set_aside(std::path::Path::new(file)).is_err()
        };
        files.iter().filter(aside).count() as i32
    }

    pub(super) fn compare(&self, earlier: i32, later: i32) -> QString {
        let Ok((library, _)) = crate::library() else {
            return QString::default();
        };
        let (Ok(a), Ok(b)) = (usize::try_from(earlier), usize::try_from(later)) else {
            return QString::default();
        };
        if a >= library.installed.len() || b >= library.installed.len() {
            return QString::default();
        }
        let first = conflicts::files_with_hidden(&loaded_folders(library, a));
        let second = conflicts::files_with_hidden(&loaded_folders(library, b));
        // The file to show: a hidden one sits under its renamed name.
        let on_disk = |(path, hidden): &(PathBuf, bool)| match hidden {
            true => format!(
                "{}{}",
                path.to_string_lossy(),
                rimstack_core::hidden::SUFFIX
            ),
            false => path.to_string_lossy().into_owned(),
        };
        let size = |file: &str| std::fs::metadata(file).map_or(0, |m| m.len());
        let textures: Vec<Value> = first
            .iter()
            .filter_map(|(name, one)| {
                let other = second.get(name)?;
                Some(json!({
                    "name": name,
                    "earlier": one.0.to_string_lossy(),
                    "later": other.0.to_string_lossy(),
                                        "earlierShown": on_disk(one),
                    "laterShown": on_disk(other),
                    "earlierBytes": size(&on_disk(one)),
                    "laterBytes": size(&on_disk(other)),
                    // The later mod wins unless its file is hidden.
                    "keepsEarlier": other.1 && !one.1,
                    "yourChoice": one.1 || other.1,
                }))
            })
            .collect();
        let report = json!({
            "earlier": { "id": a, "name": library.installed[a].display_name() },
            "later": { "id": b, "name": library.installed[b].display_name() },
            "textures": textures,
        });
        QString::from(&report.to_string())
    }

    pub(super) fn keep_texture(
        &self,
        earlier_file: &QString,
        later_file: &QString,
        keep_earlier: bool,
    ) -> QString {
        let file = hidden_file();
        let mut hidden = Hidden::read(&file);
        let earlier = PathBuf::from(earlier_file.to_string());
        let later = PathBuf::from(later_file.to_string());
        let done = hidden
            .unhide(&earlier)
            .and_then(|()| hidden.unhide(&later))
            .and_then(|()| {
                if keep_earlier {
                    hidden.hide(&later)
                } else {
                    Ok(())
                }
            })
            .and_then(|()| hidden.write(&file));
        match done {
            Ok(()) => {
                *crate::CONFLICTS.write().unwrap_or_else(|e| e.into_inner()) = None;
                QString::default()
            }
            Err(problem) => QString::from(&problem),
        }
    }

    pub(super) fn hidden_in(&self, mod_id: i32) -> i32 {
        let Ok((library, _)) = crate::library() else {
            return 0;
        };
        let Some(m) = usize::try_from(mod_id)
            .ok()
            .and_then(|i| library.installed.get(i))
        else {
            return 0;
        };
        let hidden = Hidden::read(&hidden_file());
        let pngs = hidden
            .in_mod(&m.path)
            .into_iter()
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")));
        pngs.count() as i32
    }

    pub(super) fn unhide_all(&self, mod_id: i32) -> QString {
        let Ok((library, _)) = crate::library() else {
            return QString::default();
        };
        let Some(m) = usize::try_from(mod_id)
            .ok()
            .and_then(|i| library.installed.get(i))
        else {
            return QString::default();
        };
        let file = hidden_file();
        let mut hidden = Hidden::read(&file);
        let done = hidden.unhide_all(&m.path).and_then(|_| hidden.write(&file));
        *crate::CONFLICTS.write().unwrap_or_else(|e| e.into_inner()) = None;
        match done {
            Ok(()) => QString::default(),
            Err(problem) => QString::from(&problem),
        }
    }

    /// Writes the picture of where two PNG files differ into RimStack's
    /// data folder and says where: `{ file, differing, total }`, or
    /// `{ problem }`.
    pub(super) fn texture_difference(&self, earlier: &QString, later: &QString) -> QString {
        use std::hash::{Hash, Hasher};
        let (earlier, later) = (earlier.to_string(), later.to_string());
        let dir = crate::data_dir().join("cache");
        // One picture at a time is on show; the last one's file goes.
        for old in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            if old.file_name().to_string_lossy().starts_with("difference-") {
                let _ = std::fs::remove_file(old.path());
            }
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (&earlier, &later).hash(&mut hasher);
        let file = dir.join(format!("difference-{:016x}.png", hasher.finish()));
        let (from, to) = (PathBuf::from(&earlier), PathBuf::from(&later));
        let said = match rimstack_core::texdiff::write(&from, &to, &file) {
            Ok((differing, total)) => {
                json!({ "file": file.to_string_lossy(), "differing": differing, "total": total })
            }
            Err(problem) => json!({ "problem": problem }),
        };
        QString::from(&said.to_string())
    }
}
