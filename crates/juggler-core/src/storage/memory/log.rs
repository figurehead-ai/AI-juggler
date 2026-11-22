//! The append-only AI log (ADR-0037 §7): a text-free spine and erasable
//! text sidecars.
//!
//! - **The spine** (`ai_log`) is never updated, and a row is deleted only
//!   once it is older than its stream's watermark (`ai_log_prune`, which
//!   only moves forward); triggers hold both. Its payload is JSON of ids,
//!   codes and numbers only: [`text_free`] refuses any string that isn't
//!   code-like, so a sentence can't ride in it.
//! - **A sidecar** (`ai_log_text`) holds the few words an event needs (a
//!   "this is wrong" item), secrets masked, at most 2 KB, with an expiry.
//!   It names a memory subject by its random id, so forgetting the subject
//!   erases its sidecars (`MemoryStore::forget`).

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::secure::SecureHold;
use super::types::{MemoryError, Refusal, SecretScan, code_like};
use crate::storage::write_transaction;

/// The longest code a spine field or payload string may be.
const CODE_MAX: usize = 96;
/// A sidecar's largest body, in bytes.
pub const SIDECAR_MAX_BYTES: usize = 2048;

/// The log's streams (ADR-0037 §7.2; 0.6 adds the bot's).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stream {
    /// Watch changes and fires.
    Watch,
    /// Alerts sent, held, released, consolidated.
    Alert,
    /// Digests made.
    Digest,
    /// "This is wrong".
    Feedback,
    /// Chat turns.
    Chat,
    /// Memory changes.
    Memory,
    /// The context assembler.
    Assembly,
}

impl Stream {
    /// The stored code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Watch => "watch",
            Self::Alert => "alert",
            Self::Digest => "digest",
            Self::Feedback => "feedback",
            Self::Chat => "chat",
            Self::Memory => "memory",
            Self::Assembly => "assembly",
        }
    }
}

/// Who did it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Actor {
    /// The user.
    User,
    /// AI Juggler's code.
    App,
    /// A model, by engine id (a code).
    Model(String),
}

impl Actor {
    fn code(&self) -> String {
        match self {
            Self::User => "user".to_owned(),
            Self::App => "app".to_owned(),
            Self::Model(engine) => format!("model:{engine}"),
        }
    }
}

/// An event to append.
#[derive(Clone, Debug)]
pub struct NewEvent {
    /// When it happened.
    pub at_ms: i64,
    /// Its stream.
    pub stream: Stream,
    /// The verb, a code (`remembered`, `fired`, …).
    pub verb: &'static str,
    /// Who did it.
    pub actor: Actor,
    /// What it concerns: `(kind, id)`, both codes.
    pub subject: Option<(&'static str, String)>,
    /// The memory subject's random id, so forget finds its sidecars.
    pub memory_subject: Option<String>,
    /// The record version it concerns.
    pub version: Option<i64>,
    /// Groups one turn, one digest, one fire and its alert (a code).
    pub correlation: Option<String>,
    /// The event that caused this one.
    pub cause_seq: Option<i64>,
    /// The payload type's version.
    pub schema: i64,
    /// Ids, codes and numbers only ([`text_free`]).
    pub payload: serde_json::Value,
}

impl NewEvent {
    /// An event with no subject and an empty payload.
    #[must_use]
    pub fn new(at_ms: i64, stream: Stream, verb: &'static str, actor: Actor) -> Self {
        Self {
            at_ms,
            stream,
            verb,
            actor,
            subject: None,
            memory_subject: None,
            version: None,
            correlation: None,
            cause_seq: None,
            schema: 1,
            payload: serde_json::json!({}),
        }
    }
}

/// The words an event carries, kept apart so they can be erased.
#[derive(Clone, Debug)]
pub struct NewSidecar<'a> {
    /// The words (secrets are masked before they're stored).
    pub body: &'a str,
    /// Agent-written.
    pub untrusted: bool,
    /// When they go.
    pub expires_ms: i64,
}

