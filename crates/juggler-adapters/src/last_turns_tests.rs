//! The backward scan: 0.3.4's `last_turns` tests, ported onto
//! [`scan_back`] and the one turn builder, and what 0.3.5 adds (a scan
//! never answers "nothing" before it has reached the file's start).

use std::path::{Path, PathBuf};

use super::*;
use crate::conversation_feed::{Block, FeedTurn, Lines, STEPS_MAX, Speaker};

pub(crate) fn claude(kind: &str, text: &str) -> String {
    claude_at(kind, text, "2026-10-24T10:00:00Z")
}

pub(crate) fn claude_at(kind: &str, text: &str, at: &str) -> String {
    serde_json::json!({
        "type": kind,
        "sessionId": "s",
        "timestamp": at,
        "message": {"role": kind, "content": [{"type": "text", "text": text}]},
    })
    .to_string()
}

fn tool_use(name: &str, input: &serde_json::Value) -> String {
    serde_json::json!({
        "type": "assistant",
        "timestamp": "2026-10-24T10:01:00Z",
        "message": {"role": "assistant", "model": "claude-opus-4-5", "content": [{"type": "tool_use", "id": "t", "name": name, "input": input}]},
    })
    .to_string()
}

fn tool_result() -> String {
    serde_json::json!({
        "type": "user",
        "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "t", "content": "ok"}]},
    })
    .to_string()
}

pub(crate) fn write(dir: &Path, name: &str, lines: &[String]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    path
}

fn page(path: &Path, before: Option<u64>, want: usize) -> BackPage {
    page_of(path, before, want, Lines::Session)
}

fn page_of(path: &Path, before: Option<u64>, want: usize, lines: Lines) -> BackPage {
    match scan_back(path, before, want, &TurnLog::new(lines)).unwrap() {
        Back::Page(page) => page,
        Back::Compressed => unreachable!("a plain file reads as turns"),
    }
}

pub(crate) fn texts(turns: &[FeedTurn]) -> Vec<(Speaker, String)> {
    turns.iter().map(|t| (t.speaker, t.text())).collect()
}

fn turns(page: &BackPage) -> Vec<FeedTurn> {
    page.log.turns().iter().cloned().collect()
}

/// A block's words, or its calls' lines.
fn shape(block: &Block) -> String {
    match block {
        Block::Text(text) => format!("text: {}", text.text()),
        Block::Narration(text) => format!("narration: {}", text.text()),
        Block::Steps(run) => format!(
            "steps: {}",
            run.steps
                .iter()
                .map(|step| step.line.as_str())
                .collect::<Vec<_>>()
                .join(" | ")
        ),
    }
}

#[test]
fn the_last_turns_of_a_claude_transcript_in_order_one_per_speaker() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        dir.path(),
        "s.jsonl",
        &[
            claude("user", "first ask"),
            claude("assistant", "first answer"),
            claude("user", "Fix the settings screen"),
            claude("assistant", "Reading it now."),
            tool_result(),
            claude("assistant", "Should I keep the old layout?"),
        ],
    );
    assert_eq!(
        texts(&turns(&page(&path, None, 3))),
        vec![
            (Speaker::Assistant, "first answer".to_owned()),
            (Speaker::You, "Fix the settings screen".to_owned()),
            (
                Speaker::Assistant,
                "Reading it now.\n\nShould I keep the old layout?".to_owned()
            ),
        ]
    );
}

#[test]
fn tool_calls_are_steps_in_order_with_the_turns_time_and_model() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        dir.path(),
        "s.jsonl",
        &[
            claude_at("user", "Run the tests", "2026-10-24T10:00:00Z"),
            claude_at("assistant", "Running them.", "2026-10-24T10:00:30Z"),
            tool_use("Bash", &serde_json::json!({"command": "pnpm test"})),
            tool_result(),
            tool_use("Read", &serde_json::json!({"file_path": "/w/app/src/a.ts"})),
            tool_result(),
            claude_at("assistant", "All green.", "2026-10-24T10:02:00Z"),
        ],
    );
    let page = page(&path, None, 30);
    assert_eq!(page.log.turns().len(), 2);
    assert_eq!(page.log.earlier(), None, "the whole file was read");
    let answer = &page.log.turns()[1];
    assert_eq!(
        answer.blocks.iter().map(shape).collect::<Vec<_>>(),
        vec![
            "text: Running them.".to_owned(),
            "steps: ran \"pnpm test\" | Read src/a.ts".to_owned(),
            "text: All green.".to_owned(),
        ]
    );
    assert_eq!(
        answer.at_ms,
        juggler_core::time::timestamp_ms("2026-10-24T10:02:00Z")
    );
    assert_eq!(answer.model.as_deref(), Some("claude-opus-4-5"));
    assert_eq!(
        page.log.turns()[0].at_ms,
        juggler_core::time::timestamp_ms("2026-10-24T10:00:00Z")
    );
}

