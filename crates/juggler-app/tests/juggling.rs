//! ADR-0007 integration: the juggling anchor — launching instances in a
//! directory creates the tracked project, links every session to it, and
//! surfaces the account history that powers switch/hand-over.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;

fn claude_stub(dir: &Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("claude-stub");
    std::fs::write(
        &path,
        "#!/bin/sh\nmkdir -p \"$CLAUDE_CONFIG_DIR/projects/p\"\nprintf 'run' > \"$CLAUDE_CONFIG_DIR/projects/p/s.jsonl\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn two_accounts_same_directory_share_one_tracked_project() {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(&stub)));
    let state =
        Arc::new(AppState::init_with_registry(work.path().join("app-data"), registry).unwrap());

    // Two accounts, one shared project directory.
    let account_a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let account_b = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let project_dir = work.path().join("shared-project");
    std::fs::create_dir_all(&project_dir).unwrap();

    let profile_a = commands::create_profile(&state, &account_a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &account_b.id, "Main", None, None).unwrap();
    commands::launch_profile(
        &state,
        &profile_a.id,
        Some(project_dir.to_string_lossy().into_owned()),
    )
    .unwrap();
    commands::launch_profile(
        &state,
        &profile_b.id,
        Some(project_dir.to_string_lossy().into_owned()),
    )
    .unwrap();

    // Wait for both exits so statuses settle. The app's forwarder thread
    // (started by run()) persists lifecycle; tests drive the same function.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut exited = 0;
    while exited < 2 && std::time::Instant::now() < deadline {
        if let Some(event) = state.supervisor.recv_timeout(Duration::from_millis(200)) {
            if let juggler_supervisor::LifecycleEvent::Exited { .. } = event {
                exited += 1;
            }
            let storage = state.storage.lock().unwrap();
            commands::apply_lifecycle_event(&storage, event.clone(), false, None);
        }
    }
    assert_eq!(exited, 2);

    // Exactly one project exists, named after the directory, carrying both
    // accounts in its history and both instances in its totals.
    let projects = commands::projects_list_impl(&state).unwrap();
    assert_eq!(projects.len(), 1, "one directory → one project");
    let project = &projects[0];
    assert_eq!(project.name, "shared-project");
    assert!(project.root.ends_with("shared-project"));
    assert!(project.last_activity_ms.is_some());
    assert_eq!(project.instance_count, 2);
    let mut used = project.accounts_used.clone();
    used.sort();
    assert_eq!(used, vec!["Personal".to_string(), "Work Max".to_string()]);
    assert!(
        project.active_instance.is_none(),
        "both instances exited — nothing live"
    );

    // Every session row is linked to the project and typed terminal-kind.
    let sessions = {
        let storage = state.storage.lock().unwrap();
        storage.sessions().list(None).unwrap()
    };
    assert!(
        sessions
            .iter()
            .all(|s| s.project.as_ref().map(|p| p.as_str()) == Some(project.id.as_str()))
    );
    assert!(
        sessions
            .iter()
            .all(|s| s.kind == juggler_core::model::InstanceKind::Terminal)
    );

    // A live instance surfaces as the project's active instance.
    commands::launch_profile(
        &state,
        &profile_a.id,
        Some(project_dir.to_string_lossy().into_owned()),
    )
    .unwrap();
    let projects = commands::projects_list_impl(&state).unwrap();
    let active = projects[0].active_instance.as_ref().expect("live instance");
    assert_eq!(active.account_label, "Work Max");
    assert_eq!(active.kind, "terminal");
    assert_eq!(projects[0].instance_count, 3);
}
