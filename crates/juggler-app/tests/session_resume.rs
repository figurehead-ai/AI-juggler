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
        Some("conv-1"),
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
        Some("conv-1"),
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
        Some("conv-1"),
        "the same conversation"
    );
    assert_eq!(row.profile.as_str(), profile.id, "the same launch settings");
    assert_eq!(row.cwd, folder, "in its folder");
}
