//! AJ-E2302..E2307 integration: sessions started in their runtime through
//! the app's own launch path.
//!
//! - The fake tool, driven on its profile's choice: exact states stored, an
//!   approval and a question answered only with what the tool offered,
//!   every answer in the account's audit, the stop leaving nothing behind.
//! - Codex through `fake-codex`'s app-server by default: a socket in a
//!   `0700` folder made `0600`, live limits before the first turn, the
//!   server a ledger child that goes with its session.
//! - Fallbacks: "Start in terminal" per session and per profile, a Codex
//!   too old or a protocol AI Juggler hasn't pinned → the terminal, with
//!   the reason on the session.
//! - AJ-E2310: the app core killed (a child process, `SIGKILL`) between
//!   turns brings the session back driven on its thread; mid-turn it says
//!   its turn was cut off and waits for Resume; a stopped one stays ended.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

mod common;

#[path = "support/set_up.rs"]
mod set_up;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::{AdapterRegistry, Runtime};
use juggler_adapters::{CODEX_ID, CodexAdapter, FAKE_HARNESS_ID, FakeHarnessAdapter};
use juggler_app::commands;
use juggler_app::features::runtime::commands as runtime;
use juggler_app::features::runtime::dto::{AnswerInput, PartReplyInput};
use juggler_app::state::AppState;
use juggler_core::model::SessionId;

fn fake_codex_bin() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    for profile in ["debug", "release"] {
        let candidate = target.join(profile).join("fake-codex");
        if candidate.exists() {
            return candidate;
        }
    }
    unreachable!("fake-codex binary not found under {}", target.display());
}

