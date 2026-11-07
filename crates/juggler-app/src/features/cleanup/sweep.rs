//! D-08, AJ-E508: the automatic cleanup sweep: Tier 1 for opted-in accounts,
//! Tier 2's notices with their Undo window.

use std::sync::Arc;

use juggler_core::model::SessionId;

use crate::commands::{local_machine_id, now_millis};
use crate::features::cleanup::dto::Tier2Notice;
use crate::features::kill::{
    guess_role, kill_execute_as, kill_intent_record, truncate_cmdline, verify_kill_target,
};
use crate::features::port::{hog_facts, local_ledger_rows};
use crate::state::AppState;

/// The Undo window's length (Req.1: ≥30 s — never shorter).
pub const TIER2_UNDO_WINDOW_MS: i64 = 30_000;

/// Tier 2's crash grace before the notification (Req.1: 5 min).
pub const TIER2_CRASH_GRACE_MS: i64 = 5 * 60_000;

/// The automatic tiers' sweep (AJ-E508; replaces the E410 prototype).
/// Tier 0 by default: without BOTH the Account opt-in and the tier
/// evaluator's eligibility, nothing is ever signalled. The candidate
/// source is the LEDGER's orphaned rows (the E504 reconciler's
/// verdict — the same authority the Cleanup Centre lists; the E410
/// tracker deliberately drops clean exits and cannot serve Tier 1).
/// Gates, in order: the global pause; the 24 h crash suspension; the
/// per-candidate evaluator (roles/roots/containment/machine/exit
/// kind); pacing (the auto grace; Tier 2's 5 min + ONE notification +
/// the Undo window); same-tick re-verification (a failure DEMOTES to
/// Tier 0 with an audit note); then the E507 ladder behind an E509
/// audit intent (actor `auto:tier-N`), via [`kill_execute_as`] — the
/// only kill surface, manual or automatic.
#[allow(
    clippy::too_many_lines,
    reason = "one pass per gate; splitting hides the ordering that IS the law"
)]
pub fn auto_cleanup_sweep(state: &Arc<AppState>, now: i64) {
    if cleanup_paused(state) || cleanup_suspended_until(state).is_some() {
        return;
    }
    let machine = local_machine_id(state);
    // AJ-E512 Req.3: a hog is NEVER auto-signalled at any tier —
    // Tier 3 is alert + staged sheet only (E508), so the sweep stands
    // down for hog-flagged pids (and hog-flagged trees) entirely; the
    // manual E507 sheet stays the only path.
    // Hog facts ride the REAL clock (`now` here is the sweep's
    // SCHEDULE clock — often minutes in the future in tests, which
    // would empty the 60 s ring window and un-hog everything).
    let hog_facts_now = hog_facts(state, now_millis());
    let hog_pids: std::collections::HashSet<u32> = hog_facts_now
        .iter()
        .filter(|hog| hog.pid != 0)
        .map(|hog| hog.pid)
        .collect();
    let hog_sessions: std::collections::HashSet<String> = hog_facts_now
        .iter()
        .map(|hog| hog.session.clone())
        .collect();
    // One candidates read: every ORPHANED row of this machine's
    // sessions, grouped by session with its root row (the exit-kind
    // source and the pacing clock).
    let mut by_session: std::collections::HashMap<
        String,
        (
            Vec<juggler_core::storage::LedgerRow>,
            Option<juggler_core::storage::LedgerRow>,
        ),
    > = std::collections::HashMap::new();
    // Each session's rows, every role and state: what says a group was
    // made by the session's own helper (ADR-0010, its AJ-E2405 amendment).
    let mut session_rows: std::collections::HashMap<String, Vec<juggler_core::storage::LedgerRow>> =
        std::collections::HashMap::new();
    for row in local_ledger_rows(state) {
        if row.machine_id != machine.as_str() {
            continue;
        }
        session_rows
            .entry(row.session_id.clone())
            .or_default()
            .push(row.clone());
        let entry = by_session.entry(row.session_id.clone()).or_default();
        if row.role == "root" {
            entry.1 = Some(row);
        } else if row.state == "orphaned" {
            entry.0.push(row);
        }
    }
    for (session_id, (candidates, root)) in by_session {
        let Some(root_row) = root else { continue };
        // The root's last tick while alive ≈ its exit time (its
        // last_seen stops refreshing at death).
        let parent_exit_ms = root_row.last_seen;
        if now - parent_exit_ms < juggler_supervisor::AUTO_CLEANUP_GRACE_MS {
            continue;
        }
        // The root's exit kind decides the TIER, never the
        // candidate's presence: code 0 → Tier 1's premise; anything
        // else → crash (Tier 2's); unknown → observe only (fail
        // closed).
        let root_exit_clean = root_row.exit_kind.as_deref().map(|kind| {
            match kind.strip_prefix("code:") {
                Some(code) => code == "0",
                // A signal death or an unparsable kind is a crash.
                None => false,
            }
        });
        let settings = account_tier_settings(state, &SessionId::new(session_id.clone()));
        for row in candidates {
            let Some(orphan_pid) = row.pid else { continue };
            if hog_pids.contains(&orphan_pid) || hog_sessions.contains(&session_id) {
                continue; // hog: alert + staged sheet only, never a signal
            }
            // AJ-E2404: the user said Keep (not a leftover): never tidied.
            if state
                .processes
                .is_kept(orphan_pid, row.started_at.unwrap_or_default())
            {
                continue;
            }
            let candidate = juggler_supervisor::tree::TierCandidate {
                pid: orphan_pid,
                role_guess: guess_role(&row.exe_path, &row.cmdline_trunc).to_string(),
                is_root: false,
                state: row.state.clone(),
                escaped_group: row.escaped_group,
                group_is_sessions: juggler_supervisor::tree::group_is_sessions(
                    &row,
                    session_rows.get(&session_id).map_or(&[][..], Vec::as_slice),
                ),
                root_exit_clean,
                machine_local: true,
                tier1_opt_in: settings.tier1_opt_in,
                tier2_opt_in: settings.tier2_opt_in,
            };
            let tier = match juggler_supervisor::tree::evaluate_tier(&candidate) {
                juggler_supervisor::tree::TierVerdict::Signal(tier) => tier,
                juggler_supervisor::tree::TierVerdict::ObserveOnly(_) => continue,
            };
            // Tier 2 pacing: 5 min after the crash AND one
            // notification AND an elapsed Undo window.
            if tier == 2 {
                if now - parent_exit_ms < TIER2_CRASH_GRACE_MS {
                    continue;
                }
                let orphan = juggler_supervisor::Orphan {
                    session: SessionId::new(session_id.clone()),
                    root_pid: root_row.pid.unwrap_or(0),
                    parent_label: root_row.exe_path.clone(),
                    pid: orphan_pid,
                    name: row.exe_path.clone(),
                    rss_bytes: 0,
                    parent_exit_ms,
                };
                match tier2_window(state, &orphan, &row, now) {
                    Tier2Window::Open | Tier2Window::Kept => continue,
                    Tier2Window::Elapsed => (),
                }
            }
            // Same-tick re-verification — a failure DEMOTES to Tier 0
            // (list-only) with an audit note, never a signal.
            let session = SessionId::new(session_id.clone());
            if !verify_kill_target(state, &session, &row) {
                demote_with_audit_note(state, &row, tier);
                continue;
            }
            let actor = if tier == 1 {
                juggler_core::storage::ACTOR_AUTO_TIER1
            } else {
                juggler_core::storage::ACTOR_AUTO_TIER2
            };
            let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let outcome = kill_execute_as(
                state,
                state.audit.as_ref(),
                actor,
                session_id.as_str(),
                orphan_pid,
                "pid",
                &flag,
                None,
            );
            if let Ok(outcome) = outcome
                && let Some(journal) = state.supervisor.journal_ref()
            {
                let _ = journal.record_killed(
                    &session,
                    orphan_pid,
                    if tier == 1 {
                        "auto-tier1"
                    } else {
                        "auto-tier2"
                    },
                    &outcome
                        .members
                        .iter()
                        .map(|member| member.outcome.as_str())
                        .collect::<Vec<_>>()
                        .join("; "),
                );
            }
        }
    }
}

