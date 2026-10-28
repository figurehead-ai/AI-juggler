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
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

/// A stub that records its argv (one per line) under its own home —
/// written to a temp file and renamed, so a reader never sees half.
fn stub(dir: &Path, name: &str, home_var: &str) -> PathBuf {
    let path = dir.join(name);
    stand_in::write(
        &path,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"${home_var}/last-args.tmp\"\nmv \"${home_var}/last-args.tmp\" \"${home_var}/last-args\"\n"
        ),
    );
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
/// AJ-E1541: the replay's brief — the handover file the first prompt
/// points at, on a line of its own.
fn brief_of(args: &[String]) -> String {
    let path = args
        .iter()
        .find(|line| line.ends_with("HANDOVER.md"))
        .unwrap_or_else(|| panic!("the prompt points at its handover: {args:?}"));
    std::fs::read_to_string(path).unwrap()
}

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
    // AJ-E1541: the conversation is in the handover file, never on the
    // command line (which `ps` shows every local user).
    assert!(!args.join("\n").contains("Deploy with key"), "{args:?}");
    let prompt = brief_of(&args);
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

    let args = args_of(&s.target_home);
    // Claude may read the handover's folder without asking.
    let at = args
        .iter()
        .position(|arg| arg == "--add-dir")
        .expect("--add-dir");
    assert!(
        args[at + 1].ends_with(&format!("switches/{}", info.edge_id)),
        "{args:?}"
    );
    let prompt = brief_of(&args);
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
    // ~300 KB of 3-byte characters in short lines (quoting adds bytes per
    // line): far over one argv entry's limit.
    let line = "語".repeat(40);
    let huge = vec![line.as_str(); 2_500].join("\\n");
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
    // The command line holds only the pointer; the brief is in the
    // handover file, within what the target reads.
    let args = args_of(&s.target_home);
    assert!(args.join("\n").len() < 2 * 1024, "{args:?}");
    let prompt = brief_of(&args);
    assert!(
        prompt.len() <= 96 * 1024 + 8 * 1024,
        "the brief stays within its budget: {}",
        prompt.len()
    );
    assert!(
        prompt.contains("cut to fit"),
        "the newest turn travels, cut"
    );
    assert!(prompt.contains(&"語".repeat(40)), "whole characters only");
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

