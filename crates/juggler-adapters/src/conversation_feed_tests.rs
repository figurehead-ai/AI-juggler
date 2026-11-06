//! The feed: the 0.3.4 repro's scenarios (a long turn emptied the
//! card) as regression tests, and the builder's rules (ids, growth,
//! chunking, bounds, the Claude app's stretches).

use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::*;
use crate::last_turns::{Back, scan_back};
use crate::transcript_tail::{LineHold, TailLog as _, TranscriptTail, cut_lines};

const AT: &str = "2026-10-24T10:00:00Z";

fn text(kind: &str, words: &str) -> String {
    serde_json::json!({
        "type": kind, "sessionId": "s", "timestamp": AT,
        "message": {"role": kind, "content": [{"type": "text", "text": words}]},
    })
    .to_string()
}

fn tool_use(n: usize) -> String {
    serde_json::json!({
        "type": "assistant", "timestamp": AT,
        "message": {"role": "assistant", "model": "m", "content": [{"type": "tool_use", "id": format!("t{n}"), "name": "Bash", "input": {"command": format!("make step-{n}")}}]},
    })
    .to_string()
}

fn tool_result(n: usize, size: usize) -> String {
    serde_json::json!({
        "type": "user", "timestamp": AT,
        "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": format!("t{n}"), "content": "o".repeat(size)}]},
    })
    .to_string()
}

fn sidechain(kind: &str, size: usize) -> String {
    serde_json::json!({
        "type": kind, "isSidechain": true, "timestamp": AT,
        "message": {"role": kind, "content": [{"type": "text", "text": "s".repeat(size)}]},
    })
    .to_string()
}

fn progress(size: usize) -> String {
    serde_json::json!({
        "type": "progress", "timestamp": AT, "parentToolUseID": "t0",
        "data": {"type": "agent_progress", "message": {"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": "p".repeat(size)}]}}},
    })
    .to_string()
}

fn notification(words: &str) -> String {
    serde_json::json!({
        "type": "user", "timestamp": AT, "origin": {"kind": "task-notification"},
        "message": {"role": "user", "content": words},
    })
    .to_string()
}

/// The repro's base: 12 questions and answers, a prompt, the start of a
/// reply and its first call (26 turns; the last is open).
fn base(path: &Path) {
    let mut lines = Vec::new();
    for n in 0..12 {
        lines.push(text("user", &format!("question {n}")));
        lines.push(text("assistant", &format!("answer {n}")));
    }
    lines.push(text("user", "please refactor the parser"));
    lines.push(text("assistant", "Starting on it."));
    lines.push(tool_use(0));
    std::fs::write(path, lines.join("\n") + "\n").unwrap();
}

fn append(path: &Path, bytes: &[u8]) {
    let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(bytes).unwrap();
}

fn append_lines(path: &Path, lines: &[String]) {
    append(path, (lines.join("\n") + "\n").as_bytes());
}

/// The page opening: the last 30 turns, then followed from where the
/// scan's whole lines end.
fn open(path: &Path) -> TranscriptTail<TurnLog> {
    let Back::Page(page) = scan_back(path, None, 30, &TurnLog::new(Lines::Session)).unwrap() else {
        unreachable!("plain")
    };
    TranscriptTail::resume(page.log, page.end, Some(page.file))
}

fn tick(tail: &mut TranscriptTail<TurnLog>, path: &Path) -> TurnDelta {
    tail.read_within(path, 16 * 1024 * 1024).unwrap();
    tail.log_mut().take_delta()
}

fn ids(tail: &TranscriptTail<TurnLog>) -> Vec<u64> {
    tail.log().turns().iter().map(|turn| turn.offset).collect()
}

struct Scenario {
    _dir: tempfile::TempDir,
    path: PathBuf,
    tail: TranscriptTail<TurnLog>,
    ids: Vec<u64>,
}

fn scenario() -> Scenario {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.jsonl");
    base(&path);
    let tail = open(&path);
    let ids = ids(&tail);
    assert_eq!(ids.len(), 26);
    assert!(tail.log().last_open());
    Scenario {
        _dir: dir,
        path,
        tail,
        ids,
    }
}

/// After growth: every turn shown before is still there under its id,
/// the open turn too, and the list is never empty.
fn kept(s: &Scenario) {
    let now = ids(&s.tail);
    assert!(!now.is_empty(), "never an empty conversation");
    assert_eq!(
        &now[..s.ids.len()],
        s.ids.as_slice(),
        "the earlier turns stay"
    );
}

fn steps_of(turn: &FeedTurn) -> (usize, usize) {
    turn.blocks
        .iter()
        .filter_map(|block| match block {
            Block::Steps(run) => Some((run.steps.len(), run.hidden)),
            _ => None,
        })
        .fold((0, 0), |(a, b), (c, d)| (a + c, b + d))
}

#[test]
fn repro_a_a_small_agentic_growth_extends_the_open_turn() {
    let mut s = scenario();
    let lines: Vec<String> = (1..40)
        .flat_map(|n| [tool_result(n - 1, 2_000), tool_use(n)])
        .collect();
    append_lines(&s.path, &lines);
    let delta = tick(&mut s.tail, &s.path);
    kept(&s);
    assert_eq!(delta.upserts.len(), 1, "only the open turn changed");
    assert_eq!(delta.upserts[0].offset, *s.ids.last().unwrap());
    assert_eq!(steps_of(&delta.upserts[0]), (40, 0));
}

