//! rc18 follow-up: a tool with no state reports says its own end of turn
//! through its adapter (`ToolLive::last_turn`), so a session idle at its
//! prompt after its turn is never "probably stuck", however quiet; one
//! quiet mid-turn still is. Codex in a terminal (its rollout's
//! `task_complete`) and Grok Build (its update stream's
//! `turn_completed`), from synthetic files. OpenCode's is in
//! `opencode_provider.rs`; Kimi Code's in its event-log tests.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic,
    reason = "test code"
)]

use std::path::{Path, PathBuf};

use juggler_adapter::ToolLive;
use juggler_adapter::live::TailTurn;
use juggler_adapters::{CodexAdapter, GrokBuildAdapter};
use juggler_core::model::{SessionId, SessionStatus};
use juggler_supervisor::{HeuristicEngine, HeuristicInputs, HeuristicThresholds};

/// What the heuristic makes of a session linked to `transcript`, quiet
/// for an hour with an idle CPU and a transcript as old, over two
/// evaluations past the debounce.
fn judged(adapter: &dyn ToolLive, transcript: &Path, session: &str) -> Option<SessionStatus> {
    let engine = HeuristicEngine::new(HeuristicThresholds::default());
    let now = 10_000_000_i64;
    let inputs = HeuristicInputs {
        last_output_ms: Some(now - 3_600_000),
        cpu_pct: Some(0.1),
        transcript_mtime_ms: Some(now - 3_600_000),
        prompt_alive: false,
        turn_ended: adapter.last_turn(transcript) == Some(TailTurn::Ended),
        now_ms: now,
    };
    let id = SessionId::new(session);
    let first = engine.evaluate(&id, &inputs);
    let second = engine.evaluate(
        &id,
        &HeuristicInputs {
            now_ms: now + 31_000,
            ..inputs
        },
    );
    first.or(second).map(|transition| transition.status)
}

fn write(path: &Path, lines: &[&str]) -> PathBuf {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, lines.join("\n") + "\n").unwrap();
    path.to_path_buf()
}

#[test]
fn a_finished_codex_turn_is_idle_never_probably_stuck() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = CodexAdapter::with_bin(dir.path().join("codex"));
    let started = r#"{"timestamp":"2026-11-09T10:00:00Z","type":"event_msg","payload":{"type":"task_started"}}"#;
    let message = r#"{"timestamp":"2026-11-09T10:00:05Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[]}}"#;
    let done = r#"{"timestamp":"2026-11-09T10:00:06Z","type":"event_msg","payload":{"type":"task_complete"}}"#;
    let call = r#"{"timestamp":"2026-11-09T10:00:05Z","type":"response_item","payload":{"type":"function_call","name":"shell","call_id":"c1"}}"#;
    let idle = write(
        &dir.path().join("rollout-idle.jsonl"),
        &[started, message, done],
    );
    let stalled = write(&dir.path().join("rollout-stalled.jsonl"), &[started, call]);
    assert_eq!(
        judged(&adapter, &idle, "codex-idle"),
        None,
        "a finished turn waits for you"
    );
    assert_eq!(
        judged(&adapter, &stalled, "codex-stalled"),
        Some(SessionStatus::Stuck),
        "quiet mid-turn is still probably stuck"
    );
}

#[test]
fn a_finished_grok_turn_is_idle_never_probably_stuck() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = GrokBuildAdapter::with_bin(dir.path().join("grok"));
    let line = |method: &str, kind: &str| {
        format!(
            r#"{{"timestamp":1794238678,"method":"{method}","params":{{"sessionId":"s","update":{{"sessionUpdate":"{kind}"}}}}}}"#
        )
    };
    let prompt = line("session/update", "user_message_chunk");
    let tool = line("session/update", "tool_call");
    let done = line("_x.ai/session/update", "turn_completed");
    let session = |id: &str, lines: &[&str]| {
        let folder = dir.path().join("sessions/%2Fw").join(id);
        write(&folder.join("updates.jsonl"), lines);
        write(&folder.join("summary.json"), &[r#"{"title":"x"}"#])
    };
    let idle = session("s-idle", &[&prompt, &tool, &done]);
    let stalled = session("s-stalled", &[&prompt, &tool]);
    assert_eq!(
        judged(&adapter, &idle, "grok-idle"),
        None,
        "a finished turn waits for you"
    );
    assert_eq!(
        judged(&adapter, &stalled, "grok-stalled"),
        Some(SessionStatus::Stuck),
        "quiet mid-turn is still probably stuck"
    );
}
