//! rc26 X2 (AJ-E3306): every pool membership change writes one line to
//! debug mode's `logs/debug.log` (D-75), and none while debug mode is
//! off. The walk took a member out on the canvas (the move command) and
//! the log stayed empty: the canvas's move and its role and lead changes
//! logged nothing, only the rule editor's join and leave did.
//!
//! Its own test binary: debug mode is process-wide (the layer, the flag,
//! the log's path).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test: a failed step fails the test"
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::features::app::diagnostics::debug_log;
use juggler_app::features::pools::{self, PoolsInfo};
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, Timestamp,
};
use juggler_core::storage::NO_PROJECT_POOL_ID;

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

fn account(state: &Arc<AppState>, id: &str) {
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new(id),
            harness: HarnessId::new("claude-code"),
            label: id.into(),
            plan: None,
            state_dir: state.accounts_dir().join(id),
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

fn session(state: &Arc<AppState>, id: &str, cwd: &Path) -> SessionId {
    let session = SessionId::new(id);
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: session.clone(),
            conversation: None,
            profile: ProfileId::new("rec-work"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: cwd.to_path_buf(),
            status: SessionStatus::Running,
            started_at: Timestamp::now(),
            last_activity: Some(Timestamp::now()),
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
    session
}

fn pool_id(info: &PoolsInfo, name: &str) -> String {
    info.pools
        .iter()
        .find(|pool| pool.name == name)
        .unwrap_or_else(|| panic!("no pool {name}"))
        .id
        .clone()
}

/// The pool lines in the log on disk.
fn pool_lines(state: &Arc<AppState>) -> Vec<String> {
    debug_log::text(&state.data_dir, 1024 * 1024)
        .lines()
        .filter(|line| line.split(' ').nth(1) == Some("pool"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_membership_change_writes_its_line_in_debug_mode_and_none_when_off() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    {
        let storage = state.storage.lock().unwrap();
        debug_log::install(&state.data_dir, &storage);
    }
    account(&state, "work");
    let a = session(&state, "s-a", dir.path());
    let b = session(&state, "s-b", dir.path());
    let team = pool_id(
        &pools::pool_create_impl(&state, "Team", None).unwrap(),
        "Team",
    );

    // Off: a move, a role, a lead, a rule joined and left, nothing written.
    pools::pool_move_impl(&state, a.as_str(), &team).unwrap();
    pools::pool_set_role_impl(&state, a.as_str(), "reviewer", Some(&team)).unwrap();
    pools::pool_set_role_impl(&state, a.as_str(), "lead", Some(&team)).unwrap();
    pools::model::join_impl(&state, &team, "app:rec-work", "member", None, &[]).unwrap();
    pools::model::leave_impl(&state, &team, "app:rec-work").unwrap();
    pools::pool_move_impl(&state, a.as_str(), NO_PROJECT_POOL_ID).unwrap();
    assert!(
        !state.data_dir.join(debug_log::RELATIVE).exists(),
        "debug mode off writes no log: {:?}",
        pool_lines(&state)
    );

    // On: each change, one line, in its order.
    {
        let storage = state.storage.lock().unwrap();
        debug_log::set(&storage, true).unwrap();
    }
    pools::pool_move_impl(&state, b.as_str(), &team).unwrap();
    pools::pool_set_role_impl(&state, b.as_str(), "reviewer", Some(&team)).unwrap();
    pools::pool_set_role_impl(&state, b.as_str(), "lead", Some(&team)).unwrap();
    pools::model::join_impl(&state, &team, "app:rec-work", "observer", None, &[]).unwrap();
    pools::model::leave_impl(&state, &team, "app:rec-work").unwrap();
    // The canvas's Take out of the pool: back to its own pool.
    pools::pool_move_impl(&state, b.as_str(), NO_PROJECT_POOL_ID).unwrap();

    let lines = pool_lines(&state);
    let steps: Vec<&str> = lines
        .iter()
        .map(|line| line.split(' ').nth(2).unwrap_or("-"))
        .collect();
    assert_eq!(
        steps,
        ["moved", "role", "lead", "joined", "left", "moved"],
        "{lines:#?}"
    );
    let has = |at: usize, words: &[&str]| {
        for word in words {
            assert!(
                lines[at].contains(word),
                "line {at} lacks {word}: {}",
                lines[at]
            );
        }
    };
    has(
        0,
        &[&format!("pool={team}"), "member=Session", "role=member"],
    );
    has(1, &[&format!("pool={team}"), "role=reviewer"]);
    has(2, &[&format!("pool={team}"), "role=lead"]);
    has(3, &[&format!("pool={team}"), "member=App"]);
    has(
        5,
        &[&format!("pool={NO_PROJECT_POOL_ID}"), "member=Session"],
    );
    // The member's kind only, never its name or id.
    assert!(
        lines
            .iter()
            .all(|line| !line.contains("s-b") && !line.contains("rec-work")),
        "{lines:#?}"
    );
}