#[test]
fn repro_b_big_results_and_calls_past_two_mib_keep_every_turn() {
    let mut s = scenario();
    let lines: Vec<String> = (1..=50)
        .flat_map(|n| [tool_result(n - 1, 50_000), tool_use(n)])
        .collect();
    append_lines(&s.path, &lines);
    assert!(std::fs::metadata(&s.path).unwrap().len() > 2 * 1024 * 1024);
    let delta = tick(&mut s.tail, &s.path);
    kept(&s);
    assert_eq!(ids(&s.tail).len(), 26);
    assert_eq!(delta.upserts.len(), 1);
    assert_eq!(steps_of(&delta.upserts[0]), (40, 11), "the latest calls");
    assert_eq!(delta.upserts[0].text(), "Starting on it.");
}

#[test]
fn repro_c_d_subagent_and_progress_lines_change_nothing() {
    for lines in [
        (0..250)
            .map(|n| sidechain(if n % 2 == 0 { "assistant" } else { "user" }, 10_000))
            .collect::<Vec<_>>(),
        (0..250).map(|_| progress(10_000)).collect(),
    ] {
        let mut s = scenario();
        append_lines(&s.path, &lines);
        let delta = tick(&mut s.tail, &s.path);
        kept(&s);
        assert!(delta.is_empty(), "nothing for the page: {delta:?}");
        assert_eq!(ids(&s.tail), s.ids);
        // Opening the page now still shows the conversation.
        let reopened = open(&s.path);
        assert_eq!(ids(&reopened), s.ids);
    }
}

#[test]
fn repro_e_one_result_larger_than_the_old_window() {
    let mut s = scenario();
    append_lines(&s.path, &[tool_result(0, 3 * 1024 * 1024)]);
    let delta = tick(&mut s.tail, &s.path);
    kept(&s);
    assert_eq!(ids(&s.tail), s.ids);
    // The call's result came: its turn is said again, same id.
    assert_eq!(delta.upserts.len(), 1);
    assert_eq!(delta.upserts[0].offset, *s.ids.last().unwrap());
    assert_eq!(ids(&open(&s.path)), s.ids, "and it opens whole");
}

#[test]
fn repro_f_a_partial_line_waits_then_lands_as_one_upsert() {
    let mut s = scenario();
    let before = s.tail.log().turns().back().unwrap().revision;
    let whole = text("assistant", "a reply being written right now");
    append(&s.path, &whole.as_bytes()[..whole.len() / 2]);
    assert!(
        tick(&mut s.tail, &s.path).is_empty(),
        "half a line is no news"
    );
    append(
        &s.path,
        format!("{}\n", &whole[whole.len() / 2..]).as_bytes(),
    );
    let delta = tick(&mut s.tail, &s.path);
    assert_eq!(delta.upserts.len(), 1);
    assert_eq!(delta.upserts[0].offset, *s.ids.last().unwrap());
    assert_eq!(delta.upserts[0].revision, before + 1);
    kept(&s);
}

#[test]
fn repro_g_a_giant_line_still_being_written_changes_nothing() {
    let mut s = scenario();
    let whole = tool_result(0, 3 * 1024 * 1024);
    append(&s.path, &whole.as_bytes()[..whole.len() - 10]);
    assert!(tick(&mut s.tail, &s.path).is_empty());
    kept(&s);
    assert_eq!(ids(&open(&s.path)), s.ids, "opening skips the torn line");
}

#[test]
fn repro_h_a_codex_command_output_larger_than_the_old_window() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rollout.jsonl");
    let item = |payload: serde_json::Value| {
        serde_json::json!({"timestamp": AT, "type": "response_item", "payload": payload})
            .to_string()
    };
    let mut lines = Vec::new();
    for n in 0..5 {
        lines.push(item(serde_json::json!({"type": "message", "role": "user", "content": [{"type": "input_text", "text": format!("question {n}")}]})));
        lines.push(item(serde_json::json!({"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": format!("answer {n}")}]})));
    }
    lines.push(item(serde_json::json!({"type": "function_call", "name": "shell", "call_id": "c1", "arguments": "{\"command\":[\"make\"]}"})));
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    let mut tail = open(&path);
    let before = ids(&tail);
    assert_eq!(before.len(), 10);
    append_lines(
        &path,
        &[item(
            serde_json::json!({"type": "function_call_output", "call_id": "c1", "output": "o".repeat(3 * 1024 * 1024)}),
        )],
    );
    assert!(tick(&mut tail, &path).is_empty());
    assert_eq!(ids(&tail), before);
    assert_eq!(ids(&open(&path)), before);
}

