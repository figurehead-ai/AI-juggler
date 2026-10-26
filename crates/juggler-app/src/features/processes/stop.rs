//! AJ-E2403 (D-41b): the user's own stop of another app's leftover.
//!
//! A process in a session another app runs may be stopped only:
//! - by the user's click (this path has no automatic caller, at any tier);
//! - after the session's own process has ended (every sighting of it is
//!   gone, and the live view no longer lists the session);
//! - when the live process is the sighted one: same pid, start time,
//!   program and boot ([`check`]);
//! - never the app itself, a process inside an app's bundle, the session's
//!   own process, AI Juggler or its hooks and tools, or anything in the
//!   spawn ledger (that is the ledger's own stop);
//! - through the same verify sheet, the same TERM → grace → KILL ladder
//!   (this process only) and the same audit, with `actor = user:observed`
//!   and the program's name in place of a command line.

use std::sync::Arc;

use juggler_core::storage::{CleanupAuditRecord, OUTCOME_PENDING, Sighting};

use crate::commands::{
    AuditWriter, KILL_GRACE_SECS, KillExecuteInfo, KillMemberInfo, KillMemberOutcome,
    KillVerifyInfo,
};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The audit's actor for this path.
pub const ACTOR_OBSERVED: &str = "user:observed";

/// Why a stop is refused, in words (the sheet shows the sentence and no
/// stop button).
pub mod refusal {
    /// No sighting of this process.
    pub const UNSEEN: &str =
        "AI Juggler never saw this process in a session, so it won't touch it.";
    /// The live process isn't the sighted one.
    pub const OTHER: &str =
        "This PID now belongs to a different program — AI Juggler will not touch it.";
    /// Its session still runs.
    pub const SESSION_RUNS: &str =
        "Its session is still running. Stop it from the app it runs in, or wait for it to end.";
    /// The session's own process.
    pub const SESSION_ITSELF: &str = "This is the session itself. Close it in the app it runs in.";
    /// An app's process.
    pub const APP: &str = "This is part of an app. Quit the app instead.";
    /// AI Juggler's own.
    pub const OWN: &str = "This is part of AI Juggler.";
    /// The ledger's (use its stop).
    pub const LEDGER: &str = "AI Juggler started this one: stop it from its own session.";
    /// A zombie.
    pub const ZOMBIE: &str =
        "It has already finished and waits for its parent; only the parent can clear it.";
    /// Gone.
    pub const GONE: &str = "It has already ended.";
}

/// What the live process is now, as the check needs it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Now {
    /// Its start time, epoch seconds.
    pub start_time: i64,
    /// Its executable path, when readable.
    pub exe: Option<String>,
    /// Its name.
    pub name: String,
    /// A zombie.
    pub zombie: bool,
}

/// Everything the check decides on (gathered fresh at click time).
#[derive(Clone, Debug, Default)]
#[allow(clippy::struct_excessive_bools)] // each a separate leg of the check
pub struct Facts {
    /// The process's sighting.
    pub sighting: Option<Sighting>,
    /// The process now (`None`: gone).
    pub now: Option<Now>,
    /// This boot.
    pub boot_id: String,
    /// Its session's own process(es) still run.
    pub session_alive: bool,
    /// The live view still lists its session.
    pub session_listed: bool,
    /// The pid has a spawn-ledger row on this Mac.
    pub in_ledger: bool,
    /// The pid is a host app's main process.
    pub is_app: bool,
    /// AI Juggler's own pid.
    pub own_pid: u32,
}

/// D-41b's identity and safety check, pure: `Ok` only when every leg
/// holds. The order is the law's: never AI Juggler, never the ledger's,
/// never the app, then the sighting (pid, start time, program, boot),
/// then the session's own process, then whether its session ended.
///
/// # Errors
///
/// The refusal in words.
pub fn check(pid: u32, facts: &Facts) -> Result<(), &'static str> {
    if pid <= 1 || pid == facts.own_pid {
        return Err(refusal::OWN);
    }
    if facts.in_ledger {
        return Err(refusal::LEDGER);
    }
    if facts.is_app {
        return Err(refusal::APP);
    }
    let Some(sighting) = facts.sighting.as_ref() else {
        return Err(refusal::UNSEEN);
    };
    let Some(now) = facts.now.as_ref() else {
        return Err(refusal::GONE);
    };
    if sighting.boot_id != facts.boot_id || sighting.pid != pid {
        return Err(refusal::OTHER);
    }
    if now.start_time != sighting.start_time {
        return Err(refusal::OTHER);
    }
    let program = now
        .exe
        .as_deref()
        .map_or(now.name.as_str(), super::roles::program_name);
    if program != sighting.exe_name {
        return Err(refusal::OTHER);
    }
    if let Some(exe) = now.exe.as_deref()
        && crate::features::conversations::places::outer_bundle(std::path::Path::new(exe)).is_some()
    {
        return Err(refusal::APP);
    }
    if program == juggler_agentbridge::wire::PROGRAM {
        return Err(refusal::OWN);
    }
    if now.zombie {
        return Err(refusal::ZOMBIE);
    }
    if sighting.session_root {
        return Err(refusal::SESSION_ITSELF);
    }
    if facts.session_alive || facts.session_listed {
        return Err(refusal::SESSION_RUNS);
    }
    Ok(())
}