/// `fake-codex` behind a script that sets `env`.
fn codex_with(dir: &Path, env: &str) -> PathBuf {
    let path = dir.join("codex-wrapper");
    std::fs::write(
        &path,
        format!("#!/bin/sh\n{env} exec {:?} \"$@\"\n", fake_codex_bin()),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn app_state(dir: &Path, codex: Option<PathBuf>) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(FakeHarnessAdapter::new(
        common::fake_harness::bin(),
    )));
    let codex = codex.unwrap_or_else(fake_codex_bin);
    registry.register(Arc::new(
        CodexAdapter::with_bin(codex).with_home(dir.join("home")),
    ));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn new_profile(state: &Arc<AppState>, harness: &str, cwd: &Path) -> String {
    let account = set_up::set_up(
        state,
        commands::create_account(state, harness, "Lab", None).unwrap(),
    );
    commands::create_profile(
        state,
        &account.id,
        "lab",
        None,
        Some(cwd.to_string_lossy().into_owned()),
    )
    .unwrap()
    .id
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn status_of(state: &Arc<AppState>, session: &str) -> String {
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(session))
        .unwrap()
        .status
        .as_str()
        .to_owned()
}

fn row(state: &Arc<AppState>, session: &str) -> juggler_app::features::sessions::dto::InstanceInfo {
    commands::instances_shared_snapshot(state)
        .unwrap()
        .into_iter()
        .find(|row| row.id == session)
        .unwrap()
}

fn choose(option: &str) -> AnswerInput {
    AnswerInput {
        option: Some(option.into()),
        replies: None,
    }
}

fn stop(state: &Arc<AppState>, session: &str) {
    state
        .supervisor
        .stop(&SessionId::new(session), Duration::from_secs(2))
        .unwrap();
    wait_for("the session to end", || {
        state.supervisor.is_finished(&SessionId::new(session))
    });
}

#[test]
fn a_driven_session_is_exact_and_answered_only_by_the_user() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path(), None);
    let profile = new_profile(&state, FAKE_HARNESS_ID, work.path());
    // The fake's own default is the terminal; the profile chooses driven.
    runtime::choose_impl(&state, &profile, Some("driven")).unwrap();
    let launched = commands::launch_profile(&state, &profile, None).unwrap();
    let session = launched.session_id;
    assert!(state.runtime.is_driven(&SessionId::new(session.as_str())));
    let info = row(&state, &session).runtime.expect("its runtime is shown");
    assert_eq!(info.kind, "driven");
    assert_eq!(info.label.as_deref(), Some("the fake tool's protocol"));

    // A turn with an approval: Asks you, exactly, with the tool's options.
    runtime::send_impl(&state, &session, "ask: rm -rf build").unwrap();
    wait_for("the ask", || !runtime::asks_impl(&state, 0, 10).is_empty());
    let asks = runtime::asks_impl(&state, 0, 10);
    assert_eq!(asks[0].summary, "rm -rf build");
    assert_eq!(asks[0].session_id, session);
    let labels: Vec<&str> = asks[0].options.iter().map(|o| o.label.as_str()).collect();
    assert_eq!(labels, ["Allow once", "Always allow this", "Deny"]);
    wait_for("Asks you", || status_of(&state, &session) == "waiting_user");
    let stored = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(session.as_str()))
        .unwrap();
    assert_eq!(stored.attention.as_ref().unwrap().rule, "tool_asks");
    assert_eq!(
        row(&state, &session).runtime.unwrap().state.as_deref(),
        Some("asks_you")
    );

    // Only an offered option is carried; then the turn carries on.
    assert!(runtime::answer_impl(&state, &session, &asks[0].id, choose("yes-to-all")).is_err());
    assert!(runtime::answer_impl(&state, &session, "no-such-ask", choose("once")).is_err());
    runtime::answer_impl(&state, &session, &asks[0].id, choose("once")).unwrap();
    wait_for("the ask to close", || {
        runtime::asks_impl(&state, 0, 10).is_empty()
    });
    wait_for("Running again", || status_of(&state, &session) == "running");
    wait_for("the reply", || {
        runtime::get_impl(&state, &session)
            .unwrap()
            .unwrap()
            .items
            .iter()
            .any(|item| item.text == "you chose once")
    });

    // A question, with a reply.
    runtime::send_impl(&state, &session, "question: which colour?").unwrap();
    wait_for("the question", || {
        !runtime::asks_impl(&state, 0, 10).is_empty()
    });
    let question = runtime::asks_impl(&state, 0, 10).remove(0);
    assert_eq!(question.kind, "question");
    assert!(
        runtime::answer_impl(&state, &session, &question.id, choose("once")).is_err(),
        "a question takes a reply"
    );
    let reply = AnswerInput {
        option: None,
        replies: Some(vec![PartReplyInput {
            part: question.parts[0].id.clone(),
            answers: vec!["Blue".into()],
        }]),
    };
    runtime::answer_impl(&state, &session, &question.id, reply).unwrap();
    wait_for("the reply", || {
        runtime::get_impl(&state, &session)
            .unwrap()
            .unwrap()
            .items
            .iter()
            .any(|item| item.text == "you said Blue")
    });

    // Every answer is in the account's audit: the option, never a reply's text.
    let audit = {
        let storage = state.storage.lock().unwrap();
        let record = storage
            .sessions()
            .get(&SessionId::new(session.as_str()))
            .unwrap();
        let profile = storage.profiles().get(&record.profile).unwrap();
        storage.account_audit().list_for(&profile.account).unwrap()
    };
    let answered: Vec<&str> = audit
        .iter()
        .filter(|row| row.action == "answered")
        .filter_map(|row| row.detail.as_deref())
        .collect();
    assert_eq!(answered.len(), 2, "{answered:?}");
    assert!(answered[0].contains("command") && answered[0].contains("Allow once"));
    assert!(answered[1].contains("replied") && !answered[1].contains("Blue"));
    assert!(audit.iter().all(|row| row.actor == "user"));

    // The stop leaves nothing behind.
    stop(&state, &session);
    wait_for("the driver to see the end", || {
        state
            .runtime
            .driver(&session)
            .is_none_or(|driver| driver.ended())
    });
    assert!(runtime::asks_impl(&state, 0, 10).is_empty());
}

