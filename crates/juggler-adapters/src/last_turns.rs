//! AJ-E2108: a session's last turns, for its page (D-37: read when the
//! page shows, kept nowhere).
//!
//! Only the transcript's end is read: a window of at most
//! [`MAX_WINDOW`] bytes, from a seek, whole lines only. The window grows
//! ([`WINDOWS`]) only until it holds the turns asked for. The tools' own
//! parsers ([`parse_any`]) say who spoke, what text each message holds
//! and which tool calls it made; tool results, a tool's injected lines and
//! sub-agents' lines are left out. Consecutive messages of one speaker
//! are one turn: its words and its tool calls in the order they came.
//!
//! Each turn carries where it starts in the file, so the page can ask for
//! the turns before it ("Show earlier") or everything after it (to follow
//! a working session), and a file's [`stamp`] (its length and change
//! time) lets a page that already shows its end hear "unchanged" without
//! a read. A compressed file (a finished Codex rollout) has no end to
//! seek to and is not read.
//!
//! What this returns is raw text: the caller redacts it before it goes
//! anywhere (the app's D-14 patterns).

use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;

use crate::claude_transcript::{MessageRole, ParsedMessage};
use crate::transcript::{TranscriptFormat, detect_format, is_compressed, parse_any};

/// The windows tried in turn, from the end (or from a page's start).
pub const WINDOWS: [u64; 3] = [256 * 1024, 1024 * 1024, MAX_WINDOW];

/// The most bytes one read takes from a transcript.
pub const MAX_WINDOW: u64 = 2 * 1024 * 1024;

/// The most characters one text block keeps (its end: where a question
/// is).
pub const BLOCK_CHARS: usize = 4_000;

/// The most tool calls one run of them keeps (the latest).
pub const STEPS_MAX: usize = 40;

/// The most turns a follow-along read returns.
pub const FOLLOW_MAX: usize = 200;

/// Who spoke.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speaker {
    /// The person.
    You,
    /// The tool's model.
    Assistant,
}

/// A part of a turn: what was written, or a run of tool calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// Text, at most [`BLOCK_CHARS`] (its end, after `…`).
    Text(String),
    /// Tool calls in a few words each ("ran \"pnpm test\"", "Edit a.ts"),
    /// the latest [`STEPS_MAX`]; `hidden` earlier ones not kept.
    Steps {
        /// One line per call.
        lines: Vec<String>,
        /// How many earlier calls in this run were left out.
        hidden: usize,
    },
}

/// One turn: who spoke, and what they wrote and did (raw, unredacted).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LastTurn {
    /// Who spoke.
    pub speaker: Speaker,
    /// Its parts, in order.
    pub blocks: Vec<Block>,
    /// When its last message was written (epoch ms), when the line says.
    pub at_ms: Option<i64>,
    /// The model an assistant turn ran on, when the transcript says.
    pub model: Option<String>,
    /// Where its first line starts in the file.
    pub offset: u64,
}

impl LastTurn {
    /// The turn's words, its text blocks joined.
    #[must_use]
    pub fn text(&self) -> String {
        self.blocks
            .iter()
            .filter_map(|block| match block {
                Block::Text(text) => Some(text.as_str()),
                Block::Steps { .. } => None,
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// A run of turns from one read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    /// Oldest first.
    pub turns: Vec<LastTurn>,
    /// Where these turns start: read before it for earlier ones; `None`
    /// when they reach the transcript's start.
    pub earlier: Option<u64>,
}

/// What a transcript's end holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LastTurns {
    /// Its turns (none when the window holds no text).
    Turns(Page),
    /// Stored compressed: not read.
    Compressed,
}

/// Which turns to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    /// The last ones.
    Latest,
    /// The last ones before this offset (a page's `earlier`).
    Before(u64),
    /// Everything from this offset (a page's start) to the end, when that
    /// is at most [`MAX_WINDOW`]; else the last ones.
    From(u64),
}

/// The file's length and change time, in one string: equal stamps, an
/// unchanged file.
///
/// # Errors
///
/// The file's metadata can't be read.
pub fn stamp(path: &Path) -> std::io::Result<String> {
    let meta = std::fs::metadata(path)?;
    let changed = meta
        .modified()
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |at| at.as_nanos());
    Ok(format!("{}:{changed}", meta.len()))
}