/// The process now, from one targeted read (no scan, no environment).
#[must_use]
pub fn read_now(pid: u32) -> Option<Now> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
    let mut system = sysinfo::System::new();
    let target = sysinfo::Pid::from_u32(pid);
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[target]),
        true,
        ProcessRefreshKind::nothing()
            .without_tasks()
            .with_exe(UpdateKind::Always),
    );
    let process = system.process(target)?;
    Some(Now {
        start_time: i64::try_from(process.start_time()).unwrap_or_default(),
        exe: process.exe().map(|exe| exe.to_string_lossy().into_owned()),
        name: process.name().to_string_lossy().into_owned(),
        zombie: process.status() == sysinfo::ProcessStatus::Zombie,
    })
}

/// Gathers the facts for `pid` now: its sighting (memory, else the store),
/// the live process, its session's own processes, the ledger, the apps.
#[must_use]
pub fn facts_now(state: &Arc<AppState>, pid: u32, start: Option<i64>) -> Facts {
    let boot_id = state.ledger.boot_id().to_owned();
    let mut sighting = start.and_then(|start| state.processes.sighting(pid, start));
    let mut roots: Vec<Sighting> = Vec::new();
    if let Ok(storage) = state.storage.lock() {
        let repo = storage.sightings();
        if sighting.is_none() {
            sighting = match start {
                Some(start) => repo.get(pid, start, &boot_id).ok().flatten(),
                None => repo.latest_of(pid, &boot_id).ok().flatten(),
            };
        }
        if let Some(sighting) = sighting.as_ref() {
            roots = repo
                .session_roots(&sighting.session_id, &boot_id)
                .unwrap_or_default();
        }
    }
    let session_alive = roots.iter().any(|root| {
        read_now(root.pid).is_some_and(|now| now.start_time == root.start_time && !now.zombie)
    });
    let session_listed = sighting.as_ref().is_some_and(|sighting| {
        state
            .conversations
            .live_model()
            .entries
            .iter()
            .any(|entry| {
                entry.tool == sighting.tool
                    && entry.session_id == sighting.session_id
                    && !entry.places.is_empty()
            })
    });
    let machine = crate::commands::local_machine_id(state);
    let in_ledger = state
        .ledger
        .live_rows(&machine)
        .unwrap_or_default()
        .iter()
        .any(|row| row.pid == Some(pid));
    let is_app = state
        .conversations
        .live_model()
        .entries
        .iter()
        .any(|entry| {
            entry
                .places
                .iter()
                .any(|place| place.host.as_ref().is_some_and(|host| host.pid == pid))
        });
    Facts {
        sighting,
        now: read_now(pid),
        boot_id,
        session_alive,
        session_listed,
        in_ledger,
        is_app,
        own_pid: std::process::id(),
    }
}

