//! Work done off the window's thread. Every thread RimStack starts is
//! started here, under a name the diagnostics and a debugger show.

use core::pin::Pin;
use cxx_qt::{CxxQtThread, Threading};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Runs `work` on a thread of its own.
pub(crate) fn spawn(name: &str, work: impl FnOnce() + Send + 'static) {
    let started = std::thread::Builder::new()
        .name(name.to_string())
        .spawn(work);
    if let Err(problem) = started {
        rimstack_core::problem!("{name} not started: {problem}");
    }
}

/// Runs `work` on a thread of its own, then `done` with its result on the
/// thread of the object `thread` was taken from. An object that is gone
/// by then is not told.
pub(crate) fn run<T, R>(
    name: &str,
    thread: CxxQtThread<T>,
    work: impl FnOnce() -> R + Send + 'static,
    done: impl FnOnce(Pin<&mut T>, R) + Send + 'static,
) where
    T: Threading + 'static,
    R: Send + 'static,
{
    spawn(name, move || {
        let found = work();
        if let Err(problem) = thread.queue(move |object| done(object, found)) {
            rimstack_core::problem!("a finished task found nothing to tell: {problem:?}");
        }
    });
}

/// One kind of work of which only the newest counts: a result that comes
/// back after the same work was asked for again is dropped.
#[derive(Default)]
pub(crate) struct Lane {
    asked: Arc<AtomicU64>,
}

/// Given to the work of a lane: whether it is still the newest asked.
#[derive(Clone)]
pub(crate) struct Turn {
    asked: Arc<AtomicU64>,
    mine: u64,
}

impl Turn {
    pub(crate) fn current(&self) -> bool {
        self.asked.load(Ordering::SeqCst) == self.mine
    }
}

impl Lane {
    fn next(&self) -> Turn {
        Turn {
            asked: self.asked.clone(),
            mine: self.asked.fetch_add(1, Ordering::SeqCst) + 1,
        }
    }

    /// Drops the result of whatever is under way.
    pub(crate) fn cancel(&self) {
        self.asked.fetch_add(1, Ordering::SeqCst);
    }

    /// As `run`; `done` is not called once the lane was asked again, and
    /// `work` may stop early when its turn is no longer current.
    pub(crate) fn run<T, R>(
        &self,
        name: &str,
        thread: CxxQtThread<T>,
        work: impl FnOnce(&Turn) -> R + Send + 'static,
        done: impl FnOnce(Pin<&mut T>, R) + Send + 'static,
    ) where
        T: Threading + 'static,
        R: Send + 'static,
    {
        let turn = self.next();
        spawn(name, move || {
            let found = work(&turn);
            // Checked on the window's thread: another run may be asked
            // for between here and there.
            let told = thread.queue(move |object| {
                if turn.current() {
                    done(object, found);
                }
            });
            if let Err(problem) = told {
                rimstack_core::problem!("a finished task found nothing to tell: {problem:?}");
            }
        });
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

    #[test]
    fn a_named_thread_runs_its_work() {
        let (tell, heard) = std::sync::mpsc::channel();
        spawn("test-job", move || {
            let name = std::thread::current().name().map(String::from);
            tell.send(name).unwrap();
        });
        assert_eq!(heard.recv().unwrap().as_deref(), Some("test-job"));
    }
}