/// The last `max` turns of the transcript at `path` (or those `window`
/// names), from at most [`MAX_WINDOW`] bytes.
///
/// # Errors
///
/// The file can't be opened or read.
pub fn last_turns(path: &Path, max: usize, window: Window) -> std::io::Result<LastTurns> {
    if is_compressed(path) {
        return Ok(LastTurns::Compressed);
    }
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    let max = max.max(1);
    let end = match window {
        Window::From(from) if from <= len && len - from <= MAX_WINDOW => {
            let bytes = read_range(&mut file, from, len)?;
            let mut turns = turns_at(&bytes, from);
            let cut = turns.len().saturating_sub(FOLLOW_MAX);
            turns.drain(..cut);
            let earlier = match turns.first() {
                Some(first) => (first.offset > 0).then_some(first.offset),
                None => (from > 0).then_some(from),
            };
            return Ok(LastTurns::Turns(Page { turns, earlier }));
        }
        Window::Latest | Window::From(_) => len,
        Window::Before(before) => before.min(len),
    };
    let mut page = Page {
        turns: Vec::new(),
        earlier: None,
    };
    for size in WINDOWS {
        let start = end.saturating_sub(size);
        let mut bytes = read_range(&mut file, start, end)?;
        // From the first whole line: the seek lands mid-line.
        let mut base = start;
        if start > 0 {
            let first = bytes
                .iter()
                .position(|b| *b == b'\n')
                .map_or(bytes.len(), |at| at + 1);
            bytes.drain(..first);
            base += first as u64;
        }
        let mut turns = turns_at(&bytes, base);
        // The window's first turn may have begun before it.
        if start > 0 && !turns.is_empty() {
            turns.remove(0);
        }
        let enough = turns.len() >= max || start == 0 || size == MAX_WINDOW;
        let cut = turns.len().saturating_sub(max);
        turns.drain(..cut);
        page.earlier = match turns.first() {
            Some(first) => (first.offset > 0).then_some(first.offset),
            None => (base > 0).then_some(base),
        };
        page.turns = turns;
        if enough {
            break;
        }
    }
    Ok(LastTurns::Turns(page))
}

fn read_range(file: &mut std::fs::File, start: u64, end: u64) -> std::io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.take(end.saturating_sub(start))
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// What one line says about itself, for timestamps and offsets (the
/// parsers say the rest).
#[derive(serde::Deserialize, Default)]
struct LineHead {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    message: Option<serde::de::IgnoredAny>,
    payload: Option<PayloadHead>,
}

#[derive(serde::Deserialize, Default)]
struct PayloadHead {
    id: Option<String>,
}

