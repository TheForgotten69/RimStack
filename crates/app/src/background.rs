//! Work done off the window's thread, with what it found handed back on it.

use core::pin::Pin;
use cxx_qt::{CxxQtThread, Threading};

/// Runs `work` on a thread of its own, then `done` with its result on
/// the thread of the object `thread` was taken from. An object that is
/// gone by then is not told.
pub(crate) fn run<T, R>(
    thread: CxxQtThread<T>,
    work: impl FnOnce() -> R + Send + 'static,
    done: impl FnOnce(Pin<&mut T>, R) + Send + 'static,
) where
    T: Threading + 'static,
    R: Send + 'static,
{
    std::thread::spawn(move || {
        let found = work();
        if let Err(problem) = thread.queue(move |object| done(object, found)) {
            rimstack_core::problem!("a finished task found nothing to tell: {problem:?}");
        }
    });
}
