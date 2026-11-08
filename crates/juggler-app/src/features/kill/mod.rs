//! AJ-E507: verify before kill: the sheet's verification and the manual TERM
//! then KILL ladder, with its audit.

pub mod audit;
pub mod commands;
pub mod dto;

use std::sync::Arc;

use juggler_core::model::SessionId;

use crate::commands::{local_machine_id, now_millis};
use crate::error::AppError;
use crate::features::cleanup::sweep::setting_u64;
use crate::features::kill::audit::AuditWriter;
use crate::features::kill::dto::{
    KillExecuteInfo, KillMemberInfo, KillMemberOutcome, KillVerifyInfo, StillHoldingInfo,
};
use crate::features::port::{os_listeners, ports_held_by};
use crate::state::AppState;

// --- AJ-E507: verify-before-kill sheet + manual TERM→KILL ladder ---

/// The kill sheet's default grace (pass2 §9.3 — matches the
/// `stop_session` grace).
pub const KILL_GRACE_SECS: u64 = 5;

/// The exact refusal copy when click-time verification fails
/// (Req.2 — the sheet renders it with the kill button REMOVED).
pub const KILL_REFUSAL_COPY: &str =
    "This PID now belongs to a different program — Juggler will not touch it.";

/// The refusal when the process has simply ended (processes lab
/// 2026-10-26: a shell that exited with its server was called "a
/// different program").
pub const KILL_GONE_COPY: &str = "This process has already ended — there's nothing to stop.";

/// Why a target didn't verify, in the words that are true now: it has
/// ended, it's a zombie only its parent clears, or the pid is another
/// program's. Read after the verdict, never instead of it: every case
/// sends zero signals.
fn unverified_copy(pid: u32) -> &'static str {
    match crate::features::processes::stop::read_now(pid) {
        None => KILL_GONE_COPY,
        Some(now) if now.zombie => crate::features::processes::stop::refusal::ZOMBIE,
        Some(_) => KILL_REFUSAL_COPY,
    }
}

/// The AJ-E511 foreign-machine refusal (Req.2): typed
/// [`juggler_supervisor::SupervisorError::WrongMachine`] under the
/// hood — this is its sheet-facing copy.
pub const KILL_WRONG_MACHINE_COPY: &str =
    "This process record belongs to another machine — Juggler will not touch it.";

/// A Clean refused before any signal, with its copy.
pub(crate) fn refused_kill(pid: u32, copy: &str) -> KillExecuteInfo {
    KillExecuteInfo {
        refusal: Some(copy.to_string()),
        members: vec![KillMemberOutcome {
            pid,
            outcome: "refused".into(),
        }],
        cancelled: false,
        all_succeeded: false,
        still_holding: Vec::new(),
        reoffer_pid: None,
    }
}

/// AJ-E708 Req.5: the refusal while an Instance's lineage diverged.
pub const KILL_DIVERGED_COPY: &str = "This work diverged across machines — choose what to keep (Keep mine, Keep theirs or Keep both) first. Nothing was signalled.";

/// The exact unsaved-state warning copy (Req.5), in plain words
/// ("Includes an MCP server that may hold unsaved state.").
fn unsaved_state_copy(role: &str) -> String {
    let what = match role {
        "McpServer" => "an MCP server".to_string(),
        other => format!("a {}", role_words(other)),
    };
    format!("Includes {what} that may hold unsaved state.")
}

/// A [`guess_role`] code in the words people use ("dev server",
/// "MCP server"); empty when nothing is known. The UI says "probably"
/// before it: a guess is never shown as a fact.
#[must_use]
pub fn role_words(role: &str) -> &'static str {
    match role {
        "McpServer" => "MCP server",
        "Database" => "database",
        "DevServer" => "dev server",
        "Watcher" => "watcher",
        "BuildTool" => "build tool",
        "TerminalShell" => "terminal shell",
        "Harness" => "AI tool",
        _ => "",
    }
}

