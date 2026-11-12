//! Every thread RimStack starts goes through here, named for the diagnostics.

use futures::channel::oneshot;
use gpui::{AsyncApp, Context, WeakEntity};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub fn spawn(name: &str, work: impl FnOnce() + Send + 'static) {
    let started = std::thread::Builder::new()
        .name(name.to_string())
        .spawn(work);
    if let Err(problem) = started {
        rimstack_core::problem!("{name} not started: {problem}");
    }
}

/// `work` runs off the window's thread; a view gone by then is not told.
pub fn run<V: 'static, R: Send + 'static>(
    name: &str,
    cx: &mut Context<V>,
    work: impl FnOnce() -> R + Send + 'static,
    done: impl FnOnce(&mut V, R, &mut Context<V>) + 'static,
) {
    let (found, waiting) = oneshot::channel();
    spawn(name, move || {
        let _ = found.send(work());
    });
    cx.spawn(async move |view: WeakEntity<V>, cx: &mut AsyncApp| {
        if let Ok(found) = waiting.await {
            let _ = view.update(cx, |view, cx| done(view, found, cx));
        }
    })
    .detach();
}

#[derive(Default)]
pub struct Lane {
    asked: Arc<AtomicU64>,
}

#[derive(Clone)]
pub struct Turn {
    asked: Arc<AtomicU64>,
    mine: u64,
}

impl Turn {
    pub fn current(&self) -> bool {
        self.asked.load(Ordering::SeqCst) == self.mine
    }

    /// Waits, then says whether nothing newer was asked meanwhile: the pause of a field that applies itself.
    pub fn after(&self, wait: Duration) -> bool {
        std::thread::sleep(wait);
        self.current()
    }
}

impl Lane {
    fn next(&self) -> Turn {
        Turn {
            asked: self.asked.clone(),
            mine: self.asked.fetch_add(1, Ordering::SeqCst) + 1,
        }
    }

    pub fn cancel(&self) {
        self.asked.fetch_add(1, Ordering::SeqCst);
    }

    /// `done` is not called once the lane was asked again; `work` may stop early.
    pub fn run<V: 'static, R: Send + 'static>(
        &self,
        name: &str,
        cx: &mut Context<V>,
        work: impl FnOnce(&Turn) -> R + Send + 'static,
        done: impl FnOnce(&mut V, R, &mut Context<V>) + 'static,
    ) {
        let turn = self.next();
        let mine = turn.clone();
        // Checked on the window's thread: another run may be asked for between here and there.
        run(
            name,
            cx,
            move || work(&mine),
            move |view, found, cx| {
                if turn.current() {
                    done(view, found, cx);
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_newest_turn_of_a_lane_is_current() {
        let lane = Lane::default();
        let first = lane.next();
        assert!(first.current());
        let second = lane.next();
        assert!(!first.current() && second.current());
        lane.cancel();
        assert!(!second.current());
    }
}
