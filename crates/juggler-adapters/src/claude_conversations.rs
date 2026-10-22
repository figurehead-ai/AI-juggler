//! AJ-E1529: Claude Code's conversations on disk. Each one is a
//! `projects/<encoded cwd>/<sessionId>.jsonl` under the config folder
//! (subagents' transcripts sit one level deeper and are part of their
//! parent, so they're not listed). Read-only.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use juggler_adapter::conversation::{ConversationSummary, SessionProcess, title_from, value_after};
use juggler_core::model::TokenCounts;
use juggler_core::time::timestamp_ms;

/// The longest title kept.
const TITLE_MAX: usize = 80;

/// Every `projects/*/*.jsonl` under `state_dir`.
#[must_use]
pub fn conversation_files(state_dir: &Path) -> Vec<PathBuf> {
    let Ok(folders) = std::fs::read_dir(state_dir.join("projects")) else {
        return Vec::new();
    };
    folders
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|folder| std::fs::read_dir(folder.path()).ok())
        .flat_map(Iterator::flatten)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl") && path.is_file())
        .collect()
}

/// The first text of a user message: a plain string, or the first text
/// block. Tool results and the CLI's own command echoes aren't the user
/// asking anything.
fn user_text(message: &serde_json::Value) -> Option<&str> {
    let content = message.get("content")?;
    let text = match content {
        serde_json::Value::String(text) => text.as_str(),
        serde_json::Value::Array(blocks) => blocks
            .iter()
            .find(|block| block.get("type").and_then(serde_json::Value::as_str) == Some("text"))?
            .get("text")?
            .as_str()?,
        _ => return None,
    };
    let trimmed = text.trim_start();
    (!trimmed.is_empty() && !trimmed.starts_with('<')).then_some(text)
}

/// Reads one conversation: its id, folder, branch, title, first and last
/// activity, and tokens (each assistant message counted once, by its
/// message id: the CLI writes a message's lines more than once).
#[must_use]
pub fn read_conversation(path: &Path) -> Option<ConversationSummary> {
    let file = std::fs::File::open(path).ok()?;
    summary_of(BufReader::new(file).lines().map_while(Result::ok), path)
}

/// QA batch 10: a large transcript's quick read, from its start and end
/// (`file_ends`): all but its tokens, which need the whole file.
#[must_use]
pub fn read_conversation_quick(path: &Path) -> Option<ConversationSummary> {
    let ends = juggler_adapter::conversation::file_ends(path)?;
    let text = String::from_utf8_lossy(&ends);
    let mut summary = summary_of(text.lines().map(str::to_owned), path)?;
    summary.tokens = TokenCounts::default();
    Some(summary)
}

