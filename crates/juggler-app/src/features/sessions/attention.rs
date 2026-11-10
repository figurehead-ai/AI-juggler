//! Lifecycle forwarding: supervisor events applied to storage, and the
//! attention heuristics (AJ-E402).

use std::sync::Arc;

use juggler_core::model::{SessionId, SessionRecord, SessionStatus, Timestamp};
use juggler_core::storage::Storage;
use juggler_supervisor::LifecycleEvent;

use crate::error::{AppError, AppResult};
use crate::features::processes::running_model::detect_running_models;
use crate::features::sessions::read_model::{emit_sessions, is_active, live_stats_of};
use crate::state::AppState;

/// Applies a supervisor lifecycle event to storage (status transitions).
/// Returns the updated record when one matched.
#[allow(
    clippy::needless_pass_by_value,
    reason = "its signature as it was in commands.rs; AJ-E2017 moved it unchanged"
)]
pub fn apply_lifecycle_event(
    storage: &Storage,
    event: LifecycleEvent,
    stopped_by_user: bool,
    quota_blocked: Option<&juggler_core::model::AttentionReason>,
) -> Option<SessionRecord> {
    let session_id = match &event {
        LifecycleEvent::Started { session, .. }
        | LifecycleEvent::Exited { session, .. }
        | LifecycleEvent::HelperOrphaned { session, .. } => session,
    };
    let mut record = storage.sessions().get(session_id).ok()?;
    let mut ending = None;
    match event {
        // A start only moves a row that is still starting: one read after
        // the exit, or after attention was raised, is stale and must
        // never bring the row back to Running.
        LifecycleEvent::Started { .. } if record.status != SessionStatus::Starting => return None,
        LifecycleEvent::Started { .. } => {
            record.status = SessionStatus::Running;
            record.attention = None;
        }
        // AJ-E504: helper-level delta — the SESSION record does not
        // change (the root still owns the row state); the Cleanup
        // Centre (AJ-E506) consumes these via the ledger.
        LifecycleEvent::HelperOrphaned { .. } => return None,
        LifecycleEvent::Exited { kind, .. } => {
            record.status = if stopped_by_user {
                // The stop ladder (PTY exits surface kills as plain
                // non-zero codes) — user intent, not a failure.
                SessionStatus::Stopped
            } else if let Some(reason) = quota_blocked {
                // AJ-E405: non-zero exit with a rate-limit signature —
                // QuotaBlocked (immediate, no debounce) with the reason.
                record.attention = Some(reason.clone());
                SessionStatus::QuotaBlocked
            } else {
                match kind {
                    kind if kind.is_success() => SessionStatus::Finished,
                    juggler_supervisor::ExitKind::Signaled(_) => SessionStatus::Stopped,
                    juggler_supervisor::ExitKind::Code(_) => SessionStatus::Failed,
                }
            };
            if record.status != SessionStatus::QuotaBlocked {
                record.attention = None;
            }
            // AJ-E1103: a signal nobody asked for is a crash.
            let outcome = match (record.status, kind) {
                (SessionStatus::Finished, _) => "finished",
                (SessionStatus::Stopped, _) if stopped_by_user => "stopped",
                (_, juggler_supervisor::ExitKind::Signaled(_)) => "crashed",
                _ => "failed",
            };
            crate::usage_stats::instance_ended(storage, &record, outcome);
            ending = ending_of_exit(record.status, kind, stopped_by_user);
        }
    }
    record.last_activity = Some(Timestamp::now());
    storage.sessions().update(&record).ok()?;
    // D-96 C: how it ended, in its history, from its process's own exit.
    if let Some((how, detail)) = ending {
        let _ = storage.sessions().record_lifecycle(
            &record.id,
            &juggler_core::model::ending_event(how, detail.as_deref()),
        );
    }
    Some(record)
}

/// D-96 C (AJ-E3307): how a session AI Juggler runs ended, from its
/// process's exit and the user's Stop — the only signals it has for a
/// terminal or driven session (Claude Code, Codex and every tool run in
/// its terminal alike): code 0 finished, another code an error, the
/// user's Stop "stopped by you", a signal nobody asked for a crash. A
/// limit's exit waits on the user (`QuotaBlocked`): no ending yet.
#[must_use]
pub fn ending_of_exit(
    status: SessionStatus,
    kind: juggler_supervisor::ExitKind,
    stopped_by_user: bool,
) -> Option<(juggler_core::model::Ending, Option<String>)> {
    use juggler_core::model::Ending;
    if status == SessionStatus::QuotaBlocked {
        return None;
    }
    if stopped_by_user {
        return Some((Ending::StoppedByYou, None));
    }
    Some(match kind {
        juggler_supervisor::ExitKind::Code(0) => (Ending::Finished, None),
        juggler_supervisor::ExitKind::Code(code) => {
            (Ending::Error, Some(format!("exit code {code}")))
        }
        juggler_supervisor::ExitKind::Signaled(signal) => (
            Ending::Crashed,
            signal.map(|signal| format!("signal {signal}")),
        ),
    })
}

/// Records one heuristic transition (status + structured reason) and
/// returns the updated record. No action ever follows (SPEC §7).
pub fn apply_attention_transition(
    storage: &Storage,
    session: &SessionId,
    transition: &juggler_supervisor::AttentionTransition,
) -> Option<SessionRecord> {
    let mut record = storage.sessions().get(session).ok()?;
    let entered = record.status != transition.status;
    record.status = transition.status;
    record.attention = Some(transition.reason.clone());
    storage.sessions().update(&record).ok()?;
    if entered {
        crate::usage_stats::instance_state_entered(storage, &record);
    }
    Some(record)
}

