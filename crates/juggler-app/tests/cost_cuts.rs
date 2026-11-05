//! AJ-E2902: Usage's cost cuts. A finished task is a session AI Juggler
//! ran that ended on its own as finished (D-77); wasted spend is what
//! went on sessions that crashed or were stopped while probably stuck;
//! each Account sums its folders, with its top project; sessions other
//! apps run have no known ending; a model without a list price is never
//! $0.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::features::usage_history::cuts::{Outcome, UsagePricesInfo, cuts_of, outcomes};
use juggler_app::features::usage_history::dto::{UsageSessionInfo, UsageShareInfo};
use juggler_app::features::usage_history::fold::Folded;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, Timestamp,
};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

fn record(dir: &Path, id: &str, status: SessionStatus) -> InstanceRecord {
    InstanceRecord {
        id: SessionId::new(id),
        conversation: None,
        profile: ProfileId::new("rec-work"),
        machine: MachineId::new("this-mac"),
        kind: InstanceKind::Terminal,
        project: None,
        cwd: dir.to_path_buf(),
        status,
        started_at: Timestamp::now(),
        last_activity: None,
        usage: Default::default(),
        attention: None,
        running: None,
        transcript_ref: None,
        window: None,
    }
}

#[test]
fn outcomes_read_the_ending_and_the_last_state_change() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-work"),
            harness: HarnessId::new("claude-code"),
            label: "Work".into(),
            plan: None,
            state_dir: work.path().join("state"),
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
            id: ProfileId::new("rec-work"),
            name: "Main".into(),
            account: AccountId::new("acc-work"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
    let dir = work.path();
    for (id, status) in [
        ("done", SessionStatus::Finished),
        ("crash", SessionStatus::Failed),
        ("stopped", SessionStatus::Stopped),
        ("live", SessionStatus::Running),
        ("handed", SessionStatus::HandedOver),
    ] {
        storage.sessions().insert(&record(dir, id, status)).unwrap();
    }
    // Stopped while stuck: probably stuck when it ended.
    storage
        .sessions()
        .insert(&record(dir, "was-stuck", SessionStatus::Stuck))
        .unwrap();
    storage
        .sessions()
        .update(&record(dir, "was-stuck", SessionStatus::Stopped))
        .unwrap();
    // Stuck once, then working again before the user stopped it: not stuck at the end.
    storage
        .sessions()
        .insert(&record(dir, "recovered", SessionStatus::Stuck))
        .unwrap();
    storage
        .sessions()
        .update(&record(dir, "recovered", SessionStatus::Running))
        .unwrap();
    storage
        .sessions()
        .update(&record(dir, "recovered", SessionStatus::Stopped))
        .unwrap();
    let found = outcomes(&storage).unwrap();
    assert_eq!(found.get("done"), Some(&Outcome::Finished));
    assert_eq!(found.get("crash"), Some(&Outcome::Crashed));
    assert_eq!(found.get("was-stuck"), Some(&Outcome::Stuck));
    for plain in ["stopped", "live", "handed", "recovered"] {
        assert_eq!(found.get(plain), None, "{plain}");
    }
}

fn session(
    profile: &str,
    id: &str,
    juggler: Option<&str>,
    project: Option<&str>,
    io: u64,
    usd: Option<f64>,
) -> UsageSessionInfo {
    UsageSessionInfo {
        profile: profile.into(),
        id: id.into(),
        session_id: juggler.map(Into::into),
        tool: "claude-code".into(),
        title: None,
        project_id: project.map(Into::into),
        io,
        cache: 0,
        usd,
        first_ms: 0,
        last_ms: 0,
        messages: 1,
    }
}

fn share(profile: &str, io: u64, usd: Option<f64>, sessions: u32) -> UsageShareInfo {
    UsageShareInfo {
        key: profile.into(),
        label: profile.into(),
        io,
        cache: 0,
        usd,
        sessions,
        profiles: vec![],
    }
}