/// What a transcript's lines (all, or its two ends) say.
fn summary_of(lines_in: impl Iterator<Item = String>, path: &Path) -> Option<ConversationSummary> {
    let mut summary = ConversationSummary::default();
    let mut tool_title: Option<String> = None;
    let mut first_ask: Option<String> = None;
    let mut usage: HashMap<String, TokenCounts> = HashMap::new();
    let mut unnamed = 0_u64;
    let mut lines = 0_u64;
    for line in lines_in {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue; // a damaged or half-written line
        };
        lines += 1;
        let str_of = |key: &str| value.get(key).and_then(serde_json::Value::as_str);
        if str_of("type") == Some("summary") {
            tool_title = str_of("summary").and_then(|text| title_from(text, TITLE_MAX));
            continue;
        }
        if summary.id.is_empty()
            && let Some(id) = str_of("sessionId")
        {
            id.clone_into(&mut summary.id);
        }
        if summary.cwd.is_none()
            && let Some(cwd) = str_of("cwd")
        {
            summary.cwd = Some(PathBuf::from(cwd));
        }
        if let Some(branch) = str_of("gitBranch").filter(|branch| !branch.is_empty()) {
            summary.git_branch = Some(branch.to_owned());
        }
        if let Some(at) = str_of("timestamp").and_then(timestamp_ms) {
            summary.started_ms = Some(summary.started_ms.map_or(at, |first| first.min(at)));
            summary.last_ms = Some(summary.last_ms.map_or(at, |last| last.max(at)));
        }
        let sidechain = value
            .get("isSidechain")
            .and_then(serde_json::Value::as_bool)
            == Some(true);
        let Some(message) = value.get("message") else {
            continue;
        };
        match str_of("type") {
            Some("user") if first_ask.is_none() && !sidechain => {
                first_ask = user_text(message).and_then(|text| title_from(text, TITLE_MAX));
            }
            Some("assistant") => {
                if let Some(counts) = message.get("usage").map(counts_of) {
                    let key = message
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .map_or_else(
                            || {
                                unnamed += 1;
                                format!("#{unnamed}")
                            },
                            str::to_owned,
                        );
                    usage.insert(key, counts);
                }
            }
            _ => {}
        }
    }
    if lines == 0 {
        return None;
    }
    if summary.id.is_empty() {
        summary.id = path.file_stem()?.to_string_lossy().into_owned();
    }
    summary.title = tool_title.or(first_ask);
    for counts in usage.values() {
        summary.tokens.input += counts.input;
        summary.tokens.output += counts.output;
        summary.tokens.cache_read += counts.cache_read;
        summary.tokens.cache_write += counts.cache_write;
    }
    Some(summary)
}

fn counts_of(usage: &serde_json::Value) -> TokenCounts {
    let n = |key: &str| {
        usage
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    };
    TokenCounts {
        input: n("input_tokens"),
        output: n("output_tokens"),
        cache_read: n("cache_read_input_tokens"),
        cache_write: n("cache_creation_input_tokens"),
        reasoning: 0,
    }
}