enum Tier2Window {
    /// Created now or still counting — no signal this tick.
    Open,
    /// The user chose Undo / Keep running — never signalled.
    Kept,
    /// The window elapsed without an Undo — eligible to signal.
    Elapsed,
}

fn tier2_window(
    state: &Arc<AppState>,
    orphan: &juggler_supervisor::Orphan,
    row: &juggler_core::storage::LedgerRow,
    now: i64,
) -> Tier2Window {
    let Ok(mut pending) = state.tier2_pending.lock() else {
        return Tier2Window::Open; // fail closed: no notice, no signal
    };
    match pending.get(&orphan.pid) {
        Some(notice) if notice.keep => Tier2Window::Kept,
        Some(notice) => {
            if now < notice.deadline_ms {
                Tier2Window::Open
            } else {
                pending.remove(&orphan.pid);
                Tier2Window::Elapsed
            }
        }
        None => {
            // Exactly one notification per crash-orphan (Req.1).
            pending.insert(
                orphan.pid,
                Tier2Notice {
                    pid: orphan.pid,
                    session: orphan.session.as_str().to_string(),
                    cmdline_trunc: truncate_cmdline(&row.cmdline_trunc),
                    shown_at_ms: now,
                    deadline_ms: now + TIER2_UNDO_WINDOW_MS,
                    keep: false,
                },
            );
            Tier2Window::Open
        }
    }
}

