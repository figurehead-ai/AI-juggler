//! AJ-E2108: a session's last messages, for its page.
//!
//! Read while the page shows, never before and never in the background:
//! the end of its transcript (the adapters' `last_turns`, a bounded
//! window from the end), up to [`TURNS`] turns with their tool calls in a
//! few words, with keys and tokens taken out by the same patterns a
//! context pack's redaction uses (D-14, `juggler_handover::redaction`).
//! A page that follows a working session asks again with the file's
//! stamp: an unchanged file answers `unchanged` from its metadata alone.
//! Kept nowhere: nothing here writes a file, a setting, a log line or the
//! search index; the text goes to the page and nowhere else. Off in
//! Settings, nothing is read at all.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use juggler_adapters::last_turns::{
    Block, LastTurns, Speaker, Window, last_turns, stamp, subagent_turns,
};

use crate::error::AppResult;
use crate::state::AppState;

/// The setting: `"0"` turns last messages off; anything else (unset
/// included) leaves them on.
pub const SETTING_KEY: &str = "show_last_messages";

/// The most turns one read returns, unless the page asks for fewer.
pub const TURNS: usize = 30;

/// The most turns a page may ask for at once.
pub const TURNS_MAX: usize = 60;

/// A part of a turn: its words, or a run of tool calls.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TurnBlockInfo {
    /// `text` or `steps`.
    pub kind: String,
    /// The words (markdown as written), redacted; empty for `steps`.
    pub text: String,
    /// One line per tool call (`ran "pnpm test"`, `Edit a.ts`),
    /// redacted; empty for `text`.
    pub steps: Vec<String>,
    /// Earlier calls in this run not shown.
    pub hidden: u32,
}

/// One turn, redacted.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LastTurnInfo {
    /// `you` or `assistant`.
    pub speaker: String,
    /// Its parts, in order.
    pub blocks: Vec<TurnBlockInfo>,
    /// When its last message was written (epoch ms), when known.
    #[ts(type = "number | null")]
    pub at_ms: Option<i64>,
    /// The model an assistant turn ran on, when the transcript says.
    pub model: Option<String>,
}

/// A session's last messages, or why there are none.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LastMessagesInfo {
    /// `shown`; `unchanged` (the stamp the page has is current);
    /// `off` (the setting); `none` (no transcript on this Mac: a chat
    /// its app keeps on its servers); `compressed` (a finished rollout
    /// the tool stored compressed); `unreadable`.
    pub state: String,
    /// The turns, oldest first (empty unless `shown`).
    pub turns: Vec<LastTurnInfo>,
    /// How many keys or tokens were taken out.
    pub redacted: u32,
    /// Where these turns start in the transcript: pass it as `before`
    /// for earlier ones, or as `from` to follow along; `null` when they
    /// reach its start.
    #[ts(type = "number | null")]
    pub earlier: Option<u64>,
    /// The transcript's stamp when read: pass it as `known` to hear
    /// `unchanged` while it is.
    pub stamp: Option<String>,
}

impl LastMessagesInfo {
    pub(crate) fn without(state: &str) -> Self {
        Self {
            state: state.to_owned(),
            turns: Vec::new(),
            redacted: 0,
            earlier: None,
            stamp: None,
        }
    }
}

/// What the page asks for: the last turns (none set), the ones `before`
/// an offset, or everything `from` one; `known` is the stamp it shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ask {
    /// The stamp of the turns the page shows.
    pub known: Option<String>,
    /// Earlier turns: before this offset.
    pub before: Option<u64>,
    /// Follow along: everything from this offset.
    pub from: Option<u64>,
    /// How many turns (at most [`TURNS_MAX`]).
    pub max: Option<usize>,
}

/// Whether the user leaves last messages on (the default).
#[must_use]
pub fn enabled(state: &Arc<AppState>) -> bool {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(SETTING_KEY).ok().flatten())
        .is_none_or(|raw| raw != "0")
}

/// Core: `account`'s conversation `id`'s last messages.
///
/// # Errors
///
/// None today; the result says what it couldn't read.
pub fn last_messages_impl(
    state: &Arc<AppState>,
    account_id: &str,
    conversation_id: &str,
    ask: &Ask,
) -> AppResult<LastMessagesInfo> {
    Ok(answer(
        enabled(state),
        || state.conversations.path_of(account_id, conversation_id),
        ask,
    ))
}

/// Off: nothing is looked up or read. On: the transcript `find` names.
fn answer(
    on: bool,
    find: impl FnOnce() -> Option<std::path::PathBuf>,
    ask: &Ask,
) -> LastMessagesInfo {
    if !on {
        return LastMessagesInfo::without("off");
    }
    read(find().as_deref(), ask)
}

/// The last messages of the transcript at `path` (none: no transcript),
/// redacted.
#[must_use]
pub fn read(path: Option<&Path>, ask: &Ask) -> LastMessagesInfo {
    read_by(path, ask, last_turns)
}

/// AJ-E1520 (0.2): the same for a subagent's own transcript (its lines
/// are all the subagent's).
#[must_use]
pub fn read_subagent(path: Option<&Path>, ask: &Ask) -> LastMessagesInfo {
    read_by(path, ask, subagent_turns)
}