/// A `claude` CLI (the binary, or the npm package run by node) is a
/// session, wherever it's installed: the desktop app runs each of its Code
/// sessions as one, from `…/claude-code/<version>/…/claude.app/Contents/
/// MacOS/claude`. The app's own executable is `Claude`, which isn't one.
/// `--resume <id>` (or `-r`) names its conversation.
///
/// The binary is multi-call: Claude Code runs its search helpers as the
/// same `claude` executable under another name (`ugrep`, `rg`), and they
/// can outlive their session under a stray shell (QA, the real setup). So
/// a session's program name (`argv[0]`) is `claude` too. The same rule as
/// main's `claude_code::is_cli_process` (#126; its test
/// `a_claude_run_as_its_search_helper_is_not_a_session`): one copy once
/// the branches meet.
#[must_use]
pub fn session_process(exe: Option<&Path>, args: &[String]) -> Option<SessionProcess> {
    let named_claude = args.first().is_none_or(|program| {
        Path::new(program)
            .file_name()
            .is_some_and(|name| name == "claude")
    });
    let cli = (named_claude
        && exe.is_some_and(|exe| exe.file_name().is_some_and(|name| name == "claude")))
        || args
            .iter()
            .any(|arg| arg.contains("@anthropic-ai/claude-code"));
    cli.then(|| SessionProcess {
        resumes: value_after(args, &["--resume", "-r"]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, lines: &[&str]) -> PathBuf {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        path
    }

    #[test]
    fn a_conversation_reads_its_id_folder_branch_title_times_and_tokens_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "projects/-w-proj/abc.jsonl",
            &[
                r#"{"type":"summary","summary":"Fix the flaky login test","leafUuid":"x"}"#,
                r#"{"type":"user","sessionId":"s-1","cwd":"/w/proj","gitBranch":"main","timestamp":"2026-10-21T10:00:00.000Z","message":{"role":"user","content":"why does login fail?"}}"#,
                r#"{"type":"assistant","sessionId":"s-1","cwd":"/w/proj","gitBranch":"fix/login","timestamp":"2026-10-21T10:00:05.000Z","message":{"id":"m1","usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":100,"cache_creation_input_tokens":7}}}"#,
                r#"{"type":"assistant","sessionId":"s-1","timestamp":"2026-10-21T10:00:06.000Z","message":{"id":"m1","usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":100,"cache_creation_input_tokens":7}}}"#,
                "not json",
            ],
        );
        let summary = read_conversation(&path).unwrap();
        assert_eq!(summary.id, "s-1");
        assert_eq!(summary.cwd.as_deref(), Some(Path::new("/w/proj")));
        assert_eq!(summary.git_branch.as_deref(), Some("fix/login"));
        assert_eq!(summary.title.as_deref(), Some("Fix the flaky login test"));
        assert_eq!(summary.last_ms.unwrap() - summary.started_ms.unwrap(), 6000);
        assert_eq!(
            (
                summary.tokens.input,
                summary.tokens.output,
                summary.tokens.cache_read
            ),
            (10, 5, 100)
        );
        assert_eq!(conversation_files(dir.path()), vec![path]);
    }

    #[test]
    fn without_a_summary_the_title_is_the_first_real_ask() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "projects/-w/x.jsonl",
            &[
                r#"{"type":"user","sessionId":"s","message":{"role":"user","content":"<command-name>/clear</command-name>"}}"#,
                r#"{"type":"user","sessionId":"s","isSidechain":true,"message":{"role":"user","content":"a subagent's prompt"}}"#,
                r#"{"type":"user","sessionId":"s","message":{"role":"user","content":[{"type":"text","text":"Add pagination\nto orders"}]}}"#,
            ],
        );
        assert_eq!(
            read_conversation(&path).unwrap().title.as_deref(),
            Some("Add pagination")
        );
    }

    #[test]
    fn a_cli_is_a_session_wherever_installed_and_names_what_it_resumes() {
        let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
        let cli = Path::new("/usr/local/bin/claude");
        assert_eq!(
            session_process(Some(cli), &args(&["claude"])),
            Some(SessionProcess { resumes: None })
        );
        assert_eq!(
            session_process(Some(cli), &args(&["claude", "--resume", "s-1"]))
                .unwrap()
                .resumes
                .as_deref(),
            Some("s-1")
        );
        let node = Path::new("/usr/local/bin/node");
        assert!(
            session_process(
                Some(node),
                &args(&["node", "/n/@anthropic-ai/claude-code/cli.js"])
            )
            .is_some()
        );
        // The desktop app's Code sessions run a bundled CLI: sessions.
        let bundled = Path::new(
            "/Users/u/Library/Application Support/Claude/claude-code/2.1.0/ab12/claude.app/Contents/MacOS/claude",
        );
        assert_eq!(
            session_process(Some(bundled), &args(&["claude", "-r", "s-2"]))
                .unwrap()
                .resumes
                .as_deref(),
            Some("s-2")
        );
        // The app itself is not one.
        let app = Path::new("/Applications/Claude.app/Contents/MacOS/Claude");
        assert!(session_process(Some(app), &args(&["Claude"])).is_none());
        assert!(session_process(Some(node), &args(&["node", "server.js"])).is_none());
        // Its search helpers: the same binary under another name (QA, the
        // real setup: `ugrep` left running under a stray shell).
        for helper in ["ugrep", "rg", "/x/ugrep"] {
            assert!(
                session_process(Some(bundled), &args(&[helper, "-r", "pattern", "."])).is_none(),
                "{helper}"
            );
        }
        // Started by its full path, it's still `claude`.
        assert!(session_process(Some(bundled), &args(&[bundled.to_str().unwrap()])).is_some());
    }

    #[test]
    fn an_empty_file_is_not_a_conversation_and_subagents_are_not_listed() {
        let dir = tempfile::tempdir().unwrap();
        let empty = write(dir.path(), "projects/-w/e.jsonl", &[""]);
        write(
            dir.path(),
            "projects/-w/s/subagents/agent-1.jsonl",
            &[r#"{"type":"user"}"#],
        );
        assert!(read_conversation(&empty).is_none());
        assert_eq!(conversation_files(dir.path()), vec![empty]);
    }
}
