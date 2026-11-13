//! AJ-E1529: Codex's conversations on disk: `sessions/Y/M/D/rollout-*.jsonl`
//! under its home, one per thread (a revert adds `…_<rollout>` files of the
//! same thread). Read-only.

use std::path::{Path, PathBuf};

use juggler_adapter::conversation::{ConversationSummary, SessionProcess, value_after};
use juggler_adapter::transcript_read::{TranscriptReader, UsageRead};
use juggler_core::time::timestamp_ms;

use crate::codex_transcript::{RolloutParser, thread_id_from_name};

/// The folder every rollout is under: `sessions`.
#[must_use]
pub fn conversation_roots(home: &Path) -> Vec<PathBuf> {
    vec![home.join("sessions")]
}

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

/// Reads one rollout whole: its thread id, folder, branch, title
/// ([`crate::codex_titles`]: never the context Codex or its app injects),
/// first and last activity, and tokens (each response once, as the usage
/// reading counts them, compactions included). Through the same reader
/// that follows a live rollout ([`transcript_reader`]).
#[must_use]
pub fn read_conversation(path: &Path) -> Option<ConversationSummary> {
    crate::transcript_follow::read_whole(path, transcript_reader(""))
}

/// 0.4.5 (AJ-E805): a rollout read as it grows: what it says about the
/// thread, and its usage (`quota_ingest`'s events with their carry,
/// counted under `account`), each line parsed once.
#[must_use]
pub fn transcript_reader(account: &str) -> Box<dyn TranscriptReader> {
    Box::new(Reader {
        account: account.to_owned(),
        summary: Summary::default(),
        carry: crate::quota_ingest::RolloutCarry::default(),
        index: 0,
    })
}

/// A rollout's reader ([`transcript_reader`]).
struct Reader {
    account: String,
    summary: Summary,
    carry: crate::quota_ingest::RolloutCarry,
    /// The next line's index in the file.
    index: usize,
}

impl TranscriptReader for Reader {
    fn read(&mut self, lines: &[u8]) -> UsageRead {
        let mut outcome = crate::quota_ingest::TailOutcome::default();
        // Whole lines, each through its newline: nothing follows the last.
        let lines = lines.strip_suffix(b"\n").unwrap_or(lines);
        for line in lines.split(|byte| *byte == b'\n') {
            let index = self.index;
            self.index += 1;
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) {
                self.summary.value(index, &value);
                crate::quota_ingest::rollout_value(
                    &self.account,
                    &value,
                    &mut self.carry,
                    &mut outcome,
                );
            } else {
                self.summary.line(index, line);
                outcome.parse_errors += 1;
            }
        }
        crate::quota_ingest::usage_read(outcome)
    }

    fn summary(&self, path: &Path) -> Option<ConversationSummary> {
        named(
            path,
            self.summary.summary(&path.file_name()?.to_string_lossy()),
        )
    }
}

/// The summary with Codex's own name for the thread as its title, when
/// Codex keeps one.
fn named(path: &Path, summary: Option<ConversationSummary>) -> Option<ConversationSummary> {
    let mut summary = summary?;
    if let Some(title) = crate::codex_titles::indexed(path, &summary.id) {
        summary.title = Some(title);
    }
    Some(summary)
}

/// QA batch 10: a large rollout's quick read, from its start and end
/// (`file_ends`): all but its tokens, which need the whole file.
#[must_use]
pub fn read_conversation_quick(path: &Path) -> Option<ConversationSummary> {
    if crate::transcript::is_compressed(path) {
        return compressed_head(path);
    }
    let ends = juggler_adapter::conversation::file_ends(path)?;
    let mut summary = named(
        path,
        quick_summary_of(&ends, &path.file_name()?.to_string_lossy()),
    )?;
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
    let mut summary = named(
        path,
        quick_summary_of(&head, &path.file_name()?.to_string_lossy()),
    )?;
    summary.tokens = juggler_core::model::TokenCounts::default();
    Some(summary)
}

/// A quick read's summary, only when its bytes hold the thread's first
/// `session_meta` (who started it: D-102 places a thread under the Codex
/// app or Codex CLI by it). Codex writes its whole instructions into that
/// line, so it can outgrow the quick read's head; then `None` sends the
/// caller to the whole read, plain or compressed alike, rather than list
/// the thread under the wrong App until then (0.4.4).
fn quick_summary_of(bytes: &[u8], name: &str) -> Option<ConversationSummary> {
    let summary = lines_of(bytes);
    summary.meta.as_ref()?;
    summary.summary(name)
}

