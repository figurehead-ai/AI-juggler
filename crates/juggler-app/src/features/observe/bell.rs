//! A bus subscriber's worker state (the watches' engine, the digest's
//! scheduler): what its thread guards, and the bell that wakes the thread
//! when something is queued, changed or closed.

use std::sync::{Condvar, LockResult, Mutex, MutexGuard};
use std::time::Duration;

/// What a worker thread guards, and the bell that wakes it.
#[derive(Debug, Default)]
pub struct Bell<I> {
    inner: Mutex<I>,
    wake: Condvar,
}

impl<I> Bell<I> {
    /// The guarded state.
    ///
    /// # Errors
    ///
    /// The lock is poisoned (a thread panicked holding it).
    pub fn lock(&self) -> LockResult<MutexGuard<'_, I>> {
        self.inner.lock()
    }

    /// Wakes the thread.
    pub fn ring(&self) {
        self.wake.notify_all();
    }

    /// Marks the state closed (`close`) and wakes the thread, so it ends.
    pub fn close(&self, close: impl FnOnce(&mut I)) {
        if let Ok(mut inner) = self.inner.lock() {
            close(&mut inner);
        }
        self.ring();
    }

    /// Sleeps, the lock let go, until the bell rings or (with one) the
    /// wake time `at_ms` passes; `now_ms` is the time now. `None`: the
    /// lock was poisoned, and the thread ends.
    pub fn sleep<'a>(
        &'a self,
        guard: MutexGuard<'a, I>,
        at_ms: Option<i64>,
        now_ms: i64,
    ) -> Option<MutexGuard<'a, I>> {
        match at_ms {
            Some(at) => {
                let wait = Duration::from_millis(u64::try_from(at - now_ms).unwrap_or(0));
                self.wake
                    .wait_timeout(guard, wait)
                    .ok()
                    .map(|(next, _)| next)
            }
            None => self.wake.wait(guard).ok(),
        }
    }
}
