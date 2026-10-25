//! E18 storage (schema v34): messages between the agents of a pool, and
//! the user's messages to them.
//!
//! - **Threads** in a pool, each with its hop count (agent messages in a
//!   row since a person's word) and its state: open, paused (a loop cap,
//!   or the user) or stopped (the user).
//! - **Messages**, immutable once written: kind, subject, body (already
//!   redacted by the caller), the body's SHA-256, hop, sender and its name
//!   then. Full-text search over subject and body uses the transcript
//!   index's tokenizer (`unicode61 remove_diacritics 2`).
//! - **Recipients** with their state: `unread`, `read`, `acked`, `held`
//!   (waiting on the user's OK), `queued` (to be delivered at the next
//!   turn boundary), `delivered`, `refused`.
//! - **An append-only event log** (triggers refuse updates and deletes):
//!   every send, read, ack, hold, delivery, refusal and pause.
//! - **Shared docs** (metadata; the bytes are content-addressed files in
//!   AI Juggler's own folder), **pool links** (cross-pool messages need
//!   one the user approved), **pending asks** agents wait on, and each
//!   pool's **pause** (the circuit breaker, or the user).
//!
//! Nothing here starts, stops or types into a session, and nothing reads
//! quota (D-31).

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::CoreError;
use crate::model::Timestamp;

/// v34: the E18 tables.
pub(super) const MIGRATION: &str = "
CREATE TABLE agent_threads (
    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
    id         TEXT NOT NULL UNIQUE,
    pool_id    TEXT NOT NULL REFERENCES pools(id),
    subject    TEXT NOT NULL,
    started_by TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_seq   INTEGER NOT NULL DEFAULT 0,
    count      INTEGER NOT NULL DEFAULT 0,
    hop        INTEGER NOT NULL DEFAULT 0,
    state      TEXT NOT NULL DEFAULT 'open',
    state_reason TEXT,
    state_at   INTEGER
);
CREATE INDEX idx_agent_threads_pool ON agent_threads(pool_id, last_seq);
CREATE TABLE agent_messages (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    id          TEXT NOT NULL UNIQUE,
    thread_id   TEXT NOT NULL REFERENCES agent_threads(id),
    pool_id     TEXT NOT NULL,
    sender_ref  TEXT NOT NULL,
    sender_kind TEXT NOT NULL,
    sender_name TEXT NOT NULL,
    kind        TEXT NOT NULL,
    subject     TEXT NOT NULL,
    body        TEXT NOT NULL,
    body_sha256 TEXT NOT NULL,
    hop         INTEGER NOT NULL,
    reply_to    TEXT,
    to_all      INTEGER NOT NULL DEFAULT 0,
    redacted    INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL
);
CREATE INDEX idx_agent_messages_thread ON agent_messages(thread_id, seq);
CREATE INDEX idx_agent_messages_sender ON agent_messages(pool_id, sender_ref, created_at);
CREATE INDEX idx_agent_messages_pool ON agent_messages(pool_id, created_at);
CREATE TABLE agent_recipients (
    message_id     TEXT NOT NULL REFERENCES agent_messages(id),
    recipient_ref  TEXT NOT NULL,
    recipient_name TEXT NOT NULL,
    state          TEXT NOT NULL,
    state_at       INTEGER NOT NULL,
    via            TEXT,
    PRIMARY KEY (message_id, recipient_ref)
);
CREATE INDEX idx_agent_recipients_ref ON agent_recipients(recipient_ref, state);
CREATE VIRTUAL TABLE agent_messages_fts USING fts5(
    subject,
    body,
    content = 'agent_messages',
    content_rowid = 'seq',
    tokenize = 'unicode61 remove_diacritics 2'
);
CREATE TRIGGER agent_messages_fts_insert AFTER INSERT ON agent_messages BEGIN
    INSERT INTO agent_messages_fts(rowid, subject, body) VALUES (NEW.seq, NEW.subject, NEW.body);
END;
CREATE TRIGGER agent_messages_immutable BEFORE UPDATE ON agent_messages BEGIN
    SELECT RAISE(ABORT, 'messages are immutable');
END;
CREATE TABLE agent_events (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    at         INTEGER NOT NULL,
    pool_id    TEXT NOT NULL,
    actor_ref  TEXT NOT NULL,
    verb       TEXT NOT NULL,
    object_ref TEXT NOT NULL,
    detail     TEXT
);
CREATE INDEX idx_agent_events_pool ON agent_events(pool_id, id);
CREATE INDEX idx_agent_events_verb ON agent_events(pool_id, actor_ref, verb, at);
CREATE TRIGGER agent_events_no_update BEFORE UPDATE ON agent_events BEGIN
    SELECT RAISE(ABORT, 'the event log is append-only');
END;
CREATE TRIGGER agent_events_no_delete BEFORE DELETE ON agent_events BEGIN
    SELECT RAISE(ABORT, 'the event log is append-only');