#[test]
fn repro_j_after_a_long_stretch_the_reply_lands_on_the_open_turn() {
    let mut s = scenario();
    append_lines(
        &s.path,
        &(0..250).map(|_| progress(10_000)).collect::<Vec<_>>(),
    );
    assert!(tick(&mut s.tail, &s.path).is_empty());
    append_lines(
        &s.path,
        &[text("assistant", "Done: the parser is refactored.")],
    );
    let delta = tick(&mut s.tail, &s.path);
    kept(&s);
    assert_eq!(ids(&s.tail).len(), 26, "the earlier turns stay");
    assert_eq!(delta.upserts.len(), 1);
    assert_eq!(delta.upserts[0].offset, *s.ids.last().unwrap());
    assert!(
        delta.upserts[0]
            .text()
            .ends_with("Done: the parser is refactored.")
    );
}

#[test]
fn repro_i_following_reads_only_what_was_appended() {
    let mut s = scenario();
    for _ in 0..4 {
        let lines: Vec<String> = (0..20)
            .map(|n| tool_result(n, 20_000))
            .chain((0..20).map(tool_use))
            .collect();
        let at = s.tail.offset();
        append_lines(&s.path, &lines);
        let len = std::fs::metadata(&s.path).unwrap().len();
        let (caught_up, read) = s.tail.read_within(&s.path, 16 * 1024 * 1024).unwrap();
        assert!(caught_up);
        assert_eq!(read, len - at, "only the new bytes");
        let delta = s.tail.log_mut().take_delta();
        assert_eq!(delta.upserts.len(), 1, "one turn re-sent, not the list");
    }
    kept(&s);
}

#[test]
fn a_line_longer_than_the_bound_is_passed_over_counted_and_what_follows_reads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.jsonl");
    base(&path);
    let mut tail = open(&path);
    let turns = tail.log().turns().len();
    // 40 MiB with no newline yet: held at most the bound.
    let giant = tool_result(0, 40 * 1024 * 1024);
    append(&path, &giant.as_bytes()[..giant.len() - 1]);
    for _ in 0..4 {
        tail.read_within(&path, 16 * 1024 * 1024).unwrap();
    }
    assert_eq!(tail.log().skipped(), 0);
    append(&path, b"}\n");
    append_lines(&path, &[text("assistant", "after the giant")]);
    let delta = tick(&mut tail, &path);
    assert_eq!(tail.log().skipped(), 1);
    assert_eq!(tail.log().turns().len(), turns);
    assert!(delta.upserts[0].text().ends_with("after the giant"));
}

#[test]
fn the_held_part_of_a_line_never_passes_the_bound() {
    let mut hold = LineHold::default();
    let mut log = TurnLog::new(Lines::Session);
    let chunk = vec![b'x'; 1024 * 1024];
    for n in 0..20u64 {
        cut_lines(&mut hold, &mut log, &chunk, n * 1024 * 1024);
        assert!(hold.held() <= LINE_HOLD_MAX);
    }
    assert_eq!(hold.skipping(), Some(0));
    let line = format!("\n{}\n", text("user", "next"));
    cut_lines(&mut hold, &mut log, line.as_bytes(), 20 * 1024 * 1024);
    assert_eq!(log.skipped(), 1);
    assert_eq!(log.turns().len(), 1);
    assert_eq!(
        log.turns()[0].offset,
        20 * 1024 * 1024 + 1,
        "the next line's own offset"
    );
}

/// A transcript with every kind of line the rules care about.
fn mixed_claude() -> Vec<String> {
    let meta = serde_json::json!({"type": "user", "isMeta": true, "message": {"role": "user", "content": "Caveat"}}).to_string();
    let thinking = serde_json::json!({"type": "assistant", "uuid": "th1", "timestamp": AT, "message": {"role": "assistant", "content": [{"type": "thinking", "thinking": "let me see"}]}}).to_string();
    let queued = serde_json::json!({"type": "attachment", "uuid": "q1", "timestamp": AT, "attachment": {"type": "queued_command", "origin": {"kind": "human"}, "commandMode": "prompt", "prompt": "also the footer"}}).to_string();
    let mut lines = vec![text("user", "fix the header"), meta, thinking];
    lines.push(text("assistant", "On it."));
    for n in 0..5 {
        lines.push(tool_use(n));
        lines.push(tool_result(n, 300));
        lines.push(sidechain("assistant", 200));
    }
    lines.push(queued);
    lines.push(text("assistant", "And the footer."));
    lines.push(notification("<task-notification>done</task-notification>"));
    lines.push(text("assistant", "The task finished."));
    lines.push(text("user", "<command-name>/clear</command-name>"));
    lines.push(text("user", "thanks"));
    lines.push(progress(500));
    lines.push(text("assistant", "You're welcome."));
    lines
}

fn mixed_codex() -> Vec<String> {
    let item = |payload: serde_json::Value| {
        serde_json::json!({"timestamp": AT, "type": "response_item", "payload": payload})
            .to_string()
    };
    let mut lines = vec![
        serde_json::json!({"timestamp": AT, "type": "turn_context", "payload": {"model": "gpt-5"}})
            .to_string(),
        item(
            serde_json::json!({"type": "message", "role": "user", "content": [{"type": "input_text", "text": "<environment_context>x</environment_context>"}]}),
        ),
    ];
    for n in 0..4 {
        lines.push(item(serde_json::json!({"type": "message", "role": "user", "content": [{"type": "input_text", "text": format!("ask {n}")}]})));
        lines.push(item(
            serde_json::json!({"type": "reasoning", "summary": []}),
        ));
        lines.push(item(serde_json::json!({"type": "function_call", "name": "shell", "call_id": format!("c{n}"), "arguments": "{\"command\":[\"ls\"]}"})));
        lines.push(item(serde_json::json!({"type": "function_call_output", "call_id": format!("c{n}"), "output": "ok"})));
        lines.push(item(serde_json::json!({"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": format!("answer {n}")}]})));
    }
    lines
}

