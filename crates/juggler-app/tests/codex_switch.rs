//! AJ-E1304 integration: switching with Codex, end to end against stub
//! binaries — Codex→Codex transplants the rollout and resumes it on the
//! target; Claude→Codex and Codex→Claude replay the (redacted)
//! conversation into a primed session; an unanswered secret policy
//! blocks a replay. The source file is byte-identical every time.

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

use juggler_adapter::AdapterRegistry;
use juggler_adapter::handover::sha256_hex;
use juggler_adapters::{CLAUDE_CODE_ID, CODEX_ID, ClaudeCodeAdapter, CodexAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::{SessionId, SessionStatus, SwitchEdgeId, SwitchMechanism};

/// A stub that records its argv (one per line) under its own home —
/// written to a temp file and renamed, so a reader never sees half.
fn stub(dir: &Path, name: &str, home_var: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"${home_var}/last-args.tmp\"\nmv \"${home_var}/last-args.tmp\" \"${home_var}/last-args\"\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn app_state(dir: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(stub(
        dir,
        "claude-stub",
        "CLAUDE_CONFIG_DIR",
    )));
    let codex = Arc::new(CodexAdapter::with_bin(stub(
        dir,
        "codex-stub",
        "CODEX_HOME",
    )));
    registry.register(claude.clone());
    registry.register_handover(claude);
    registry.register(codex.clone());
    registry.register_handover(codex);
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn home(state: &Arc<AppState>, provider: &str, account: &str) -> PathBuf {
    state.accounts_dir().join(provider).join(account)
}

