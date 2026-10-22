//! AJ-E1529: Codex's conversations on disk: `sessions/Y/M/D/rollout-*.jsonl`
//! under its home, one per thread (a revert adds `…_<rollout>` files of the
//! same thread). Read-only.

use std::path::{Path, PathBuf};

use juggler_adapter::conversation::{
    ConversationSummary, SessionProcess, timestamp_ms, title_from, value_after,
};

use crate::claude_transcript::MessageRole;
use crate::codex_transcript::{parse_rollout, rollout_cwd, rollout_thread_id};

/// The longest title kept.
const TITLE_MAX: usize = 80;

/// Every `sessions/*/*/*/rollout-*.jsonl` under `home`.
#[must_use]
pub fn conversation_files(home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut level = vec![home.join("sessions")];
    for _ in 0..3 {
        level = level
            .iter()
            .filter_map(|dir| std::fs::read_dir(dir).ok())
            .flat_map(Iterator::flatten)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .map(|entry| entry.path())
            .collect();
    }
    for day in level {
        let Ok(entries) = std::fs::read_dir(&day) else {
            continue;
        };
        out.extend(entries.flatten().map(|entry| entry.path()).filter(|path| {
            path.extension().is_some_and(|ext| ext == "jsonl")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("rollout-"))
        }));
    }
    out
}

/// Reads one rollout: its thread id, folder, branch, title (the first
/// thing the user asked, not the context Codex injects), first and last
/// activity, and tokens (each response counted once).
#[must_use]
pub fn read_conversation(path: &Path) -> Option<ConversationSummary> {
    let bytes = std::fs::read(path).ok()?;
    summary_of(&bytes, &path.file_name()?.to_string_lossy())
}

/// QA batch 10: a large rollout's quick read, from its start and end
/// (`file_ends`): all but its tokens, which need the whole file.
#[must_use]
pub fn read_conversation_quick(path: &Path) -> Option<ConversationSummary> {
    let ends = juggler_adapter::conversation::file_ends(path)?;
    let mut summary = summary_of(&ends, &path.file_name()?.to_string_lossy())?;
    summary.tokens = juggler_core::model::TokenCounts::default();
    Some(summary)
}

/// What a rollout's bytes (whole, or its two ends) say.
fn summary_of(bytes: &[u8], name: &str) -> Option<ConversationSummary> {
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return None;
    }
    let parsed = parse_rollout(bytes);
    let mut summary = ConversationSummary {
        id: rollout_thread_id(bytes, name)?,
        cwd: rollout_cwd(bytes).map(PathBuf::from),
        tokens: parsed.usage,
        title: parsed
            .messages
            .iter()
            .find(|message| message.role == MessageRole::User)
            .and_then(|message| title_from(&message.text, TITLE_MAX)),
        ..ConversationSummary::default()
    };
    for line in String::from_utf8_lossy(bytes).lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(at) = value
            .get("timestamp")
            .and_then(serde_json::Value::as_str)
            .and_then(timestamp_ms)
        {
            summary.started_ms = Some(summary.started_ms.map_or(at, |first| first.min(at)));
            summary.last_ms = Some(summary.last_ms.map_or(at, |last| last.max(at)));
        }
        // `session_meta.payload.git.branch`, when Codex recorded the repo.
        if value.get("type").and_then(serde_json::Value::as_str) == Some("session_meta")
            && let Some(branch) = value
                .pointer("/payload/git/branch")
                .and_then(serde_json::Value::as_str)
                .filter(|branch| !branch.is_empty())
        {
            summary.git_branch = Some(branch.to_owned());
        }
    }
    Some(summary)
}

/// Codex subcommands that aren't a session: the desktop app's servers
/// and its command sandbox, MCP, sign-in and the like.
const HELPERS: &[&str] = &[
    "app-server",
    "exec-server",
    "sandbox",
    "debug",
    "mcp",
    "mcp-server",
    "proto",
    "login",
    "logout",
    "completion",
    "apply",
];

