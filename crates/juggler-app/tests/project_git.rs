//! AJ-E1533: a project's page header reads its git state from the
//! project's own root, named by the project's id — its branch and what
//! isn't committed — and a folder that isn't a git checkout has none.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::features::projects::git::git_status_impl;
use juggler_app::state::AppState;

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?}");
}

fn transcript(config: &Path, cwd: &Path, id: &str) {
    let folder = config
        .join("projects")
        .join(juggler_adapter::handover::mangled_dir(
            &cwd.to_string_lossy(),
        ));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join(format!("{id}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"{}\",\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"hi\"}}}}\n",
            cwd.display()
        ),
    )
    .unwrap();
}

#[test]
fn a_projects_header_reads_its_branch_and_what_isnt_committed() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let repo = root.join("Dev/app");
    let plain = root.join("Dev/notes");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&plain).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("a.txt"), "one\n").unwrap();
    git(&repo, &["add", "a.txt"]);
    git(&repo, &["commit", "-q", "-m", "first"]);
    std::fs::write(repo.join("a.txt"), "two\n").unwrap(); // changed
    std::fs::write(repo.join("b.txt"), "new\n").unwrap(); // untracked

    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    transcript(&config, &repo, "c-1");
    transcript(&config, &plain, "c-2");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap();
    commands::projects_backfill_impl(&state).unwrap();
    let projects = commands::projects_list_impl(&state).unwrap();
    let id = |name: &str| projects.iter().find(|p| p.name == name).unwrap().id.clone();

    let status = git_status_impl(&state, &id("app"))
        .unwrap()
        .expect("a git checkout");
    assert_eq!(status.branch.as_deref(), Some("main"));
    assert_eq!((status.changed, status.untracked), (1, 1));
    assert!(!status.worktree);
    // Not a git checkout: nothing to say.
    assert!(git_status_impl(&state, &id("notes")).unwrap().is_none());
    // An id that isn't a project's: refused.
    assert!(git_status_impl(&state, "nope").is_err());
}