#[test]
fn codex_runs_through_its_own_app_server_by_default() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path(), None);
    let profile = new_profile(&state, CODEX_ID, work.path());
    let launched = commands::launch_profile(&state, &profile, None).unwrap();
    let session = launched.session_id;
    let driver = state.runtime.driver(&session).expect("Codex is driven");
    // One app-server per session over stdio: no socket anywhere (the lab
    // of 2026-10-25), the server the session's own ledger child.
    assert!(driver.socket.is_none());
    assert!(
        !state.data_dir.join("runtime").exists()
            || std::fs::read_dir(state.data_dir.join("runtime"))
                .unwrap()
                .next()
                .is_none()
    );
    assert!(
        state
            .supervisor
            .is_supervised(&SessionId::new(session.as_str()))
    );

    // Linked to its thread's rollout, in the account's own folder.
    wait_for("the link", || {
        state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .get(&SessionId::new(session.as_str()))
            .unwrap()
            .transcript_ref
            .is_some_and(|t| t.path.is_file() && t.native_id.is_some())
    });

    // Limits with resets, before any turn.
    wait_for("the limits", || {
        !row(&state, &session).runtime.unwrap().limits.is_empty()
    });
    let limits = row(&state, &session).runtime.unwrap().limits;
    assert_eq!(limits[0].window_mins, Some(300));
    assert!(limits[0].resets_at_ms.is_some());

    // An approval with Codex's own rule.
    runtime::send_impl(&state, &session, "ask: npm test").unwrap();
    wait_for("the ask", || !runtime::asks_impl(&state, 0, 10).is_empty());
    let ask = runtime::asks_impl(&state, 0, 10).remove(0);
    assert!(
        ask.options
            .iter()
            .any(|o| o.label == "Always allow `npm test`")
    );
    runtime::answer_impl(&state, &session, &ask.id, choose("accept")).unwrap();
    wait_for("the turn's end", || {
        row(&state, &session).runtime.unwrap().state.as_deref() == Some("idle")
    });

    stop(&state, &session);
}

#[test]
fn open_in_terminal_hands_the_thread_over_and_back() {
    use juggler_app::features::runtime::commands::{Toward, handover_impl};
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path(), None);
    let profile = new_profile(&state, CODEX_ID, work.path());
    let session = commands::launch_profile(&state, &profile, None)
        .unwrap()
        .session_id;
    runtime::send_impl(&state, &session, "hello").unwrap();
    wait_for("the turn's end", || {
        row(&state, &session).runtime.unwrap().state.as_deref() == Some("idle")
    });
    let thread = state
        .runtime
        .driver(&session)
        .unwrap()
        .snapshot()
        .native_id
        .unwrap();
    assert!(
        handover_impl(None, &state, &session, Toward::Driven).is_err(),
        "it already runs in AI Juggler"
    );

    // Open in terminal: the driver lets go, its server ends, the terminal
    // resumes the thread as a new session; the old one is handed over.
    let to_terminal = handover_impl(None, &state, &session, Toward::Terminal).unwrap();
    assert_ne!(to_terminal.session_id, session);
    assert_eq!(status_of(&state, &session), "handed_over");
    assert!(
        state
            .supervisor
            .is_finished(&SessionId::new(session.as_str())),
        "one writer"
    );
    assert!(
        state.runtime.driver(&to_terminal.session_id).is_none(),
        "in the terminal"
    );
    let record = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(to_terminal.session_id.as_str()))
        .unwrap();
    assert_eq!(
        record
            .conversation
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some(thread.as_str())
    );

    // Run in AI Juggler: only once the terminal has ended.
    wait_for("the terminal to end", || {
        state
            .supervisor
            .is_finished(&SessionId::new(to_terminal.session_id.as_str()))
    });
    let back = handover_impl(None, &state, &to_terminal.session_id, Toward::Driven).unwrap();
    let driver = state
        .runtime
        .driver(&back.session_id)
        .expect("driven again");
    assert_eq!(
        driver.snapshot().native_id.as_deref(),
        Some(thread.as_str()),
        "the same thread"
    );
    stop(&state, &back.session_id);
}