/// Clears attention and returns the record to `Running` — the recovery
/// path (output resumed; AJ-E405 Req.5).
pub fn apply_attention_recovery(storage: &Storage, session: &SessionId) -> Option<SessionRecord> {
    let mut record = storage.sessions().get(session).ok()?;
    if !matches!(
        record.status,
        SessionStatus::Stuck | SessionStatus::WaitingUser | SessionStatus::QuotaBlocked
    ) {
        return None;
    }
    record.status = SessionStatus::Running;
    record.attention = None;
    record.last_activity = Some(Timestamp::now());
    storage.sessions().update(&record).ok()?;
    Some(record)
}

/// One 15 s heuristic cycle (AJ-E405): evaluates every active Instance
/// against the supervisor's measured inputs and records + emits any
/// confirmed transitions. Purely passive — no action follows.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn heuristic_tick(app: &tauri::AppHandle, state: &Arc<AppState>) -> AppResult<()> {
    // rc19: a session whose hooks never reported waits on its tool's own
    // screen (a picker, a question), never "Working".
    crate::features::runtime::reports::overdue_tick(state, Timestamp::now().millis());
    let engine = &state.heuristics;
    let live = live_stats_of(state);
    let mut entered = Vec::new();
    let mut recovered = false;
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        // AJ-E1004 Req.1: each Instance is judged against its Provider's
        // own no-output threshold when one is configured; its tool's
        // adapter says whether its last turn ended.
        let mut harness_of: std::collections::BTreeMap<String, juggler_core::model::HarnessId> =
            std::collections::BTreeMap::new();
        for account in storage.accounts().list()? {
            for recipe in storage.profiles().list_by_account(&account.id)? {
                harness_of.insert(recipe.id.to_string(), account.harness.clone());
            }
        }
        for record in storage.sessions().list(None)? {
            // AJ-E2302, 0.3: a driven session's or a limit's state is the tool's.
            if !is_active(record.status)
                || record.status == SessionStatus::QuotaBlocked
                || state.runtime.is_exact(&record.id)
            {
                engine.forget(&record.id);
                continue;
            }
            let now = Timestamp::now().millis();
            let transcript_mtime_ms = record.transcript_ref.as_ref().and_then(|tr| {
                std::fs::metadata(&tr.path)
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|dur| i64::try_from(dur.as_millis()).unwrap_or_default())
            });
            let tail = state
                .supervisor
                .output_tail(&record.id, 2048)
                .unwrap_or_default();
            let harness = harness_of.get(record.profile.as_str());
            // A tool with no state reports says through its adapter
            // whether its last turn ended, from its own record (its store,
            // event log or rollout): then it waits for the person,
            // however quiet.
            let turn_ended = harness
                .and_then(|harness| state.registry.get(harness))
                .zip(record.transcript_ref.as_ref())
                .and_then(|(adapter, transcript)| adapter.last_turn(&transcript.path))
                == Some(juggler_adapter::live::TailTurn::Ended);
            let inputs = juggler_supervisor::HeuristicInputs {
                last_output_ms: state.supervisor.last_output_ms(&record.id),
                cpu_pct: live
                    .get(&record.id.to_string())
                    .and_then(|stats| stats.cpu_pct),
                transcript_mtime_ms,
                prompt_alive: juggler_supervisor::prompt_alive(&tail),
                turn_ended,
                now_ms: now,
            };
            let provider_stuck_secs = harness
                .and_then(|harness| state.provider_stuck_secs.get(harness.as_str()))
                .copied();
            if let Some(transition) = engine.evaluate_for(&record.id, &inputs, provider_stuck_secs)
            {
                apply_attention_transition(&storage, &record.id, &transition);
                entered.push((record.id.to_string(), transition.status));
            } else if turn_ended && record.status == SessionStatus::Stuck {
                // Called stuck before its turn was seen to end: it is idle.
                engine.forget(&record.id);
                recovered |= apply_attention_recovery(&storage, &record.id).is_some();
            }
        }
    }
    // AJ-E1510: waiting or stuck, as a notification when asked for (and
    // what quiet hours held, once they end).
    for (session, status) in &entered {
        crate::features::notifications::session_status(state, session, status.as_str());
    }
    // 0.3: open sessions at their limit, and marks whose reset passed.
    let limits_changed = crate::features::sessions::limits::tick(state);
    crate::features::notifications::show_due(state);
    if !entered.is_empty() || limits_changed || recovered {
        emit_sessions(app, state)?;
    }
    Ok(())
}

/// Event-driven recovery drain (Req.5, <300 ms): every session with
/// fresh output that currently sits in a heuristic state returns to
/// `Running` and the UI is told immediately.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn attention_recovery_drain(app: &tauri::AppHandle, state: &Arc<AppState>) -> AppResult<()> {
    let mut changed = false;
    let drained: Vec<SessionId> = state.supervisor.drain_activity();
    for session in &drained {
        changed |= crate::features::sessions::limits::output_moved(state, session);
        if let Some(running) = state.heuristics.recover(session) {
            debug_assert_eq!(running, SessionStatus::Running);
            {
                let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
                if apply_attention_recovery(&storage, session).is_some() {
                    changed = true;
                }
            }
            crate::features::notifications::session_status(state, session.as_str(), "running");
        }
    }
    if changed {
        emit_sessions(app, state)?;
    }
    // AJ-E412 Req.2: the same fresh-output signal drives transcript
    // tail-scans for the running model (only the active Instances) —
    // the event path keeps the <300 ms budget; `detect_running_models`
    // emits `sessions_updated` itself when a model changed.
    if !drained.is_empty() {
        detect_running_models(app, state, Some(&drained));
    }
    Ok(())
}
