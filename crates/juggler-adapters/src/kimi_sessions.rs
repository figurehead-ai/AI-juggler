//! Kimi Code's sessions (AJ-E1603 Req.2), read-only, from the documented
//! layout (research `docs/research/kimi.md` §4):
//!
//! - `$KIMI_CODE_HOME/session_index.jsonl`: one line per session, with
//!   `sessionId`, `sessionDir` and `workDir`;
//! - `sessions/<workDirKey>/<sessionId>/state.json`: its title,
//!   `lastPrompt` and times; `agents/main/wire.jsonl` beside it holds the
//!   conversation itself.
//!
//! Only those names are documented; the rest of each schema isn't (unknown).
//! Fields are read by their documented names (`snake_case` accepted too),
//! and what isn't there is left out, never guessed: a time missing from
//! `state.json` comes from the file. `wire.jsonl` isn't read, so a Kimi
//! conversation's tokens are unknown (never zero). A session's
//! conversation file, for the index and for linking, is its `state.json`:
//! it is there from the session's start and changes with each prompt.

use std::path::{Path, PathBuf};

use juggler_adapter::conversation::ConversationSummary;

/// The session list in a home.
pub const INDEX_FILE: &str = "session_index.jsonl";
/// The sessions' folder in a home.
pub const SESSIONS_DIR: &str = "sessions";
/// A session's own record in its folder.
pub const STATE_FILE: &str = "state.json";

/// The most bytes read from the session list or one session's record.
const MAX_READ: u64 = 16 * 1024 * 1024;

/// One line of `session_index.jsonl`.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct IndexLine {
    /// Kimi Code's id for the session (what `--session` takes).
    #[serde(rename = "sessionId", alias = "session_id", default)]
    pub session_id: String,
    /// Its folder, as the index records it.
    #[serde(rename = "sessionDir", alias = "session_dir", default)]
    pub session_dir: Option<String>,
    /// The folder it ran in.
    #[serde(rename = "workDir", alias = "work_dir", default)]
    pub work_dir: Option<String>,
}

