//! Bot's world model on this store (ADR-0038, AJ-E3832): gyrus's tables
//! live in this database (V56), and every write of them passes this
//! store's write gate, in one short hold with its log event, as a memory
//! record's does (ADR-0037 §1.3):
//!
//! 1. **The writer** covers the world model ([`KindWriter::covers_world`]).
//! 2. **The pack** declares every node kind, link kind and its ends; the
//!    bounds hold; an agent's node is untrusted by type (gyrus's checks).
//! 3. **No secrets:** the D-14 pass over every label, alias, property and
//!    provenance; a hit refuses the whole batch.
//! 4. **Authority:** a change never comes from a lower authority than the
//!    node's (gyrus, in the transaction).
//! 5. **Committed, then published:** the change set reaches the in-memory
//!    index only after the commit ([`gyrus::Graph::publish`]).
//!
//! Forget reaches the graph from [`MemoryStore::forget`]: a forgotten
//! subject's `memory` node (keyed by its random subject id, never its
//! words) and the nodes of the Chat turns it redacted go in the same
//! transaction, with `secure_delete` on and the keyword buckets that held
//! them rebuilt; then the WAL is truncated; then the in-memory removal.

use rusqlite::Connection;

use super::log::{Actor, NewEvent, Stream, append_in};
use super::secure::SecureHold;
use super::store::MemoryStore;
use super::types::{KindWriter, MemoryError, Refusal, SecretScan};
use crate::storage::write_transaction;

/// V56 (ADR-0038 §1): gyrus's tables, in this database's migration chain
/// after V55. The batch is gyrus's own frozen `SCHEMA_V1`.
pub const V56: &str = gyrus::SCHEMA_V1;

/// The node key of a memory record's subject (its random id: a key never
/// holds the words).
#[must_use]
pub fn memory_key(subject_id: &str) -> String {
    format!("memory:{subject_id}")
}

/// The node key of a Chat turn (a pointer: the words stay in the turn).
#[must_use]
pub fn turn_key(turn_id: &str) -> String {
    format!("turn:{turn_id}")
}

/// The node key of a Chat topic.
#[must_use]
pub fn topic_key(topic_id: &str) -> String {
    format!("topic:{topic_id}")
}

fn gyrus_error(error: gyrus::Error) -> MemoryError {
    match error {
        gyrus::Error::Sqlite(e) => MemoryError::from(e),
        gyrus::Error::Refused(why) => MemoryError::Refused(Refusal::World(why)),
        other => MemoryError::Refused(Refusal::World(match other {
            gyrus::Error::Pack(_) => gyrus::Refusal::UndeclaredKind,
            _ => gyrus::Refusal::Bounds,
        })),
    }
}

/// A world error as SQLite's, for the backup's scrub (which speaks
/// `rusqlite`).
pub(super) fn sqlite(error: MemoryError) -> rusqlite::Error {
    match error {
        MemoryError::Core(crate::error::CoreError::Storage(e)) => e,
        other => rusqlite::Error::ToSqlConversionFailure(Box::new(other)),
    }
}

/// gyrus's tables are in this database (V56).
///
/// # Errors
///
/// Storage failures.
pub fn installed(conn: &Connection) -> Result<bool, MemoryError> {
    gyrus::schema::installed(conn).map_err(gyrus_error)
}

fn log_world_refusal(conn: &Connection, why: Refusal, now_ms: i64) -> Result<(), MemoryError> {
    let mut event = NewEvent::new(now_ms, Stream::Memory, "refused", Actor::App);
    event.payload = serde_json::json!({ "kind": "world", "reason": why.code() });
    append_in(conn, &event, None).map(|_| ())
}

/// The gate's checks that need no database.
fn gate(
    writer: &KindWriter,
    secrets: &dyn SecretScan,
    pack: &gyrus::Pack,
    batch: &gyrus::Batch,
) -> Result<(), Refusal> {
    if !writer.covers_world() {
        return Err(Refusal::WrongWriter);
    }
    gyrus::store::check(pack, batch).map_err(|e| match e {
        gyrus::Error::Refused(why) => Refusal::World(why),
        _ => Refusal::World(gyrus::Refusal::Bounds),
    })?;
    if batch.words().iter().any(|word| secrets.finds_secret(word)) {
        return Err(Refusal::Secret);
    }
    Ok(())
}

