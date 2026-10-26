//! The tasks' wire types (E19).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One task, epic or story, as a row.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TaskInfo {
    /// Its short id (`aj-7f3c`).
    pub id: String,
    /// Its pool.
    pub pool_id: String,
    /// `epic` | `story` | `task`.
    pub kind: String,
    /// The epic or story it belongs to.
    pub parent_id: Option<String>,
    /// Its title.
    pub title: String,
    /// The honest status (`ready`, `in_progress`, `in_review`, `blocked`,
    /// `todo`, `done`, `cancelled`): blocked while an open task blocks it.
    pub status: String,
    /// What was set, when it differs (a ready task an open one blocks).
    pub stored_status: Option<String>,
    /// `urgent` | `high` | `normal` | `low`.
    pub priority: String,
    /// Who has it, by name ("You" for the user).
    pub holder: Option<String>,
    /// The session it's with now, when one runs (its page opens on a click).
    pub holder_session_id: Option<String>,
    /// The open tasks that block it.
    pub blocked_by: Vec<String>,
    /// Items under it, and how many are done.
    pub items: u32,
    /// …
    pub items_done: u32,
    /// Who wrote it, by name ("You" for the user).
    pub written_by: String,
    /// An agent wrote it.
    pub by_agent: bool,
    /// Its last change, ms.
    #[ts(type = "number")]
    pub updated_at_ms: i64,
    /// The issue it comes from: its name there (`owner/repo#12`).
    pub issue_label: Option<String>,
}

/// How many of a pool's tasks are in each honest status (the filter's
/// numbers).
#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TaskCounts {
    /// Ready for anyone.
    pub ready: u32,
    /// Someone is on it.
    pub in_progress: u32,
    /// Waits on a review.
    pub in_review: u32,
    /// Something blocks it.
    pub blocked: u32,
    /// Not ready yet.
    pub todo: u32,
    /// Finished.
    pub done: u32,
    /// Dropped.
    pub cancelled: u32,
}

/// One link, from the task's side.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TaskLinkInfo {
    /// `blocks` | `blocked_by` | `relates_to` | `duplicates` |
    /// `duplicated_by`.
    pub kind: String,
    /// The other task.
    pub task_id: String,
    /// Its title.
    pub title: String,
    /// Its honest status.
    pub status: String,
}

/// A session that worked the task.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TaskSessionInfo {
    /// The session.
    pub session_id: String,
    /// Its name.
    pub name: String,
    /// `claimed` | `started_for` | `assigned` | `worked`.
    pub how: String,
    /// It runs now.
    pub live: bool,
    /// Since when, ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// One event in the task's history.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TaskEventInfo {
    /// Its id (the cursor).
    #[ts(type = "number")]
    pub id: i64,
    /// When, ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// Who, by name ("You" for the user).
    pub who: String,
    /// An agent did it.
    pub by_agent: bool,
    /// What happened, in words.
    pub what: String,
}

/// A thread whose messages name the task.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TaskThreadInfo {
    /// The thread.
    pub thread_id: String,
    /// Its subject.
    pub subject: String,
    /// How many messages.
    pub count: u32,
    /// Its latest message, ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// The issue a task comes from.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TaskIssueInfo {
    /// The link.
    pub url: String,
    /// `github` | `linear` | `web`.
    pub service: String,
    /// Its name there (`owner/repo#12`, `ENG-123`).
    pub label: Option<String>,
    /// Its title there, as last looked up.
    pub title: Option<String>,
    /// Its state there (`open`, `closed`, `merged`).
    pub state: Option<String>,
    /// When it was looked up, ms.
    #[ts(type = "number | null")]
    pub checked_at_ms: Option<i64>,
    /// It can be looked up (a GitHub link, with `gh` on this Mac).
    pub can_look_up: bool,
}

/// A task in full.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TaskDetail {
    /// The row.
    pub task: TaskInfo,
    /// Its pool's name.
    pub pool_name: String,
    /// What it asks.
    pub body: String,
    /// When it's done.
    pub acceptance: String,
    /// Its parent's title, when it has one.
    pub parent_title: Option<String>,
    /// The items under it (a page).
    pub children: Vec<TaskInfo>,
    /// Its links.
    pub links: Vec<TaskLinkInfo>,
    /// The sessions that worked it, first first.
    pub sessions: Vec<TaskSessionInfo>,
    /// Threads whose messages name it.
    pub threads: Vec<TaskThreadInfo>,
    /// Its history, newest first (a page).
    pub history: Vec<TaskEventInfo>,
    /// The cursor for older history, when there is some.
    pub history_next: Option<String>,
    /// The issue it comes from.
    pub issue: Option<TaskIssueInfo>,
    /// A request for a new session for it waits on you (its id).
    pub session_asked: Option<String>,
}

/// What the user writes for a new task.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TaskInput {
    /// The pool.
    pub pool_id: String,
    /// One line.
    pub title: String,
    /// What it asks.
    #[serde(default)]
    pub body: String,
    /// When it's done.
    #[serde(default)]
    pub acceptance: String,
    /// `task` | `story` | `epic`.
    pub kind: String,
    /// Its epic or story.
    #[serde(default)]
    pub parent_id: Option<String>,
    /// `urgent` | `high` | `normal` | `low`.
    pub priority: String,
    /// `ready` | `todo`.
    pub status: String,
    /// The issue it comes from.
    #[serde(default)]
    pub url: Option<String>,
    /// The tasks that must be done first.
    #[serde(default)]
    pub blocked_by: Vec<String>,
}

/// The user's change to a task. `None` leaves a field as it is.
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TaskEdit {
    /// The task.
    pub task_id: String,
    /// A new title.
    #[serde(default)]
    pub title: Option<String>,
    /// A new body.
    #[serde(default)]
    pub body: Option<String>,
    /// New acceptance criteria.
    #[serde(default)]
    pub acceptance: Option<String>,
    /// A new priority.
    #[serde(default)]
    pub priority: Option<String>,
    /// A new status.
    #[serde(default)]
    pub status: Option<String>,
    /// A new parent (`""` takes it out).
    #[serde(default)]
    pub parent_id: Option<String>,
    /// A new link (`""` removes it).
    #[serde(default)]
    pub url: Option<String>,
    /// A note for its history.
    #[serde(default)]
    pub note: Option<String>,
}

/// Something about tasks that waits on the user, for Needs you.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TaskNeedInfo {
    /// Its id (the answer's key).
    pub id: String,
    /// `ask` (an agent's change waits on your OK) | `launch` (an agent
    /// asks for a new session; you start it).
    pub kind: String,
    /// The pool.
    pub pool_id: String,
    /// Its name.
    pub pool_name: String,
    /// The task, when it's one that exists.
    pub task_id: Option<String>,
    /// Its title.
    pub task_title: Option<String>,
    /// In one line.
    pub title: String,
    /// What it would change, or the asker's note.
    pub detail: String,
    /// Offer "Always allow in this pool" (from the second approval).
    pub offer_always: bool,
    /// For `launch`: the pool's project, where the session would work.
    pub project_id: Option<String>,
    /// Since when, ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// A session started for a task.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TaskStartedInfo {
    /// The session.
    pub session_id: String,
    /// The task, now with it.
    pub task: TaskInfo,
}
