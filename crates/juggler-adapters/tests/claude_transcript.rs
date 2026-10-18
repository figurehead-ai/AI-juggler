//! Claude transcript parser golden tests (M1-5): fixture corpus, malformed
//! tolerance, resume dedupe, and the notify-based watcher.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::time::{Duration, Instant};

use juggler_adapter::{HarnessAdapter, SessionEvent, SessionWatcher};
use juggler_adapters::ClaudeCodeAdapter;
use juggler_adapters::claude_transcript::{MessageRole, dedupe_messages, parse_transcript};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/claude-code");

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(Path::new(FIXTURES).join(name)).unwrap()
}

fn wait_for_event(
    watcher: &mut dyn SessionWatcher,
    predicate: impl Fn(&SessionEvent) -> bool,
) -> Option<SessionEvent> {
    // Watcher poll blocks up to 2s per call; overall deadline 8s for CI
    // variance in fs-event delivery.
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if let Ok(Some(event)) = watcher.poll(Duration::from_secs(2))
            && predicate(&event)
        {
            return Some(event);
        }
    }
    None
}

#[test]
fn basic_fixture_parses_with_full_line_coverage() {
    let parsed = parse_transcript(&fixture("session-basic.jsonl"));
    // 5 conversation lines + 1 summary = 6 non-empty lines.
    assert_eq!(parsed.lines_total, 6);
    assert_eq!(parsed.lines_malformed, 0);
    assert_eq!(parsed.kinds.get("user").copied(), Some(2));
    assert_eq!(parsed.kinds.get("assistant").copied(), Some(3));
    assert_eq!(parsed.kinds.get("summary").copied(), Some(1));
    // Full line coverage: every line is accounted for as a kind or malformed.
    let accounted: usize = parsed.kinds.values().sum();
    assert_eq!(accounted + parsed.lines_malformed, parsed.lines_total);

    assert_eq!(parsed.messages.len(), 5);
    assert_eq!(parsed.messages[0].role, MessageRole::User);
    assert_eq!(parsed.messages[0].text, "Fix the flaky login test");
    assert_eq!(parsed.messages[1].text, "I'll reproduce the failure first.");
    // Tool-use line carries usage but no text.
    assert_eq!(parsed.messages[2].text, "");
    assert_eq!(parsed.messages[2].usage.unwrap().output, 77);
    // Tool-result user line has no plain text.
    assert_eq!(parsed.messages[3].text, "");
    assert!(parsed.messages[0].parent_uuid.is_none());
    assert_eq!(parsed.messages[1].parent_uuid.as_deref(), Some("m-0001"));
    assert_eq!(
        parsed.messages[0].session_id.as_deref(),
        Some("7f0c1a55-0001")
    );

    // Usage totals: 1214+1440+2210 input, 42+77+150 output, 1214+1440 read,
    // 226 cache creation.
    assert_eq!(parsed.usage.input, 4864);
    assert_eq!(parsed.usage.output, 269);
    assert_eq!(parsed.usage.cache_read, 2654);
    assert_eq!(parsed.usage.cache_write, 226);
}

#[test]
fn subagent_fixture_marks_sidechain() {
    let parsed = parse_transcript(&fixture("session-subagent.jsonl"));
    assert_eq!(parsed.messages.len(), 5);
    assert!(parsed.messages[2].sidechain);
    assert!(parsed.messages[3].sidechain);
    assert!(!parsed.messages[0].sidechain);
    assert!(!parsed.messages[4].sidechain);
    let accounted: usize = parsed.kinds.values().sum();
    assert_eq!(accounted + parsed.lines_malformed, parsed.lines_total);
}

#[test]
fn resumed_fixture_dedupes_repeated_history() {
    let parsed = parse_transcript(&fixture("session-resumed.jsonl"));
    assert_eq!(parsed.messages.len(), 5); // r1, r2 duplicated on resume
    let deduped = dedupe_messages(&parsed.messages);
    assert_eq!(deduped.len(), 3);
    assert_eq!(deduped[0].uuid.as_deref(), Some("r-0001"));
    assert_eq!(deduped[2].uuid.as_deref(), Some("r-0003"));
    // Order preserved.
    assert_eq!(deduped[2].text, "continue after resume");
}