/// Stages `batch` in the caller's transaction after the gate, with its
/// `memory.linked` event (counts only). The caller commits, then publishes.
pub(super) fn stage_in(
    tx: &Connection,
    graph: &gyrus::Graph,
    batch: &gyrus::Batch,
    now_ms: i64,
) -> Result<gyrus::Pending, MemoryError> {
    let pending = gyrus::store::stage(tx, graph.pack(), batch, now_ms).map_err(gyrus_error)?;
    let mut event = NewEvent::new(now_ms, Stream::Memory, "linked", Actor::App);
    event.payload = serde_json::json!({
        "nodes": pending.nodes.len(),
        "links": pending.links.len(),
        "unlinked": pending.unlinked.len(),
        "removed": pending.removed.len(),
    });
    append_in(tx, &event, None)?;
    Ok(pending)
}

/// Forgets the world nodes a forget of these memory subjects reaches, in
/// the caller's transaction: each subject's `memory` node and the nodes of
/// every redacted turn. `None` when gyrus's tables aren't here.
pub(super) fn forget_in(
    tx: &Connection,
    subjects: &[String],
) -> Result<Option<(gyrus::Pending, gyrus::Erased)>, MemoryError> {
    if !installed(tx)? {
        return Ok(None);
    }
    let mut keys: Vec<String> = subjects.iter().map(|s| memory_key(s)).collect();
    // What was derived from a forgotten memory (a fact its words became)
    // carries its words: it goes too.
    let mut derived = tx.prepare_cached(
        "SELECT s.key FROM gyrus_links l
         JOIN gyrus_nodes s ON s.id = l.src JOIN gyrus_nodes d ON d.id = l.dst
         WHERE l.kind = 'derived_from' AND d.key = ?1",
    )?;
    for subject in subjects {
        for key in derived.query_map([memory_key(subject)], |row| row.get::<_, String>(0))? {
            keys.push(key?);
        }
    }
    let mut stmt = tx.prepare_cached("SELECT id FROM chat_turns WHERE redacted = 1")?;
    let turns = stmt.query_map([], |row| row.get::<_, String>(0))?;
    for turn in turns {
        keys.push(turn_key(&turn?));
    }
    forget_keys(tx, &keys).map(Some)
}

/// gyrus's forget of these keys, in the caller's transaction.
fn forget_keys(
    tx: &Connection,
    keys: &[String],
) -> Result<(gyrus::Pending, gyrus::Erased), MemoryError> {
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    gyrus::store::forget_in(tx, &refs).map_err(gyrus_error)
}

/// Every node whose key starts with one of `prefixes`, forgotten in the
/// caller's transaction (Clear chat history takes every turn and topic
/// node with the turns). `None` when gyrus's tables aren't here.
pub(super) fn forget_prefixed_in(
    tx: &Connection,
    prefixes: &[&str],
) -> Result<Option<(gyrus::Pending, gyrus::Erased)>, MemoryError> {
    if !installed(tx)? {
        return Ok(None);
    }
    let mut keys: Vec<String> = Vec::new();
    for prefix in prefixes {
        let mut stmt =
            tx.prepare_cached("SELECT key FROM gyrus_nodes WHERE substr(key, 1, length(?1)) = ?1")?;
        for key in stmt.query_map([prefix], |row| row.get::<_, String>(0))? {
            keys.push(key?);
        }
    }
    forget_keys(tx, &keys).map(Some)
}

/// Empties every gyrus table (a copy that holds no memory). No-op without
/// them.
pub(super) fn clear_in(tx: &Connection) -> Result<usize, MemoryError> {
    if !installed(tx)? {
        return Ok(0);
    }
    gyrus::store::clear_in(tx).map_err(gyrus_error)
}

