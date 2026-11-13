//! Every thread RimStack starts goes through here, named for the diagnostics.
//!
//! Under test, a job asked for on the window's thread runs on the test's own executor instead:
//! gpui's test scheduler takes a wake from any other thread for a fault, and hung when one came
//! while a test's window was closing (2026-11-12).

use futures::channel::oneshot;
use gpui::{AsyncApp, Context, Task, WeakEntity};
use rimstack_core::base::locks::Held;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(test)]
thread_local! {
    static TESTED: std::cell::RefCell<Option<gpui::BackgroundExecutor>> =
        const { std::cell::RefCell::new(None) };
}

/// The jobs asked for on this thread from now on run on `executor`, as a test drives it.
#[cfg(test)]
pub fn run_on(executor: gpui::BackgroundExecutor) {
    TESTED.with(|tested| *tested.borrow_mut() = Some(executor));
}

/// Threads running jobs at most; a job asked beyond it waits for one of them, and the asker
/// never waits.
const MOST_THREADS: usize = 32;

type Job = Box<dyn FnOnce() + Send>;

/// Threads running jobs, and the jobs waiting for one.
static RUNNING: Mutex<(usize, std::collections::VecDeque<(String, Job)>)> =
    Mutex::new((0, std::collections::VecDeque::new()));

pub fn spawn(name: &str, work: impl FnOnce() + Send + 'static) {
    #[cfg(test)]
    if let Some(executor) = TESTED.with(|tested| tested.borrow().clone()) {
        executor.spawn(async move { work() }).detach();
        return;
    }
    let mut running = RUNNING.held();
    if running.0 >= MOST_THREADS {
        running.1.push_back((name.to_string(), Box::new(work)));
        return;
    }
    running.0 += 1;
    drop(running);
    start(name.to_string(), Box::new(work));
}

fn start(name: String, work: Job) {
    let started = std::thread::Builder::new()
        .name(name.clone())
        .spawn(move || {
            let mut work = work;
            loop {
                work();
                let mut running = RUNNING.held();
                match running.1.pop_front() {
                    Some((_, next)) => work = next,
                    None => {
                        running.0 -= 1;
                        return;
                    }
                }
            }
        });
    if let Err(problem) = started {
        RUNNING.held().0 -= 1;
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
    let waiting = off_thread(name, work);
    cx.spawn(async move |view: WeakEntity<V>, cx: &mut AsyncApp| {
        if let Ok(found) = waiting.await {
            let _ = view.update(cx, |view, cx| done(view, found, cx));
        }
    })
    .detach();
}

/// `work`'s answer, to be awaited on the window's thread.
pub fn off_thread<R: Send + 'static>(
    name: &str,
    work: impl FnOnce() -> R + Send + 'static,
) -> oneshot::Receiver<R> {
    let (found, waiting) = oneshot::channel();
    spawn(name, move || {
        let _ = found.send(work());
    });
    waiting
}

/// A wait on the window's executor, which a test does not sit through.
fn pause(wait: Duration, cx: &AsyncApp) -> Task<()> {
    match cfg!(test) || wait.is_zero() {
        true => Task::ready(()),
        false => cx.background_executor().timer(wait),
    }
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
        self.run_after(Duration::ZERO, name, cx, work, done);
    }

    /// As `run`, once `wait` has passed with nothing newer asked: the pause of a field that
    /// applies itself. No thread is held while waiting.
    pub fn run_after<V: 'static, R: Send + 'static>(
        &self,
        wait: Duration,
        name: &str,
        cx: &mut Context<V>,
        work: impl FnOnce(&Turn) -> R + Send + 'static,
        done: impl FnOnce(&mut V, R, &mut Context<V>) + 'static,
    ) {
        let turn = self.next();
        let name = name.to_string();
        cx.spawn(async move |view: WeakEntity<V>, cx: &mut AsyncApp| {
            pause(wait, cx).await;
            if !turn.current() {
                return;
            }
            let mine = turn.clone();
            let Ok(found) = off_thread(&name, move || work(&mine)).await else {
                return;
            };
            // Checked on the window's thread: another run may be asked for meanwhile.
            if turn.current() {
                let _ = view.update(cx, |view, cx| done(view, found, cx));
            }
        })
        .detach();
    }

    /// `done` on the window's thread once `wait` has passed with nothing newer asked.
    pub fn after<V: 'static>(
        &self,
        wait: Duration,
        cx: &mut Context<V>,
        done: impl FnOnce(&mut V, &mut Context<V>) + 'static,
    ) {
        let turn = self.next();
        cx.spawn(async move |view: WeakEntity<V>, cx: &mut AsyncApp| {
            pause(wait, cx).await;
            if turn.current() {
                let _ = view.update(cx, |view, cx| done(view, cx));
            }
        })
        .detach();
    }
}

/// Work of which one runs at a time: asked again while it runs, it runs once more after with
/// the newest of what was asked, however often that was.
pub struct Single<T> {
    /// Whether a run is under way, and what the next is to be given.
    state: Arc<Mutex<(bool, Option<T>)>>,
}

impl<T> Default for Single<T> {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new((false, None))),
        }
    }
}

impl<T: Send + 'static> Single<T> {
    pub fn ask(&self, name: &str, given: T, work: impl Fn(T) + Send + 'static) {
        let mut state = self.state.held();
        state.1 = Some(given);
        if state.0 {
            return;
        }
        state.0 = true;
        drop(state);
        let state = self.state.clone();
        spawn(name, move || {
            loop {
                let next = {
                    let mut state = state.held();
                    match state.1.take() {
                        Some(next) => next,
                        None => {
                            state.0 = false;
                            return;
                        }
                    }
                };
                work(next);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jobs_past_the_most_threads_wait_for_one_and_all_run() {
        let (release, held) = std::sync::mpsc::channel::<()>();
        let held = Arc::new(Mutex::new(held));
        let (done, finished) = std::sync::mpsc::channel();
        let asked = MOST_THREADS + 8;
        for at in 0..asked {
            let (held, done) = (held.clone(), done.clone());
            spawn("held", move || {
                if at < MOST_THREADS {
                    let _ = held.held().recv();
                }
                let _ = done.send(at);
            });
        }
        assert!(RUNNING.held().0 <= MOST_THREADS);
        for _ in 0..MOST_THREADS {
            release.send(()).unwrap();
        }
        let wait = std::time::Duration::from_secs(5);
        let ran = (0..asked)
            .filter(|_| finished.recv_timeout(wait).is_ok())
            .count();
        assert_eq!(ran, asked);
    }

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
    fn work_asked_while_it_runs_runs_once_more_with_the_newest() {
        let single = Single::default();
        let (started, running) = std::sync::mpsc::channel::<()>();
        let (release, held) = std::sync::mpsc::channel::<()>();
        let (seen, heard) = std::sync::mpsc::channel();
        let held = Arc::new(Mutex::new(held));
        let work = move |given: u32| {
            if given == 1 {
                let _ = started.send(());
                let _ = held.held().recv();
            }
            let _ = seen.send(given);
        };
        single.ask("first", 1, work.clone());
        running.recv().unwrap();
        single.ask("second", 2, work.clone());
        single.ask("third", 3, work);
        release.send(()).unwrap();
        let got: Vec<u32> = (0..2).map(|_| heard.recv().unwrap()).collect();
        assert_eq!(got, [1, 3]);
        assert!(
            heard
                .recv_timeout(std::time::Duration::from_millis(50))
                .is_err()
        );
    }
}