/// Role guess, pass2 §5 vocabulary — inferred from exe/cmdline, always
/// shown as a guess, never as fact. Pure (unit-tested).
#[must_use]
pub fn guess_role(exe: &str, cmdline: &str) -> &'static str {
    let exe_name = exe.rsplit('/').next().unwrap_or(exe).to_ascii_lowercase();
    let line = cmdline.to_ascii_lowercase();
    let hay = format!("{exe_name} {line}");
    let contains_any = |needles: &[&str]| needles.iter().any(|n| hay.contains(n));
    // AJ-E1704: AI Juggler's own agent bridge is an MCP server owned by
    // its session (so never cleaned up or offered alone for a kill).
    let bridge = juggler_agentbridge::wire::PROGRAM;
    if exe_name == bridge
        || line.contains(&format!("/{bridge} --mcp"))
        || contains_any(&["mcp-server", "mcp_server", "/mcp ", "mcp.json"])
    {
        "McpServer"
    } else if contains_any(&[
        "postgres",
        "mysqld",
        "mariadbd",
        "redis-server",
        "mongod",
        "memcached",
        "sqlite3",
    ]) {
        "Database"
    } else if contains_any(&[
        "vite",
        "webpack",
        "esbuild",
        "next dev",
        "--watch-tags",
        "dev-server",
        "serve -s",
        "http.server",
    ]) {
        "DevServer"
    } else if contains_any(&["watchman", "fswatch", "chokidar", "nodemon", "--watch"]) {
        "Watcher"
    } else if contains_any(&[
        "cargo build",
        "make -",
        "tsc ",
        "rustc",
        "clang",
        "esbuild --build",
    ]) {
        "BuildTool"
    } else if exe_name == "sh"
        || exe_name == "bash"
        || exe_name == "zsh"
        || exe_name == "fish"
        || exe_name.ends_with("node") && line.contains("-e")
    {
        "TerminalShell"
    } else if contains_any(juggler_adapters::HARNESS_PROCESS_HINTS) {
        "Harness"
    } else {
        "Unknown"
    }
}