/// A file's text, when it is there and no bigger than [`MAX_READ`].
fn read_small(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_READ {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// Every session the home's list names (lines that don't read are
/// skipped).
#[must_use]
pub fn index(home: &Path) -> Vec<IndexLine> {
    read_small(&home.join(INDEX_FILE))
        .map(|text| {
            text.lines()
                .filter_map(|line| serde_json::from_str::<IndexLine>(line).ok())
                .filter(|line| !line.session_id.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Every session's record in the home (`sessions/<key>/<id>/state.json`).
#[must_use]
pub fn conversation_files(home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(keys) = std::fs::read_dir(home.join(SESSIONS_DIR)) else {
        return out;
    };
    for key in keys.flatten().filter(|entry| entry.path().is_dir()) {
        let Ok(sessions) = std::fs::read_dir(key.path()) else {
            continue;
        };
        for session in sessions.flatten() {
            let state = session.path().join(STATE_FILE);
            if state.is_file() {
                out.push(state);
            }
        }
    }
    out
}

/// The home a session's record is in: `<home>/sessions/<key>/<id>/state.json`.
fn home_of(state: &Path) -> Option<&Path> {
    state.parent()?.parent()?.parent()?.parent()
}

/// A time in `state.json`: epoch ms or seconds, or an ISO 8601 string.
fn time_of(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Number(number) => {
            let n = number.as_i64()?;
            // Seconds until 2286; anything bigger is milliseconds.
            Some(if n < 10_000_000_000 { n * 1000 } else { n })
        }
        serde_json::Value::String(text) => juggler_core::time::timestamp_ms(text),
        _ => None,
    }
}

/// The first of `keys` that `state` has as a time.
fn first_time(state: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| state.get(*key).and_then(time_of))
}

/// When a file was made (else last changed), epoch ms.
fn born_ms(path: &Path) -> Option<i64> {
    let meta = std::fs::metadata(path).ok()?;
    let at = meta.created().or_else(|_| meta.modified()).ok()?;
    i64::try_from(at.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis()).ok()
}

/// What a session's record (`state.json`) says, with its folder from the
/// home's list. `None` when it isn't a session's record.
#[must_use]
pub fn read_conversation(path: &Path) -> Option<ConversationSummary> {
    if path.file_name()? != STATE_FILE {
        return None;
    }
    let dir = path.parent()?;
    let id = dir.file_name()?.to_string_lossy().into_owned();
    if id.is_empty() || id.starts_with('-') {
        return None;
    }
    let state: serde_json::Value = read_small(path)
        .and_then(|text| serde_json::from_str(&text).ok())
        .filter(serde_json::Value::is_object)?;
    let text = |key: &str| state.get(key).and_then(serde_json::Value::as_str);
    let title = text("title")
        .filter(|title| !title.trim().is_empty())
        .or_else(|| text("lastPrompt").or_else(|| text("last_prompt")))
        .and_then(|title| juggler_adapter::conversation::title_from(title, 120));
    let cwd = home_of(path).and_then(|home| {
        index(home)
            .into_iter()
            .find(|line| line.session_id == id)
            .and_then(|line| line.work_dir)
            .map(PathBuf::from)
    });
    Some(ConversationSummary {
        cwd,
        git_branch: None,
        title,
        started_ms: first_time(
            &state,
            &["createdAt", "created_at", "startedAt", "started_at"],
        )
        .or_else(|| born_ms(dir)),
        last_ms: first_time(&state, &["updatedAt", "updated_at", "lastActiveAt"]),
        // `wire.jsonl`'s schema is undocumented: unknown, never zero.
        tokens: juggler_core::model::TokenCounts::default(),
        id,
    })
}

/// The one session in `home` that ran in `cwd`, started at or after
/// `since_ms` and not already linked (`taken`): its record and id. Two is
/// an honest `None`, never a guess.
#[must_use]
pub fn locate(
    home: &Path,
    cwd: &Path,
    since_ms: i64,
    taken: &[PathBuf],
) -> Option<(PathBuf, String)> {
    let records: Vec<PathBuf> = conversation_files(home);
    let here: Vec<PathBuf> = index(home)
        .into_iter()
        .filter(|line| {
            line.work_dir
                .as_deref()
                .is_some_and(|dir| crate::claude_code::same_folder(dir, cwd))
        })
        .filter_map(|line| {
            records
                .iter()
                .find(|record| {
                    record
                        .parent()
                        .and_then(Path::file_name)
                        .is_some_and(|name| name.to_string_lossy() == line.session_id)
                })
                .cloned()
        })
        .collect();
    let found = crate::claude_code::only_since(here, since_ms, taken, |_| true)?;
    let id = found.parent()?.file_name()?.to_string_lossy().into_owned();
    Some((found, id))
}

/// The session id of a record path (for the watcher).
#[must_use]
pub fn session_id_of(path: &Path) -> String {
    path.ancestors()
        .find(|dir| {
            dir.parent()
                .and_then(Path::parent)
                .and_then(Path::file_name)
                .is_some_and(|name| name == SESSIONS_DIR)
        })
        .and_then(Path::file_name)
        .map_or_else(
            || crate::claude_code::file_stem(path),
            |name| name.to_string_lossy().into_owned(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(home: &Path, id: &str, cwd: &str, state: &str) -> PathBuf {
        let dir = home.join(SESSIONS_DIR).join("wd_app_0123456789ab").join(id);
        std::fs::create_dir_all(dir.join("agents/main")).unwrap();
        std::fs::write(dir.join(STATE_FILE), state).unwrap();
        std::fs::write(dir.join("agents/main/wire.jsonl"), "{}\n").unwrap();
        let line = format!(
            "{{\"sessionId\":\"{id}\",\"sessionDir\":\"{}\",\"workDir\":\"{cwd}\"}}\n",
            dir.display()
        );
        let index = home.join(INDEX_FILE);
        let before = std::fs::read_to_string(&index).unwrap_or_default();
        std::fs::write(index, before + &line).unwrap();
        dir.join(STATE_FILE)
    }

    #[test]
    fn a_session_reads_its_title_folder_and_times_and_no_tokens() {
        let home = tempfile::tempdir().unwrap();
        let path = session(
            home.path(),
            "ses-1",
            "/w/app",
            r#"{"title":"Fix the cart","lastPrompt":"and the tax","createdAt":"2026-10-21T10:00:00Z","updatedAt":1792580500000}"#,
        );
        assert_eq!(conversation_files(home.path()), vec![path.clone()]);
        let read = read_conversation(&path).unwrap();
        assert_eq!(read.id, "ses-1");
        assert_eq!(read.title.as_deref(), Some("Fix the cart"));
        assert_eq!(read.cwd.as_deref(), Some(Path::new("/w/app")));
        assert_eq!(
            read.started_ms,
            juggler_core::time::timestamp_ms("2026-10-21T10:00:00Z")
        );
        assert_eq!(read.last_ms, Some(1_792_580_500_000));
        assert_eq!(read.tokens, juggler_core::model::TokenCounts::default());
        assert_eq!(session_id_of(&path), "ses-1");
        assert_eq!(
            session_id_of(&path.parent().unwrap().join("agents/main/wire.jsonl")),
            "ses-1"
        );
    }

    #[test]
    fn an_untitled_session_takes_its_last_prompt_and_a_broken_record_is_none() {
        let home = tempfile::tempdir().unwrap();
        let path = session(
            home.path(),
            "ses-2",
            "/w/app",
            r#"{"lastPrompt":"  hello\nworld"}"#,
        );
        let read = read_conversation(&path).unwrap();
        assert_eq!(read.title.as_deref(), Some("hello"));
        assert!(read.started_ms.is_some(), "the folder's time stands in");
        assert_eq!(read.last_ms, None, "the index takes the file's");
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(read_conversation(&path), None);
        assert_eq!(read_conversation(&home.path().join("x.json")), None);
    }
}
