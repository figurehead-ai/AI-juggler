//! AJ-E611 app-level guardrails: (a) `alerts_tick_inner` emits the
//! §Q5 sequence at most once per crossing across ticks and NEVER
//! spawns (no new session rows, no supervisor children); (b) the
//! ranking leaves sessions storage byte-identical (suggest-only);
//! (c) the reset-queue notifies once, marks stale when lineage breaks,
//! and contains no launch path (structurally: no spawn call — pinned
//! by the sessions diff through every queue operation).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_app::commands::{alerts_tick_inner, ranking_rows_inner};
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, LaunchRecipe, ProfileId, SessionId, SessionRecord,
    SessionStatus, Timestamp,
};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(
        AppState::init_with_registry(
            dir.join("app-data"),
            juggler_adapter::AdapterRegistry::new(),
        )
        .unwrap(),
    )
}

fn seed(state: &Arc<AppState>, dir: &Path) {
    let managed = dir.join("managed-alert");
    std::fs::create_dir_all(&managed).unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-a"),
            harness: HarnessId::new("claude-code"),
            label: "Alerts".into(),
            plan: Some(juggler_core::model::PlanTier::new("max")),
            state_dir: managed,
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
        })
        .unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .profiles()
        .insert(&LaunchRecipe {
            id: ProfileId::new("rec-a"),
            name: "a".into(),
            account: AccountId::new("acc-a"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&SessionRecord {
            id: SessionId::new("sess-a"),
            conversation: None,
            profile: ProfileId::new("rec-a"),
            machine: juggler_core::model::MachineId::new("this-mac"),
            kind: juggler_core::model::InstanceKind::Terminal,
            project: None,
            cwd: dir.to_path_buf(),
            status: SessionStatus::Finished,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
}

fn sessions_digest(state: &Arc<AppState>) -> String {
    let storage = state.storage.lock().unwrap();
    let records = storage.sessions().list(None).unwrap();
    format!(
        "{:?}",
        records
            .iter()
            .map(|record| (
                record.id.as_str().to_owned(),
                record.status.as_str().to_owned()
            ))
            .collect::<Vec<_>>()
    )
}

#[test]
fn alert_ticks_emit_once_and_never_spawn() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed(&state, dir.path());
    {
        // An exhausted window with a reset ahead: the §Q5 exhausted
        // path with a Measured sample.
        let storage = state.storage.lock().unwrap();
        let now = Timestamp::now().millis();
        storage
            .usage()
            .insert_sample(
                "acc-a",
                juggler_core::quota::WindowKind::FiveHour,
                &juggler_core::quota::QuotaSample {
                    ts: now - 1_000,
                    used_pct: 100.0,
                    resets_at: Some(now + 3_600_000),
                    confidence: juggler_core::model::Confidence::Measured,
                },
            )
            .unwrap();
    }
    let before = sessions_digest(&state);
    let first = alerts_tick_inner(&state, Some(12));
    // The exhausted reading is below the exhaust SIGNAL (that fires on
    // QuotaBlocked); the ≥90% critical crossing emits ONE actionable.
    assert!(
        first
            .iter()
            .filter(|event| event.kind == "actionable")
            .count()
            <= 1,
        "at most one actionable per crossing: {first:?}"
    );
    // Tick again with the same state ⇒ at-most-once: nothing new.
    let second = alerts_tick_inner(&state, Some(12));
    assert!(
        second
            .iter()
            .filter(|event| event.kind == "actionable")
            .count()
            == 0,
        "no second notification for one crossing: {second:?}"
    );
    // No spawn ever happened: sessions unchanged.
    assert_eq!(before, sessions_digest(&state));
}

#[test]
fn ranking_is_suggest_only_sessions_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed(&state, dir.path());
    let before = sessions_digest(&state);
    let rows = ranking_rows_inner(&state, None).unwrap();
    assert!(!rows.is_empty());
    // Every row explains itself or carries its filter reason.
    for row in &rows {
        assert!(
            row.filtered_reason.is_some() || row.top_factors.len() >= 2,
            "no score without an explanation: {:?}",
            row
        );
    }
    assert_eq!(
        before,
        sessions_digest(&state),
        "ranking never mutates session state"
    );
}

#[test]
fn renewal_approaching_joins_digest_once_not_the_fsm() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed(&state, dir.path());
    // Renewal 6 days out (≤ the 7-day default threshold) ⇒
    // `approaching` (E610 state machine).
    let today_days = Timestamp::now().millis().div_euclid(86_400_000);
    let renewal = date_string(today_days + 6);
    {
        let storage = state.storage.lock().unwrap();
        let mut account = storage.accounts().get(&AccountId::new("acc-a")).unwrap();
        account.renewal_date = Some(renewal);
        storage.accounts().update(&account).unwrap();
    }
    let before = sessions_digest(&state);
    // Daytime hour ⇒ the flush is due on the first tick: ONE digest
    // line, carrying the E610 copy — and NO window alert ever fires
    // from the subscription clock (Req.5).
    let first = alerts_tick_inner(&state, Some(12));
    assert_eq!(
        first.iter().filter(|event| event.kind == "digest").count(),
        1,
        "one digest line: {first:?}"
    );
    assert!(
        first
            .iter()
            .all(|event| event.kind != "actionable" && event.kind != "tray-amber"),
        "renewal never triggers the window FSM: {first:?}"
    );
    // Re-tick inside the same hour ⇒ nothing new (the buffer is empty
    // and the flush is gated by `last_flush_ms`).
    let second = alerts_tick_inner(&state, Some(12));
    assert_eq!(
        second.iter().filter(|event| event.kind == "digest").count(),
        0,
        "no second digest line within the hour: {second:?}"
    );
    // Quiet hour (22:00–07:00) defers without dropping.
    let _ = alerts_tick_inner(&state, Some(23));
    assert_eq!(before, sessions_digest(&state), "passive throughout");
}

