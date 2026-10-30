//! AJ-E1520 (0.2): the subagents a session started, as each tool records
//! them — one record for every tool, so the app and its UI never know
//! which tool wrote it (ADR-0002: a tool's layout lives in its adapter).
//!
//! Every adapter answers [`crate::HarnessAdapter::subagent_support`]:
//! either it reads its tool's records ([`SubagentSupport::Supported`],
//! and [`crate::HarnessAdapter::subagent_reader`] lists a session's
//! subagents), or it says why it can't ([`SubagentSupport::NotSupported`]).
//! [`SubagentSupport::Unanswered`] is only the trait's default: the
//! conformance kit (AJ-E1521, check 11) refuses an adapter that leaves it.
//!
//! Reading is read-only (D-28, D-37): files and stores are opened to read,
//! never written, and no process environment is read. What a reader
//! returns is raw: the caller redacts a task's words before they go
//! anywhere (D-14). A status is what the records say, never a guess
//! (D-06): with nothing recorded either way it is [`SubagentStatus::Unknown`].

use std::path::{Path, PathBuf};

/// Where a subagent stands, as its tool's records say.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SubagentStatus {
    /// Started, and nothing recorded says it has ended. The app shows it
    /// as running only while its session is open; once the session has
    /// ended it is unknown (the record never said how it ended).
    Running,
    /// Finished.
    Done,
    /// Ended with an error.
    Failed,
    /// Stopped before it finished (by the user, or by its session).
    Stopped,
    /// The records don't say (a subagent's files without the record that
    /// links it to its start). The default: never a guess.
    #[default]
    Unknown,
}

impl SubagentStatus {
    /// Done, failed or stopped: nothing later changes it.
    #[must_use]
    pub fn is_final(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Stopped)
    }
}

/// One subagent a session (or one of its subagents) started.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Subagent {
    /// The tool's id for it, unique within the session (Claude Code's
    /// agent id, Codex's thread id, a child session's id).
    pub id: String,
    /// Its type or name as the tool records it (`Explore`,
    /// `frontend-worker`, `explorer`), when it does.
    pub kind: Option<String>,
    /// What it was asked to do (the tool call's description), when
    /// recorded. Raw: the caller redacts it.
    pub task: Option<String>,
    /// Where it stands.
    pub status: SubagentStatus,
    /// When it started, epoch ms, when recorded.
    pub started_ms: Option<i64>,
    /// When it ended, epoch ms: only for a final status, when recorded.
    pub ended_ms: Option<i64>,
    /// The subagent that started it; `None` when the session itself did
    /// (or the record that would say is missing).
    pub parent: Option<String>,
    /// How deep it is: 1 when the session started it, 2 when one of its
    /// subagents did, and so on.
    pub depth: u32,
    /// Whether it ran in the background (the session went on meanwhile);
    /// `None` when the tool doesn't record it.
    pub background: Option<bool>,
    /// Its own conversation, when the tool keeps one in a file the
    /// session page's reader reads (`SubagentConversations::Read`).
    pub transcript: Option<PathBuf>,
    /// AJ-E1520 (0.3): what else its records say (its prompt, result,
    /// tokens, work); every part `None` or empty when the tool doesn't
    /// record it.
    pub details: SubagentDetails,
}

/// AJ-E1520 (0.3): what a subagent's records say beyond where it stands
/// — what it was given, what it answered, what it used and did. Raw text
/// (the caller redacts, D-14), capped by the reader; each part `None`
/// (or empty) when its tool doesn't record it: never a guess.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubagentDetails {
    /// The prompt it was given (at most [`PROMPT_CHARS`]).
    pub prompt: Option<String>,
    /// The start of its answer (at most [`RESULT_CHARS`]), markdown.
    pub result: Option<String>,
    /// Why it failed, in one line, when it did and the tool says.
    pub failure: Option<String>,
    /// The model it ran on, as the tool names it.
    pub model: Option<String>,
    /// The tokens it used (its own, not its subagents').
    pub tokens: Option<SubagentTokens>,
    /// Its tool calls by kind.
    pub tools: Option<ToolCounts>,
    /// Lines its edits added and removed.
    pub lines: Option<LinesChanged>,
    /// The files it edited or wrote, most-edited first (at most
    /// [`FILES_MAX`]).
    pub files: Vec<String>,
    /// How many files it edited in all (more than `files` holds when
    /// capped).
    pub files_total: u32,
    /// The commands it ran, oldest first (at most [`COMMANDS_MAX`], the
    /// latest kept): what a process it probably started is matched by.
    pub commands: Vec<SubagentCommand>,
    /// The user's message the turn that started it began with (the
    /// session's subagents only; a nested one is under its parent's).
    pub asked: Option<Asked>,
    /// Whether the user stopped it, when the tool records it.
    pub stopped_by_user: Option<bool>,
    /// The worktree it ran in, when it had its own.
    pub worktree: Option<String>,
    /// That worktree's branch.
    pub branch: Option<String>,
    /// When its records last changed, epoch ms.
    pub last_activity_ms: Option<i64>,
    /// How long it ran, as the tool reported at its end (ms).
    pub duration_ms: Option<i64>,
}

