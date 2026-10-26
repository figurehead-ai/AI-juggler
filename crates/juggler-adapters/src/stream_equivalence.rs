//! 0.1: a transcript read in bounded memory says exactly what reading it
//! whole said. Each reader that now streams is held to the whole-file code
//! it replaced, on hundreds of mixes of a rollout's lines: a resumed thread
//! (its records repeated), injected context, compaction, damaged and blank
//! lines, a subagent's rollout, branches set late.

use std::path::{Path, PathBuf};

use juggler_adapter::conversation::ConversationSummary;
use juggler_core::time::timestamp_ms;

use crate::claude_transcript::MessageRole;
use crate::codex_transcript::{parse_rollout, reference, rollout_cwd, rollout_thread_id};
use crate::quota_ingest::{RolloutCarry, rollout_events_after, tail_jsonl_by};

const THREAD: &str = "0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b";
const NAME: &str = "rollout-2026-10-21T10-00-00-0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b.jsonl";

/// The lines a mix is drawn from.
fn pool() -> Vec<String> {
    let at = |minute: u32| format!("2026-10-21T10:{minute:02}:00.000Z");
    let line = |minute: u32, kind: &str, payload: &str| {
        format!(
            r#"{{"timestamp":"{}","type":"{kind}","payload":{payload}}}"#,
            at(minute)
        )
    };
    vec![
        line(
            0,
            "session_meta",
            &format!(r#"{{"id":"{THREAD}","cwd":"/w/api","source":"cli"}}"#),
        ),
        line(
            1,
            "session_meta",
            &format!(r#"{{"id":"{THREAD}","cwd":"/w/web","git":{{"branch":"fix/x"}}}}"#),
        ),
        line(
            2,
            "session_meta",
            r#"{"id":"other","source":{"subagent":{"thread_spawn":{"parent_thread_id":"p"}}}}"#,
        ),
        line(3, "turn_context", r#"{"model":"gpt-5.5"}"#),
        line(
            4,
            "response_item",
            r#"{"type":"message","role":"user","content":[{"type":"input_text","text":"fix the flaky login test"}]}"#,
        ),
        line(
            5,
            "response_item",
            r#"{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>cwd</environment_context>"}]}"#,
        ),
        line(
            6,
            "response_item",
            r#"{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Looking at it now."}]}"#,
        ),
        line(
            7,
            "response_item",
            r#"{"type":"function_call","name":"shell","arguments":"{\"command\":[\"cargo\",\"test\"]}","call_id":"c1"}"#,
        ),
        line(
            8,
            "token_usage_record",
            r#"{"thread_id":"t","turn_id":"1","response_id":"r1","usage":{"input_tokens":100,"cached_input_tokens":40,"output_tokens":20,"reasoning_output_tokens":5}}"#,
        ),
        line(
            9,
            "token_usage_record",
            r#"{"thread_id":"t","turn_id":"2","response_id":"r2","usage":{"input_tokens":300,"cached_input_tokens":200,"output_tokens":30}}"#,
        ),
        line(
            10,
            "event_msg",
            r#"{"type":"token_count","info":{"total_token_usage":{"total_tokens":400},"last_token_usage":{"total_tokens":120}}}"#,
        ),
        line(11, "compacted", r#"{"message":"summary"}"#),
        line(
            12,
            "response_item",
            r#"{"type":"message","role":"user","content":"a second ask"}"#,
        ),
        // The Codex app's own block: in the conversation, never its title.
        line(
            13,
            "response_item",
            r#"{"type":"message","role":"user","content":[{"type":"input_text","text":"<external_codex_apps_operation>{}</external_codex_apps_operation>"}]}"#,
        ),
        "{ not json".to_owned(),
        String::new(),
        "   ".to_owned(),
    ]
}

/// A deterministic mix: `seed` picks up to 24 lines from the pool.
fn mix(seed: u64) -> Vec<u8> {
    let pool = pool();
    let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    let mut next = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 33
    };
    let count = 1 + next() % 24;
    let mut out = String::new();
    for _ in 0..count {
        out.push_str(&pool[usize::try_from(next()).unwrap() % pool.len()]);
        out.push('\n');
    }
    if next() % 3 == 0 {
        out.pop(); // no newline at the end
    }
    out.into_bytes()
}

/// The Codex conversation summary as it was read whole (with 0.1's rule
/// that a subagent's rollout is no conversation of its own).
fn whole_summary(bytes: &[u8], name: &str) -> Option<ConversationSummary> {
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return None;
    }
    let first_meta = String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|value| {
            value.get("type").and_then(serde_json::Value::as_str) == Some("session_meta")
        });
    if first_meta.is_some_and(|meta| meta.pointer("/payload/source/subagent").is_some()) {
        return None;
    }
    let parsed = reference::parse_rollout(bytes);
    let mut summary = ConversationSummary {
        id: rollout_thread_id(bytes, name)?,
        cwd: rollout_cwd(bytes).map(PathBuf::from),
        tokens: parsed.usage,
        title: parsed
            .messages
            .iter()
            .filter(|message| message.role == MessageRole::User)
            .find_map(|message| crate::codex_titles::title(&message.text)),
        ..ConversationSummary::default()
    };
    for line in String::from_utf8_lossy(bytes).lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(at) = value
            .get("timestamp")
            .and_then(serde_json::Value::as_str)
            .and_then(timestamp_ms)
        {
            summary.started_ms = Some(summary.started_ms.map_or(at, |first| first.min(at)));
            summary.last_ms = Some(summary.last_ms.map_or(at, |last| last.max(at)));
        }
        if value.get("type").and_then(serde_json::Value::as_str) == Some("session_meta")
            && let Some(branch) = value
                .pointer("/payload/git/branch")
                .and_then(serde_json::Value::as_str)
                .filter(|branch| !branch.is_empty())
        {
            summary.git_branch = Some(branch.to_owned());
        }
    }
    Some(summary)
}

