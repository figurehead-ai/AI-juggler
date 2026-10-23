//! AJ-E1502: sessions and projects can be named, pinned, opened and
//! tidied.
//!
//! - A session's label shows over its own title; pins lead the rail.
//! - A project's rename is a label (the folder keeps its name); pin;
//!   open in the chosen editor, else Finder; show in Finder.
//! - Forget hides a project until it is used again (a session started
//!   there, or newer work there outside AI Juggler); refused while a
//!   session there is open.
//! - Relocate repoints a moved folder; a missing or taken folder is
//!   refused in words.
//! - Labels and pins survive a restart and a backup.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::ClaudeCodeAdapter;
use juggler_app::commands;
use juggler_app::error::AppError;
use juggler_app::manage::{self, OpenIn};
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, ProjectHistory, ProjectId, SessionId, SessionStatus, Timestamp,
};

/// Compares without echoing either side: the session and project rows
/// carry data derived from launch settings (CodeQL's cleartext-logging
/// rule), so a failure names the check, not the values.
macro_rules! check_eq {
    ($left:expr, $right:expr $(,)?) => {
        assert!(
            $left == $right,
            "{}",
            concat!(stringify!($left), " == ", stringify!($right))
        )
    };
    ($left:expr, $right:expr, $($message:tt)+) => {
        assert!($left == $right, $($message)+)
    };
}

/// Each action's budget (the task's: < 100 ms).
const BUDGET: Duration = Duration::from_millis(100);

/// A `claude` stand-in that exits at once (launches in these tests only
/// need to reach the project).
fn claude_stub(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("claude-stub");
    std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn app_state(data: &Path, stub: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        ClaudeCodeAdapter::with_bin(stub).with_overlays_root(data.join("overlays")),
    ));
    Arc::new(AppState::init_with_registry(data.to_path_buf(), registry).unwrap())
}