/// The most characters of a prompt a reader keeps.
pub const PROMPT_CHARS: usize = 4_000;
/// The most characters of a result a reader keeps.
pub const RESULT_CHARS: usize = 4_000;
/// The most characters of the user's message a reader keeps (its first
/// line).
pub const ASKED_CHARS: usize = 240;
/// The most files a reader lists for one subagent.
pub const FILES_MAX: usize = 200;
/// The most commands a reader keeps for one subagent.
pub const COMMANDS_MAX: usize = 40;
/// The most characters of one command a reader keeps.
pub const COMMAND_CHARS: usize = 200;

/// Tokens by kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubagentTokens {
    /// Fresh input.
    pub input: u64,
    /// Output (with any reasoning).
    pub output: u64,
    /// Read from the cache.
    pub cache_read: u64,
    /// Written to the cache.
    pub cache_write: u64,
}

impl SubagentTokens {
    /// Every kind together.
    #[must_use]
    pub fn total(self) -> u64 {
        self.input
            .saturating_add(self.output)
            .saturating_add(self.cache_read)
            .saturating_add(self.cache_write)
    }

    /// `self` and `other` together.
    #[must_use]
    pub fn plus(self, other: Self) -> Self {
        Self {
            input: self.input.saturating_add(other.input),
            output: self.output.saturating_add(other.output),
            cache_read: self.cache_read.saturating_add(other.cache_read),
            cache_write: self.cache_write.saturating_add(other.cache_write),
        }
    }
}

/// How a tool's subagents' tokens relate to their session's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TokensCounted {
    /// Not recorded per subagent.
    #[default]
    NotRecorded,
    /// Part of the session's own total (Claude Code: the session's usage
    /// counts its subagents' files): shown as a share, never added on top.
    InSession,
    /// Apart from the session's (Codex: each child is a thread of its
    /// own).
    Separate,
}

/// A subagent's tool calls by kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolCounts {
    /// Every call.
    pub total: u32,
    /// Files read.
    pub reads: u32,
    /// Searches (files, text, the web).
    pub searches: u32,
    /// Commands run.
    pub commands: u32,
    /// Files edited or written.
    pub edits: u32,
    /// Anything else.
    pub other: u32,
}

/// Lines added and removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LinesChanged {
    /// Added.
    pub added: u64,
    /// Removed.
    pub removed: u64,
}

/// One command a subagent ran.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubagentCommand {
    /// The command (at most [`COMMAND_CHARS`]). Raw: the caller redacts.
    pub command: String,
    /// When it was asked for, epoch ms.
    pub at_ms: Option<i64>,
    /// Asked to run in the background.
    pub background: bool,
}

/// The user's message a turn began with.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Asked {
    /// Its first line (at most [`ASKED_CHARS`]). Raw: the caller redacts.
    pub text: String,
    /// When it was sent, epoch ms.
    pub at_ms: Option<i64>,
    /// The tool's id for the message (stable: subagents of one turn
    /// share it).
    pub key: String,
}

/// Which kind of text a reader hands its checkpoint filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubagentText {
    /// What it was asked to do (the call's short description).
    Task,
    /// The prompt it was given.
    Prompt,
    /// Its answer, or why it failed.
    Result,
    /// The user's message the turn began with.
    Asked,
    /// A file it edited.
    File,
    /// A command it ran.
    Command,
}

/// What a checkpoint's text passes through: the text kept (redacted), or
/// `None` to keep none of it.
pub type TextFilter<'a> = dyn Fn(SubagentText, &str) -> Option<String> + 'a;

/// Whether a subagent's own conversation can be shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubagentConversations {
    /// Each subagent's [`Subagent::transcript`], when it has one, is read
    /// by the same reader as the session's own conversation.
    Read,
    /// Not shown, and why (for the user and the capabilities manifest).
    NotRead(&'static str),
}

/// What an adapter can say about its tool's subagents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubagentSupport {
    /// The trait's default: the adapter hasn't answered. Never shipped
    /// (the conformance kit refuses it); the app treats it as not
    /// supported.
    Unanswered,
    /// The tool keeps no record AI Juggler may read, and why.
    NotSupported(&'static str),
    /// The adapter lists a session's subagents from the tool's records.
    Supported {
        /// The route it reads (for the manifest: "documented files",
        /// "the store, read-only", …).
        route: &'static str,
        /// Whether each subagent's conversation can be shown.
        conversations: SubagentConversations,
        /// AJ-E1520 (0.3): how to stop one subagent in the tool itself
        /// (AI Juggler can't stop one inside a session), when it can be.
        stop_hint: Option<&'static str>,
    },
}

impl SubagentSupport {
    /// Whether a session's subagents can be listed.
    #[must_use]
    pub fn is_supported(self) -> bool {
        matches!(self, Self::Supported { .. })
    }

