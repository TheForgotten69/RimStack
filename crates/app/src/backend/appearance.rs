//! What changed on disk while the window was open, and how the last
//! start of the game went.

use super::*;

impl qobject::Backend {
    pub(super) fn look(mut self: Pin<&mut Self>, game_running: bool, in_the_way: bool) -> QString {
        let now = crate::app().stamps();
        let noticed = self
            .as_mut()
            .rust_mut()
            .watcher
            .look(now, game_running, in_the_way);
        crate::reports::said(&noticed)
    }

    pub(super) fn last_start(&self, log_written: i64, previous: bool, errors: i32) -> QString {
        let app = crate::app();
        let recorded = launches::read(&launches_file(app)).pop();
        let read = u64::try_from(log_written)
            .ok()
            .map(|written| launches::LogRead {
                written,
                previous,
                errors: usize::try_from(errors).unwrap_or(0),
            });
        let written = app.log_written() / 1000;
        crate::reports::said(&launches::last_start(recorded.as_ref(), read, written))
    }
}
