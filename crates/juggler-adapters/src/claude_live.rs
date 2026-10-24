//! AJ-E2101 (D-37): Claude Code's open sessions from its own files.
//!
//! - **The profile's pid files.** Claude Code writes
//!   `<profile>/sessions/<pid>.json` for each running session and removes it
//!   on exit. Of each, only `pid`, `sessionId`, `cwd`, `status`,
//!   `procStart`, `hostSessionId`, `entrypoint`, `kind` and `name` are
//!   read. The `*.key` files beside them are never opened.
//! - **The Claude app's session state.** Each instance of the Claude app
//!   (one per `--user-data-dir`) keeps
//!   `<instance>/claude-code-sessions/<account>/<org>/local_<id>.json`. Of
//!   each, only `cliSessionId`, `cwd`, `title`, `isArchived`,
//!   `lastActivityAt`, `lastFocusedAt`, `postTurnSummary.status_category`
//!   and `spawnedFrom.sessionId` are read: never `needs_action`,
//!   `status_detail` or any other summary text, which serde skips
//!   without keeping (the structs below name every field kept).
//!
//! Both formats are undocumented: a file of another shape is left out.

use std::path::{Path, PathBuf};

use juggler_adapter::live::{
    AppSession, HostApp, HostKind, LiveSession, LiveStatus, TurnOutcome, ctime_utc_secs, read_small,
};

/// The largest pid file read (they are under 1 KB).
const PID_FILE_MAX: u64 = 16 * 1024;
/// The largest app session file read (they are tens of KB).
const APP_FILE_MAX: u64 = 2 * 1024 * 1024;
/// The most app session files read per instance.
const APP_FILES_MAX: usize = 2_000;

/// A pid file, the fields kept.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PidFile {
    pid: Option<u64>,
    session_id: Option<String>,
    cwd: Option<String>,
    status: Option<String>,
    proc_start: Option<String>,
    host_session_id: Option<String>,
    entrypoint: Option<String>,
    kind: Option<String>,
    name: Option<String>,
}

/// The pid files' folder in a profile.
#[must_use]
pub fn sessions_dir(profile_dir: &Path) -> PathBuf {
    profile_dir.join("sessions")
}

/// The pid files in a profile: `<digits>.json` only.
#[must_use]
pub fn pid_files(profile_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(sessions_dir(profile_dir)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "json")
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| {
                        !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_digit())
                    })
        })
        .collect()
}

/// One pid file, when it is one: its pid must be its file's name.
#[must_use]
pub fn read_pid_file(path: &Path) -> Option<LiveSession> {
    let text = read_small(path, PID_FILE_MAX)?;
    let file: PidFile = serde_json::from_str(&text).ok()?;
    let named: u32 = path.file_stem()?.to_str()?.parse().ok()?;
    let pid = u32::try_from(file.pid?).ok()?;
    if pid != named {
        return None;
    }
    let session_id = file.session_id.filter(|id| !id.is_empty())?;
    let status = match file.status.as_deref() {
        Some("busy") => LiveStatus::Busy,
        Some("idle") => LiveStatus::Idle,
        _ => LiveStatus::Unknown,
    };
    let background = file
        .kind
        .as_deref()
        .is_some_and(|kind| kind != "interactive")
        || file
            .entrypoint
            .as_deref()
            .is_some_and(|entry| entry.starts_with("sdk"));
    Some(LiveSession {
        session_id,
        pid: Some(pid),
        started_secs: file.proc_start.as_deref().and_then(ctime_utc_secs),
        cwd: file.cwd.filter(|cwd| !cwd.is_empty()).map(PathBuf::from),
        status,
        host_session_id: file.host_session_id.filter(|id| !id.is_empty()),
        entrypoint: file.entrypoint,
        name: file.name.filter(|name| !name.is_empty()),
        background,
    })
}

/// Every pid file in a profile that reads.
#[must_use]
pub fn live_sessions(profile_dir: &Path) -> Vec<LiveSession> {
    pid_files(profile_dir)
        .iter()
        .filter_map(|path| read_pid_file(path))
        .collect()
}

/// The Claude app's last-turn summary: its status word, nothing else.
#[derive(serde::Deserialize)]
struct TurnSummary {
    status_category: Option<String>,
}