fn written(dir: &Path, bytes: &[u8], compressed: bool) -> PathBuf {
    let path = dir.join(if compressed {
        format!("{NAME}.zst")
    } else {
        NAME.to_owned()
    });
    if compressed {
        std::fs::write(&path, zstd::encode_all(bytes, 3).unwrap()).unwrap();
    } else {
        std::fs::write(&path, bytes).unwrap();
    }
    path
}

#[test]
fn the_line_at_a_time_parser_is_the_whole_file_one() {
    for seed in 0..500 {
        let bytes = mix(seed);
        assert_eq!(
            parse_rollout(&bytes),
            reference::parse_rollout(&bytes),
            "mix {seed}"
        );
    }
}

#[test]
fn a_streamed_conversation_summary_is_the_whole_file_one() {
    let dir = tempfile::tempdir().unwrap();
    for seed in 0..500 {
        let bytes = mix(seed);
        let expected = whole_summary(&bytes, NAME);
        for compressed in [false, true] {
            let path = written(dir.path(), &bytes, compressed);
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let expected = if compressed {
                whole_summary(&bytes, &name)
            } else {
                expected.clone()
            };
            assert_eq!(
                crate::codex_conversations::read_conversation(&path),
                expected,
                "mix {seed}, compressed {compressed}"
            );
        }
    }
}

#[test]
fn usage_tailed_in_chunks_is_usage_read_at_once() {
    let dir = tempfile::tempdir().unwrap();
    for seed in 0..300 {
        let bytes = mix(seed);
        let path = written(dir.path(), &bytes, false);
        let parse = |bytes: &[u8], carry: RolloutCarry| rollout_events_after("a", bytes, carry);
        let whole = tail_jsonl_by(&path, 0, &RolloutCarry::default(), parse, u64::MAX >> 1);
        // Each chunk is longer than any one line, so none is skipped.
        for chunk in [400, 701, 1_500] {
            let chunked = tail_jsonl_by(&path, 0, &RolloutCarry::default(), parse, chunk);
            assert_eq!(chunked, whole, "mix {seed}, chunk {chunk}");
        }
    }
}
