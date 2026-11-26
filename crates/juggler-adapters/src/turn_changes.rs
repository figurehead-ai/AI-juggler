//! AJ-E3713 R6a (0.5.2 G8): what a turn changed, for Home's feed: the
//! files its own edit and write calls touched and the lines they added
//! and removed, read from the session's own transcript (never git in the
//! user's project: nothing here runs a command).
//!
//! The turn is the last one that began at or before `until` (a feed
//! item's time): the calls after the last thing the user asked, up to
//! then. What each tool's calls say:
//!
//! - Claude Code: `Edit` (its `old_string` and `new_string`, line by
//!   line), `MultiEdit` (each of its edits), `Write` (a new file's lines,
//!   by its result's `type: "create"`). A call's result, when written,
//!   is the better word: its `structuredPatch` counts a `replace_all` and
//!   a `Write` over a file that was there; a failed call changed nothing.
//!   `NotebookEdit` says which notebook only, and a `Write` whose result
//!   isn't read says nothing of what the file held: files only.
//! - Codex: `apply_patch` (its patch's `+` and `-` lines, file by file);
//!   a deleted file's lines aren't in its patch: files only.
//!
//! A turn with one call that can't count shows its files only: a sum
//! that leaves a change out would claim less than happened.

use std::collections::HashMap;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;

use juggler_adapter::subagents::LinesChanged;

use crate::transcript::{TranscriptFormat, is_compressed, line_format};

/// The windows read back from the end, in turn, until one holds the
/// turn's start.
pub const WINDOWS: [u64; 3] = [256 * 1024, 2 * 1024 * 1024, MAX_WINDOW];

/// The most bytes read: a turn longer than this is not told (its start
/// isn't found, so a count would leave its first calls out).
pub const MAX_WINDOW: u64 = 8 * 1024 * 1024;

/// A line written this long after `until` still belongs to the turn (the
/// feed's item is recorded as the turn ends, a moment before its last
/// lines can be).
pub const SLACK_MS: i64 = 3_000;

/// The most file names kept (the count goes on).
pub const NAMES_MAX: usize = 50;

/// An `Edit`'s strings longer than this many lines each are counted by
/// their changed middle, without the line-by-line match.
const DIFF_CELLS_MAX: usize = 250_000;

/// What Claude Code writes as a user turn when you stop it mid-answer:
/// not something asked, so not a turn's start.
const INTERRUPTED: &str = "[Request interrupted by user";

/// What one turn changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TurnChanges {
    /// Where the turn starts in the file (its first line's byte): the
    /// same turn read for two items is one turn.
    pub start: u64,
    /// The files it changed, each once.
    pub files: u32,
    /// Their names (the last two parts of each path, as a tool call's
    /// summary names a file), in the order first changed, at most
    /// [`NAMES_MAX`].
    pub names: Vec<String>,
    /// The lines added and removed; `None` when a call doesn't say (files
    /// only).
    pub lines: Option<LinesChanged>,
}

/// What the turn that ended by `until` (epoch ms; the file's end when
/// `None`) of the transcript at `path` changed; `None` when it changed no
/// file, its start isn't within [`MAX_WINDOW`] of the end, or the file is
/// stored compressed.
///
/// # Errors
///
/// The file can't be opened or read.
pub fn turn_changes(path: &Path, until: Option<i64>) -> std::io::Result<Option<TurnChanges>> {
    if is_compressed(path) {
        return Ok(None);
    }
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    for size in WINDOWS {
        let start = len.saturating_sub(size);
        file.seek(SeekFrom::Start(start))?;
        let mut bytes = Vec::new();
        (&file).take(len - start).read_to_end(&mut bytes)?;
        // From the first whole line.
        let from = if start == 0 {
            0
        } else {
            bytes
                .iter()
                .position(|b| *b == b'\n')
                .map_or(bytes.len(), |at| at + 1)
        };
        let mut tally = Tally::new(until);
        tally.feed(&bytes[from..], start + from as u64);
        if tally.started || start == 0 {
            return Ok(tally.changes());
        }
    }
    Ok(None)
}

