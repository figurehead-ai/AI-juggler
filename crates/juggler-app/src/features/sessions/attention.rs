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
    let stop = if stopped_by_user {
        Stop::ByYou
    } else {
        Stop::None
    };
    apply_lifecycle_event_as(storage, event, stop, quota_blocked)
}

/// Who stopped a session whose process ended: no one, the user's Stop, or
/// a switch (its work goes on in the session the switch started).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Nobody asked it to stop.
    None,
    /// The user's Stop.
    ByYou,
    /// A switch stopped it (`AppState::switch_stops`).
    BySwitch,
}

/// [`apply_lifecycle_event`] with who stopped it: a switch's stop ends it
/// "stopped by the switch", never "stopped by you" (the rc27 P1 walk).
#[allow(clippy::needless_pass_by_value, reason = "as apply_lifecycle_event")]
pub fn apply_lifecycle_event_as(
    storage: &Storage,
    event: LifecycleEvent,
    stop: Stop,
    quota_blocked: Option<&juggler_core::model::AttentionReason>,
) -> Option<SessionRecord> {
    let stopped_by_user = stop != Stop::None;
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
            ending = ending_of_exit(record.status, kind, stop);
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
    stop: Stop,
) -> Option<(juggler_core::model::Ending, Option<String>)> {
    use juggler_core::model::Ending;
    if status == SessionStatus::QuotaBlocked {
        return None;
    }
    match stop {
        Stop::ByYou => return Some((Ending::StoppedByYou, None)),
        Stop::BySwitch => return Some((Ending::Switched, None)),
        Stop::None => {}
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
            // Its tool's own screen question is [`screen_asks`]' to say
            // and to end; nothing here calls it stuck.
            if record
                .attention
                .as_ref()
                .is_some_and(|reason| reason.rule == TOOL_ASKS)
            {
                engine.forget(&record.id);
                continue;
            }
            // A tool with no state reports says through its adapter
            // whether its last turn ended, from its own record (its store,
            // event log or rollout): then it waits for the person,
            // however quiet.
            let turn_ended = waits_for_person(
                record.transcript_ref.is_some(),
                harness
                    .and_then(|harness| state.registry.get(harness))
                    .zip(record.transcript_ref.as_ref())
                    .and_then(|(adapter, transcript)| adapter.last_turn(&transcript.path)),
            );
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

/// Whether a session with no state reports waits for the person rather
/// than works: its tool's own record says its last turn ended, or it has
/// no conversation recorded yet (`None`), so no turn has started (rc27
/// live pass 2: a fresh OpenCode session idle at its prompt for two
/// minutes read "Probably stuck" and counted as needing you). `linked`:
/// a conversation is recorded; `last_turn`, its last turn as its adapter
/// reads it.
fn waits_for_person(linked: bool, last_turn: Option<juggler_adapter::live::TailTurn>) -> bool {
    !linked || last_turn == Some(juggler_adapter::live::TailTurn::Ended)
}

/// How much of a session's output its tool's screen question is read
/// from ([`screen_asks`]).
const SCREEN_TAIL: usize = 8192;

/// The rc27 P1 walk: a switch's Codex in its terminal sat at its own
/// "Trust this folder?" and read Working; the 120 s prompt rule said so
/// only later. A session whose tool sends no state reports and whose
/// fresh output (`sessions`) ends on a question its adapter knows
/// (`ToolConfig::screen_ask`) asks you at once, with the question as its
/// words (`tool_asks`, as a tool's own ask reads); once the screen moves
/// on, it works again. Returns the sessions whose status changed.
pub fn screen_asks(
    state: &Arc<AppState>,
    sessions: &[SessionId],
) -> Vec<(SessionId, SessionStatus)> {
    let mut changed = Vec::new();
    for session in sessions {
        if state.runtime.is_exact(session) {
            continue;
        }
        let Ok(storage) = state.storage.lock() else {
            break;
        };
        let Some((adapter, account, cwd)) = adapter_of(&storage, state, session) else {
            continue;
        };
        let tail = state
            .supervisor
            .output_tail(session, SCREEN_TAIL)
            .unwrap_or_default();
        // The rc29 walk: Codex confirms a restricted open the user already
        // chose in AI Juggler (passed in memory) on every launch. That
        // answer is theirs: given once (a failed write asks them instead).
        if let Some(keys) = users_answer(&storage, adapter.as_ref(), &account, &cwd, &tail)
            && state.runtime.first_screen_answer(session)
            && state.supervisor.write_input(session, keys.as_bytes()).is_ok()
        {
            continue;
        }
        let asked = adapter.screen_ask(&tail);
        if let Some(status) = apply_screen_ask(
            &storage,
            session,
            asked.as_deref(),
            Timestamp::now().millis(),
        ) {
            state.heuristics.forget(session);
            changed.push((session.clone(), status));
        }
    }
    if !changed.is_empty() {
        state.runtime.mark_needs_changed();
    }
    changed
}

/// The adapter of `session`'s tool, through its launch settings' folder,
/// with its account and the folder it runs in.
fn adapter_of(
    storage: &Storage,
    state: &AppState,
    session: &SessionId,
) -> Option<(
    juggler_adapter::SharedAdapter,
    juggler_core::model::Account,
    std::path::PathBuf,
)> {
    let record = storage.sessions().get(session).ok()?;
    let profile = storage.profiles().get(&record.profile).ok()?;
    let account = storage.accounts().get(&profile.account).ok()?;
    let adapter = state.registry.get(&account.harness)?;
    Some((adapter, account, record.cwd))
}

/// The keys that give the tool's own folder screen (`tail`) the answer
/// the user gave in AI Juggler for `cwd`: only an answer they gave (never
/// AI Juggler's default), only for a folder whose trust AI Juggler passes
/// (not one decided in the tool), and only where the screen confirms
/// exactly that answer (`ToolConfig::screen_answer`).
pub fn users_answer(
    storage: &Storage,
    adapter: &dyn juggler_adapter::HarnessAdapter,
    account: &juggler_core::model::Account,
    cwd: &std::path::Path,
    tail: &str,
) -> Option<&'static str> {
    if adapter.folder_trust_decided(account, cwd) {
        return None;
    }
    let trusted = crate::features::runtime::trust::choice(storage, adapter.id().as_str(), cwd)?;
    adapter.screen_answer(tail, trusted)
}

/// The rule a tool's own question reads under (`runtime::driver`'s too).
const TOOL_ASKS: &str = "tool_asks";

/// [`screen_asks`] for one session: `asked`, the question its screen shows
/// now (`None`: none). An open session asking takes it as its ask; one
/// whose screen question was answered works again. Returns its new status
/// when it changed.
pub fn apply_screen_ask(
    storage: &Storage,
    session: &SessionId,
    asked: Option<&str>,
    now_ms: i64,
) -> Option<SessionStatus> {
    let mut record = storage.sessions().get(session).ok()?;
    let ours = record.status == SessionStatus::WaitingUser
        && record
            .attention
            .as_ref()
            .is_some_and(|reason| reason.rule == TOOL_ASKS);
    match asked {
        Some(question)
            if matches!(
                record.status,
                SessionStatus::Starting | SessionStatus::Running | SessionStatus::Stuck
            ) || (record.status == SessionStatus::WaitingUser && !ours) =>
        {
            record.status = SessionStatus::WaitingUser;
            record.attention = Some(juggler_core::model::AttentionReason {
                rule: TOOL_ASKS.into(),
                inputs: vec![question.to_owned()],
                since_ms: now_ms,
            });
        }
        None if ours => {
            record.status = SessionStatus::Running;
            record.attention = None;
        }
        _ => return None,
    }
    record.last_activity = Some(Timestamp::now());
    storage.sessions().update(&record).ok()?;
    crate::usage_stats::instance_state_entered(storage, &record);
    Some(record.status)
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
    // The rc27 P1 walk: a tool's own screen question asks at once.
    for (session, status) in screen_asks(state, &drained) {
        changed = true;
        crate::features::notifications::session_status(state, session.as_str(), status.as_str());
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

#[cfg(test)]
mod tests {
    use super::waits_for_person;
    use juggler_adapter::live::TailTurn;
    use juggler_core::model::SessionId;
    use juggler_supervisor::{HeuristicEngine, HeuristicInputs, HeuristicThresholds};

    /// rc27 live pass 2: a fresh OpenCode session idle at its prompt for
    /// about two minutes read "Probably stuck". No conversation recorded
    /// yet means no turn has started: it waits, however quiet.
    #[test]
    fn a_session_with_no_turn_yet_waits_and_is_never_stuck() {
        assert!(waits_for_person(false, None), "no conversation yet");
        assert!(waits_for_person(true, Some(TailTurn::Ended)));
        assert!(!waits_for_person(true, Some(TailTurn::Running)));
        assert!(!waits_for_person(true, None), "a record it can't read");
        let engine = HeuristicEngine::new(HeuristicThresholds::default());
        let session = SessionId::new("sess-fresh");
        let quiet = |now_ms: i64| HeuristicInputs {
            last_output_ms: Some(0),
            cpu_pct: Some(0.0),
            transcript_mtime_ms: None,
            prompt_alive: false,
            turn_ended: waits_for_person(false, None),
            now_ms,
        };
        // Long past the threshold and its debounce: still nothing.
        for minutes in [10, 11, 12, 30] {
            assert_eq!(engine.evaluate(&session, &quiet(minutes * 60_000)), None);
        }
    }

    /// The rc29 walk: Codex in `~/lab/web` confirms the restricted open
    /// the user chose there (`runtime.trust`: untrusted) on every launch.
    /// That screen gets the user's own answer; with no answer kept, a
    /// trust kept, or the folder decided in Codex itself, it asks them.
    #[test]
    fn codexs_restricted_confirmation_gets_only_the_users_own_answer() {
        use super::users_answer;
        let dir = tempfile::tempdir().unwrap();
        let storage = juggler_core::storage::Storage::open(&dir.path().join("s.sqlite")).unwrap();
        let codex = juggler_adapters::CodexAdapter::new();
        let account = juggler_core::model::Account {
            id: juggler_core::model::AccountId::new("acc-g"),
            harness: juggler_core::model::HarnessId::new("codex"),
            label: "Account G".into(),
            plan: None,
            state_dir: dir.path().join("codex-home"),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        };
        let web = dir.path().join("web");
        std::fs::create_dir_all(&web).unwrap();
        let screen = "\u{1b}[1;1HFolder access\u{1b}[2;1H/Users/you/lab/web\u{1b}[4;1HConfig, hooks, and exec policies from untrusted folders stay disabled. Opening will not change saved trust.\u{1b}[7;1H\u{1b}[7m\u{203a} 1. Open restricted\u{1b}[0m\u{1b}[8;1H  2. Quit\u{1b}[10;1Henter continue \u{b7} esc quit";
        assert_eq!(
            users_answer(&storage, &codex, &account, &web, screen),
            None,
            "no answer of the user's yet: it asks them"
        );
        crate::features::runtime::trust::choose(&storage, "codex", &web, false).unwrap();
        assert_eq!(
            users_answer(&storage, &codex, &account, &web, screen),
            Some("\r"),
            "Open restricted, as they chose"
        );
        assert_eq!(
            users_answer(
                &storage,
                &codex,
                &account,
                &web,
                "Working (3s \u{b7} esc to interrupt)"
            ),
            None
        );
        // Decided in Codex itself: Codex's own, never answered here.
        std::fs::create_dir_all(&account.state_dir).unwrap();
        std::fs::write(
            account.state_dir.join("config.toml"),
            format!(
                "[projects.\"{}\"]\ntrust_level = \"untrusted\"\n",
                web.canonicalize().unwrap().display()
            ),
        )
        .unwrap();
        assert_eq!(users_answer(&storage, &codex, &account, &web, screen), None);
    }
}