impl MemoryStore<'_> {
    /// Writes nodes and links of Bot's world model through the write gate
    /// (see the module): one `IMMEDIATE` transaction with `secure_delete`
    /// on, its `memory.linked` event in it; then the change set is
    /// published to `graph`'s index.
    ///
    /// # Errors
    ///
    /// [`MemoryError::Refused`] naming the reason (logged, never the
    /// words); storage failures. Nothing is written then.
    pub fn write_world(
        &self,
        writer: &KindWriter,
        secrets: &dyn SecretScan,
        graph: &gyrus::Graph,
        batch: &gyrus::Batch,
        now_ms: i64,
    ) -> Result<gyrus::Pending, MemoryError> {
        if let Err(why) = gate(writer, secrets, graph.pack(), batch) {
            let tx = write_transaction(self.conn)?;
            log_world_refusal(&tx, why, now_ms)?;
            tx.commit()?;
            return Err(MemoryError::Refused(why));
        }
        let pending = {
            let _hold = SecureHold::on(self.conn)?;
            let tx = write_transaction(self.conn)?;
            match stage_in(&tx, graph, batch, now_ms) {
                Ok(pending) => {
                    tx.commit()?;
                    pending
                }
                Err(MemoryError::Refused(why)) => {
                    drop(tx);
                    let tx = write_transaction(self.conn)?;
                    log_world_refusal(&tx, why, now_ms)?;
                    tx.commit()?;
                    return Err(MemoryError::Refused(why));
                }
                Err(other) => return Err(other),
            }
        };
        graph.publish(&pending);
        Ok(pending)
    }

    /// The queued uses of `graph`'s nodes (ADR-0038's `touch`), applied in
    /// this writer hold and published. Counts only: no gate needed beyond
    /// the hold.
    ///
    /// # Errors
    ///
    /// Storage failures (the uses are dropped).
    pub fn flush_world_uses(
        &self,
        graph: &gyrus::Graph,
        now_ms: i64,
    ) -> Result<usize, MemoryError> {
        let uses = graph.take_uses();
        if uses.is_empty() {
            return Ok(0);
        }
        let pending = {
            let _hold = SecureHold::on(self.conn)?;
            let tx = write_transaction(self.conn)?;
            let pending = gyrus::store::touch_in(&tx, &uses, now_ms).map_err(gyrus_error)?;
            tx.commit()?;
            pending
        };
        graph.publish(&pending);
        Ok(uses.len())
    }

    /// Forgets, at most `limit` at a time, the `memory` nodes whose record
    /// is gone (removed another way: Clear chat history, the history
    /// bound, a backup's scrub), with forget's own care: `secure_delete`
    /// on, their keyword buckets rebuilt, the WAL folded; then out of
    /// `graph`'s index. Returns how many went.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn forget_dangling_world(
        &self,
        graph: &gyrus::Graph,
        limit: usize,
    ) -> Result<usize, MemoryError> {
        if !installed(self.conn)? {
            return Ok(0);
        }
        let keys: Vec<String> = self
            .conn
            .prepare_cached(
                "SELECT n.key FROM gyrus_nodes n
                 WHERE n.kind = 'memory' AND NOT EXISTS (
                     SELECT 1 FROM memory_records r
                     WHERE 'memory:' || r.subject_id = n.key)
                 LIMIT ?1",
            )?
            .query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        if keys.is_empty() {
            return Ok(0);
        }
        let pending = {
            let _hold = SecureHold::on(self.conn)?;
            let tx = write_transaction(self.conn)?;
            let (pending, _) = forget_keys(&tx, &keys)?;
            tx.commit()?;
            pending
        };
        super::secure::fold_wal(self.conn)?;
        graph.publish(&pending);
        Ok(keys.len())
    }

    /// The current version of the subject with this random id (a
    /// `memory` node names its record by it).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn current_by_subject(
        &self,
        subject_id: &str,
    ) -> Result<Option<super::store::Record>, MemoryError> {
        use rusqlite::OptionalExtension;
        Ok(self
            .conn
            .query_row(
                &format!(
                    "SELECT {} FROM memory_records
                     WHERE subject_id = ?1 AND superseded_by IS NULL",
                    super::store::RECORD_COLUMNS
                ),
                [subject_id],
                super::store::record_row,
            )
            .optional()?)
    }

    /// The current, trusted versions after the cursor `after` (their
    /// `seq`), oldest first, at most `limit`, with their cursor: the
    /// world model's one-time linking pass reads every record by pages.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn current_page(
        &self,
        after: i64,
        limit: u32,
    ) -> Result<Vec<(i64, super::store::Record)>, MemoryError> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {}, seq FROM memory_records
             WHERE superseded_by IS NULL AND untrusted = 0 AND seq > ?1
             ORDER BY seq LIMIT ?2",
            super::store::RECORD_COLUMNS
        ))?;
        let rows = stmt
            .query_map(rusqlite::params![after, limit], |row| {
                // The record's columns come first, `seq` after them.
                Ok((row.get::<_, i64>(24)?, super::store::record_row(row)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// The gate's checks for a batch, without writing (a record's own
    /// write runs them before its transaction).
    pub(super) fn world_gate(
        writer: &KindWriter,
        secrets: &dyn SecretScan,
        graph: &gyrus::Graph,
        batch: &gyrus::Batch,
    ) -> Result<(), Refusal> {
        gate(writer, secrets, graph.pack(), batch)
    }
}

/// Reads of Bot's world model on this database: the index's build and its
/// generation check, counts and labels. Writes go only through
/// [`MemoryStore::write_world`] (the gate).
#[derive(Debug)]
pub struct WorldReader<'c> {
    conn: &'c Connection,
}

impl<'c> WorldReader<'c> {
    /// The reader on `conn`.
    #[must_use]
    pub const fn new(conn: &'c Connection) -> Self {
        Self { conn }
    }

    /// gyrus's tables are here.
    #[must_use]
    pub fn installed(&self) -> bool {
        installed(self.conn).unwrap_or(false)
    }

    /// Builds `graph`'s index from SQLite (one read transaction).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn build(&self, graph: &gyrus::Graph) -> Result<gyrus::Built, MemoryError> {
        graph.build(self.conn).map_err(gyrus_error)
    }

    /// Rebuilds `graph`'s index when it is missing or behind SQLite;
    /// `true` when it rebuilt.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn ensure(&self, graph: &gyrus::Graph) -> Result<bool, MemoryError> {
        graph.ensure(self.conn).map_err(gyrus_error)
    }

    /// `graph`'s index is current with SQLite.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn check(&self, graph: &gyrus::Graph) -> Result<bool, MemoryError> {
        graph.check(self.conn).map_err(gyrus_error)
    }

    /// How many nodes are stored.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn nodes(&self) -> Result<i64, MemoryError> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM gyrus_nodes", [], |row| row.get(0))?)
    }

    /// The starting nodes for a question ([`gyrus::ops::seed`]: the host's
    /// matches and exact aliases, then keyword search).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn seed(
        &self,
        snap: &gyrus::Snapshot,
        pack: &gyrus::Pack,
        query: &gyrus::ops::SeedQuery<'_>,
    ) -> Result<Vec<gyrus::ops::Seed>, MemoryError> {
        gyrus::ops::seed(self.conn, snap, pack, query).map_err(gyrus_error)
    }

    /// One bounded batch of the idle pass ([`gyrus::dream::plan`]); the
    /// caller writes it through [`MemoryStore::write_world`].
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn dream(
        &self,
        snap: &gyrus::Snapshot,
        pack: &gyrus::Pack,
        opts: &gyrus::dream::DreamOpts,
    ) -> Result<gyrus::dream::Dreamed, MemoryError> {
        gyrus::dream::plan(self.conn, snap, pack, opts).map_err(gyrus_error)
    }

    /// The labels of these nodes (the few a context shows).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn labels(
        &self,
        ids: &[gyrus::NodeId],
    ) -> Result<std::collections::HashMap<gyrus::NodeId, String>, MemoryError> {
        gyrus::store::labels(self.conn, ids).map_err(gyrus_error)
    }
}
