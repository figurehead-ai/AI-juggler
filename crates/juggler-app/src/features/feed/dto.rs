//! The feed's wire types (D-118). Facts only: every sentence is the
//! window's (`ui/src/lib/feed-words.ts`), so the words are written and
//! tested in one place.

use crate::features::notifications::centre::NoticeTarget;

/// Who acted: the author line of an item.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FeedAuthorInfo {
    /// `app` (a session's App), `agent` (a pool agent), `account` (an
    /// Account's room), `juggler` (AI Juggler's own act) or `you`.
    pub kind: String,
    /// Its name, when the item names one (an agent's, an Account's).
    pub name: Option<String>,
    /// An App's tool (`claude-code`, `codex`…).
    pub provider: Option<String>,
    /// An App's Account folder (`accounts.id`).
    pub folder: Option<String>,
    /// That folder's name.
    pub folder_label: Option<String>,
    /// The App runs on the desktop (the Claude app), not in a terminal.
    pub desktop: bool,
}

/// What an item's sentence and facts line are made from. Every field is
/// optional: an item fills what its journal keeps, never a guess.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FeedFactsInfo {
    /// The project, by name and id.
    pub project: Option<String>,
    /// See `project`.
    pub project_id: Option<String>,
    /// A session's working folder (its name when nothing better).
    pub cwd: Option<String>,
    /// How long it ran, ms (an ending: since it started).
    #[ts(type = "number | null")]
    pub duration_ms: Option<i64>,
    /// An ending's detail (`exit code 1`, `signal 9`), or why a switch failed.
    pub detail: Option<String>,
    /// A restart's try, of `tries`.
    pub attempt: Option<u32>,
    /// See `attempt`.
    pub tries: Option<u32>,
    /// An Account's window (`5-hour` | `weekly`).
    pub window: Option<String>,
    /// When it resets, epoch ms.
    #[ts(type = "number | null")]
    pub resets_at_ms: Option<i64>,
    /// Room left, percent (measured).
    #[ts(type = "number | null")]
    pub meter: Option<i64>,
    /// Live sessions on the Account.
    pub sessions: Option<u32>,
    /// Credits that carry on, in the tool's words.
    pub credits: Option<String>,
    /// A version (an update).
    pub version: Option<String>,
    /// A switch: from and to (Account folder name and tool), how, and
    /// what it said it carried.
    pub from_label: Option<String>,
    /// See `from_label`.
    pub from_provider: Option<String>,
    /// See `from_label`.
    pub to_label: Option<String>,
    /// See `from_label`.
    pub to_provider: Option<String>,
    /// See `from_label`.
    pub mechanism: Option<String>,
    /// See `from_label`.
    pub fidelity: Option<String>,
    /// A pool, by name and id.
    pub pool: Option<String>,
    /// See `pool`.
    pub pool_id: Option<String>,
    /// A thread's subject, and its messages in all.
    pub thread: Option<String>,
    /// See `thread`.
    #[ts(type = "number | null")]
    pub thread_count: Option<i64>,
    /// An agent's words, quoted (a pool message's body, at most 280
    /// characters; redacted when written).
    pub quote: Option<String>,
    /// A task's change, as logged (`In progress → Done`).
    pub change: Option<String>,
    /// A resolved need's kind (`asks`, `limit`…): the history line's.
    pub need: Option<String>,
    /// The notice's own line, for a kind the window has no words for yet.
    pub line: Option<String>,
}

/// One item of the feed, newest first.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FeedItemInfo {
    /// Unique in the feed (`n:12`, `l:34`, `w:56`, `t:78`, `m:90`).
    pub id: String,
    /// When it happened, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// Its type (the catalogue's, `docs/research/home-feed/events.md`):
    /// `finished`, `failed`, `crashed`, `ended`, `restarted`, `stopped`,
    /// `reviewed`, `archived`, `answered` (a need resolved), `switch`,
    /// `switch_failed`, `quota`, `reset`, `renewal`, `credits`,
    /// `this_mac`, `app_started`, `keep_alive_paused`, `update`, `found`,
    /// `import`, `task_done`, `task_review`, `task_blocked`, `message`.
    pub kind: String,
    /// The filter chip it is under: `needs` | `finished` | `switches` |
    /// `pools` | `mac` | `other`.
    pub category: String,
    /// Its attention level when it happened (D-117: 3 broke, 2 needed
    /// you, 1 a note, 0 calm). In the stream every item is a quiet line;
    /// the level keeps rollups apart and orders the away digest.
    pub level: u8,
    /// The user's own act (hidden unless "Include my actions").
    pub mine: bool,
    /// One thing, one card: items with this key are one card, its newest
    /// state shown ("one session, one card").
    pub coalesce: Option<String>,
    /// Items with this key roll up into one card when they fall within
    /// `rollup_window_ms` of each other. The level is part of the key, so
    /// nothing broken is ever folded into a quiet rollup.
    pub rollup: Option<String>,
    /// See `rollup`.
    #[ts(type = "number")]
    pub rollup_window_ms: i64,
    /// What a rollup belongs to (an App's folder, a pool): another kind of
    /// item of the same scope starts the window again.
    pub scope: Option<String>,
    /// Who acted.
    pub author: FeedAuthorInfo,
    /// The object's name (a session, a task, a thread, an Account): the
    /// value of the sentence, and its link.
    pub subject: String,
    /// The facts its sentence and facts line say.
    pub facts: FeedFactsInfo,
    /// What a click opens.
    pub target: NoticeTarget,
}

/// What the feed is narrowed to (the window's filters).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FeedFiltersInfo {
    /// One chip's category (`needs`, `finished`, `switches`, `pools`,
    /// `mac`); none: All.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub category: Option<String>,
    /// One project, by id.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub project: Option<String>,
    /// One pool, by id.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub pool: Option<String>,
    /// The Account folders of one Account or App.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub folders: Option<Vec<String>>,
    /// The user's own acts too (off by default).
    #[serde(default)]
    pub include_mine: bool,
}