/// Click-time identity verification — ONE rule shared by the sheet
/// (step 1) and the execute gate (step 3) so they can never
/// disagree. Helpers carry the kernel-recorded exe (the full
/// boot/start-time/exe triple); LIVE-supervised roots verify via the
/// handle's settled identity with start-time only — the exe moves
/// legitimately across `exec` (E503/E504's `/bin/sh`-vs-`/bin/bash`
/// aliasing rule). Anything else (no record, probe error) fails
/// CLOSED.
pub(crate) fn verify_kill_target(
    state: &Arc<AppState>,
    session: &SessionId,
    row: &juggler_core::storage::LedgerRow,
) -> bool {
    let Some(pid) = row.pid else {
        return false;
    };
    // AJ-E511 Req.2 — the machine gate BEFORE any kernel probe: a
    // foreign-machine row is refused by the TYPED WrongMachine path
    // (kill_verify_inner / kill_execute_as); the sweep already skips
    // foreign rows at candidate read (audit-note-only, never a
    // signal).
    if row.machine_id != local_machine_id(state).as_str() {
        return false;
    }
    if row.role == "root"
        && let Some(identity) = state.supervisor.root_identity(session)
    {
        return juggler_supervisor::tree::verify_start_time(pid, &identity)
            == juggler_supervisor::tree::Verification::Verified;
    }
    // A helper young enough to still be settling (caught between fork
    // and exec) is verified on boot + start time; its exe leg waits out
    // the window, as the reconciler's listing verdict does (AJ-E508).
    let settling = row.role == "helper"
        && juggler_core::model::Timestamp::now()
            .millis()
            .saturating_sub(row.first_seen)
            < juggler_core::storage::EXEC_SETTLE_MS;
    row.started_at.is_some_and(|start_time| {
        let identity = juggler_supervisor::tree::ProcessIdentity {
            pid,
            pgid: row.pgid,
            start_time,
            exe: std::path::PathBuf::from(&row.exe_path),
            boot_id: row.boot_id.clone(),
        };
        juggler_supervisor::tree::verify_settling(pid, &identity, settling)
            == juggler_supervisor::tree::Verification::Verified
    })
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// [`AppError::NotFound`] when no ledger row exists for
/// `(session, pid)`.
#[allow(clippy::similar_names, reason = "pid/pgid are the kernel's own names")]
pub fn kill_verify_inner(
    state: &Arc<AppState>,
    session_id: &str,
    pid: u32,
) -> Result<KillVerifyInfo, AppError> {
    use juggler_supervisor::tree::LedgerStore as _;
    let session = SessionId::new(session_id.to_string());
    let rows = state.ledger.session_rows(&session).unwrap_or_default();

    let row = rows
        .iter()
        .find(|row| row.pid == Some(pid))
        .ok_or_else(|| AppError::NotFound {
            entity: "ledger row",
            id: format!("{session_id}/{pid}"),
        })?;
    // AJ-E511 Req.2 — the typed machine gate first: a foreign row
    // gets its OWN refusal copy (not the identity one) and never
    // reaches a kernel probe or a member listing.
    if row.machine_id != local_machine_id(state).as_str() {
        return Ok(KillVerifyInfo {
            machine_id: row.machine_id.clone(),
            pid,
            session: session_id.to_string(),
            verified: false,
            refusal: Some(KILL_WRONG_MACHINE_COPY.to_string()),
            exe_path: row.exe_path.clone(),
            cmdline: truncate_cmdline(&row.cmdline_trunc),
            cwd: row.cwd.clone(),
            ports: Vec::new(),
            pgid: None,
            escaped_group: row.escaped_group,
            members: Vec::new(),
            tree_pids: Vec::new(),
            platform: juggler_supervisor::ladder_platform().to_string(),
            grace_secs: KILL_GRACE_SECS,
            provider: row.provider.clone(),
            account_id: row.account_id.clone(),
            project_id: row.project_id.clone(),
        });
    }
    // Click-time verification: identity against the kernel NOW —
    // the shared rule (`verify_kill_target`). Unverifiable rows
    // (state `unverifiable`, a missing start-time, a probe error)
    // fail CLOSED — the sheet explains.
    let verified = verify_kill_target(state, &session, row);
    // Re-read NOW (the live facts; recorded values when unreadable).
    let (cmdline, cwd) = juggler_supervisor::live_details(pid).map_or_else(
        || (row.cmdline_trunc.clone(), row.cwd.clone()),
        |(cmd, cwd)| {
            (
                if cmd.is_empty() {
                    row.cmdline_trunc.clone()
                } else {
                    cmd
                },
                if cwd.is_empty() { row.cwd.clone() } else { cwd },
            )
        },
    );
    let ports = os_listeners()
        .map(|listeners| {
            listeners
                .iter()
                .filter(|listener| listener.pid == pid)
                .map(|listener| listener.port)
                .collect::<Vec<u16>>()
        })
        .unwrap_or_default();
    let pgid = juggler_supervisor::group::pgid_of(pid).ok();
    let (members, tree_pids) = if verified {
        kill_collateral(&rows, pgid)
    } else {
        (Vec::new(), Vec::new())
    };
    Ok(KillVerifyInfo {
        machine_id: row.machine_id.clone(),
        pid,
        session: session_id.to_string(),
        verified,
        refusal: (!verified).then(|| unverified_copy(pid).to_string()),
        exe_path: row.exe_path.clone(),
        cmdline: truncate_cmdline(&cmdline),
        cwd,
        ports,
        pgid,
        escaped_group: row.escaped_group,
        members,
        tree_pids,
        platform: juggler_supervisor::ladder_platform().to_string(),
        grace_secs: KILL_GRACE_SECS,
        provider: row.provider.clone(),
        account_id: row.account_id.clone(),
        project_id: row.project_id.clone(),
    })
}

/// The PGID collateral list + the whole-tree enumeration (the verify
/// sheet's scope data). Ledger rows give attribution; strangers are
/// named as strangers (Req.5's shared-group warning data).
#[allow(clippy::similar_names, reason = "pid/pgid are the kernel's own names")]
fn kill_collateral(
    rows: &[juggler_core::storage::LedgerRow],
    pgid: Option<u32>,
) -> (Vec<KillMemberInfo>, Vec<u32>) {
    let ledger_by_pid: std::collections::HashMap<u32, &juggler_core::storage::LedgerRow> = rows
        .iter()
        .filter_map(|row| row.pid.map(|pid| (pid, row)))
        .collect();
    let mut members = Vec::new();
    if let Some(pgid) = pgid {
        for member_pid in juggler_supervisor::pgid_members(pgid) {
            let ledger_row = ledger_by_pid.get(&member_pid).copied();
            let (member_exe, member_cmd) = ledger_row
                .map(|row| (row.exe_path.clone(), row.cmdline_trunc.clone()))
                .or_else(|| {
                    juggler_supervisor::live_details(member_pid)
                        .map(|(cmd, _)| ("(unattributed)".into(), cmd))
                })
                .unwrap_or_else(|| ("(unattributed)".into(), String::new()));
            let role = guess_role(&member_exe, &member_cmd);
            members.push(KillMemberInfo {
                pid: member_pid,
                pgid: juggler_supervisor::group::pgid_of(member_pid).ok(),
                exe: member_exe.rsplit('/').next().unwrap_or("?").to_string(),
                cmdline: truncate_cmdline(&member_cmd),
                role_guess: role_words(role),
                unsaved_state_warning: matches!(role, "McpServer" | "TerminalShell" | "Database")
                    .then(|| unsaved_state_copy(role)),
                from_ledger: ledger_row.is_some(),
            });
        }
    }
    // Scope 3's enumeration: the Instance tree's live pids, each once,
    // and only those still running as the process the row recorded (a
    // row whose exit isn't confirmed yet names no process to stop).
    let mut tree_pids: Vec<u32> = rows
        .iter()
        .filter(|row| row.state == "running" || row.state == "orphaned")
        .filter(|row| still_as_recorded(row))
        .filter_map(|row| row.pid)
        .collect();
    tree_pids.sort_unstable();
    tree_pids.dedup();
    (members, tree_pids)
}

/// Display truncation for command lines (200 chars, the ledger's own
/// convention).
pub(crate) fn truncate_cmdline(cmdline: &str) -> String {
    if cmdline.chars().count() > 200 {
        let truncated: String = cmdline.chars().take(197).collect();
        format!("{truncated}...")
    } else {
        cmdline.to_string()
    }
}

/// `kill_execute` with an injected audit writer (the guardrail
/// gate's seam; production passes the real repo).
///
/// # Errors
///
/// As [`kill_execute`](crate::features::kill::commands::kill_execute).
pub fn kill_execute_audited(
    state: &Arc<AppState>,
    audit: &dyn AuditWriter,
    session_id: &str,
    pid: u32,
    scope: &str,
    cancel_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    goal_port: Option<u16>,
) -> Result<KillExecuteInfo, AppError> {
    kill_execute_as(
        state,
        audit,
        "user:kill_sheet",
        session_id,
        pid,
        scope,
        cancel_flag,
        goal_port,
    )
}

/// `kill_execute` with an explicit audit ACTOR — the automatic tiers'
/// entry (AJ-E508): actor `auto:tier-1` / `auto:tier-2` per the E509
/// vocabulary. Same guardrails, same ladder, same audit floor.
///
/// # Errors
///
/// As [`kill_execute`](crate::features::kill::commands::kill_execute).
#[allow(
    clippy::too_many_arguments,
    reason = "the audit actor rides the E507 kill path's shape"
)]
pub fn kill_execute_as(
    state: &Arc<AppState>,
    audit: &dyn AuditWriter,
    actor: &str,
    session_id: &str,
    pid: u32,
    scope: &str,
    cancel_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    goal_port: Option<u16>,
) -> Result<KillExecuteInfo, AppError> {
    use juggler_supervisor::tree::LedgerStore as _;
    let session = SessionId::new(session_id.to_string());
    let rows = state.ledger.session_rows(&session).unwrap_or_default();
    let row = rows
        .iter()
        .find(|row| row.pid == Some(pid))
        .ok_or_else(|| AppError::NotFound {
            entity: "ledger row",
            id: format!("{session_id}/{pid}"),
        })?;
    // Guardrail 0 — the machine gate (AJ-E511 Req.2): a foreign row
    // gets the TYPED WrongMachine refusal with zero signals, before
    // any identity probe (which would misleadingly read as a
    // PID-reuse refusal).
    if row.machine_id != local_machine_id(state).as_str() {
        return Ok(refused_kill(pid, KILL_WRONG_MACHINE_COPY));
    }
    // AJ-E708 Req.5: a diverged Instance takes no destructive action
    // until the user picks what to keep.
    if crate::crossmachine::switch_back::is_diverged(state, session_id) {
        return Ok(refused_kill(pid, KILL_DIVERGED_COPY));
    }
    // Guardrail 1 — SAME-TICK re-verification (E503 against the
    // kernel NOW, not the sheet's earlier verdict; the SAME shared
    // rule the sheet used): a mismatched or unverifiable anchor gets
    // ZERO signals and the exact copy.
    let verified = verify_kill_target(state, &session, row);
    if !verified {
        crate::usage_stats::count(
            "leftover.verify_refused",
            &[("reason", "ownership_unverified")],
        );
        return Ok(KillExecuteInfo {
            refusal: Some(unverified_copy(pid).to_string()),
            members: vec![KillMemberOutcome {
                pid,
                outcome: "refused".into(),
            }],
            cancelled: false,
            all_succeeded: false,
            still_holding: Vec::new(),
            reoffer_pid: None,
        });
    }
    // Scope enumeration (Req.1's three scopes; the UI defaults to
    // the narrowest that covers the goal).
    let candidates = scope_members(scope, &rows, pid)?;
    // The port facts: the goal port when the user stated one (the
    // port that opened the sheet — it may be held by an ESCAPED
    // sibling, which is exactly the honest-still-holding case),
    // else whatever the target itself holds.
    let member_ports = ports_held_by(pid);
    let ports_before = match goal_port {
        Some(port) => Some(port.to_string()),
        None => member_ports.clone(),
    };
    // Guardrail 2 — NO AUDIT, NO KILL: the intent row names the real
    // target (scope, pgid, attribution), written BEFORE any signal.
    let record = kill_intent_record(
        state,
        row,
        session_id,
        pid,
        scope,
        ports_before.as_deref(),
        member_ports.as_deref(),
        actor,
    );
    let intent_id = audit
        .intent(&record)
        .map_err(|_| AppError::AuditUnavailable)?;
    // ADR-0010 verify-or-refuse, per MEMBER, the last thing before the
    // first signal: every other process the scope names passes the same
    // identity check as the target, or is left alone and reported (a
    // reused pid, a process AI Juggler didn't start).
    let (members, left_alone) = verified_members(&rows, pid, candidates, |row| {
        verify_kill_target(state, &session, row)
    });
    // AJ-E1103: memory the members hold now (freed if they go).
    let rss_before = crate::usage_stats::rss_of(state, &members);
    // Guardrail 3 — the ladder itself (cancellable through grace).
    let cancel = std::sync::Arc::clone(cancel_flag);
    let outcome = juggler_supervisor::terminate_group(
        // AJ-E512: settings-backed (default the spec's 5 s) so the
        // collateral suites can widen it on loaded machines — a starved
        // fixture's TERM handler must land inside the grace window.
        std::time::Duration::from_secs(setting_u64(state, "kill_grace_secs", KILL_GRACE_SECS)),
        move || members,
        move || cancel.load(std::sync::atomic::Ordering::Relaxed),
    );
    // The honest post-ladder port re-probe (C2): report what is
    // STILL held and by whom — including an escaped member that keeps
    // the port (then re-offer pid-only after fresh verification).
    let (still_holding, reoffer_pid) =
        still_holding_report(ports_before.as_deref(), pid, outcome.cancelled);
    // Audit completion: a CANCELLED ladder leaves the intent pending
    // ("TERM'd, outcome unknown").
    if !outcome.cancelled {
        complete_kill_audit(audit, intent_id, &outcome, &left_alone, &still_holding);
    }
    if outcome.all_succeeded() && !outcome.cancelled {
        let held: Vec<u16> = still_holding.iter().map(|held| held.port).collect();
        crate::usage_stats::leftover_killed(row, actor, ports_before.as_deref(), &held, rss_before);
    }
    let members = outcome
        .members
        .iter()
        .map(|(pid, label)| KillMemberOutcome {
            pid: *pid,
            outcome: label.as_str().to_string(),
        })
        .chain(left_alone)
        .collect();
    Ok(KillExecuteInfo {
        refusal: None,
        members,
        cancelled: outcome.cancelled,
        all_succeeded: outcome.all_succeeded(),
        still_holding,
        reoffer_pid,
    })
}