/// Waits for (and removes) a stub's argv record.
fn args_of(home: &Path) -> Vec<String> {
    let file = home.join("last-args");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !file.is_file() {
        assert!(std::time::Instant::now() < deadline, "the stub never ran");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let args = std::fs::read_to_string(&file)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    std::fs::remove_file(&file).unwrap();
    args
}

const THREAD: &str = "019a1b2c-0000-7000-8000-00000000c0de";
const SECRET: &str = "sk-abc123def456ghi789";

fn write_rollout(home: &Path, cwd: &str) -> PathBuf {
    let day = home.join("sessions/2026/10/20");
    std::fs::create_dir_all(&day).unwrap();
    let path = day.join(format!("rollout-2026-10-20T08-00-00-{THREAD}.jsonl"));
    let lines = [
        format!(
            r#"{{"timestamp":"2026-10-20T08:00:00.000Z","type":"session_meta","payload":{{"id":"{THREAD}","cwd":"{cwd}","cli_version":"0.163.0"}}}}"#
        ),
        r#"{"timestamp":"2026-10-20T08:00:01.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Add a health check endpoint"}]}}"#.to_owned(),
        r#"{"timestamp":"2026-10-20T08:00:05.000Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Added GET /health."}]}}"#.to_owned(),
    ];
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    path
}

fn write_claude_transcript(home: &Path, cwd: &str) -> PathBuf {
    let dir = home.join("projects").join(cwd.replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("claude-sess-1.jsonl");
    let lines = [
        format!(
            r#"{{"type":"user","uuid":"u1","cwd":"{cwd}","sessionId":"claude-sess-1","message":{{"role":"user","content":"Deploy with key {SECRET} please"}}}}"#
        ),
        format!(
            r#"{{"type":"assistant","uuid":"u2","parentUuid":"u1","cwd":"{cwd}","sessionId":"claude-sess-1","message":{{"role":"assistant","content":[{{"type":"text","text":"Deployed."}}]}}}}"#
        ),
    ];
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    path
}

struct Setup {
    state: Arc<AppState>,
    cwd: String,
    source_session: String,
    source_home: PathBuf,
    target_home: PathBuf,
    target_recipe: String,
}

fn setup(work: &Path, from: &str, to: &str) -> Setup {
    let state = app_state(work);
    let project = work.join("storefront");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let a = commands::create_account(&state, from, "Source", None).unwrap();
    let b = commands::create_account(&state, to, "Target", None).unwrap();
    let recipe_a = commands::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let recipe_b = commands::create_profile(&state, &b.id, "Main", None, None).unwrap();
    let launched = commands::launch_profile(&state, &recipe_a.id, Some(cwd.clone())).unwrap();
    let source_home = home(&state, from, &a.id);
    let _ = args_of(&source_home); // the setup launch's own record
    Setup {
        target_home: home(&state, to, &b.id),
        state,
        cwd,
        source_session: launched.session_id,
        source_home,
        target_recipe: recipe_b.id,
    }
}

fn mechanism_of(state: &Arc<AppState>, edge: &str) -> SwitchMechanism {
    state
        .storage
        .lock()
        .unwrap()
        .switch_graph()
        .edge(&SwitchEdgeId::new(edge))
        .unwrap()
        .mechanism
}

fn status_of(state: &Arc<AppState>, session: &str) -> SessionStatus {
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(session))
        .unwrap()
        .status
}

#[test]
fn codex_to_codex_transplants_the_rollout_and_resumes_it() {
    let work = tempfile::tempdir().unwrap();
    let s = setup(work.path(), CODEX_ID, CODEX_ID);
    let source = write_rollout(&s.source_home, &s.cwd);
    let before = sha256_hex(&std::fs::read(&source).unwrap());

    let info = commands::switch_execute_impl(
        &s.state,
        &s.source_session,
        &s.target_recipe,
        "finish-turn",
        None,
        &[],
    )
    .unwrap();

    let args = args_of(&s.target_home);
    assert_eq!(
        &args[..2],
        &["resume".to_owned(), THREAD.to_owned()],
        "{args:?}"
    );
    assert!(args.contains(&"--no-daemon".to_owned()));
    assert!(
        s.target_home
            .join("sessions/2026/10/20")
            .join(format!("rollout-2026-10-20T08-00-00-{THREAD}.jsonl"))
            .is_file(),
        "installed where Codex looks"
    );
    assert_eq!(sha256_hex(&std::fs::read(&source).unwrap()), before);
    assert_eq!(
        mechanism_of(&s.state, &info.edge_id),
        SwitchMechanism::Transplant
    );
    assert_eq!(
        status_of(&s.state, &s.source_session),
        SessionStatus::HandedOver
    );
}

#[test]
fn claude_to_codex_replays_the_redacted_conversation() {
    let work = tempfile::tempdir().unwrap();
    let s = setup(work.path(), CLAUDE_CODE_ID, CODEX_ID);
    let source = write_claude_transcript(&s.source_home, &s.cwd);
    let before = sha256_hex(&std::fs::read(&source).unwrap());
    commands::redaction_policy_set_global_impl(&s.state, Some("strip")).unwrap();

    let info = commands::switch_execute_impl(
        &s.state,
        &s.source_session,
        &s.target_recipe,
        "finish-turn",
        None,
        &[],
    )
    .unwrap();

    let args = args_of(&s.target_home);
    assert_eq!(args[0], "--no-daemon", "a primed interactive Codex session");
    let prompt = args[1..].join("\n");
    assert!(
        prompt.contains("[user]:\n> Deploy with key"),
        "the conversation travels"
    );
    assert!(prompt.contains("[assistant]:\n> Deployed."));
    assert!(
        !prompt.contains(SECRET),
        "the key is stripped before it leaves"
    );
    assert!(prompt.contains("•••"));
    assert_eq!(sha256_hex(&std::fs::read(&source).unwrap()), before);
    assert_eq!(
        mechanism_of(&s.state, &info.edge_id),
        SwitchMechanism::Replay
    );
    assert_eq!(
        status_of(&s.state, &s.source_session),
        SessionStatus::HandedOver
    );
    // Nothing of Claude's was installed into the Codex home.
    assert!(!s.target_home.join("projects").exists());
    assert!(!s.target_home.join("sessions").exists());
}

#[test]
fn codex_to_claude_replays_too() {
    let work = tempfile::tempdir().unwrap();
    let s = setup(work.path(), CODEX_ID, CLAUDE_CODE_ID);
    let source = write_rollout(&s.source_home, &s.cwd);
    let before = sha256_hex(&std::fs::read(&source).unwrap());

    let info = commands::switch_execute_impl(
        &s.state,
        &s.source_session,
        &s.target_recipe,
        "finish-turn",
        Some("keep"),
        &[],
    )
    .unwrap();

    let prompt = args_of(&s.target_home).join("\n");
    assert!(prompt.contains("[user]:\n> Add a health check endpoint"));
    assert!(prompt.contains("[assistant]:\n> Added GET /health."));
    assert_eq!(sha256_hex(&std::fs::read(&source).unwrap()), before);
    assert_eq!(
        mechanism_of(&s.state, &info.edge_id),
        SwitchMechanism::Replay
    );
}

#[test]
fn a_replay_waits_for_a_secret_policy() {
    let work = tempfile::tempdir().unwrap();
    let s = setup(work.path(), CLAUDE_CODE_ID, CODEX_ID);
    write_claude_transcript(&s.source_home, &s.cwd);

    // Unset: blocked, nothing launched, the source keeps running.
    let unset = commands::switch_execute_impl(
        &s.state,
        &s.source_session,
        &s.target_recipe,
        "finish-turn",
        None,
        &[],
    )
    .unwrap_err();
    assert!(
        unset.to_string().contains("secrets"),
        "names the missing choice"
    );
    // "Ask each time" with no answer on this switch: blocked too.
    commands::redaction_policy_set_global_impl(&s.state, Some("ask")).unwrap();
    assert!(
        commands::switch_execute_impl(
            &s.state,
            &s.source_session,
            &s.target_recipe,
            "finish-turn",
            None,
            &[],
        )
        .is_err()
    );
    assert!(
        !s.target_home.join("last-args").exists(),
        "nothing launched"
    );
    assert_ne!(
        status_of(&s.state, &s.source_session),
        SessionStatus::HandedOver
    );
}

#[test]
fn an_oversized_newest_turn_still_travels_cut_to_fit() {
    let work = tempfile::tempdir().unwrap();
    let s = setup(work.path(), CLAUDE_CODE_ID, CODEX_ID);
    let dir = s.source_home.join("projects").join(s.cwd.replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    let huge = "é".repeat(70_000); // multi-byte: the cut must respect chars
    std::fs::write(
        dir.join("claude-sess-1.jsonl"),
        format!(
            "{{\"type\":\"user\",\"uuid\":\"u1\",\"cwd\":\"{}\",\"message\":{{\"role\":\"user\",\"content\":\"{huge}\"}}}}\n",
            s.cwd
        ),
    )
    .unwrap();
    commands::switch_execute_impl(
        &s.state,
        &s.source_session,
        &s.target_recipe,
        "finish-turn",
        Some("keep"),
        &[],
    )
    .unwrap();
    let prompt = args_of(&s.target_home).join("\n");
    assert!(
        prompt.contains("cut to fit"),
        "the newest turn travels, cut"
    );
    assert!(prompt.contains(&"é".repeat(1_000)));
}

#[test]
fn a_session_with_no_conversation_is_not_replayed() {
    let work = tempfile::tempdir().unwrap();
    let s = setup(work.path(), CLAUDE_CODE_ID, CODEX_ID);
    let dir = s.source_home.join("projects").join(s.cwd.replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("claude-sess-1.jsonl"),
        format!("{{\"type\":\"system\",\"cwd\":\"{}\"}}\n", s.cwd),
    )
    .unwrap();
    let error = commands::switch_execute_impl(
        &s.state,
        &s.source_session,
        &s.target_recipe,
        "finish-turn",
        Some("keep"),
        &[],
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("no conversation"),
        "refused in plain words"
    );
    assert!(
        !s.target_home.join("last-args").exists(),
        "nothing launched"
    );
    assert_ne!(
        status_of(&s.state, &s.source_session),
        SessionStatus::HandedOver
    );
}
