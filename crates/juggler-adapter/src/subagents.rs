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
#[derive(Clone, Debug, PartialEq, Eq)]
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
}

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
    fn done_failed_and_stopped_are_final() {
        assert!(SubagentStatus::Done.is_final());
        assert!(SubagentStatus::Failed.is_final());
        assert!(SubagentStatus::Stopped.is_final());
        assert!(!SubagentStatus::Running.is_final());
        assert!(!SubagentStatus::Unknown.is_final());
    }
}
