//! AJ-E1002 integration over a real git repository (stub `claude`).
//! Proves: a second Instance on a busy project is offered its own
//! worktree (`juggler/<id>`) at a shown path; a non-git project says
//! `Unavailable — not a git project`; two Instances editing the same file
//! concurrently in their own checkouts lose nothing; the Instance shows
//! its branch and worktree; removing a worktree with uncommitted work is
//! refused until its folder name is typed back, and the branch survives.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::state::AppState;
use juggler_app::{commands, worktree};
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

fn claude_stub(dir: &Path) -> PathBuf {
    let path = dir.join("claude-stub");
    stand_in::write(&path, "#!/bin/sh\nsleep 30\n");
    path
}

fn app_state(dir: &Path) -> Arc<AppState> {
    let stub = claude_stub(dir);
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn repo(dir: &Path) -> PathBuf {
    let root = dir.join("storefront");
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    std::fs::write(root.join("app.txt"), "v1\n").unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-q", "-m", "first"]);
    root.canonicalize().unwrap()
}

fn set_status(state: &Arc<AppState>, session: &str, status: juggler_core::model::SessionStatus) {
    let storage = state.storage.lock().unwrap();
    let mut row = storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(session))
        .unwrap();
    row.status = status;
    storage.sessions().update(&row).unwrap();
}

#[test]
fn a_second_instance_gets_its_own_worktree_and_nothing_is_overwritten() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let root = repo(work.path());
    let cwd = root.to_string_lossy().into_owned();
    let a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let b = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let recipe_a = commands::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let recipe_b = commands::create_profile(&state, &b.id, "Main", None, None).unwrap();

    // Nobody works here yet: no offer.
    assert!(
        worktree::isolation_offer_impl(&state, &cwd)
            .unwrap()
            .is_none()
    );
    let first = commands::launch_profile(&state, &recipe_a.id, Some(cwd.clone())).unwrap();

    // The second launch is offered its own worktree, path shown.
    let offer = worktree::isolation_offer_impl(&state, &cwd)
        .unwrap()
        .unwrap();
    assert!(offer.available);
    assert_eq!(offer.in_use_by, ["Work Max"]);
    assert_eq!(offer.base_branch.as_deref(), Some("main"));
    let branch = offer.proposed_branch.clone().unwrap();
    assert!(branch.starts_with("juggler/"), "{branch}");
    let path = offer.proposed_path.clone().unwrap();
    assert!(path.contains("storefront.juggler"), "{path}");

    let started = Instant::now();
    let created = worktree::worktree_create_impl(&state, &cwd, &path, &branch).unwrap();
    let took = started.elapsed();
    eprintln!("[perf] worktree create: {took:?}");
    assert!(took < Duration::from_secs(5));
    assert!(git(&root, &["branch", "--list", &branch]).contains(&branch));
    // Creating over an existing path is refused.
    assert!(worktree::worktree_create_impl(&state, &cwd, &path, "juggler/other").is_err());

    let second = commands::launch_profile(&state, &recipe_b.id, Some(created.clone())).unwrap();
    let rows = {
        let storage = state.storage.lock().unwrap();
        commands::session_infos_snapshot(&storage).unwrap()
    };
    let row = rows.iter().find(|row| row.id == second.session_id).unwrap();
    assert_eq!(
        row.git_branch.as_deref(),
        Some(branch.as_str()),
        "the branch of a linked worktree"
    );
    assert_eq!(row.git_worktree, Some(true));

    // Both Instances edit app.txt at once — each in its own checkout.
    let main_file = root.join("app.txt");
    let tree_file = Path::new(&created).join("app.txt");
    let writers: Vec<_> = [
        (main_file.clone(), "from Work Max\n"),
        (tree_file.clone(), "from Personal\n"),
    ]
    .into_iter()
    .map(|(file, text)| {
        std::thread::spawn(move || {
            for _ in 0..50 {
                std::fs::write(&file, text).unwrap();
            }
        })
    })
    .collect();
    for writer in writers {
        writer.join().unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&main_file).unwrap(),
        "from Work Max\n"
    );
    assert_eq!(
        std::fs::read_to_string(&tree_file).unwrap(),
        "from Personal\n",
        "zero silent overwrites"
    );

    // On focus: the dirty state, read now.
    let status = worktree::git_status_refresh_impl(&state, &second.session_id)
        .unwrap()
        .unwrap();
    assert_eq!(status.changed, 1);
    assert!(status.worktree && status.created_by_juggler);

    // Removal: refused while the Instance runs in it…
    assert!(worktree::worktree_remove_impl(&state, &created, None).is_err());
    set_status(
        &state,
        &second.session_id,
        juggler_core::model::SessionStatus::Stopped,
    );
    // …and with uncommitted work, until the folder name is typed back.
    std::fs::write(Path::new(&created).join("notes.md"), "draft\n").unwrap();
    let preview = worktree::worktree_removal_preview_impl(&state, &created).unwrap();
    assert!(preview.needs_confirmation);
    assert_eq!((preview.changed, preview.untracked), (1, 1));
    assert!(
        preview
            .summary
            .contains("Discards 1 changed and 1 untracked files"),
        "{}",
        preview.summary
    );
    assert!(worktree::worktree_remove_impl(&state, &created, None).is_err());
    assert!(worktree::worktree_remove_impl(&state, &created, Some("wrong")).is_err());
    assert!(Path::new(&created).is_dir(), "nothing was removed");
    worktree::worktree_remove_impl(&state, &created, Some(&preview.folder)).unwrap();
    assert!(!Path::new(&created).exists());
    assert!(
        git(&root, &["branch", "--list", &branch]).contains(&branch),
        "the branch (and its commits) stays"
    );
    let _ = first;
}

#[test]
fn a_non_git_project_says_isolation_is_unavailable() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let plain = work.path().join("notes");
    std::fs::create_dir_all(&plain).unwrap();
    let cwd = plain.canonicalize().unwrap().to_string_lossy().into_owned();
    let a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let recipe = commands::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let _ = commands::launch_profile(&state, &recipe.id, Some(cwd.clone())).unwrap();
    let offer = worktree::isolation_offer_impl(&state, &cwd)
        .unwrap()
        .unwrap();
    assert!(!offer.available);
    assert_eq!(
        offer.unavailable_reason.as_deref(),
        Some("Unavailable — not a git project")
    );
    assert!(offer.proposed_path.is_none(), "no fake alternative");
    assert!(worktree::worktree_create_impl(&state, &cwd, "/tmp/x", "juggler/x").is_err());
}

#[test]
fn only_juggler_worktrees_and_juggler_branches_are_touched() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let root = repo(work.path());
    let cwd = root.to_string_lossy().into_owned();
    // A branch outside juggler/ is refused.
    assert!(worktree::worktree_create_impl(&state, &cwd, &format!("{cwd}-x"), "main2").is_err());
    // A worktree the user made by hand is never removed from here.
    let manual = work.path().join("manual");
    git(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "feature",
            manual.to_str().unwrap(),
        ],
    );
    assert!(
        worktree::worktree_remove_impl(&state, manual.to_str().unwrap(), Some("manual")).is_err()
    );
    assert!(manual.is_dir());
}
