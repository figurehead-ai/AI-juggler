//! AJ-E2108: a session's last few turns, for its page (D-37: read when the
//! page opens, kept nowhere).
//!
//! Only the transcript's end is read: at most [`TAIL_BYTES`], from a seek
//! to the end, whole lines only. The tools' own parsers ([`parse_any`])
//! say who spoke and what text each message holds; tool calls, tool
//! results, a tool's injected lines and sub-agents' lines are left out.
//! Consecutive messages of one speaker are one turn. A compressed file (a
//! finished Codex rollout) has no end to seek to and is not read.
//!
//! What this returns is raw text: the caller redacts it before it goes
//! anywhere (the app's D-14 patterns).
//!
//! release/0.2's `transcript_tail.rs` follows a transcript as it grows;
//! this is a one-off read of its end, so it lives apart.

use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;

use crate::claude_transcript::MessageRole;
use crate::transcript::{is_compressed, parse_any};

/// The most bytes read from a transcript's end (the quick read's tail).
pub const TAIL_BYTES: u64 = juggler_adapter::conversation::QUICK_TAIL;

/// The most characters one turn keeps (its end: where a question is).
pub const TURN_CHARS: usize = 1_200;

/// Who spoke.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speaker {
    /// The person.
    You,
    /// The tool's model.
    Assistant,
}

/// One turn: who spoke, and what they wrote (raw, unredacted).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LastTurn {
    /// Who spoke.
    pub speaker: Speaker,
    /// The text, at most [`TURN_CHARS`] (its end, after `…`).
    pub text: String,
}

/// What a transcript's end holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LastTurns {
    /// Its last turns, oldest first (none when its end holds no text).
    Turns(Vec<LastTurn>),
    /// Stored compressed: not read.
    Compressed,
}

/// The last `max` text turns of the transcript at `path`, from its last
/// [`TAIL_BYTES`].
///
/// # Errors
///
/// The file can't be opened or read.
pub fn last_turns(path: &Path, max: usize) -> std::io::Result<LastTurns> {
    if is_compressed(path) {
        return Ok(LastTurns::Compressed);
    }
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(TAIL_BYTES);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.take(TAIL_BYTES).read_to_end(&mut bytes)?;
    // From the first whole line: the seek lands mid-line.
    if start > 0 {
        let first = bytes
            .iter()
            .position(|b| *b == b'\n')
            .map_or(bytes.len(), |at| at + 1);
        bytes.drain(..first);
    }
    Ok(LastTurns::Turns(turns_of(&bytes, max)))
}

/// The last `max` turns in whole JSONL lines.
#[must_use]
pub fn turns_of(bytes: &[u8], max: usize) -> Vec<LastTurn> {
    // A line the tool marks as its own (Claude Code's `isMeta`: the
    // caveats around a local command) is not a turn.
    let kept: Vec<u8> = bytes
        .split(|b| *b == b'\n')
        .filter(|line| !contains(line, br#""isMeta":true"#))
        .flat_map(|line| line.iter().copied().chain(*b"\n"))
        .collect();
    let mut turns: Vec<LastTurn> = Vec::new();
    for message in parse_any(&kept).messages {
        let speaker = match message.role {
            MessageRole::User => Speaker::You,
            MessageRole::Assistant => Speaker::Assistant,
            MessageRole::Other => continue,
        };
        let text = message.text.trim();
        // A tool's own lines (`<command-name>`, reminders) open with a tag.
        if message.sidechain
            || text.is_empty()
            || (speaker == Speaker::You && text.starts_with('<'))
        {
            continue;
        }
        match turns.last_mut() {
            Some(last) if last.speaker == speaker => {
                last.text.push_str("\n\n");
                last.text.push_str(text);
            }
            _ => turns.push(LastTurn {
                speaker,
                text: text.to_owned(),
            }),
        }
    }
    let from = turns.len().saturating_sub(max);
    turns
        .drain(from..)
        .map(|turn| LastTurn {
            text: ending(&turn.text, TURN_CHARS),
            ..turn
        })
        .collect()
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
        serde_json::json!({
            "type": kind,
            "sessionId": "s",
            "message": {"role": kind, "content": [{"type": "text", "text": text}]},
        })
        .to_string()
    }

    fn write(dir: &Path, name: &str, lines: &[String]) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        path
    }

    fn texts(turns: &LastTurns) -> Vec<(Speaker, String)> {
        match turns {
            LastTurns::Turns(turns) => turns.iter().map(|t| (t.speaker, t.text.clone())).collect(),
            LastTurns::Compressed => Vec::new(),
        }
    }

    #[test]
    fn the_last_turns_of_a_claude_transcript_in_order_one_per_speaker() {
        let dir = tempfile::tempdir().unwrap();
        let tool_result = serde_json::json!({
            "type": "user",
            "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "t", "content": "ok"}]},
        })
        .to_string();
        let path = write(
            dir.path(),
            "s.jsonl",
            &[
                claude("user", "first ask"),
                claude("assistant", "first answer"),
                claude("user", "Fix the settings screen"),
                claude("assistant", "Reading it now."),
                tool_result,
                claude("assistant", "Should I keep the old layout?"),
            ],
        );
        assert_eq!(
            texts(&last_turns(&path, 3).unwrap()),
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
        assert_eq!(
            texts(&last_turns(&path, 3).unwrap()),
            vec![
                (Speaker::You, "real ask".to_owned()),
                (Speaker::Assistant, "real answer".to_owned()),
            ]
        );
    }

    #[test]
    fn only_the_end_is_read_and_a_cut_line_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let mut lines = vec![claude("user", "long ago")];
        // Well past the tail: the early lines are never read.
        let filler = "x".repeat(1_000);
        for _ in 0..200 {
            lines.push(claude("assistant", &filler));
        }
        lines.push(claude("user", "the latest ask"));
        let path = write(dir.path(), "s.jsonl", &lines);
        assert!(std::fs::metadata(&path).unwrap().len() > TAIL_BYTES * 2);
        let turns = texts(&last_turns(&path, 3).unwrap());
        assert!(turns.iter().all(|(_, text)| !text.contains("long ago")));
        assert_eq!(
            turns.last().unwrap(),
            &(Speaker::You, "the latest ask".to_owned())
        );
        // A turn keeps its end.
        assert!(turns[0].1.starts_with('…'));
        assert!(turns[0].1.chars().count() <= TURN_CHARS + 1);
    }

    #[test]
    fn a_codex_rollout_reads_through_its_own_parser() {
        let dir = tempfile::tempdir().unwrap();
        let line = |role: &str, kind: &str, text: &str| {
            serde_json::json!({
                "timestamp": "2026-10-24T10:00:00Z",
                "type": "response_item",
                "payload": {"type": "message", "role": role, "content": [{"type": kind, "text": text}]},
            })
            .to_string()
        };
        let path = write(
            dir.path(),
            "rollout-2026-10-24T10-00-00-019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a7b.jsonl",
            &[
                line(
                    "user",
                    "input_text",
                    "<environment_context>cwd</environment_context>",
                ),
                line("user", "input_text", "Add pagination"),
                line("assistant", "output_text", "Done: page and limit."),
            ],
        );
        assert_eq!(
            texts(&last_turns(&path, 3).unwrap()),
            vec![
                (Speaker::You, "Add pagination".to_owned()),
                (Speaker::Assistant, "Done: page and limit.".to_owned()),
            ]
        );
    }

    #[test]
    fn a_compressed_file_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rollout.jsonl.zst");
        std::fs::write(&path, b"not zstd").unwrap();
        assert_eq!(last_turns(&path, 3).unwrap(), LastTurns::Compressed);
    }
}
