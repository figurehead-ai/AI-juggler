//! The pool canvas (0.3, redesign §5; schema v38): where the user put each
//! member on a pool's canvas, and the narrow reads the canvas draws its
//! edges and its live stream from.
//!
//! - **Positions** are the user's: one row a member of a pool, keyed by
//!   what the member stands for (its Work item's ref, else its session's),
//!   so a switch keeps the node where it was. "Tidy up" forgets a pool's
//!   rows and the app lays the pool out again. Nothing an agent runs
//!   reaches them.
//! - **Edges** are counted, never listed: messages from one member to
//!   another and tasks one handed another, since a given time, grouped by
//!   the pair (I6: never the whole history).
//! - **The stream** is the pool's newest messages and task events, a few
//!   at a time.
//!
//! Nothing here starts, stops or types into a session, and nothing reads
//! quota (D-31).

use rusqlite::{Connection, params};

use crate::error::CoreError;
use crate::model::Timestamp;

/// v38: the canvas positions, and an index so a pool's task events are
/// read without walking every pool's.
pub(super) const MIGRATION: &str = "
CREATE TABLE pool_layout (
    pool_id    TEXT NOT NULL REFERENCES pools(id),
    member_ref TEXT NOT NULL,
    x          REAL NOT NULL,
    y          REAL NOT NULL,
    placed_at  INTEGER NOT NULL,
    PRIMARY KEY (pool_id, member_ref)
);
CREATE INDEX idx_task_events_pool ON task_events(pool_id, id);
";

/// The farthest a node may sit from the canvas's middle, either way (a
/// position past it is a bug or a hand-made row, never the user's drag).
pub const POSITION_LIMIT: f64 = 100_000.0;

/// Where the user put one member.
#[derive(Clone, Debug, PartialEq)]
pub struct NodePosition {
    /// What the member stands for (`work:<id>` | `session:<id>`).
    pub member_ref: String,
    /// Across, in canvas units.
    pub x: f64,
    /// Down, in canvas units.
    pub y: f64,
}

/// Messages from one member to another since a time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageTraffic {
    /// The sender's ref.
    pub from_ref: String,
    /// The recipient's ref.
    pub to_ref: String,
    /// How many.
    pub count: u32,
    /// The newest, epoch ms.
    pub last_at: i64,
    /// The newest one's thread.
    pub thread_id: String,
}

/// Tasks one member gave another (`assigned`) since a time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskHandoff {
    /// Who gave it.
    pub from_ref: String,
    /// Who holds it now.
    pub to_ref: String,
    /// How many.
    pub count: u32,
    /// The newest, epoch ms.
    pub last_at: i64,
}

/// A message one member hasn't got yet: waiting in its inbox (`unread`)
/// or held for the user's answer (`held`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingMessage {
    /// The message.
    pub message_id: String,
    /// Its thread.
    pub thread_id: String,
    /// Its subject.
    pub subject: String,
    /// The sender's ref.
    pub from_ref: String,
    /// The recipient's ref (what Deliver names).
    pub to_ref: String,
    /// `unread` | `held`.
    pub state: String,
    /// When it was sent, epoch ms.
    pub at: i64,
}

/// One line of a pool's live stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamEntry {
    /// When, epoch ms.
    pub at: i64,
    /// `message` | `task`.
    pub source: &'static str,
    /// Who did it (a member's ref, or `user`).
    pub actor_ref: String,
    /// Their name then.
    pub actor_name: String,
    /// For a message: its recipients' refs and names; for a task event:
    /// none (the holder is in `detail` where it matters).
    pub to: Vec<(String, String)>,
    /// A message's kind, or a task event's verb.
    pub verb: String,
    /// The message's subject, or the task's title.
    pub title: String,
    /// The event's detail (a task's new holder, a status change).
    pub detail: Option<String>,
}

/// Typed access to the canvas's table and reads.
#[derive(Debug)]
pub struct PoolCanvasRepo<'a> {
    conn: &'a Connection,
}

