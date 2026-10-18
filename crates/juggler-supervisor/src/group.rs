//! Process-group ownership (AJ-E502, pass2 §4): ownership is created
//! AT SPAWN — the piped root becomes its own process-group leader, so
//! "terminate the tree" is later a deterministic `kill(-pgid)` (the
//! E507 ladder), not a racy ppid walk (F-9: a helper forking between
//! walk and signal escapes — the handover trap; do not fix it with a
//! walk).
//!
//! Mechanism pin (recorded in this task's PR): the task's gate allows
//! one `unsafe` `pre_exec` `setsid` — the stronger workspace law
//! `unsafe_code = "forbid"` (which E410's fixtures already routed
//! around) still bars it, and the STABLE std API
//! [`std::os::unix::process::CommandExt::process_group`] provides the
//! same new-process-group semantics (`setpgid(0, 0)`: the child leads
//! a fresh group, PGID == PID) with zero unsafe. A group, not a
//! session, is exactly the kill primitive pass2 §4 asks for; the PTY
//! path deliberately does NOT regroup (portable-pty owns session
//! leadership there) — it RECORDS the child's PGID instead.
//!
//! Windows: group operations are a documented
//! [`GroupError::UnsupportedPlatform`] — never a silent PID-only kill
//! fallback. The Job Object seam (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`,
//! pass2 §4.2 / EPICS M7-5) lands separately.

/// Group-operation failures (honest vocabulary; no silent fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupError {
    /// This platform has no process-group primitive behind the seam
    /// (Windows until the Job Object seam lands).
    UnsupportedPlatform,
    /// The process is gone (`getpgid` found nothing).
    ProcessGone,
}

impl std::fmt::Display for GroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                write!(f, "process groups unsupported on this platform")
            }
            Self::ProcessGone => write!(f, "process gone before its group was read"),
        }
    }
}

/// Makes a `Command`'s child its own process-group leader (Unix) so
/// every in-group helper it spawns is deterministically reachable as
/// one owned group. Called from the piped spawn site ONLY (the grep
/// gate: group mutations live in exactly the spawn path).
///
/// # Errors
///
/// [`GroupError::UnsupportedPlatform`] on platforms without the
/// primitive (Windows until the Job Object seam) — the caller spawns
/// anyway and records the honest groupless state.
#[cfg(unix)]
pub fn make_group_leader(command: &mut std::process::Command) -> Result<(), GroupError> {
    use std::os::unix::process::CommandExt;
    // 0 = a NEW process group with the child as leader (PGID == PID).
    command.process_group(0);
    Ok(())
}

/// The Windows seam: spawning works, groups do not exist behind this
/// interface — `UnsupportedPlatform` so kill paths can never pretend.
#[cfg(not(unix))]
pub fn make_group_leader(_command: &mut std::process::Command) -> Result<(), GroupError> {
    Err(GroupError::UnsupportedPlatform)
}

/// The process-group id of a live pid (one `getpgid` syscall — the
/// budget's per-root capture, reused per helper at reconcile time for
/// escape detection).
///
/// # Errors
///
/// [`GroupError::ProcessGone`] when the pid is gone;
/// [`GroupError::UnsupportedPlatform`] where the primitive does not
/// exist.
#[cfg(unix)]
pub fn pgid_of(pid: u32) -> Result<u32, GroupError> {
    let i32_pid = i32::try_from(pid).map_err(|_| GroupError::ProcessGone)?;
    nix::unistd::getpgid(Some(nix::unistd::Pid::from_raw(i32_pid)))
        .map(|pgid| u32::try_from(pgid.as_raw()).unwrap_or(0))
        .map_err(|_| GroupError::ProcessGone)
}

/// The Windows seam for group reads.
#[cfg(not(unix))]
pub fn pgid_of(_pid: u32) -> Result<u32, GroupError> {
    Err(GroupError::UnsupportedPlatform)
}
