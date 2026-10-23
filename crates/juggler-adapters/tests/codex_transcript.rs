//! AJ-E1303: Codex rollouts into the neutral parsed form, format
//! detection, the session locator, the watcher and the identity probe.
//! Fixtures are synthetic rollouts in the 0.163.0 shape
//! (`tests/fixtures/codex/`); every `auth.json` is synthetic.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use juggler_adapter::{HarnessAdapter, IdentityProbe, SessionEvent};
use juggler_adapters::claude_transcript::MessageRole;
use juggler_adapters::codex_transcript::{
    parse_rollout, rollout_cwd, rollout_thread_id, thread_id_from_name,
};
use juggler_adapters::transcript::{TranscriptFormat, detect_format};
use juggler_adapters::{ClaudeCodeAdapter, CodexAdapter, parse_any};

const THREAD: &str = "019a1b2c-0000-7000-8000-00000000c0de";

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

#[test]
fn paginated_rollout_reads_the_conversation_without_injected_context() {
    let parsed = parse_rollout(&fixture("codex/rollout-paginated.jsonl"));
    assert_eq!(parsed.lines_malformed, 0);
    let turns: Vec<(MessageRole, &str)> = parsed
        .messages
        .iter()
        .map(|m| (m.role, m.text.as_str()))
        .collect();
    assert_eq!(
        turns,
        vec![
            (MessageRole::User, "Add a health check endpoint"),
            (MessageRole::Assistant, "Added GET /health returning 200."),
            (MessageRole::User, "Now add a test"),
            (MessageRole::Assistant, "Test added in tests/health.rs."),
        ],
        "developer and environment turns are context, not conversation"
    );
    // The model per assistant turn comes from the latest turn_context.
    assert_eq!(parsed.messages[1].model.as_deref(), Some("gpt-5-codex"));
    assert_eq!(parsed.messages[3].model.as_deref(), Some("gpt-5.1-codex"));
    assert!(
        parsed
            .messages
            .iter()
            .all(|m| m.session_id.as_deref() == Some(THREAD))
    );
    // Ids: the item id when present, else a stable line id.
    assert_eq!(parsed.messages[1].uuid.as_deref(), Some("msg_fixture_1"));
    assert!(
        parsed.messages[0]
            .uuid
            .as_deref()
            .unwrap()
            .starts_with(&format!("{THREAD}:line:"))
    );
    // Tokens: token_usage_record rows only (token_count is a rollup).
    // Fresh prompt only: input_tokens minus the cached (re-sent) part.
    assert_eq!(parsed.usage.input, (1200 - 800) + (1500 - 1200));
    assert_eq!(parsed.usage.cache_read, 2000);
    assert_eq!(parsed.usage.output, 150);
    assert_eq!(parsed.usage.reasoning, 40);
    // Compaction caps fidelity like Claude's compact boundary.
    assert!(parsed.has_compact_boundary());
}

#[test]
fn legacy_rollout_parses_and_counts_damage() {
    let parsed = parse_rollout(&fixture("codex/rollout-legacy.jsonl"));
    assert_eq!(parsed.lines_malformed, 1, "the torn line is counted");
    let texts: Vec<&str> = parsed.messages.iter().map(|m| m.text.as_str()).collect();
    // event_msg user_message/agent_message duplicate response_items in
    // legacy mode — only the response_items are read.
    assert_eq!(texts, vec!["Rename the crate", "Renamed."]);
    assert!(!parsed.has_compact_boundary());
}

#[test]
fn ids_and_cwd_come_from_session_meta_or_the_name() {
    let bytes = fixture("codex/rollout-paginated.jsonl");
    let name = format!("rollout-2026-10-20T08-00-00-{THREAD}.jsonl");
    assert_eq!(rollout_thread_id(&bytes, &name).as_deref(), Some(THREAD));
    assert_eq!(rollout_thread_id(b"", &name).as_deref(), Some(THREAD));
    assert_eq!(rollout_cwd(&bytes).as_deref(), Some("/work/app"));
    assert_eq!(
        thread_id_from_name(&format!("rollout-2026-10-20T08-00-00-{THREAD}_rev2.jsonl")).as_deref(),
        Some(THREAD),
        "a reverted thread keeps its id"
    );
    assert_eq!(
        thread_id_from_name(&format!("rollout-2026-10-20T08-00-00-{THREAD}.jsonl.zst")).as_deref(),
        Some(THREAD)
    );
    assert_eq!(thread_id_from_name("session.jsonl"), None);
    assert_eq!(thread_id_from_name("rollout-short.jsonl"), None);
}

#[test]
fn any_transcript_parses_by_its_own_format() {
    let codex = fixture("codex/rollout-paginated.jsonl");
    let claude = fixture("claude-code/session-basic.jsonl");
    assert_eq!(detect_format(&codex), TranscriptFormat::CodexRollout);
    assert_eq!(detect_format(&claude), TranscriptFormat::ClaudeCode);
    assert_eq!(detect_format(b""), TranscriptFormat::ClaudeCode);
    assert_eq!(parse_any(&codex).messages.len(), 4);
    assert_eq!(
        parse_any(&claude),
        juggler_adapters::parse_transcript(&claude),
        "Claude parsing is unchanged"
    );
}