/// AJ-E1541: a switch to another tool carries the work, not only the chat:
/// what each tool call did, the task list as last set, and the project's
/// uncommitted changes. Before, only user and assistant text travelled.
#[test]
fn claude_to_codex_carries_tool_calls_the_task_list_and_the_changes() {
    let work = tempfile::tempdir().unwrap();
    let s = setup(work.path(), CLAUDE_CODE_ID, CODEX_ID);
    let git = |args: &[&str]| {
        assert!(
            std::process::Command::new("git")
                .args(args)
                .current_dir(&s.cwd)
                .output()
                .unwrap()
                .status
                .success()
        );
    };
    git(&["init", "-q"]);
    std::fs::write(std::path::Path::new(&s.cwd).join("README.md"), "start\n").unwrap();
    git(&["add", "-A"]);
    git(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@example.com",
        "commit",
        "-qm",
        "start",
    ]);
    std::fs::write(
        std::path::Path::new(&s.cwd).join("calc.py"),
        "def add(a, b):\n    return a + b\n",
    )
    .unwrap();

    let dir = s.source_home.join("projects").join(s.cwd.replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    let cwd = &s.cwd;
    let lines = [
        format!(
            r#"{{"type":"user","uuid":"u1","cwd":"{cwd}","sessionId":"claude-sess-2","message":{{"role":"user","content":"Write calc.py with add, and keep a task list"}}}}"#
        ),
        format!(
            r#"{{"type":"assistant","uuid":"u2","cwd":"{cwd}","sessionId":"claude-sess-2","message":{{"role":"assistant","content":[{{"type":"tool_use","id":"t1","name":"Write","input":{{"file_path":"{cwd}/calc.py","content":"…"}}}},{{"type":"tool_use","id":"t2","name":"TodoWrite","input":{{"todos":[{{"content":"add","status":"completed"}},{{"content":"sub","status":"pending"}}]}}}}]}}}}"#
        ),
        format!(
            r#"{{"type":"assistant","uuid":"u3","cwd":"{cwd}","sessionId":"claude-sess-2","message":{{"role":"assistant","content":[{{"type":"text","text":"calc.py has add."}}]}}}}"#
        ),
    ];
    std::fs::write(dir.join("claude-sess-2.jsonl"), lines.join("\n") + "\n").unwrap();
    std::fs::write(
        std::path::Path::new(&s.cwd).join("CLAUDE.md"),
        "Run the tests.\n",
    )
    .unwrap();
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
    let prompt = brief_of(&args_of(&s.target_home));
    assert!(
        prompt.contains("Write storefront/calc.py"),
        "the tool call travels: {prompt}"
    );
    assert!(
        prompt.contains("Task list:\n[x] add\n[ ] sub"),
        "the task list travels: {prompt}"
    );
    assert!(
        prompt.contains("Not committed yet"),
        "the changes travel: {prompt}"
    );
    assert!(prompt.contains("?? calc.py"), "{prompt}");
    // The rules files in play, so Codex reads Claude's rules too.
    assert!(
        prompt.contains("CLAUDE.md (read by Claude Code)"),
        "the rules travel: {prompt}"
    );
    let state: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            s.state
                .data_dir
                .join("switches")
                .join(&info.edge_id)
                .join("state.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(state["to"]["tool"], CODEX_ID);
    assert_eq!(state["rules_files"][0]["name"], "CLAUDE.md");
    assert_eq!(state["turns_left_out"], 0);
}

/// AJ-E1541 Req.2: older turns that don't fit are named by what was asked
/// in them, not only counted.
#[test]
fn older_turns_that_dont_fit_are_named_by_what_was_asked() {
    let work = tempfile::tempdir().unwrap();
    let s = setup(work.path(), CLAUDE_CODE_ID, CODEX_ID);
    let dir = s.source_home.join("projects").join(s.cwd.replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    let cwd = &s.cwd;
    let huge = "x".repeat(120 * 1024);
    let lines = [
        format!(
            r#"{{"type":"user","uuid":"u1","cwd":"{cwd}","message":{{"role":"user","content":"Plan the payments refactor\nin three steps"}}}}"#
        ),
        format!(
            r#"{{"type":"user","uuid":"u2","cwd":"{cwd}","message":{{"role":"user","content":"Paste the log: {huge}"}}}}"#
        ),
        format!(
            r#"{{"type":"assistant","uuid":"u3","cwd":"{cwd}","message":{{"role":"assistant","content":[{{"type":"text","text":"Step one is done."}}]}}}}"#
        ),
    ];
    std::fs::write(dir.join("claude-sess-1.jsonl"), lines.join("\n") + "\n").unwrap();
    commands::switch_execute_impl(
        &s.state,
        &s.source_session,
        &s.target_recipe,
        "finish-turn",
        Some("keep"),
        &[],
    )
    .unwrap();
    let prompt = brief_of(&args_of(&s.target_home));
    assert!(
        prompt.contains("(2 earlier turns are not included. What was asked in them, oldest first:"),
        "{}",
        &prompt[..400.min(prompt.len())]
    );
    assert!(
        prompt.contains("\n> - Plan the payments refactor\n"),
        "the topic, its first line"
    );
    assert!(
        prompt.contains("Step one is done."),
        "the newest turn travels"
    );
    assert!(
        !prompt.contains(&"x".repeat(200)),
        "the left-out turn itself doesn't"
    );
}

/// A Codex conversation in `cwd` whose first message is `first`, with one
/// reply. Dated ahead, so it began after any launch in the test.
fn write_later_rollout(home: &Path, cwd: &str, id: &str, first: &str, reply: &str) {
    let day = home.join("sessions/2030/01/01");
    std::fs::create_dir_all(&day).unwrap();
    let text = |s: &str| serde_json::to_string(s).unwrap();
    let lines = [
        format!(
            r#"{{"timestamp":"2030-01-01T08:00:00.000Z","type":"session_meta","payload":{{"id":"{id}","cwd":"{cwd}","cli_version":"0.163.0"}}}}"#
        ),
        // As real rollouts do: Codex's own context comes first.
        r#"{"timestamp":"2030-01-01T08:00:00.500Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>cwd</environment_context>"}]}}"#.to_owned(),
        format!(
            r#"{{"timestamp":"2030-01-01T08:00:01.000Z","type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":{}}}]}}}}"#,
            text(first)
        ),
        format!(
            r#"{{"timestamp":"2030-01-01T08:00:05.000Z","type":"response_item","payload":{{"type":"message","role":"assistant","content":[{{"type":"output_text","text":{}}}]}}}}"#,
            text(reply)
        ),
    ];
    std::fs::write(
        day.join(format!("rollout-2030-01-01T08-00-00-{id}.jsonl")),
        lines.join("\n") + "\n",
    )
    .unwrap();
}

#[test]
fn switching_back_finds_the_replays_conversation_beside_another_one() {
    // Lab 8: another Codex session began in the same folder after the
    // replay, so its conversation couldn't be told apart by time, and
    // switching back left the Codex work out without a word.
    let work = tempfile::tempdir().unwrap();
    let s = setup(work.path(), CLAUDE_CODE_ID, CODEX_ID);
    write_claude_transcript(&s.source_home, &s.cwd);
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
    // What the replay hands Codex: the argument after `--`.
    let args = args_of(&s.target_home);
    let at = args.iter().position(|a| a == "--").unwrap();
    let prompt = args[at + 1..].join("\n");
    write_later_rollout(
        &s.target_home,
        &s.cwd,
        "019a1b2c-0000-7000-8000-0000000000aa",
        &prompt,
        "Picked it up: the deploy is done.",
    );
    write_later_rollout(
        &s.target_home,
        &s.cwd,
        "019a1b2c-0000-7000-8000-0000000000bb",
        "What does this repo do?",
        "It's a storefront.",
    );

    let plan = commands::switch_back_plan_impl(&s.state, &info.session_id).unwrap();
    assert!(
        plan.delta_preview
            .contains("Picked it up: the deploy is done."),
        "{}",
        plan.delta_preview
    );
    assert!(!plan.delta_preview.contains("storefront"));
    assert!(!plan.delta_preview.contains("couldn't find"));
}

#[test]
fn switching_back_says_when_it_cant_tell_which_conversation() {
    let work = tempfile::tempdir().unwrap();
    let s = setup(work.path(), CLAUDE_CODE_ID, CODEX_ID);
    write_claude_transcript(&s.source_home, &s.cwd);
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
    let _ = args_of(&s.target_home);
    // Two conversations, neither the replay's: never a guess.
    for (id, first) in [
        ("019a1b2c-0000-7000-8000-0000000000cc", "Fix the header"),
        ("019a1b2c-0000-7000-8000-0000000000dd", "Fix the footer"),
    ] {
        write_later_rollout(&s.target_home, &s.cwd, id, first, "Done.");
    }

    let plan = commands::switch_back_plan_impl(&s.state, &info.session_id).unwrap();
    assert!(
        plan.delta_preview.contains(
            "On Target: AI Juggler couldn't find the conversation there, so what happened on it isn't included."
        ),
        "{}",
        plan.delta_preview
    );
}

#[test]
fn a_replay_that_cant_go_never_stops_the_source() {
    // Review of #132 (M4): the source was stopped first, and the missing
    // secrets choice refused the switch after: the user's session ended
    // for nothing. A Claude stand-in that keeps running shows it.
    let work = tempfile::tempdir().unwrap();
    let dir = work.path();
    let mut registry = AdapterRegistry::new();
    let lingering = dir.join("claude-lingering");
    stand_in::write(&lingering, "#!/bin/sh\nexec sleep 30\n");
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(lingering));
    let codex = Arc::new(CodexAdapter::with_bin(stub(
        dir,
        "codex-stub",
        "CODEX_HOME",
    )));
    registry.register(claude.clone());
    registry.register_handover(claude);
    registry.register(codex.clone());
    registry.register_handover(codex);
    let state = Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap());
    let project = dir.join("storefront");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let a = commands::create_account(&state, CLAUDE_CODE_ID, "Source", None).unwrap();
    let b = commands::create_account(&state, CODEX_ID, "Target", None).unwrap();
    let recipe_a = commands::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let recipe_b = commands::create_profile(&state, &b.id, "Main", None, None).unwrap();
    let launched = commands::launch_profile(&state, &recipe_a.id, Some(cwd.clone())).unwrap();
    write_claude_transcript(&home(&state, CLAUDE_CODE_ID, &a.id), &cwd);
    let source = SessionId::new(launched.session_id.clone());
    assert!(state.supervisor.is_supervised(&source));

    let refused = commands::switch_execute_impl(
        &state,
        &launched.session_id,
        &recipe_b.id,
        "stop-now",
        None,
        &[],
    )
    .unwrap_err();
    assert!(refused.to_string().contains("secrets"), "{refused}");
    assert!(
        !state.supervisor.is_finished(&source),
        "the source keeps running"
    );
    let _ = state
        .supervisor
        .stop(&source, std::time::Duration::from_secs(2));
}

