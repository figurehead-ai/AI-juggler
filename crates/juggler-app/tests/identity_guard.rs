//! AJ-E1001 integration: the wrong-account billing guard with a stub
//! `claude`. Proves: a matched Account launches and its Instance shows
//! the verified identity; a deliberately mismatched config dir BLOCKS the
//! launch (nothing spawns) with expected vs detected and the Launch
//! settings that are signed in as the detected identity; a signed-out
//! dir launches honestly labelled; switches re-assert the target; the
//! history records every assertion; the probe stays inside its budget.

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
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::state::AppState;
use juggler_app::{commands, identity};
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

fn claude_stub(dir: &Path) -> PathBuf {
    let path = dir.join("claude-stub");
    stand_in::write(
        &path,
        // Written whole, then moved into place: a reader never sees it
        // half-written (the same fix as parked_reset's stub).
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CLAUDE_CONFIG_DIR/last-args.tmp\" && mv \"$CLAUDE_CONFIG_DIR/last-args.tmp\" \"$CLAUDE_CONFIG_DIR/last-args\"\n",
    );
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

fn config_dir(state: &Arc<AppState>, account: &str) -> PathBuf {
    state.accounts_dir().join(CLAUDE_CODE_ID).join(account)
}

fn sign_in(state: &Arc<AppState>, account: &str, uuid: &str, email: &str) {
    let dir = config_dir(state, account);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(".claude.json"),
        format!(
            r#"{{"numStartups":2,"oauthAccount":{{"accountUuid":"{uuid}","emailAddress":"{email}","organizationName":"Acme"}}}}"#
        ),
    )
    .unwrap();
}

fn row(state: &Arc<AppState>, session: &str) -> commands::InstanceInfo {
    let storage = state.storage.lock().unwrap();
    commands::session_infos_snapshot(&storage)
        .unwrap()
        .into_iter()
        .find(|row| row.id == session)
        .unwrap()
}

#[test]
fn a_matched_account_launches_and_a_mismatched_one_is_blocked_with_nothing_run() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let cwd = work.path().to_string_lossy().into_owned();
    let work_max = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let personal = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let work_recipe = commands::create_profile(&state, &work_max.id, "Main", None, None).unwrap();
    let personal_recipe =
        commands::create_profile(&state, &personal.id, "Main", None, None).unwrap();
    sign_in(&state, &work_max.id, "acct-work", "olivia@work.example");
    sign_in(&state, &personal.id, "acct-home", "olivia@home.example");

    // First sight: remembered, shown as first seen, before any prompt.
    let check = identity::identity_check_impl(&state, &work_recipe.id).unwrap();
    assert_eq!(check.state, "first_seen");
    assert_eq!(check.detected.as_deref(), Some("o…@work.example · Acme"));
    // Each of the three launches below runs the identity probe; the budget
    // holds for the best of them (wall time: the launch spans processes, so
    // a busy machine's turns land in any one reading).
    let mut launches = Vec::new();
    let started = Instant::now();
    let first = commands::launch_profile(&state, &work_recipe.id, Some(cwd.clone())).unwrap();
    launches.push(started.elapsed());
    assert_eq!(
        row(&state, &first.session_id).identity_state.as_deref(),
        Some("first_seen")
    );
    let started = Instant::now();
    let _ = commands::launch_profile(&state, &personal_recipe.id, Some(cwd.clone())).unwrap();
    launches.push(started.elapsed());

    // Matched afterwards: verified, the label on the Instance row.
    let started = Instant::now();
    let second = commands::launch_profile(&state, &work_recipe.id, Some(cwd.clone())).unwrap();
    launches.push(started.elapsed());
    let verified = row(&state, &second.session_id);
    assert_eq!(verified.identity_state.as_deref(), Some("verified"));
    assert_eq!(
        verified.identity_label.as_deref(),
        Some("o…@work.example · Acme")
    );

    // Someone signs Work Max's config dir into the HOME account.
    sign_in(&state, &work_max.id, "acct-home", "olivia@home.example");
    // The earlier launches' stubs record their argv asynchronously —
    // wait for that write, then clear it.
    let args = config_dir(&state, &work_max.id).join("last-args");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !args.is_file() {
        assert!(Instant::now() < deadline, "setup stub never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(200));
    let _ = std::fs::remove_file(&args);
    let check = identity::identity_check_impl(&state, &work_recipe.id).unwrap();
    assert_eq!(check.state, "mismatch");
    assert_eq!(check.expected.as_deref(), Some("o…@work.example · Acme"));
    assert_eq!(check.detected.as_deref(), Some("o…@home.example · Acme"));
    assert_eq!(
        check
            .alternatives
            .iter()
            .map(|a| a.account_label.as_str())
            .collect::<Vec<_>>(),
        ["Personal"],
        "switch selection: the Account that IS that identity"
    );
    let before = {
        let storage = state.storage.lock().unwrap();
        storage.sessions().list(None).unwrap().len()
    };
    let error = commands::launch_profile(&state, &work_recipe.id, Some(cwd.clone())).unwrap_err();
    assert_eq!(error.wire().code, "identity_mismatch");
    assert!(
        error.wire().user_message.contains("nothing was launched"),
        "{}",
        error.wire().user_message
    );
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !config_dir(&state, &work_max.id).join("last-args").exists(),
        "the harness never ran"
    );
    let after = {
        let storage = state.storage.lock().unwrap();
        storage.sessions().list(None).unwrap().len()
    };
    assert_eq!(before, after, "no Instance was created");

    // The history records who ran where, when.
    let storage = state.storage.lock().unwrap();
    let history = storage.identity().history(&second.session_id).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].moment, "launch");
    drop(storage);
    eprintln!("[perf] launch incl. identity probe: {launches:?}");
    assert!(
        launches.iter().any(|took| *took < Duration::from_secs(1)),
        "every launch over 1 s: {launches:?}"
    );
}