/// The verify sheet for another app's leftover: the check's verdict now,
/// and what the sheet shows (its program, what it holds). Zero signals.
#[must_use]
pub fn verify(state: &Arc<AppState>, pid: u32, start: Option<i64>) -> KillVerifyInfo {
    let facts = facts_now(state, pid, start);
    let verdict = check(pid, &facts);
    let sighting = facts.sighting.clone();
    let exe_path = facts
        .now
        .as_ref()
        .and_then(|now| now.exe.clone())
        .unwrap_or_default();
    let (cmdline, cwd) = juggler_supervisor::tree::live_details(pid).unwrap_or_default();
    let ports: Vec<u16> = crate::commands::ports_held_by(pid)
        .unwrap_or_default()
        .split(',')
        .filter_map(|port| port.parse().ok())
        .collect();
    let role = super::roles::role_of(&exe_path, &cmdline, false);
    KillVerifyInfo {
        machine_id: crate::commands::local_machine_id(state).as_str().to_owned(),
        pid,
        session: sighting
            .as_ref()
            .map(|sighting| sighting.session_id.clone())
            .unwrap_or_default(),
        verified: verdict.is_ok(),
        refusal: verdict.err().map(str::to_owned),
        exe_path: exe_path.clone(),
        cmdline: cmdline.clone(),
        cwd,
        ports,
        pgid: None,
        escaped_group: false,
        members: vec![KillMemberInfo {
            pid,
            pgid: None,
            exe: super::roles::program_name(&exe_path).to_owned(),
            cmdline,
            role_guess: role.cleanup_word(),
            unsaved_state_warning: matches!(
                role,
                super::roles::Role::McpServer
                    | super::roles::Role::Shell
                    | super::roles::Role::Database
            )
            .then(|| format!("Includes a {} that may hold unsaved state.", role.words())),
            from_ledger: false,
        }],
        tree_pids: vec![pid],
        platform: if cfg!(windows) {
            "TerminateProcess only".to_owned()
        } else {
            "TERM → grace → KILL".to_owned()
        },
        grace_secs: crate::commands::setting_u64(state, "kill_grace_secs", KILL_GRACE_SECS),
        provider: sighting.map(|sighting| sighting.tool).unwrap_or_default(),
        account_id: None,
        project_id: None,
    }
}

/// The audit record for this path: the program's name and the keyed hash
/// stand in for the command line (D-41: the line is never stored).
fn record(state: &Arc<AppState>, sighting: &Sighting, ports: Option<&str>) -> CleanupAuditRecord {
    CleanupAuditRecord {
        id: 0,
        at_ms: crate::commands::now_millis(),
        actor: ACTOR_OBSERVED.to_owned(),
        pid: sighting.pid,
        pgid: None,
        exe_path: sighting.exe_name.clone(),
        cmdline_hash: sighting.cmd_hash.clone(),
        cmdline_trunc: sighting.exe_name.clone(),
        session_id: sighting.session_id.clone(),
        provider: sighting.tool.clone(),
        account_id: None,
        project_id: None,
        machine_id: crate::commands::local_machine_id(state).as_str().to_owned(),
        identity: format!(
            "sighting: pid {} start {} program {} boot {} — matched; session ended",
            sighting.pid, sighting.start_time, sighting.exe_name, sighting.boot_id
        ),
        scope: "pid".to_owned(),
        outcome: OUTCOME_PENDING.to_owned(),
        ports_before: ports.map(str::to_owned),
        ports_after: None,
    }
}

