//! AJ-E1529: Codex's conversations on disk: `sessions/Y/M/D/rollout-*.jsonl`
//! under its home, one per thread (a revert adds `…_<rollout>` files of the
//! same thread). Read-only.

use std::path::{Path, PathBuf};

use juggler_adapter::conversation::{ConversationSummary, SessionProcess, title_from, value_after};
use juggler_core::time::timestamp_ms;

use crate::codex_transcript::{RolloutParser, thread_id_from_name};

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
            path.file_name().is_some_and(|name| {
                let name = name.to_string_lossy();
                // AJ-E1540: Codex compresses older rollouts (`.jsonl.zst`).
                name.starts_with("rollout-")
                    && (name.ends_with(".jsonl") || name.ends_with(".jsonl.zst"))
            })
        }));
    }
    out
}

/// Reads one rollout: its thread id, folder, branch, title (the first
/// thing the user asked, not the context Codex injects), first and last
/// activity, and tokens (each response counted once).
#[must_use]
pub fn read_conversation(path: &Path) -> Option<ConversationSummary> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    let mut summary = Summary::default();
    crate::transcript::for_each_line(path, |index, line| {
        summary.line(index, line);
        true
    })
    .ok()?;
    summary.finish(&name)
}

/// QA batch 10: a large rollout's quick read, from its start and end
/// (`file_ends`): all but its tokens, which need the whole file.
#[must_use]
pub fn read_conversation_quick(path: &Path) -> Option<ConversationSummary> {
    if crate::transcript::is_compressed(path) {
        return compressed_head(path);
    }
    let ends = juggler_adapter::conversation::file_ends(path)?;
    let mut summary = summary_of(&ends, &path.file_name()?.to_string_lossy())?;
    summary.tokens = juggler_core::model::TokenCounts::default();
    Some(summary)
}

/// AJ-E1540: a compressed rollout's quick read: only its head is
/// decompressed (a compressed rollout is a finished one; reading all of
/// them whole would slow the first listing). Its last activity is its
/// start until the whole read fills it in.
fn compressed_head(path: &Path) -> Option<ConversationSummary> {
    use std::io::Read as _;
    let file = std::fs::File::open(path).ok()?;
    let mut head = Vec::new();
    zstd::stream::Decoder::new(file)
        .ok()?
        .take(juggler_adapter::conversation::QUICK_HEAD)
        .read_to_end(&mut head)
        .ok()?;
    head.truncate(
        head.iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |end| end + 1),
    );
    let mut summary = summary_of(&head, &path.file_name()?.to_string_lossy())?;
    summary.tokens = juggler_core::model::TokenCounts::default();
    Some(summary)
}

/// What a rollout's bytes (whole, or its two ends) say.
fn summary_of(bytes: &[u8], name: &str) -> Option<ConversationSummary> {
    let mut summary = Summary::default();
    for (index, line) in bytes.split(|byte| *byte == b'\n').enumerate() {
        summary.line(index, line);
    }
    summary.finish(name)
}

/// A rollout's summary, built a line at a time: what a conversation's row
/// needs and nothing more, so a rollout of any size reads in bounded
/// memory.
#[derive(Default)]
struct Summary {
    parser: Option<RolloutParser>,
    /// A line with anything on it.
    any: bool,
    /// The first `session_meta` seen (a subagent's says so there).
    meta_seen: bool,
    subagent: bool,
    /// From a `session_meta` in the first five lines.
    id: Option<String>,
    cwd: Option<String>,
    started_ms: Option<i64>,
    last_ms: Option<i64>,
    git_branch: Option<String>,
}