#[test]
fn the_terminal_runs_when_chosen_or_when_codex_cant_be_driven() {
    let work = tempfile::tempdir().unwrap();
    // This session's own "Start in terminal".
    let state = app_state(work.path(), None);
    let profile = new_profile(&state, CODEX_ID, work.path());
    let launched =
        commands::launch_profile_as(&state, &profile, None, Some(Runtime::Terminal)).unwrap();
    assert!(state.runtime.driver(&launched.session_id).is_none());
    assert!(
        row(&state, &launched.session_id).runtime.is_none(),
        "no fallback note: chosen"
    );
    stop(&state, &launched.session_id);
    // The profile's choice.
    runtime::choose_impl(&state, &profile, Some("terminal")).unwrap();
    let launched = commands::launch_profile(&state, &profile, None).unwrap();
    assert!(state.runtime.driver(&launched.session_id).is_none());
    stop(&state, &launched.session_id);

    // A Codex older than the pinned range.
    let old = tempfile::tempdir().unwrap();
    let state = app_state(
        old.path(),
        Some(codex_with(old.path(), "FAKE_CODEX_VERSION=0.150.0")),
    );
    let profile = new_profile(&state, CODEX_ID, old.path());
    let launched = commands::launch_profile(&state, &profile, None).unwrap();
    assert!(state.runtime.driver(&launched.session_id).is_none());
    let why = row(&state, &launched.session_id)
        .runtime
        .and_then(|r| r.fell_back)
        .unwrap();
    assert!(why.contains("older than 0.163.0"), "{why}");
    stop(&state, &launched.session_id);

    // A server on another home: refused after it started; the terminal
    // starts under a new id and the refused server is gone.
    let elsewhere = tempfile::tempdir().unwrap();
    let state = app_state(
        elsewhere.path(),
        Some(codex_with(
            elsewhere.path(),
            "FAKE_CODEX_HOME=/somewhere/else",
        )),
    );
    let profile = new_profile(&state, CODEX_ID, elsewhere.path());
    let launched = commands::launch_profile(&state, &profile, None).unwrap();
    assert!(launched.session_id.ends_with("-t"));
    let why = row(&state, &launched.session_id)
        .runtime
        .and_then(|r| r.fell_back)
        .unwrap();
    assert!(why.contains("/somewhere/else"), "{why}");
    let refused = launched.session_id.trim_end_matches("-t").to_owned();
    wait_for("the refused server to stop", || {
        state
            .supervisor
            .is_finished(&SessionId::new(refused.as_str()))
    });
    stop(&state, &launched.session_id);
}

/// AJ-E2305: a driven Codex session switched to Claude, then a driven one
/// with an approval open switched with Stop now: the driver lets go at the
/// tool's own turn boundary (or interrupts), its server ends, and the
/// switch carries the conversation as it does for a terminal session.
#[test]
fn switching_away_from_a_driven_session_releases_it_first() {
    use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
    let work = tempfile::tempdir().unwrap();
    let claude_stub = work.path().join("claude-stub");
    std::fs::write(
        &claude_stub,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CLAUDE_CONFIG_DIR/last-args.tmp\"\nmv \"$CLAUDE_CONFIG_DIR/last-args.tmp\" \"$CLAUDE_CONFIG_DIR/last-args\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&claude_stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut registry = AdapterRegistry::new();
    let codex =
        Arc::new(CodexAdapter::with_bin(fake_codex_bin()).with_home(work.path().join("home")));
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&claude_stub));
    registry.register(codex.clone());
    registry.register_handover(codex);
    registry.register(claude.clone());
    registry.register_handover(claude);
    let state =
        Arc::new(AppState::init_with_registry(work.path().join("app-data"), registry).unwrap());
    let project = work.path().join("storefront");
    std::fs::create_dir_all(&project).unwrap();
    let source_profile = new_profile(&state, CODEX_ID, &project);
    let target_profile = new_profile(&state, CLAUDE_CODE_ID, &project);

    for (freeze, prompt) in [("finish-turn", "hello"), ("stop-now", "ask: rm -rf build")] {
        let session = commands::launch_profile(&state, &source_profile, None)
            .unwrap()
            .session_id;
        assert!(state.runtime.is_driven(&SessionId::new(session.as_str())));
        runtime::send_impl(&state, &session, prompt).unwrap();
        if freeze == "finish-turn" {
            replied(&state, &session);
        } else {
            wait_for("the ask", || !runtime::asks_impl(&state, 0, 10).is_empty());
        }
        commands::switch_execute_impl(&state, &session, &target_profile, freeze, Some("keep"), &[])
            .unwrap_or_else(|error| panic!("{freeze}: {error}"));
        assert!(state.runtime.driver(&session).is_none(), "{freeze}: let go");
        assert!(
            state
                .supervisor
                .is_finished(&SessionId::new(session.as_str())),
            "{freeze}: its server ended before the target started"
        );
        assert!(runtime::asks_impl(&state, 0, 10).is_empty());
    }
}

fn fake_claude_bin() -> PathBuf {
    fake_codex_bin().with_file_name("fake-claude")
}

