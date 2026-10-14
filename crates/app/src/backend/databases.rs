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
            use crate::reports::ReleaseSaid;
            let found = match rimstack_core::sources::release::latest() {
                Ok(Some(release))
                    if rimstack_core::sources::release::newer(running, &release.version) =>
                {
                    ReleaseSaid::Newer {
                        version: release.version,
                        page: release.page,
                    }
                }
                Ok(_) => ReleaseSaid::Latest {},
                Err(problem) => ReleaseSaid::Failed {
                    problem: problem.into(),
                },
            };
            crate::reports::said(&found).to_string()
        };
        crate::background::run(self.qt_thread(), asked, |mut backend, found| {
            backend.as_mut().set_release(QString::from(&found));
            backend.as_mut().set_release_busy(false);
        });
    }
}