/// Completes a ladder's audit row: each pid's outcome (the ones a wider
/// stop left alone included) and the ports still held after it.
fn complete_kill_audit(
    audit: &dyn AuditWriter,
    intent_id: i64,
    outcome: &juggler_supervisor::tree::LadderOutcome,
    left_alone: &[KillMemberOutcome],
    still_holding: &[StillHoldingInfo],
) {
    let vocabulary = outcome
        .members
        .iter()
        .map(|(pid, outcome)| format!("pid {pid}: {}", outcome.as_str()))
        .chain(
            left_alone
                .iter()
                .map(|left| format!("pid {}: {}", left.pid, left.outcome)),
        )
        .collect::<Vec<_>>()
        .join("; ");
    let ports_after = still_holding
        .iter()
        .map(|held| held.port.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let ports_after = (!ports_after.is_empty()).then_some(ports_after);
    // Best-effort (E509's floor is the intent row itself).
    let _ = audit.complete(intent_id, &vocabulary, ports_after.as_deref());
}

/// The kill sheet's audit intent row (Req.6 / AJ-E509 Req.2): the
/// full attribution from the ledger row, the verified-at-click-time
/// identity note, and the goal-port-aware before/after facts.
#[allow(clippy::similar_names, reason = "pid/pgid are the kernel's own names")]
#[allow(
    clippy::too_many_arguments,
    reason = "mirrors the audit record's own columns"
)]
pub(crate) fn kill_intent_record(
    state: &Arc<AppState>,
    row: &juggler_core::storage::LedgerRow,
    session_id: &str,
    pid: u32,
    scope: &str,
    ports_before: Option<&str>,
    member_ports: Option<&str>,
    actor: &str,
) -> juggler_core::storage::CleanupAuditRecord {
    juggler_core::storage::CleanupAuditRecord {
        id: 0,
        at_ms: now_millis(),
        actor: actor.to_string(),
        pid,
        pgid: juggler_supervisor::group::pgid_of(pid).ok(),
        exe_path: row.exe_path.clone(),
        cmdline_hash: row.cmdline_hash.clone(),
        cmdline_trunc: truncate_cmdline(&row.cmdline_trunc),
        session_id: session_id.to_string(),
        provider: row.provider.clone(),
        account_id: row.account_id.clone(),
        project_id: row.project_id.clone(),
        machine_id: local_machine_id(state).as_str().to_string(),
        identity: format!(
            "E503 verified at click time (start {}, exe {:?})",
            row.started_at.map_or("?".to_string(), |t| t.to_string()),
            row.exe_path
        ),
        scope: scope.to_string(),
        outcome: String::new(),
        ports_before: ports_before
            .map(str::to_string)
            .or_else(|| member_ports.map(str::to_string)),
        ports_after: None,
    }
}