/// Feeds `bytes` in chunks of the sizes `sizes` cycles through.
fn chunked(bytes: &[u8], seed: u64) -> TurnLog {
    let mut log = TurnLog::new(Lines::Session);
    let mut hold = LineHold::default();
    let mut state = seed;
    let mut at = 0;
    while at < bytes.len() {
        // A small LCG: chunk sizes from 1 byte to 64 KiB.
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let size = usize::try_from(state >> 48).unwrap() % (64 * 1024) + 1;
        let size = if state.is_multiple_of(3) {
            size % 7 + 1
        } else {
            size
        };
        let end = (at + size).min(bytes.len());
        cut_lines(&mut hold, &mut log, &bytes[at..end], at as u64);
        at = end;
    }
    log
}

#[test]
fn any_chunking_gives_the_turns_of_a_one_shot_read() {
    for lines in [mixed_claude(), mixed_codex()] {
        let bytes = (lines.join("\n") + "\n").into_bytes();
        let mut whole = TurnLog::new(Lines::Session);
        whole.feed_at(&bytes, 0);
        assert!(whole.turns().len() >= 4);
        for seed in 1..=12 {
            let parts = chunked(&bytes, seed);
            assert_eq!(parts.turns(), whole.turns(), "seed {seed}");
        }
    }
}

#[test]
fn the_claude_rules_turns_stretches_and_what_a_title_needs() {
    let mut log = TurnLog::new(Lines::Session);
    log.feed_at(&(mixed_claude().join("\n") + "\n").into_bytes(), 0);
    let shown: Vec<(Speaker, String)> = log.turns().iter().map(|t| (t.speaker, t.text())).collect();
    assert_eq!(
        shown,
        vec![
            (Speaker::You, "fix the header".to_owned()),
            (Speaker::Assistant, "On it.".to_owned()),
            // The person's message mid-turn splits it there.
            (Speaker::You, "also the footer".to_owned()),
            (Speaker::Assistant, "And the footer.".to_owned()),
            // A notification ends the turn before it.
            (Speaker::Assistant, "The task finished.".to_owned()),
            (Speaker::You, "thanks".to_owned()),
            (Speaker::Assistant, "You're welcome.".to_owned()),
        ]
    );
    let work = &log.turns()[1];
    // Thinking first (a run with its thought), then the words, then one
    // run of five calls, each done.
    assert!(
        matches!(&work.blocks[0], Block::Steps(run) if run.steps.is_empty() && run.thoughts == ["let me see"])
    );
    let Block::Text(words) = &work.blocks[1] else {
        unreachable!("words")
    };
    assert!(words.after_thinking);
    let Block::Steps(run) = &work.blocks[2] else {
        unreachable!("a run")
    };
    assert_eq!(run.steps.len(), 5);
    assert!(
        run.steps
            .iter()
            .all(|step| step.result.as_ref().is_some_and(|r| !r.failed))
    );
    assert_eq!(run.steps[0].input.command.as_deref(), Some("make step-0"));
    assert_eq!(log.layout(), Layout::Stretches);
    assert_eq!(
        fold_title(log.layout(), run).map(|(done, _)| done),
        Some("Ran 5 commands".to_owned())
    );
}

#[test]
fn narration_shows_as_its_own_block() {
    // field 2 → 1 → 8 = "narration", base64.
    let signature = "CAESDQoLQgluYXJyYXRpb24=";
    let line = serde_json::json!({"type": "assistant", "timestamp": AT, "message": {"role": "assistant", "content": [{"type": "thinking", "thinking": "Checking the header first.", "signature": signature}]}}).to_string();
    let mut log = TurnLog::new(Lines::Session);
    log.feed_at(
        (format!(
            "{}\n{line}\n{}\n",
            text("user", "go"),
            text("assistant", "The header is fixed.")
        ))
        .as_bytes(),
        0,
    );
    let turn = &log.turns()[1];
    let Block::Narration(narration) = &turn.blocks[0] else {
        unreachable!("narration: {:?}", turn.blocks)
    };
    assert_eq!(narration.text(), "Checking the header first.");
    assert!(matches!(&turn.blocks[1], Block::Text(text) if text.text() == "The header is fixed."));
}