// --- AJ-E1511: a saved pack opens in any tool ---------------------------

use juggler_app::features::packs;

/// A pack exported from the source side of a switch out of `from`.
fn exported_pack(work: &Path, from: &str) -> (Setup, PathBuf) {
    let other = if from == CLAUDE_CODE_ID {
        CODEX_ID
    } else {
        CLAUDE_CODE_ID
    };
    let s = setup(work, from, other);
    if from == CLAUDE_CODE_ID {
        write_claude_transcript(&s.source_home, &s.cwd);
    } else {
        write_rollout(&s.source_home, &s.cwd);
    }
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
    let _ = args_of(&s.target_home);
    let epoch = s
        .state
        .storage
        .lock()
        .unwrap()
        .switch_graph()
        .edge(&SwitchEdgeId::new(info.edge_id.as_str()))
        .unwrap()
        .from_epoch
        .to_string();
    let path = PathBuf::from(commands::pack_export_impl(&s.state, &epoch).unwrap());
    (s, path)
}

/// A launch setting on a new account of `tool`, and that account's home.
fn recipe_for(s: &Setup, tool: &str) -> (String, PathBuf) {
    let account = commands::create_account(&s.state, tool, "Opener", None).unwrap();
    let recipe = commands::create_profile(&s.state, &account.id, "Main", None, None).unwrap();
    (recipe.id, home(&s.state, tool, &account.id))
}