/// The turns in whole JSONL lines that start at `base` in the file.
#[must_use]
pub fn turns_at(bytes: &[u8], base: u64) -> Vec<LastTurn> {
    // A line the tool marks as its own (Claude Code's `isMeta`: the
    // caveats around a local command) is not a turn. Each kept line is
    // remembered with where it starts.
    let mut kept: Vec<u8> = Vec::with_capacity(bytes.len() + 1);
    let mut offsets: Vec<u64> = Vec::new();
    let mut at = base;
    for line in bytes.split(|b| *b == b'\n') {
        if !contains(line, br#""isMeta":true"#) {
            offsets.push(at);
            kept.extend_from_slice(line);
            kept.push(b'\n');
        }
        at += line.len() as u64 + 1;
    }
    let heads: Vec<LineHead> = kept
        .split(|b| *b == b'\n')
        .map(|line| serde_json::from_slice(line).unwrap_or_default())
        .collect();
    let format = detect_format(&kept);
    let messages = parse_any(&kept).messages;
    let lines = message_lines(format, &heads, &messages);
    let mut turns: Vec<LastTurn> = Vec::new();
    for (message, line) in messages.iter().zip(lines) {
        let speaker = match message.role {
            MessageRole::User => Speaker::You,
            MessageRole::Assistant => Speaker::Assistant,
            MessageRole::Other => continue,
        };
        let text = message.text.trim();
        let tools: &[String] = if speaker == Speaker::Assistant {
            &message.tools
        } else {
            &[]
        };
        // A tool's own lines (`<command-name>`, reminders) open with a tag;
        // a tool result has no text of its own.
        if message.sidechain
            || (text.is_empty() && tools.is_empty())
            || (speaker == Speaker::You && text.starts_with('<'))
        {
            continue;
        }
        let head = line.and_then(|index| heads.get(index));
        let at_ms = head
            .and_then(|head| head.timestamp.as_deref())
            .and_then(juggler_core::time::timestamp_ms);
        let offset = line
            .and_then(|index| offsets.get(index).copied())
            .unwrap_or(base);
        if turns.last().is_none_or(|last| last.speaker != speaker) {
            turns.push(LastTurn {
                speaker,
                blocks: Vec::new(),
                at_ms: None,
                model: None,
                offset,
            });
        }
        let Some(turn) = turns.last_mut() else {
            continue;
        };
        turn.at_ms = at_ms.or(turn.at_ms);
        if message.model.is_some() {
            turn.model.clone_from(&message.model);
        }
        // A rollout's calls come before the words that follow them; a
        // Claude message's words before its calls.
        if format == TranscriptFormat::CodexRollout {
            push_steps(&mut turn.blocks, tools);
            push_text(&mut turn.blocks, text);
        } else {
            push_text(&mut turn.blocks, text);
            push_steps(&mut turn.blocks, tools);
        }
    }
    for turn in &mut turns {
        for block in &mut turn.blocks {
            match block {
                Block::Text(text) => *text = ending(text, BLOCK_CHARS),
                Block::Steps { lines, hidden } => {
                    let cut = lines.len().saturating_sub(STEPS_MAX);
                    lines.drain(..cut);
                    *hidden += cut;
                }
            }
        }
    }
    turns
}

fn push_text(blocks: &mut Vec<Block>, text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(Block::Text(last)) = blocks.last_mut() {
        last.push_str("\n\n");
        last.push_str(text);
    } else {
        blocks.push(Block::Text(text.to_owned()));
    }
}

fn push_steps(blocks: &mut Vec<Block>, tools: &[String]) {
    if tools.is_empty() {
        return;
    }
    if let Some(Block::Steps { lines, .. }) = blocks.last_mut() {
        lines.extend(tools.iter().cloned());
    } else {
        blocks.push(Block::Steps {
            lines: tools.to_vec(),
            hidden: 0,
        });
    }
}

/// The line each message came from (its index among the kept lines).
/// Claude Code: one message per `user`/`assistant` line that carries a
/// message, in order. A rollout: by the item's id, else the line number
/// its parser put in the id.
fn message_lines(
    format: TranscriptFormat,
    heads: &[LineHead],
    messages: &[ParsedMessage],
) -> Vec<Option<usize>> {
    match format {
        TranscriptFormat::ClaudeCode => {
            let mut lines = heads.iter().enumerate().filter(|(_, head)| {
                matches!(head.kind.as_deref(), Some("user" | "assistant")) && head.message.is_some()
            });
            messages
                .iter()
                .map(|_| lines.next().map(|(index, _)| index))
                .collect()
        }
        TranscriptFormat::CodexRollout => {
            let by_id: std::collections::HashMap<&str, usize> = heads
                .iter()
                .enumerate()
                .filter_map(|(index, head)| {
                    head.payload
                        .as_ref()
                        .and_then(|payload| payload.id.as_deref())
                        .map(|id| (id, index))
                })
                .collect();
            let last = heads.iter().rposition(|head| head.timestamp.is_some());
            messages
                .iter()
                .map(|message| {
                    let uuid = message.uuid.as_deref()?;
                    if let Some(index) = by_id.get(uuid) {
                        return Some(*index);
                    }
                    let (_, number) = uuid.rsplit_once(':')?;
                    let index: usize = number.parse().ok()?;
                    // Calls nothing followed: flushed at the next ask, or
                    // at the end (`usize::MAX`).
                    if index >= heads.len() {
                        last
                    } else {
                        Some(index)
                    }
                })
                .collect()
        }
    }
}