#[test]
fn an_open_turn_grows_under_its_id_until_a_new_prompt() {
    let mut log = TurnLog::new(Lines::Session);
    let mut at = 0u64;
    let mut feed = |log: &mut TurnLog, line: String| {
        let bytes = format!("{line}\n");
        log.feed_at(bytes.as_bytes(), at);
        at += bytes.len() as u64;
        log.take_delta()
    };
    feed(&mut log, text("user", "go"));
    let first = feed(&mut log, text("assistant", "Starting."));
    let id = first.upserts.last().unwrap().offset;
    let call = feed(&mut log, tool_use(1));
    assert_eq!(call.upserts.len(), 1);
    assert_eq!(call.upserts[0].offset, id, "a call extends the open turn");
    let more = feed(&mut log, text("assistant", "Done."));
    assert_eq!(more.upserts[0].offset, id);
    assert!(more.upserts[0].revision > call.upserts[0].revision);
    let next = feed(&mut log, text("user", "next"));
    // The new prompt, and the turn it closed.
    assert_eq!(next.upserts.len(), 2);
    assert_eq!(next.upserts[0].offset, id);
    assert_ne!(next.upserts[1].offset, id);
}

#[test]
fn the_log_keeps_its_bound_and_a_scan_joins_it_without_a_seam() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.jsonl");
    let lines: Vec<String> = (0..5_000)
        .flat_map(|n| {
            [
                text("user", &format!("ask {n}")),
                text("assistant", &format!("answer {n}")),
            ]
        })
        .collect();
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    let mut tail: TranscriptTail<TurnLog> = TranscriptTail::default();
    tail.read_all(&path).unwrap();
    let log = tail.log();
    assert_eq!(log.turns().len(), Limits::default().turns);
    let earlier = log.earlier().expect("earlier turns were dropped");
    assert_eq!(earlier, log.turns()[0].offset);
    let Back::Page(page) =
        scan_back(&path, Some(earlier), 30, &TurnLog::new(Lines::Session)).unwrap()
    else {
        unreachable!("plain")
    };
    let before = page.log.turns();
    assert_eq!(before.len(), 30);
    assert_eq!(before.back().unwrap().end, earlier, "no gap, no overlap");
    assert_eq!(before.back().unwrap().text(), "answer 4849");
    assert_eq!(log.turns()[0].text(), "ask 4850");
}

struct Stars;

impl Scrub for Stars {
    fn scrub(&self, pieces: &mut [String]) -> u32 {
        let mut count = 0;
        for piece in pieces {
            if piece.contains("SECRET") {
                *piece = piece.replace("SECRET", "•••");
                count += 1;
            }
        }
        count
    }
}

#[test]
fn a_scrubbing_log_keeps_only_redacted_words_and_counts_them() {
    let mut log = TurnLog::new(Lines::Session).with_scrub(std::sync::Arc::new(Stars));
    let call = serde_json::json!({"type": "assistant", "timestamp": AT, "message": {"role": "assistant", "content": [{"type": "tool_use", "id": "t", "name": "Bash", "input": {"command": "curl -H SECRET", "description": "Call it with SECRET"}}]}}).to_string();
    log.feed_at(
        format!("{}\n{call}\n", text("user", "the key is SECRET")).as_bytes(),
        0,
    );
    assert_eq!(log.turns()[0].text(), "the key is •••");
    assert_eq!(log.turns()[0].redacted, 1);
    let Block::Steps(run) = &log.turns()[1].blocks[0] else {
        unreachable!("a run")
    };
    let step = &run.steps[0];
    assert!(!step.line.contains("SECRET"));
    assert_eq!(step.input.command.as_deref(), Some("curl -H •••"));
    assert_eq!(step.input.description.as_deref(), Some("Call it with •••"));
    assert_eq!(log.turns()[1].redacted, 3);
}

// A pasted image's base64 (inline data) is left out of a line as it is
// held: the message shows, its offsets the file's own.

const BIG: usize = 12 * 1024 * 1024;

/// `size` bytes of base64, `/` and `+` among them.
fn base64_of(size: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    (0..size)
        .map(|n| char::from(ALPHABET[(n * 7 + n / 64) % 64]))
        .collect()
}

fn image_part(size: usize) -> serde_json::Value {
    serde_json::json!({"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": base64_of(size)}})
}

/// A message of yours: `words` and a pasted image of `size` base64 bytes;
/// with `escaped`, every `/` written `\/` (as some writers do).
fn with_image(words: &str, size: usize, escaped: bool) -> String {
    let line = serde_json::json!({
        "type": "user", "timestamp": AT,
        "message": {"role": "user", "content": [{"type": "text", "text": words}, image_part(size)]},
    })
    .to_string();
    if escaped {
        line.replace('/', "\\/")
    } else {
        line
    }
}

/// The result of call `n`: a screenshot of `size` base64 bytes.
fn result_with_image(n: usize, size: usize) -> String {
    serde_json::json!({
        "type": "user", "timestamp": AT,
        "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": format!("t{n}"), "content": [{"type": "text", "text": "shot"}, image_part(size)]}]},
    })
    .to_string()
}

fn codex_with_image(words: &str, size: usize) -> String {
    serde_json::json!({
        "timestamp": AT, "type": "response_item",
        "payload": {"type": "message", "role": "user", "content": [
            {"type": "input_text", "text": words},
            {"type": "input_image", "image_url": format!("data:image/png;base64,{}", base64_of(size))},
        ]},
    })
    .to_string()
}

/// Where each of `lines` starts, and where the last ends.
fn offsets(lines: &[String]) -> Vec<u64> {
    let mut at = vec![0];
    for line in lines {
        at.push(at.last().unwrap() + line.len() as u64 + 1);
    }
    at
}