/// AJ-E2308: Claude Code without a terminal is a per-session Beta: only
/// with its switch on, only in a folder Claude Code trusts, never a
/// profile's standing choice; answered here, handed back to the terminal.
#[test]
fn claude_runs_without_a_terminal_only_as_a_per_session_beta() {
    use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
    use juggler_app::features::runtime::commands::{Toward, handover_impl};
    let work = tempfile::tempdir().unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(fake_claude_bin())));
    let state =
        Arc::new(AppState::init_with_registry(work.path().join("app-data"), registry).unwrap());
    let project = work.path().canonicalize().unwrap().join("lab");
    std::fs::create_dir_all(&project).unwrap();
    let profile = new_profile(&state, CLAUDE_CODE_ID, &project);
    let driven = || Some(Runtime::Driven);

    // The switch is off: the terminal, with the reason.
    let launched = commands::launch_profile_as(&state, &profile, None, driven()).unwrap();
    assert!(state.runtime.driver(&launched.session_id).is_none());
    let why = row(&state, &launched.session_id)
        .runtime
        .unwrap()
        .fell_back
        .unwrap();
    assert!(why.contains("(Beta) is off"), "{why}");
    // On, but Claude Code hasn't been trusted in the folder yet.
    runtime::beta_impl(&state, true).unwrap();
    let launched = commands::launch_profile_as(&state, &profile, None, driven()).unwrap();
    let why = row(&state, &launched.session_id)
        .runtime
        .unwrap()
        .fell_back
        .unwrap();
    assert!(why.contains("trusted"), "{why}");
    // Trusted (as Claude Code records it), and picked for this session.
    let state_dir = {
        let storage = state.storage.lock().unwrap();
        let recipe = storage
            .profiles()
            .get(&juggler_core::model::ProfileId::new(profile.as_str()))
            .unwrap();
        storage.accounts().get(&recipe.account).unwrap().state_dir
    };
    std::fs::write(
        state_dir.join(".claude.json"),
        serde_json::json!({"projects": {project.to_string_lossy(): {"hasTrustDialogAccepted": true}}}).to_string(),
    )
    .unwrap();
    let session = commands::launch_profile_as(&state, &profile, None, driven())
        .unwrap()
        .session_id;
    let driver = state.runtime.driver(&session).expect("driven, as picked");
    assert!(driver.beta);
    assert_eq!(driver.label, "Claude Code without a terminal");
    // A Beta is never a profile's standing choice.
    runtime::choose_impl(&state, &profile, Some("driven")).unwrap();
    let standing = commands::launch_profile(&state, &profile, None).unwrap();
    assert!(state.runtime.driver(&standing.session_id).is_none());

    // Its approval, with Claude Code's own rule, answered here.
    runtime::send_impl(&state, &session, "ask: ls -la").unwrap();
    wait_for("the ask", || {
        runtime::asks_impl(&state, 0, 10)
            .iter()
            .any(|a| a.session_id == session)
    });
    let ask = runtime::asks_impl(&state, 0, 10)
        .into_iter()
        .find(|a| a.session_id == session)
        .unwrap();
    assert!(
        ask.options
            .iter()
            .any(|o| o.id == "rule:0" && o.effect == "allow_rule")
    );
    runtime::answer_impl(&state, &session, &ask.id, choose("rule:0")).unwrap();
    wait_for("the turn's end", || {
        row(&state, &session).runtime.unwrap().state.as_deref() == Some("idle")
    });

    // Open in terminal: one writer at a time, `claude --resume <id>`.
    let next = handover_impl(None, &state, &session, Toward::Terminal).unwrap();
    assert_eq!(status_of(&state, &session), "handed_over");
    assert!(
        state
            .supervisor
            .is_finished(&SessionId::new(session.as_str()))
    );
    assert!(state.runtime.driver(&next.session_id).is_none());
}

#[test]
fn a_profile_choice_is_kept_and_a_bad_word_refused() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path(), None);
    let profile = new_profile(&state, CODEX_ID, work.path());
    let choices = runtime::choose_impl(&state, &profile, Some("terminal")).unwrap();
    let row = choices
        .profiles
        .iter()
        .find(|p| p.profile_id == profile)
        .unwrap();
    assert_eq!(row.chosen.as_deref(), Some("terminal"));
    assert_eq!(row.default, "driven");
    assert_eq!(row.label, "Codex's app-server");
    assert!(runtime::choose_impl(&state, &profile, Some("pty")).is_err());
    let cleared = runtime::choose_impl(&state, &profile, None).unwrap();
    assert!(cleared.profiles.iter().all(|p| p.chosen.is_none()));
    assert!(!cleared.beta_on);
    assert!(runtime::beta_impl(&state, true).unwrap().beta_on);
}