#[test]
fn a_signed_out_dir_launches_honestly_and_forgetting_relearns() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let cwd = work.path().to_string_lossy().into_owned();
    let account = commands::create_account(&state, CLAUDE_CODE_ID, "Fresh", None).unwrap();
    let recipe = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
    let launched = commands::launch_profile(&state, &recipe.id, Some(cwd.clone())).unwrap();
    assert_eq!(
        row(&state, &launched.session_id).identity_state.as_deref(),
        Some("signed_out")
    );

    sign_in(&state, &account.id, "acct-a", "a@example.com");
    let _ = commands::launch_profile(&state, &recipe.id, Some(cwd.clone())).unwrap();
    sign_in(&state, &account.id, "acct-b", "b@example.com");
    assert!(commands::launch_profile(&state, &recipe.id, Some(cwd.clone())).is_err());
    // The user says this Account is now b@: forget, then it is learned.
    identity::account_forget_identity_impl(&state, &account.id).unwrap();
    let relearned = commands::launch_profile(&state, &recipe.id, Some(cwd)).unwrap();
    assert_eq!(
        row(&state, &relearned.session_id).identity_state.as_deref(),
        Some("first_seen")
    );
}

#[test]
fn a_switch_to_a_mismatched_target_is_refused_before_anything_moves() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project = work.path().join("storefront");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let b = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let recipe_a = commands::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let recipe_b = commands::create_profile(&state, &b.id, "Main", None, None).unwrap();
    sign_in(&state, &b.id, "acct-b", "b@example.com");
    let _ = commands::launch_profile(&state, &recipe_b.id, Some(cwd.clone())).unwrap();
    // B's dir is now signed in as someone else.
    sign_in(&state, &b.id, "acct-x", "x@example.com");
    let source = commands::launch_profile(&state, &recipe_a.id, Some(cwd.clone())).unwrap();
    let transcript = config_dir(&state, &a.id)
        .join("projects")
        .join(cwd.replace('/', "-"))
        .join("native-1.jsonl");
    std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    std::fs::write(
        &transcript,
        format!(
            "{{\"type\":\"user\",\"uuid\":\"u1\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"user\",\"content\":\"hi\"}}}}\n"
        ),
    )
    .unwrap();
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                source.session_id.clone(),
            ))
            .unwrap();
        record.transcript_ref = Some(juggler_core::model::TranscriptRef {
            path: transcript,
            native_id: Some("native-1".into()),
        });
        storage.sessions().update(&record).unwrap();
    }
    let error = commands::switch_execute_impl(
        &state,
        &source.session_id,
        &recipe_b.id,
        "finish-turn",
        None,
        &[],
    )
    .unwrap_err();
    assert_eq!(error.wire().code, "identity_mismatch");
    // Nothing moved: no transcript landed in B.
    assert!(
        !config_dir(&state, &b.id)
            .join("projects")
            .join(cwd.replace('/', "-"))
            .join("native-1.jsonl")
            .exists()
    );
}
