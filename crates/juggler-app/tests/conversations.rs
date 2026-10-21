//! AJ-E1529: an account's conversations started outside AI Juggler are
//! listed from the tool's own files, folded into their project, can be
//! resumed from a terminal (the adapter's line) and continued in AI
//! Juggler, refused in words when that can't be done safely.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::features::conversations::{self, commands::list_impl};
use juggler_app::state::AppState;

fn transcript(config: &Path, cwd: &Path, id: &str, ask: &str) {
    let dir = config
        .join("projects")
        .join(cwd.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{id}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"{}\",\"gitBranch\":\"main\",\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"{ask}\"}}}}\n",
            cwd.display()
        ),
    )
    .unwrap();
}

#[test]
fn conversations_outside_ai_juggler_are_listed_resumable_and_continued_safely() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let repo = root.join("Dev/app");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let removed = repo.join(".claude/worktrees/gone"); // a removed worktree: only its transcript is left
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    transcript(&config, &repo, "c-1", "Fix the login test");
    transcript(&config, &removed, "c-2", "Tidy imports");

    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let account = commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap()
    .account;
    let account_id = account.id.to_string();
    commands::projects_backfill_impl(&state).unwrap();

    let listed = list_impl(&state).unwrap();
    assert_eq!(listed.len(), 2);
    let project = commands::projects_list_impl(&state)
        .unwrap()
        .into_iter()
        .find(|p| p.name == "app")
        .unwrap();
    for conversation in &listed {
        assert_eq!(conversation.account_label, "Work");
        assert_eq!(
            conversation.project_id.as_deref(),
            Some(project.id.as_str()),
            "both fold into the repo"
        );
        assert!(!conversation.live && conversation.session_id.is_none());
    }
    assert_eq!(project.history_sessions, 2);
    let first = listed.iter().find(|c| c.id == "c-1").unwrap();
    assert_eq!(first.title.as_deref(), Some("Fix the login test"));

    let line = conversations::resume_command_impl(&state, &account_id, "c-1").unwrap();
    assert!(
        line.contains("'--resume' 'c-1'") && line.contains(&format!("cd '{}'", repo.display())),
        "{line}"
    );

    // Its folder is gone: refused in words, nothing launched.
    let error = conversations::continue_impl(&state, &account_id, "c-2").unwrap_err();
    assert!(format!("{error}").contains("folder isn't there"), "{error}");
    // Not one of the account's: refused.
    assert!(conversations::continue_impl(&state, &account_id, "nope").is_err());
}
