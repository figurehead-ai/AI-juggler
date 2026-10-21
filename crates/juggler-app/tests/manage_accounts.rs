//! AJ-E1501: an account can be renamed, removed and made the default
//! without leaving AI Juggler.
//!
//! - Rename changes the label everywhere it is read (the accounts list,
//!   launch settings, sessions, a project's imported history); a label
//!   another account has is refused in words.
//! - Remove forgets an account used where it is without touching its
//!   folder; a copy's folder stays, or goes to the Trash when the user
//!   chooses that (never deleted). Open sessions refuse it, by name. Its
//!   history stays, marked "removed account".
//! - The global and per-project defaults; every change is an audit row.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_app::commands;
use juggler_app::error::AppError;
use juggler_app::manage::{self, CopyFolder};
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, ProjectHistory, SessionId, SessionStatus, Timestamp,
};

/// Each action's budget (the task's: < 200 ms, no rescan).
const BUDGET: Duration = Duration::from_millis(200);

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

/// An account with one launch setting. `in_place` accounts point at
/// `folder` (the user's own); copies live in AI Juggler's data.
fn seed(state: &Arc<AppState>, id: &str, label: &str, folder: Option<&Path>) -> PathBuf {
    let state_dir = folder.map_or_else(
        || state.accounts_dir().join("claude-code").join(id),
        Path::to_path_buf,
    );
    std::fs::create_dir_all(state_dir.join("projects")).unwrap();
    std::fs::write(state_dir.join("settings.json"), "{}").unwrap();
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new(id),
            harness: HarnessId::new("claude-code"),
            label: label.into(),
            plan: None,
            state_dir: state_dir.clone(),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: folder.map(|f| f.to_string_lossy().into_owned()),
            import_origin: None,
            imported_at: None,
            in_place: folder.is_some(),
        })
        .unwrap();
    storage
        .profiles()
        .insert(&LaunchRecipe {
            id: ProfileId::new(format!("rec-{id}")),
            name: "Main".into(),
            account: AccountId::new(id),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
    state_dir
}

fn session(state: &Arc<AppState>, account: &str, id: &str, cwd: &Path, status: SessionStatus) {
    std::fs::create_dir_all(cwd).unwrap();
    let now = Timestamp::now().millis();
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: SessionId::new(id),
            conversation: None,
            profile: ProfileId::new(format!("rec-{account}")),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: cwd.to_path_buf(),
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
}

/// Every path under `root` with its size and modification time.
fn snapshot(root: &Path) -> Vec<(String, u64, std::time::SystemTime)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, u64, std::time::SystemTime)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let meta = std::fs::symlink_metadata(entry.path()).unwrap();
            let rel = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            out.push((rel, meta.len(), meta.modified().unwrap()));
            if meta.is_dir() {
                walk(&entry.path(), root, out);
            }
        }
    }
    let meta = std::fs::metadata(root).unwrap();
    let mut out = vec![(".".to_owned(), meta.len(), meta.modified().unwrap())];
    walk(root, root, &mut out);
    out.sort();
    out
}

/// A stand-in Trash: moves the folder into `bin` and records it.
fn fake_trash(
    bin: PathBuf,
    log: Arc<Mutex<Vec<PathBuf>>>,
) -> impl Fn(&Path) -> std::io::Result<()> {
    move |path: &Path| {
        std::fs::create_dir_all(&bin)?;
        std::fs::rename(path, bin.join(path.file_name().unwrap()))?;
        log.lock().unwrap().push(path.to_path_buf());
        Ok(())
    }
}

fn never_trash(path: &Path) -> std::io::Result<()> {
    panic!("the Trash was asked to take {}", path.display());
}

fn labels(state: &Arc<AppState>) -> Vec<String> {
    commands::accounts_info(state)
        .unwrap()
        .into_iter()
        .map(|info| info.label)
        .collect()
}

fn refusal(error: AppError) -> String {
    match error {
        AppError::Refused(words) => words,
        other => panic!("expected a refusal in words, got {other:?}"),
    }
}

fn audit_actions(state: &Arc<AppState>, id: &str) -> Vec<(String, String)> {
    manage::account_audit_impl(state, id)
        .unwrap()
        .into_iter()
        .map(|row| {
            assert_eq!(row.actor, "user");
            assert!(row.at > 0);
            (row.action, row.detail.unwrap_or_default())
        })
        .collect()
}