/// A spine row as read back.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    /// Its place in the log.
    pub seq: i64,
    /// When it happened.
    pub at_ms: i64,
    /// Its stream code.
    pub stream: String,
    /// Its verb.
    pub verb: String,
    /// Its actor code.
    pub actor: String,
    /// What it concerns.
    pub subject: Option<(String, String)>,
    /// The memory subject's random id.
    pub memory_subject: Option<String>,
    /// The record version.
    pub version: Option<i64>,
    /// Its correlation.
    pub correlation: Option<String>,
    /// Its cause.
    pub cause_seq: Option<i64>,
    /// The payload's version.
    pub schema: i64,
    /// The payload.
    pub payload: serde_json::Value,
}

/// A sidecar as read back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sidecar {
    /// The words.
    pub body: String,
    /// Agent-written.
    pub untrusted: bool,
    /// When they go.
    pub expires_ms: i64,
}

/// The JSON holds no free text: every key and string is code-like.
#[must_use]
pub fn text_free(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(s) => s.is_empty() || code_like(s, CODE_MAX),
        serde_json::Value::Array(items) => items.iter().all(text_free),
        serde_json::Value::Object(map) => map
            .iter()
            .all(|(key, item)| code_like(key, CODE_MAX) && text_free(item)),
        _ => true,
    }
}

fn check(event: &NewEvent) -> Result<(), Refusal> {
    let mut codes: Vec<&str> = vec![event.verb];
    codes.extend(event.correlation.as_deref());
    codes.extend(event.memory_subject.as_deref());
    if let Some((kind, id)) = &event.subject {
        codes.extend([*kind, id.as_str()]);
    }
    if let Actor::Model(engine) = &event.actor {
        codes.push(engine);
    }
    if codes.iter().all(|code| code_like(code, CODE_MAX)) && text_free(&event.payload) {
        Ok(())
    } else {
        Err(Refusal::NotTextFree)
    }
}

/// Appends inside the caller's transaction (a record change and its event
/// commit together, ADR-0037 §7.1).
pub(super) fn append_in(
    conn: &Connection,
    event: &NewEvent,
    sidecar: Option<(&NewSidecar<'_>, &dyn SecretScan)>,
) -> Result<i64, MemoryError> {
    check(event).map_err(MemoryError::Refused)?;
    let payload = serde_json::to_string(&event.payload)?;
    conn.execute(
        "INSERT INTO ai_log (at_ms, stream, verb, actor, subject_kind, subject_id, memory_subject,
                             version, correlation, cause_seq, schema, payload)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            event.at_ms,
            event.stream.as_str(),
            event.verb,
            event.actor.code(),
            event.subject.as_ref().map(|s| s.0),
            event.subject.as_ref().map(|s| s.1.as_str()),
            event.memory_subject,
            event.version,
            event.correlation,
            event.cause_seq,
            event.schema,
            payload,
        ],
    )?;
    let seq = conn.last_insert_rowid();
    if let Some((sidecar, secrets)) = sidecar {
        let body = secrets.mask(sidecar.body);
        let body = truncate_bytes(&body, SIDECAR_MAX_BYTES);
        conn.execute(
            "INSERT INTO ai_log_text (seq, memory_subject, body, untrusted, expires_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                seq,
                event.memory_subject,
                body,
                sidecar.untrusted,
                sidecar.expires_ms
            ],
        )?;
    }
    Ok(seq)
}

fn truncate_bytes(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn event_row(row: &Row<'_>) -> rusqlite::Result<Event> {
    let kind: Option<String> = row.get(5)?;
    let id: Option<String> = row.get(6)?;
    let payload: String = row.get(12)?;
    Ok(Event {
        seq: row.get(0)?,
        at_ms: row.get(1)?,
        stream: row.get(2)?,
        verb: row.get(3)?,
        actor: row.get(4)?,
        subject: kind.zip(id),
        memory_subject: row.get(7)?,
        version: row.get(8)?,
        correlation: row.get(9)?,
        cause_seq: row.get(10)?,
        schema: row.get(11)?,
        payload: serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null),
    })
}

/// The AI log over one connection (under the caller's writer lock,
/// ADR-0030).
#[derive(Debug)]
pub struct AiLog<'c> {
    conn: &'c Connection,
}

impl<'c> AiLog<'c> {
    /// The log on `conn` (V54's tables must be there).
    #[must_use]
    pub const fn new(conn: &'c Connection) -> Self {
        Self { conn }
    }