/// A `codex` CLI (the binary, or the npm package run by node), wherever
/// it's installed, is a session unless it runs one of its helper
/// subcommands (an app's `app-server`, `exec-server`, …: told by what
/// it does, not by its path). `codex resume <id>` names its conversation.
#[must_use]
pub fn session_process(exe: Option<&Path>, args: &[String]) -> Option<SessionProcess> {
    let cli = exe.is_some_and(|exe| exe.file_name().is_some_and(|name| name == "codex"))
        || args.iter().any(|arg| arg.contains("@openai/codex"));
    // Anywhere in its arguments: options come first (`codex -c key=value
    // app-server`), so the first bare word can be an option's value.
    let helper = args
        .iter()
        .skip(1)
        .any(|arg| HELPERS.contains(&arg.as_str()));
    (cli && !helper).then(|| SessionProcess {
        resumes: value_after(args, &["resume"]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cli_session_is_told_from_the_apps_helpers() {
        let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
        let cli = Path::new("/opt/homebrew/bin/codex");
        assert_eq!(
            session_process(Some(cli), &args(&["codex"])),
            Some(SessionProcess { resumes: None })
        );
        assert_eq!(
            session_process(Some(cli), &args(&["codex", "resume", "0199a2b3"]))
                .unwrap()
                .resumes
                .as_deref(),
            Some("0199a2b3")
        );
        // ChatGPT's own Codex servers are the app's, not sessions.
        let app = Path::new("/Applications/ChatGPT.app/Contents/Resources/codex");
        assert!(session_process(Some(app), &args(&["codex", "app-server"])).is_none());
        assert!(session_process(Some(cli), &args(&["codex", "exec-server"])).is_none());
        assert!(session_process(Some(cli), &args(&["codex", "login"])).is_none());
        // A session the app runs from its bundle is a session; the app
        // itself isn't one.
        assert!(session_process(Some(app), &args(&["codex", "resume", "0199a2b3"])).is_some());
        // The app's helpers as it runs them: options before the subcommand.
        let bundled = Path::new(
            "/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
        );
        for helper in [
            &[
                "codex",
                "-c",
                "features.code_mode_host=true",
                "app-server",
                "--analytics-default-enabled",
            ][..],
            &[
                "codex",
                "-c",
                "forced_login_method=\"chatgpt\"",
                "exec-server",
                "--remote",
                "https://example.invalid",
            ][..],
            &[
                "codex",
                "sandbox",
                "-c",
                "shell_environment_policy.inherit=\"all\"",
            ][..],
            &["codex", "app-server", "--listen", "stdio://"][..],
        ] {
            assert!(
                session_process(Some(bundled), &args(helper)).is_none(),
                "{helper:?}"
            );
        }
        let chatgpt = Path::new("/Applications/ChatGPT.app/Contents/MacOS/ChatGPT");
        assert!(session_process(Some(chatgpt), &args(&["ChatGPT"])).is_none());
    }

    #[test]
    fn a_rollout_reads_its_thread_folder_branch_ask_times_and_tokens() {
        let dir = tempfile::tempdir().unwrap();
        let day = dir.path().join("sessions/2026/10/21");
        std::fs::create_dir_all(&day).unwrap();
        let path =
            day.join("rollout-2026-10-21T10-00-00-0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b.jsonl");
        std::fs::write(
            &path,
            [
                r#"{"timestamp":"2026-10-21T10:00:00.000Z","type":"session_meta","payload":{"id":"0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b","cwd":"/w/api","git":{"branch":"main"}}}"#,
                r#"{"timestamp":"2026-10-21T10:00:01.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>cwd</environment_context>"}]}}"#,
                r#"{"timestamp":"2026-10-21T10:00:02.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Speed up the import job"}]}}"#,
                r#"{"timestamp":"2026-10-21T10:00:09.000Z","type":"token_usage_record","payload":{"thread_id":"t","turn_id":"1","response_id":"r1","usage":{"input_tokens":100,"output_tokens":20}}}"#,
                r#"{"timestamp":"2026-10-21T10:00:09.500Z","type":"token_usage_record","payload":{"thread_id":"t","turn_id":"1","response_id":"r1","usage":{"input_tokens":100,"output_tokens":20}}}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        let summary = read_conversation(&path).unwrap();
        assert_eq!(summary.id, "0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b");
        assert_eq!(summary.cwd.as_deref(), Some(Path::new("/w/api")));
        assert_eq!(summary.git_branch.as_deref(), Some("main"));
        assert_eq!(summary.title.as_deref(), Some("Speed up the import job"));
        assert_eq!(summary.last_ms.unwrap() - summary.started_ms.unwrap(), 9500);
        assert_eq!((summary.tokens.input, summary.tokens.output), (100, 20));
        assert_eq!(conversation_files(dir.path()), vec![path]);
        // Small: no quick read, the whole file is as quick.
        assert!(read_conversation_quick(&summary_path(&dir)).is_none());
    }

    fn summary_path(dir: &tempfile::TempDir) -> std::path::PathBuf {
        dir.path()
            .join("sessions/2026/10/21/rollout-2026-10-21T10-00-00-0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b.jsonl")
    }

    /// QA batch 10: a large rollout's quick read takes its thread, folder,
    /// branch, ask and first time from its start and its last activity
    /// from its end, and leaves its tokens to the whole read.
    #[test]
    fn a_large_rollouts_quick_read_takes_its_ends() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir
            .path()
            .join("rollout-2026-10-21T10-00-00-0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b.jsonl");
        let mut text = [
            r#"{"timestamp":"2026-10-21T10:00:00.000Z","type":"session_meta","payload":{"id":"0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b","cwd":"/w/api","git":{"branch":"main"}}}"#,
            r#"{"timestamp":"2026-10-21T10:00:02.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Speed up the import job"}]}}"#,
        ]
        .join("\n");
        text.push('\n');
        for n in 0..3_000 {
            text.push_str(&format!(
                r#"{{"timestamp":"2026-10-21T11:00:00.000Z","type":"token_usage_record","payload":{{"thread_id":"t","turn_id":"{n}","response_id":"r{n}","usage":{{"input_tokens":10,"output_tokens":1}}}},"pad":"{}"}}"#,
                "x".repeat(200)
            ));
            text.push('\n');
        }
        text.push_str(r#"{"timestamp":"2026-10-21T17:30:00.000Z","type":"event_msg","payload":{"type":"agent_message","message":"done"}}"#);
        text.push('\n');
        std::fs::write(&path, &text).unwrap();
        let quick = read_conversation_quick(&path).unwrap();
        let whole = read_conversation(&path).unwrap();
        assert_eq!(quick.id, whole.id);
        assert_eq!(
            (quick.cwd.clone(), quick.git_branch.clone()),
            (whole.cwd.clone(), whole.git_branch.clone())
        );
        assert_eq!(quick.title.as_deref(), Some("Speed up the import job"));
        assert_eq!(
            (quick.started_ms, quick.last_ms),
            (whole.started_ms, whole.last_ms)
        );
        assert_eq!(quick.tokens, juggler_core::model::TokenCounts::default());
        assert_eq!((whole.tokens.input, whole.tokens.output), (30_000, 3_000));
    }
}
