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
use juggler_app::worktree;
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

/// The rc27 pool pass: after a quit and reopen, a session whose process
/// was gone still read as live, and New session asked the worktree
/// question for it. Only a session that really runs is counted: its
/// process under the supervisor, or a launch just under way.
#[test]
fn only_a_session_that_really_runs_is_counted_as_working_in_the_folder() {
    use juggler_core::model::{SessionId, SessionStatus, Timestamp};
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let root = repo(work.path());
    let cwd = root.to_string_lossy().into_owned();
    let a =
        juggler_app::features::account::create_account(&state, CLAUDE_CODE_ID, "Work Max", None)
            .unwrap();
    let recipe =
        juggler_app::features::profile::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let first =
        juggler_app::features::launch::launch_profile(&state, &recipe.id, Some(cwd.clone()))
            .unwrap();
    // Two rows a run before this one left live, with no process: one
    // running, one starting long ago.
    {
        let storage = state.storage.lock().unwrap();
        let mut ghost = storage
            .sessions()
            .get(&SessionId::new(first.session_id.as_str()))
            .unwrap();
        let old = Timestamp::from_millis(Timestamp::now().millis() - 3_600_000);
        ghost.started_at = old;
        ghost.last_activity = Some(old);
        for (id, status) in [
            ("ghost-running", SessionStatus::Running),
            ("ghost-starting", SessionStatus::Starting),
        ] {
            ghost.id = SessionId::new(id);
            ghost.status = status;
            storage.sessions().insert(&ghost).unwrap();
        }
    }
    let offer = worktree::isolation_offer_impl(&state, &cwd)
        .unwrap()
        .unwrap();
    let busy: Vec<&str> = offer.in_use.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(busy, [first.session_id.as_str()], "only the one that runs");
    // Its process ends: nobody works there.
    state
        .supervisor
        .stop(
            &SessionId::new(first.session_id.as_str()),
            Duration::from_secs(2),
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !state
        .supervisor
        .is_finished(&SessionId::new(first.session_id.as_str()))
    {
        assert!(Instant::now() < deadline, "the stub never ended");
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        worktree::isolation_offer_impl(&state, &cwd)
            .unwrap()
            .is_none(),
        "a finished process isn't working there"
    );
}

/// `worktree_create`'s steps: its folder vouched for, the path where the
/// offer proposes, then the worktree.
fn create(
    state: &Arc<AppState>,
    cwd: &str,
    path: &str,
    branch: &str,
) -> juggler_app::error::AppResult<String> {
    juggler_app::picked::work_folder(state, cwd)?;
    juggler_app::picked::vouch(path, juggler_app::picked::picked_as(&[]), |p| {
        worktree::is_proposed(Path::new(cwd), p)
    })?;
    worktree::worktree_create_impl(state, cwd, path, branch)
}

/// `launch_session`'s steps: its folder vouched for, then the launch.
fn launch(
    state: &Arc<AppState>,
    recipe: &str,
    cwd: &str,
) -> juggler_app::error::AppResult<juggler_app::features::launch::dto::LaunchResult> {
    juggler_app::picked::work_folder(state, cwd)?;
    juggler_app::features::launch::launch_profile(state, recipe, Some(cwd.to_owned()))
}

/// rc29 persona walks: "Its own worktree (recommended)" › "Create the
/// worktree and launch" always ended in "Choose that place again". The
/// create re-asked for the offer, which draws a fresh id, so the path the
/// user confirmed never matched; and the launch refused the new worktree,
/// which sits beside the project (`<project>.juggler/<id>`), not inside
/// it. The whole path runs through the checks the commands make
/// (`worktree_create`, `launch_session`: `create`, `launch` here), from a
/// folder named the way the user picked it (`/var/…` here, `/tmp/…` in
/// the lab), while git names the real one (`/private/…`).
#[test]
fn the_recommended_worktree_is_created_and_launched_in() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    repo(work.path());
    let picked = work.path().join("storefront");
    let cwd = picked.to_string_lossy().into_owned();
    let a =
        juggler_app::features::account::create_account(&state, CLAUDE_CODE_ID, "Work Max", None)
            .unwrap();
    let b =
        juggler_app::features::account::create_account(&state, CLAUDE_CODE_ID, "Personal", None)
            .unwrap();
    let recipe_a =
        juggler_app::features::profile::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let recipe_b =
        juggler_app::features::profile::create_profile(&state, &b.id, "Main", None, None).unwrap();
    let first = launch(&state, &recipe_a.id, &cwd);
    // Nothing vouches for the folder yet (no pick in a test): it starts
    // the way an earlier session would have.
    assert!(first.is_err(), "an unknown folder is refused");
    let first =
        juggler_app::features::launch::launch_profile(&state, &recipe_a.id, Some(cwd.clone()))
            .unwrap();

    let offer = worktree::isolation_offer_impl(&state, &cwd)
        .unwrap()
        .unwrap();
    let path = offer.proposed_path.unwrap();
    let branch = offer.proposed_branch.unwrap();
    // A place the page names is still refused, even beside the project.
    let home = Path::new(&path).parent().unwrap();
    for elsewhere in [
        home.join("not-an-id").to_string_lossy().into_owned(),
        work.path().join("abcdef").to_string_lossy().into_owned(),
    ] {
        assert!(
            create(&state, &cwd, &elsewhere, "juggler/x").is_err(),
            "{elsewhere}"
        );
    }
    let created = create(&state, &cwd, &path, &branch).unwrap();
    let second = launch(&state, &recipe_b.id, &created).unwrap();
    assert_ne!(second.session_id, first.session_id);
    // A folder beside it that AI Juggler didn't make stays unknown.
    let stray = home.join("fedcba");
    std::fs::create_dir_all(&stray).unwrap();
    assert!(!juggler_app::picked::known_work_folder(&state, &stray));
}

