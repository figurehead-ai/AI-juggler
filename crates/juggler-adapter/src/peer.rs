//! AJ-E3013 (D-116, ADR-0020's D-116 addendum): who a caller with no key
//! is, as a desktop App's adapter attests it.
//!
//! AI Juggler doesn't start a desktop app's sessions, so their bridge
//! never holds a key. The app attests such a caller instead, on every
//! call: it takes the peer's pid from the socket, its parent chain from
//! the kernel's own records ([`ProcStep`]), and asks each desktop App's
//! adapter whether that chain is one of the App's sessions, from the
//! tool's own records ([`DesktopPeer::attest`]). The adapter's answer is
//! [`PeerVerdict::Attested`] only when every check holds; anything less is
//! [`PeerVerdict::Unattested`], with the reason. Nothing the caller says
//! is part of it.

use std::path::Path;

/// One process in a caller's chain, as the kernel lists it (never as the
/// caller says).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcStep {
    /// Its pid.
    pub pid: u32,
    /// When it started, epoch seconds.
    pub start_secs: i64,
}

/// Where a desktop App keeps its files: the tool's config folder it runs
/// with (Claude Code's `CLAUDE_CONFIG_DIR`, Codex's `CODEX_HOME`) and the
/// app's own data folder (its `--user-data-dir`).
#[derive(Clone, Copy, Debug)]
pub struct DesktopPlace<'a> {
    /// The tool's config folder.
    pub config_dir: &'a Path,
    /// The app instance's own folder.
    pub instance_dir: &'a Path,
}

/// The session an adapter found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttestedSession {
    /// The tool's own conversation id (Claude Code's `sessionId`).
    pub conversation: String,
    /// The app's own id for it (the Claude app's `local_<id>`).
    pub host_session: String,
    /// The tool's process the caller descends from.
    pub tool_pid: u32,
    /// That process's start time, epoch seconds, as the kernel lists it.
    pub tool_start: i64,
}

/// An adapter's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeerVerdict {
    /// Every check held: the caller is this session.
    Attested(AttestedSession),
    /// Not this App's session, or not provably: why, in words. `stale`
    /// names a conversation whose record no longer matches its process
    /// (the app forgets what it kept for it).
    Unattested {
        /// Why.
        reason: String,
        /// A conversation found stale on the way.
        stale: Option<String>,
    },
}

impl PeerVerdict {
    /// Not attested, for `reason`.
    #[must_use]
    pub fn no(reason: impl Into<String>) -> Self {
        Self::Unattested {
            reason: reason.into(),
            stale: None,
        }
    }
}

/// The most steps up from the caller a tool's process may be: its MCP
/// server is its child; a hook runs under `sh`, so a grandchild.
pub const MAX_HOPS: usize = 2;

/// A desktop App's half of the attestation (Claude: its session registry
/// and the app's own record; Codex: none yet).
pub trait DesktopPeer: Send + Sync {
    /// Whether the caller whose chain is `chain` (the caller first, then
    /// its parent and grandparent, as the kernel lists them) is one of
    /// the sessions of the App at `place`. Reads only the tool's own
    /// records, never a secret, and never trusts anything the caller
    /// said.
    fn attest(&self, place: &DesktopPlace<'_>, chain: &[ProcStep]) -> PeerVerdict;

    /// Whether this App's sessions can be woken by a pool message (D-119):
    /// `false` when no session of it has an inbox AI Juggler's connector
    /// can reach (a Codex app thread).
    fn can_wake(&self) -> bool;

    /// H3: whether `session`, attested earlier, is still the one the
    /// tool's own records name for its process (its registry entry still
    /// there, for the same conversation, with the same start time).
    /// `false` (the default) drops what was kept for it at once.
    fn still_current(&self, place: &DesktopPlace<'_>, session: &AttestedSession) -> bool {
        let _ = (place, session);
        false
    }

    /// Whether this App's sessions can be attested at all (`false`: every
    /// keyless call from them is refused, and the window says why).
    fn attests(&self) -> bool {
        true
    }

    /// Why AI Juggler's connector isn't offered for this App yet, in the
    /// window's words; `None`: it is. An App whose sessions can't be told
    /// apart says so, and the window offers no Add for it.
    fn not_offered(&self) -> Option<String> {
        None
    }
}

/// Whether a recorded start time and the process's own are the same
/// process: both known, at most a second apart (one is rounded, the
/// other truncated). Unlike [`crate::live::same_start`], nothing recorded
/// is never a match: attestation takes nothing on trust.
#[must_use]
pub fn same_start_strict(recorded: Option<i64>, actual: i64) -> bool {
    recorded.is_some_and(|recorded| (recorded - actual).abs() <= 1)
}

#[cfg(test)]
mod tests {
    use super::same_start_strict;

    #[test]
    fn nothing_recorded_is_never_the_same_process() {
        assert!(same_start_strict(Some(100), 100));
        assert!(same_start_strict(Some(101), 100));
        assert!(!same_start_strict(Some(102), 100));
        assert!(!same_start_strict(None, 100));
    }
}