/// Every line as a log that leaves out inline data is handed it: where
/// it starts and ends, and its bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Kept {
    lines: Vec<(u64, u64, String)>,
    skipped: Vec<u64>,
}

impl crate::transcript_tail::TailLog for Kept {
    const LINE_MAX: usize = LINE_HOLD_MAX;
    const DROP_INLINE_DATA: bool = true;

    fn feed(&mut self, _bytes: &[u8]) {
        unreachable!("a tail says where lines start");
    }

    fn feed_at(&mut self, bytes: &[u8], base: u64) {
        let mut at = base;
        for line in bytes
            .strip_suffix(b"\n")
            .unwrap_or(bytes)
            .split(|b| *b == b'\n')
        {
            let end = at + line.len() as u64 + 1;
            self.lines
                .push((at, end, String::from_utf8(line.to_vec()).unwrap()));
            at = end;
        }
    }

    fn feed_line(&mut self, at: u64, end: u64, bytes: &[u8]) {
        let line = bytes.strip_suffix(b"\n").unwrap();
        self.lines
            .push((at, end, String::from_utf8(line.to_vec()).unwrap()));
    }

    fn skipped_line(&mut self, at: u64) {
        self.skipped.push(at);
    }
}

fn cut_whole<L: crate::transcript_tail::TailLog>(bytes: &[u8]) -> L {
    let mut log = L::default();
    cut_lines(&mut LineHold::default(), &mut log, bytes, 0);
    log
}

/// The image parts of a Claude line's message (and of its results).
fn image_sources(line: &str) -> Vec<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(line).unwrap();
    let mut found = Vec::new();
    let mut parts: Vec<&serde_json::Value> = value["message"]["content"]
        .as_array()
        .unwrap()
        .iter()
        .collect();
    while let Some(part) = parts.pop() {
        if part["type"] == "image" {
            found.push(part["source"].clone());
        }
        if let Some(inner) = part["content"].as_array() {
            parts.extend(inner);
        }
    }
    found
}

#[test]
fn a_message_long_only_for_its_image_shows_with_its_words_and_calls() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.jsonl");
    let lines = vec![
        text("assistant", "hello"),
        with_image("look at this", BIG, false),
        tool_use(1),
        result_with_image(1, BIG),
        text("assistant", "after"),
    ];
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    let at = offsets(&lines);
    // Read 16 MiB at a time: the first image whole in a read, the second
    // across two.
    let mut tail = TranscriptTail::<TurnLog>::default();
    tail.read_all(&path).unwrap();
    let log = tail.log();
    assert_eq!(log.skipped(), 0);
    assert_eq!(log.turns().len(), 3);
    let yours = &log.turns()[1];
    assert_eq!(yours.speaker, Speaker::You);
    assert_eq!(yours.text(), "look at this");
    assert_eq!((yours.offset, yours.end), (at[1], at[2]));
    let reply = &log.turns()[2];
    assert_eq!((reply.offset, reply.end), (at[2], at[5]));
    assert_eq!(reply.text(), "after");
    let Block::Steps(run) = &reply.blocks[0] else {
        unreachable!("a run")
    };
    assert_eq!(run.steps[0].call_id.as_deref(), Some("t1"));
    assert_eq!(
        run.steps[0].result,
        Some(StepResult {
            failed: false,
            kind: None
        }),
        "the result with a screenshot was read"
    );
    assert_eq!(tail.offset(), at[5]);
    assert_eq!(tail.rewound().offset(), at[5]);

    // What the log is handed: each image part still there, still
    // counted, its bytes left out; the offsets the file's.
    let kept: Kept = cut_whole(&std::fs::read(&path).unwrap());
    assert!(kept.skipped.is_empty());
    let spans: Vec<(u64, u64)> = kept.lines.iter().map(|(a, e, _)| (*a, *e)).collect();
    assert_eq!(
        spans,
        at.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>()
    );
    for n in [1, 3] {
        let line = &kept.lines[n].2;
        assert!(line.len() < 1024, "line {n} kept {} bytes", line.len());
        assert_eq!(
            image_sources(line),
            vec![serde_json::json!({"type": "base64", "media_type": "image/png", "data": ""})]
        );
    }
    // A small image is kept as it is.
    let small = with_image("small", 1_000, false);
    let kept: Kept = cut_whole(format!("{small}\n").as_bytes());
    assert_eq!(kept.lines[0].2, small);
}

#[test]
fn the_backward_reader_shows_the_image_message_the_same() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.jsonl");
    let lines = vec![
        text("user", "q"),
        text("assistant", "hello"),
        with_image("look at this", BIG, true),
        text("assistant", "after"),
    ];
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    let at = offsets(&lines);
    let mut forward = TranscriptTail::<TurnLog>::default();
    forward.read_all(&path).unwrap();
    let mut forward = forward.log().clone();
    forward.settle();
    let Back::Page(page) = scan_back(&path, None, 30, &TurnLog::new(Lines::Session)).unwrap()
    else {
        unreachable!("plain")
    };
    assert_eq!(page.log.skipped(), 0);
    assert_eq!(page.log.turns(), forward.turns());
    assert_eq!(page.end, at[4]);
    assert_eq!(page.log.turns()[2].text(), "look at this");
    assert_eq!(page.log.turns()[2].offset, at[2]);
    // "Show earlier" from the reply after it.
    let Back::Page(earlier) =
        scan_back(&path, Some(at[3]), 1, &TurnLog::new(Lines::Session)).unwrap()
    else {
        unreachable!("plain")
    };
    assert_eq!(earlier.log.turns().len(), 1);
    assert_eq!(earlier.log.turns()[0].text(), "look at this");
    assert_eq!(
        (earlier.log.turns()[0].offset, earlier.log.turns()[0].end),
        (at[2], at[3])
    );
}

