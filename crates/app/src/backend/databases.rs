//! What is fetched from GitHub on request: the community databases, and
//! whether a newer RimStack is published.

use super::*;

impl qobject::Backend {
    pub(super) fn check_release(mut self: Pin<&mut Self>) {
        if self.release_busy {
            return;
        }
        self.as_mut().set_release_busy(true);
        let asked = || {
            let running = env!("CARGO_PKG_VERSION");
            let found = match rimstack_core::release::latest() {
                Ok(Some(release)) if rimstack_core::release::newer(running, &release.version) => {
                    json!({ "version": release.version, "page": release.page })
                }
                Ok(_) => json!({}),
                Err(problem) => json!({ "problem": problem }),
            };
            found.to_string()
        };
        crate::background::run(self.qt_thread(), asked, |mut backend, found| {
            backend.as_mut().set_release(QString::from(&found));
            backend.as_mut().set_release_busy(false);
        });
    }
}
