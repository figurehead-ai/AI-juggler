//! AJ-E1519 evidence: the Claude Squad and Vibe Kanban importers over
//! fixture homes in each state: their files as they write them, a
//! schema or shape we don't know, and nothing there. Folders and the two
//! known variables only; nothing else they hold comes back.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};

use juggler_adapters::other_managers::{
    CLAUDE_SQUAD, ManagedWorktree, VIBE_KANBAN, aliases, worktrees,
};

/// A repository: a folder with a `.git` folder.
fn repo(at: &Path) -> PathBuf {
    std::fs::create_dir_all(at.join(".git")).unwrap();
    std::fs::canonicalize(at).unwrap()
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Claude Squad's `state.json` as it writes it: two sessions, one of
/// whose worktrees it has removed (a paused session), and one whose
/// repository is gone.
#[test]
fn claude_squad_worktrees_come_with_their_repositories() {
    let home = tempfile::tempdir().unwrap();
    let app = repo(&home.path().join("Dev/app"));
    let live = home
        .path()
        .join(".claude-squad/worktrees/fix-login_18a3b2c4");
    std::fs::create_dir_all(&live).unwrap();
    let paused = home.path().join(".claude-squad/worktrees/docs_77aa");
    let state = serde_json::json!({
        "help_screens_seen": 3,
        "instances": [
            {
                "title": "fix-login", "path": app, "branch": "dev/fix-login",
                "status": 0, "program": "claude --prompt 'a secret plan'",
                "worktree": {
                    "repo_path": app, "worktree_path": live,
                    "session_name": "fix-login", "branch_name": "dev/fix-login",
                    "base_commit_sha": "abc", "is_existing_branch": false
                },
                "diff_stats": { "added": 3, "removed": 1, "content": "the diff itself" }
            },
            {
                "title": "docs", "worktree": { "repo_path": app, "worktree_path": paused }
            },
            {
                "title": "gone", "worktree": {
                    "repo_path": home.path().join("Dev/moved"),
                    "worktree_path": home.path().join(".claude-squad/worktrees/gone_1")
                }
            }
        ]
    });
    write(
        &home.path().join(".claude-squad/state.json"),
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    let found = worktrees(home.path());
    assert_eq!(
        found,
        vec![
            ManagedWorktree {
                manager: CLAUDE_SQUAD,
                worktree: std::fs::canonicalize(&live).unwrap(),
                repo: app.clone(),
            },
            ManagedWorktree {
                manager: CLAUDE_SQUAD,
                worktree: paused,
                repo: app,
            },
        ],
        "a removed worktree still says where it belongs; a gone repository doesn't"
    );
    let words = format!("{found:?}");
    assert!(!words.contains("secret plan") && !words.contains("the diff itself"));
}

#[test]
fn a_claude_squad_state_we_dont_know_reads_as_nothing() {
    let home = tempfile::tempdir().unwrap();
    for text in [
        "{ not json",
        r#"{"instances": "[]"}"#,
        r#"{"instances": [{"worktree": 7}]}"#,
    ] {
        write(&home.path().join(".claude-squad/state.json"), text);
        assert!(worktrees(home.path()).is_empty(), "{text}");
    }
    assert!(worktrees(tempfile::tempdir().unwrap().path()).is_empty());
}

/// Claude Squad's profiles start programs through the shell: a leading
/// `CLAUDE_CONFIG_DIR=` / `CODEX_HOME=` picks the account.
#[test]
fn claude_squad_profiles_offer_the_folders_they_point_at() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".claude-work")).unwrap();
    std::fs::create_dir_all(home.path().join("codex-b")).unwrap();
    let config = serde_json::json!({
        "default_program": "claude",
        "auto_yes": false,
        "branch_prefix": "dev/",
        "profiles": [
            { "name": "work", "program": "CLAUDE_CONFIG_DIR=~/.claude-work claude" },
            { "name": "b", "program": format!("env CODEX_HOME={} codex", home.path().join("codex-b").display()) },
            { "name": "nowhere", "program": "CLAUDE_CONFIG_DIR=~/.not-there claude" }
        ]
    });
    write(
        &home.path().join(".claude-squad/config.json"),
        &config.to_string(),
    );

    let hints = aliases(home.path());
    let got: Vec<_> = hints
        .iter()
        .map(|hint| {
            (
                hint.variable.as_str(),
                hint.path.clone(),
                hint.set_by.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            (
                "CLAUDE_CONFIG_DIR",
                home.path().join(".claude-work"),
                Some(CLAUDE_SQUAD)
            ),
            (
                "CODEX_HOME",
                home.path().join("codex-b"),
                Some(CLAUDE_SQUAD)
            ),
        ],
        "a folder that isn't there isn't offered"
    );
    assert!(
        hints
            .iter()
            .all(|hint| hint.source_file.ends_with(".claude-squad/config.json"))
    );
}

fn kanban_dir(home: &Path) -> PathBuf {
    home.join("Library/Application Support/ai.bloop.vibe-kanban")
}

