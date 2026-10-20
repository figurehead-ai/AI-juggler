//! The rail's project list (0.0.8 dogfood): a worktree's history belongs
//! to its repository, earlier worktree-only rows fold away, the last time
//! the user worked somewhere is the transcript's, and clones of one
//! repository share an identity.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;

fn git_repo(dir: &Path, remote: &str) {
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    std::fs::write(
        dir.join(".git/config"),
        format!("[core]\n\tbare = false\n[remote \"origin\"]\n\turl = {remote}\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n"),
    )
    .unwrap();
}

fn worktree(repo: &Path, at: &Path, name: &str) {
    std::fs::create_dir_all(repo.join(".git/worktrees").join(name)).unwrap();
    std::fs::create_dir_all(at).unwrap();
    std::fs::write(
        at.join(".git"),
        format!(
            "gitdir: {}\n",
            repo.join(".git/worktrees").join(name).display()
        ),
    )
    .unwrap();
}

/// A transcript for `cwd` inside an account's folder.
fn transcript(config: &Path, cwd: &Path, name: &str) {
    let dir = config
        .join("projects")
        .join(cwd.to_string_lossy().replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{name}.jsonl")),
        format!("{{\"type\":\"user\",\"cwd\":\"{}\"}}\n", cwd.display()),
    )
    .unwrap();
}

#[test]
fn worktrees_fold_into_their_repository_and_clones_share_an_identity() {
    let work = tempfile::tempdir().unwrap();
    let work_root = work.path().canonicalize().unwrap();
    let repo = work_root.join("Dev/app");
    git_repo(&repo, "git@github.com:Owner/App.git");
    let tree = repo.join(".claude/worktrees/feature-x");
    worktree(&repo, &tree, "feature-x");
    let clone = work_root.join("Dev/app-2");
    git_repo(&clone, "https://someone@github.com/owner/app");

    let config = work_root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    transcript(&config, &repo, "a");
    transcript(&config, &tree, "b");
    transcript(&config, &tree, "c");
    transcript(&config, &clone, "d");

    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state =
        Arc::new(AppState::init_with_registry(work_root.join("app-data"), registry).unwrap());
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

    // An earlier backfill made a row for the worktree itself.
    state
        .storage
        .lock()
        .unwrap()
        .projects()
        .touch(&tree, None)
        .unwrap();

    commands::projects_backfill_impl(&state).unwrap();
    let projects = commands::projects_list_impl(&state).unwrap();
    let names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
    assert!(
        !names.contains(&"feature-x"),
        "the worktree folds into its repository: {names:?}"
    );
    let app = projects.iter().find(|p| p.name == "app").unwrap();
    assert_eq!(
        app.history_sessions, 3,
        "the worktree's two conversations count for the repository"
    );
    assert!(
        app.history_last_used_ms.is_some(),
        "when the user last worked there"
    );
    let other = projects.iter().find(|p| p.name == "app-2").unwrap();
    assert_eq!(app.repo.as_deref(), Some("github.com/owner/app"));
    assert_eq!(
        other.repo, app.repo,
        "clones of one repository share an identity"
    );
}

#[test]
fn remotes_normalise_to_host_owner_name() {
    for (url, want) in [
        ("git@github.com:Owner/Name.git", "github.com/owner/name"),
        (
            "https://user@github.com/Owner/Name",
            "github.com/owner/name",
        ),
        (
            "ssh://git@github.com:22/Owner/Name.git",
            "github.com/owner/name",
        ),
        (
            "https://gitlab.example.com/123org/tool.git",
            "gitlab.example.com/123org/tool",
        ),
    ] {
        assert_eq!(
            commands::normalise_remote(url).as_deref(),
            Some(want),
            "{url}"
        );
    }
    assert_eq!(commands::normalise_remote("not-a-url"), None);
}