#[test]
fn rename_changes_the_label_everywhere_and_refuses_a_duplicate() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    seed(&state, "acc-max", "personal-max", None);
    seed(&state, "acc-work", "Work", None);
    session(
        &state,
        "acc-max",
        "sess-1",
        &work.path().join("app"),
        SessionStatus::Finished,
    );
    // A project whose imported history names the account by label.
    {
        let storage = state.storage.lock().unwrap();
        let project = storage
            .projects()
            .touch(&work.path().join("app"), None)
            .unwrap();
        storage
            .projects()
            .merge_history(
                &project.id,
                &ProjectHistory {
                    accounts: vec!["personal-max".into(), "Work".into()],
                    native_sessions: 3,
                    last_used_ms: None,
                },
            )
            .unwrap();
    }

    let started = Instant::now();
    let renamed =
        manage::account_rename_impl(&state, "acc-max", "  Max (personal) ", Some("max-20x"))
            .unwrap();
    assert!(started.elapsed() < BUDGET, "{:?}", started.elapsed());
    assert_eq!(renamed.label, "Max (personal)");
    assert_eq!(renamed.plan.as_deref(), Some("max-20x"));

    // Everywhere it is read, with no restart.
    assert_eq!(labels(&state), ["Max (personal)", "Work"]);
    let storage = state.storage.lock().unwrap();
    let sessions = commands::session_infos_snapshot(&storage).unwrap();
    assert_eq!(sessions[0].account_label, "Max (personal)");
    let project = storage.projects().list().unwrap().remove(0);
    assert_eq!(project.history.accounts, ["Max (personal)", "Work"]);
    drop(storage);

    // Another account's name, in any case, is refused in words.
    let duplicate =
        manage::account_rename_impl(&state, "acc-work", "max (PERSONAL)", None).unwrap_err();
    let words = refusal(duplicate);
    assert!(words.contains("already called «Max (personal)»"), "{words}");
    assert!(
        refusal(manage::account_rename_impl(&state, "acc-work", "   ", None).unwrap_err())
            .contains("name")
    );
    assert_eq!(
        labels(&state),
        ["Max (personal)", "Work"],
        "nothing changed"
    );

    // Who, what, when.
    assert_eq!(
        audit_actions(&state, "acc-max"),
        [
            (
                "rename".to_owned(),
                "«personal-max» → «Max (personal)»".to_owned()
            ),
            ("plan".to_owned(), "none → max-20x".to_owned()),
        ]
    );
    assert!(audit_actions(&state, "acc-work").is_empty());
}

#[test]
fn removing_an_account_used_where_it_is_never_touches_its_folder() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let folder = work.path().join("home/.claude");
    seed(&state, "acc-own", "Mine", Some(&folder));
    std::fs::write(folder.join(".credentials.json"), "{}").unwrap();
    std::fs::write(folder.join("projects/s.jsonl"), "{}\n").unwrap();
    session(
        &state,
        "acc-own",
        "sess-old",
        &work.path().join("app"),
        SessionStatus::Finished,
    );
    let before = snapshot(&folder);

    // Asking for the Trash is refused: the folder is the user's.
    let words = refusal(
        manage::account_remove_impl(&state, "acc-own", CopyFolder::Trash, &never_trash)
            .unwrap_err(),
    );
    assert!(words.contains("never moves it"), "{words}");

    let started = Instant::now();
    let removed =
        manage::account_remove_impl(&state, "acc-own", CopyFolder::Keep, &never_trash).unwrap();
    assert!(started.elapsed() < BUDGET, "{:?}", started.elapsed());
    assert_eq!(removed.folder, "untouched");
    assert_eq!(snapshot(&folder), before, "the folder is exactly as it was");

    // Gone from every list; its history stays, marked.
    assert!(labels(&state).is_empty());
    let storage = state.storage.lock().unwrap();
    let rows = commands::session_infos_snapshot(&storage).unwrap();
    assert_eq!(rows.len(), 1, "the session's history stays");
    assert_eq!(rows[0].account_label, "Mine (removed account)");
    assert_eq!(rows[0].account_id.as_deref(), Some("acc-own"));
    drop(storage);

    // It starts nothing any more, and can't be removed twice.
    let launch = commands::launch_profile(&state, "rec-acc-own", None).unwrap_err();
    assert!(refusal(launch).contains("was removed"));
    assert!(
        manage::account_remove_impl(&state, "acc-own", CopyFolder::Keep, &never_trash).is_err()
    );
    assert_eq!(
        audit_actions(&state, "acc-own"),
        [(
            "remove".to_owned(),
            "used where it is: folder untouched".to_owned()
        )]
    );
}