#[test]
fn garbage_never_panics() {
    for bytes in [
        &b"\n\n{"[..],
        b"{\"type\":\"response_item\",\"payload\":7,\"timestamp\":\"x\"}",
        b"{\"type\":\"response_item\",\"timestamp\":\"x\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":42}}",
        b"\xff\xfe\x00",
    ] {
        let _ = parse_rollout(bytes);
        let _ = parse_any(bytes);
    }
}

fn now_ms() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

fn write_rollout(home: &Path, thread: &str, cwd: &str) -> PathBuf {
    let day = home.join("sessions/2026/10/20");
    std::fs::create_dir_all(&day).unwrap();
    let path = day.join(format!("rollout-2026-10-20T08-00-00-{thread}.jsonl"));
    std::fs::write(
        &path,
        format!(
            "{{\"timestamp\":\"2026-10-20T08:00:00.000Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{thread}\",\"cwd\":\"{cwd}\"}}}}\n"
        ),
    )
    .unwrap();
    path
}

#[test]
fn a_launched_codex_session_finds_its_rollout_only_when_it_is_the_one() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let adapter = CodexAdapter::new();
    // Older session in the same folder, well before the launch (beyond
    // the 2 s slack even with whole-second timestamps): not a candidate.
    write_rollout(home, "019a0000-0000-7000-8000-000000000001", "/work/app");
    std::thread::sleep(Duration::from_millis(3_100));
    let since = now_ms();
    write_rollout(home, "019a0000-0000-7000-8000-000000000002", "/work/other");
    let ours = write_rollout(home, "019a0000-0000-7000-8000-000000000003", "/work/app");

    let found = adapter
        .locate_transcript(home, Path::new("/work/app"), since, &[])
        .expect("the one match");
    assert_eq!(found.path, ours);
    assert_eq!(
        found.native_id.as_deref(),
        Some("019a0000-0000-7000-8000-000000000003")
    );
    assert!(
        adapter
            .locate_transcript(home, Path::new("/nowhere"), since, &[])
            .is_none()
    );

    // A second session's rollout in the same folder: two matches are
    // never guessed between …
    let theirs = write_rollout(home, "019a0000-0000-7000-8000-000000000004", "/work/app");
    assert!(
        adapter
            .locate_transcript(home, Path::new("/work/app"), since, &[])
            .is_none()
    );
    // … unless the other is already linked to its own session.
    let found = adapter
        .locate_transcript(home, Path::new("/work/app"), since, &[theirs])
        .expect("the other one is taken");
    assert_eq!(found.path, ours);
}

#[test]
fn a_huge_first_line_still_carries_the_cwd() {
    let tmp = tempfile::tempdir().unwrap();
    let since = now_ms();
    let day = tmp.path().join("sessions/2026/10/20");
    std::fs::create_dir_all(&day).unwrap();
    let path = day.join(format!("rollout-2026-10-20T08-00-00-{THREAD}.jsonl"));
    // session_meta carries the base instructions: one line, megabytes.
    let instructions = "x".repeat(3 * 1024 * 1024);
    std::fs::write(
        &path,
        format!(
            "{{\"timestamp\":\"t\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{THREAD}\",\"cwd\":\"/work/app\",\"base_instructions\":\"{instructions}\"}}}}\n"
        ),
    )
    .unwrap();
    assert!(
        CodexAdapter::new()
            .locate_transcript(tmp.path(), Path::new("/work/app"), since, &[])
            .is_some()
    );
}

#[cfg(unix)]
#[test]
fn the_folder_matches_through_a_symlink() {
    let tmp = tempfile::tempdir().unwrap();
    let real = tmp.path().join("real");
    std::fs::create_dir_all(&real).unwrap();
    let link = tmp.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let since = now_ms();
    let home = tmp.path().join("home");
    write_rollout(&home, THREAD, &real.to_string_lossy());
    assert!(
        CodexAdapter::new()
            .locate_transcript(&home, &link, since, &[])
            .is_some(),
        "the session's recorded folder and the launch folder are the same place"
    );
}

#[test]
fn a_repeated_usage_row_counts_once() {
    let row = format!(
        "{{\"timestamp\":\"t\",\"type\":\"token_usage_record\",\"payload\":{{\"thread_id\":\"{THREAD}\",\"turn_id\":\"t1\",\"response_id\":\"r1\",\"usage\":{{\"input_tokens\":100,\"cached_input_tokens\":40,\"output_tokens\":10}}}}}}\n"
    );
    let parsed = parse_rollout(format!("{row}{row}").as_bytes());
    assert_eq!(parsed.usage.input, 60);
    assert_eq!(parsed.usage.cache_read, 40);
    assert_eq!(parsed.usage.output, 10);
}

