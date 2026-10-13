//! What is fetched from GitHub on request: the community databases, and
//! whether a newer RimStack is published.

use super::*;
use rimstack_core::tongue::named;
use rimstack_core::{community, replacements, workshopdb};

impl qobject::Backend {
    pub(super) fn check_release(mut self: Pin<&mut Self>) {
        if self.release_busy {
            return;
        }
        self.as_mut().set_release_busy(true);
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let running = env!("CARGO_PKG_VERSION");
            let found = match rimstack_core::release::latest() {
                Ok(Some(release)) if rimstack_core::release::newer(running, &release.version) => {
                    json!({ "version": release.version, "page": release.page })
                }
                Ok(_) => json!({}),
                Err(problem) => json!({ "problem": problem }),
            };
            let _ = qt_thread.queue(move |mut backend| {
                backend
                    .as_mut()
                    .set_release(QString::from(&found.to_string()));
                backend.as_mut().set_release_busy(false);
            });
        });
    }

    /// Each database as `{ id, name, use, updated, on }`: `updated` in
    /// seconds since 1970, 0 for one not downloaded; `on` unless the
    /// settings leave it out.
    pub(super) fn databases(&self) -> QString {
        let data = crate::data_dir();
        let version = crate::PATHS.game_version.clone().unwrap_or_default();
        let all = [
            (
                "rules",
                "Community Rules Database",
                named("Load-order rules, for Sort"),
                community::file_in(&data),
            ),
            (
                "worksOn",
                "No Version Warning",
                named("Mods that work on this game version without saying so"),
                community::works_on_file(&data, &version),
            ),
            (
                "replacements",
                "Use This Instead",
                named("Maintained replacements for abandoned mods"),
                replacements::file_in(&data),
            ),
            (
                "names",
                "Steam Workshop Database",
                named("Names of mods that are not installed"),
                workshopdb::file_in(&data),
            ),
        ];
        let rows: Vec<Value> = all
            .iter()
            .map(|(id, name, used, file)| {
                let updated = rimstack_core::watch::changed_at(file) / 1000;
                json!({ "id": id, "name": name, "use": used, "updated": updated, "on": crate::database_on(id) })
            })
            .collect();
        QString::from(&json!(rows).to_string())
    }
}