/// The three scopes' member enumeration (Req.1): `pid` — the one
/// verified process; `pgid` — every live member of its process group;
/// `tree` — the Instance's live ledger rows. Candidates only: each is
/// checked by [`verified_members`] before any signal.
///
/// # Errors
///
/// [`AppError::Validation`] for an unknown scope, or for `pgid` when
/// the group is unreadable (the honest narrower-scope prompt).
#[allow(clippy::similar_names, reason = "pid/pgid are the kernel's own names")]
fn scope_members(
    scope: &str,
    rows: &[juggler_core::storage::LedgerRow],
    pid: u32,
) -> Result<Vec<u32>, AppError> {
    match scope {
        "pid" => Ok(vec![pid]),
        "pgid" => {
            let pgid =
                juggler_supervisor::group::pgid_of(pid).map_err(|_| AppError::Validation {
                    subject: "scope",
                    message: "the process group is unreadable — choose `This process only`".into(),
                })?;
            Ok(juggler_supervisor::pgid_members(pgid))
        }
        "tree" => Ok(rows
            .iter()
            .filter(|row| row.pid.is_some() && (row.state == "running" || row.state == "orphaned"))
            .filter_map(|row| row.pid)
            .collect()),
        other => Err(AppError::Validation {
            subject: "scope",
            message: format!("unknown scope {other:?} (pid | pgid | tree)"),
        }),
    }
}

