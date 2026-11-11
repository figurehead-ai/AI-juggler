//! AJ-E2908: Resume on an ended session's page continues the same
//! conversation on the same account and launch settings; it says why when
//! it can't (still running, no conversation yet).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

#[path = "support/set_up.rs"]
mod set_up;
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::state::AppState;
use juggler_core::model::{ConversationId, SessionId, SessionStatus};

fn app_state(dir: &Path) -> Arc<AppState> {
    let stub = dir.join("claude-stub");
    stand_in::write(&stub, "#!/bin/sh\nexit 0\n");
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn set(state: &Arc<AppState>, id: &str, status: SessionStatus, conversation: Option<&str>) {
    let storage = state.storage.lock().unwrap();
    let mut row = storage
        .sessions()
        .get(&SessionId::new(id.to_owned()))
        .unwrap();
    row.status = status;
    row.conversation = conversation.map(|c| ConversationId::new(c.to_owned()));
    storage.sessions().update(&row).unwrap();
}

#[test]
fn an_ended_session_resumes_its_own_conversation_on_its_own_account() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let folder = work.path().join("shop");
    std::fs::create_dir_all(&folder).unwrap();
    let account = set_up::set_up(
        &state,
        juggler_app::features::account::create_account(&state, CLAUDE_CODE_ID, "Work", None)
            .unwrap(),
    );
    let profile =
        juggler_app::features::profile::create_profile(&state, &account.id, "Main", None, None)
            .unwrap();
    let launched = juggler_app::features::launch::launch_profile(
        &state,
        &profile.id,
        Some(folder.to_string_lossy().into_owned()),
    )
    .unwrap();

    set(
        &state,
        &launched.session_id,
        SessionStatus::Running,
        Some("c0c0c0c0-0000-4000-8000-000000000001"),
    );
    let refused =
        juggler_app::features::switch::resume::session_resume_impl(&state, &launched.session_id)
            .unwrap_err();
    assert!(refused.to_string().contains("still running"), "{refused}");

    set(&state, &launched.session_id, SessionStatus::Finished, None);
    let refused =
        juggler_app::features::switch::resume::session_resume_impl(&state, &launched.session_id)
            .unwrap_err();
    assert!(
        refused.to_string().contains("no conversation to resume"),
        "{refused}"
    );

    set(
        &state,
        &launched.session_id,
        SessionStatus::Finished,
        Some("c0c0c0c0-0000-4000-8000-000000000001"),
    );
    let resumed =
        juggler_app::features::switch::resume::session_resume_impl(&state, &launched.session_id)
            .unwrap();
    assert_ne!(resumed.session_id, launched.session_id, "a new session");
    let row = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(resumed.session_id.clone()))
        .unwrap();
    assert_eq!(
        row.conversation.as_ref().map(|c| c.as_str()),
        Some("c0c0c0c0-0000-4000-8000-000000000001"),
        "the same conversation"
    );
    assert_eq!(row.profile.as_str(), profile.id, "the same launch settings");
    assert_eq!(row.cwd, folder, "in its folder");
}

/// E3002-R3 step 10 (the rc32 walk): a resumed session is the same line
/// of work. It keeps the pools the original was put in by hand, with its
/// role there and its place on the canvas; the original's node is the
/// resumed one now (not left behind as "Stopped by you").
#[test]
fn a_resumed_session_keeps_the_pools_it_was_put_in() {
    use juggler_core::storage::{MemberRef, PoolRole};
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let folder = work.path().join("shop");
    std::fs::create_dir_all(&folder).unwrap();
    let account = set_up::set_up(
        &state,
        juggler_app::features::account::create_account(&state, CLAUDE_CODE_ID, "Work", None)
            .unwrap(),
    );
    let profile =
        juggler_app::features::profile::create_profile(&state, &account.id, "Main", None, None)
            .unwrap();
    let launched = juggler_app::features::launch::launch_profile(
        &state,
        &profile.id,
        Some(folder.to_string_lossy().into_owned()),
    )
    .unwrap();
    let old = SessionId::new(launched.session_id.clone());
    let pool = juggler_app::features::pools::pool_create_impl(&state, "Team", None)
        .unwrap()
        .pools
        .iter()
        .find(|pool| pool.name == "Team")
        .unwrap()
        .id
        .clone();
    {
        let storage = state.storage.lock().unwrap();
        storage
            .pools()
            .join(
                &pool,
                &MemberRef::Session(old.clone()),
                &[],
                PoolRole::Lead,
                None,
            )
            .unwrap();
        storage
            .pool_canvas()
            .place(
                &pool,
                &MemberRef::Session(old.clone()).to_wire(),
                120.0,
                80.0,
            )
            .unwrap();
    }
    set(
        &state,
        &launched.session_id,
        SessionStatus::Stopped,
        Some("c0c0c0c0-0000-4000-8000-000000000002"),
    );
    let resumed =
        juggler_app::features::switch::resume::session_resume_impl(&state, &launched.session_id)
            .unwrap();
    let new = SessionId::new(resumed.session_id.clone());
    assert_ne!(new, old);
    let storage = state.storage.lock().unwrap();
    let role = juggler_app::features::pools::members::role_in(&storage, &new, &pool).unwrap();
    assert_eq!(
        role,
        Some(PoolRole::Lead),
        "the resumed session is in Team, as its Lead"
    );
    let rows: Vec<_> = storage
        .pools()
        .members()
        .unwrap()
        .into_iter()
        .filter(|row| row.pool_id == pool)
        .map(|row| row.member)
        .collect();
    assert_eq!(
        rows,
        [MemberRef::Session(new.clone())],
        "one node: the resumed one"
    );
    let at = storage.pool_canvas().positions(&pool).unwrap();
    assert!(
        at.iter().any(
            |at| at.member_ref == MemberRef::Session(new.clone()).to_wire()
                && (at.x, at.y) == (120.0, 80.0)
        ),
        "{at:?}"
    );
}