type TurnsReader = fn(&Path, usize, Window) -> std::io::Result<LastTurns>;

fn read_by(path: Option<&Path>, ask: &Ask, reader: TurnsReader) -> LastMessagesInfo {
    let Some(path) = path else {
        return LastMessagesInfo::without("none");
    };
    let now = stamp(path).ok();
    // The page shows the end already and the file hasn't changed: no read.
    if ask.before.is_none() && now.is_some() && ask.known == now {
        return LastMessagesInfo {
            stamp: now,
            ..LastMessagesInfo::without("unchanged")
        };
    }
    let window = match (ask.before, ask.from) {
        (Some(before), _) => Window::Before(before),
        (None, Some(from)) => Window::From(from),
        (None, None) => Window::Latest,
    };
    let max = ask.max.unwrap_or(TURNS).clamp(1, TURNS_MAX);
    match reader(path, max, window) {
        Ok(LastTurns::Turns(page)) => {
            // Every text and every call line, redacted together.
            let mut pieces: Vec<String> = Vec::new();
            for turn in &page.turns {
                for block in &turn.blocks {
                    match block {
                        Block::Text(text) => pieces.push(text.clone()),
                        Block::Steps { lines, .. } => pieces.extend(lines.iter().cloned()),
                    }
                }
            }
            let (clean, counts) =
                juggler_handover::redaction::strip_segments(&pieces, true, &BTreeSet::new());
            let mut clean = clean.into_iter();
            let mut next = || clean.next().unwrap_or_default();
            let mut turns = Vec::with_capacity(page.turns.len());
            for turn in &page.turns {
                let mut blocks = Vec::with_capacity(turn.blocks.len());
                for block in &turn.blocks {
                    blocks.push(match block {
                        Block::Text(_) => TurnBlockInfo {
                            kind: "text".to_owned(),
                            text: next(),
                            steps: Vec::new(),
                            hidden: 0,
                        },
                        Block::Steps { lines, hidden } => TurnBlockInfo {
                            kind: "steps".to_owned(),
                            text: String::new(),
                            steps: lines.iter().map(|_| next()).collect(),
                            hidden: u32::try_from(*hidden).unwrap_or(u32::MAX),
                        },
                    });
                }
                turns.push(LastTurnInfo {
                    speaker: match turn.speaker {
                        Speaker::You => "you",
                        Speaker::Assistant => "assistant",
                    }
                    .to_owned(),
                    blocks,
                    at_ms: turn.at_ms,
                    model: turn.model.clone(),
                });
            }
            LastMessagesInfo {
                state: "shown".to_owned(),
                turns,
                redacted: counts.iter().map(|count| count.count).sum(),
                earlier: page.earlier,
                stamp: now,
            }
        }
        Ok(LastTurns::Compressed) => LastMessagesInfo::without("compressed"),
        Err(_) => LastMessagesInfo::without("unreadable"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(kind: &str, text: &str) -> String {
        serde_json::json!({
            "type": kind,
            "message": {"role": kind, "content": [{"type": "text", "text": text}]},
        })
        .to_string()
    }

    fn command(line: &str) -> String {
        serde_json::json!({
            "type": "assistant",
            "message": {"role": "assistant", "content": [{"type": "tool_use", "id": "t", "name": "Bash", "input": {"command": line}}]},
        })
        .to_string()
    }

    fn transcript(lines: &[String]) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        (dir, path)
    }

    fn all_text(info: &LastMessagesInfo) -> String {
        info.turns
            .iter()
            .flat_map(|t| t.blocks.iter())
            .flat_map(|b| std::iter::once(b.text.clone()).chain(b.steps.iter().cloned()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn an_api_key_in_a_turn_shows_redacted() {
        let (_dir, path) = transcript(&[
            turn(
                "user",
                "Use the key sk-ant-api03-abcdefghijklmnop to call it",
            ),
            turn(
                "assistant",
                "Set OPENAI_API_KEY=sk-proj-1234567890abcdef and Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload",
            ),
            command(
                "curl -H 'Authorization: Bearer eyJzZWNyZXQiOiJ2YWx1ZSJ9.sig' https://example.test",
            ),
        ]);
        let info = read(Some(&path), &Ask::default());
        assert_eq!(info.state, "shown");
        let all = all_text(&info);
        for secret in [
            "abcdefghijklmnop",
            "1234567890abcdef",
            "eyJhbGciOiJIUzI1NiJ9",
            "eyJzZWNyZXQiOiJ2YWx1ZSJ9",
        ] {
            // The message never prints the value (CodeQL: a secret in a log).
            assert!(
                !all.contains(secret),
                "a test secret leaked into the shown text"
            );
        }
        assert!(all.contains("•••"), "{all}");
        assert!(info.redacted >= 4, "{info:?}");
        assert_eq!(info.turns[0].speaker, "you");
        assert_eq!(info.turns[1].speaker, "assistant");
        assert_eq!(info.turns[1].blocks[1].kind, "steps");
    }

    #[test]
    fn a_private_key_block_is_taken_out_whole() {
        let (_dir, path) = transcript(&[turn(
            "user",
            "here:\n-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXk\n-----END OPENSSH PRIVATE KEY-----\nthanks",
        )]);
        let text = &read(Some(&path), &Ask::default()).turns[0].blocks[0].text;
        assert!(
            !text.contains("b3BlbnNzaC1rZXk"),
            "the key block leaked into the shown text"
        );
        assert!(
            text.starts_with("here:") && text.ends_with("thanks"),
            "the words around the key block were lost"
        );
    }

    #[test]
    fn up_to_thirty_turns_and_a_cursor_for_earlier_ones() {
        let lines: Vec<String> = (0..40)
            .flat_map(|n| {
                [
                    turn("user", &format!("ask {n}")),
                    turn("assistant", &format!("answer {n}")),
                ]
            })
            .collect();
        let (_dir, path) = transcript(&lines);
        let info = read(Some(&path), &Ask::default());
        assert_eq!(info.turns.len(), TURNS);
        assert_eq!(info.turns.last().unwrap().blocks[0].text, "answer 39");
        // The whole file fit the first window: the cursor is where the
        // shown turns start, and earlier ones come from before it.
        let cursor = info.earlier.expect("more before");
        let before = read(
            Some(&path),
            &Ask {
                before: Some(cursor),
                ..Ask::default()
            },
        );
        assert_eq!(before.turns.len(), TURNS);
        assert_eq!(before.turns.last().unwrap().blocks[0].text, "answer 24");
        assert_eq!(before.turns[0].blocks[0].text, "ask 10");
        let first = read(
            Some(&path),
            &Ask {
                before: before.earlier,
                ..Ask::default()
            },
        );
        assert_eq!(first.turns.len(), 80 - 2 * TURNS);
        assert_eq!(first.turns[0].blocks[0].text, "ask 0");
        assert_eq!(first.earlier, None, "it reaches the start");
        // Asking for more than the most is held to it.
        let many = read(
            Some(&path),
            &Ask {
                max: Some(1_000),
                ..Ask::default()
            },
        );
        assert_eq!(many.turns.len(), TURNS_MAX);
    }

    #[test]
    fn an_unchanged_transcript_answers_unchanged_without_reading_turns() {
        let (_dir, path) = transcript(&[turn("user", "hello"), turn("assistant", "hi")]);
        let first = read(Some(&path), &Ask::default());
        let known = first.stamp.clone();
        assert!(known.is_some());
        let again = read(
            Some(&path),
            &Ask {
                known: known.clone(),
                ..Ask::default()
            },
        );
        assert_eq!(again.state, "unchanged");
        assert!(again.turns.is_empty());
        assert_eq!(again.stamp, known);
        // Once it grows, the read is a real one again.
        let mut more = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        std::io::Write::write_all(&mut more, format!("{}\n", turn("user", "more")).as_bytes())
            .unwrap();
        let grown = read(
            Some(&path),
            &Ask {
                known,
                from: first.earlier.or(Some(0)),
                ..Ask::default()
            },
        );
        assert_eq!(grown.state, "shown");
        assert_eq!(grown.turns.len(), 3);
    }

    #[test]
    fn no_transcript_or_a_compressed_one_says_so_and_reads_nothing() {
        assert_eq!(read(None, &Ask::default()).state, "none");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rollout.jsonl.zst");
        std::fs::write(&path, b"x").unwrap();
        assert_eq!(read(Some(&path), &Ask::default()).state, "compressed");
        assert_eq!(
            read(Some(&dir.path().join("gone.jsonl")), &Ask::default()).state,
            "unreadable"
        );
    }

    #[test]
    fn off_in_settings_no_transcript_is_looked_up_or_read() {
        let (_dir, path) = transcript(&[turn("user", "hello")]);
        let mut looked = false;
        let info = answer(
            false,
            || {
                looked = true;
                Some(path.clone())
            },
            &Ask::default(),
        );
        assert_eq!(info.state, "off");
        assert!(info.turns.is_empty());
        assert!(!looked, "off reads nothing");
        assert_eq!(
            answer(true, || Some(path.clone()), &Ask::default()).state,
            "shown"
        );
    }

    #[test]
    fn nothing_is_kept_written_or_logged() {
        // The page's text lives in this read's answer only: this module
        // writes no file or setting, logs nothing and feeds no index.
        let source = include_str!("last_messages.rs");
        let code = source.split("#[cfg(test)]").next().unwrap();
        for word in [
            "fs::write",
            "File::create",
            "OpenOptions",
            ".set(",
            "tracing::",
            "log::",
            "println!",
            "eprintln!",
            "transcript_index",
            "outside_search",
        ] {
            assert!(
                !code.contains(word),
                "last messages must keep nothing: {word}"
            );
        }
        // Reading leaves the folder as it was.
        let (dir, path) = transcript(&[turn("user", "hello")]);
        let before = std::fs::read_dir(dir.path()).unwrap().count();
        let _ = read(Some(&path), &Ask::default());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
    }
}