/// A signed-in account with one launch setting.
fn seed_account(state: &Arc<AppState>) {
    let dir = state.accounts_dir().join("claude-code/acc-a");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(".credentials.json"), "{}").unwrap();
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-a"),
            harness: HarnessId::new("claude-code"),
            label: "Work".into(),
            plan: None,
            state_dir: dir,
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        })
        .unwrap();
    storage
        .profiles()
        .insert(&LaunchRecipe {
            id: ProfileId::new("rec-a"),
            name: "Main".into(),
            account: AccountId::new("acc-a"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
}

/// A project at `root` (created on disk), with a session in `status`.
fn project_with_session(
    state: &Arc<AppState>,
    root: &Path,
    session: &str,
    status: SessionStatus,
) -> ProjectId {
    std::fs::create_dir_all(root).unwrap();
    let root = root.canonicalize().unwrap();
    let storage = state.storage.lock().unwrap();
    let project = storage.projects().touch(&root, None).unwrap();
    let now = Timestamp::now().millis();
    storage
        .sessions()
        .insert(&InstanceRecord {
            id: SessionId::new(session),
            conversation: None,
            profile: ProfileId::new("rec-a"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: Some(project.id.clone()),
            cwd: root,
            status,
            started_at: Timestamp::from_millis(now - 60_000),
            last_activity: Some(Timestamp::from_millis(now)),
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
    project.id
}

fn row(state: &Arc<AppState>, id: &str) -> commands::InstanceInfo {
    let storage = state.storage.lock().unwrap();
    commands::session_infos_snapshot(&storage)
        .unwrap()
        .into_iter()
        .find(|row| row.id == id)
        .unwrap()
}

fn project(state: &Arc<AppState>, id: &ProjectId) -> commands::ProjectInfo {
    commands::projects_list_impl(state)
        .unwrap()
        .into_iter()
        .find(|row| row.id == id.as_str())
        .unwrap()
}

fn refusal(error: AppError) -> String {
    match error {
        AppError::Refused(words) => words,
        other => panic!("expected a refusal in words, got {other:?}"),
    }
}

/// Runs `action` and checks its own work stays inside [`BUDGET`]: this
/// thread's CPU time, which other programs' turns don't add to (wall time,
/// printed beside it, read 419 ms on a loaded machine).
fn timed<T>(action: impl FnOnce() -> T) -> T {
    let wall = Instant::now();
    let cpu = cpu_time::ThreadTime::now();
    let out = action();
    let (cpu, wall) = (cpu.elapsed(), wall.elapsed());
    eprintln!("[budget] {cpu:?} CPU, {wall:?} wall");
    assert!(cpu < BUDGET, "{cpu:?} CPU (wall {wall:?})");
    out
}

#[test]
fn a_session_label_shows_over_its_title_and_a_pin_marks_it() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(&work.path().join("data"), &claude_stub(work.path()));
    seed_account(&state);
    project_with_session(
        &state,
        &work.path().join("api"),
        "s1",
        SessionStatus::Finished,
    );
    check_eq!(row(&state, "s1").label, None);
    check_eq!(row(&state, "s1").pinned, None);

    timed(|| manage::session_rename_impl(&state, "s1", Some("  fix the login bug "))).unwrap();
    timed(|| manage::session_set_pinned_impl(&state, "s1", true)).unwrap();
    let marked = row(&state, "s1");
    check_eq!(marked.label.as_deref(), Some("fix the login bug"));
    check_eq!(marked.pinned, Some(true));

    // Blank goes back to the session's own title; unpin.
    manage::session_rename_impl(&state, "s1", Some("   ")).unwrap();
    manage::session_set_pinned_impl(&state, "s1", false).unwrap();
    check_eq!(row(&state, "s1").label, None);
    check_eq!(row(&state, "s1").pinned, None);

    let long = "x".repeat(81);
    assert!(
        refusal(manage::session_rename_impl(&state, "s1", Some(&long)).unwrap_err()).contains("80")
    );
    assert!(matches!(
        manage::session_rename_impl(&state, "nope", Some("x")),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn a_project_rename_is_a_label_and_the_folder_keeps_its_name() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(&work.path().join("data"), &claude_stub(work.path()));
    seed_account(&state);
    let folder = work.path().join("dev-api");
    let id = project_with_session(&state, &folder, "s1", SessionStatus::Finished);

    timed(|| manage::project_rename_impl(&state, id.as_str(), "Billing API")).unwrap();
    let renamed = project(&state, &id);
    check_eq!(renamed.name, "Billing API");
    assert!(
        renamed.root.ends_with("/dev-api"),
        "the root is still the dev-api folder"
    );
    assert!(folder.is_dir(), "the folder is where it was, as it was");
    assert!(!work.path().join("Billing API").exists());

    // Blank: the folder's name again.
    manage::project_rename_impl(&state, id.as_str(), " ").unwrap();
    check_eq!(project(&state, &id).name, "dev-api");

    timed(|| manage::project_set_pinned_impl(&state, id.as_str(), true)).unwrap();
    check_eq!(project(&state, &id).pinned, Some(true));
    manage::project_set_pinned_impl(&state, id.as_str(), false).unwrap();
    check_eq!(project(&state, &id).pinned, None);
}

#[test]
fn a_forgotten_project_is_hidden_until_it_is_used_again() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(&work.path().join("data"), &claude_stub(work.path()));
    seed_account(&state);
    let busy = project_with_session(
        &state,
        &work.path().join("busy"),
        "s-run",
        SessionStatus::Running,
    );
    let old = project_with_session(
        &state,
        &work.path().join("old"),
        "s-old",
        SessionStatus::Finished,
    );

    // An open session keeps it, named.
    let words = refusal(manage::project_forget_impl(&state, busy.as_str(), true).unwrap_err());
    check_eq!(
        words,
        "busy has a session open: busy. Stop it first, then forget the project"
    );
    check_eq!(project(&state, &busy).forgotten, None);

    // Forget: hidden, its pin dropped, its session still in History.
    manage::project_set_pinned_impl(&state, old.as_str(), true).unwrap();
    timed(|| manage::project_forget_impl(&state, old.as_str(), true)).unwrap();
    let forgotten = project(&state, &old);
    check_eq!(forgotten.forgotten, Some(true));
    check_eq!(forgotten.pinned, None);
    check_eq!(row(&state, "s-old").status, "finished");

    // Used again: a session started there brings it back.
    let root = PathBuf::from(&forgotten.root);
    let launched =
        commands::launch_profile(&state, "rec-a", Some(root.to_string_lossy().into_owned()))
            .unwrap();
    check_eq!(
        project(&state, &old).forgotten,
        None,
        "a launch brings it back"
    );

    // That session ends (the stand-in exits at once).
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&SessionId::new(launched.session_id.as_str()))
            .unwrap();
        record.status = SessionStatus::Finished;
        storage.sessions().update(&record).unwrap();
    }

    // Used again elsewhere: newer work in its history brings it back too.
    manage::project_forget_impl(&state, old.as_str(), true).unwrap();
    check_eq!(project(&state, &old).forgotten, Some(true));
    std::thread::sleep(Duration::from_millis(5));
    state
        .storage
        .lock()
        .unwrap()
        .projects()
        .merge_history(
            &old,
            &ProjectHistory {
                accounts: vec!["Work".into()],
                native_sessions: 1,
                last_used_ms: Some(Timestamp::now().millis()),
            },
        )
        .unwrap();
    check_eq!(project(&state, &old).forgotten, None, "worked in since");

    // And it can be brought back by hand (the undo).
    manage::project_forget_impl(&state, old.as_str(), true).unwrap();
    manage::project_forget_impl(&state, old.as_str(), false).unwrap();
    check_eq!(project(&state, &old).forgotten, None);
}

#[test]
fn relocate_repoints_a_moved_folder_and_refuses_a_missing_or_taken_one() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(&work.path().join("data"), &claude_stub(work.path()));
    seed_account(&state);
    let first = project_with_session(
        &state,
        &work.path().join("app"),
        "s1",
        SessionStatus::Finished,
    );
    let other = project_with_session(
        &state,
        &work.path().join("other"),
        "s2",
        SessionStatus::Finished,
    );

    // The folder moved: the project is missing.
    let moved = work.path().join("moved/app");
    std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
    std::fs::rename(work.path().join("app"), &moved).unwrap();
    assert!(project(&state, &first).missing);

    // Opening it is refused in words (there's nothing to open).
    let words = refusal(
        manage::project_open_target_impl(&state, first.as_str(), OpenIn::Editor).unwrap_err(),
    );
    assert!(words.contains("Relocate"));

    // A folder that isn't there, or one another project has, is refused.
    let nowhere = work.path().join("nowhere");
    let words =
        refusal(manage::project_relocate_impl(&state, first.as_str(), &nowhere).unwrap_err());
    assert!(words.contains("isn't a folder"));
    let taken = PathBuf::from(project(&state, &other).root);
    let words = refusal(manage::project_relocate_impl(&state, first.as_str(), &taken).unwrap_err());
    check_eq!(words, "other already uses that folder");
    assert!(project(&state, &first).missing);

    timed(|| manage::project_relocate_impl(&state, first.as_str(), &moved)).unwrap();
    let found = project(&state, &first);
    assert!(!found.missing);
    check_eq!(PathBuf::from(&found.root), moved.canonicalize().unwrap());
    check_eq!(found.name, "app", "its name stays");
}

#[test]
fn a_project_opens_in_the_chosen_editor_else_finder() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(&work.path().join("data"), &claude_stub(work.path()));
    seed_account(&state);
    let id = project_with_session(
        &state,
        &work.path().join("app"),
        "s1",
        SessionStatus::Finished,
    );
    let root = PathBuf::from(project(&state, &id).root);

    let target =
        timed(|| manage::project_open_target_impl(&state, id.as_str(), OpenIn::Editor)).unwrap();
    check_eq!(
        target,
        manage::OpenTarget {
            folder: root.clone(),
            app: None,
            reveal: false
        },
        "no editor chosen: a Finder window"
    );

    // The choice: only editors this Mac has are offered, and only known
    // ones are accepted.
    let apps = work.path().join("Applications");
    std::fs::create_dir_all(apps.join("Zed.app")).unwrap();
    std::fs::create_dir_all(apps.join("Visual Studio Code.app")).unwrap();
    std::fs::create_dir_all(apps.join("Unknown Thing.app")).unwrap();
    let choices = manage::editor_choices_impl(&state, &[apps]).unwrap();
    check_eq!(choices.installed, ["Visual Studio Code", "Zed"]);
    check_eq!(choices.chosen, None);
    assert!(
        refusal(manage::editor_set_impl(&state, Some("Unknown Thing")).unwrap_err())
            .contains("doesn't know")
    );
    manage::editor_set_impl(&state, Some("Zed")).unwrap();

    let target = manage::project_open_target_impl(&state, id.as_str(), OpenIn::Editor).unwrap();
    check_eq!(target.app.as_deref(), Some("Zed"));
    assert!(!target.reveal);
    let target = manage::project_open_target_impl(&state, id.as_str(), OpenIn::Finder).unwrap();
    check_eq!(
        target,
        manage::OpenTarget {
            folder: root,
            app: None,
            reveal: true
        }
    );

    // Back to Finder.
    manage::editor_set_impl(&state, None).unwrap();
    check_eq!(
        manage::project_open_target_impl(&state, id.as_str(), OpenIn::Editor)
            .unwrap()
            .app,
        None
    );
}

#[test]
fn labels_and_pins_survive_a_restart_and_a_backup() {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let data = work.path().join("data");
    let archive = work.path().join("backup.ajbackup");
    let id = {
        let state = app_state(&data, &stub);
        seed_account(&state);
        let id = project_with_session(
            &state,
            &work.path().join("app"),
            "s1",
            SessionStatus::Finished,
        );
        manage::session_rename_impl(&state, "s1", Some("the login fix")).unwrap();
        manage::session_set_pinned_impl(&state, "s1", true).unwrap();
        manage::project_rename_impl(&state, id.as_str(), "Billing").unwrap();
        manage::project_set_pinned_impl(&state, id.as_str(), true).unwrap();
        juggler_app::backup::backup_export_impl(&state, &archive, &mut |_, _| {}).unwrap();
        id
    };

    // A restart: the same data, opened again.
    let state = app_state(&data, &stub);
    check_eq!(row(&state, "s1").label.as_deref(), Some("the login fix"));
    check_eq!(row(&state, "s1").pinned, Some(true));
    check_eq!(project(&state, &id).name, "Billing");
    check_eq!(project(&state, &id).pinned, Some(true));
    drop(state);

    // A backup restored on another Mac.
    let other_data = work.path().join("other-mac");
    let other = app_state(&other_data, &stub);
    let plan = juggler_app::backup::backup_verify_impl(&other, &archive).unwrap();
    juggler_app::backup::backup_restore_impl(&other, &archive, &plan.token).unwrap();
    drop(other);
    let restored = app_state(&other_data, &stub);
    check_eq!(row(&restored, "s1").label.as_deref(), Some("the login fix"));
    check_eq!(row(&restored, "s1").pinned, Some(true));
    check_eq!(project(&restored, &id).name, "Billing");
    check_eq!(project(&restored, &id).pinned, Some(true));
}
