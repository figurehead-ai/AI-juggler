//! AJ-E2903: how an Account's sessions went in the last 7 days — errors
//! and retries over turns (only where the tool counts turns), stuck
//! episodes from AI Juggler's recorded states, context at finish where
//! reported, tokens per finished task (D-77); figures not kept yet are
//! pending, never zero; a session ended before the window isn't counted;
//! and settling keeps a session's figures once.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::features::quality::{account_quality_impl, settle_ended};
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, EndQuality, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe,
    MachineId, ProfileId, SessionId, SessionStatus, Timestamp, TokenCounts,
};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

fn seed_folder(state: &Arc<AppState>, dir: &Path, folder: &str, recipe: &str) {
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new(folder),
            harness: HarnessId::new("claude-code"),
            label: folder.into(),
            plan: None,
            state_dir: dir.join(folder),
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
            id: ProfileId::new(recipe),
            name: "Main".into(),
            account: AccountId::new(folder),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
}

fn record(
    dir: &Path,
    id: &str,
    recipe: &str,
    at_end: Option<EndQuality>,
    tokens: u64,
) -> InstanceRecord {
    let mut record = InstanceRecord {
        id: SessionId::new(id),
        conversation: None,
        profile: ProfileId::new(recipe),
        machine: MachineId::new("this-mac"),
        kind: InstanceKind::Terminal,
        project: None,
        cwd: dir.to_path_buf(),
        status: SessionStatus::Running,
        started_at: Timestamp::now(),
        last_activity: None,
        usage: Default::default(),
        attention: None,
        running: None,
        transcript_ref: None,
        window: None,
    };
    record.usage.at_end = at_end;
    record.usage.tokens = TokenCounts {
        input: tokens,
        ..Default::default()
    };
    record
}

/// Inserts `record` running, walks it through `path`, ending on its last status.
fn run(state: &Arc<AppState>, mut record: InstanceRecord, path: &[SessionStatus]) {
    let storage = state.storage.lock().unwrap();
    storage.sessions().insert(&record).unwrap();
    for status in path {
        record.status = *status;
        storage.sessions().update(&record).unwrap();
    }
}

#[test]
fn an_accounts_week_in_figures_with_their_sources() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let dir = work.path();
    seed_folder(&state, dir, "work-a", "rec-a");
    seed_folder(&state, dir, "work-b", "rec-b");
    seed_folder(&state, dir, "personal", "rec-p");
    let q = |errors, turns, used: Option<u64>, window: Option<u64>| {
        Some(EndQuality {
            errors,
            turns,
            context_used: used,
            context_window: window,
        })
    };
    use SessionStatus::*;
    run(
        &state,
        record(
            dir,
            "f1",
            "rec-a",
            q(3, 100, Some(140_000), Some(200_000)),
            1_000,
        ),
        &[Finished],
    );
    run(
        &state,
        record(
            dir,
            "f2",
            "rec-b",
            q(6, 200, Some(60_000), Some(200_000)),
            3_000,
        ),
        &[Stuck, Running, Finished],
    );
    run(
        &state,
        record(dir, "crash", "rec-a", q(0, 0, None, None), 500),
        &[Stuck, Failed],
    );
    run(
        &state,
        record(dir, "pending", "rec-a", None, 50),
        &[Stopped],
    );
    run(&state, record(dir, "live", "rec-a", None, 10), &[Stuck]);
    run(
        &state,
        record(dir, "theirs", "rec-p", q(9, 300, None, None), 99),
        &[Finished],
    );
    let folders = vec!["work-a".to_owned(), "work-b".to_owned()];
    let now = Timestamp::now().millis();
    let figures = account_quality_impl(&state, &folders, now).unwrap();
    assert_eq!(figures.tool, "claude-code");
    assert_eq!((figures.sessions, figures.pending), (4, 1));
    assert!(figures.errors_measured);
    assert_eq!(
        (figures.errors, figures.turns),
        (9, 300),
        "the crash's tool kept no turns: left out, not zero"
    );
    assert_eq!(
        figures.stuck_episodes, 3,
        "f2 once, the crash once, the live one once"
    );
    assert_eq!(figures.context_sessions, 2);
    assert!((figures.context_avg_pct.unwrap() - 50.0).abs() < 1e-9);
    assert_eq!(
        (figures.finished, figures.tokens_per_finished),
        (2, Some(2_000))
    );
    // A week later, nothing ended in the window: no sessions, nothing to show.
    let later = account_quality_impl(&state, &folders, now + 8 * 86_400_000).unwrap();
    assert_eq!(
        (later.sessions, later.finished, later.tokens_per_finished),
        (0, 0, None)
    );
}

#[test]
fn settling_keeps_an_ended_sessions_figures_once() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let dir = work.path();
    seed_folder(&state, dir, "work-a", "rec-a");
    run(
        &state,
        record(dir, "ended", "rec-a", None, 10),
        &[SessionStatus::Finished],
    );
    run(
        &state,
        record(dir, "open", "rec-a", None, 10),
        &[SessionStatus::Running],
    );
    assert_eq!(settle_ended(&state).unwrap(), 1);
    let storage = state.storage.lock().unwrap();
    // No transcript: kept as no turns (errors can't be counted), never re-read.
    assert_eq!(
        storage
            .sessions()
            .get(&SessionId::new("ended"))
            .unwrap()
            .usage
            .at_end,
        Some(EndQuality::default())
    );
    assert_eq!(
        storage
            .sessions()
            .get(&SessionId::new("open"))
            .unwrap()
            .usage
            .at_end,
        None
    );
    drop(storage);
    assert_eq!(settle_ended(&state).unwrap(), 0);
}
