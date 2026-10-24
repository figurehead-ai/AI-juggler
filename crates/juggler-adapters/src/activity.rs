//! AJ-E1703: what a session is working on, read from its transcript for
//! a pool's context cards — the files it changed most recently, and the
//! step its own task list says is in progress.
//!
//! Each tool's parser hands its tool calls here
//! (`claude_transcript::activity_line`, `codex_transcript::activity_line`);
//! [`crate::tool_summary`] says which files a call changed and which task
//! list it set. Nothing else is kept: no message, no command, no output,
//! and a path only as its last two parts.

use crate::transcript::{TranscriptFormat, line_format};

/// How many recently changed files are kept, newest first.
pub const FILES_KEPT: usize = 5;

/// The most characters of the step in progress kept.
pub const STEP_CHARS: usize = 140;

/// What one transcript says its session is working on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityLog {
    /// The files changed most recently, newest first, each once.
    files: Vec<String>,
    /// The step the session's own task list has in progress.
    step: Option<String>,
    /// Whether the session has set a task list at all.
    has_plan: bool,
    /// Decided by the first parsable line.
    format: Option<TranscriptFormat>,
}

impl ActivityLog {
    /// Reads whole lines (`\n`-separated); a malformed line is skipped.
    pub fn feed(&mut self, bytes: &[u8]) {
        for line in bytes.split(|b| *b == b'\n') {
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
                continue;
            };
            let format = *self.format.get_or_insert_with(|| line_format(&value));
            match format {
                TranscriptFormat::ClaudeCode => {
                    crate::claude_transcript::activity_line(self, &value)
                }
                TranscriptFormat::CodexRollout => {
                    crate::codex_transcript::activity_line(self, &value);
                }
            }
        }
    }

    /// The files changed most recently, newest first (at most
    /// [`FILES_KEPT`]).
    #[must_use]
    pub fn files(&self) -> &[String] {
        &self.files
    }

    /// The step the session's own task list has in progress, on one line.
    #[must_use]
    pub fn step(&self) -> Option<&str> {
        self.step.as_deref()
    }

    /// Whether the session has set a task list at all (so "no step in
    /// progress" is something it said, not something unknown).
    #[must_use]
    pub fn has_plan(&self) -> bool {
        self.has_plan
    }

    /// A tool call, as the tool's parser read it.
    pub(crate) fn call(&mut self, name: &str, input: &serde_json::Value) {
        for file in crate::tool_summary::edited_files(name, input) {
            self.files.retain(|kept| *kept != file);
            self.files.insert(0, file);
        }
        self.files.truncate(FILES_KEPT);
        if let Some(items) = crate::tool_summary::task_list(name, input) {
            self.has_plan = true;
            self.step = items
                .iter()
                .find(|item| item.in_progress)
                .map(|item| one_line(&item.text));
        }
    }
}

impl crate::transcript_tail::TailLog for ActivityLog {
    fn feed(&mut self, bytes: &[u8]) {
        Self::feed(self, bytes);
    }
}

/// Reads a transcript as it grows, into an [`ActivityLog`].
pub type ActivityTail = crate::transcript_tail::TranscriptTail<ActivityLog>;