// ---------------------------------------------------------------------
// AJ-E2310: a driven session comes back after the app core is killed.

/// Waits for the driven session's turn to end with the agent's reply
/// (idle alone can be the state before the turn started).
fn replied(state: &Arc<AppState>, session: &str) {
    wait_for("the reply", || {
        let snap = state.runtime.driver(session).unwrap().snapshot();
        !snap.turn_running && snap.items.iter().any(|item| item.kind == "agent_message")
    });
}

/// The data folder the child app core runs in.
const CORE_DATA: &str = "AIJ_PICKUP_CORE";
/// Where the child leaves its session when killed: `between`, `mid` or
/// `stopped`.
const CORE_STAGE: &str = "AIJ_PICKUP_STAGE";

/// The app core, in a child process: a Codex session on the fake
/// app-server brought to the stage asked, its ids written down, then it
/// waits to be killed (and ends by itself after a minute, never left
/// behind).
#[test]
#[ignore = "run by the pickup tests below, in a child process"]
fn app_core_in_a_child() {
    let work = PathBuf::from(std::env::var(CORE_DATA).unwrap());
    let stage = std::env::var(CORE_STAGE).unwrap();
    let state = app_state(&work, None);
    let profile = new_profile(&state, CODEX_ID, &work);
    let launched = commands::launch_profile(&state, &profile, None).unwrap();
    let session = launched.session_id;
    runtime::send_impl(&state, &session, "hello").unwrap();
    replied(&state, &session);
    let thread = state
        .runtime
        .driver(&session)
        .unwrap()
        .snapshot()
        .native_id
        .unwrap();
    let marks = || juggler_app::features::runtime::pickup::marks(&state.storage.lock().unwrap());
    wait_for("the first turn's mark", || {
        marks().get(&session).is_some_and(|mark| !mark.in_turn)
    });
    match stage.as_str() {
        "mid" => {
            runtime::send_impl(&state, &session, "ask: npm test").unwrap();
            wait_for("the ask", || !runtime::asks_impl(&state, 0, 10).is_empty());
        }
        "stopped" => stop(&state, &session),
        _ => {}
    }
    // The mark is in the store before the kill (no message text in it).
    if stage == "stopped" {
        assert!(marks().is_empty(), "{:?}", marks());
    } else {
        wait_for("the mark", || {
            marks()
                .get(&session)
                .is_some_and(|mark| mark.native_id == thread && mark.in_turn == (stage == "mid"))
        });
    }
    // Written whole, then moved into place: the parent reads it as soon as
    // it exists, and a half-written one failed under load (0.3.4 preflight).
    let partial = work.join("core-ready.tmp");
    std::fs::write(&partial, format!("{session} {thread} {}", launched.pid)).unwrap();
    std::fs::rename(&partial, work.join("core-ready")).unwrap();
    std::thread::sleep(Duration::from_secs(60));
}

/// Whether process `pid` is alive (`kill -0`).
fn alive(pid: &str) -> bool {
    std::process::Command::new("kill")
        .args(["-0", pid])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Runs the app core in a child to `stage`, kills it (`SIGKILL`), and
/// waits for the tool's server to end with it. Returns the session and
/// its thread.
fn kill_core_at(work: &Path, stage: &str) -> (String, String) {
    let mut core = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "app_core_in_a_child", "--ignored", "--nocapture"])
        .env(CORE_DATA, work)
        .env(CORE_STAGE, stage)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let ready = work.join("core-ready");
    let deadline = Instant::now() + Duration::from_secs(60);
    while !ready.is_file() {
        if let Some(status) = core.try_wait().unwrap() {
            panic!("the app core ended before its session was ready: {status}");
        }
        assert!(Instant::now() < deadline, "the app core never got ready");
        std::thread::sleep(Duration::from_millis(50));
    }
    let text = std::fs::read_to_string(&ready).unwrap();
    let mut words = text.split_whitespace();
    let (session, thread, server) = (
        words.next().unwrap().to_owned(),
        words.next().unwrap().to_owned(),
        words.next().unwrap().to_owned(),
    );
    core.kill().unwrap();
    core.wait().unwrap();
    // Its input closed with the core: the tool's server ends on its own.
    wait_for("the server to end with the app core", || !alive(&server));
    (session, thread)
}

