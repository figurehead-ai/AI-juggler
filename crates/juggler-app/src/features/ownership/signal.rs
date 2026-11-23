//! AJ-E2318 (the review's A1.6, A1.7 and E17): a process is asked to quit
//! only through its [`ProcId`] (its pid and when it started, to the
//! microsecond), checked against the kernel's listing right before the
//! signal: a pid the system gave another program since (between the look
//! that chose it and the signal) is never signalled. Before, the move
//! checked a pid's start to the second (with a second's slack), then sent
//! SIGTERM by the bare pid.

use std::collections::HashMap;

use super::ProcId;

/// The identity of each of `pids` running now, from the kernel's listing
/// (one listing for them all). A gone pid is absent. Where the system has
/// no such listing, the start is the process table's, to the second.
#[must_use]
pub fn identify(pids: &[u32]) -> HashMap<u32, ProcId> {
    if pids.is_empty() {
        return HashMap::new();
    }
    if let Some(listing) = juggler_proc_list::list() {
        return listing
            .iter()
            .filter(|listed| !listed.zombie && pids.contains(&listed.pid))
            .map(|listed| (listed.pid, ProcId::of(listed)))
            .collect();
    }
    crate::features::conversations::procs::some(pids)
        .into_values()
        .filter_map(|process| {
            let secs = u128::try_from(process.start?).ok()?;
            Some((
                process.pid,
                ProcId {
                    pid: process.pid,
                    start_micros: secs * 1_000_000,
                },
            ))
        })
        .collect()
}

impl ProcId {
    /// Whether this is the process a table row started at `start_secs`
    /// (epoch seconds, the process table's) names: the same second.
    #[must_use]
    pub fn started_in(&self, start_secs: Option<i64>) -> bool {
        start_secs
            .and_then(|secs| u128::try_from(secs).ok())
            .is_some_and(|secs| self.start_micros / 1_000_000 == secs)
    }
}

/// Why a process wasn't asked to quit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAsked {
    /// The pid is gone, or is another process now.
    NotThatProcess,
    /// The system refused, in its words.
    Refused(String),
}

/// Asks `id` to quit by `quit` (the pid's normal quit), only when `now`
/// (the kernel's listing, read right before) still names the very same
/// process at its pid.
///
/// # Errors
///
/// [`NotAsked`]: nothing was signalled, or the system refused.
pub fn quit_checked(
    id: ProcId,
    now: &dyn Fn(u32) -> Option<ProcId>,
    quit: &dyn Fn(u32) -> Result<(), String>,
) -> Result<(), NotAsked> {
    if now(id.pid) != Some(id) {
        return Err(NotAsked::NotThatProcess);
    }
    quit(id.pid).map_err(NotAsked::Refused)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    const ID: ProcId = ProcId {
        pid: 4242,
        start_micros: 1_795_428_000_000_123,
    };

    /// The very process at its pid: asked, once.
    #[test]
    fn the_same_process_is_asked_to_quit() {
        let asked = RefCell::new(Vec::new());
        let now = |_: u32| Some(ID);
        let quit = |pid| {
            asked.borrow_mut().push(pid);
            Ok(())
        };
        assert_eq!(quit_checked(ID, &now, &quit), Ok(()));
        assert_eq!(*asked.borrow(), [4242]);
    }

    /// A pid reused since the look (a start a microsecond later, the same
    /// second) or gone is never signalled.
    #[test]
    fn a_reused_or_gone_pid_is_never_signalled() {
        let asked = RefCell::new(Vec::new());
        let quit = |pid| {
            asked.borrow_mut().push(pid);
            Ok(())
        };
        let later = ProcId {
            start_micros: ID.start_micros + 1,
            ..ID
        };
        let reused = |_: u32| Some(later);
        let gone = |_: u32| None;
        assert_eq!(
            quit_checked(ID, &reused, &quit),
            Err(NotAsked::NotThatProcess)
        );
        assert_eq!(
            quit_checked(ID, &gone, &quit),
            Err(NotAsked::NotThatProcess)
        );
        assert!(asked.borrow().is_empty());
        assert!(later.started_in(Some(1_795_428_000)));
        assert!(!later.started_in(Some(1_795_428_001)));
        assert!(!later.started_in(None));
    }
}