/// What a rollout's bytes (whole, or its two ends) say, a line at a time.
fn lines_of(bytes: &[u8]) -> Summary {
    let mut summary = Summary::default();
    for (index, line) in bytes.split(|byte| *byte == b'\n').enumerate() {
        summary.line(index, line);
    }
    summary
}

/// Whether `originator` is the Codex app's: it writes "Codex Desktop", and
/// its work edition `codex_work_desktop` (ADR-0032, seen on the
/// maintainer's Mac); `codex_exec`, `codex_cli_rs` and AI Juggler's own
/// are a CLI App's.
fn desktop_originator(originator: &str) -> bool {
    originator.to_ascii_lowercase().contains("desktop")
}

/// 0.4.3 (D-102): the sources of a thread a Codex surface lists (the CLI's
/// `codex resume` picker, the Codex app): interactive ones. `exec` (a
/// script's `codex exec`), `mcp` and the rest are History only; a rollout
/// that names none is an older interactive one.
const LISTED_SOURCES: [&str; 2] = ["cli", "vscode"];

/// Whether the first `session_meta`'s `payload` is a script's run:
/// `codex exec`, by its source or its originator. AI Juggler's own
/// (`ai_juggler`, through `codex app-server`: source `vscode`) isn't.
fn script_run(payload: Option<&serde_json::Value>) -> bool {
    let word = |key: &str| {
        payload
            .and_then(|payload| payload.get(key))
            .and_then(serde_json::Value::as_str)
    };
    word("originator") == Some("codex_exec")
        || word("source").is_some_and(|source| !LISTED_SOURCES.contains(&source))
}

/// What a rollout's first `session_meta` says of its thread.
struct Meta {
    /// A subagent's rollout (part of the thread that started it).
    subagent: bool,
    /// 0.3.5: the Codex app, when it started the thread
    /// (`session_meta.payload.originator` is `Codex Desktop`).
    started_in: Option<String>,
    /// 0.4.3 (D-102): a script's run ([`script_run`]).
    by_script: bool,
}

/// A rollout's summary, built a line at a time: what a conversation's row
/// needs and nothing more, so a rollout of any size reads in bounded
/// memory.
#[derive(Default)]
struct Summary {
    parser: Option<RolloutParser>,
    /// A line with anything on it.
    any: bool,
    /// The first `session_meta`, once seen.
    meta: Option<Meta>,
    /// From a `session_meta` in the first five lines.
    id: Option<String>,
    cwd: Option<String>,
    started_ms: Option<i64>,
    last_ms: Option<i64>,
    git_branch: Option<String>,
    /// The thread pursues a goal (`thread_goal_updated`): its latest
    /// objective, empty when it has none.
    goal: Option<String>,
}