/// The first line of a step, cut to [`STEP_CHARS`].
fn one_line(text: &str) -> String {
    let first = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    let mut cut: String = first.chars().take(STEP_CHARS).collect();
    if first.chars().count() > STEP_CHARS {
        cut.push('…');
    }
    cut
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{ActivityLog, ActivityTail, FILES_KEPT};

    fn claude_call(name: &str, input: &str) -> String {
        format!(
            r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"tool_use","id":"t","name":"{name}","input":{input}}}]}}}}"#
        )
    }

    fn codex_call(name: &str, arguments: &str) -> String {
        let arguments = serde_json::to_string(arguments).unwrap();
        format!(
            r#"{{"timestamp":"2026-10-24T00:00:00Z","type":"response_item","payload":{{"type":"function_call","name":"{name}","call_id":"c","arguments":{arguments}}}}}"#
        )
    }

    #[test]
    fn claude_edits_and_its_task_list_are_read() {
        let lines = [
            claude_call("Read", r#"{"file_path":"/Users/you/shop/README.md"}"#),
            claude_call("Edit", r#"{"file_path":"/Users/you/shop/src/calc.py"}"#),
            claude_call(
                "Write",
                r#"{"file_path":"/Users/you/shop/tests/test_calc.py"}"#,
            ),
            claude_call(
                "TodoWrite",
                r#"{"todos":[{"content":"Add sub","status":"completed"},{"content":"Add mul\nwith tests","status":"in_progress"}]}"#,
            ),
        ];
        let mut log = ActivityLog::default();
        log.feed(lines.join("\n").as_bytes());
        assert_eq!(
            log.files(),
            ["tests/test_calc.py", "src/calc.py"],
            "reading isn't changing"
        );
        assert_eq!(log.step(), Some("Add mul"));
        assert!(log.has_plan());
    }

    #[test]
    fn codex_patches_and_its_plan_are_read() {
        let patch = "*** Begin Patch\n*** Update File: /w/shop/src/calc.py\n*** Add File: /w/shop/src/mul.py\n*** End Patch";
        let lines = [
            r#"{"timestamp":"2026-10-24T00:00:00Z","type":"session_meta","payload":{"id":"x"}}"#
                .to_owned(),
            format!(
                r#"{{"timestamp":"2026-10-24T00:00:00Z","type":"response_item","payload":{{"type":"custom_tool_call","name":"apply_patch","call_id":"p","input":{}}}}}"#,
                serde_json::to_string(patch).unwrap()
            ),
            codex_call(
                "update_plan",
                r#"{"plan":[{"step":"Write mul","status":"in_progress"}]}"#,
            ),
        ];
        let mut log = ActivityLog::default();
        log.feed(lines.join("\n").as_bytes());
        assert_eq!(log.files(), ["src/mul.py", "src/calc.py"]);
        assert_eq!(log.step(), Some("Write mul"));
    }

    #[test]
    fn the_newest_files_are_kept_each_once_and_a_finished_plan_has_no_step() {
        let mut lines: Vec<String> = (0..8)
            .map(|n| claude_call("Edit", &format!(r#"{{"file_path":"/w/f{n}.rs"}}"#)))
            .collect();
        lines.push(claude_call("Edit", r#"{"file_path":"/w/f3.rs"}"#));
        lines.push(claude_call(
            "TodoWrite",
            r#"{"todos":[{"content":"All","status":"completed"}]}"#,
        ));
        let mut log = ActivityLog::default();
        log.feed(lines.join("\n").as_bytes());
        assert_eq!(log.files().len(), FILES_KEPT);
        assert_eq!(log.files()[0], "w/f3.rs");
        assert_eq!(log.files().iter().filter(|f| *f == "w/f3.rs").count(), 1);
        assert_eq!(log.step(), None);
        assert!(log.has_plan(), "it said: nothing in progress");
    }

    #[test]
    fn nothing_read_says_nothing() {
        let mut log = ActivityLog::default();
        log.feed(b"not json\n{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}\n");
        assert!(log.files().is_empty());
        assert_eq!(log.step(), None);
        assert!(!log.has_plan());
    }

    #[test]
    fn a_tail_reads_only_what_was_appended() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        std::fs::write(
            &path,
            format!("{}\n", claude_call("Edit", r#"{"file_path":"/w/a.rs"}"#)),
        )
        .unwrap();
        let mut tail = ActivityTail::default();
        assert!(tail.read(&path).unwrap());
        assert!(!tail.read(&path).unwrap(), "nothing new");
        let more = format!(
            "{}\n{}\n",
            claude_call("Edit", r#"{"file_path":"/w/a.rs"}"#),
            claude_call("Edit", r#"{"file_path":"/w/b.rs"}"#)
        );
        let mut all = std::fs::read_to_string(&path).unwrap();
        all.push_str(&more);
        std::fs::write(&path, all).unwrap();
        assert!(tail.read(&path).unwrap());
        assert_eq!(tail.log().files(), ["w/b.rs", "w/a.rs"]);
    }
}
