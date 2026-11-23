//! AJ-E2318 (the review's B2 and A1.10): which running processes write a
//! moving Claude conversation, by Claude Code's own evidence. Claude Code
//! appends to a history and closes it, so no open file says it writes;
//! two things do:
//!
//! - **its pid file**: each running session writes
//!   `<tool_dir>/sessions/<pid>.json` naming its `sessionId` and when its
//!   process started (`procStart`). The file counts only while that pid is
//!   still the process it names: its recorded start matches the live
//!   process's (a file left by a session that ended, its pid given to
//!   another program since, is stale). A file that records no start counts
//!   only for a process that is a Claude Code session;
//! - **`--resume`**: a live Claude Code process started to resume the
//!   conversation (`--resume=<id>`, as the Claude app's Code tab passes it,
//!   or `--resume <id>`).
//!
//! Only sessions of the moving family count: a session of another
//! conversation in the same folder writes none of its files.

use std::collections::HashMap;
use std::path::Path;

use juggler_adapter::live::same_start;
use juggler_adapter::moving::{ProcView, Writer, WriterEvidence};

/// The writers of `conversations` under `tool_dir` in `table`.
#[must_use]
pub fn of(tool_dir: &Path, conversations: &[String], table: &[ProcView<'_>]) -> Vec<Writer> {
    let by_pid: HashMap<u32, &ProcView<'_>> = table.iter().map(|p| (p.pid, p)).collect();
    let resume_of = |process: &ProcView<'_>| {
        crate::claude_conversations::session_process(process.exe, process.args)
            .and_then(|session| session.resumes)
    };
    let mut out: Vec<Writer> = Vec::new();
    for session in crate::claude_live::live_sessions(tool_dir) {
        let Some(process) = session.pid.and_then(|pid| by_pid.get(&pid)) else {
            continue;
        };
        let current = match session.started_secs {
            Some(_) => same_start(session.started_secs, process.start_secs),
            None => {
                crate::claude_conversations::session_process(process.exe, process.args).is_some()
            }
        };
        let resumed = resume_of(process);
        let ours = conversations.contains(&session.session_id)
            || resumed
                .as_ref()
                .is_some_and(|id| conversations.contains(id));
        if current && ours && !out.iter().any(|w| w.pid == process.pid) {
            out.push(Writer {
                pid: process.pid,
                start_secs: process.start_secs,
                conversation: Some(session.session_id),
                evidence: WriterEvidence::SessionFile,
            });
        }
    }
    for process in table {
        if out.iter().any(|w| w.pid == process.pid) {
            continue;
        }
        if let Some(id) = resume_of(process).filter(|id| conversations.contains(id)) {
            out.push(Writer {
                pid: process.pid,
                start_secs: process.start_secs,
                conversation: Some(id),
                evidence: WriterEvidence::ResumeArg,
            });
        }
    }
    out.sort_by_key(|w| w.pid);
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const CODE: &str = "/Users/you/Library/Application Support/ClaudeProfiles/work/claude-code/2.1/abc/claude.app/Contents/MacOS/claude";

    struct Row {
        pid: u32,
        exe: PathBuf,
        args: Vec<String>,
        start: i64,
    }

    fn row(pid: u32, exe: &str, args: &[&str], start: i64) -> Row {
        Row {
            pid,
            exe: PathBuf::from(exe),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            start,
        }
    }

    fn view(rows: &[Row]) -> Vec<ProcView<'_>> {
        rows.iter()
            .map(|r| ProcView {
                pid: r.pid,
                ppid: Some(1),
                exe: Some(&r.exe),
                args: &r.args,
                start_secs: Some(r.start),
            })
            .collect()
    }

    /// A pid file for `pid` on `session`, started at `start` (epoch
    /// seconds, written as Claude Code writes it: a ctime in UTC).
    fn pid_file(dir: &Path, pid: u32, session: &str, start: Option<&str>) {
        let sessions = dir.join("sessions");
        std::fs::create_dir_all(&sessions).unwrap();
        let start = start.map_or_else(String::new, |s| format!(r#","procStart":"{s}""#));
        std::fs::write(
            sessions.join(format!("{pid}.json")),
            format!(r#"{{"pid":{pid},"sessionId":"{session}","status":"idle"{start}}}"#),
        )
        .unwrap();
    }

    /// 2026-11-23 10:00:00 UTC.
    const AT: i64 = 1_795_428_000;
    const AT_WORDS: &str = "Mon Nov 23 10:00:00 2026";

    fn family() -> Vec<String> {
        vec!["main".into(), "kid".into()]
    }

    /// A session's pid file whose process is still the one it names
    /// writes the conversation: a writer, by its file.
    #[test]
    fn a_session_pid_file_names_a_writer() {
        let tmp = tempfile::tempdir().unwrap();
        pid_file(tmp.path(), 40, "kid", Some(AT_WORDS));
        let rows = [row(40, CODE, &["claude"], AT)];
        let found = of(tmp.path(), &family(), &view(&rows));
        assert_eq!(
            found,
            [Writer {
                pid: 40,
                start_secs: Some(AT),
                conversation: Some("kid".into()),
                evidence: WriterEvidence::SessionFile,
            }]
        );
    }

    /// A pid file left by a session that ended, its pid now another
    /// program's (its start differs), is stale: no writer.
    #[test]
    fn a_stale_pid_file_is_no_writer() {
        let tmp = tempfile::tempdir().unwrap();
        pid_file(tmp.path(), 40, "main", Some(AT_WORDS));
        let rows = [row(
            40,
            "/Applications/Safari.app/Contents/MacOS/Safari",
            &["Safari"],
            AT + 600,
        )];
        assert!(of(tmp.path(), &family(), &view(&rows)).is_empty());
        // Gone altogether: none either.
        assert!(of(tmp.path(), &family(), &[]).is_empty());
        // No start recorded: only a Claude Code session counts.
        pid_file(tmp.path(), 41, "main", None);
        let rows = [row(41, "/bin/zsh", &["-zsh"], AT)];
        assert!(of(tmp.path(), &family(), &view(&rows)).is_empty());
    }

    /// A live `claude` resuming a family member writes it, by either
    /// spelling; one resuming another conversation, or a session of
    /// another conversation in the folder, writes none of its files.
    #[test]
    fn a_resume_names_a_writer_of_its_conversation_only() {
        let tmp = tempfile::tempdir().unwrap();
        pid_file(tmp.path(), 52, "other", Some(AT_WORDS));
        let rows = [
            row(50, CODE, &["claude", "--resume=main"], AT),
            row(
                51,
                "/Users/you/.local/bin/claude",
                &["claude", "--resume", "kid"],
                AT,
            ),
            row(52, CODE, &["claude"], AT),
            row(
                53,
                "/Users/you/.local/bin/claude",
                &["claude", "--resume", "other"],
                AT,
            ),
            row(
                54,
                "/opt/homebrew/bin/node",
                &["node", "server.js", "--resume=main"],
                AT,
            ),
        ];
        let found = of(tmp.path(), &family(), &view(&rows));
        let pids: Vec<(u32, WriterEvidence)> = found.iter().map(|w| (w.pid, w.evidence)).collect();
        assert_eq!(
            pids,
            [
                (50, WriterEvidence::ResumeArg),
                (51, WriterEvidence::ResumeArg)
            ]
        );
    }
}
