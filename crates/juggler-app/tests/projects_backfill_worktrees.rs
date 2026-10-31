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
        .join(juggler_adapter::handover::mangled_dir(
            &cwd.to_string_lossy(),
        ));
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
    assert_eq!(commands::normalise_remote("C:\\repo"), None);
    assert_eq!(commands::normalise_remote("/srv/repo.git"), None);
}

/// Review of #84: a relative `gitdir` (submodules, `worktree.useRelativePaths`)
/// resolves from the `.git` file's folder, and the repository comes back in
/// one canonical spelling (a symlinked parent folder doesn't split it).
#[test]
fn relative_and_linked_worktree_paths_resolve_to_one_canonical_repository() {
    use juggler_core::storage::worktree_repo_root;
    let work = tempfile::tempdir().unwrap();
    let real = work.path().canonicalize().unwrap().join("real");
    let repo = real.join("app");
    git_repo(&repo, "git@github.com:o/app.git");
    std::fs::create_dir_all(repo.join(".git/worktrees/wt")).unwrap();
    let tree = repo.join(".claude/worktrees/wt");
    std::fs::create_dir_all(&tree).unwrap();
    std::fs::write(tree.join(".git"), "gitdir: ../../../.git/worktrees/wt\n").unwrap();
    assert_eq!(
        worktree_repo_root(&tree),
        Some(repo.clone()),
        "relative gitdir"
    );

    // The same tree reached through a link to its parent folder.
    let link = work.path().join("linked");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    std::fs::write(
        tree.join(".git"),
        format!("gitdir: {}\n", link.join("app/.git/worktrees/wt").display()),
    )
    .unwrap();
    assert_eq!(
        worktree_repo_root(&tree),
        Some(repo),
        "one canonical spelling"
    );
}

/// A worktree project that already has sessions is kept by the cleanup.
#[test]
fn a_worktree_project_with_sessions_is_kept() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let repo = root.join("Dev/app");
    git_repo(&repo, "git@github.com:o/app.git");
    let tree = repo.join(".claude/worktrees/x");
    worktree(&repo, &tree, "x");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
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
    .unwrap();
    let storage = state.storage.lock().unwrap();
    let profile = storage
        .profiles()
        .list_by_account(&juggler_core::model::AccountId::new(
            account.account.id.clone(),
        ))
        .unwrap()
        .remove(0);
    let project = storage.projects().touch(&tree, None).unwrap();
    use juggler_core::model::{
        InstanceKind, InstanceRecord, MachineId, SessionId, SessionStatus, Timestamp,
    };
    storage
        .sessions()
        .insert(&InstanceRecord {
            id: SessionId::new("s1"),
            conversation: None,
            profile: profile.id,
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: Some(project.id.clone()),
            cwd: tree.clone(),
            status: SessionStatus::Finished,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
    drop(storage);
    commands::projects_backfill_impl(&state).unwrap();
    let kept = state.storage.lock().unwrap().projects().list().unwrap();
    assert!(
        kept.iter().any(|p| p.id == project.id),
        "referenced by a session: kept"
    );
}

/// AJ-E1519: Claude Squad removes a session's worktree when it's paused
/// or killed. While its `state.json` lists the worktree, AI Juggler
/// remembers the repository, so the worktree's conversations stay with
/// it afterwards instead of becoming a project named `fix-login_18a3b2c4`.
#[test]
fn a_worktree_claude_squad_removed_stays_with_its_repository() {
    let work = tempfile::tempdir().unwrap();
    let work_root = work.path().canonicalize().unwrap();
    let home = work_root.join("home");
    let repo = home.join("Dev/app");
    git_repo(&repo, "git@github.com:owner/app.git");
    // Removed already: only its transcript remains, and Claude Squad's
    // record of it.
    let tree = home.join(".claude-squad/worktrees/fix-login_18a3b2c4");
    std::fs::create_dir_all(home.join(".claude-squad")).unwrap();
    std::fs::write(
        home.join(".claude-squad/state.json"),
        serde_json::json!({
            "instances": [{
                "title": "fix-login",
                "worktree": { "repo_path": repo, "worktree_path": tree }
            }]
        })
        .to_string(),
    )
    .unwrap();

    let config = work_root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    transcript(&config, &repo, "a");
    transcript(&config, &tree, "b");

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
    // Before AI Juggler read Claude Squad's record, the worktree was a
    // project of its own.
    state
        .storage
        .lock()
        .unwrap()
        .projects()
        .touch(&tree, None)
        .unwrap();

    commands::projects_backfill_with(&state, Some(&home)).unwrap();
    let projects = commands::projects_list_impl(&state).unwrap();
    let names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["app"], "no project of its own: {names:?}");
    assert_eq!(projects[0].history_sessions, 2);

    // Claude Squad forgets it (the session was killed): AI Juggler still
    // knows where it belongs.
    std::fs::write(
        home.join(".claude-squad/state.json"),
        r#"{"instances": []}"#,
    )
    .unwrap();
    commands::projects_backfill_with(&state, Some(&home)).unwrap();
    let conversations = juggler_app::features::conversations::commands::list_impl(&state).unwrap();
    let in_tree = conversations.iter().find(|c| c.id == "b").unwrap();
    assert_eq!(in_tree.project_id.as_deref(), Some(projects[0].id.as_str()));
}