#[test]
fn a_session_between_turns_comes_back_driven_on_its_thread_after_a_kill() {
    use juggler_app::features::runtime::pickup;
    let work = tempfile::tempdir().unwrap();
    let (session, thread) = kill_core_at(work.path(), "between");

    // A new app core on the same data.
    let state = app_state(work.path(), None);
    assert_eq!(
        status_of(&state, &session),
        "starting",
        "kept for the pickup"
    );
    pickup::after_restart(&state);
    wait_for("the session to come back", || {
        state.runtime.driver(&session).is_some()
    });
    let driver = state.runtime.driver(&session).unwrap();
    assert_eq!(
        driver.snapshot().native_id.as_deref(),
        Some(thread.as_str()),
        "the same thread"
    );
    assert!(
        state
            .supervisor
            .is_supervised(&SessionId::new(session.as_str()))
    );
    // No turn was sent by itself; the next one is the user's.
    assert!(runtime::asks_impl(&state, 0, 10).is_empty());
    runtime::send_impl(&state, &session, "again").unwrap();
    replied(&state, &session);
    let rollout = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(session.as_str()))
        .unwrap()
        .transcript_ref
        .unwrap()
        .path;
    let lines = std::fs::read_to_string(rollout).unwrap();
    assert!(
        lines.contains("hello") && lines.contains("again"),
        "one conversation"
    );
    stop(&state, &session);
    assert!(
        pickup::marks(&state.storage.lock().unwrap()).is_empty(),
        "a stop forgets it"
    );
}

#[test]
fn a_session_killed_mid_turn_says_so_and_waits_for_resume() {
    use juggler_app::features::runtime::pickup;
    let work = tempfile::tempdir().unwrap();
    let (session, thread) = kill_core_at(work.path(), "mid");

    let state = app_state(work.path(), None);
    pickup::after_restart(&state);
    let info = row(&state, &session).runtime.unwrap();
    assert_eq!(info.cut_off.as_deref(), Some(pickup::CUT_OFF_WORDS));
    assert_eq!(info.label.as_deref(), Some("Codex's app-server"));
    assert_eq!(status_of(&state, &session), "waiting_user");
    let record = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(session.as_str()))
        .unwrap();
    assert_eq!(record.attention.unwrap().rule, pickup::CUT_OFF_RULE);
    // Nothing runs by itself.
    std::thread::sleep(Duration::from_millis(500));
    assert!(state.runtime.driver(&session).is_none());
    assert!(
        !state
            .supervisor
            .is_supervised(&SessionId::new(session.as_str()))
    );

    // The user's Resume: the same thread, nothing sent.
    let resumed = pickup::resume_impl(&state, &session).unwrap();
    assert_eq!(resumed.session_id, session);
    let driver = state.runtime.driver(&session).expect("driven again");
    assert_eq!(
        driver.snapshot().native_id.as_deref(),
        Some(thread.as_str())
    );
    assert!(row(&state, &session).runtime.unwrap().cut_off.is_none());
    assert!(pickup::resume_impl(&state, &session).is_err(), "once");
    runtime::send_impl(&state, &session, "hello again").unwrap();
    replied(&state, &session);
    stop(&state, &session);
}

#[test]
fn a_cut_off_session_can_be_ended_and_a_stopped_one_never_comes_back() {
    use juggler_app::features::runtime::pickup;
    let work = tempfile::tempdir().unwrap();
    let (session, _) = kill_core_at(work.path(), "mid");
    let state = app_state(work.path(), None);
    pickup::after_restart(&state);
    pickup::end_impl(&state, &session).unwrap();
    assert_eq!(status_of(&state, &session), "stopped");
    assert!(pickup::marks(&state.storage.lock().unwrap()).is_empty());
    assert!(state.runtime.info(&session).is_none());
    drop(state);

    let other = tempfile::tempdir().unwrap();
    let (stopped, _) = kill_core_at(other.path(), "stopped");
    let state = app_state(other.path(), None);
    pickup::after_restart(&state);
    std::thread::sleep(Duration::from_millis(500));
    assert!(state.runtime.driver(&stopped).is_none());
    assert_ne!(status_of(&state, &stopped), "starting");
    assert!(state.runtime.info(&stopped).is_none());
}