/// A failed same-tick re-verification: the candidate is silently
/// demoted to Tier 0 (list-only) WITH an audit note — the E509 log
/// carries intent + `demoted` completion so the History tells the
/// truth about what was considered and why nothing was signalled.
fn demote_with_audit_note(state: &Arc<AppState>, row: &juggler_core::storage::LedgerRow, tier: u8) {
    let actor = if tier == 1 {
        juggler_core::storage::ACTOR_AUTO_TIER1
    } else {
        juggler_core::storage::ACTOR_AUTO_TIER2
    };
    let record = kill_intent_record(
        state,
        row,
        &row.session_id,
        row.pid.unwrap_or(0),
        "pid",
        None,
        None,
        actor,
    );
    if let Ok(id) = state.audit.intent(&record) {
        let _ = state.audit.complete(
            id,
            "demoted — re-verification failed, listed only (Tier 0)",
            None,
            now_millis(),
        );
    }
}

/// True when the global `Pause automatic cleanup` kill-switch is on
/// (Req.4): the sweep stands down within one tick of the flag.
pub(crate) fn cleanup_paused(state: &Arc<AppState>) -> bool {
    let Ok(storage) = state.storage.lock() else {
        return true; // a lock failure pauses — never signals
    };
    storage
        .app_settings()
        .get("cleanup_paused")
        .ok()
        .flatten()
        .is_some_and(|value| value == "1")
}

/// The 24 h crash-suspension deadline (Req.5), if active: Tiers 1-2
/// stand down until the user re-confirms in the Cleanup Centre.
pub(crate) fn cleanup_suspended_until(state: &Arc<AppState>) -> Option<i64> {
    let storage = state.storage.lock().ok()?;
    let until = storage
        .app_settings()
        .get("cleanup_suspended_until")
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok())?;
    (until > now_millis()).then_some(until)
}

/// Resolves an Instance's Account tier settings (session → launch
/// settings → account). Unknown links read as both-off (fail closed).
fn account_tier_settings(
    state: &Arc<AppState>,
    session: &SessionId,
) -> juggler_core::storage::CleanupTierSettings {
    let account = (|| {
        let storage = state.storage.lock().ok()?;
        let record = storage.sessions().get(session).ok()?;
        let recipe = storage.profiles().get(&record.profile).ok()?;
        storage.accounts().get(&recipe.account).ok()
    })();
    let account_id = account.as_ref().map(|a| a.id.as_str().to_string());
    match account_id {
        Some(account_id) => state.cleanup_settings.get(&account_id).unwrap_or_default(),
        None => juggler_core::storage::CleanupTierSettings::default(),
    }
}

/// Reads a u64 app setting with a default (AJ-E512 test seams for
/// the hog/idle thresholds; production defaults are the arguments).
pub(crate) fn setting_u64(state: &Arc<AppState>, key: &str, default: u64) -> u64 {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(key).ok().flatten())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

/// Reads an f64 app setting with a default (same seam as
/// [`setting_u64`]).
pub(crate) fn setting_f64(state: &Arc<AppState>, key: &str, default: f64) -> f64 {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(key).ok().flatten())
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}