#[test]
fn malformed_lines_skip_and_count_never_crash() {
    let parsed = parse_transcript(&fixture("session-malformed.jsonl"));
    // Lines: valid, garbage, {"type":"user"} (no message body), valid,
    // truncated json, system line → 6 non-empty.
    assert_eq!(parsed.lines_total, 6);
    assert_eq!(parsed.lines_malformed, 2); // garbage + truncated
    // type:user without a message object contributes to kinds but yields no
    // message; the two complete lines yield messages.
    assert_eq!(parsed.messages.len(), 2);
    assert_eq!(parsed.kinds.get("system").copied(), Some(1));
    let accounted: usize = parsed.kinds.values().sum();
    assert_eq!(accounted + parsed.lines_malformed, parsed.lines_total);

    // Fuzz-style robustness: arbitrary bytes never panic.
    let _ = parse_transcript(&[]);
    let _ = parse_transcript(&[0xff_u8; 4096]);
    let _ = parse_transcript(b"{\"type\":\"user\",\"message\":null}\n\n\n");
}

#[test]
fn messages_without_uuid_survive_dedupe() {
    let parsed = parse_transcript(b"{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"a\"}}\n{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"b\"}}\n");
    assert_eq!(parsed.messages.len(), 2);
    assert!(parsed.messages.iter().all(|m| m.uuid.is_none()));
    assert_eq!(dedupe_messages(&parsed.messages).len(), 2);
}

#[test]
fn notify_watcher_emits_discovered_then_appended() {
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    let project_dir = projects.join("Users-dev-proj");
    std::fs::create_dir_all(&project_dir).unwrap();

    let adapter = ClaudeCodeAdapter::new();
    let mut watcher = adapter.watcher(&projects).unwrap();

    // File appears → Discovered.
    let transcript = project_dir.join("7f0c1a55-0001.jsonl");
    std::fs::write(&transcript, String::new() + "{\"type\":\"user\",\"uuid\":\"m-1\",\"message\":{\"role\":\"user\",\"content\":\"hi\"}}\n").unwrap();
    let discovered = wait_for_event(
        &mut *watcher,
        |event| matches!(event, SessionEvent::Discovered { native_id, .. } if native_id == "7f0c1a55-0001"),
    );
    assert!(
        discovered.is_some(),
        "watcher should discover the new session file"
    );
    let SessionEvent::Discovered {
        transcript_path, ..
    } = discovered.unwrap()
    else {
        unreachable!()
    };
    assert!(transcript_path.ends_with("7f0c1a55-0001.jsonl"));

    // Growth → MessageAppended (drain any pending first).
    while watcher.poll(Duration::from_millis(10)).unwrap().is_some() {}
    std::fs::write(
        &transcript,
        std::fs::read_to_string(&transcript).unwrap()
            + "{\"type\":\"assistant\",\"uuid\":\"m-2\",\"message\":{\"role\":\"assistant\",\"content\":\"hello\"}}\n",
    )
    .unwrap();
    let appended = wait_for_event(
        &mut *watcher,
        |event| matches!(event, SessionEvent::MessageAppended { native_id, .. } if native_id == "7f0c1a55-0001"),
    );
    assert!(
        appended.is_some(),
        "watcher should report transcript growth"
    );
}

// AJ-E412: the per-turn model observation — ParsedMessage.model from
// top-level `model` or nested `message.model`, across the four fixture
// shapes (launch-only, /model change, per-turn mixed, no-model).
mod running_model_fixtures {
    use juggler_adapters::claude_transcript::parse_transcript;

    fn fixture(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read(&path).unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    fn assistant_models(bytes: &[u8]) -> Vec<Option<String>> {
        parse_transcript(bytes)
            .messages
            .into_iter()
            .filter(|message| {
                message.role == juggler_adapters::claude_transcript::MessageRole::Assistant
            })
            .map(|message| message.model)
            .collect()
    }

    #[test]
    fn launch_only_transcript_records_no_model() {
        let models = assistant_models(&fixture("model_launch_only.jsonl"));
        assert!(models.iter().all(Option::is_none));
    }

    #[test]
    fn model_change_mid_transcript_is_observed_per_turn() {
        let models = assistant_models(&fixture("model_change_mid_transcript.jsonl"));
        assert_eq!(
            models,
            vec![
                Some("opus-4.8".into()),
                Some("sonnet-5".into()),
                Some("sonnet-5".into())
            ]
        );
    }

    #[test]
    fn per_turn_mixed_session_reads_top_level_and_nested_shapes() {
        let models = assistant_models(&fixture("model_per_turn_mixed.jsonl"));
        // The fourth line carries the model NESTED under message.model —
        // both shapes ship in the wild, both must read.
        assert_eq!(
            models,
            vec![
                Some("opus-4.8".into()),
                None,
                Some("sonnet-5".into()),
                Some("opus-4.8".into())
            ]
        );
    }

    #[test]
    fn no_model_provider_records_none_everywhere() {
        let models = assistant_models(&fixture("model_none.jsonl"));
        assert!(!models.is_empty());
        assert!(models.iter().all(Option::is_none));
    }
}