/// Vibe Kanban's database as its current schema has it: a workspace's
/// worktree is `<container_ref>/<repo name>`, one per repository.
#[test]
fn vibe_kanban_workspaces_come_with_their_repositories() {
    let home = tempfile::tempdir().unwrap();
    let app = repo(&home.path().join("Dev/app"));
    let api = repo(&home.path().join("Dev/api"));
    let container = home.path().join("tmp/vibe-kanban/worktrees/1a2b-fix-login");
    std::fs::create_dir_all(container.join("app")).unwrap();
    let dir = kanban_dir(home.path());
    std::fs::create_dir_all(&dir).unwrap();
    {
        let db = rusqlite::Connection::open(dir.join("db.v2.sqlite")).unwrap();
        db.execute_batch(
            "CREATE TABLE workspaces (id BLOB PRIMARY KEY, container_ref TEXT, branch TEXT, name TEXT, setup_prompt TEXT);
             CREATE TABLE repos (id BLOB PRIMARY KEY, path TEXT, name TEXT, display_name TEXT);
             CREATE TABLE workspace_repos (id BLOB PRIMARY KEY, workspace_id BLOB, repo_id BLOB, target_branch TEXT);",
        )
        .unwrap();
        db.execute(
            "INSERT INTO workspaces VALUES (x'01', ?1, 'vk/1a2b-fix-login', 'Fix login', 'a secret plan'), (x'02', NULL, 'vk/x', 'Not started', NULL)",
            [container.to_string_lossy()],
        )
        .unwrap();
        db.execute(
            "INSERT INTO repos VALUES (x'0a', ?1, 'app', 'App'), (x'0b', ?2, 'api', 'API')",
            [app.to_string_lossy(), api.to_string_lossy()],
        )
        .unwrap();
        db.execute_batch(
            "INSERT INTO workspace_repos VALUES (x'f1', x'01', x'0a', 'main'), (x'f2', x'01', x'0b', 'main'), (x'f3', x'02', x'0a', 'main');",
        )
        .unwrap();
    }

    let mut found = worktrees(home.path());
    found.sort_by(|a, b| a.repo.cmp(&b.repo));
    assert_eq!(
        found,
        vec![
            ManagedWorktree {
                manager: VIBE_KANBAN,
                worktree: container.join("api"),
                repo: api,
            },
            ManagedWorktree {
                manager: VIBE_KANBAN,
                worktree: std::fs::canonicalize(container.join("app")).unwrap(),
                repo: app,
            },
        ],
        "one per repository; a workspace not started yet has no worktree"
    );
    assert!(!format!("{found:?}").contains("secret plan"));
}

#[test]
fn a_vibe_kanban_schema_we_dont_know_reads_as_nothing() {
    let home = tempfile::tempdir().unwrap();
    let dir = kanban_dir(home.path());
    std::fs::create_dir_all(&dir).unwrap();
    {
        let db = rusqlite::Connection::open(dir.join("db.v2.sqlite")).unwrap();
        db.execute_batch("CREATE TABLE task_attempts (id BLOB, container_ref TEXT);")
            .unwrap();
    }
    assert!(worktrees(home.path()).is_empty());
    std::fs::write(dir.join("db.v2.sqlite"), "not a database").unwrap();
    assert!(worktrees(home.path()).is_empty());
}

/// Vibe Kanban's executor profiles: only `CLAUDE_CONFIG_DIR` and
/// `CODEX_HOME` in each variant's `env`; any other variable there (an API
/// key) is never read into anything.
#[test]
fn vibe_kanban_profiles_offer_the_folders_they_point_at() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("claude-max")).unwrap();
    let profiles = serde_json::json!({
        "executors": {
            "CLAUDE_CODE": {
                "DEFAULT": { "CLAUDE_CODE": { "dangerously_skip_permissions": true } },
                "MAX": { "CLAUDE_CODE": { "env": {
                    "CLAUDE_CONFIG_DIR": "~/claude-max",
                    "ANTHROPIC_API_KEY": "sk-ant-never-read"
                } } }
            },
            "CODEX": {
                "DEFAULT": { "CODEX": { "sandbox": "danger-full-access", "env": { "OPENAI_API_KEY": "sk-never-read" } } }
            }
        }
    });
    write(
        &kanban_dir(home.path()).join("profiles.json"),
        &profiles.to_string(),
    );

    let hints = aliases(home.path());
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].variable, "CLAUDE_CONFIG_DIR");
    assert_eq!(hints[0].path, home.path().join("claude-max"));
    assert_eq!(hints[0].set_by.as_deref(), Some(VIBE_KANBAN));
    assert!(!format!("{hints:?}").contains("never-read"));
}

/// The static guardrail: the importer's code never names a sign-in or
/// key file, so it can't open one.
#[test]
fn the_importers_never_name_a_sign_in_or_key_file() {
    let source = include_str!("../src/other_managers.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for name in [
        "credentials",
        "signing_key",
        "auth.json",
        "oauth",
        "token",
        "github",
    ] {
        assert!(!code.to_lowercase().contains(name), "{name}");
    }
}
