//! Grok Build's sessions (AJ-E1604 Req.3), read-only, from the documented
//! layout (research `docs/research/grok.md` §4):
//! `$GROK_HOME/sessions/<url-encoded cwd>/<session id>/`, with a long
//! folder name shortened (a slug and a hash) and the folder written in a
//! `.cwd` file beside its sessions. Each session's `summary.json` holds
//! its title, times, model and parent session; `signals.json` its token
//! counts.
//!
//! Only those descriptions are documented, not the field names (unknown):
//! fields are read by their likely names, and what isn't there is left
//! out, never guessed (a time from the file instead). `signals.json`
//! isn't read, so a Grok conversation's tokens are unknown (never zero).
//! A sub-agent's session (one with a parent) belongs to its parent and
//! isn't listed. A session's conversation file is its `summary.json`.

use std::path::{Path, PathBuf};

use juggler_adapter::conversation::ConversationSummary;

/// The sessions' folder in a home.
pub const SESSIONS_DIR: &str = "sessions";
/// A session's own record in its folder.
pub const SUMMARY_FILE: &str = "summary.json";
/// The file naming a shortened folder's working folder.
const CWD_FILE: &str = ".cwd";

/// The most bytes read from one record.
const MAX_READ: u64 = 4 * 1024 * 1024;

/// `%XX` sequences decoded (a folder name is the working folder,
/// URL-encoded); `None` when it isn't valid.
fn url_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let hex = text.get(at + 1..at + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The working folder a folder of sessions belongs to: its `.cwd` file
/// when there is one, else its name decoded (an absolute path).
#[must_use]
pub fn folder_of(dir: &Path) -> Option<PathBuf> {
    if let Ok(text) = std::fs::read_to_string(dir.join(CWD_FILE))
        && !text.trim().is_empty()
    {
        return Some(PathBuf::from(text.trim()));
    }
    let decoded = url_decode(&dir.file_name()?.to_string_lossy())?;
    decoded.starts_with('/').then(|| PathBuf::from(decoded))
}

/// A record's JSON, when it is there, small enough and an object.
fn summary(path: &Path) -> Option<serde_json::Value> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_READ {
        return None;
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .filter(serde_json::Value::is_object)
}

/// Whether a record names a parent session (a sub-agent's).
fn has_parent(summary: &serde_json::Value) -> bool {
    [
        "parentSessionId",
        "parent_session_id",
        "parentId",
        "parent_id",
        "parent",
    ]
    .iter()
    .any(|key| {
        summary
            .get(*key)
            .is_some_and(|value| !value.is_null() && value.as_str() != Some(""))
    })
}

/// Every top-level session's record in the home.
#[must_use]
pub fn conversation_files(home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(folders) = std::fs::read_dir(home.join(SESSIONS_DIR)) else {
        return out;
    };
    for folder in folders.flatten().filter(|entry| entry.path().is_dir()) {
        let Ok(sessions) = std::fs::read_dir(folder.path()) else {
            continue;
        };
        for session in sessions.flatten() {
            let record = session.path().join(SUMMARY_FILE);
            if record.is_file() && summary(&record).is_none_or(|value| !has_parent(&value)) {
                out.push(record);
            }
        }
    }
    out
}

/// A time in a record: epoch ms or seconds, or an ISO 8601 string.
fn time_of(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Number(number) => {
            let n = number.as_i64()?;
            Some(if n < 10_000_000_000 { n * 1000 } else { n })
        }
        serde_json::Value::String(text) => juggler_core::time::timestamp_ms(text),
        _ => None,
    }
}

fn first_time(record: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| record.get(*key).and_then(time_of))
}

/// When a file was made (else last changed), epoch ms.
fn born_ms(path: &Path) -> Option<i64> {
    let meta = std::fs::metadata(path).ok()?;
    let at = meta.created().or_else(|_| meta.modified()).ok()?;
    i64::try_from(at.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis()).ok()
}