#[test]
fn a_tools_own_lines_and_sub_agents_are_not_turns() {
    let dir = tempfile::tempdir().unwrap();
    let meta = serde_json::json!({
        "type": "user",
        "isMeta": true,
        "message": {"role": "user", "content": "Caveat: the messages below were generated by a local command"},
    })
    .to_string();
    let sidechain = serde_json::json!({
        "type": "assistant",
        "isSidechain": true,
        "message": {"role": "assistant", "content": [{"type": "text", "text": "sub-agent chatter"}]},
    })
    .to_string();
    let path = write(
        dir.path(),
        "s.jsonl",
        &[
            claude("user", "real ask"),
            meta,
            claude("user", "<command-name>/clear</command-name>"),
            sidechain,
            claude("assistant", "real answer"),
        ],
    );
    let page = page(&path, None, 3);
    assert_eq!(
        texts(&turns(&page)),
        vec![
            (Speaker::You, "real ask".to_owned()),
            (Speaker::Assistant, "real answer".to_owned()),
        ]
    );
    // The offsets are the lines' own: the answer starts at its line.
    let bytes = std::fs::read(&path).unwrap();
    let at = usize::try_from(page.log.turns()[1].offset).unwrap();
    let line = String::from_utf8_lossy(&bytes[at..]);
    assert!(line.lines().next().unwrap().contains("real answer"));
}

#[test]
fn a_subagents_own_transcript_is_its_conversation() {
    let dir = tempfile::tempdir().unwrap();
    let side = |kind: &str, text: &str| {
        serde_json::json!({
            "type": kind,
            "isSidechain": true,
            "timestamp": "2026-10-27T10:00:00Z",
            "message": {"role": kind, "content": [{"type": "text", "text": text}]},
        })
        .to_string()
    };
    let path = write(
        dir.path(),
        "agent-a1.jsonl",
        &[
            side("user", "Find the parser"),
            side("assistant", "Found it"),
        ],
    );
    // As a session's, every line is a subagent's: nothing to show, and
    // that is the whole file's answer.
    let session = page(&path, None, 5);
    assert!(session.log.turns().is_empty());
    assert_eq!(session.log.earlier(), None);
    assert_eq!(
        texts(&turns(&page_of(&path, None, 5, Lines::Subagent))),
        vec![
            (Speaker::You, "Find the parser".to_owned()),
            (Speaker::Assistant, "Found it".to_owned()),
        ]
    );
}

pub(crate) fn long_session(dir: &Path, turns: usize, filler: usize) -> PathBuf {
    let pad = "x".repeat(filler);
    let mut lines = Vec::new();
    for n in 0..turns {
        lines.push(claude("user", &format!("ask {n} {pad}")));
        lines.push(claude("assistant", &format!("answer {n} {pad}")));
    }
    write(dir, "s.jsonl", &lines)
}

#[test]
fn up_to_want_turns_with_a_cursor_to_page_back_and_no_seam() {
    let dir = tempfile::tempdir().unwrap();
    // 600 turns of ~1.2 KB: past twice the first window.
    let path = long_session(dir.path(), 300, 1_000);
    let len = std::fs::metadata(&path).unwrap().len();
    assert!(len > WINDOWS[0] * 2);
    let first = page(&path, None, 30);
    let shown = turns(&first);
    assert_eq!(shown.len(), 30);
    assert!(shown.last().unwrap().text().starts_with("answer 299"));
    assert!(shown[0].text().starts_with("ask 285"));
    let cursor = first.log.earlier().expect("more before");
    assert_eq!(cursor, shown[0].offset);
    // The page before ends where this one starts: nothing lost or
    // repeated at the seam.
    let before = page(&path, Some(cursor), 30);
    let earlier = turns(&before);
    assert_eq!(earlier.len(), 30);
    assert!(earlier.last().unwrap().text().starts_with("answer 284"));
    assert!(earlier[0].text().starts_with("ask 270"));
    assert!(earlier.last().unwrap().end <= cursor);
    assert!(before.log.earlier().unwrap() < cursor);
}

