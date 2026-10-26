//! The messages' wire types (E18).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One page of threads.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ThreadsPage {
    /// Newest message first.
    pub threads: Vec<ThreadInfo>,
    /// The next page's cursor, when there is one.
    pub next: Option<String>,
    /// For one pool's page: why its messages are paused, when they are.
    pub paused: Option<PauseInfo>,
}

/// Why a pool's messages are paused.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PauseInfo {
    /// The pool.
    pub pool_id: String,
    /// Its name.
    pub pool_name: String,
    /// Since when, ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// Why, in words.
    pub reason: String,
}

/// One thread, as a row.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ThreadInfo {
    /// Its id.
    pub id: String,
    /// Its pool.
    pub pool_id: String,
    /// Its pool's name.
    pub pool_name: String,
    /// Its subject.
    pub subject: String,
    /// Who's in it (senders and recipients, by name; "You" for the user).
    pub people: Vec<String>,
    /// How many messages.
    pub count: u32,
    /// Agent messages in a row since a person's word.
    pub hop: u32,
    /// The most a thread takes in a row.
    pub hop_cap: u32,
    /// `open` | `paused` | `stopped`.
    pub state: String,
    /// Why it's paused or stopped.
    pub state_reason: Option<String>,
    /// Its latest message, ms.
    #[ts(type = "number")]
    pub updated_at_ms: i64,
    /// Its latest message: who, and its first line.
    pub last_from: String,
    /// …
    pub last_line: String,
    /// Messages to the user not read yet.
    pub unread: u32,
}

/// One page of a thread, oldest message first.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ThreadPage {
    /// The thread.
    pub thread: ThreadInfo,
    /// Its messages on this page, oldest first.
    pub messages: Vec<MessageInfo>,
    /// The cursor for earlier messages, when there are some.
    pub earlier: Option<String>,
}

/// One message.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MessageInfo {
    /// Its id.
    pub id: String,
    /// Its thread.
    pub thread_id: String,
    /// Who sent it ("You" for the user).
    pub from_name: String,
    /// `agent` | `user`.
    pub from_kind: String,
    /// The sending session, when an agent sent it.
    pub from_session_id: Option<String>,
    /// Its kind's word.
    pub kind: String,
    /// Its subject.
    pub subject: String,
    /// Its body (secrets already withheld as the user chose).
    pub body: String,
    /// Its hop.
    pub hop: u32,
    /// Secrets withheld from it.
    pub redacted: u32,
    /// Sent to the whole pool.
    pub to_all: bool,
    /// When, ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// Who it went to, and where it is for each.
    pub recipients: Vec<RecipientInfo>,
}

/// Where a message is for one recipient.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RecipientInfo {
    /// Who, as AI Juggler keeps it (what Deliver names).
    pub id: String,
    /// Their name ("You" for the user).
    pub name: String,
    /// Their session now, when they have a live one.
    pub session_id: Option<String>,
    /// `unread` | `read` | `acked` | `held` | `queued` | `delivered` | `refused`.
    pub state: String,
    /// `pull` | `turn` | `user`, when it got there.
    pub via: Option<String>,
    /// The user can click Deliver: the session runs in AI Juggler, it
    /// hasn't got it yet, and nothing stops it.
    pub deliverable: bool,
    /// What Deliver does or why there's none, in words.
    pub how: String,
}

/// The user's message.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ComposeInput {
    /// The pool it's in.
    pub pool_id: String,
    /// A member's session id, or `pool`.
    pub to: String,
    /// The thread it answers, when it does.
    pub thread_id: Option<String>,
    /// Its kind's word (a note when not said).
    pub kind: Option<String>,
    /// Its subject (a new thread's).
    pub subject: Option<String>,
    /// What it says.
    pub body: String,
    /// Also put it into the recipients' sessions at their next turn
    /// boundary (the user's click is the permission, D-29).
    pub deliver: bool,
}

/// One shared doc.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DocInfo {
    /// Its id.
    pub id: String,
    /// Its pool.
    pub pool_id: String,
    /// Its name.
    pub name: String,
    /// Its type.
    pub mime: String,
    /// Its size, bytes.
    #[ts(type = "number")]
    pub size_bytes: i64,
    /// Who shared it.
    pub shared_by: String,
    /// When, ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// The choice it was stored under: `strip` | `keep`.
    pub policy: String,
    /// The choice in words, as the Privacy settings say it.
    pub policy_words: String,
    /// What was found in it and how it was treated.
    pub withheld: Vec<WithheldInfo>,
}

/// What kind of secret, and how many.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct WithheldInfo {
    /// The kind (`sk-key`, `bearer`, `env-value`, `private-key`, …).
    pub kind: String,
    /// How many.
    pub count: u32,
}

/// One page of a pool's docs.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DocsPage {
    /// Newest first.
    pub docs: Vec<DocInfo>,
    /// The next page's cursor.
    pub next: Option<String>,
}

/// A doc's text.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DocContent {
    /// The doc.
    pub doc: DocInfo,
    /// Its text, as stored (secrets already handled).
    pub text: String,
}

/// Something about messages that waits on the user (Needs you).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MessageNeedInfo {
    /// Stable key.
    pub key: String,
    /// `ask` (an agent's message waits on your OK) | `deliver` (a chain
    /// reached the hop limit) | `question` (an agent asks you) |
    /// `waiting` (messages wait for a session you run here) |
    /// `pool_paused` | `thread_paused`.
    pub kind: String,
    /// Its pool.
    pub pool_id: String,
    /// The pool's name.
    pub pool_name: String,
    /// Its thread, when it has one.
    pub thread_id: Option<String>,
    /// Its message, when it has one.
    pub message_id: Option<String>,
    /// Its session, when it has one.
    pub session_id: Option<String>,
    /// The held ask to answer, for `ask` and `deliver`.
    pub ask_id: Option<String>,
    /// Offer "Always allow in this pool" (from the second approval).
    pub offer_always: bool,
    /// In one line.
    pub title: String,
    /// Where, or why.
    pub detail: String,
    /// The message's first lines, for `ask` and `question`.
    pub preview: Option<String>,
    /// Since when, ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}
