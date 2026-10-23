//! The suffix new ids are made from (`sess-…`, accounts, launch settings,
//! switches).
//!
//! It's the clock in nanoseconds (mod 10^12), and never the same twice in
//! this process. On macOS the clock reads in microseconds, so two
//! launches in the same instant got one session id and the second failed
//! to save (found by AJ-E1522's two-session test).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// The last suffix handed out in this process.
static LAST: AtomicU64 = AtomicU64::new(0);

/// A new suffix: the clock's, or one more than the last when the clock
/// hasn't moved on.
#[must_use]
pub fn unique_suffix() -> u128 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() % 1_000_000_000_000)
        .unwrap_or_default();
    let now = u64::try_from(now).unwrap_or_default();
    let mut last = LAST.load(Ordering::Relaxed);
    loop {
        let next = if now > last { now } else { last + 1 };
        match LAST.compare_exchange_weak(last, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return u128::from(next),
            Err(seen) => last = seen,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::unique_suffix;

    #[test]
    fn no_two_ids_are_the_same_even_in_one_instant() {
        let threads: Vec<_> = (0..4)
            .map(|_| std::thread::spawn(|| (0..5_000).map(|_| unique_suffix()).collect::<Vec<_>>()))
            .collect();
        let mut all: Vec<u128> = threads
            .into_iter()
            .flat_map(|t| t.join().unwrap())
            .collect();
        let total = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), total, "every suffix is new");
    }
}