/// A stop's outcome for a group member AI Juggler didn't start: never
/// signalled (ADR-0010: not in the ledger, never touched).
pub const LEFT_NOT_OURS: &str = "left alone: not started by AI Juggler";

/// A stop's outcome for a member whose record no longer matches a live
/// process it can confirm (start time, program, boot): never signalled.
pub const LEFT_UNVERIFIED: &str = "left alone: couldn't confirm it";

/// ADR-0010 verify-or-refuse for every member of a stop, not only its
/// target: `candidates` (a scope's enumeration) split into the pids to
/// signal and the ones left alone, each with why. `anchor` was verified
/// by the caller. Any other pid is signalled only when a ledger row for
/// it passes `verifies` (the shared click-time rule,
/// [`verify_kill_target`]). One whose recorded process is no longer the
/// live one (it ended; its pid may be another program's now) reads
/// `already gone`; a live process with no ledger row is
/// [`LEFT_NOT_OURS`]; a row the check can't confirm, [`LEFT_UNVERIFIED`].
pub(crate) fn verified_members(
    rows: &[juggler_core::storage::LedgerRow],
    anchor: u32,
    candidates: Vec<u32>,
    verifies: impl Fn(&juggler_core::storage::LedgerRow) -> bool,
) -> (Vec<u32>, Vec<KillMemberOutcome>) {
    let mut signal = Vec::new();
    let mut left = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for pid in candidates {
        if !seen.insert(pid) {
            continue;
        }
        if pid == anchor {
            signal.push(pid);
            continue;
        }
        let recorded: Vec<&juggler_core::storage::LedgerRow> =
            rows.iter().filter(|row| row.pid == Some(pid)).collect();
        if recorded.iter().any(|row| verifies(row)) {
            signal.push(pid);
            continue;
        }
        let outcome = if recorded.is_empty() {
            LEFT_NOT_OURS
        } else if recorded.iter().any(|row| still_as_recorded(row)) {
            LEFT_UNVERIFIED
        } else {
            juggler_supervisor::tree::ProcessOutcome::AlreadyGone.as_str()
        };
        left.push(KillMemberOutcome {
            pid,
            outcome: outcome.to_string(),
        });
    }
    (signal, left)
}

