//! The app's background ticks, each on its own named thread. A pass that
//! panics ends that pass only: the panic is kept as a crash report (the
//! hook in [`crate::crash`] runs first) and the tick goes on. Before
//! 0.1.8 one panic stopped its tick until AI Juggler restarted (2026-10-26
//! lab: a debug assertion in the disk read stopped the 2 s tick, and the
//! process graph, the cards and the subtasks stopped updating with it).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

/// Runs `pass` every `period`, first after one `period`.
pub fn every(name: &str, period: Duration, mut pass: impl FnMut() + Send + 'static) {
    spawn(name, move || {
        std::thread::sleep(period);
        once(&mut pass);
    });
}

/// Runs `pass` at once, then every `period` after each pass ends.
pub fn every_from_now(name: &str, period: Duration, mut pass: impl FnMut() + Send + 'static) {
    spawn(name, move || {
        once(&mut pass);
        std::thread::sleep(period);
    });
}

/// One pass; a panic in it ends the pass, not the tick.
pub fn once(pass: &mut impl FnMut()) {
    // The state a pass reads is behind locks that each reader already
    // takes with `lock().ok()` or recovers, so a pass cut short leaves
    // nothing half-seen for the next one.
    let _ = catch_unwind(AssertUnwindSafe(pass));
}

fn spawn(name: &str, mut step: impl FnMut() + Send + 'static) {
    let _ = std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(move || {
            loop {
                step();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