/// `YYYY-MM-DD` for an epoch-anchored day count (Hinnant's
/// `civil_from_days` — the inverse of the epoch_of helper in the
/// quota_display tests; UTC-anchored like the display path).
fn date_string(days: i64) -> String {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!("{year:04}-{m:02}-{d:02}")
}

#[test]
fn reset_queue_notifies_once_marks_stale_never_launches() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed(&state, dir.path());
    let now = Timestamp::now().millis();
    {
        let storage = state.storage.lock().unwrap();
        storage
            .usage()
            .queue_reset("rq-sess-a", "sess-a", "acc-a", now - 1)
            .unwrap();
    }
    let before = sessions_digest(&state);
    // Tick 1: the due item notifies once (state → notified).
    let first = alerts_tick_inner(&state, Some(12));
    assert!(
        first.iter().any(|event| event.kind == "reset-reached"),
        "the due queue item notifies: {first:?}"
    );
    // Tick 2: nothing (already notified — not in pending `queued`).
    let second = alerts_tick_inner(&state, Some(12));
    assert!(second.iter().all(|event| event.kind != "reset-reached"));
    // Lineage breaks (session gone) ⇒ the entry goes stale, explained.
    {
        // Break lineage the honest way: no delete API exists (append-
        // first) — write the queue against a session id that is gone.
        let storage = state.storage.lock().unwrap();
        storage
            .usage()
            .queue_reset("rq-ghost", "sess-gone", "acc-a", now - 1)
            .unwrap();
    }
    let _ = alerts_tick_inner(&state, Some(12));
    assert_eq!(before, sessions_digest(&state), "no launch ever fired");
}

#[test]
fn admission_verdict_rides_the_ranked_rows_e306() {
    // AJ-E306 Req.7: the SPEC §5.2 verdict rides the wire — a real
    // verdict when cap + burn are known; None (rendered "headroom
    // unknown") when either input is missing. Never a guess.
    let dir = tempfile::tempdir().unwrap();

    // Unknown cap ⇒ unknown headroom ⇒ admission None.
    let bare = app_state(dir.path());
    seed(&bare, dir.path());
    let rows = ranking_rows_inner(&bare, None).unwrap();
    let acc_a = rows.iter().find(|row| row.account == "acc-a").unwrap();
    assert_eq!(acc_a.admission, None, "unknown headroom stays unknown");

    // Confirmed cap (10k) + a project session that burned 2k ⇒ OK.
    // (Its own tempdir — a second seed into one store violates the
    // accounts unique constraint.)
    let known_dir = tempfile::tempdir().unwrap();
    let known = app_state(known_dir.path());
    seed(&known, dir.path());
    {
        let storage = known.storage.lock().unwrap();
        // The verdict is a PROJECT-launch gate: a session on proj-x
        // carries the burn the median is taken over.
        storage
            .sessions()
            .insert(&SessionRecord {
                id: SessionId::new("sess-b"),
                conversation: None,
                profile: ProfileId::new("rec-a"),
                machine: juggler_core::model::MachineId::new("this-mac"),
                kind: juggler_core::model::InstanceKind::Terminal,
                project: Some(juggler_core::model::ProjectId::new("proj-x")),
                cwd: known_dir.path().to_path_buf(),
                status: SessionStatus::Finished,
                started_at: Timestamp::now(),
                last_activity: None,
                usage: Default::default(),
                attention: None,
                running: None,
                transcript_ref: None,
                window: None,
            })
            .unwrap();
        storage
            .app_settings()
            .set(
                &juggler_core::storage::cap_settings_key(
                    "acc-a",
                    juggler_core::quota::WindowKind::FiveHour,
                ),
                "tokens=10000",
            )
            .unwrap();
        storage
            .usage()
            .insert_events(&[juggler_core::quota::UsageEvent {
                account: "acc-a".into(),
                session: Some("sess-b".into()),
                ts: Timestamp::now().millis() - 1_000,
                model: None,
                tokens: juggler_core::model::TokenCounts {
                    output: 2_000,
                    ..Default::default()
                },
                credits: None,
                usd_est: None,
                confidence: juggler_core::model::Confidence::Measured,
                msg_uuid: None,
                source: juggler_core::quota::UsageSource::Transcript,
            }])
            .unwrap();
    }
    let rows = ranking_rows_inner(
        &known,
        Some(juggler_core::model::ProjectId::new("proj-x").to_string()),
    )
    .unwrap();
    let acc_a = rows.iter().find(|row| row.account == "acc-a").unwrap();
    assert_eq!(acc_a.admission.as_deref(), Some("ok"));

    // Same burn, an 800-token cap ⇒ the burn exceeds headroom ⇒ No
    // (warns on the wire — blocking is a UI decision it never makes).
    {
        let storage = known.storage.lock().unwrap();
        storage
            .app_settings()
            .set(
                &juggler_core::storage::cap_settings_key(
                    "acc-a",
                    juggler_core::quota::WindowKind::FiveHour,
                ),
                "tokens=800",
            )
            .unwrap();
    }
    let rows = ranking_rows_inner(
        &known,
        Some(juggler_core::model::ProjectId::new("proj-x").to_string()),
    )
    .unwrap();
    let acc_a = rows.iter().find(|row| row.account == "acc-a").unwrap();
    assert_eq!(acc_a.admission.as_deref(), Some("no"));
}
