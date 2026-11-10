//! X2 (D-62.20, AJ-E3004): a pool of 50 opens at once. The backend's
//! share of opening one: its members' first page (each member's row and
//! card) and the canvas read for them. Prints a `[perf]` line.
#![allow(clippy::print_stdout, reason = "a budget test prints what it measured")]
// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic, reason = "a test: a failed setup step is the failure")]

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_app::features::pools;
use juggler_app::paging::PageRequest;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, Timestamp,
};

fn seed(dir: &Path) -> (Arc<AppState>, String) {
    let state = Arc::new(
        AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    let root = dir.join("web");
    std::fs::create_dir_all(&root).unwrap();
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("work"),
                harness: HarnessId::new("claude-code"),
                label: "work".into(),
                plan: None,
                state_dir: state.accounts_dir().join("work"),
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
                account: AccountId::new("work"),
                settings_overlay: Default::default(),
                default_cwd: None,
            })
            .unwrap();
        let project = storage.projects().touch(&root, Some("web")).unwrap().id;
        for n in 0..50 {
            storage
                .sessions()
                .insert(&InstanceRecord {
                    id: SessionId::new(format!("s-{n}")),
                    conversation: None,
                    profile: ProfileId::new("rec-work"),
                    machine: MachineId::new("this-mac"),
                    kind: InstanceKind::Terminal,
                    project: Some(project.clone()),
                    cwd: root.clone(),
                    status: SessionStatus::Finished,
                    started_at: Timestamp::now(),
                    last_activity: Some(Timestamp::now()),
                    usage: Default::default(),
                    attention: None,
                    running: None,
                    transcript_ref: None,
                    window: None,
                })
                .unwrap();
        }
    }
    let info = pools::pool_create_impl(&state, "Big", None).unwrap();
    let big = info
        .pools
        .iter()
        .find(|pool| pool.name == "Big")
        .unwrap()
        .id
        .clone();
    for n in 0..50 {
        pools::pool_move_impl(&state, &format!("s-{n}"), &big).unwrap();
    }
    (state, big)
}

fn median(mut runs: Vec<Duration>) -> Duration {
    runs.sort();
    runs[runs.len() / 2]
}

#[test]
fn a_pool_of_fifty_opens_inside_its_budget() {
    let dir = tempfile::tempdir().unwrap();
    let (state, big) = seed(dir.path());
    let mut members_runs = Vec::new();
    let mut canvas_runs = Vec::new();
    for _ in 0..7 {
        let at = Instant::now();
        let page = pools::members::members_page(&state, &big, &PageRequest::first(50)).unwrap();
        members_runs.push(at.elapsed());
        assert_eq!(page.items.len(), 50);
        let ids: Vec<String> = page
            .items
            .iter()
            .map(|row| row.session.id.clone())
            .collect();
        let at = Instant::now();
        pools::canvas::canvas_impl(&state, &big, ids).unwrap();
        canvas_runs.push(at.elapsed());
    }
    let (members, canvas) = (median(members_runs), median(canvas_runs));
    println!("[perf] pool of 50: members page {members:?}, canvas {canvas:?}");
    // Debug build, a shared Mac: ~5 ms and ~0.3 ms measured (2026-11-10).
    // The X2 walk's 120–140 ms wasn't here: the two reads waited one on
    // the other once the page opened (now read ahead, `prefetchPool`).
    assert!(
        members + canvas < Duration::from_millis(50),
        "a pool of 50's reads take {members:?} + {canvas:?}, past 50 ms"
    );
}