    /// Appends one event, with its words in a sidecar when it has any.
    ///
    /// # Errors
    ///
    /// [`MemoryError::Refused`] with [`Refusal::NotTextFree`] when a code
    /// or the payload holds free text; storage failures.
    pub fn append(
        &self,
        event: &NewEvent,
        sidecar: Option<(&NewSidecar<'_>, &dyn SecretScan)>,
    ) -> Result<i64, MemoryError> {
        let _hold = SecureHold::on(self.conn)?;
        let tx = write_transaction(self.conn)?;
        let seq = append_in(&tx, event, sidecar)?;
        tx.commit()?;
        Ok(seq)
    }

    /// Up to `limit` events after `after_seq`, oldest first; one stream or all.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn read(
        &self,
        stream: Option<Stream>,
        after_seq: i64,
        limit: u32,
    ) -> Result<Vec<Event>, MemoryError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT seq, at_ms, stream, verb, actor, subject_kind, subject_id, memory_subject,
                    version, correlation, cause_seq, schema, payload
             FROM ai_log WHERE seq > ?1 AND (?2 IS NULL OR stream = ?2)
             ORDER BY seq LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(
                params![after_seq, stream.map(Stream::as_str), limit],
                event_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Up to `limit` events about one subject (`subject_kind`, `subject_id`),
    /// newest first (the Watches view's history of one watch, AJ-E3814).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn of_subject(
        &self,
        subject_kind: &str,
        subject_id: &str,
        limit: u32,
    ) -> Result<Vec<Event>, MemoryError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT seq, at_ms, stream, verb, actor, subject_kind, subject_id, memory_subject,
                    version, correlation, cause_seq, schema, payload
             FROM ai_log WHERE subject_kind = ?1 AND subject_id = ?2
             ORDER BY seq DESC LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(params![subject_kind, subject_id, limit], event_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// How many events about one subject have `verb` ("told 2 times").
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn count_of_subject(
        &self,
        subject_kind: &str,
        subject_id: &str,
        verb: &str,
    ) -> Result<u64, MemoryError> {
        let count: i64 = self.conn.query_row(
            "SELECT count(*) FROM ai_log WHERE subject_kind = ?1 AND subject_id = ?2 AND verb = ?3",
            params![subject_kind, subject_id, verb],
            |row| row.get(0),
        )?;
        Ok(u64::try_from(count).unwrap_or(0))
    }

    /// An event's words, while they are kept.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn sidecar(&self, seq: i64) -> Result<Option<Sidecar>, MemoryError> {
        Ok(self
            .conn
            .query_row(
                "SELECT body, untrusted, expires_ms FROM ai_log_text WHERE seq = ?1",
                [seq],
                |row| {
                    Ok(Sidecar {
                        body: row.get(0)?,
                        untrusted: row.get(1)?,
                        expires_ms: row.get(2)?,
                    })
                },
            )
            .optional()?)
    }

    /// Moves `stream`'s watermark to `before_ms` (never back) and removes
    /// its spine rows (and their sidecars) older than it. Returns how many
    /// spine rows went.
    ///
    /// # Errors
    ///
    /// Storage failures; the trigger's refusal when `before_ms` is behind
    /// the watermark.
    pub fn prune(&self, stream: Stream, before_ms: i64) -> Result<usize, MemoryError> {
        let _hold = SecureHold::on(self.conn)?;
        let tx = write_transaction(self.conn)?;
        tx.execute(
            "INSERT INTO ai_log_prune (stream, before_ms) VALUES (?1, ?2)
             ON CONFLICT (stream) DO UPDATE SET before_ms = excluded.before_ms",
            params![stream.as_str(), before_ms],
        )?;
        let gone = tx.execute(
            "DELETE FROM ai_log WHERE stream = ?1 AND at_ms < ?2",
            params![stream.as_str(), before_ms],
        )?;
        tx.commit()?;
        Ok(gone)
    }

    /// Erases the sidecars whose time is up. Returns how many went.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn expire_sidecars(&self, now_ms: i64) -> Result<usize, MemoryError> {
        let _hold = SecureHold::on(self.conn)?;
        Ok(self
            .conn
            .execute("DELETE FROM ai_log_text WHERE expires_ms <= ?1", [now_ms])?)
    }
}