#[test]
fn a_pack_opens_in_every_tool_primed_labelled_and_in_the_graph() {
    for (from, into, said) in [
        (CLAUDE_CODE_ID, CODEX_ID, "> Deploy with key"),
        (CLAUDE_CODE_ID, CLAUDE_CODE_ID, "> Deploy with key"),
        (CODEX_ID, CLAUDE_CODE_ID, "> Add a health check endpoint"),
        (CODEX_ID, CODEX_ID, "> Add a health check endpoint"),
    ] {
        let work = tempfile::tempdir().unwrap();
        let (s, pack) = exported_pack(work.path(), from);
        let preview = packs::preview_impl(&s.state, &pack).unwrap();
        assert_eq!(preview.from_account, "Source", "{from}");
        assert!(preview.turns >= 2, "{from}: {preview:?}");
        assert!(
            preview.carries[0].starts_with("the conversation"),
            "{preview:?}"
        );
        assert!(
            preview.fidelity.contains("Beta"),
            "never proven: {preview:?}"
        );
        assert_eq!(preview.project_folder.as_deref(), Some(s.cwd.as_str()));
        if from == CLAUDE_CODE_ID {
            assert_eq!(
                preview.withheld.len(),
                1,
                "the key was withheld: {preview:?}"
            );
            assert_eq!(preview.withheld[0].count, 1);
        }

        let (recipe, opener_home) = recipe_for(&s, into);
        let started =
            packs::start_impl(&s.state, &pack, &preview.digest, &recipe, Path::new(&s.cwd))
                .unwrap();
        let args = args_of(&opener_home);
        let brief = brief_of(&args);
        assert!(
            brief.contains(said),
            "{from} → {into}: the conversation travels:\n{brief}"
        );
        assert!(
            brief.contains("This session continues from a saved handover"),
            "{brief}"
        );
        assert!(!brief.contains(SECRET), "redacted when it was made");
        assert!(!args.join("\n").contains(said), "never on the command line");
        // Lineage: a pack edge with the pack's id and fidelity, confirmed,
        // its manifest recorded.
        let edge = s
            .state
            .storage
            .lock()
            .unwrap()
            .switch_graph()
            .edge(&SwitchEdgeId::new(started.edge_id.as_str()))
            .unwrap();
        assert_eq!(edge.mechanism, SwitchMechanism::Pack);
        assert!(edge.pack_id.is_some());
        assert_eq!(edge.fidelity_shown.as_deref(), Some("F1"));
        assert_eq!(edge.state, juggler_core::model::EdgeState::Confirmed);
        assert!(
            matches!(
                status_of(&s.state, &started.session_id),
                SessionStatus::Starting | SessionStatus::Running
            ),
            "{from} → {into}: a session started"
        );
    }
}