#[test]
fn an_image_line_reads_the_same_at_any_chunk_boundary() {
    let lines = vec![
        text("user", "q"),
        with_image("look at this", BIG, true),
        tool_use(1),
        result_with_image(1, 200_000),
        text("assistant", "after"),
    ];
    let bytes = (lines.join("\n") + "\n").into_bytes();
    let at = offsets(&lines);
    let whole: TurnLog = cut_whole(&bytes);
    assert_eq!(whole.skipped(), 0);
    assert_eq!(whole.turns()[0].text(), "q\n\nlook at this");
    assert_eq!(whole.turns()[1].offset, at[2]);
    for seed in 1..=3 {
        let parts = chunked(&bytes, seed);
        assert_eq!(parts.skipped(), 0);
        assert_eq!(parts.turns(), whole.turns(), "seed {seed}");
    }
    // Read a MiB at a time, what is held of the image line stays small:
    // its bytes are never held.
    let mut hold = LineHold::default();
    let mut log = TurnLog::new(Lines::Session);
    for (n, chunk) in bytes.chunks(1024 * 1024).enumerate() {
        cut_lines(&mut hold, &mut log, chunk, (n * 1024 * 1024) as u64);
        assert!(hold.held() <= crate::inline_data::KEEP_MAX + 1024);
    }
    assert_eq!(log.turns(), whole.turns());
}

#[test]
fn an_image_split_inside_a_key_an_escape_or_at_the_bound_reads_the_same() {
    let line = with_image("look", 100_000, true);
    let bytes = format!("{line}\n").into_bytes();
    let whole: Kept = cut_whole(&bytes);
    assert_eq!(whole.lines.len(), 1);
    assert_eq!(
        (whole.lines[0].0, whole.lines[0].1),
        (0, bytes.len() as u64)
    );
    assert_eq!(image_sources(&whole.lines[0].2)[0]["data"], "");
    let key = line.find("\"data\":\"").unwrap();
    let content = key + 8;
    let bound = content + crate::inline_data::KEEP_MAX;
    let escape = bound + line[bound..].find("\\/").unwrap();
    let end = content + line[content..].find('"').unwrap();
    let mut splits: Vec<usize> = Vec::new();
    for (from, to) in [
        (key - 4, content + 8),
        (bound - 12, bound + 12),
        (escape - 3, escape + 4),
        (end - 4, end + 4),
    ] {
        splits.extend(from..to);
    }
    for split in splits {
        let mut hold = LineHold::default();
        let mut log = Kept::default();
        cut_lines(&mut hold, &mut log, &bytes[..split], 0);
        cut_lines(&mut hold, &mut log, &bytes[split..], split as u64);
        assert_eq!(log, whole, "split at {split}");
    }
    // A byte at a time through the escapes around the bound.
    let mut hold = LineHold::default();
    let mut log = Kept::default();
    cut_lines(&mut hold, &mut log, &bytes[..bound - 100], 0);
    for at in bound - 100..bound + 100 {
        cut_lines(&mut hold, &mut log, &bytes[at..=at], at as u64);
    }
    cut_lines(
        &mut hold,
        &mut log,
        &bytes[bound + 100..],
        (bound + 100) as u64,
    );
    assert_eq!(log, whole);
}

#[test]
fn a_codex_image_keeps_its_data_url_header() {
    let lines = vec![
        serde_json::json!({"timestamp": AT, "type": "turn_context", "payload": {"model": "gpt-5"}})
            .to_string(),
        codex_with_image("look at this", BIG),
        serde_json::json!({"timestamp": AT, "type": "response_item", "payload": {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "a cat"}]}})
            .to_string(),
    ];
    let bytes = (lines.join("\n") + "\n").into_bytes();
    let at = offsets(&lines);
    let kept: Kept = cut_whole(&bytes);
    let value: serde_json::Value = serde_json::from_str(&kept.lines[1].2).unwrap();
    assert_eq!(
        value["payload"]["content"][1],
        serde_json::json!({"type": "input_image", "image_url": "data:image/png;base64,"})
    );
    let log = chunked(&bytes, 7);
    assert_eq!(log.skipped(), 0);
    let shown: Vec<(u64, String)> = log
        .turns()
        .iter()
        .map(|turn| (turn.offset, turn.text()))
        .collect();
    assert_eq!(
        shown,
        vec![
            (at[1], "look at this".to_owned()),
            (at[2], "a cat".to_owned())
        ]
    );
}