#[test]
fn removing_a_copy_keeps_its_folder_or_moves_it_to_the_trash() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let kept = seed(&state, "acc-keep", "Old", None);
    let trashed = seed(&state, "acc-bin", "Older", None);
    let log = Arc::new(Mutex::new(Vec::new()));
    let bin = work.path().join("Trash");

    // Keep: the copy's folder stays where it is.
    let before = snapshot(&kept);
    let result =
        manage::account_remove_impl(&state, "acc-keep", CopyFolder::Keep, &never_trash).unwrap();
    assert_eq!(result.folder, "kept");
    assert_eq!(snapshot(&kept), before);

    // A Trash that refuses: nothing is removed.
    let failing =
        |_: &Path| -> std::io::Result<()> { Err(std::io::Error::other("the Trash is full")) };
    assert!(manage::account_remove_impl(&state, "acc-bin", CopyFolder::Trash, &failing).is_err());
    assert_eq!(labels(&state), ["Older"]);
    assert!(trashed.exists());

    // Trash: moved, never deleted.
    let result = manage::account_remove_impl(
        &state,
        "acc-bin",
        CopyFolder::Trash,
        &fake_trash(bin.clone(), log.clone()),
    )
    .unwrap();
    assert_eq!(result.folder, "trashed");
    assert!(!trashed.exists());
    assert!(
        bin.join("acc-bin/settings.json").exists(),
        "it is in the Trash"
    );
    assert_eq!(log.lock().unwrap().len(), 1);
    assert!(labels(&state).is_empty());
    let audit = audit_actions(&state, "acc-bin");
    assert_eq!(audit.len(), 1);
    assert!(
        audit[0].1.starts_with("copy's folder moved to the Trash"),
        "{audit:?}"
    );
}

#[test]
fn a_copy_outside_ai_jugglers_data_is_never_sent_to_the_trash() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    // An old copy row whose folder lies elsewhere (made by hand, or by an
    // earlier build): Remove can forget it, but never moves it.
    let elsewhere = work.path().join("somewhere/else");
    seed(&state, "acc-odd", "Odd", Some(&elsewhere));
    state
        .storage
        .lock()
        .unwrap()
        .raw_conn()
        .execute("UPDATE accounts SET in_place = 0 WHERE id = 'acc-odd'", [])
        .unwrap();
    let words = refusal(
        manage::account_remove_impl(&state, "acc-odd", CopyFolder::Trash, &never_trash)
            .unwrap_err(),
    );
    assert!(words.contains("stays where it is"), "{words}");
    assert!(elsewhere.exists());
    assert_eq!(labels(&state), ["Odd"]);
}

#[test]
fn an_account_with_open_sessions_is_not_removed_and_they_are_named() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    seed(&state, "acc-busy", "Busy", None);
    session(
        &state,
        "acc-busy",
        "s1",
        &work.path().join("api"),
        SessionStatus::Running,
    );
    session(
        &state,
        "acc-busy",
        "s2",
        &work.path().join("web"),
        SessionStatus::QuotaBlocked,
    );
    session(
        &state,
        "acc-busy",
        "s3",
        &work.path().join("done"),
        SessionStatus::Finished,
    );

    let words = refusal(
        manage::account_remove_impl(&state, "acc-busy", CopyFolder::Trash, &never_trash)
            .unwrap_err(),
    );
    assert_eq!(
        words,
        "«Busy» has 2 sessions open: api, web. Stop them first, then remove the account"
    );
    assert_eq!(labels(&state), ["Busy"]);
    assert!(audit_actions(&state, "acc-busy").is_empty());
}

