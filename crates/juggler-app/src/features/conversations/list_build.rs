//! The conversation list's first build, as the window sees it: Home and
//! the rail say how far it is ("Reading conversations: 120 of 1,000")
//! while it runs, and, when a pass fails, say so with a Try again.
//!
//! The window asks again whenever it's told the conversations changed;
//! the build's end is always told (rc24 walk K1: a first start with
//! nothing new to read told nothing, so the line never went away).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::state::AppState;

/// How far the first build is.
#[derive(Debug, Default)]
pub struct Progress {
    /// Files listed for the first round.
    total: AtomicUsize,
    /// Of those, the ones done.
    read: AtomicUsize,
    /// The last pass failed while the list was still building.
    failed: AtomicBool,
}

impl Progress {
    /// A pass's first round starts over `total` files, `done` of them
    /// already read.
    pub(super) fn start(&self, total: usize, done: usize) {
        self.total.store(total, Ordering::Relaxed);
        self.read.store(done.min(total), Ordering::Relaxed);
    }

    /// Try again was pressed: the window shows the build running again
    /// until the pass it starts says how it went.
    pub(super) fn retry(&self) {
        self.failed.store(false, Ordering::Relaxed);
    }

    /// One more file of the first round is done.
    pub(super) fn step(&self) {
        let total = self.total.load(Ordering::Relaxed);
        let _ = self
            .read
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |read| {
                (read < total).then_some(read + 1)
            });
    }
}

/// A pass's first round starts over `total` files, `done` of them read.
pub(super) fn started(state: &AppState, total: usize, done: usize) {
    state.conversations.build.start(total, done);
}

/// The pass's first round is done: the list is whole, and the first build
/// ends here, not minutes later (the second round only counts tokens).
pub(super) fn listed(state: &AppState) {
    state
        .conversations
        .first_done
        .store(true, Ordering::Relaxed);
}

/// The first build, for the window.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ListBuildInfo {
    /// The list is still being built for the first time.
    pub building: bool,
    /// Conversation files read so far.
    pub read: u32,
    /// Conversation files found (0 until they're listed).
    pub total: u32,
    /// The last pass failed: the window says so and offers Try again.
    pub failed: bool,
}

/// Where the first build is now.
#[must_use]
pub fn info(state: &AppState) -> ListBuildInfo {
    let progress = &state.conversations.build;
    let building = super::building(state);
    let count =
        |value: &AtomicUsize| u32::try_from(value.load(Ordering::Relaxed)).unwrap_or(u32::MAX);
    ListBuildInfo {
        building,
        read: count(&progress.read),
        total: count(&progress.total),
        failed: building && progress.failed.load(Ordering::Relaxed),
    }
}

/// One background pass of the index ([`super::tick_with`]), telling the
/// window (`tell`) when anything it shows changed, and always when the
/// first build ends or fails (or recovers from failing).
pub fn tick_and_tell(state: &Arc<AppState>, every: std::time::Duration, tell: &dyn Fn()) {
    let before = info(state);
    let changed = match super::tick_with(state, every, tell) {
        Ok(changed) => {
            state
                .conversations
                .build
                .failed
                .store(false, Ordering::Relaxed);
            changed
        }
        Err(error) => {
            tracing::warn!(%error, "conversation list pass failed");
            if super::building(state) {
                state
                    .conversations
                    .build
                    .failed
                    .store(true, Ordering::Relaxed);
            }
            false
        }
    };
    let after = info(state);
    if changed || before.building != after.building || before.failed != after.failed {
        tell();
    }
}

#[cfg(test)]
#[allow(clippy::panic, reason = "a reader that dies is what the tests stage")]
mod tests {
    use super::*;

    /// A first start's state (nothing saved), and how many times one
    /// background pass over it told the window.
    fn state() -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::bare_state(dir.path().join("data"));
        (dir, state)
    }

    fn counted(state: &Arc<AppState>) -> usize {
        let told = AtomicUsize::new(0);
        let tell = || {
            told.fetch_add(1, Ordering::Relaxed);
        };
        tick_and_tell(state, super::super::PROGRESS_EVERY, &tell);
        told.into_inner()
    }

    /// rc24 walk K1: a first start with nothing to read (no transcripts
    /// yet, or none changed) found nothing new, so the window was never
    /// told, never asked again, and said "Building the conversation
    /// list…" for good.
    #[test]
    fn a_build_that_reads_nothing_still_tells_the_window_it_ended() {
        let (_dir, state) = state();
        assert!(info(&state).building, "a first start builds");
        assert!(
            counted(&state) >= 1,
            "the window hears that the build ended"
        );
        assert!(!info(&state).building, "the pass ended the build");
        assert_eq!(
            counted(&state),
            0,
            "a pass that changes nothing tells nothing"
        );
    }

    /// A pass that fails while the list builds says so (the window shows
    /// words and a Try again); the next good pass ends the build and
    /// tells that too.
    #[test]
    fn a_failing_build_says_so_and_a_good_pass_ends_it() {
        let (_dir, state) = state();
        let poisoned = Arc::clone(&state);
        let _ = std::thread::spawn(move || {
            let _storage = poisoned.storage.lock().unwrap();
            panic!("a writer died holding storage");
        })
        .join();
        assert_eq!(counted(&state), 1, "the failure is told");
        let failed = info(&state);
        assert!(failed.building && failed.failed, "{failed:?}");
        assert_eq!(counted(&state), 0, "told once, not every tick");
        state.storage.clear_poison();
        assert_eq!(counted(&state), 1, "the recovery is told");
        let after = info(&state);
        assert!(!after.building && !after.failed, "{after:?}");
    }

    /// A pass that panicked (its tick goes on) poisoned the one-pass lock:
    /// every later pass failed with it, and the build never ended.
    #[test]
    fn a_pass_that_panicked_never_stops_the_next() {
        let (_dir, state) = state();
        let poisoned = Arc::clone(&state);
        let _ = std::thread::spawn(move || {
            let _pass = poisoned.conversations.pass.lock().unwrap();
            panic!("a reader died mid-pass");
        })
        .join();
        assert!(!state.conversations.is_reading(), "no pass is reading");
        counted(&state);
        let after = info(&state);
        assert!(!after.building && !after.failed, "{after:?}");
    }

    #[test]
    fn progress_counts_the_first_round_and_never_past_its_end() {
        let progress = Progress::default();
        progress.start(3, 1);
        progress.step();
        progress.step();
        progress.step();
        assert_eq!(progress.read.load(Ordering::Relaxed), 3);
        assert_eq!(progress.total.load(Ordering::Relaxed), 3);
    }
}