#[test]
fn ids_never_collide_across_rollouts() {
    let one = |thread: &str| {
        format!(
            "{{\"timestamp\":\"t\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{thread}\"}}}}\n{{\"timestamp\":\"t\",\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"hi\"}}]}}}}\n"
        )
    };
    let a = parse_rollout(one("019a0000-0000-7000-8000-00000000000a").as_bytes());
    let b = parse_rollout(one("019a0000-0000-7000-8000-00000000000b").as_bytes());
    assert_ne!(a.messages[0].uuid, b.messages[0].uuid);
}

#[test]
fn a_launched_claude_session_finds_its_transcript_too() {
    let tmp = tempfile::tempdir().unwrap();
    let since = now_ms();
    let project = tmp.path().join("projects/-work-app");
    std::fs::create_dir_all(&project).unwrap();
    let path = project.join("abc-123.jsonl");
    std::fs::write(
        &path,
        "{\"type\":\"user\",\"cwd\":\"/work/app\",\"sessionId\":\"abc-123\"}\n",
    )
    .unwrap();
    let found = ClaudeCodeAdapter::new()
        .locate_transcript(tmp.path(), Path::new("/work/app"), since, &[])
        .expect("found");
    assert_eq!(found.path, path);
    assert_eq!(found.native_id.as_deref(), Some("abc-123"));
}

#[test]
fn the_watcher_reports_thread_ids() {
    let tmp = tempfile::tempdir().unwrap();
    let sessions = tmp.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let mut watcher = CodexAdapter::new().watcher(&sessions).unwrap();
    let path = write_rollout(tmp.path(), THREAD, "/work/app");
    let mut event = None;
    for _ in 0..20 {
        event = watcher.poll(Duration::from_millis(100)).unwrap();
        if event.is_some() {
            break;
        }
    }
    assert_eq!(
        event,
        Some(SessionEvent::Discovered {
            native_id: THREAD.to_owned(),
            transcript_path: path,
        })
    );
}

#[test]
fn identity_is_the_account_id_masked_never_a_token() {
    let tmp = tempfile::tempdir().unwrap();
    let probe = |auth: Option<&str>| {
        let dir = tmp.path().join(format!("h{}", auth.map_or(0, str::len)));
        std::fs::create_dir_all(&dir).unwrap();
        if let Some(auth) = auth {
            std::fs::write(dir.join("auth.json"), auth).unwrap();
        }
        juggler_adapters::codex::probe_codex_identity(&dir)
    };
    assert_eq!(
        probe(Some(
            r#"{"auth_mode":"chatgpt","tokens":{"id_token":"fixture.id.token","refresh_token":"fixture","account_id":"acct-fixture-9f2c"}}"#
        )),
        IdentityProbe::SignedIn {
            subject: "acct-fixture-9f2c".into(),
            label: "ChatGPT account …9f2c".into(),
        }
    );
    assert!(matches!(
        probe(Some(
            r#"{"auth_mode":"apikey","OPENAI_API_KEY":"sk-fixture"}"#
        )),
        IdentityProbe::Unverifiable { .. }
    ));
    assert!(matches!(
        probe(Some(
            r#"{"auth_mode":"chatgpt","tokens":{"refresh_token":"fixture"}}"#
        )),
        IdentityProbe::Unverifiable { .. }
    ));
    assert_eq!(probe(None), IdentityProbe::SignedOut);
}

/// AJ-E1520: `spawn_agent` calls, named by their output, ended by a
/// `wait_agent` status or `close_agent`; a spawn that gave no agent failed.
#[test]
fn subagents_are_read_from_spawn_wait_and_close() {
    use juggler_adapters::{SubtaskState, subtasks_of};
    let log = subtasks_of(&fixture("codex/rollout-subagents.jsonl"));
    let seen: Vec<(&str, &str, Option<&str>, SubtaskState)> = log
        .subtasks()
        .iter()
        .map(|item| {
            (
                item.id.as_str(),
                item.label.as_str(),
                item.kind.as_deref(),
                item.state,
            )
        })
        .collect();
    assert_eq!(
        seen,
        vec![
            (
                "call_a",
                "Map the parsers",
                Some("explorer"),
                SubtaskState::Done
            ),
            (
                "call_b",
                "Review error handling",
                Some("worker"),
                SubtaskState::Done
            ),
            ("call_c", "Run the slow suite", None, SubtaskState::Failed),
            // A close that answered with an error string changes nothing.
            (
                "call_d",
                "Write tests for the tail reader",
                Some("worker"),
                SubtaskState::Running
            ),
            // Errored, then closed: it stays failed.
            (
                "call_f",
                "Profile the parser",
                Some("worker"),
                SubtaskState::Failed
            ),
        ]
    );
    assert_eq!(
        log.running(),
        1,
        "a shell call's output is never a subagent"
    );
}