/// One edit or write call.
#[derive(Debug)]
struct Call {
    path: String,
    lines: Option<LinesChanged>,
    failed: bool,
}

/// The calls of the turn being read.
#[derive(Debug)]
struct Tally {
    until: Option<i64>,
    format: Option<TranscriptFormat>,
    calls: Vec<Call>,
    by_id: HashMap<String, usize>,
    /// A turn's start was read.
    started: bool,
    /// Where the turn read starts in the file.
    start: u64,
    /// Where the line being read starts.
    at: u64,
    /// A line past `until` was read: the rest is later turns.
    past: bool,
}

impl Tally {
    fn new(until: Option<i64>) -> Self {
        Self {
            until,
            format: None,
            calls: Vec::new(),
            by_id: HashMap::new(),
            started: false,
            start: 0,
            at: 0,
            past: false,
        }
    }

    /// Whole lines, `\n`-separated, the first at byte `base` of the
    /// file; a malformed one is skipped.
    fn feed(&mut self, bytes: &[u8], base: u64) {
        self.start = base;
        let mut next = base;
        for line in bytes.split(|b| *b == b'\n') {
            self.at = next;
            next += line.len() as u64 + 1;
            if self.past {
                return;
            }
            if !may_matter(line) {
                continue;
            }
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
                continue;
            };
            if let (Some(until), Some(at)) = (self.until, line_time(&value))
                && at > until + SLACK_MS
            {
                self.past = true;
                return;
            }
            match *self.format.get_or_insert_with(|| line_format(&value)) {
                TranscriptFormat::ClaudeCode => self.claude_line(&value),
                TranscriptFormat::CodexRollout => self.codex_line(&value),
            }
        }
    }

    /// The user asked something: a new turn.
    fn start(&mut self) {
        self.calls.clear();
        self.by_id.clear();
        self.started = true;
        self.start = self.at;
    }

    fn claude_line(&mut self, value: &serde_json::Value) {
        let content = value.get("message").and_then(|m| m.get("content"));
        match value.get("type").and_then(serde_json::Value::as_str) {
            Some("assistant") => {
                for part in parts(content) {
                    if kind_of(part) != Some("tool_use") {
                        continue;
                    }
                    let name = part.get("name").and_then(serde_json::Value::as_str);
                    let input = part.get("input").unwrap_or(&serde_json::Value::Null);
                    let id = part.get("id").and_then(serde_json::Value::as_str);
                    if let Some(name) = name {
                        self.claude_call(name, input, id);
                    }
                }
            }
            Some("user") => {
                let results: Vec<&serde_json::Value> = parts(content)
                    .filter(|part| kind_of(part) == Some("tool_result"))
                    .collect();
                if results.is_empty() {
                    if asked(value, content) {
                        self.start();
                    }
                    return;
                }
                // `toolUseResult` describes the line's one result.
                let described = (results.len() == 1)
                    .then(|| value.get("toolUseResult"))
                    .flatten()
                    .filter(|result| result.is_object());
                for result in results {
                    let Some(call) = result
                        .get("tool_use_id")
                        .and_then(serde_json::Value::as_str)
                        .and_then(|id| self.by_id.get(id))
                        .and_then(|at| self.calls.get_mut(*at))
                    else {
                        continue;
                    };
                    if result.get("is_error").and_then(serde_json::Value::as_bool) == Some(true) {
                        call.failed = true;
                    } else if let Some(lines) = described.and_then(result_lines) {
                        call.lines = Some(lines);
                    }
                }
            }
            _ => {}
        }
    }

    fn claude_call(&mut self, name: &str, input: &serde_json::Value, id: Option<&str>) {
        let text = |key: &str| input.get(key).and_then(serde_json::Value::as_str);
        let lines = match name {
            "Edit" => Some(edit_lines(
                text("old_string").unwrap_or(""),
                text("new_string").unwrap_or(""),
            )),
            "MultiEdit" => Some(
                input
                    .get("edits")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|edit| {
                        let text = |key: &str| edit.get(key).and_then(serde_json::Value::as_str);
                        edit_lines(
                            text("old_string").unwrap_or(""),
                            text("new_string").unwrap_or(""),
                        )
                    })
                    .fold(LinesChanged::default(), add),
            ),
            // What the file held before isn't in the call: its result says.
            "Write" | "NotebookEdit" => None,
            _ => return,
        };
        let Some(path) = text("file_path").or_else(|| text("notebook_path")) else {
            return;
        };
        // A `replace_all` changes every match: only its result says how
        // many.
        let lines = lines.filter(|_| {
            input
                .get("replace_all")
                .and_then(serde_json::Value::as_bool)
                != Some(true)
        });
        if let Some(id) = id {
            self.by_id.insert(id.to_owned(), self.calls.len());
        }
        self.calls.push(Call {
            path: path.to_owned(),
            lines,
            failed: false,
        });
    }

    fn codex_line(&mut self, value: &serde_json::Value) {
        let null = serde_json::Value::Null;
        let payload = value.get("payload").unwrap_or(&null);
        let kind = |of: &serde_json::Value| {
            of.get("type")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        };
        match (kind(value).as_deref(), kind(payload).as_deref()) {
            (Some("event_msg"), Some("user_message")) => self.start(),
            (Some("response_item"), Some("function_call" | "custom_tool_call")) => {
                let Some((name, input)) = crate::codex_transcript::call_of(payload) else {
                    return;
                };
                if name != "apply_patch" {
                    return;
                }
                let patch = input
                    .get("input")
                    .or_else(|| input.get("patch"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                for (path, lines) in patch_files(patch) {
                    self.calls.push(Call {
                        path,
                        lines,
                        failed: false,
                    });
                }
            }
            _ => {}
        }
    }

    /// The turn's changes, `None` when it changed no file.
    fn changes(&self) -> Option<TurnChanges> {
        let done: Vec<&Call> = self.calls.iter().filter(|call| !call.failed).collect();
        if done.is_empty() {
            return None;
        }
        let mut paths: Vec<&str> = Vec::new();
        for call in &done {
            if !paths.contains(&call.path.as_str()) {
                paths.push(&call.path);
            }
        }
        let mut names: Vec<String> = Vec::new();
        for path in &paths {
            let name = crate::tool_summary::tail_of_path(path);
            if !names.contains(&name) && names.len() < NAMES_MAX {
                names.push(name);
            }
        }
        let lines = done.iter().try_fold(LinesChanged::default(), |sum, call| {
            Some(add(sum, call.lines?))
        });
        Some(TurnChanges {
            start: self.start,
            files: u32::try_from(paths.len()).unwrap_or(u32::MAX),
            names,
            lines,
        })
    }
}

/// Whether a line can be a turn's start, a call or a call's result
/// worth reading (most of a transcript is tool output: skipped unread).
fn may_matter(line: &[u8]) -> bool {
    let has = |needle: &[u8]| memchr::memmem::find(line, needle).is_some();
    if has(br#""tool_result""#) {
        return has(br#""structuredPatch""#)
            || has(br#""type":"create""#)
            || has(br#""is_error":true"#);
    }
    has(br#""type":"user""#) || has(br#""tool_use""#) || has(b"user_message") || has(b"apply_patch")
}

/// When a line was written, epoch ms, when it says.
fn line_time(value: &serde_json::Value) -> Option<i64> {
    value
        .get("timestamp")
        .and_then(serde_json::Value::as_str)
        .and_then(juggler_core::time::timestamp_ms)
}

fn parts(content: Option<&serde_json::Value>) -> impl Iterator<Item = &serde_json::Value> {
    content
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
}

fn kind_of(part: &serde_json::Value) -> Option<&str> {
    part.get("type").and_then(serde_json::Value::as_str)
}

/// Whether a Claude Code user line is the user asking something (a new
/// turn): not a subagent's line, a note Claude Code adds, a compaction's
/// summary or a stop.
fn asked(value: &serde_json::Value, content: Option<&serde_json::Value>) -> bool {
    let flag = |key: &str| value.get(key).and_then(serde_json::Value::as_bool) == Some(true);
    if flag("isSidechain") || flag("isMeta") || flag("isCompactSummary") {
        return false;
    }
    let text = match content {
        Some(serde_json::Value::String(text)) => Some(text.as_str()),
        _ => parts(content)
            .find(|part| kind_of(part) == Some("text"))
            .and_then(|part| part.get("text"))
            .and_then(serde_json::Value::as_str),
    };
    text.is_some_and(|text| !text.trim_start().starts_with(INTERRUPTED))
        || parts(content).any(|part| kind_of(part) == Some("image"))
}

/// The lines a call's result says it changed: a new file's lines, or its
/// patch's.
fn result_lines(result: &serde_json::Value) -> Option<LinesChanged> {
    if result.get("type").and_then(serde_json::Value::as_str) == Some("create")
        && let Some(content) = result.get("content").and_then(serde_json::Value::as_str)
    {
        return Some(LinesChanged {
            added: content.lines().count() as u64,
            removed: 0,
        });
    }
    let hunks = result.get("structuredPatch")?.as_array()?;
    let mut lines = LinesChanged::default();
    for line in hunks
        .iter()
        .filter_map(|hunk| hunk.get("lines")?.as_array())
        .flatten()
        .filter_map(serde_json::Value::as_str)
    {
        if line.starts_with('+') {
            lines.added += 1;
        } else if line.starts_with('-') {
            lines.removed += 1;
        }
    }
    Some(lines)
}

fn add(a: LinesChanged, b: LinesChanged) -> LinesChanged {
    LinesChanged {
        added: a.added + b.added,
        removed: a.removed + b.removed,
    }
}

/// The lines an edit from `old` to `new` adds and removes, as a line
/// diff counts them: the lines both keep in order are unchanged.
#[must_use]
pub fn edit_lines(old: &str, new: &str) -> LinesChanged {
    let old: Vec<&str> = old.lines().collect();
    let new: Vec<&str> = new.lines().collect();
    let head = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let old = &old[head..];
    let new = &new[head..];
    let tail = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let old = &old[..old.len() - tail];
    let new = &new[..new.len() - tail];
    let kept = if old.len().saturating_mul(new.len()) <= DIFF_CELLS_MAX {
        common_lines(old, new)
    } else {
        0
    };
    LinesChanged {
        added: (new.len() - kept) as u64,
        removed: (old.len() - kept) as u64,
    }
}

/// The longest run of lines both keep in order.
fn common_lines(old: &[&str], new: &[&str]) -> usize {
    let mut row = vec![0_usize; new.len() + 1];
    for a in old {
        let mut diagonal = 0;
        for (at, b) in new.iter().enumerate() {
            let above = row[at + 1];
            row[at + 1] = if a == b {
                diagonal + 1
            } else {
                above.max(row[at])
            };
            diagonal = above;
        }
    }
    row[new.len()]
}

/// A Codex patch's files and each one's lines (`None` for a deleted
/// file: its lines aren't in the patch).
fn patch_files(patch: &str) -> Vec<(String, Option<LinesChanged>)> {
    let mut files: Vec<(String, Option<LinesChanged>)> = Vec::new();
    for line in patch.lines() {
        if let Some(path) = line.strip_prefix("*** Delete File: ") {
            files.push((path.trim().to_owned(), None));
        } else if let Some(path) = line
            .strip_prefix("*** Update File: ")
            .or_else(|| line.strip_prefix("*** Add File: "))
        {
            files.push((path.trim().to_owned(), Some(LinesChanged::default())));
        } else if line.starts_with("*** ") {
            // `*** Begin Patch`, `*** End Patch`, `*** Move to: …`.
        } else if let Some((_, Some(lines))) = files.last_mut() {
            if line.starts_with('+') {
                lines.added += 1;
            } else if line.starts_with('-') {
                lines.removed += 1;
            }
        }
    }
    files
}

#[cfg(test)]
#[path = "turn_changes_tests.rs"]
mod tests;