fn collect<T>(rows: impl Iterator<Item = rusqlite::Result<T>>) -> Result<Vec<T>, CoreError> {
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn count(n: i64) -> u32 {
    u32::try_from(n.max(0)).unwrap_or(u32::MAX)
}

impl PoolCanvasRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> PoolCanvasRepo<'_> {
        PoolCanvasRepo { conn }
    }

    /// Where the user put `pool_id`'s members.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn positions(&self, pool_id: &str) -> Result<Vec<NodePosition>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT member_ref, x, y FROM pool_layout WHERE pool_id = ?1 ORDER BY member_ref",
        )?;
        collect(stmt.query_map(params![pool_id], |row| {
            Ok(NodePosition {
                member_ref: row.get(0)?,
                x: row.get(1)?,
                y: row.get(2)?,
            })
        })?)
    }

    /// Puts `member_ref`'s node at (`x`, `y`) on `pool_id`'s canvas.
    ///
    /// # Errors
    ///
    /// [`CoreError::Validation`] for a position that isn't a finite number
    /// within [`POSITION_LIMIT`]; [`CoreError::Storage`] on SQLite failures.
    pub fn place(&self, pool_id: &str, member_ref: &str, x: f64, y: f64) -> Result<(), CoreError> {
        if !(x.is_finite() && y.is_finite()) || x.abs() > POSITION_LIMIT || y.abs() > POSITION_LIMIT
        {
            return Err(CoreError::Validation {
                subject: "node position",
                message: "a node's place is a number within the canvas".to_owned(),
            });
        }
        self.conn.execute(
            "INSERT INTO pool_layout (pool_id, member_ref, x, y, placed_at) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (pool_id, member_ref) DO UPDATE SET x = ?3, y = ?4, placed_at = ?5",
            params![pool_id, member_ref, x, y, Timestamp::now().millis()],
        )?;
        Ok(())
    }

    /// Forgets where the user put `pool_id`'s members ("Tidy up"); returns
    /// how many it forgot.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn forget(&self, pool_id: &str) -> Result<usize, CoreError> {
        Ok(self.conn.execute(
            "DELETE FROM pool_layout WHERE pool_id = ?1",
            params![pool_id],
        )?)
    }

    /// Agents' messages to other members in `pool_id` since `since`, one
    /// row a (sender, recipient) pair, the busiest first, at most `limit`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn message_traffic(
        &self,
        pool_id: &str,
        since: i64,
        limit: u32,
    ) -> Result<Vec<MessageTraffic>, CoreError> {
        // One MAX() aggregate: SQLite takes the bare `thread_id` from the
        // newest row of each pair.
        let mut stmt = self.conn.prepare(
            "SELECT m.sender_ref, r.recipient_ref, COUNT(*), MAX(m.created_at), m.thread_id
               FROM agent_messages m JOIN agent_recipients r ON r.message_id = m.id
              WHERE m.pool_id = ?1 AND m.created_at >= ?2 AND m.sender_kind = 'agent'
                AND r.recipient_ref != 'user' AND r.recipient_ref != m.sender_ref
              GROUP BY m.sender_ref, r.recipient_ref
              ORDER BY COUNT(*) DESC, MAX(m.created_at) DESC LIMIT ?3",
        )?;
        collect(
            stmt.query_map(params![pool_id, since, i64::from(limit)], |row| {
                Ok(MessageTraffic {
                    from_ref: row.get(0)?,
                    to_ref: row.get(1)?,
                    count: count(row.get(2)?),
                    last_at: row.get(3)?,
                    thread_id: row.get(4)?,
                })
            })?,
        )
    }

    /// Tasks in `pool_id` one member gave another since `since` and that
    /// one still holds, one row a (giver, holder) pair, at most `limit`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn task_handoffs(
        &self,
        pool_id: &str,
        since: i64,
        limit: u32,
    ) -> Result<Vec<TaskHandoff>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT e.actor_ref, t.assignee_ref, COUNT(*), MAX(e.at)
               FROM task_events e JOIN tasks t ON t.id = e.task_id
              WHERE e.pool_id = ?1 AND e.verb = 'assigned' AND e.at >= ?2
                AND t.assignee_ref IS NOT NULL AND e.detail = t.assignee_name
                AND e.actor_ref != t.assignee_ref AND e.actor_ref != 'user'
              GROUP BY e.actor_ref, t.assignee_ref
              ORDER BY MAX(e.at) DESC LIMIT ?3",
        )?;
        collect(
            stmt.query_map(params![pool_id, since, i64::from(limit)], |row| {
                Ok(TaskHandoff {
                    from_ref: row.get(0)?,
                    to_ref: row.get(1)?,
                    count: count(row.get(2)?),
                    last_at: row.get(3)?,
                })
            })?,
        )
    }

    /// Agents' messages in `pool_id` since `since` that a member hasn't
    /// got yet (waiting in its inbox, or held for the user), newest first,
    /// at most `limit`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn pending(
        &self,
        pool_id: &str,
        since: i64,
        limit: u32,
    ) -> Result<Vec<PendingMessage>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.thread_id, m.subject, m.sender_ref, r.recipient_ref, r.state, m.created_at
               FROM agent_messages m JOIN agent_recipients r ON r.message_id = m.id
              WHERE m.pool_id = ?1 AND m.created_at >= ?2 AND m.sender_kind = 'agent'
                AND r.state IN ('unread', 'held') AND r.recipient_ref != 'user'
              ORDER BY m.seq DESC LIMIT ?3",
        )?;
        collect(
            stmt.query_map(params![pool_id, since, i64::from(limit)], |row| {
                Ok(PendingMessage {
                    message_id: row.get(0)?,
                    thread_id: row.get(1)?,
                    subject: row.get(2)?,
                    from_ref: row.get(3)?,
                    to_ref: row.get(4)?,
                    state: row.get(5)?,
                    at: row.get(6)?,
                })
            })?,
        )
    }

    /// `pool_id`'s newest messages and task events, newest first, at most
    /// `limit` in all.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn stream(&self, pool_id: &str, limit: u32) -> Result<Vec<StreamEntry>, CoreError> {
        let limit = i64::from(limit.clamp(1, 100));
        let mut messages = self.conn.prepare(
            "SELECT m.created_at, m.sender_ref, m.sender_name, m.kind, m.subject,
                    (SELECT json_group_array(json_array(r.recipient_ref, r.recipient_name))
                       FROM agent_recipients r WHERE r.message_id = m.id)
               FROM agent_messages m WHERE m.pool_id = ?1
              ORDER BY m.created_at DESC, m.seq DESC LIMIT ?2",
        )?;
        let mut out = collect(messages.query_map(params![pool_id, limit], |row| {
            let to: String = row.get(5)?;
            Ok(StreamEntry {
                at: row.get(0)?,
                source: "message",
                actor_ref: row.get(1)?,
                actor_name: row.get(2)?,
                to: serde_json::from_str::<Vec<(String, String)>>(&to).unwrap_or_default(),
                verb: row.get(3)?,
                title: row.get(4)?,
                detail: None,
            })
        })?)?;
        let mut tasks = self.conn.prepare(
            "SELECT e.at, e.actor_ref, e.actor_name, e.verb, t.title, e.detail
               FROM task_events e JOIN tasks t ON t.id = e.task_id
              WHERE e.pool_id = ?1 ORDER BY e.id DESC LIMIT ?2",
        )?;
        out.extend(collect(tasks.query_map(
            params![pool_id, limit],
            |row| {
                Ok(StreamEntry {
                    at: row.get(0)?,
                    source: "task",
                    actor_ref: row.get(1)?,
                    actor_name: row.get(2)?,
                    to: Vec::new(),
                    verb: row.get(3)?,
                    title: row.get(4)?,
                    detail: row.get(5)?,
                })
            },
        )?)?);
        out.sort_by_key(|entry| std::cmp::Reverse(entry.at));
        out.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
        Ok(out)
    }

    /// How many tasks are in progress in each of `pool_ids` ("tasks
    /// moving" on the Pools screen); a pool with none is left out.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn moving(&self, pool_ids: &[String]) -> Result<Vec<(String, u32)>, CoreError> {
        let ids = serde_json::to_string(pool_ids).unwrap_or_else(|_| "[]".to_owned());
        let mut stmt = self.conn.prepare(
            "SELECT pool_id, COUNT(*) FROM tasks
              WHERE pool_id IN (SELECT value FROM json_each(?1)) AND status = 'in_progress'
              GROUP BY pool_id",
        )?;
        collect(stmt.query_map(params![ids], |row| {
            Ok((row.get::<_, String>(0)?, count(row.get(1)?)))
        })?)
    }
}
