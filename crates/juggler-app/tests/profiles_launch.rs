//! M1-3 integration: profile CRUD → parallel launch through the supervisor
//! → cross-talk and write-containment guardrails (issue #19 acceptance).
//!
//! The `claude` binary is stubbed with a shell script that behaves like the
//! real one for our purposes: it writes a transcript into
//! `$CLAUDE_CONFIG_DIR/projects/…` and exits — proving isolation through the
//! same env path the real harness uses.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;

/// A `claude` stub: writes one transcript line under its isolated config
/// dir, echoes the config dir to stdout (cross-talk probe), exits 0.
fn claude_stub(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("claude-stub");
    std::fs::write(
        &path,
        "#!/bin/sh\n\
         mkdir -p \"$CLAUDE_CONFIG_DIR/projects/p\"\n\
         printf '%s' \"$CLAUDE_CONFIG_DIR\" > \"$CLAUDE_CONFIG_DIR/projects/p/s1.jsonl\"\n\
         echo \"config-dir: $CLAUDE_CONFIG_DIR\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn app_state(dir: &Path, stub: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(stub)));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

#[test]
fn two_accounts_launch_in_parallel_with_full_isolation() {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let state = app_state(work.path(), &stub);

    // Two accounts, two profiles, same harness.
    let account_a =
        commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", Some("max-20x".into()))
            .unwrap();
    let account_b = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    assert_ne!(account_a.id, account_b.id);

    let profile_a = commands::create_profile(
        &state,
        &account_a.id,
        "Work default",
        Some("opus".into()),
        Some(work.path().to_string_lossy().into_owned()),
    )
    .unwrap();
    let profile_b = commands::create_profile(
        &state,
        &account_b.id,
        "Personal default",
        None,
        Some(work.path().to_string_lossy().into_owned()),
    )
    .unwrap();

    // Launch both in parallel (no wait between the launches).
    let launch_a = commands::launch_profile(&state, &profile_a.id, None).unwrap();
    let launch_b = commands::launch_profile(&state, &profile_b.id, None).unwrap();
    assert_ne!(launch_a.session_id, launch_b.session_id);
    assert_ne!(launch_a.pid, launch_b.pid, "two live processes");

    // Both complete; wait for both EXIT events specifically (Started events
    // flow on the same channel and must not satisfy this loop).
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut exits = 0;
    while exits < 2 && std::time::Instant::now() < deadline {
        if let Some(juggler_supervisor::LifecycleEvent::Exited { .. }) =
            state.supervisor.recv_timeout(Duration::from_millis(200))
        {
            exits += 1;
        }
    }
    assert_eq!(exits, 2, "both sessions must exit");

    // The model overlay wrote settings.json only into account A's dir.
    let settings_a = state
        .data_dir
        .join("accounts")
        .join(CLAUDE_CODE_ID)
        .join(&account_a.id)
        .join("settings.json");
    let settings_b = state
        .data_dir
        .join("accounts")
        .join(CLAUDE_CODE_ID)
        .join(&account_b.id)
        .join("settings.json");
    assert!(settings_a.is_file(), "overlay settings written for A");
    assert!(!settings_b.is_file(), "no overlay for B");
    assert!(
        std::fs::read_to_string(&settings_a)
            .unwrap()
            .contains("opus")
    );

    // Cross-talk: each stub wrote its OWN config dir path into its
    // transcript; A's file must not contain B's dir and vice versa.
    let dir_a = state
        .data_dir
        .join("accounts")
        .join(CLAUDE_CODE_ID)
        .join(&account_a.id);
    let dir_b = state
        .data_dir
        .join("accounts")
        .join(CLAUDE_CODE_ID)
        .join(&account_b.id);
    let transcript_a = std::fs::read_to_string(dir_a.join("projects/p/s1.jsonl")).unwrap();
    let transcript_b = std::fs::read_to_string(dir_b.join("projects/p/s1.jsonl")).unwrap();
    assert_eq!(
        transcript_a,
        dir_a.to_str().unwrap(),
        "A wrote only its own dir"
    );
    assert_eq!(
        transcript_b,
        dir_b.to_str().unwrap(),
        "B wrote only its own dir"
    );
    assert_ne!(transcript_a, transcript_b, "no shared config dir");

    // Managed markers prove both dirs are ours and isolated.
    assert!(juggler_adapters::claude_code::is_managed_dir(&dir_a));
    assert!(juggler_adapters::claude_code::is_managed_dir(&dir_b));
}

#[test]
fn writes_are_confined_to_the_app_data_dir() {
    // The deterministic form of the "zero writes to ~/.claude" guardrail:
    // everything the app + stub harness touch lives inside the sandbox
    // (app data dir); nothing outside it was created.
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let state = app_state(work.path(), &stub);

    let account = commands::create_account(&state, CLAUDE_CODE_ID, "Solo", None).unwrap();
    let profile = commands::create_profile(&state, &account.id, "Solo run", None, None).unwrap();
    let launch = commands::launch_profile(
        &state,
        &profile.id,
        Some(work.path().to_string_lossy().into_owned()),
    )
    .unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut exits = 0;
    while exits < 1 && std::time::Instant::now() < deadline {
        if let Some(juggler_supervisor::LifecycleEvent::Exited { .. }) =
            state.supervisor.recv_timeout(Duration::from_millis(200))
        {
            exits += 1;
        }
    }
    assert_eq!(exits, 1, "session must exit");

    // Every path the app created lives under the sandbox; the only trees
    // are the app data dir and the stub/cwd scratch.
    let sandbox = work.path();
    let mut unexpected: Vec<PathBuf> = Vec::new();
    collect_created(work.path(), sandbox, &mut unexpected);
    assert!(
        unexpected.is_empty(),
        "files created outside expectations: {unexpected:?}"
    );
    assert!(launch.pid > 0);
}

/// Walks the sandbox and collects paths that are neither under the app data
/// dir nor the stub script nor test scratch inside the sandbox root itself.
fn collect_created(dir: &Path, sandbox: &Path, unexpected: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_created(&path, sandbox, unexpected);
            continue;
        }
        let rel = path.strip_prefix(sandbox).unwrap();
        let top = rel
            .iter()
            .next()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        // Allowed: app-data/** and the stub itself at the root.
        if top == "app-data" || rel.to_str() == Some("claude-stub") {
            continue;
        }
        unexpected.push(path);
    }
}