impl Summary {
    fn line(&mut self, index: usize, line: &[u8]) {
        if line.iter().all(u8::is_ascii_whitespace) {
            return;
        }
        self.any = true;
        let parser = self.parser.get_or_insert_with(|| RolloutParser::new(false));
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
            parser.line(index, line); // counted as damage
            return;
        };
        if let Some(at) = value
            .get("timestamp")
            .and_then(serde_json::Value::as_str)
            .and_then(timestamp_ms)
        {
            self.started_ms = Some(self.started_ms.map_or(at, |first| first.min(at)));
            self.last_ms = Some(self.last_ms.map_or(at, |last| last.max(at)));
        }
        if value.get("type").and_then(serde_json::Value::as_str) == Some("session_meta") {
            let field = |pointer: &str| {
                value
                    .pointer(pointer)
                    .and_then(serde_json::Value::as_str)
                    .filter(|text| !text.is_empty())
                    .map(str::to_owned)
            };
            if index < 5 {
                self.id = self.id.take().or_else(|| field("/payload/id"));
                self.cwd = self.cwd.take().or_else(|| field("/payload/cwd"));
            }
            if !self.meta_seen {
                self.meta_seen = true;
                self.subagent = value.pointer("/payload/source/subagent").is_some();
            }
            // `session_meta.payload.git.branch`, when Codex recorded the repo.
            if let Some(branch) = field("/payload/git/branch") {
                self.git_branch = Some(branch);
            }
        }
        parser.value(index, &value);
    }

    /// The summary; `None` for an empty file, or a subagent's rollout,
    /// which is part of the conversation that started it (as Claude Code's
    /// subagents are), not one of its own. Its usage is still the
    /// account's (0.1 QA 3.1).
    fn finish(self, name: &str) -> Option<ConversationSummary> {
        if !self.any || self.subagent {
            return None;
        }
        let parser = self.parser?;
        Some(ConversationSummary {
            id: self.id.or_else(|| thread_id_from_name(name))?,
            cwd: self.cwd.map(PathBuf::from),
            tokens: parser.usage(),
            title: parser
                .first_ask()
                .and_then(|text| title_from(text, TITLE_MAX)),
            started_ms: self.started_ms,
            last_ms: self.last_ms,
            git_branch: self.git_branch,
        })
    }
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
    fn a_compressed_rollout_is_a_conversation_too() {
        let home = tempfile::tempdir().unwrap();
        let day = home.path().join("sessions/2026/10/15");
        std::fs::create_dir_all(&day).unwrap();
        let id = "01a13e20-8ff6-7091-9ab8-75d6f59b8894";
        let rollout = [
            format!(r#"{{"timestamp":"2026-10-15T01:54:43.000Z","type":"session_meta","payload":{{"id":"{id}","cwd":"/Users/you/Dev/shop","git":{{"branch":"main"}}}}}}"#),
            r#"{"timestamp":"2026-10-15T01:55:00.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Fix the cart total"}]}}"#.to_owned(),
            r#"{"timestamp":"2026-10-15T02:10:00.000Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Fixed."}]}}"#.to_owned(),
        ]
        .join("\n")
            + "\n";
        let path = day.join(format!("rollout-2026-10-15T01-54-43-{id}.jsonl.zst"));
        std::fs::write(
            &path,
            zstd::stream::encode_all(rollout.as_bytes(), 3).unwrap(),
        )
        .unwrap();

        assert_eq!(conversation_files(home.path()), vec![path.clone()]);
        let whole = read_conversation(&path).expect("a compressed rollout reads");
        assert_eq!(whole.id, id);
        assert_eq!(whole.title.as_deref(), Some("Fix the cart total"));
        assert_eq!(whole.git_branch.as_deref(), Some("main"));
        let quick = read_conversation_quick(&path).expect("and reads quickly from its head");
        assert_eq!(
            (quick.id.as_str(), quick.title.as_deref()),
            (id, Some("Fix the cart total"))
        );
    }

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

    #[test]
    fn a_subagents_rollout_is_not_a_conversation_of_its_own() {
        let parent = r#"{"timestamp":"2026-10-21T10:00:00.000Z","type":"session_meta","payload":{"id":"0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b","cwd":"/w/api","source":"cli"}}"#;
        let child = r#"{"timestamp":"2026-10-21T10:01:00.000Z","type":"session_meta","payload":{"id":"0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5c","cwd":"/w/api","source":{"subagent":{"thread_spawn":{"parent_thread_id":"0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b"}}}}}"#;
        let ask = r#"{"timestamp":"2026-10-21T10:01:01.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"look into the flaky test"}]}}"#;
        assert!(summary_of(format!("{parent}\n{ask}\n").as_bytes(), "rollout-a.jsonl").is_some());
        assert!(summary_of(format!("{child}\n{ask}\n").as_bytes(), "rollout-b.jsonl").is_none());
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