#[test]
fn a_window_grows_only_to_the_bound_and_never_reads_the_whole_file() {
    let dir = tempfile::tempdir().unwrap();
    // Turns of ~1 MB: 30 of them would need 30 MB.
    let path = long_session(dir.path(), 15, 1_000_000);
    let len = std::fs::metadata(&path).unwrap().len();
    assert!(len > MAX_WINDOW * 3 / 2);
    let read = page(&path, None, 30);
    let span = len - read.log.earlier().unwrap();
    assert!(span <= MAX_WINDOW, "read past the bound: {span}");
    let shown = turns(&read);
    assert!(!shown.is_empty() && shown.len() < 30);
    // A turn keeps its end.
    assert!(shown[0].text().starts_with('…'));
}

#[test]
fn only_the_end_is_read_and_a_turn_begun_before_the_window_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let mut lines = vec![claude("user", "long ago")];
    let filler = "x".repeat(10_000);
    for _ in 0..300 {
        lines.push(claude("assistant", &filler));
    }
    lines.push(claude("user", "the latest ask"));
    let path = write(dir.path(), "s.jsonl", &lines);
    let read = page(&path, None, 1);
    assert!(read.read < 2 * WINDOWS[0], "read {}", read.read);
    assert_eq!(
        texts(&turns(&read)),
        vec![(Speaker::You, "the latest ask".to_owned())]
    );
}

#[test]
fn reading_forward_from_a_pages_start_takes_the_new_turns() {
    let dir = tempfile::tempdir().unwrap();
    let path = long_session(dir.path(), 50, 10);
    let first = page(&path, None, 10);
    let from = first.log.turns()[0].offset;
    let mut more = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    std::io::Write::write_all(
        &mut more,
        format!(
            "{}\n{}\n",
            claude("user", "one more"),
            claude("assistant", "done")
        )
        .as_bytes(),
    )
    .unwrap();
    let Back::Page(next) = read_forward(&path, from, 10, &TurnLog::new(Lines::Session)).unwrap()
    else {
        unreachable!("plain")
    };
    let next = turns(&next);
    assert_eq!(next.len(), 12, "the page's turns and the new ones");
    assert_eq!(next[0], first.log.turns()[0]);
    assert_eq!(next.last().unwrap().text(), "done");
}

#[test]
fn the_stamp_changes_when_the_file_grows_and_not_otherwise() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "s.jsonl", &[claude("user", "hi")]);
    let one = stamp(&path).unwrap();
    assert_eq!(stamp(&path).unwrap(), one);
    let mut more = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    std::io::Write::write_all(
        &mut more,
        format!("{}\n", claude("assistant", "hello")).as_bytes(),
    )
    .unwrap();
    assert_ne!(stamp(&path).unwrap(), one);
}

#[test]
fn a_codex_rollout_reads_through_its_own_rules_calls_first() {
    let dir = tempfile::tempdir().unwrap();
    let line = |role: &str, kind: &str, text: &str, at: &str| {
        serde_json::json!({
            "timestamp": at,
            "type": "response_item",
            "payload": {"type": "message", "role": role, "content": [{"type": kind, "text": text}]},
        })
        .to_string()
    };
    let call = serde_json::json!({
        "timestamp": "2026-10-24T10:00:20Z",
        "type": "response_item",
        "payload": {"type": "function_call", "name": "shell", "arguments": "{\"command\":[\"pnpm\",\"test\"]}"},
    })
    .to_string();
    let path = write(
        dir.path(),
        "rollout-2026-10-24T10-00-00-019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a7b.jsonl",
        &[
            line(
                "user",
                "input_text",
                "<environment_context>cwd</environment_context>",
                "2026-10-24T10:00:00Z",
            ),
            line(
                "user",
                "input_text",
                "Add pagination",
                "2026-10-24T10:00:10Z",
            ),
            call,
            line(
                "assistant",
                "output_text",
                "Done: page and limit.",
                "2026-10-24T10:00:40Z",
            ),
        ],
    );
    let read = page(&path, None, 3);
    assert_eq!(read.log.layout(), crate::conversation_feed::Layout::Folded);
    let shown = turns(&read);
    assert_eq!(
        texts(&shown),
        vec![
            (Speaker::You, "Add pagination".to_owned()),
            (Speaker::Assistant, "Done: page and limit.".to_owned()),
        ]
    );
    assert!(
        matches!(shown[1].blocks[0], Block::Steps(_)),
        "{:?}",
        shown[1].blocks
    );
    assert_eq!(
        shown[1].at_ms,
        juggler_core::time::timestamp_ms("2026-10-24T10:00:40Z")
    );
    assert_eq!(
        shown[0].at_ms,
        juggler_core::time::timestamp_ms("2026-10-24T10:00:10Z")
    );
}