fn contains(line: &[u8], needle: &[u8]) -> bool {
    line.windows(needle.len()).any(|window| window == needle)
}

/// A text's last `max` characters, after `…` when it was longer.
fn ending(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_owned();
    }
    let tail: String = text.chars().skip(count - max).collect();
    format!("…{}", tail.trim_start())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claude(kind: &str, text: &str) -> String {
        claude_at(kind, text, "2026-10-24T10:00:00Z")
    }

    fn claude_at(kind: &str, text: &str, at: &str) -> String {
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

    fn write(dir: &Path, name: &str, lines: &[String]) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        path
    }

    fn page(turns: LastTurns) -> Page {
        match turns {
            LastTurns::Turns(page) => page,
            LastTurns::Compressed => unreachable!("a plain file reads as turns"),
        }
    }

    fn texts(turns: &LastTurns) -> Vec<(Speaker, String)> {
        match turns {
            LastTurns::Turns(page) => page.turns.iter().map(|t| (t.speaker, t.text())).collect(),
            LastTurns::Compressed => Vec::new(),
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
            texts(&last_turns(&path, 3, Window::Latest).unwrap()),
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
        let page = page(last_turns(&path, 30, Window::Latest).unwrap());
        assert_eq!(page.turns.len(), 2);
        assert_eq!(page.earlier, None, "the whole file was read");
        let answer = &page.turns[1];
        assert_eq!(
            answer.blocks,
            vec![
                Block::Text("Running them.".to_owned()),
                Block::Steps {
                    lines: vec!["ran \"pnpm test\"".to_owned(), "Read src/a.ts".to_owned()],
                    hidden: 0
                },
                Block::Text("All green.".to_owned()),
            ]
        );
        assert_eq!(
            answer.at_ms,
            juggler_core::time::timestamp_ms("2026-10-24T10:02:00Z")
        );
        assert_eq!(answer.model.as_deref(), Some("claude-opus-4-5"));
        assert_eq!(
            page.turns[0].at_ms,
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
        let read = last_turns(&path, 3, Window::Latest).unwrap();
        assert_eq!(
            texts(&read),
            vec![
                (Speaker::You, "real ask".to_owned()),
                (Speaker::Assistant, "real answer".to_owned()),
            ]
        );
        // The offsets survive the lines left out: the answer starts at its
        // own line.
        let page = page(read);
        let bytes = std::fs::read(&path).unwrap();
        let at = usize::try_from(page.turns[1].offset).unwrap();
        let line = String::from_utf8_lossy(&bytes[at..]);
        assert!(line.lines().next().unwrap().contains("real answer"));
    }

    fn long_session(dir: &Path, turns: usize, filler: usize) -> std::path::PathBuf {
        let pad = "x".repeat(filler);
        let mut lines = Vec::new();
        for n in 0..turns {
            lines.push(claude("user", &format!("ask {n} {pad}")));
            lines.push(claude("assistant", &format!("answer {n} {pad}")));
        }
        write(dir, "s.jsonl", &lines)
    }

    #[test]
    fn up_to_max_turns_from_a_bounded_window_with_a_cursor_to_page_back() {
        let dir = tempfile::tempdir().unwrap();
        // 600 turns of ~1.2 KB: past twice the first window.
        let path = long_session(dir.path(), 300, 1_000);
        let len = std::fs::metadata(&path).unwrap().len();
        assert!(len > WINDOWS[0] * 2);
        let first = page(last_turns(&path, 30, Window::Latest).unwrap());
        assert_eq!(first.turns.len(), 30);
        assert!(first.turns.last().unwrap().text().starts_with("answer 299"));
        assert!(first.turns[0].text().starts_with("ask 285"));
        let cursor = first.earlier.expect("more before");
        assert_eq!(cursor, first.turns[0].offset);
        // The page before ends where this one starts, nothing lost or
        // repeated at the seam.
        let before = page(last_turns(&path, 30, Window::Before(cursor)).unwrap());
        assert_eq!(before.turns.len(), 30);
        assert!(
            before
                .turns
                .last()
                .unwrap()
                .text()
                .starts_with("answer 284")
        );
        assert!(before.turns[0].text().starts_with("ask 270"));
        assert!(before.earlier.unwrap() < cursor);
    }

    #[test]
    fn a_window_grows_only_to_the_bound_and_never_reads_the_whole_file() {
        let dir = tempfile::tempdir().unwrap();
        // Turns of ~200 KB: 30 of them would need 6 MB.
        let path = long_session(dir.path(), 30, 200_000);
        let len = std::fs::metadata(&path).unwrap().len();
        assert!(len > MAX_WINDOW * 4);
        let read = page(last_turns(&path, 30, Window::Latest).unwrap());
        let span = len - read.earlier.unwrap();
        assert!(span <= MAX_WINDOW, "read past the bound: {span}");
        assert!(!read.turns.is_empty() && read.turns.len() < 30);
        // A turn keeps its end.
        assert!(read.turns[0].text().starts_with('…'));
        assert!(read.turns[0].text().chars().count() <= BLOCK_CHARS + 1);
    }

    #[test]
    fn only_the_end_is_read_and_a_cut_line_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let mut lines = vec![claude("user", "long ago")];
        // Well past the largest window: the early lines are never read.
        let filler = "x".repeat(10_000);
        for _ in 0..300 {
            lines.push(claude("assistant", &filler));
        }
        lines.push(claude("user", "the latest ask"));
        let path = write(dir.path(), "s.jsonl", &lines);
        assert!(std::fs::metadata(&path).unwrap().len() > MAX_WINDOW);
        let turns = texts(&last_turns(&path, 3, Window::Latest).unwrap());
        assert!(turns.iter().all(|(_, text)| !text.contains("long ago")));
        assert_eq!(
            turns.last().unwrap(),
            &(Speaker::You, "the latest ask".to_owned())
        );
    }

    #[test]
    fn following_reads_everything_from_a_pages_start() {
        let dir = tempfile::tempdir().unwrap();
        let path = long_session(dir.path(), 50, 10);
        let first = page(last_turns(&path, 10, Window::Latest).unwrap());
        let from = first.turns[0].offset;
        // It wrote two more turns.
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
        let next = page(last_turns(&path, 10, Window::From(from)).unwrap());
        assert_eq!(next.turns.len(), 12, "the page's turns and the new ones");
        assert_eq!(next.turns[0], first.turns[0]);
        assert_eq!(next.turns.last().unwrap().text(), "done");
        assert_eq!(next.earlier, first.earlier);
        // A start past the bound reads the end instead.
        let far = page(last_turns(&path, 10, Window::From(u64::MAX)).unwrap());
        assert_eq!(far.turns.len(), 10);
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
    fn a_codex_rollout_reads_through_its_own_parser_calls_first() {
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
        let read = last_turns(&path, 3, Window::Latest).unwrap();
        assert_eq!(
            texts(&read),
            vec![
                (Speaker::You, "Add pagination".to_owned()),
                (Speaker::Assistant, "Done: page and limit.".to_owned()),
            ]
        );
        let page = page(read);
        assert!(
            matches!(page.turns[1].blocks[0], Block::Steps { .. }),
            "{:?}",
            page.turns[1].blocks
        );
        assert_eq!(
            page.turns[1].at_ms,
            juggler_core::time::timestamp_ms("2026-10-24T10:00:40Z")
        );
        assert_eq!(
            page.turns[0].at_ms,
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
        let page = page(last_turns(&path, 3, Window::Latest).unwrap());
        let Block::Steps { lines, hidden } = &page.turns[1].blocks[0] else {
            unreachable!("the turn is its calls")
        };
        assert_eq!(lines.len(), STEPS_MAX);
        assert_eq!(*hidden, 5);
        assert!(
            lines
                .last()
                .unwrap()
                .contains(&format!("step {}", STEPS_MAX + 4))
        );
    }

    #[test]
    fn a_compressed_file_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rollout.jsonl.zst");
        std::fs::write(&path, b"not zstd").unwrap();
        assert_eq!(
            last_turns(&path, 3, Window::Latest).unwrap(),
            LastTurns::Compressed
        );
    }
}
