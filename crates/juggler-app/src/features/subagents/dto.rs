//! The Subagents tab's wire types (AJ-E1520, 0.2).

use serde::Serialize;
use ts_rs::TS;

use crate::paging::Page;

/// A session's subagents, a page at a time, or why there are none.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentsInfo {
    /// `listed`; `unsupported` (its tool keeps no record AI Juggler
    /// reads: `reason` says why); `none` (no record of the session on
    /// this Mac); `unreadable` (its record couldn't be read just now).
    pub state: String,
    /// Why they aren't listed (`unsupported`, `unreadable`).
    pub reason: Option<String>,
    /// This page of them: newest first, each followed by the ones it
    /// started.
    pub page: Page<SubagentInfo>,
    /// How many of all of them are running now.
    pub running: u32,
    /// A very long session is still being read: what's found so far is
    /// listed; ask again for the rest.
    pub reading: bool,
    /// Whether the session is open now (a subagent the records never saw
    /// end is unknown once it isn't).
    pub live: bool,
    /// Why a subagent's conversation can't be opened, when its tool keeps
    /// none AI Juggler reads; `null` when it can.
    pub conversations_note: Option<String>,
}

/// One subagent, as the tab shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentInfo {
    /// The tool's id for it (unique in the session).
    pub id: String,
    /// Its type or name, when the tool records one.
    pub kind: Option<String>,
    /// What it was asked to do, redacted (D-14), when recorded.
    pub task: Option<String>,
    /// `running`, `done`, `failed`, `stopped` or `unknown` (D-06: never a
    /// guess).
    pub status: String,
    /// When it started, epoch ms.
    #[ts(type = "number | null")]
    pub started_ms: Option<i64>,
    /// When it ended, epoch ms (only once ended).
    #[ts(type = "number | null")]
    pub ended_ms: Option<i64>,
    /// The subagent that started it; `null` when the session did.
    pub parent: Option<String>,
    /// 1 when the session started it, 2 for one of its subagents', ….
    pub depth: u32,
    /// Whether it ran in the background, when the tool records it.
    pub background: Option<bool>,
    /// Whether its own conversation can be opened.
    pub has_conversation: bool,
}
