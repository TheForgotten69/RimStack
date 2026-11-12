//! `Theme.quick` and `Theme.calm`, and the motions `PopIn.qml`, `PopOut.qml` and `Appear.qml`
//! give: what is opened fades in, what closes fades out, a section shown fades in. gpui-fast
//! scales no `div`, so `PopIn`'s growth from 0.97 is left out. Reduce motion stills them all
//! (`AnimationExt` reads `App::reduce_motion`).

use gpui::{Animation, AnimationExt, ElementId, IntoElement, Styled};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

pub const QUICK: Duration = Duration::from_millis(140);
pub const CALM: Duration = Duration::from_millis(220);

pub fn out_cubic(t: f32) -> f32 {
    1. - (1. - t).powi(3)
}

static OPENINGS: AtomicUsize = AtomicUsize::new(0);

/// A turn of its own for each opening, so that its motion starts again.
pub fn opening() -> usize {
    OPENINGS.fetch_add(1, Ordering::Relaxed) + 1
}

pub fn pop_in<E: Styled + IntoElement + 'static>(element: E, turn: usize) -> impl IntoElement {
    element.with_animation(("pop-in", turn), Animation::new(QUICK), |e, t| e.opacity(t))
}

pub fn pop_out<E: Styled + IntoElement + 'static>(element: E, turn: usize) -> impl IntoElement {
    element.with_animation(("pop-out", turn), Animation::new(QUICK), |e, t| {
        e.opacity(1. - t)
    })
}

pub fn appear<E: Styled + IntoElement + 'static>(
    element: E,
    id: impl Into<ElementId>,
) -> impl IntoElement {
    element.with_animation(id, Animation::new(CALM).with_easing(out_cubic), |e, t| {
        e.opacity(t)
    })
}

/// A spring that settles in about `settled`, as `Behavior on x` with `Easing.OutCubic` did, to
/// within `epsilon` of `to` in its own measure; it starts from where it was, so a change met
/// halfway turns back smoothly, and a new one starts at its end.
pub fn towards<T: gpui::SpringTarget>(
    to: T,
    epsilon: f32,
    settled: Duration,
) -> gpui::SpringAnimation<T> {
    // Critically damped: (1 + 7) e^-7, under 1% of the way, is left once `settled` is over.
    let frequency = 7. / settled.as_secs_f32();
    gpui::SpringAnimation::new(gpui::SpringConfig::new(
        frequency * frequency,
        2. * frequency,
        1.,
    ))
    .to(to)
    .with_epsilon(epsilon)
}

thread_local! {
    /// The controls pressed, and when: only their springs run, as Qt's `settled` let a
    /// `Behavior` run once a control was shown, so a value that arrives later shows at once.
    static PRESSED: std::cell::RefCell<std::collections::HashMap<ElementId, std::time::Instant>> =
        Default::default();
}

pub fn pressed(id: &ElementId) {
    PRESSED.with_borrow_mut(|pressed| {
        pressed.insert(id.clone(), std::time::Instant::now());
    });
}

/// Running for a control pressed within the last second, at its end otherwise: a pick may come
/// back through a read off the window's thread (Settings reads its report again, Git's version
/// among it), so the second is a bound on that read, not on the motion.
pub fn playback(id: &ElementId) -> gpui::SpringPlayback {
    let running = PRESSED.with_borrow_mut(|pressed| {
        pressed.retain(|_, when| when.elapsed() < Duration::from_secs(1));
        pressed.contains_key(id)
    });
    match running {
        true => gpui::SpringPlayback::Running,
        false => gpui::SpringPlayback::Completed,
    }
}
