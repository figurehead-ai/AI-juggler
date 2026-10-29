//! The session stop's identity guard (D-01, zero false kills): which of
//! a session's pids still name our processes, read at stop time.
//!
//! - A spawned child that is not yet reaped is ours by construction (its
//!   pid cannot be reused until we wait on it).
//! - An ADOPTED root is not our fork: its pid is ours only while its
//!   recorded boot + start time still verify (AJ-E503).
//! - A reaped child's pid may already name a stranger. Its group is
//!   still ours while no process holds that number (a pid is never
//!   handed out while a group of that id exists), and never once one
//!   does.

use crate::tree::{Delivery, ProcessIdentity, Signal, Verification, signal_pid, verify_start_time};

/// `(root, group)`: whether the root pid, and the group it leads, may
/// be signalled now.
pub(crate) fn stop_targets(
    pid: u32,
    finished: bool,
    identity: Option<&ProcessIdentity>,
    adopted: bool,
) -> (bool, bool) {
    let root_ours = if adopted {
        identity.is_some_and(|identity| verify_start_time(pid, identity) == Verification::Verified)
    } else {
        !finished
    };
    // Only a number the system says no process holds: one it can't check
    // (another user's, or no signals on this system) is never ours.
    let group_ours =
        root_ours || (!adopted && signal_pid(pid, Signal::Probe) == Delivery::NoProcess);
    (root_ours, group_ours)
}

/// The session root's group members to ladder, the root excluded (a
/// dead-but-unreaped root would make the ladder escalate on a corpse).
/// Refusals surface on the next reconciler tick as orphans (AJ-E504);
/// a session stop's report stays the Exited lifecycle event.
///
/// GROUP GUARD: only the child's OWN group (`pgid == pid` — the
/// deterministic-kill group the piped path creates, AJ-E502). A child
/// that never became a leader (PTY/desktop paths, or a wrapper that
/// `setsid`'d) shares SOMEONE ELSE's group — signalling it would TERM
/// strangers, up to and including this process (the desktop suite's
/// three `stop()` calls caught exactly that).
#[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
pub(crate) fn group_helpers(pid: u32, pgid: Option<u32>) -> Vec<u32> {
    pgid.filter(|pgid| *pgid == pid)
        .map(crate::tree::pgid_members)
        .unwrap_or_default()
        .into_iter()
        .filter(|member| *member != pid)
        .collect()
}
