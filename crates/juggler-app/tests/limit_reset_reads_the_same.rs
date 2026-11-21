//! 0.4.9 (AJ-E2312 item 1, the PM5 walk): once a session's limit has
//! reset, nothing says it is at its limit. Work read 100% used to a reset
//! three minutes after the lab opened; past it, the session's header kept
//! "Limit reached · 4 min", Home's summary "Needs you — quota reached"
//! under a heading of "All clear". One state, read from one record: the
//! row every view reads (header, rail, Home), the bell's Needs you and the
//! summary agree. Every value is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::features::notifications::needs::{NeedSources, work_out};
use juggler_app::features::sessions::dto::{InstanceInfo, NeedsAttention};
use juggler_app::features::sessions::{limits, read_model};
use juggler_app::overnight;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, AttentionReason, Confidence, HarnessId, InstanceKind, InstanceRecord,
    LaunchRecipe, MachineId, ProfileId, SessionId, SessionStatus, Timestamp,
};
use juggler_core::quota::{QuotaSample, UsageSource, WindowKind};

const MIN: i64 = 60_000;

fn folder(state: &Arc<AppState>, dir: &Path, id: &str) {
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new(id),
            harness: HarnessId::new("claude-code"),
            label: format!("{id} label"),
            plan: None,
            state_dir: dir.join(id),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        })
        .unwrap();
    storage
        .profiles()
        .insert(&LaunchRecipe {
            id: ProfileId::new(format!("rec-{id}")),
            name: "Main".into(),
            account: AccountId::new(id),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
}

/// Folder `id` read used up `read_ago` ago, its five hours resetting at `resets_at`.
fn used_up(state: &Arc<AppState>, id: &str, read_at: i64, resets_at: i64) {
    let sample = QuotaSample {
        ts: read_at,
        used_pct: 100.0,
        resets_at: Some(resets_at),
        confidence: Confidence::Measured,
        held_until: None,
    };
    state
        .storage
        .lock()
        .unwrap()
        .usage()
        .insert_sourced_samples(&[(
            id.to_owned(),
            WindowKind::FiveHour,
            sample,
            UsageSource::Statusline,
        )])
        .unwrap();
}

/// A session on folder `id` that ended at its limit `since` (the exit
/// classifier's mark, as a quit Claude Code leaves it).
fn ended_at_limit(state: &Arc<AppState>, dir: &Path, session: &str, id: &str, since: i64) {
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: SessionId::new(session),
            conversation: None,
            profile: ProfileId::new(format!("rec-{id}")),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: dir.to_path_buf(),
            status: SessionStatus::QuotaBlocked,
            started_at: Timestamp::from_millis(since - 30 * MIN),
            last_activity: Some(Timestamp::from_millis(since)),
            usage: Default::default(),
            attention: Some(AttentionReason {
                rule: limits::EXIT_RULE.to_owned(),
                inputs: vec!["rate limit".to_owned()],
                since_ms: since,
            }),
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
}

fn rows(state: &Arc<AppState>) -> Vec<InstanceInfo> {
    read_model::session_infos_snapshot(&state.storage.lock().unwrap()).unwrap()
}

fn row<'a>(rows: &'a [InstanceInfo], id: &str) -> &'a InstanceInfo {
    rows.iter().find(|row| row.id == id).unwrap()
}

/// The bell's Needs you kind for session `id` (none: not listed).
fn bell(rows: &[InstanceInfo], id: &str, now: i64) -> Option<String> {
    let sources = NeedSources {
        sessions: rows.to_vec(),
        computer: "Mac",
        now_ms: now,
        ..NeedSources::default()
    };
    let (needs, _) = work_out(&sources, &HashSet::<String>::new());
    needs
        .into_iter()
        .find(|need| need.key == format!("s:{id}"))
        .map(|need| need.kind)
}

/// The summary's group and words for session `id`.
fn summary(state: &Arc<AppState>, id: &str) -> (String, String) {
    let summary = overnight::overnight_summary_impl(state, Some(0)).unwrap();
    let row = summary
        .rows
        .into_iter()
        .find(|row| row.session_id == id)
        .unwrap();
    (row.group, row.words)
}

#[test]
fn past_its_reset_a_session_that_stopped_at_its_limit_reads_room_again_everywhere() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(dir.path().join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    let now = Timestamp::now().millis();
    // Work reset a minute ago; Spare resets in three minutes.
    folder(&state, dir.path(), "work");
    folder(&state, dir.path(), "spare");
    used_up(&state, "work", now - 6 * MIN, now - MIN);
    used_up(&state, "spare", now - 6 * MIN, now + 3 * MIN);
    ended_at_limit(&state, dir.path(), "s-work", "work", now - 5 * MIN);
    ended_at_limit(&state, dir.path(), "s-spare", "spare", now - 5 * MIN);

    assert!(
        limits::tick(&state),
        "the reset is seen on the tick, so the views are told"
    );
    let all = rows(&state);

    // Past its reset: its limit is gone, in the one word every view reads.
    let work = row(&all, "s-work");
    assert_eq!(
        work.status, "quota_blocked",
        "it ended at its limit: its status is kept"
    );
    assert_eq!(work.needs_attention, Some(NeedsAttention::LimitReset));
    assert_eq!(work.attention.as_ref().unwrap().rule, limits::RESET_RULE);
    assert_eq!(
        work.attention.as_ref().unwrap().since_ms,
        now - MIN,
        "it waits since its reset"
    );
    assert_eq!(bell(&all, "s-work", now).as_deref(), Some("resume"));
    let (group, words) = summary(&state, "s-work");
    assert_eq!(group, "needs_you", "it waits on you: to resume it");
    assert!(!words.contains("quota reached"), "{words}");

    // Before its reset: still at its limit, everywhere.
    let spare = row(&all, "s-spare");
    assert_eq!(spare.needs_attention, Some(NeedsAttention::QuotaBlocked));
    assert_eq!(bell(&all, "s-spare", now).as_deref(), Some("limit"));
    assert_eq!(summary(&state, "s-spare").1, "Needs you — quota reached");

    // Seen once: the next tick changes nothing.
    assert!(!limits::tick(&state));
}

#[test]
fn a_parked_session_waits_apart_from_needs_you_in_the_summary_as_on_home() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(dir.path().join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    let now = Timestamp::now().millis();
    folder(&state, dir.path(), "work");
    used_up(&state, "work", now - 6 * MIN, now - MIN);
    ended_at_limit(&state, dir.path(), "s-parked", "work", now - 5 * MIN);
    // Parked until Work's reset (its page's "Park until reset"), as the
    // queue keeps it.
    let parked = serde_json::json!({ "s-parked": {
        "account_id": "work", "account_label": "work label", "parked_at_ms": now - 4 * MIN,
        "resets_at_ms": now - MIN, "confidence": "measured", "pack_dir": dir.path().join("pack"),
        "offers": 0, "snoozed_until_ms": null, "quiet": false } });
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set("parked_instances", &parked.to_string())
        .unwrap();
    limits::tick(&state);
    let all = rows(&state);
    assert!(row(&all, "s-parked").parked.is_some());
    assert_eq!(
        bell(&all, "s-parked", now),
        None,
        "Needs you leaves a parked one out"
    );
    // The summary said "Needs you (1)" under Home's "All clear".
    assert_eq!(summary(&state, "s-parked").0, "parked");
}