END;
CREATE TABLE agent_docs (
    seq          INTEGER PRIMARY KEY AUTOINCREMENT,
    id           TEXT NOT NULL UNIQUE,
    pool_id      TEXT NOT NULL REFERENCES pools(id),
    sha256       TEXT NOT NULL,
    name         TEXT NOT NULL,
    mime         TEXT NOT NULL,
    size_bytes   INTEGER NOT NULL,
    shared_by    TEXT NOT NULL,
    shared_by_name TEXT NOT NULL,
    policy       TEXT NOT NULL,
    withheld     TEXT NOT NULL,
    created_at   INTEGER NOT NULL
);
CREATE INDEX idx_agent_docs_pool ON agent_docs(pool_id, seq);
CREATE TABLE pool_links (
    a_pool      TEXT NOT NULL REFERENCES pools(id),
    b_pool      TEXT NOT NULL REFERENCES pools(id),
    approved_at INTEGER NOT NULL,
    revoked_at  INTEGER
);
CREATE UNIQUE INDEX idx_pool_links_live ON pool_links(a_pool, b_pool) WHERE revoked_at IS NULL;
CREATE TABLE agent_asks (
    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
    id         TEXT NOT NULL UNIQUE,
    pool_id    TEXT NOT NULL,
    actor_ref  TEXT NOT NULL,
    actor_name TEXT NOT NULL,
    action     TEXT NOT NULL,
    object_ref TEXT NOT NULL,
    summary    TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    answered_at INTEGER,
    answer     TEXT
);
CREATE INDEX idx_agent_asks_open ON agent_asks(answered_at, seq);
CREATE TABLE pool_pauses (
    pool_id   TEXT PRIMARY KEY REFERENCES pools(id),
    paused_at INTEGER NOT NULL,
    reason    TEXT NOT NULL
);
";

/// Who sent a message: an agent, or the user in AI Juggler.
pub const SENDER_USER: &str = "user";

/// A thread's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadState {
    /// Messages flow.
    Open,
    /// Held: a loop cap tripped, or the user paused it.
    Paused,
    /// The user stopped it.
    Stopped,
}

impl ThreadState {
    /// The stored word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Paused => "paused",
            Self::Stopped => "stopped",
        }
    }

    /// Reads a stored word; an unknown one (a newer build's) reads as
    /// stopped, never as open.
    #[must_use]
    pub fn parse(word: &str) -> Self {
        match word {
            "open" => Self::Open,
            "paused" => Self::Paused,
            _ => Self::Stopped,
        }
    }
}

/// A recipient's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipientState {
    /// Not read yet.
    Unread,
    /// Read (through `read_inbox`, or the user).
    Read,
    /// The agent said it handled it.
    Acked,
    /// Waiting on the user's OK (an ask).
    Held,
    /// To be delivered into its session at the next turn boundary.
    Queued,
    /// Delivered as a turn.
    Delivered,
    /// The user said no, or it couldn't go.
    Refused,
}

impl RecipientState {
    /// Every state.
    pub const ALL: [Self; 7] = [
        Self::Unread,
        Self::Read,
        Self::Acked,
        Self::Held,
        Self::Queued,
        Self::Delivered,
        Self::Refused,
    ];

    /// The stored word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unread => "unread",
            Self::Read => "read",
            Self::Acked => "acked",
            Self::Held => "held",
            Self::Queued => "queued",
            Self::Delivered => "delivered",
            Self::Refused => "refused",
        }
    }

    /// Reads a stored word; an unknown one reads as refused (never shown
    /// to an agent, never delivered).
    #[must_use]
    pub fn parse(word: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|state| state.as_str() == word)
            .unwrap_or(Self::Refused)
    }
}

/// One thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thread {
    /// Its id (`thr-…`).
    pub id: String,
    /// Its pool.
    pub pool_id: String,
    /// Its subject (its first message's).
    pub subject: String,
    /// Who started it (a member ref, or [`SENDER_USER`]).
    pub started_by: String,
    /// When, ms.
    pub created_at: i64,
    /// Its latest message, ms.
    pub updated_at: i64,
    /// Its latest message's sequence number (the list's cursor).
    pub last_seq: i64,
    /// How many messages it has.
    pub count: u32,
    /// Agent messages in a row since a person's word.
    pub hop: u32,
    /// Open, paused or stopped.
    pub state: ThreadState,
    /// Why it's paused or stopped, in words.
    pub state_reason: Option<String>,
}

/// One message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// Its sequence number (the cursor).
    pub seq: i64,
    /// Its id (`msg-…`).
    pub id: String,
    /// Its thread.
    pub thread_id: String,
    /// Its pool.
    pub pool_id: String,
    /// The sender's member ref, or [`SENDER_USER`].
    pub sender_ref: String,
    /// `agent` | `user`.
    pub sender_kind: String,
    /// The sender's name when it sent it.
    pub sender_name: String,
    /// Its kind's word.
    pub kind: String,
    /// Its subject.
    pub subject: String,
    /// Its body (redacted before it was stored).
    pub body: String,
    /// The SHA-256 of what it said and to whom (duplicates).
    pub body_sha256: String,
    /// Its thread's hop when it was sent.
    pub hop: u32,
    /// The message it answers.
    pub reply_to: Option<String>,
    /// Sent to the whole pool.
    pub to_all: bool,
    /// How many secrets were withheld from it.
    pub redacted: u32,
    /// When, ms.
    pub created_at: i64,
}

/// One recipient of a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recipient {
    /// The message.
    pub message_id: String,
    /// Who (a member ref, or [`SENDER_USER`]).
    pub recipient_ref: String,
    /// Their name when it was sent.
    pub recipient_name: String,
    /// Where the message is for them.
    pub state: RecipientState,
    /// When it got there, ms.
    pub state_at: i64,
    /// How it reached them (`pull`, `turn`, `user`).
    pub via: Option<String>,
}

/// A message for one of a set of refs, with its state for them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboxRow {
    /// The message.
    pub message: Message,
    /// Its recipient row.
    pub recipient: Recipient,
}

/// A shared doc's record (its bytes are a file named by its hash).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Doc {
    /// Its id (`doc-…`).
    pub id: String,
    /// Its pool.
    pub pool_id: String,
    /// The SHA-256 of its stored (redacted) bytes.
    pub sha256: String,
    /// Its name.
    pub name: String,
    /// Its type (`text/markdown`, `text/plain`, `application/json`).
    pub mime: String,
    /// Its size, bytes.
    pub size_bytes: i64,
    /// Who shared it (a member ref, or [`SENDER_USER`]).
    pub shared_by: String,
    /// Their name then.
    pub shared_by_name: String,
    /// The redaction policy it was stored under (`strip` | `keep`).
    pub policy: String,
    /// What was found and withheld, as JSON (`[{kind, count}]`).
    pub withheld: String,
    /// When, ms.
    pub created_at: i64,
}