/// Where a session was started from: the parent's id, nothing else.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpawnedFrom {
    session_id: Option<String>,
}

/// An app session file, the fields kept.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppFile {
    cli_session_id: Option<String>,
    cwd: Option<String>,
    title: Option<String>,
    is_archived: Option<bool>,
    last_activity_at: Option<i64>,
    last_focused_at: Option<i64>,
    post_turn_summary: Option<TurnSummary>,
    spawned_from: Option<SpawnedFrom>,
}

/// The app session files of one Claude app instance.
#[must_use]
pub fn app_files(instance_dir: &Path) -> Vec<PathBuf> {
    let root = instance_dir.join("claude-code-sessions");
    let mut out = Vec::new();
    let dirs = |dir: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                    .map(|entry| entry.path())
                    .collect()
            })
            .unwrap_or_default()
    };
    for account in dirs(&root) {
        for org in dirs(&account) {
            let Ok(entries) = std::fs::read_dir(&org) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with("local_") && name.ends_with(".json") {
                    out.push(path);
                    if out.len() >= APP_FILES_MAX {
                        return out;
                    }
                }
            }
        }
    }
    out
}

/// One app session file, when it reads.
#[must_use]
pub fn read_app_file(path: &Path) -> Option<AppSession> {
    let host_session_id = path.file_stem()?.to_str()?.to_owned();
    let text = read_small(path, APP_FILE_MAX)?;
    let file: AppFile = serde_json::from_str(&text).ok()?;
    Some(AppSession {
        host_session_id,
        session_id: file.cli_session_id.filter(|id| !id.is_empty()),
        cwd: file.cwd.filter(|cwd| !cwd.is_empty()).map(PathBuf::from),
        title: file.title.filter(|title| !title.trim().is_empty()),
        archived: file.is_archived.unwrap_or(false),
        last_activity_ms: file.last_activity_at,
        last_focused_ms: file.last_focused_at,
        outcome: file
            .post_turn_summary
            .and_then(|summary| summary.status_category)
            .as_deref()
            .and_then(TurnOutcome::from_word),
        spawned_from: file
            .spawned_from
            .and_then(|from| from.session_id)
            .filter(|id| !id.is_empty()),
    })
}

/// Every app session file of an instance that reads.
#[must_use]
pub fn app_sessions(instance_dir: &Path) -> Vec<AppSession> {
    app_files(instance_dir)
        .iter()
        .filter_map(|path| read_app_file(path))
        .collect()
}