fn prices() -> UsagePricesInfo {
    UsagePricesInfo {
        version: "v4".into(),
        captured: "2026-11-03".into(),
        stale: false,
        overridden: false,
    }
}

#[test]
fn cuts_count_finished_and_wasted_and_sum_accounts_with_their_top_project() {
    let now = Folded {
        sessions: vec![
            session("w1", "c1", Some("done-1"), Some("aij"), 1000, Some(1.5)),
            session("w2", "c2", Some("done-2"), Some("aij"), 3000, Some(2.5)),
            session("w1", "c3", Some("crash"), Some("site"), 500, Some(4.0)),
            session("p1", "c4", Some("stuck"), Some("site"), 800, Some(2.3)),
            session("p1", "c5", None, Some("notes"), 200, None),
            session("p1", "c6", Some("live"), None, 50, Some(0.1)),
        ],
        by_profile: vec![
            share("w1", 1500, Some(5.5), 2),
            share("w2", 3000, Some(2.5), 1),
            share("p1", 1050, Some(2.4), 3),
        ],
        ..Folded::default()
    };
    let outcomes: HashMap<String, Outcome> = [
        ("done-1", Outcome::Finished),
        ("done-2", Outcome::Finished),
        ("crash", Outcome::Crashed),
        ("stuck", Outcome::Stuck),
    ]
    .into_iter()
    .map(|(id, outcome)| (id.to_owned(), outcome))
    .collect();
    let account_of: juggler_app::features::usage_history::cuts::AccountOf = [
        ("w1", ("work", Some("acc-work"), "Work", "claude-code")),
        ("w2", ("work", Some("acc-work"), "Work", "claude-code")),
        ("p1", ("personal", Some("acc-p"), "Personal", "codex")),
    ]
    .into_iter()
    .map(|(folder, (key, id, label, tool))| {
        (
            folder.to_owned(),
            (
                key.to_owned(),
                id.map(Into::into),
                label.to_owned(),
                tool.to_owned(),
            ),
        )
    })
    .collect();
    let projects: HashMap<String, String> = [
        ("aij", "AIJuggler"),
        ("site", "Website"),
        ("notes", "Notes"),
    ]
    .into_iter()
    .map(|(a, b)| (a.to_owned(), b.to_owned()))
    .collect();
    let cuts = cuts_of(&now, &outcomes, &account_of, &projects, prices());
    assert_eq!((cuts.finished.count, cuts.finished.io), (2, 4000));
    assert!((cuts.finished.usd.unwrap() - 4.0).abs() < 1e-9);
    assert_eq!((cuts.wasted.count, cuts.crashed, cuts.stuck), (2, 1, 1));
    assert!((cuts.wasted.usd.unwrap() - 6.3).abs() < 1e-9);
    assert_eq!(cuts.unknown_endings, 1);
    // Work: both folders summed (one Account), costliest first; top project AIJuggler.
    let work = &cuts.by_account[0];
    assert_eq!(
        (work.key.as_str(), work.io, work.sessions),
        ("work", 4500, 3)
    );
    assert!((work.usd.unwrap() - 8.0).abs() < 1e-9);
    assert_eq!(
        work.top_project.as_ref().map(|top| top.label.as_str()),
        Some("AIJuggler")
    );
    assert!(!work.unpriced);
    // Personal: a session with no list price makes it unpriced, never $0.
    let personal = &cuts.by_account[1];
    assert!(personal.unpriced);
    assert_eq!(
        personal.top_project.as_ref().map(|top| top.label.as_str()),
        Some("Website")
    );
}

#[test]
fn nothing_finished_or_wasted_is_zero_counts_and_no_estimate() {
    let cuts = cuts_of(
        &Folded::default(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        prices(),
    );
    assert_eq!(cuts.finished.count, 0);
    assert_eq!(cuts.finished.usd, None);
    assert_eq!(cuts.wasted.usd, None);
    assert!(cuts.by_account.is_empty());
}