/// What a session's record says. `None` when it isn't one, or is a
/// sub-agent's.
#[must_use]
pub fn read_conversation(path: &Path) -> Option<ConversationSummary> {
    if path.file_name()? != SUMMARY_FILE {
        return None;
    }
    let dir = path.parent()?;
    let id = dir.file_name()?.to_string_lossy().into_owned();
    if id.is_empty() || id.starts_with('-') {
        return None;
    }
    let record = summary(path)?;
    if has_parent(&record) {
        return None;
    }
    let title = record
        .get("title")
        .and_then(serde_json::Value::as_str)
        .and_then(|title| juggler_adapter::conversation::title_from(title, 120));
    Some(ConversationSummary {
        cwd: dir.parent().and_then(folder_of),
        git_branch: None,
        title,
        started_ms: first_time(
            &record,
            &["createdAt", "created_at", "startedAt", "started_at"],
        )
        .or_else(|| born_ms(dir)),
        last_ms: first_time(&record, &["updatedAt", "updated_at", "lastActiveAt"]),
        // `signals.json`'s fields aren't documented: unknown, never zero.
        tokens: juggler_core::model::TokenCounts::default(),
        id,
    })
}

/// The one top-level session in `home` that ran in `cwd`, started at or
/// after `since_ms` and not already linked: its record and id. Two is an
/// honest `None`, never a guess.
#[must_use]
pub fn locate(
    home: &Path,
    cwd: &Path,
    since_ms: i64,
    taken: &[PathBuf],
) -> Option<(PathBuf, String)> {
    let here: Vec<PathBuf> = conversation_files(home)
        .into_iter()
        .filter(|record| {
            record
                .parent()
                .and_then(Path::parent)
                .and_then(folder_of)
                .is_some_and(|folder| {
                    crate::claude_code::same_folder(&folder.to_string_lossy(), cwd)
                })
        })
        .collect();
    let found = crate::claude_code::only_since(here, since_ms, taken, |_| true)?;
    let id = found.parent()?.file_name()?.to_string_lossy().into_owned();
    Some((found, id))
}

/// The session id of any file under a session's folder (the watcher).
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

/// A folder's name as Grok Build writes it: the path, URL-encoded (the
/// fixtures' and the tests' form; long names aren't shortened here).
#[must_use]
pub fn folder_name(cwd: &Path) -> String {
    let mut out = String::new();
    for byte in cwd.to_string_lossy().bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(home: &Path, cwd: &Path, id: &str, record: &str) -> PathBuf {
        let dir = home.join(SESSIONS_DIR).join(folder_name(cwd)).join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(SUMMARY_FILE), record).unwrap();
        dir.join(SUMMARY_FILE)
    }

    #[test]
    fn a_session_reads_its_title_folder_and_times_and_a_sub_agent_is_left_out() {
        let home = tempfile::tempdir().unwrap();
        let cwd = Path::new("/w/my app");
        let top = session(
            home.path(),
            cwd,
            "0199aa00-0000-7000-8000-000000000001",
            r#"{"title":"Fix the cart","createdAt":"2026-10-21T10:00:00Z","updatedAt":1792580500000}"#,
        );
        session(
            home.path(),
            cwd,
            "0199aa00-0000-7000-8000-000000000002",
            r#"{"title":"sub","parentSessionId":"0199aa00-0000-7000-8000-000000000001"}"#,
        );
        assert_eq!(conversation_files(home.path()), vec![top.clone()]);
        let read = read_conversation(&top).unwrap();
        assert_eq!(read.id, "0199aa00-0000-7000-8000-000000000001");
        assert_eq!(read.title.as_deref(), Some("Fix the cart"));
        assert_eq!(read.cwd.as_deref(), Some(cwd));
        assert_eq!(read.last_ms, Some(1_792_580_500_000));
        assert_eq!(read.tokens, juggler_core::model::TokenCounts::default());
        assert_eq!(session_id_of(&top.with_file_name("updates.jsonl")), read.id);
    }

    #[test]
    fn a_shortened_folder_names_its_working_folder_in_a_cwd_file() {
        let home = tempfile::tempdir().unwrap();
        let dir = home.path().join(SESSIONS_DIR).join("long-name-3fa2c1");
        std::fs::create_dir_all(dir.join("s1")).unwrap();
        std::fs::write(dir.join(CWD_FILE), "/w/a/very/long/path\n").unwrap();
        std::fs::write(dir.join("s1").join(SUMMARY_FILE), r#"{"title":"x"}"#).unwrap();
        let read = read_conversation(&dir.join("s1").join(SUMMARY_FILE)).unwrap();
        assert_eq!(read.cwd.as_deref(), Some(Path::new("/w/a/very/long/path")));
        assert_eq!(url_decode("%2Fw%2Fapp").as_deref(), Some("/w/app"));
        assert_eq!(url_decode("%zz"), None);
    }
}