#[test]
fn a_second_instance_gets_its_own_worktree_and_nothing_is_overwritten() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let root = repo(work.path());
    let cwd = root.to_string_lossy().into_owned();
    let a =
        juggler_app::features::account::create_account(&state, CLAUDE_CODE_ID, "Work Max", None)
            .unwrap();
    let b =
        juggler_app::features::account::create_account(&state, CLAUDE_CODE_ID, "Personal", None)
            .unwrap();
    let recipe_a =
        juggler_app::features::profile::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let recipe_b =
        juggler_app::features::profile::create_profile(&state, &b.id, "Main", None, None).unwrap();

    // Nobody works here yet: no offer.
    assert!(
        worktree::isolation_offer_impl(&state, &cwd)
            .unwrap()
            .is_none()
    );
    let first =
        juggler_app::features::launch::launch_profile(&state, &recipe_a.id, Some(cwd.clone()))
            .unwrap();

    // The second launch is offered its own worktree, path shown.
    let offer = worktree::isolation_offer_impl(&state, &cwd)
        .unwrap()
        .unwrap();
    assert!(offer.available);
    let busy: Vec<&str> = offer.in_use.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(busy, [first.session_id.as_str()]);
    assert_eq!(offer.in_use[0].account_label, "Work Max");
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

    let second =
        juggler_app::features::launch::launch_profile(&state, &recipe_b.id, Some(created.clone()))
            .unwrap();
    let rows = {
        let storage = state.storage.lock().unwrap();
        juggler_app::features::sessions::read_model::session_infos_snapshot(&storage).unwrap()
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

    // On focus: the dirty state. A read is kept for up to 2 s (a focus
    // never runs git twice in a row), so a status read while the tree was
    // still clean can answer first: the test waits out that window, never
    // a fixed sleep (PR #201's CI read 0 inside it).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let status = loop {
        let status = worktree::git_status_refresh_impl(&state, &second.session_id)
            .unwrap()
            .unwrap();
        if status.changed == 1 || std::time::Instant::now() >= deadline {
            break status;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    };
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
    let a =
        juggler_app::features::account::create_account(&state, CLAUDE_CODE_ID, "Work Max", None)
            .unwrap();
    let recipe =
        juggler_app::features::profile::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let _ = juggler_app::features::launch::launch_profile(&state, &recipe.id, Some(cwd.clone()))
        .unwrap();
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
