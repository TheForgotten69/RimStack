//! Locks taken whatever became of the thread that last held them. What
//! RimStack keeps behind a lock is replaced whole, never left half
//! changed, so a panic elsewhere leaves it fit to read.

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub trait Shared<T> {
    fn reading(&self) -> RwLockReadGuard<'_, T>;
    fn writing(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T> Shared<T> for RwLock<T> {
    fn reading(&self) -> RwLockReadGuard<'_, T> {
        self.read().unwrap_or_else(|e| e.into_inner())
    }

    fn writing(&self) -> RwLockWriteGuard<'_, T> {
        self.write().unwrap_or_else(|e| e.into_inner())
    }
}

pub trait Held<T> {
    fn held(&self) -> MutexGuard<'_, T>;
}

impl<T> Held<T> for Mutex<T> {
    fn held(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lock_is_still_taken_after_its_holder_panicked() {
        let kept = std::sync::Arc::new((RwLock::new(1), Mutex::new(2)));
        let held = kept.clone();
        let _ = std::thread::spawn(move || {
            let _both = (held.0.write().unwrap(), held.1.lock().unwrap());
            panic!("while holding both");
        })
        .join();
        *kept.0.writing() += 1;
        assert_eq!((*kept.0.reading(), *kept.1.held()), (2, 2));
    }
}