impl Summary {
    fn line(&mut self, index: usize, line: &[u8]) {
        if line.iter().all(u8::is_ascii_whitespace) {
            return;
        }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
            self.any = true;
            let parser = self.parser.get_or_insert_with(|| RolloutParser::new(false));
            parser.line(index, line); // counted as damage
            return;
        };
        self.value(index, &value);
    }

    /// One line that parsed.
    fn value(&mut self, index: usize, value: &serde_json::Value) {
        self.any = true;
        let parser = self.parser.get_or_insert_with(|| RolloutParser::new(false));
        if let Some(at) = value
            .get("timestamp")
            .and_then(serde_json::Value::as_str)
            .and_then(timestamp_ms)
        {
            self.started_ms = Some(self.started_ms.map_or(at, |first| first.min(at)));
            self.last_ms = Some(self.last_ms.map_or(at, |last| last.max(at)));
        }
        if value
            .pointer("/payload/type")
            .and_then(serde_json::Value::as_str)
            == Some("thread_goal_updated")
        {
            let objective = value
                .pointer("/payload/goal/objective")
                .and_then(serde_json::Value::as_str);
            self.goal = Some(objective.unwrap_or_default().to_owned());
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
            if self.meta.is_none() {
                self.meta = Some(Meta {
                    subagent: value.pointer("/payload/source/subagent").is_some(),
                    started_in: field("/payload/originator")
                        .is_some_and(|originator| desktop_originator(&originator))
                        .then(|| crate::codex_live::CODEX_APP_HOST.to_owned()),
                    by_script: script_run(value.get("payload")),
                });
            }
            // `session_meta.payload.git.branch`, when Codex recorded the repo.
            if let Some(branch) = field("/payload/git/branch") {
                self.git_branch = Some(branch);
            }
        }
        parser.value(index, value);
    }

    /// The summary so far, with no tokens (its usage's, [`Reader`]);
    /// `None` for an empty file, or a subagent's rollout, which is part of
    /// the conversation that started it (as Claude Code's subagents are),
    /// not one of its own. Its usage is still the account's (0.1 QA 3.1).
    fn summary(&self, name: &str) -> Option<ConversationSummary> {
        let (subagent, started_in, by_script) =
            self.meta.as_ref().map_or((false, None, false), |meta| {
                (meta.subagent, meta.started_in.clone(), meta.by_script)
            });
        if !self.any || subagent {
            return None;
        }
        let parser = self.parser.as_ref()?;
        let cwd = self.cwd.as_ref().map(PathBuf::from);
        let title = crate::codex_titles::from_rollout(&crate::codex_titles::Parts {
            first_ask: parser.first_ask(),
            goal: self.goal.is_some(),
            objective: self.goal.as_deref(),
            cwd: cwd.as_deref(),
        });
        Some(ConversationSummary {
            id: self.id.clone().or_else(|| thread_id_from_name(name))?,
            cwd,
            tokens: juggler_core::model::TokenCounts::default(),
            title,
            opening: parser.first_message().and_then(|text| {
                juggler_adapter::conversation::title_from(text, crate::codex_titles::TITLE_MAX)
            }),
            started_ms: self.started_ms,
            last_ms: self.last_ms,
            git_branch: self.git_branch.clone(),
            started_in,
            by_script,
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

    /// 0.4.5 (AJ-E805): a rollout read as it grows says what it says read
    /// whole, and its tokens are the usage reading's (P14, P17): reasoning
    /// and cache writes counted once each, a compaction's own usage
    /// counted, a repeated record once. The conversation list's own count
    /// had reasoning twice, cache writes inside the input too, and no
    /// compactions.
    #[test]
    fn a_rollout_read_as_it_grows_counts_tokens_as_the_usage_reading_does() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir
            .path()
            .join("sessions/2026/10/21/rollout-2026-10-21T10-00-00-0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b.jsonl");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let record = |response: &str| {
            format!(
                r#"{{"timestamp":"2026-10-21T10:00:09.000Z","type":"token_usage_record","payload":{{"thread_id":"t","turn_id":"1","response_id":"{response}","usage":{{"input_tokens":100,"cached_input_tokens":60,"cache_write_input_tokens":10,"output_tokens":20,"reasoning_output_tokens":5}}}}}}"#
            )
        };
        let lines = [
            r#"{"timestamp":"2026-10-21T10:00:00.000Z","type":"session_meta","payload":{"id":"0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b","cwd":"/w/api","source":"cli"}}"#.to_owned(),
            r#"{"timestamp":"2026-10-21T10:00:01.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"fix the flaky test"}]}}"#.to_owned(),
            record("r1"),
            record("r1"),
            r#"{"timestamp":"2026-10-21T10:00:10.000Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":120},"last_token_usage":{"input_tokens":100,"output_tokens":20}}}}"#.to_owned(),
            r#"{"timestamp":"2026-10-21T10:00:11.000Z","type":"compacted","payload":{"message":""}}"#.to_owned(),
            r#"{"timestamp":"2026-10-21T10:00:12.000Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":150},"last_token_usage":{"input_tokens":25,"output_tokens":5}}}}"#.to_owned(),
            record("r2"),
        ];
        let text = lines.join("\n") + "\n";
        std::fs::write(&path, &text).unwrap();
        let whole = read_conversation(&path).unwrap();
        // Two responses: input 100 - 60 cached - 10 written = 30, output
        // 20 - 5 reasoning = 15; and the compaction's own 25 in, 5 out.
        let t = whole.tokens;
        assert_eq!(
            (t.input, t.output, t.cache_read, t.cache_write, t.reasoning),
            (85, 35, 120, 20, 10)
        );
        let ends: Vec<usize> = text.match_indices('\n').map(|(at, _)| at + 1).collect();
        for cut in &ends {
            let mut file =
                juggler_adapter::transcript_read::ConversationFile::new(transcript_reader("acc"));
            let _ = file.read(&text.as_bytes()[..*cut]);
            let _ = file.read(&text.as_bytes()[*cut..]);
            assert_eq!(file.summary(&path).as_ref(), Some(&whole), "cut at {cut}");
        }
        let adapter = crate::codex::CodexAdapter::new();
        assert!(
            juggler_adapter::ToolTranscripts::transcript_reader(&adapter, &path, "a").is_some()
        );
        assert!(
            juggler_adapter::ToolTranscripts::transcript_reader(
                &adapter,
                &dir.path().join("session_index.jsonl"),
                "a"
            )
            .is_none()
        );
    }

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

    /// 0.4.4 (D-102): a rollout says who started it, plain or compressed,
    /// in its quick read as in its whole one: its first `session_meta`,
    /// whatever its size (Codex writes its whole instructions into it; 20 to
    /// 50 KB on the maintainer's Mac), so it is never listed under the
    /// wrong App. A first line longer than the quick read's head gives no
    /// quick read (the whole read follows), never one without it.
    #[test]
    fn a_rollout_says_who_started_it_however_long_its_first_line() {
        let home = tempfile::tempdir().unwrap();
        let day = home.path().join("sessions/2026/10/21");
        std::fs::create_dir_all(&day).unwrap();
        for (n, size) in [(1_u32, 40_000_usize), (2, 400_000)] {
            let id = format!("0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a{n:02}");
            let meta = format!(
                r#"{{"timestamp":"2026-10-21T10:00:00.000Z","type":"session_meta","payload":{{"id":"{id}","cwd":"/w/api","source":"vscode","originator":"Codex Desktop","base_instructions":{{"text":"{}"}}}}}}"#,
                "x".repeat(size)
            );
            let ask = r#"{"timestamp":"2026-10-21T10:00:02.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Speed up the import job"}]}}"#;
            let plain = day.join(format!("rollout-2026-10-21T10-00-00-{id}.jsonl"));
            std::fs::write(&plain, format!("{meta}\n{ask}\n")).unwrap();
            let packed = day.join(format!("rollout-2026-10-21T10-00-00-{id}.jsonl.zst"));
            std::fs::write(
                &packed,
                zstd::stream::encode_all(format!("{meta}\n{ask}\n").as_bytes(), 3).unwrap(),
            )
            .unwrap();
            let app = Some(crate::codex_live::CODEX_APP_HOST);
            for path in [&plain, &packed] {
                let whole = read_conversation(path).unwrap();
                assert_eq!(whole.started_in.as_deref(), app, "{size}: {path:?}");
                match read_conversation_quick(path) {
                    Some(quick) => assert_eq!(
                        (quick.id.as_str(), quick.started_in.as_deref()),
                        (id.as_str(), app),
                        "{size}: {path:?}"
                    ),
                    // A plain file this small is read whole anyway.
                    None => assert!(
                        size > 256 * 1024 || path == &plain,
                        "{size}: {path:?} reads quickly"
                    ),
                }
            }
        }
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
        assert_eq!(summary.started_in, None, "no originator: a terminal's");
        // ADR-0032: the work edition is the app too; exec and AI Juggler's
        // own are a CLI App's.
        assert!(desktop_originator("Codex Desktop"));
        assert!(desktop_originator("codex_work_desktop"));
        assert!(!desktop_originator("codex_exec"));
        assert!(!desktop_originator("ai_juggler"));
        assert_eq!(summary.last_ms.unwrap() - summary.started_ms.unwrap(), 9500);
        assert_eq!((summary.tokens.input, summary.tokens.output), (100, 20));
        assert_eq!(conversation_files(dir.path()), vec![path]);
        // Small: no quick read, the whole file is as quick.
        assert!(read_conversation_quick(&summary_path(&dir)).is_none());
    }

    /// 0.3.5: a thread the Codex app started is that app's, even when it
    /// isn't open (the maintainer's walk: 59 app threads read as a
    /// terminal's).
    #[test]
    fn a_thread_the_codex_app_started_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir
            .path()
            .join("rollout-2026-10-21T10-00-00-0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b.jsonl");
        std::fs::write(
            &path,
            [
                r#"{"timestamp":"2026-10-21T10:00:00.000Z","type":"session_meta","payload":{"id":"0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b","cwd":"/w/api","originator":"Codex Desktop"}}"#,
                r#"{"timestamp":"2026-10-21T10:00:02.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Speed up the import job"}]}}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        let summary = read_conversation(&path).unwrap();
        assert_eq!(
            summary.started_in.as_deref(),
            Some(crate::codex_live::CODEX_APP_HOST)
        );
    }

    /// 0.4.5: an untitled thread is named by its first prompt, as `codex
    /// resume` lists it: past the context Codex injects, one line, never
    /// a pasted key or a bare path (the next prompt names it then), and
    /// none at all when there's no prompt.
    #[test]
    fn a_rollout_is_named_by_its_first_prompt() {
        let dir = tempfile::tempdir().unwrap();
        let meta = r#"{"timestamp":"2026-11-13T08:00:00.000Z","type":"session_meta","payload":{"id":"019a7c11-2b3c-7d4e-8f5a-6b7c8d9e0f1a","cwd":"/w/shop","originator":"codex_cli_rs","source":"cli"}}"#;
        let user = |text: &str| {
            format!(
                r#"{{"timestamp":"2026-11-13T08:00:01.000Z","type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":{}}}]}}}}"#,
                serde_json::Value::from(text)
            )
        };
        let rollout = |name: &str, lines: &[String]| {
            let path = dir.path().join(format!(
                "rollout-2026-11-13T08-00-00-019a7c11-2b3c-7d4e-8f5a-6b7c8d9e0f1a-{name}.jsonl"
            ));
            let mut all = vec![meta.to_owned()];
            all.extend_from_slice(lines);
            std::fs::write(&path, all.join("\n")).unwrap();
            read_conversation(&path).unwrap()
        };
        let context = user("<environment_context>\n  <cwd>/w/shop</cwd>\n</environment_context>");
        let named = rollout(
            "a",
            &[
                user("# AGENTS.md instructions for /w/shop"),
                context.clone(),
                user("/Users/you/Downloads/cart.csv"),
                user("Round   the cart total\nto cents, then run the tests"),
            ],
        );
        assert_eq!(named.title.as_deref(), Some("Round the cart total"));
        assert_eq!(
            juggler_core::naming::short_name(
                None,
                named.title.as_deref(),
                None,
                &juggler_core::naming::Facts::default()
            ),
            "Round the cart total"
        );
        let unnamed = rollout("b", &[context, user("sk-test-0123456789abcdefXYZ")]);
        assert_eq!(unnamed.title, None);
        assert_eq!(
            juggler_core::naming::short_name(
                None,
                unnamed.title.as_deref(),
                None,
                &juggler_core::naming::Facts {
                    folder: Some("shop"),
                    started: Some("started 8:00 AM")
                }
            ),
            "shop · started 8:00 AM"
        );
    }

    /// The 0.1.5 review: a Codex app thread was titled with the block the
    /// app injects (`<external_codex_apps_…>`).
    #[test]
    fn an_app_block_never_titles_a_thread() {
        let meta = r#"{"timestamp":"2026-10-25T08:00:00.000Z","type":"session_meta","payload":{"id":"0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b","cwd":"/w/shop"}}"#;
        let user = |text: &str| {
            format!(
                r#"{{"timestamp":"2026-10-25T08:00:01.000Z","type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":{}}}]}}}}"#,
                serde_json::Value::from(text)
            )
        };
        let agents = user("# AGENTS.md instructions for /w/shop");
        let block = user(
            "<external_codex_apps_operation>{\"op\":\"goal\"}</external_codex_apps_operation>",
        );
        let ask = user("Speed up checkout. It takes 4 s.");
        let goal = |objective: &str| {
            format!(
                r#"{{"timestamp":"2026-10-25T08:00:02.000Z","type":"event_msg","payload":{{"type":"thread_goal_updated","threadId":"t","goal":{{"objective":{},"status":"active"}}}}}}"#,
                serde_json::Value::from(objective)
            )
        };
        let title = |lines: &[&str]| {
            lines_of(format!("{}\n", lines.join("\n")).as_bytes())
                .summary("rollout-a.jsonl")
                .unwrap()
                .title
        };
        // The first thing typed, to its first sentence.
        assert_eq!(
            title(&[meta, &agents, &block, &ask]).as_deref(),
            Some("Speed up checkout.")
        );
        // A goal's objective comes first.
        let objective = goal("Keep the nightly build green");
        assert_eq!(
            title(&[meta, &block, &objective, &ask]).as_deref(),
            Some("Keep the nightly build green")
        );
        // A goal with no words of its own: the goal and the project.
        let wordless = goal("");
        assert_eq!(
            title(&[meta, &block, &wordless]).as_deref(),
            Some("Codex goal · shop")
        );
        // Nothing typed and no goal: no title, never the block.
        assert_eq!(title(&[meta, &agents, &block]), None);
    }

    /// 0.4.3 (D-102): each kind of rollout a Codex home holds, as the
    /// reader says it: whose it is (`started_in`) and whether a Codex
    /// surface lists it (not `by_script`). Archived ones are moved out of
    /// `sessions/` (to `archived_sessions/`), so they're never listed.
    #[test]
    fn each_kind_of_rollout_says_whose_it_is_and_whether_a_surface_lists_it() {
        let home = tempfile::tempdir().unwrap();
        let day = home.path().join("sessions/2026/10/21");
        std::fs::create_dir_all(&day).unwrap();
        std::fs::create_dir_all(home.path().join("archived_sessions")).unwrap();
        let rollout = |at: &Path, n: u32, source: &str, originator: &str| {
            let id = format!("0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a{n:02}");
            let meta = format!(
                r#"{{"timestamp":"2026-10-21T10:00:00.000Z","type":"session_meta","payload":{{"id":"{id}","cwd":"/w/api","source":{source},"originator":"{originator}"}}}}"#
            );
            let ask = r#"{"timestamp":"2026-10-21T10:00:02.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Speed up the import job"}]}}"#;
            let path = at.join(format!("rollout-2026-10-21T10-00-00-{id}.jsonl"));
            std::fs::write(&path, format!("{meta}\n{ask}\n")).unwrap();
            path
        };
        let app = Some(crate::codex_live::CODEX_APP_HOST);
        let subagent = r#"{"subagent":{"thread_spawn":{"parent_thread_id":"p"}}}"#;
        for (n, source, originator, started_in, by_script) in [
            (1, r#""vscode""#, "Codex Desktop", app, false),
            (2, r#""vscode""#, "codex_work_desktop", app, false),
            (3, r#""vscode""#, "ai_juggler", None, false),
            (4, r#""cli""#, "codex_cli_rs", None, false),
            (5, r#""exec""#, "codex_exec", None, true),
            (6, r#""mcp""#, "codex_mcp", None, true),
            (7, r#""cli""#, "codex_exec", None, true),
        ] {
            let summary = read_conversation(&rollout(&day, n, source, originator)).unwrap();
            assert_eq!(
                (summary.started_in.as_deref(), summary.by_script),
                (started_in, by_script),
                "{source} {originator}"
            );
        }
        let child = rollout(&day, 8, subagent, "Codex Desktop");
        assert!(
            read_conversation(&child).is_none(),
            "a subagent's: its parent's"
        );
        rollout(
            &home.path().join("archived_sessions"),
            9,
            r#""vscode""#,
            "Codex Desktop",
        );
        let listed = conversation_files(home.path());
        assert_eq!(listed.len(), 8, "the archived one isn't in sessions/");
        assert!(
            !listed
                .iter()
                .any(|path| path.to_string_lossy().contains("4a09"))
        );
    }

    #[test]
    fn a_subagents_rollout_is_not_a_conversation_of_its_own() {
        let parent = r#"{"timestamp":"2026-10-21T10:00:00.000Z","type":"session_meta","payload":{"id":"0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b","cwd":"/w/api","source":"cli"}}"#;
        let child = r#"{"timestamp":"2026-10-21T10:01:00.000Z","type":"session_meta","payload":{"id":"0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5c","cwd":"/w/api","source":{"subagent":{"thread_spawn":{"parent_thread_id":"0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b"}}}}}"#;
        let ask = r#"{"timestamp":"2026-10-21T10:01:01.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"look into the flaky test"}]}}"#;
        assert!(
            lines_of(format!("{parent}\n{ask}\n").as_bytes())
                .summary("rollout-a.jsonl")
                .is_some()
        );
        assert!(
            lines_of(format!("{child}\n{ask}\n").as_bytes())
                .summary("rollout-b.jsonl")
                .is_none()
        );
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