/// An agent's action that waits on the user's OK.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldAsk {
    /// Its id (`ask-…`).
    pub id: String,
    /// Its pool.
    pub pool_id: String,
    /// Who asked (a member ref).
    pub actor_ref: String,
    /// Their name then.
    pub actor_name: String,
    /// The action's word.
    pub action: String,
    /// What it's about (a message id).
    pub object_ref: String,
    /// What it asks, in one line.
    pub summary: String,
    /// When, ms.
    pub created_at: i64,
}

/// One event in the log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageEvent {
    /// Its id.
    pub id: i64,
    /// When, ms.
    pub at: i64,
    /// Its pool.
    pub pool_id: String,
    /// Who.
    pub actor_ref: String,
    /// What happened (`sent`, `read`, `acked`, `held`, `delivered`, …).
    pub verb: String,
    /// To what (a message, thread or pool).
    pub object_ref: String,
    /// Detail in words, when there is some.
    pub detail: Option<String>,
}

/// A message to write.
#[derive(Clone, Debug)]
pub struct NewMessage<'a> {
    /// Its id.
    pub id: &'a str,
    /// Its thread (it must exist).
    pub thread_id: &'a str,
    /// Its pool.
    pub pool_id: &'a str,
    /// The sender's ref.
    pub sender_ref: &'a str,
    /// `agent` | `user`.
    pub sender_kind: &'a str,
    /// The sender's name.
    pub sender_name: &'a str,
    /// Its kind's word.
    pub kind: &'a str,
    /// Its subject.
    pub subject: &'a str,
    /// Its body (already redacted).
    pub body: &'a str,
    /// What it said and to whom, hashed.
    pub body_sha256: &'a str,
    /// The thread's hop with this message.
    pub hop: u32,
    /// The message it answers.
    pub reply_to: Option<&'a str>,
    /// Sent to the whole pool.
    pub to_all: bool,
    /// Secrets withheld.
    pub redacted: u32,
    /// Its recipients: (ref, name, starting state).
    pub recipients: &'a [(String, String, RecipientState)],
}

/// Typed access to the E18 tables.
#[derive(Debug)]
pub struct MessagesRepo<'a> {
    conn: &'a Connection,
}

const THREAD_COLUMNS: &str = "id, pool_id, subject, started_by, created_at, updated_at, last_seq, count, hop, state, state_reason";
const MESSAGE_COLUMNS: &str = "m.seq, m.id, m.thread_id, m.pool_id, m.sender_ref, m.sender_kind, m.sender_name, m.kind, m.subject, m.body, m.body_sha256, m.hop, m.reply_to, m.to_all, m.redacted, m.created_at";
const DOC_COLUMNS: &str = "id, pool_id, sha256, name, mime, size_bytes, shared_by, shared_by_name, policy, withheld, created_at";

fn thread_of(row: &Row<'_>) -> rusqlite::Result<Thread> {
    Ok(Thread {
        id: row.get(0)?,
        pool_id: row.get(1)?,
        subject: row.get(2)?,
        started_by: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        last_seq: row.get(6)?,
        count: row.get(7)?,
        hop: row.get(8)?,
        state: ThreadState::parse(&row.get::<_, String>(9)?),
        state_reason: row.get(10)?,
    })
}

fn message_of(row: &Row<'_>) -> rusqlite::Result<Message> {
    Ok(Message {
        seq: row.get(0)?,
        id: row.get(1)?,
        thread_id: row.get(2)?,
        pool_id: row.get(3)?,
        sender_ref: row.get(4)?,
        sender_kind: row.get(5)?,
        sender_name: row.get(6)?,
        kind: row.get(7)?,
        subject: row.get(8)?,
        body: row.get(9)?,
        body_sha256: row.get(10)?,
        hop: row.get(11)?,
        reply_to: row.get(12)?,
        to_all: row.get(13)?,
        redacted: row.get(14)?,
        created_at: row.get(15)?,
    })
}

/// A recipient from columns starting at `at`.
fn recipient_at(row: &Row<'_>, at: usize) -> rusqlite::Result<Recipient> {
    Ok(Recipient {
        message_id: row.get(at)?,
        recipient_ref: row.get(at + 1)?,
        recipient_name: row.get(at + 2)?,
        state: RecipientState::parse(&row.get::<_, String>(at + 3)?),
        state_at: row.get(at + 4)?,
        via: row.get(at + 5)?,
    })
}

fn doc_of(row: &Row<'_>) -> rusqlite::Result<Doc> {
    Ok(Doc {
        id: row.get(0)?,
        pool_id: row.get(1)?,
        sha256: row.get(2)?,
        name: row.get(3)?,
        mime: row.get(4)?,
        size_bytes: row.get(5)?,
        shared_by: row.get(6)?,
        shared_by_name: row.get(7)?,
        policy: row.get(8)?,
        withheld: row.get(9)?,
        created_at: row.get(10)?,
    })
}

/// A list of refs as the JSON array `json_each` reads.
fn refs_json(refs: &[String]) -> String {
    serde_json::to_string(refs).unwrap_or_else(|_| "[]".to_owned())
}

/// A page size within 1..=200.
fn page(limit: u32) -> i64 {
    i64::from(limit.clamp(1, 200))
}