/// The process a ledger row recorded still runs at its pid (same start
/// time, not a zombie) — whether or not the full check confirms it.
fn still_as_recorded(row: &juggler_core::storage::LedgerRow) -> bool {
    row.pid.zip(row.started_at).is_some_and(|(pid, start)| {
        crate::features::processes::stop::read_now(pid)
            .is_some_and(|now| now.start_time == start && !now.zombie)
    })
}

/// The post-ladder port re-probe (FMEA C2): who STILL holds each of
/// the target's ports, and the pid-only re-offer when an escapee —
/// not the killed pid — keeps one. Never a success claim.
pub(crate) fn still_holding_report(
    ports_before: Option<&str>,
    pid: u32,
    cancelled: bool,
) -> (Vec<StillHoldingInfo>, Option<u32>) {
    let target_ports: Vec<u16> = ports_before
        .unwrap_or("")
        .split(',')
        .filter_map(|port| port.parse::<u16>().ok())
        .collect();
    let mut still_holding = Vec::new();
    if let Ok(listeners) = os_listeners() {
        for port in &target_ports {
            for listener in listeners.iter().filter(|l| l.port == *port) {
                let exe = holder_exe(listener.pid);
                still_holding.push(StillHoldingInfo {
                    port: *port,
                    pid: listener.pid,
                    exe,
                });
            }
        }
    }
    // The re-offer only after a COMPLETED ladder (a cancelled one
    // reports the facts and leaves the choice entirely to the user).
    let reoffer_pid = if cancelled {
        None
    } else {
        still_holding
            .iter()
            .find(|held| held.pid != pid)
            .map(|held| held.pid)
    };
    (still_holding, reoffer_pid)
}

