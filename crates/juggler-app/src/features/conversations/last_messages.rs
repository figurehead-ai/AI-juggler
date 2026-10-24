//! AJ-E2108: a session's last messages, for its page.
//!
//! Read when the page opens, never before and never in the background:
//! the end of its transcript (the adapters' `last_turns`, at most 64 KB),
//! its last 2–3 text turns, with keys and tokens taken out by the same
//! patterns a context pack's redaction uses (D-14,
//! `juggler_handover::redaction`). Kept nowhere: nothing here writes a
//! file, a setting, a log line or the search index; the text goes to the
//! page and nowhere else. Off in Settings, nothing is read at all.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use juggler_adapters::last_turns::{LastTurns, Speaker, last_turns};

use crate::error::AppResult;
use crate::state::AppState;

/// The setting: `"0"` turns last messages off; anything else (unset
/// included) leaves them on.
pub const SETTING_KEY: &str = "show_last_messages";

/// The most turns a page shows.
pub const TURNS: usize = 3;

/// One turn, redacted.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LastTurnInfo {
    /// `you` or `assistant`.
    pub speaker: String,
    /// What was written, with keys and tokens replaced by `•••`.
    pub text: String,
}

/// A session's last messages, or why there are none.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LastMessagesInfo {
    /// `shown`; `off` (the setting); `none` (no transcript on this Mac:
    /// a chat its app keeps on its servers); `compressed` (a finished
    /// rollout the tool stored compressed); `unreadable`.
    pub state: String,
    /// The turns, oldest first (empty unless `shown`).
    pub turns: Vec<LastTurnInfo>,
    /// How many keys or tokens were taken out.
    pub redacted: u32,
}

impl LastMessagesInfo {
    fn without(state: &str) -> Self {
        Self {
            state: state.to_owned(),
            turns: Vec::new(),
            redacted: 0,
        }
    }
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
) -> AppResult<LastMessagesInfo> {
    Ok(answer(enabled(state), || {
        state.conversations.path_of(account_id, conversation_id)
    }))
}

/// Off: nothing is looked up or read. On: the transcript `find` names.
fn answer(on: bool, find: impl FnOnce() -> Option<std::path::PathBuf>) -> LastMessagesInfo {
    if !on {
        return LastMessagesInfo::without("off");
    }
    read(find().as_deref())
}

/// The last messages of the transcript at `path` (none: no transcript),
/// redacted.
#[must_use]
pub fn read(path: Option<&Path>) -> LastMessagesInfo {
    let Some(path) = path else {
        return LastMessagesInfo::without("none");
    };
    match last_turns(path, TURNS) {
        Ok(LastTurns::Turns(turns)) => {
            let texts: Vec<String> = turns.iter().map(|turn| turn.text.clone()).collect();
            let (texts, counts) =
                juggler_handover::redaction::strip_segments(&texts, true, &BTreeSet::new());
            LastMessagesInfo {
                state: "shown".to_owned(),
                turns: turns
                    .iter()
                    .zip(texts)
                    .map(|(turn, text)| LastTurnInfo {
                        speaker: match turn.speaker {
                            Speaker::You => "you",
                            Speaker::Assistant => "assistant",
                        }
                        .to_owned(),
                        text,
                    })
                    .collect(),
                redacted: counts.iter().map(|count| count.count).sum(),
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

    fn transcript(lines: &[String]) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        (dir, path)
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
        ]);
        let info = read(Some(&path));
        assert_eq!(info.state, "shown");
        let all = info
            .turns
            .iter()
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        for secret in [
            "abcdefghijklmnop",
            "1234567890abcdef",
            "eyJhbGciOiJIUzI1NiJ9",
        ] {
            // The message never prints the value (CodeQL: a secret in a log).
            assert!(
                !all.contains(secret),
                "a test secret leaked into the shown text"
            );
        }
        assert!(all.contains("•••"), "{all}");
        assert!(info.redacted >= 3, "{info:?}");
        assert_eq!(info.turns[0].speaker, "you");
        assert_eq!(info.turns[1].speaker, "assistant");
    }

    #[test]
    fn a_private_key_block_is_taken_out_whole() {
        let (_dir, path) = transcript(&[turn(
            "user",
            "here:\n-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXk\n-----END OPENSSH PRIVATE KEY-----\nthanks",
        )]);
        let text = &read(Some(&path)).turns[0].text;
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
    fn no_transcript_or_a_compressed_one_says_so_and_reads_nothing() {
        assert_eq!(read(None).state, "none");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rollout.jsonl.zst");
        std::fs::write(&path, b"x").unwrap();
        assert_eq!(read(Some(&path)).state, "compressed");
        assert_eq!(
            read(Some(&dir.path().join("gone.jsonl"))).state,
            "unreadable"
        );
    }

    #[test]
    fn off_in_settings_no_transcript_is_looked_up_or_read() {
        let (_dir, path) = transcript(&[turn("user", "hello")]);
        let mut looked = false;
        let info = answer(false, || {
            looked = true;
            Some(path.clone())
        });
        assert_eq!(info.state, "off");
        assert!(info.turns.is_empty());
        assert!(!looked, "off reads nothing");
        assert_eq!(answer(true, || Some(path.clone())).state, "shown");
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
        let _ = read(Some(&path));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
    }
}