fn collect<T>(rows: impl Iterator<Item = rusqlite::Result<T>>) -> Result<Vec<T>, CoreError> {
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

impl MessagesRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> MessagesRepo<'_> {
        MessagesRepo { conn }
    }

    /// Starts a thread in a pool.
    ///
    /// # Errors
    ///
    /// Storage failures (a pool that isn't there is a constraint error).
    pub fn create_thread(
        &self,
        id: &str,
        pool_id: &str,
        subject: &str,
        started_by: &str,
    ) -> Result<Thread, CoreError> {
        let now = Timestamp::now().millis();
        self.conn.execute(
            "INSERT INTO agent_threads (id, pool_id, subject, started_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, pool_id, subject, started_by, now],
        )?;
        self.thread(id)?
            .ok_or_else(|| super::accounts::not_found("thread", id))
    }

    /// A thread, when it's there.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn thread(&self, id: &str) -> Result<Option<Thread>, CoreError> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {THREAD_COLUMNS} FROM agent_threads WHERE id = ?1"),
                params![id],
                thread_of,
            )
            .optional()?)
    }

    /// One page of threads, newest message first: a pool's (`pool_id`),
    /// or those any of `refs` wrote or got (a session's), or both;
    /// `before` is the previous page's last `last_seq`.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn threads(
        &self,
        pool_id: Option<&str>,
        refs: Option<&[String]>,
        before: Option<i64>,
        limit: u32,
    ) -> Result<Vec<Thread>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {THREAD_COLUMNS} FROM agent_threads t
              WHERE (?1 IS NULL OR t.pool_id = ?1)
                AND (?2 IS NULL OR t.id IN (
                      SELECT m.thread_id FROM agent_messages m
                       WHERE m.sender_ref IN (SELECT value FROM json_each(?2))
                      UNION
                      SELECT m.thread_id FROM agent_recipients r
                        JOIN agent_messages m ON m.id = r.message_id
                       WHERE r.recipient_ref IN (SELECT value FROM json_each(?2))))
                AND (?3 IS NULL OR t.last_seq < ?3)
              ORDER BY t.last_seq DESC LIMIT ?4"
        ))?;
        let refs = refs.map(refs_json);
        let rows = stmt.query_map(params![pool_id, refs, before, page(limit)], thread_of)?;
        collect(rows)
    }

    /// Writes a message and its recipients in one transaction, moves its
    /// thread on (its hop, count and time), and logs it (`sent`, or `held`
    /// when its recipients wait on the user).
    ///
    /// # Errors
    ///
    /// Storage failures; a missing thread.
    pub fn insert_message(&self, message: &NewMessage<'_>) -> Result<Message, CoreError> {
        let now = Timestamp::now().millis();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO agent_messages (id, thread_id, pool_id, sender_ref, sender_kind, sender_name,
                 kind, subject, body, body_sha256, hop, reply_to, to_all, redacted, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                message.id,
                message.thread_id,
                message.pool_id,
                message.sender_ref,
                message.sender_kind,
                message.sender_name,
                message.kind,
                message.subject,
                message.body,
                message.body_sha256,
                message.hop,
                message.reply_to,
                message.to_all,
                message.redacted,
                now
            ],
        )?;
        let seq = tx.last_insert_rowid();
        for (recipient, name, state) in message.recipients {
            tx.execute(
                "INSERT OR IGNORE INTO agent_recipients (message_id, recipient_ref, recipient_name, state, state_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![message.id, recipient, name, state.as_str(), now],
            )?;
        }
        let changed = tx.execute(
            "UPDATE agent_threads SET updated_at = ?2, last_seq = ?3, count = count + 1, hop = ?4
              WHERE id = ?1",
            params![message.thread_id, now, seq, message.hop],
        )?;
        if changed == 0 {
            return Err(super::accounts::not_found("thread", message.thread_id));
        }
        let held = message
            .recipients
            .iter()
            .any(|(_, _, state)| *state == RecipientState::Held);
        tx.execute(
            "INSERT INTO agent_events (at, pool_id, actor_ref, verb, object_ref, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                now,
                message.pool_id,
                message.sender_ref,
                if held { "held" } else { "sent" },
                message.id,
                format!(
                    "to {}",
                    message
                        .recipients
                        .iter()
                        .map(|(_, name, _)| name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            ],
        )?;
        tx.commit()?;
        self.message(message.id)?
            .ok_or_else(|| super::accounts::not_found("message", message.id))
    }

    /// A message, when it's there.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn message(&self, id: &str) -> Result<Option<Message>, CoreError> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {MESSAGE_COLUMNS} FROM agent_messages m WHERE m.id = ?1"),
                params![id],
                message_of,
            )
            .optional()?)
    }

    /// A message's recipients.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn recipients(&self, message_id: &str) -> Result<Vec<Recipient>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT message_id, recipient_ref, recipient_name, state, state_at, via
               FROM agent_recipients WHERE message_id = ?1 ORDER BY recipient_name",
        )?;
        let rows = stmt.query_map(params![message_id], |row| recipient_at(row, 0))?;
        collect(rows)
    }

    /// One page of a thread's messages, newest first; `before` is the
    /// previous page's last `seq`.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn thread_messages(
        &self,
        thread_id: &str,
        before: Option<i64>,
        limit: u32,
    ) -> Result<Vec<Message>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {MESSAGE_COLUMNS} FROM agent_messages m
              WHERE m.thread_id = ?1 AND (?2 IS NULL OR m.seq < ?2)
              ORDER BY m.seq DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(params![thread_id, before, page(limit)], message_of)?;
        collect(rows)
    }

    /// One page of the messages for any of `refs` in `states`, newest
    /// first; `before` is the previous page's last `seq`.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn inbox(
        &self,
        refs: &[String],
        states: &[RecipientState],
        thread_id: Option<&str>,
        before: Option<i64>,
        limit: u32,
    ) -> Result<Vec<InboxRow>, CoreError> {
        let states: Vec<String> = states.iter().map(|s| s.as_str().to_owned()).collect();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {MESSAGE_COLUMNS}, r.message_id, r.recipient_ref, r.recipient_name, r.state, r.state_at, r.via
               FROM agent_recipients r JOIN agent_messages m ON m.id = r.message_id
              WHERE r.recipient_ref IN (SELECT value FROM json_each(?1))
                AND r.state IN (SELECT value FROM json_each(?2))
                AND (?3 IS NULL OR m.thread_id = ?3)
                AND (?4 IS NULL OR m.seq < ?4)
              ORDER BY m.seq DESC LIMIT ?5"
        ))?;
        let rows = stmt.query_map(
            params![
                refs_json(refs),
                refs_json(&states),
                thread_id,
                before,
                page(limit)
            ],
            |row| {
                Ok(InboxRow {
                    message: message_of(row)?,
                    recipient: recipient_at(row, 16)?,
                })
            },
        )?;
        collect(rows)
    }

    /// How many messages wait for any of `refs` in `state`, and their
    /// senders' names (newest first, each once).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn waiting(
        &self,
        refs: &[String],
        state: RecipientState,
    ) -> Result<(usize, Vec<String>), CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.sender_name FROM agent_recipients r JOIN agent_messages m ON m.id = r.message_id
              WHERE r.recipient_ref IN (SELECT value FROM json_each(?1)) AND r.state = ?2
              ORDER BY m.seq DESC LIMIT 500",
        )?;
        let names = collect(
            stmt.query_map(params![refs_json(refs), state.as_str()], |row| {
                row.get::<_, String>(0)
            })?,
        )?;
        let count = names.len();
        let mut senders: Vec<String> = Vec::new();
        for name in names {
            if !senders.contains(&name) {
                senders.push(name);
            }
        }
        Ok((count, senders))
    }

    /// Moves the recipient rows of `message_ids` for any of `refs` from one
    /// of `from` to `to` (with how), and logs it once per message; how many
    /// rows moved.
    ///
    /// # Errors
    ///
    /// Storage failures.
    #[allow(clippy::too_many_arguments)]
    pub fn set_state(
        &self,
        message_ids: &[String],
        refs: &[String],
        from: &[RecipientState],
        to: RecipientState,
        via: Option<&str>,
        actor: &str,
        verb: &str,
    ) -> Result<usize, CoreError> {
        let now = Timestamp::now().millis();
        let from: Vec<String> = from.iter().map(|s| s.as_str().to_owned()).collect();
        let tx = self.conn.unchecked_transaction()?;
        let mut moved = 0;
        for id in message_ids {
            let n = tx.execute(
                "UPDATE agent_recipients SET state = ?3, state_at = ?4, via = COALESCE(?5, via)
                  WHERE message_id = ?1
                    AND recipient_ref IN (SELECT value FROM json_each(?2))
                    AND state IN (SELECT value FROM json_each(?6))",
                params![id, refs_json(refs), to.as_str(), now, via, refs_json(&from)],
            )?;
            if n > 0 {
                let pool: Option<String> = tx
                    .query_row(
                        "SELECT pool_id FROM agent_messages WHERE id = ?1",
                        params![id],
                        |row| row.get(0),
                    )
                    .optional()?;
                tx.execute(
                    "INSERT INTO agent_events (at, pool_id, actor_ref, verb, object_ref)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![now, pool.unwrap_or_default(), actor, verb, id],
                )?;
            }
            moved += n;
        }
        tx.commit()?;
        Ok(moved)
    }

    /// When `sender_ref` sent its messages in a pool since `since`.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn sent_since(
        &self,
        pool_id: &str,
        sender_ref: &str,
        since: i64,
    ) -> Result<Vec<i64>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT created_at FROM agent_messages
              WHERE pool_id = ?1 AND sender_ref = ?2 AND created_at >= ?3",
        )?;
        collect(stmt.query_map(params![pool_id, sender_ref, since], |row| row.get(0))?)
    }

    /// A message `sender_ref` sent in the pool since `since` with the same
    /// hash, when there is one.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn duplicate_of(
        &self,
        pool_id: &str,
        sender_ref: &str,
        sha256: &str,
        since: i64,
    ) -> Result<Option<Message>, CoreError> {
        Ok(self
            .conn
            .query_row(
                &format!(
                    "SELECT {MESSAGE_COLUMNS} FROM agent_messages m
                      WHERE m.pool_id = ?1 AND m.sender_ref = ?2 AND m.body_sha256 = ?3
                        AND m.created_at >= ?4 ORDER BY m.seq DESC LIMIT 1"
                ),
                params![pool_id, sender_ref, sha256, since],
                message_of,
            )
            .optional()?)
    }

    /// The pool's messages since `since`, oldest first, as the loop check
    /// sees them: (sender, recipients, when, a person wrote it).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn traffic(
        &self,
        pool_id: &str,
        since: i64,
    ) -> Result<Vec<crate::messaging::Traffic>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.sender_ref, m.sender_kind, m.created_at,
                    (SELECT json_group_array(r.recipient_ref) FROM agent_recipients r WHERE r.message_id = m.id)
               FROM agent_messages m WHERE m.pool_id = ?1 AND m.created_at >= ?2
              ORDER BY m.seq DESC LIMIT 200",
        )?;
        let mut rows = collect(stmt.query_map(params![pool_id, since], |row| {
            let to: String = row.get(3)?;
            Ok(crate::messaging::Traffic {
                from: row.get(0)?,
                by_a_person: row.get::<_, String>(1)? == SENDER_USER,
                at: row.get(2)?,
                to: serde_json::from_str(&to).unwrap_or_default(),
            })
        })?)?;
        rows.reverse();
        Ok(rows)
    }

    /// Sets a thread's state (with why), and logs it.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a thread that isn't there; storage
    /// failures.
    pub fn set_thread_state(
        &self,
        thread_id: &str,
        state: ThreadState,
        reason: Option<&str>,
        actor: &str,
    ) -> Result<(), CoreError> {
        let now = Timestamp::now().millis();
        let changed = self.conn.execute(
            "UPDATE agent_threads SET state = ?2, state_reason = ?3, state_at = ?4 WHERE id = ?1",
            params![thread_id, state.as_str(), reason, now],
        )?;
        if changed == 0 {
            return Err(super::accounts::not_found("thread", thread_id));
        }
        let pool: String = self.conn.query_row(
            "SELECT pool_id FROM agent_threads WHERE id = ?1",
            params![thread_id],
            |row| row.get(0),
        )?;
        self.log(&pool, actor, state.as_str(), thread_id, reason)
    }

    /// A person's word or click: the thread's hop goes back to 0 (D-29).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn reset_hop(&self, thread_id: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE agent_threads SET hop = 0 WHERE id = ?1",
            params![thread_id],
        )?;
        Ok(())
    }

    /// Pauses a pool's messages (the breaker, or the user), with why.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn pause_pool(&self, pool_id: &str, reason: &str, actor: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO pool_pauses (pool_id, paused_at, reason) VALUES (?1, ?2, ?3)
             ON CONFLICT(pool_id) DO UPDATE SET paused_at = excluded.paused_at, reason = excluded.reason",
            params![pool_id, Timestamp::now().millis(), reason],
        )?;
        self.log(pool_id, actor, "paused", pool_id, Some(reason))
    }

    /// The user resumes a pool's messages.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn resume_pool(&self, pool_id: &str, actor: &str) -> Result<(), CoreError> {
        if self.conn.execute(
            "DELETE FROM pool_pauses WHERE pool_id = ?1",
            params![pool_id],
        )? > 0
        {
            self.log(pool_id, actor, "resumed", pool_id, None)?;
        }
        Ok(())
    }

    /// Why a pool's messages are paused (when, why), if they are.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn pool_pause(&self, pool_id: &str) -> Result<Option<(i64, String)>, CoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT paused_at, reason FROM pool_pauses WHERE pool_id = ?1",
                params![pool_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?)
    }

    /// Every paused pool: (pool, when, why).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn paused_pools(&self) -> Result<Vec<(String, i64, String)>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT pool_id, paused_at, reason FROM pool_pauses ORDER BY paused_at")?;
        collect(stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?)
    }

    /// Paused threads, oldest pause first, at most `limit`.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn paused_threads(&self, limit: u32) -> Result<Vec<Thread>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {THREAD_COLUMNS} FROM agent_threads WHERE state = 'paused'
              ORDER BY state_at LIMIT ?1"
        ))?;
        collect(stmt.query_map(params![page(limit)], thread_of)?)
    }

    /// Writes one event.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn log(
        &self,
        pool_id: &str,
        actor: &str,
        verb: &str,
        object: &str,
        detail: Option<&str>,
    ) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO agent_events (at, pool_id, actor_ref, verb, object_ref, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                Timestamp::now().millis(),
                pool_id,
                actor,
                verb,
                object,
                detail
            ],
        )?;
        Ok(())
    }

    /// When `actor` did `verb` in a pool since `since` (the breaker counts
    /// rate-limit hits).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn verbs_since(
        &self,
        pool_id: &str,
        actor: &str,
        verb: &str,
        since: i64,
    ) -> Result<Vec<i64>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT at FROM agent_events WHERE pool_id = ?1 AND actor_ref = ?2 AND verb = ?3 AND at >= ?4",
        )?;
        collect(stmt.query_map(params![pool_id, actor, verb, since], |row| row.get(0))?)
    }

    /// One page of a pool's events, newest first; `before` is the previous
    /// page's last id.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn events(
        &self,
        pool_id: &str,
        before: Option<i64>,
        limit: u32,
    ) -> Result<Vec<MessageEvent>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, at, pool_id, actor_ref, verb, object_ref, detail FROM agent_events
              WHERE pool_id = ?1 AND (?2 IS NULL OR id < ?2) ORDER BY id DESC LIMIT ?3",
        )?;
        collect(
            stmt.query_map(params![pool_id, before, page(limit)], |row| {
                Ok(MessageEvent {
                    id: row.get(0)?,
                    at: row.get(1)?,
                    pool_id: row.get(2)?,
                    actor_ref: row.get(3)?,
                    verb: row.get(4)?,
                    object_ref: row.get(5)?,
                    detail: row.get(6)?,
                })
            })?,
        )
    }

    /// A pool's messages whose subject or body has every word of `query`
    /// (the transcript index's tokenizer), newest first.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn search(
        &self,
        pool_id: &str,
        query: &str,
        limit: u32,
    ) -> Result<Vec<Message>, CoreError> {
        let Some(expression) = super::fts_expression(query) else {
            return Ok(Vec::new());
        };
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {MESSAGE_COLUMNS} FROM agent_messages_fts f
               JOIN agent_messages m ON m.seq = f.rowid
              WHERE agent_messages_fts MATCH ?1 AND m.pool_id = ?2
              ORDER BY m.seq DESC LIMIT ?3"
        ))?;
        collect(stmt.query_map(params![expression, pool_id, page(limit)], message_of)?)
    }

    /// Records a shared doc, and logs it.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn insert_doc(&self, doc: &Doc) -> Result<(), CoreError> {
        self.conn.execute(
            &format!(
                "INSERT INTO agent_docs ({DOC_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
            ),
            params![
                doc.id,
                doc.pool_id,
                doc.sha256,
                doc.name,
                doc.mime,
                doc.size_bytes,
                doc.shared_by,
                doc.shared_by_name,
                doc.policy,
                doc.withheld,
                doc.created_at
            ],
        )?;
        self.log(
            &doc.pool_id,
            &doc.shared_by,
            "doc_shared",
            &doc.id,
            Some(&doc.name),
        )
    }

    /// One page of a pool's docs, newest first; `before` is the previous
    /// page's last doc's position (from [`Self::doc_seq`]).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn docs(
        &self,
        pool_id: &str,
        before: Option<i64>,
        limit: u32,
    ) -> Result<Vec<(i64, Doc)>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {DOC_COLUMNS}, seq FROM agent_docs
              WHERE pool_id = ?1 AND (?2 IS NULL OR seq < ?2) ORDER BY seq DESC LIMIT ?3"
        ))?;
        collect(
            stmt.query_map(params![pool_id, before, page(limit)], |row| {
                Ok((row.get(11)?, doc_of(row)?))
            })?,
        )
    }

    /// A pool's doc, when it's there.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn doc(&self, pool_id: &str, id: &str) -> Result<Option<Doc>, CoreError> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {DOC_COLUMNS} FROM agent_docs WHERE pool_id = ?1 AND id = ?2"),
                params![pool_id, id],
                doc_of,
            )
            .optional()?)
    }

    /// Whether any doc record still names the bytes `sha256`.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn doc_bytes_named(&self, sha256: &str) -> Result<bool, CoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM agent_docs WHERE sha256 = ?1",
            params![sha256],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// The user links two pools: their members may message each other.
    ///
    /// # Errors
    ///
    /// [`CoreError::Validation`] for a pool linked to itself; storage
    /// failures.
    pub fn link(&self, a: &str, b: &str) -> Result<(), CoreError> {
        if a == b {
            return Err(CoreError::Validation {
                subject: "link",
                message: "a pool is already linked to itself".into(),
            });
        }
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        self.conn.execute(
            "INSERT OR IGNORE INTO pool_links (a_pool, b_pool, approved_at) VALUES (?1, ?2, ?3)",
            params![a, b, Timestamp::now().millis()],
        )?;
        self.log(a, SENDER_USER, "linked", b, None)
    }

    /// The user takes a link back (it stops at once).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn unlink(&self, a: &str, b: &str) -> Result<(), CoreError> {
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        if self.conn.execute(
            "UPDATE pool_links SET revoked_at = ?3 WHERE a_pool = ?1 AND b_pool = ?2 AND revoked_at IS NULL",
            params![a, b, Timestamp::now().millis()],
        )? > 0
        {
            self.log(a, SENDER_USER, "unlinked", b, None)?;
        }
        Ok(())
    }

    /// The pools linked with `pool_id` now.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn links(&self, pool_id: &str) -> Result<Vec<String>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT CASE WHEN a_pool = ?1 THEN b_pool ELSE a_pool END FROM pool_links
              WHERE revoked_at IS NULL AND (a_pool = ?1 OR b_pool = ?1) ORDER BY approved_at",
        )?;
        collect(stmt.query_map(params![pool_id], |row| row.get(0))?)
    }

    /// Records an agent's action that waits on the user's OK.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn hold(&self, ask: &HeldAsk) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO agent_asks (id, pool_id, actor_ref, actor_name, action, object_ref, summary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                ask.id,
                ask.pool_id,
                ask.actor_ref,
                ask.actor_name,
                ask.action,
                ask.object_ref,
                ask.summary,
                ask.created_at
            ],
        )?;
        Ok(())
    }

    /// The asks still open, oldest first, one page.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn open_asks(&self, offset: u32, limit: u32) -> Result<Vec<HeldAsk>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pool_id, actor_ref, actor_name, action, object_ref, summary, created_at
               FROM agent_asks WHERE answered_at IS NULL ORDER BY seq LIMIT ?1 OFFSET ?2",
        )?;
        collect(stmt.query_map(params![page(limit), offset], |row| {
            Ok(HeldAsk {
                id: row.get(0)?,
                pool_id: row.get(1)?,
                actor_ref: row.get(2)?,
                actor_name: row.get(3)?,
                action: row.get(4)?,
                object_ref: row.get(5)?,
                summary: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?)
    }

    /// Closes an open ask with the user's answer; `false` when it wasn't
    /// open.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn close_ask(&self, id: &str, answer: &str) -> Result<bool, CoreError> {
        Ok(self.conn.execute(
            "UPDATE agent_asks SET answered_at = ?2, answer = ?3 WHERE id = ?1 AND answered_at IS NULL",
            params![id, Timestamp::now().millis(), answer],
        )? > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    fn storage() -> Storage {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .pools()
            .create("Team", None)
            .map(|_| ())
            .unwrap_or(());
        storage
    }

    fn pool(storage: &Storage) -> String {
        storage
            .pools()
            .list()
            .unwrap()
            .into_iter()
            .find(|pool| pool.name == "Team")
            .unwrap()
            .id
    }

    fn send(
        repo: &MessagesRepo<'_>,
        id: &str,
        thread: &str,
        pool: &str,
        from: &str,
        to: &str,
        body: &str,
    ) -> Message {
        let recipients = vec![(to.to_owned(), to.to_owned(), RecipientState::Unread)];
        repo.insert_message(&NewMessage {
            id,
            thread_id: thread,
            pool_id: pool,
            sender_ref: from,
            sender_kind: "agent",
            sender_name: from,
            kind: "note",
            subject: "Start on the API",
            body,
            body_sha256: body,
            hop: 1,
            reply_to: None,
            to_all: false,
            redacted: 0,
            recipients: &recipients,
        })
        .unwrap()
    }

    #[test]
    fn a_message_moves_its_thread_and_reaches_its_recipient() {
        let storage = storage();
        let pool = pool(&storage);
        let repo = storage.messages();
        let thread = repo
            .create_thread("thr-1", &pool, "Start", "session:a")
            .unwrap();
        assert_eq!(
            (thread.count, thread.hop, thread.state),
            (0, 0, ThreadState::Open)
        );
        let sent = send(
            &repo,
            "msg-1",
            "thr-1",
            &pool,
            "session:a",
            "session:b",
            "the schema",
        );
        let thread = repo.thread("thr-1").unwrap().unwrap();
        assert_eq!(
            (thread.count, thread.hop, thread.last_seq),
            (1, 1, sent.seq)
        );
        let refs = vec!["session:b".to_owned()];
        let inbox = repo
            .inbox(&refs, &[RecipientState::Unread], None, None, 10)
            .unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].message.body, "the schema");
        assert_eq!(
            repo.waiting(&refs, RecipientState::Unread).unwrap(),
            (1, vec!["session:a".to_owned()])
        );
        let moved = repo
            .set_state(
                &["msg-1".to_owned()],
                &refs,
                &[RecipientState::Unread],
                RecipientState::Read,
                Some("pull"),
                "session:b",
                "read",
            )
            .unwrap();
        assert_eq!(moved, 1);
        assert!(
            repo.inbox(&refs, &[RecipientState::Unread], None, None, 10)
                .unwrap()
                .is_empty()
        );
        // Only the recipient's row moves.
        let other = vec!["session:c".to_owned()];
        assert_eq!(
            repo.set_state(
                &["msg-1".to_owned()],
                &other,
                &[RecipientState::Read],
                RecipientState::Acked,
                None,
                "session:c",
                "acked"
            )
            .unwrap(),
            0
        );
        let threads = repo.threads(Some(&pool), Some(&refs), None, 10).unwrap();
        assert_eq!(threads.len(), 1);
        assert!(
            repo.threads(Some(&pool), Some(&other), None, 10)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn messages_and_the_log_cant_be_rewritten() {
        let storage = storage();
        let pool = pool(&storage);
        let repo = storage.messages();
        repo.create_thread("thr-1", &pool, "Start", "session:a")
            .unwrap();
        send(
            &repo,
            "msg-1",
            "thr-1",
            &pool,
            "session:a",
            "session:b",
            "x",
        );
        let conn = storage.raw_conn();
        assert!(
            conn.execute("UPDATE agent_messages SET body = 'changed'", [])
                .is_err()
        );
        assert!(
            conn.execute("UPDATE agent_events SET verb = 'x'", [])
                .is_err()
        );
        assert!(conn.execute("DELETE FROM agent_events", []).is_err());
        let events = repo.events(&pool, None, 10).unwrap();
        assert_eq!(events[0].verb, "sent");
        assert_eq!(events[0].detail.as_deref(), Some("to session:b"));
    }

    #[test]
    fn search_uses_the_index_and_stays_in_its_pool() {
        let storage = storage();
        let pool = pool(&storage);
        let repo = storage.messages();
        repo.create_thread("thr-1", &pool, "Start", "session:a")
            .unwrap();
        send(
            &repo,
            "msg-1",
            "thr-1",
            &pool,
            "session:a",
            "session:b",
            "The schéma is in the shared doc",
        );
        assert_eq!(repo.search(&pool, "schema shared", 10).unwrap().len(), 1);
        assert!(repo.search(&pool, "billing", 10).unwrap().is_empty());
        assert!(
            repo.search("pool-default:none", "schema", 10)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn links_are_symmetric_and_revocable_and_pauses_are_kept() {
        let storage = storage();
        let pool = pool(&storage);
        let repo = storage.messages();
        repo.link(&pool, "pool-default:none").unwrap();
        assert_eq!(repo.links("pool-default:none").unwrap(), [pool.clone()]);
        assert!(repo.link(&pool, &pool).is_err());
        repo.unlink("pool-default:none", &pool).unwrap();
        assert!(repo.links(&pool).unwrap().is_empty());
        repo.pause_pool(&pool, "loop", "system").unwrap();
        assert_eq!(repo.pool_pause(&pool).unwrap().unwrap().1, "loop");
        repo.resume_pool(&pool, "user").unwrap();
        assert!(repo.pool_pause(&pool).unwrap().is_none());
    }

    #[test]
    fn a_thousand_messages_page_quickly() {
        let storage = storage();
        let pool = pool(&storage);
        let repo = storage.messages();
        for thread in 0..50 {
            repo.create_thread(&format!("thr-{thread}"), &pool, "Work", "session:a")
                .unwrap();
        }
        for n in 0..1000 {
            send(
                &repo,
                &format!("msg-{n}"),
                &format!("thr-{}", n % 50),
                &pool,
                "session:a",
                "session:b",
                &format!("body {n}"),
            );
        }
        let started = std::time::Instant::now();
        let threads = repo.threads(Some(&pool), None, None, 30).unwrap();
        let page = repo.thread_messages(&threads[0].id, None, 30).unwrap();
        let inbox = repo
            .inbox(
                &["session:b".to_owned()],
                &[RecipientState::Unread],
                None,
                None,
                20,
            )
            .unwrap();
        let took = started.elapsed();
        assert_eq!((threads.len(), page.len(), inbox.len()), (30, 20, 20));
        let next = repo
            .threads(Some(&pool), None, Some(threads[29].last_seq), 30)
            .unwrap();
        assert_eq!(next.len(), 20, "the rest of the 50");
        assert!(took < std::time::Duration::from_millis(100), "{took:?}");
    }
}