/// The user's stop of another app's leftover: the check again NOW (a
/// reused pid gets zero signals), the audit intent before any signal, then
/// TERM → grace → KILL for this process only.
///
/// # Errors
///
/// [`AppError::AuditUnavailable`] when the intent can't be written (no
/// audit, no stop).
pub fn execute(
    state: &Arc<AppState>,
    audit: &dyn AuditWriter,
    pid: u32,
    start: Option<i64>,
    cancel_flag: &Arc<std::sync::atomic::AtomicBool>,
) -> AppResult<KillExecuteInfo> {
    let facts = facts_now(state, pid, start);
    if let Err(copy) = check(pid, &facts) {
        return Ok(crate::commands::refused_kill(pid, copy));
    }
    let Some(sighting) = facts.sighting else {
        return Ok(crate::commands::refused_kill(pid, refusal::UNSEEN));
    };
    let ports_before = crate::commands::ports_held_by(pid);
    let intent = audit
        .intent(&record(state, &sighting, ports_before.as_deref()))
        .map_err(|_| AppError::AuditUnavailable)?;
    let cancel = Arc::clone(cancel_flag);
    // The last check, a moment before the first signal.
    let expected = sighting.start_time;
    let members = move || {
        read_now(pid)
            .filter(|now| now.start_time == expected && !now.zombie)
            .map_or_else(Vec::new, |_| vec![pid])
    };
    let outcome = juggler_supervisor::terminate_group(
        std::time::Duration::from_secs(crate::commands::setting_u64(
            state,
            "kill_grace_secs",
            KILL_GRACE_SECS,
        )),
        members,
        move || cancel.load(std::sync::atomic::Ordering::Relaxed),
    );
    let (still_holding, reoffer_pid) =
        crate::commands::still_holding_report(ports_before.as_deref(), pid, outcome.cancelled);
    if !outcome.cancelled {
        let words = if outcome.members.is_empty() {
            format!("pid {pid}: already gone")
        } else {
            outcome
                .members
                .iter()
                .map(|(pid, outcome)| format!("pid {pid}: {}", outcome.as_str()))
                .collect::<Vec<_>>()
                .join("; ")
        };
        let after = still_holding
            .iter()
            .map(|held| held.port.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let _ = audit.complete(
            intent,
            &words,
            (!after.is_empty()).then_some(after.as_str()),
        );
    }
    let members: Vec<KillMemberOutcome> = if outcome.members.is_empty() {
        vec![KillMemberOutcome {
            pid,
            outcome: "already gone".into(),
        }]
    } else {
        outcome
            .members
            .iter()
            .map(|(pid, outcome)| KillMemberOutcome {
                pid: *pid,
                outcome: outcome.as_str().to_owned(),
            })
            .collect()
    };
    Ok(KillExecuteInfo {
        refusal: None,
        all_succeeded: outcome.members.is_empty() || outcome.all_succeeded(),
        members,
        cancelled: outcome.cancelled,
        still_holding,
        reoffer_pid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sighting(root: bool) -> Sighting {
        Sighting {
            pid: 4242,
            start_time: 1_000,
            boot_id: "boot-1".into(),
            exe_name: "node".into(),
            cmd_hash: "h".into(),
            tool: "tool-a".into(),
            session_id: "conv-1".into(),
            app: Some("An app".into()),
            session_root: root,
            parent_pid: Some(1),
            first_seen: 0,
            last_seen: 0,
        }
    }

    fn facts() -> Facts {
        Facts {
            sighting: Some(sighting(false)),
            now: Some(Now {
                start_time: 1_000,
                exe: Some("/usr/local/bin/node".into()),
                name: "node".into(),
                zombie: false,
            }),
            boot_id: "boot-1".into(),
            own_pid: 99,
            ..Facts::default()
        }
    }

    #[test]
    fn a_verified_leftover_of_an_ended_session_may_be_stopped() {
        assert_eq!(check(4242, &facts()), Ok(()));
    }

    /// Every leg of the identity check fails closed.
    #[test]
    fn the_identity_check_fails_closed_on_every_leg() {
        type Case = (&'static str, Box<dyn Fn(&mut Facts)>, &'static str);
        let cases: Vec<Case> = vec![
            (
                "start time",
                Box::new(|f| f.now.as_mut().unwrap().start_time = 1_001),
                refusal::OTHER,
            ),
            (
                "program",
                Box::new(|f| f.now.as_mut().unwrap().exe = Some("/bin/zsh".into())),
                refusal::OTHER,
            ),
            (
                "boot",
                Box::new(|f| f.boot_id = "boot-2".into()),
                refusal::OTHER,
            ),
            ("gone", Box::new(|f| f.now = None), refusal::GONE),
            ("unseen", Box::new(|f| f.sighting = None), refusal::UNSEEN),
            (
                "zombie",
                Box::new(|f| f.now.as_mut().unwrap().zombie = true),
                refusal::ZOMBIE,
            ),
            (
                "session runs",
                Box::new(|f| f.session_alive = true),
                refusal::SESSION_RUNS,
            ),
            (
                "session listed",
                Box::new(|f| f.session_listed = true),
                refusal::SESSION_RUNS,
            ),
            (
                "session itself",
                Box::new(|f| f.sighting = Some(sighting(true))),
                refusal::SESSION_ITSELF,
            ),
            ("ledger", Box::new(|f| f.in_ledger = true), refusal::LEDGER),
            ("app", Box::new(|f| f.is_app = true), refusal::APP),
            (
                "inside an app",
                Box::new(|f| {
                    f.now.as_mut().unwrap().exe =
                        Some("/Applications/An.app/Contents/Helpers/node".into());
                }),
                refusal::APP,
            ),
        ];
        for (leg, change, expected) in cases {
            let mut case = facts();
            change(&mut case);
            assert_eq!(check(4242, &case), Err(expected), "leg: {leg}");
        }
        assert_eq!(check(99, &facts()), Err(refusal::OWN), "AI Juggler itself");
        assert_eq!(check(1, &facts()), Err(refusal::OWN), "launchd");
    }
}