/// The Claude app, as data: one bundle id for every instance, told apart
/// by `--user-data-dir` (so Go to it activates the instance's process,
/// never the bundle id).
#[must_use]
pub fn host_apps() -> Vec<HostApp> {
    vec![HostApp {
        id: "claude-app".to_owned(),
        name: "Claude".to_owned(),
        bundle: "Claude.app".to_owned(),
        bundle_id: Some("com.anthropic.claudefordesktop".to_owned()),
        kind: HostKind::Desktop,
        instance_arg: Some("--user-data-dir".to_owned()),
        default_instance: Some("Library/Application Support/Claude".to_owned()),
        thread_link: None,
        folder_link: None,
        folder_command: None,
        tab_script: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "SUMMARY-TEXT-NEVER-KEPT";

    fn pid_file(dir: &Path, pid: u32, body: &str) -> PathBuf {
        let path = sessions_dir(dir).join(format!("{pid}.json"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn a_pid_file_reads_its_session_and_the_key_file_is_never_listed() {
        let dir = tempfile::tempdir().unwrap();
        pid_file(
            dir.path(),
            4242,
            r#"{"pid":4242,"sessionId":"s-1","cwd":"/w/p","status":"busy","procStart":"Sat Oct 24 08:12:34 2026","hostSessionId":"local_abc","entrypoint":"claude-desktop","kind":"interactive","name":"fix","messagingSocketPath":"/tmp/x","peerFeatures":[],"version":"2"}"#,
        );
        std::fs::write(sessions_dir(dir.path()).join("4242.key"), "key").unwrap();
        std::fs::write(sessions_dir(dir.path()).join("notes.json"), "{}").unwrap();
        assert_eq!(pid_files(dir.path()).len(), 1);
        let live = live_sessions(dir.path());
        assert_eq!(live.len(), 1);
        let session = &live[0];
        assert_eq!(session.session_id, "s-1");
        assert_eq!(session.pid, Some(4242));
        assert_eq!(session.started_secs, Some(1_792_829_554));
        assert_eq!(session.status, LiveStatus::Busy);
        assert_eq!(session.host_session_id.as_deref(), Some("local_abc"));
        assert_eq!(session.cwd.as_deref(), Some(Path::new("/w/p")));
        assert!(!session.background);
    }

    #[test]
    fn a_pid_file_of_another_shape_or_pid_is_left_out() {
        let dir = tempfile::tempdir().unwrap();
        // Its pid isn't its name.
        pid_file(dir.path(), 1, r#"{"pid":2,"sessionId":"s"}"#);
        // Not JSON.
        pid_file(dir.path(), 3, "not json");
        // A field of another type.
        pid_file(dir.path(), 4, r#"{"pid":"4","sessionId":"s"}"#);
        // No session id.
        pid_file(dir.path(), 5, r#"{"pid":5}"#);
        assert!(live_sessions(dir.path()).is_empty());
        // An unknown status and a new start format read as unknown, not stale.
        pid_file(
            dir.path(),
            6,
            r#"{"pid":6,"sessionId":"s","status":"thinking","procStart":"2026-10-24T08:12:34Z","kind":"print"}"#,
        );
        let live = live_sessions(dir.path());
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].status, LiveStatus::Unknown);
        assert_eq!(live[0].started_secs, None);
        assert!(
            live[0].background,
            "a non-interactive session is a background run"
        );
    }

    fn app_file(instance: &Path, id: &str, body: &str) -> PathBuf {
        let path = instance
            .join("claude-code-sessions/acct/org")
            .join(format!("{id}.json"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn an_app_session_keeps_its_status_word_and_never_its_summary_text() {
        let instance = tempfile::tempdir().unwrap();
        app_file(
            instance.path(),
            "local_abc",
            &format!(
                r#"{{"sessionId":"local_abc","cliSessionId":"s-1","cwd":"/w/p","title":"Fix login","isArchived":false,"lastActivityAt":1792829554000,"lastFocusedAt":1792829000000,"postTurnSummary":{{"status_category":"blocked","needs_action":"{SECRET}","status_detail":"{SECRET}","summarizes_uuid":"u"}},"spawnedFrom":{{"sessionId":"local_parent","title":"{SECRET}","name":"n","taskId":"t","sideSession":false}},"lastTurnReport":"{SECRET}","pendingSystemReminder":"{SECRET}"}}"#
            ),
        );
        let sessions = app_sessions(instance.path());
        assert_eq!(sessions.len(), 1);
        let session = &sessions[0];
        assert_eq!(session.host_session_id, "local_abc");
        assert_eq!(session.session_id.as_deref(), Some("s-1"));
        assert_eq!(session.title.as_deref(), Some("Fix login"));
        assert_eq!(session.outcome, Some(TurnOutcome::Blocked));
        assert_eq!(session.last_focused_ms, Some(1_792_829_000_000));
        assert_eq!(session.spawned_from.as_deref(), Some("local_parent"));
        assert!(
            !format!("{session:?}").contains(SECRET),
            "no summary text is kept"
        );
    }

    #[test]
    fn app_session_words_map_and_other_shapes_are_left_out() {
        let instance = tempfile::tempdir().unwrap();
        app_file(
            instance.path(),
            "local_a",
            r#"{"cliSessionId":"a","postTurnSummary":{"status_category":"review_ready"},"isArchived":true}"#,
        );
        app_file(
            instance.path(),
            "local_b",
            r#"{"cliSessionId":"b","postTurnSummary":{"status_category":"something_new"}}"#,
        );
        // A field of another shape: the file is left out.
        app_file(
            instance.path(),
            "local_c",
            r#"{"cliSessionId":"c","lastFocusedAt":"yesterday"}"#,
        );
        app_file(instance.path(), "other", r#"{"cliSessionId":"d"}"#);
        let mut sessions = app_sessions(instance.path());
        sessions.sort_by(|a, b| a.host_session_id.cmp(&b.host_session_id));
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].outcome, Some(TurnOutcome::ReviewReady));
        assert!(sessions[0].archived);
        assert_eq!(sessions[1].outcome, None);
    }
}