/// The still-holding report's holder exe (basename of the live
/// command's first token — `Listeners` carry no exe by design).
fn holder_exe(pid: u32) -> String {
    juggler_supervisor::live_details(pid).map_or_else(
        || "?".to_string(),
        |(cmdline, _)| {
            cmdline
                .split_whitespace()
                .next()
                .unwrap_or("?")
                .rsplit('/')
                .next()
                .unwrap_or("?")
                .to_string()
        },
    )
}

#[cfg(test)]
mod verified_members_tests {
    use juggler_core::storage::LedgerRow;

    use super::{LEFT_NOT_OURS, LEFT_UNVERIFIED, verified_members};

    fn row(pid: u32, started_at: i64) -> LedgerRow {
        LedgerRow {
            pid: Some(pid),
            pgid: None,
            started_at: Some(started_at),
            exe_path: "/bin/sleep".into(),
            cmdline_hash: "h".into(),
            cmdline_trunc: "sleep".into(),
            boot_id: "boot".into(),
            session_id: "s".into(),
            instance_id: None,
            provider: "tool".into(),
            account_id: None,
            project_id: None,
            machine_id: "this-mac".into(),
            root_pid: None,
            parent_pid: None,
            depth: 1,
            role: "helper".into(),
            state: "orphaned".into(),
            listens_on: None,
            cwd: String::new(),
            peak_rss: None,
            peak_cpu: None,
            exit_kind: None,
            reaped_by_us: None,
            escaped_group: false,
            down_period: false,
            first_seen: 0,
            last_seen: 0,
        }
    }

    /// The anchor (verified by the caller) and members whose rows pass the
    /// check are signalled; a group member with no ledger row, a row whose
    /// process ended, and a live process the check can't confirm are left
    /// alone, each with why.
    #[test]
    fn only_members_that_pass_the_check_are_signalled() {
        let own = std::process::id();
        let own_start = crate::features::processes::stop::read_now(own)
            .expect("this test's own process is readable")
            .start_time;
        // Pids far above any real one: nothing runs there.
        let (anchor, ok, ended, stranger) = (4_000_001, 4_000_002, 4_000_003, 4_000_004);
        let rows = vec![
            row(anchor, 1),
            row(ok, 2),
            row(ended, 3),
            // Running as recorded, yet the full check fails.
            row(own, own_start),
        ];
        let verifies = |row: &LedgerRow| row.pid == Some(ok);
        let (signal, left) = verified_members(
            &rows,
            anchor,
            vec![anchor, ok, ended, stranger, own, ok],
            verifies,
        );
        assert_eq!(
            signal,
            vec![anchor, ok],
            "only the anchor and the verified member"
        );
        let said: Vec<(u32, &str)> = left
            .iter()
            .map(|left| (left.pid, left.outcome.as_str()))
            .collect();
        assert_eq!(
            said,
            vec![
                (ended, "already gone"),
                (stranger, LEFT_NOT_OURS),
                (own, LEFT_UNVERIFIED),
            ],
            "each member left alone says why"
        );
    }
}
