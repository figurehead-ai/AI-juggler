//! AJ-E610 backend evidence: `quota_display_inner` typed states — the
//! §Q8 machine mapped to wire states, badge wiring, Codex-Pro 5 h
//! omission WITH reason, Gemini unavailability, D-12 renewal copy with
//! zero inferred dates, and the forecast pill data.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_app::commands::quota_display_inner;
use juggler_app::state::AppState;
use juggler_core::model::{Account, AccountId, HarnessId, Timestamp};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(
        AppState::init_with_registry(
            dir.join("app-data"),
            juggler_adapter::AdapterRegistry::new(),
        )
        .unwrap(),
    )
}

fn account(provider: &str, plan: Option<&str>) -> Account {
    Account {
        id: AccountId::new("acc-d"),
        harness: HarnessId::new(provider),
        label: "display".into(),
        plan: plan.map(juggler_core::model::PlanTier::new),
        state_dir: dir_unused(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
    }
}

fn dir_unused() -> std::path::PathBuf {
    std::path::PathBuf::from("/nonexistent-managed")
}

#[test]
fn no_data_states_render_unknown_and_learning_not_zero() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let storage = state.storage.lock().unwrap();
    let info = quota_display_inner(
        &storage,
        &account("claude-code", Some("max")),
        Timestamp::now().millis(),
    );
    assert_eq!(info.five_hour.as_ref().unwrap().state, "unknown");
    assert_eq!(info.five_hour.as_ref().unwrap().percentage, None);
    assert_eq!(info.renewal.state, "none");
    assert_eq!(info.renewal.label, "Renewal: unknown — add date");
    assert!(!info.renewal.entered_by_user, "D-12: nothing inferred");
    assert_eq!(info.forecast.state, "learning");
}

#[test]
fn tracking_state_carries_percentage_and_badge() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    {
        let storage = state.storage.lock().unwrap();
        let now = Timestamp::now().millis();
        // Two events + a Measured sample ⇒ Tracking with Measured badge.
        let events: Vec<juggler_core::quota::UsageEvent> = (0..3)
            .map(|back| juggler_core::quota::UsageEvent {
                account: "acc-d".into(),
                session: None,
                ts: now - i64::from(back) * 60_000,
                model: Some("claude-opus-4".into()),
                tokens: juggler_core::model::TokenCounts {
                    input: 1_000,
                    ..juggler_core::model::TokenCounts::default()
                },
                credits: None,
                usd_est: None,
                source: juggler_core::quota::UsageSource::Transcript,
                confidence: juggler_core::model::Confidence::Measured,
                msg_uuid: Some(format!("d-{back}")),
            })
            .collect();
        storage.usage().insert_events(&events).unwrap();
        storage
            .usage()
            .insert_sample(
                "acc-d",
                juggler_core::quota::WindowKind::FiveHour,
                &juggler_core::quota::QuotaSample {
                    ts: now - 60_000,
                    used_pct: 82.0,
                    resets_at: Some(now + 3 * 3_600_000),
                    confidence: juggler_core::model::Confidence::Measured,
                },
            )
            .unwrap();
    }
    let storage = state.storage.lock().unwrap();
    let info = quota_display_inner(
        &storage,
        &account("claude-code", Some("max")),
        Timestamp::now().millis(),
    );
    let five = info.five_hour.as_ref().unwrap();
    assert_eq!(five.state, "tracking");
    assert!(five.percentage.is_some_and(|p| (p - 82.0).abs() < 1e-9));
    assert_eq!(five.badge.as_deref(), Some("measured"));
    assert!(five.resets_at_ms.is_some());
}

#[test]
fn codex_pro_omits_the_five_hour_ring_with_reason() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let storage = state.storage.lock().unwrap();
    let info = quota_display_inner(
        &storage,
        &account("codex", Some("pro")),
        Timestamp::now().millis(),
    );
    assert!(info.five_hour.is_none());
    assert!(
        info.five_hour_omitted_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("no five-hour limit")),
        "omitted WITH reason, never an empty ring"
    );
    assert!(info.weekly.is_some(), "Pro renders the weekly window");
}

#[test]
fn gemini_is_unavailable_with_reason() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let storage = state.storage.lock().unwrap();
    let info = quota_display_inner(
        &storage,
        &account("gemini", None),
        Timestamp::now().millis(),
    );
    assert!(
        info.unavailable_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("No verified quota source"))
    );
    assert!(info.five_hour.is_none() && info.weekly.is_none());
}

#[test]
fn renewal_states_map_with_user_tunable_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    {
        let storage = state.storage.lock().unwrap();
        storage
            .app_settings()
            .set("renewal_warn_days", "3")
            .unwrap();
    }
    let storage = state.storage.lock().unwrap();
    let mut with_date = account("claude-code", None);
    // 6 days out with a 3-day threshold ⇒ `set`, not `approaching`.
    with_date.renewal_date = Some("2026-10-22".into());
    let info = quota_display_inner(&storage, &with_date, epoch_of("2026-10-16"));
    assert_eq!(info.renewal.state, "set");
    // 2 days out ⇒ approaching with the exact copy.
    let mut soon = account("claude-code", None);
    soon.renewal_date = Some("2026-10-18".into());
    let info = quota_display_inner(&storage, &soon, epoch_of("2026-10-16"));
    assert_eq!(info.renewal.state, "approaching");
    assert_eq!(info.renewal.label, "Renews in 2 days — check your plan");
}

/// A fixed epoch ms for a UTC date (the display path takes `today`
/// from the same clock; dates in this test are UTC-anchored).
fn epoch_of(date: &str) -> i64 {
    // Small civil-date → epoch-ms helper (UTC midnight).
    let mut parts = date.split('-');
    let year: i64 = parts.next().unwrap().parse().unwrap();
    let month: i64 = parts.next().unwrap().parse().unwrap();
    let day: i64 = parts.next().unwrap().parse().unwrap();
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    days * 86_400_000
}