#[test]
fn a_pack_changed_after_the_preview_or_unsafe_starts_nothing() {
    let work = tempfile::tempdir().unwrap();
    let (s, pack) = exported_pack(work.path(), CLAUDE_CODE_ID);
    let preview = packs::preview_impl(&s.state, &pack).unwrap();
    let (recipe, opener_home) = recipe_for(&s, CODEX_ID);

    // Changed since the preview: its digest no longer matches.
    let changed = packs::start_impl(
        &s.state,
        &pack,
        "not-the-digest",
        &recipe,
        Path::new(&s.cwd),
    )
    .unwrap_err();
    assert!(
        changed.to_string().contains("changed since you looked"),
        "{changed}"
    );
    // A file edited by hand: refused before anything is written.
    std::fs::write(
        pack.join("conversation.md"),
        "# Conversation\n\n## User\n\nDo something else.\n",
    )
    .unwrap();
    let edited = packs::preview_impl(&s.state, &pack).unwrap_err();
    assert!(
        edited.to_string().contains("changed after it was made"),
        "{edited}"
    );
    let refused = packs::start_impl(&s.state, &pack, &preview.digest, &recipe, Path::new(&s.cwd))
        .unwrap_err();
    assert!(
        refused.to_string().contains("changed after it was made"),
        "{refused}"
    );
    // A folder that isn't a pack.
    let not = packs::preview_impl(&s.state, Path::new(&s.cwd)).unwrap_err();
    assert!(not.to_string().contains("isn't a saved handover"), "{not}");
    // Nothing launched, no handover written for it.
    assert!(!opener_home.join("last-args").exists());
    let switches = s.state.data_dir.join("switches");
    let written = std::fs::read_dir(&switches)
        .map(|dir| dir.count())
        .unwrap_or(0);
    assert_eq!(written, 1, "only the switch's own handover");
}
