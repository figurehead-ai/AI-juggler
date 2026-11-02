//! The feed: the 0.3.4 repro's scenarios (a long turn emptied the
//! card) as regression tests, and the builder's rules (ids, growth,
//! chunking, bounds, the Claude app's stretches).

use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::*;
use crate::last_turns::{Back, scan_back};
use crate::transcript_tail::{TailLog as _, TranscriptTail, cut_lines};

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
    let mut partial = Vec::new();
    let mut skipping = None;
    let mut log = TurnLog::new(Lines::Session);
    let chunk = vec![b'x'; 1024 * 1024];
    for n in 0..20u64 {
        cut_lines(
            &mut partial,
            &mut skipping,
            &mut log,
            &chunk,
            n * 1024 * 1024,
        );
        assert!(partial.len() <= LINE_HOLD_MAX);
    }
    assert_eq!(skipping, Some(0));
    let line = format!("\n{}\n", text("user", "next"));
    cut_lines(
        &mut partial,
        &mut skipping,
        &mut log,
        line.as_bytes(),
        20 * 1024 * 1024,
    );
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
    let mut partial = Vec::new();
    let mut skipping = None;
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
        cut_lines(
            &mut partial,
            &mut skipping,
            &mut log,
            &bytes[at..end],
            at as u64,
        );
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
