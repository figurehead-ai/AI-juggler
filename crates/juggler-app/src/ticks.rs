//! The app's background ticks, each on its own named thread. A pass that
//! panics ends that pass only: the panic is kept as a crash report (the
//! hook in [`crate::crash`] runs first) and the tick goes on. Before
//! 0.1.8 one panic stopped its tick until AI Juggler restarted (2026-10-26
//! lab: a debug assertion in the disk read stopped the 2 s tick, and the
//! process graph, the cards and the subtasks stopped updating with it).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

/// Whether the window has painted once (the UI says so, after its first
/// paint): heavy first work waits for it.
static PAINTED: Mutex<bool> = Mutex::new(false);
static PAINTED_NOW: Condvar = Condvar::new();

/// The window painted its first frame: work waiting on it may start.
pub fn painted() {
    if let Ok(mut painted) = PAINTED.lock() {
        *painted = true;
        PAINTED_NOW.notify_all();
    }
}

/// Waits until the window has painted, or `at_most` passes (a window
/// started hidden, at login, never paints until shown).
pub fn wait_for_paint(at_most: Duration) {
    let until = Instant::now() + at_most;
    let Ok(mut painted) = PAINTED.lock() else {
        return;
    };
    while !*painted {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return;
        }
        match PAINTED_NOW.wait_timeout(painted, left) {
            Ok((next, _)) => painted = next,
            Err(_) => return,
        }
    }
}

/// Runs `pass` once the window has painted (or after `at_most`), then
/// every `period` after each pass ends, on a thread of the lowest
/// priority: heavy first work (a first conversation list read over
/// every transcript) never competes with the first paint or the window.
pub fn every_after_paint(
    name: &str,
    at_most: Duration,
    period: Duration,
    mut pass: impl FnMut() + Send + 'static,
) {
    let mut first = true;
    spawn(name, move || {
        if first {
            first = false;
            let _ = juggler_platform::thread::lowest_priority();
            wait_for_paint(at_most);
        }
        once(&mut pass);
        std::thread::sleep(period);
        true
    });
}

/// Runs `pass` every `period`, first after one `period`.
pub fn every(name: &str, period: Duration, mut pass: impl FnMut() + Send + 'static) {
    spawn(name, move || {
        std::thread::sleep(period);
        once(&mut pass);
        true
    });
}

/// Runs `pass` every `period`, first after `first` (work that would
/// compete with the window's first paint waits for it), on a thread of
/// the lowest priority (background work).
pub fn every_after(
    name: &str,
    first: Duration,
    period: Duration,
    mut pass: impl FnMut() + Send + 'static,
) {
    let mut wait = first;
    let mut lowered = false;
    spawn(name, move || {
        if !lowered {
            lowered = true;
            let _ = juggler_platform::thread::lowest_priority();
        }
        std::thread::sleep(wait);
        wait = period;
        once(&mut pass);
        true
    });
}

/// Runs `pass` at once, then every `period` after each pass ends.
pub fn every_from_now(name: &str, period: Duration, mut pass: impl FnMut() + Send + 'static) {
    spawn(name, move || {
        once(&mut pass);
        std::thread::sleep(period);
        true
    });
}

/// Runs `pass` every `period`, first after one `period`, until a pass
/// returns `false` (what it serves is gone: the app state it holds
/// weakly, as the 0.2 emitters do). A pass that panics goes on.
pub fn every_while(name: &str, period: Duration, mut pass: impl FnMut() -> bool + Send + 'static) {
    spawn(name, move || {
        std::thread::sleep(period);
        catch_unwind(AssertUnwindSafe(&mut pass)).unwrap_or(true)
    });
}

/// One pass; a panic in it ends the pass, not the tick.
pub fn once(pass: &mut impl FnMut()) {
    // The state a pass reads is behind locks that each reader already
    // takes with `lock().ok()` or recovers, so a pass cut short leaves
    // nothing half-seen for the next one.
    let _ = catch_unwind(AssertUnwindSafe(pass));
}

/// The tick's thread: `step` again while it says to go on.
fn spawn(name: &str, mut step: impl FnMut() -> bool + Send + 'static) {
    let _ = std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(move || while step() {});
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_waiting_for_the_first_paint_starts_when_the_window_paints() {
        let painter = std::thread::spawn(|| {
            std::thread::sleep(Duration::from_millis(50));
            painted();
        });
        let start = Instant::now();
        wait_for_paint(Duration::from_secs(10));
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "woken by the paint"
        );
        painter.join().unwrap();
        // painted once: later waits don't wait
        let start = Instant::now();
        wait_for_paint(Duration::from_secs(10));
        assert!(start.elapsed() < Duration::from_secs(1));
    }
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn a_tick_while_ends_when_a_pass_says_so_and_a_panic_doesnt_end_it() {
        let passes = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&passes);
        every_while("test-tick-while", Duration::from_millis(10), move || {
            let n = seen.fetch_add(1, Ordering::SeqCst);
            assert!(n != 1, "the second pass panics");
            n < 3
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while passes.load(Ordering::SeqCst) < 4 {
            assert!(
                std::time::Instant::now() < deadline,
                "the tick stopped after a panic"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        // The fourth pass said stop: no fifth.
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(passes.load(Ordering::SeqCst), 4);
    }

    /// The subagent count's first pass waits for the window (the go
    /// sitting's startup minute), then runs on its period.
    #[test]
    fn a_tick_after_waits_for_its_first_then_keeps_its_period() {
        let passes = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&passes);
        every_after(
            "test-tick-after",
            Duration::from_millis(300),
            Duration::from_millis(20),
            move || {
                seen.fetch_add(1, Ordering::SeqCst);
            },
        );
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(passes.load(Ordering::SeqCst), 0, "not before its first");
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while passes.load(Ordering::SeqCst) < 3 {
            assert!(std::time::Instant::now() < deadline, "it ran on");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn a_pass_that_panics_doesnt_stop_the_tick() {
        let passes = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&passes);
        every_from_now("test-tick", Duration::from_millis(10), move || {
            let n = seen.fetch_add(1, Ordering::SeqCst);
            assert!(n != 1, "the second pass panics");
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while passes.load(Ordering::SeqCst) < 4 {
            assert!(
                std::time::Instant::now() < deadline,
                "the tick stopped after a panic"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
