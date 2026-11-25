//! AJ-E2326 R1: every start path honours a move's hold on a conversation:
//! Continue in AI Juggler (the sessions' one start, which Resume, a
//! pickup and an epoch's Resume share) is refused while a move holds it,
//! and for a short while after a refused or undone move (it is opening
//! again where it was), never a second writer on it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "test crates are pragmatic by policy (QUALITY.md §2)"
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::features::conversations;
use juggler_app::features::places;
use juggler_app::state::AppState;

const C_1: &str = "1c1c1c1c-0000-4000-8000-000000000001";

fn transcript(config: &Path, cwd: &Path, id: &str) {
    let dir = config
        .join("projects")
        .join(cwd.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{id}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"{}\",\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"Fix the login test\"}}}}\n",
            cwd.display()
        ),
    )
    .unwrap();
}

/// An account with one conversation outside AI Juggler, in a folder that
/// is there (so Continue would launch it).
fn lab(root: &Path) -> (Arc<AppState>, String) {
    let repo = root.join("Dev/app");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    transcript(&config, &repo, C_1);
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let account = juggler_app::features::account::import::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap()
    .account;
    juggler_app::features::projects::backfill::projects_backfill_impl(&state).unwrap();
    (state, account.id.to_string())
}

#[test]
fn continue_is_refused_while_a_move_holds_the_conversation() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let (state, account) = lab(&root);
    let hold = places::ConversationHold::take(&state, C_1, "m-1");
    let error = conversations::continue_impl(&state, &account, C_1).unwrap_err();
    assert!(
        error.to_string().contains("being moved to another account"),
        "{error}"
    );
    drop(hold);
    assert_eq!(places::conversation_held(&state, C_1), None);
}

#[test]
fn continue_is_refused_in_the_settle_window_after_a_refused_move() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let (state, account) = lab(&root);
    let now = juggler_core::model::Timestamp::now().millis();
    places::settle_conversation(&state, C_1, "m-1", now + places::SETTLE_MS);
    let error = conversations::continue_impl(&state, &account, C_1).unwrap_err();
    assert!(
        error.to_string().contains("opening again where it was"),
        "{error}"
    );
    // Another conversation of the account is never held by it.
    assert_eq!(places::conversation_held(&state, "another"), None);
}