#[test]
fn a_long_run_of_calls_keeps_the_latest() {
    let dir = tempfile::tempdir().unwrap();
    let mut lines = vec![claude("user", "go")];
    for n in 0..(STEPS_MAX + 5) {
        lines.push(tool_use(
            "Bash",
            &serde_json::json!({"command": format!("step {n}")}),
        ));
        lines.push(tool_result());
    }
    let path = write(dir.path(), "s.jsonl", &lines);
    let read = page(&path, None, 3);
    let Block::Steps(run) = &read.log.turns()[1].blocks[0] else {
        unreachable!("the turn is its calls")
    };
    assert_eq!(run.steps.len(), STEPS_MAX);
    assert_eq!(run.hidden, 5);
    assert!(
        run.steps
            .last()
            .unwrap()
            .line
            .contains(&format!("step {}", STEPS_MAX + 4))
    );
}

#[test]
fn a_compressed_file_is_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rollout.jsonl.zst");
    std::fs::write(&path, b"not zstd").unwrap();
    assert!(matches!(
        scan_back(&path, None, 3, &TurnLog::new(Lines::Session)).unwrap(),
        Back::Compressed
    ));
}

fn notification(text: &str) -> String {
    serde_json::json!({
        "type": "user",
        "timestamp": "2026-10-27T10:00:00Z",
        "origin": {"kind": "task-notification"},
        "message": {"role": "user", "content": text},
    })
    .to_string()
}

/// 0.1.8: a Claude app session replying to its tasks' notifications has
/// no turn of yours in between; each reply is its own turn.
#[test]
fn a_notification_ends_the_reply_before_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        dir.path(),
        "s.jsonl",
        &[
            claude("assistant", "first reply"),
            notification("<task-notification>done</task-notification>"),
            claude("assistant", "second reply"),
        ],
    );
    let shown = turns(&page(&path, None, 6));
    assert_eq!(shown.len(), 2, "two replies, two turns");
    assert!(shown.iter().all(|turn| turn.speaker == Speaker::Assistant));
}

/// 0.1.8: a run of replies longer than the largest window shows its end.
#[test]
fn a_reply_run_longer_than_the_window_shows_its_end() {
    let dir = tempfile::tempdir().unwrap();
    let block = "x".repeat(1024 * 1024);
    let lines: Vec<String> = (0..20).map(|_| claude("assistant", &block)).collect();
    let path = write(dir.path(), "long.jsonl", &lines);
    assert!(std::fs::metadata(&path).unwrap().len() > MAX_WINDOW);
    let shown = turns(&page(&path, None, 6));
    assert_eq!(shown.len(), 1, "the run's end is one turn");
    assert_eq!(shown[0].speaker, Speaker::Assistant);
}

/// 0.3.5 (test plan 8): 20 MiB with no turn before the last prompt. The
/// open shows the prompt; reading on from where it stopped finds the
/// earlier turns; no scan answers "nothing" before the file's start.
#[test]
fn a_long_stretch_with_no_turn_is_read_past_never_answered_as_empty() {
    let dir = tempfile::tempdir().unwrap();
    let mut lines = Vec::new();
    for n in 0..5 {
        lines.push(claude("user", &format!("ask {n}")));
        lines.push(claude("assistant", &format!("answer {n}")));
    }
    let progress =
        serde_json::json!({"type": "progress", "data": {"text": "p".repeat(100_000)}}).to_string();
    for _ in 0..210 {
        lines.push(progress.clone());
    }
    lines.push(claude("user", "the last prompt"));
    let path = write(dir.path(), "s.jsonl", &lines);
    assert!(std::fs::metadata(&path).unwrap().len() > 20 * 1024 * 1024);
    let open = page(&path, None, 30);
    assert_eq!(
        texts(&turns(&open)),
        vec![(Speaker::You, "the last prompt".to_owned())]
    );
    let mut cursor = open.log.earlier().expect("more before");
    let mut calls = 0;
    let earlier = loop {
        calls += 1;
        let read = page(&path, Some(cursor), 30);
        if !read.log.turns().is_empty() {
            break turns(&read);
        }
        cursor = read
            .log
            .earlier()
            .expect("an empty read before the start says where it got to");
        assert!(calls < 3, "it reads on");
    };
    assert_eq!(calls, 1, "one scan passes 20 MiB of nothing");
    assert_eq!(earlier.len(), 10);
    assert_eq!(earlier.last().unwrap().text(), "answer 4");
}