    /// Why they can't be, when they can't.
    #[must_use]
    pub fn reason(self) -> Option<&'static str> {
        match self {
            Self::Unanswered => Some(UNANSWERED),
            Self::NotSupported(reason) => Some(reason),
            Self::Supported { .. } => None,
        }
    }
}

/// What the app says for an adapter that hasn't answered.
pub const UNANSWERED: &str = "This tool's adapter doesn't read its subagents yet.";

/// A session's subagents, read as its records grow.
///
/// The app keeps a reader while a session's page shows, and calls
/// [`SubagentReader::refresh`] before each listing: a reader reads only
/// what was added since the last refresh where its tool's records allow
/// (an appended transcript), so following a working session stays cheap.
pub trait SubagentReader: Send {
    /// Reads what changed in the records since the last refresh, within a
    /// bounded amount of work: a very long session is read over several
    /// refreshes, each listing what was found so far. True when it has
    /// caught up with the records (false: refresh again for the rest).
    ///
    /// # Errors
    ///
    /// The session's own record can't be read (gone, or unreadable).
    fn refresh(&mut self) -> std::io::Result<bool>;

    /// Every subagent read so far, in no particular order (the app
    /// orders and pages them).
    fn subagents(&self) -> Vec<Subagent>;

    /// AJ-E1520 (0.3): how many are running as far as read (the rail's
    /// count, read every few seconds: a reader that can answer without
    /// building every subagent's details should).
    fn running(&self) -> u64 {
        self.subagents()
            .iter()
            .filter(|agent| agent.status == SubagentStatus::Running)
            .count() as u64
    }

    /// AJ-E1520 (0.3): reads what each subagent's own records gained
    /// (its tokens, tools, files) within at most `budget` bytes, beyond
    /// what [`Self::refresh`] reads. True when caught up. The rail's
    /// count never asks; the Subagents tab does.
    ///
    /// # Errors
    ///
    /// As [`Self::refresh`].
    fn refresh_details(&mut self, budget: u64) -> std::io::Result<bool> {
        let _ = budget;
        Ok(true)
    }

    /// AJ-E1520 (0.3): the session's own tokens (without its subagents'),
    /// as far as read, when the tool records them per message.
    fn session_tokens(&self) -> Option<SubagentTokens> {
        None
    }

    /// AJ-E1520 (0.3): how its subagents' tokens relate to the session's.
    fn tokens_counted(&self) -> TokensCounted {
        TokensCounted::NotRecorded
    }

    /// AJ-E1520 (0.3): what it has read, as bytes the app may save and
    /// hand back to [`Self::resume`] (so a long session's tab opens at
    /// once next time), every text passed through `text` first (the
    /// app's redaction, D-14). `None`: it keeps no checkpoint.
    fn checkpoint(&self, text: &TextFilter<'_>) -> Option<Vec<u8>> {
        let _ = text;
        None
    }

    /// AJ-E1520 (0.3): picks up from a [`Self::checkpoint`]; false when
    /// it can't (another version, another session): it reads from the
    /// start as before. Files that changed since are read again by the
    /// next refresh (a replaced or shorter one from its start).
    fn resume(&mut self, saved: &[u8]) -> bool {
        let _ = saved;
        false
    }
}

/// The conversation a reader is asked about: its record as the
/// conversations index has it (a file, or a store's `path#id` key), and
/// the tool's id for it.
#[derive(Clone, Copy, Debug)]
pub struct SubagentScope<'a> {
    /// The record the index found.
    pub path: &'a Path,
    /// The tool's id for the conversation.
    pub id: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_supported_answer_lists_and_every_other_says_why() {
        let supported = SubagentSupport::Supported {
            route: "files",
            conversations: SubagentConversations::Read,
            stop_hint: None,
        };
        assert!(supported.is_supported());
        assert_eq!(supported.reason(), None);
        assert_eq!(SubagentSupport::Unanswered.reason(), Some(UNANSWERED));
        assert_eq!(
            SubagentSupport::NotSupported("no record").reason(),
            Some("no record")
        );
        assert!(!SubagentSupport::NotSupported("x").is_supported());
    }

    #[test]
    fn tokens_add_up_and_never_overflow() {
        let one = SubagentTokens {
            input: 1,
            output: 2,
            cache_read: 3,
            cache_write: 4,
        };
        assert_eq!(one.total(), 10);
        assert_eq!(one.plus(one).total(), 20);
        let huge = SubagentTokens {
            input: u64::MAX,
            ..SubagentTokens::default()
        };
        assert_eq!(huge.plus(one).input, u64::MAX);
    }

    #[test]
    fn done_failed_and_stopped_are_final() {
        assert!(SubagentStatus::Done.is_final());
        assert!(SubagentStatus::Failed.is_final());
        assert!(SubagentStatus::Stopped.is_final());
        assert!(!SubagentStatus::Running.is_final());
        assert!(!SubagentStatus::Unknown.is_final());
    }
}