#[test]
fn a_line_long_for_other_reasons_is_still_passed_over() {
    let size = LINE_HOLD_MAX + 1024;
    let lines = vec![
        text("user", "q"),
        // Long words.
        text("assistant", &"w".repeat(size)),
        // Base64 that is someone's words, not an image's data.
        text("user", &base64_of(size)),
        // A `data` value that isn't base64.
        serde_json::json!({"type": "user", "data": "some words ".repeat(size / 11 + 1)})
            .to_string(),
        text("assistant", "after"),
    ];
    let bytes = (lines.join("\n") + "\n").into_bytes();
    let at = offsets(&lines);
    let kept: Kept = cut_whole(&bytes);
    assert_eq!(kept.skipped, vec![at[1], at[2], at[3]]);
    for log in [cut_whole::<TurnLog>(&bytes), chunked(&bytes, 5)] {
        assert_eq!(log.skipped(), 3);
        let shown: Vec<(u64, String)> = log
            .turns()
            .iter()
            .map(|turn| (turn.offset, turn.text()))
            .collect();
        assert_eq!(
            shown,
            vec![(at[0], "q".to_owned()), (at[4], "after".to_owned())]
        );
    }
}

#[test]
fn a_save_mid_image_holds_no_part_of_it_and_a_restore_reads_it_whole() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.jsonl");
    let lines = vec![
        text("user", "q"),
        with_image("look at this", BIG, true),
        text("assistant", "after"),
    ];
    let bytes = (lines.join("\n") + "\n").into_bytes();
    std::fs::write(&path, &bytes).unwrap();
    let at = offsets(&lines);
    let mut straight = TranscriptTail::<Kept>::default();
    straight.read_all(&path).unwrap();
    // Stopped right after a `\` deep in the image's data.
    let line = &lines[1];
    let content = line.find("\"data\":\"").unwrap() + 8;
    let escape = content + 5_000_000 + line[content + 5_000_000..].find("\\/").unwrap();
    let stop = at[1] + escape as u64 + 1;
    let mut tail = TranscriptTail::<Kept>::default();
    tail.read_up_to(&path, stop).unwrap();
    assert_eq!(tail.offset(), stop);
    // A save: back to the image line's start, nothing of it held, in
    // the names a save always had.
    let saved = serde_json::to_value(tail.rewound()).unwrap();
    assert_eq!(saved["offset"], at[1]);
    assert_eq!(saved["partial"], serde_json::json!([]));
    assert_eq!(saved["skipping"], serde_json::Value::Null);
    let mut restored: TranscriptTail<Kept> = serde_json::from_value(saved).unwrap();
    restored.read_all(&path).unwrap();
    assert_eq!(restored.log(), straight.log());
    // Even a tail kept mid-line (not a save) goes on where it was.
    let copy = serde_json::to_vec(&tail).unwrap();
    assert!(copy.len() < 2 * crate::inline_data::KEEP_MAX);
    let mut copy: TranscriptTail<Kept> = serde_json::from_slice(&copy).unwrap();
    copy.read_all(&path).unwrap();
    assert_eq!(copy.log(), straight.log());
    // The feed's own tail, rewound mid-line, reads the message once.
    let mut feed = TranscriptTail::<TurnLog>::default();
    feed.read_up_to(&path, stop).unwrap();
    let mut again = feed.rewound();
    assert_eq!(again.offset(), at[1]);
    again.read_all(&path).unwrap();
    let shown: Vec<(u64, String)> = again
        .log()
        .turns()
        .iter()
        .map(|turn| (turn.offset, turn.text()))
        .collect();
    assert_eq!(
        shown,
        vec![
            (at[0], "q\n\nlook at this".to_owned()),
            (at[2], "after".to_owned())
        ]
    );
}

/// AJ-E3106 Req.3: Codex's own mark decides a turn's reply. The message it
/// marked `final_answer` is the reply; its commentary is its own block,
/// never merged into it, even when it comes after.
#[test]
fn codexs_final_answer_is_its_own_block_and_marked() {
    let message = |phase: &str, words: &str| {
        serde_json::json!({"timestamp": AT, "type": "response_item", "payload": {"type": "message", "role": "assistant", "phase": phase, "content": [{"type": "output_text", "text": words}]}})
            .to_string()
    };
    let lines = [
        serde_json::json!({"timestamp": AT, "type": "turn_context", "payload": {"model": "gpt-5"}}).to_string(),
        serde_json::json!({"timestamp": AT, "type": "response_item", "payload": {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "Fix the refunds"}]}}).to_string(),
        message("commentary", "Looking at the handler first."),
        message("final_answer", "Refunds are fixed and tested."),
        message("commentary", "One more note on the logs."),
    ];
    let bytes = (lines.join("\n") + "\n").into_bytes();
    let log = chunked(&bytes, 3);
    let turn = log.turns().back().expect("the reply's turn");
    let texts: Vec<(String, bool)> = turn
        .blocks
        .iter()
        .filter_map(|block| match block {
            crate::conversation_feed::Block::Text(text) => Some((text.text(), text.final_answer)),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        vec![
            ("Looking at the handler first.".to_owned(), false),
            ("Refunds are fixed and tested.".to_owned(), true),
            ("One more note on the logs.".to_owned(), false),
        ]
    );
}