#[test]
fn defaults_preselect_and_follow_a_removal() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    seed(&state, "acc-a", "A", None);
    seed(&state, "acc-b", "B", None);
    let project = state
        .storage
        .lock()
        .unwrap()
        .projects()
        .touch(&work.path().join("repo"), None)
        .unwrap();
    let defaults = |state: &Arc<AppState>| -> Vec<(String, bool)> {
        commands::accounts_info(state)
            .unwrap()
            .into_iter()
            .map(|info| (info.label, info.is_default == Some(true)))
            .collect()
    };
    // None set: nothing is marked (the most recently used one leads, as
    // before).
    assert_eq!(defaults(&state), [("A".into(), false), ("B".into(), false)]);

    let started = Instant::now();
    let rows = manage::account_set_default_impl(&state, Some("acc-b")).unwrap();
    assert!(started.elapsed() < BUDGET);
    assert_eq!(
        rows.iter().filter(|r| r.is_default == Some(true)).count(),
        1
    );
    assert_eq!(defaults(&state), [("A".into(), false), ("B".into(), true)]);

    manage::project_set_default_account_impl(&state, project.id.as_str(), Some("acc-a")).unwrap();
    let default_of = |state: &Arc<AppState>| {
        commands::projects_list_impl(state)
            .unwrap()
            .remove(0)
            .default_account_id
    };
    assert_eq!(default_of(&state).as_deref(), Some("acc-a"));

    // Removing an account clears it as a default, everywhere.
    manage::account_remove_impl(&state, "acc-a", CopyFolder::Keep, &never_trash).unwrap();
    assert_eq!(default_of(&state), None);
    manage::account_remove_impl(&state, "acc-b", CopyFolder::Keep, &never_trash).unwrap();
    assert!(defaults(&state).is_empty());
    let stored = state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .get(manage::DEFAULT_ACCOUNT_KEY)
        .unwrap();
    assert_eq!(stored, None);

    // A removed account can't become a default.
    assert!(
        refusal(manage::account_set_default_impl(&state, Some("acc-a")).unwrap_err())
            .contains("was removed")
    );
    assert_eq!(
        audit_actions(&state, "acc-a")
            .into_iter()
            .map(|(action, _)| action)
            .collect::<Vec<_>>(),
        ["project_default", "remove"]
    );
    assert_eq!(
        audit_actions(&state, "acc-b")
            .into_iter()
            .map(|(action, _)| action)
            .collect::<Vec<_>>(),
        ["default", "remove"]
    );
}

#[test]
fn a_backup_still_withholds_a_removed_accounts_environment_values() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    seed(&state, "acc-gone", "Gone", None);
    {
        let storage = state.storage.lock().unwrap();
        let mut recipe = storage
            .profiles()
            .get(&ProfileId::new("rec-acc-gone"))
            .unwrap();
        recipe
            .settings_overlay
            .env
            .insert("API_TOKEN".into(), "s3cr3t-value".into());
        storage.profiles().update(&recipe).unwrap();
    }
    manage::account_remove_impl(&state, "acc-gone", CopyFolder::Keep, &never_trash).unwrap();
    let archive = work.path().join("backup.ajbackup");
    let exported =
        juggler_app::backup::backup_export_impl(&state, &archive, &mut |_, _| {}).unwrap();
    assert!(
        exported
            .manifest
            .withheld
            .iter()
            .any(|line| line.contains("API_TOKEN")),
        "{:?}",
        exported.manifest.withheld
    );
}

/// The real Trash (NSFileManager trashItem): a folder goes there and is
/// not deleted. Ignored by default (it puts a small folder in the
/// user's Trash); run it by hand on a Mac.
#[cfg(target_os = "macos")]
#[test]
#[ignore = "moves a scratch folder into this Mac's Trash"]
fn move_to_trash_uses_the_macos_trash() {
    let work = tempfile::tempdir().unwrap();
    let name = format!("aij-e1501-trash-check-{}", Timestamp::now().millis());
    let folder = work.path().join(&name);
    std::fs::create_dir_all(folder.join("projects")).unwrap();
    std::fs::write(folder.join("settings.json"), "{}").unwrap();
    manage::move_to_trash(&folder).unwrap();
    assert!(!folder.exists(), "moved away");
    // Listing the Trash needs Full Disk Access; its entry by name doesn't.
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap();
    let trashed = home.join(".Trash").join(&name);
    assert!(
        trashed.join("settings.json").exists(),
        "{} is in the Trash, not deleted",
        trashed.display()
    );
}