// ---------------------------------------------------------------------
// Lab 2: Codex's folder trust is the user's choice in AI Juggler, passed
// in memory; Codex's own config is never written.

/// The launches `fake-codex` saw (one line each) that contain `word`.
fn launches_with(log: &Path, word: &str) -> Vec<String> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains(word))
        .map(str::to_owned)
        .collect()
}

#[test]
fn codex_gets_the_folders_trust_in_memory_and_its_config_is_never_written() {
    use juggler_app::features::runtime::commands::{Toward, handover_impl};
    use juggler_app::features::runtime::trust;
    let work = tempfile::tempdir().unwrap();
    let log = work.path().join("codex-args.log");
    let state = app_state(
        work.path(),
        Some(codex_with(
            work.path(),
            &format!("FAKE_CODEX_ARGS_LOG={}", log.display()),
        )),
    );
    let profile = new_profile(&state, CODEX_ID, work.path());
    let folder = work.path().canonicalize().unwrap();
    let override_of = |level: &str| {
        format!(
            "-c projects={{\"{}\"={{trust_level=\"{level}\"}}}}",
            folder.display()
        )
    };

    // Codex's own question, not answered yet.
    let asked = trust::trust_impl(&state, &profile, None).unwrap().unwrap();
    assert_eq!(
        (
            asked.question.as_str(),
            asked.trust.as_str(),
            asked.restrict.as_str()
        ),
        (
            "Trust this folder?",
            "Trust and continue",
            "Open restricted"
        )
    );
    assert!(asked.choice.is_none());

    // Not answered: restricted, in memory (Codex's trust screen would
    // have saved an answer to its config).
    let first = commands::launch_profile(&state, &profile, None)
        .unwrap()
        .session_id;
    wait_for("the launch", || !launches_with(&log, "--listen").is_empty());
    assert!(launches_with(&log, "--listen")[0].starts_with(&override_of("untrusted")));
    stop(&state, &first);

    // The user's answer, kept and passed to each launch.
    let answered = trust::trust_choose_impl(&state, &profile, None, true)
        .unwrap()
        .unwrap();
    assert_eq!(answered.choice.as_deref(), Some("trusted"));
    let session = commands::launch_profile(&state, &profile, None)
        .unwrap()
        .session_id;
    wait_for("the second launch", || {
        launches_with(&log, "--listen").len() == 2
    });
    assert!(launches_with(&log, "--listen")[1].starts_with(&override_of("trusted")));

    // Open in terminal carries it too.
    runtime::send_impl(&state, &session, "hello").unwrap();
    replied(&state, &session);
    let terminal = handover_impl(None, &state, &session, Toward::Terminal).unwrap();
    wait_for("the terminal", || {
        !launches_with(&log, " resume ").is_empty()
    });
    assert!(launches_with(&log, " resume ")[0].starts_with(&override_of("trusted")));
    wait_for("the terminal to end", || {
        state
            .supervisor
            .is_finished(&SessionId::new(terminal.session_id.as_str()))
    });

    // Codex's own config was never written.
    let home = state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .list()
        .unwrap()
        .into_iter()
        .find(|a| a.harness.as_str() == CODEX_ID)
        .unwrap()
        .state_dir;
    assert!(!home.join("config.toml").exists());

    // A folder the user decided in Codex itself is left to Codex.
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!(
            "[projects.\"{}\"]\ntrust_level = \"untrusted\"\n",
            folder.display()
        ),
    )
    .unwrap();
    let decided = trust::trust_impl(&state, &profile, None).unwrap().unwrap();
    assert_eq!(decided.choice.as_deref(), Some("in_tool"));
    let third = commands::launch_profile(&state, &profile, None)
        .unwrap()
        .session_id;
    wait_for("the third launch", || {
        launches_with(&log, "--listen").len() == 3
    });
    assert!(!launches_with(&log, "--listen")[2].contains("-c projects="));
    stop(&state, &third);
}
