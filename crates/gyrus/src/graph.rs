//! The engine: a pack, the current snapshot of the index, and the queue
//! of uses. SQLite first, then the index: a write commits, and only then
//! is its change set applied ([`Graph::publish`]). If the process dies
//! between the two, the next start rebuilds the index from SQLite, so the
//! two never disagree after a restart (`tests/kill.rs` kills it at every
//! step and compares).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use rusqlite::Connection;

use crate::error::{Error, Result};
use crate::index::Snapshot;
use crate::pack::Pack;
use crate::store::{self, Erased, Pending};
use crate::types::{Batch, NodeId};

/// Where a write is, for a test that stops the process at each step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The batch is written in the open transaction.
    Staged,
    /// The transaction committed.
    Committed,
    /// The change set is in the index.
    Published,
}

/// A typed graph over one SQLite database.
#[derive(Debug)]
pub struct Graph {
    pack: Arc<Pack>,
    current: RwLock<Option<Arc<Snapshot>>>,
    stale: AtomicBool,
    uses: Mutex<HashMap<NodeId, u32>>,
}

/// What a build read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Built {
    /// Nodes.
    pub nodes: usize,
    /// Links.
    pub links: usize,
    /// The generation it includes.
    pub generation: i64,
}

impl Graph {
    /// A graph for `pack`, not built yet (reads answer
    /// [`Error::WarmingUp`] until [`Self::build`]).
    #[must_use]
    pub fn new(pack: Pack) -> Self {
        Self {
            pack: Arc::new(pack),
            current: RwLock::new(None),
            stale: AtomicBool::new(false),
            uses: Mutex::new(HashMap::new()),
        }
    }

    /// The pack.
    #[must_use]
    pub fn pack(&self) -> &Pack {
        &self.pack
    }

    /// (Re)builds the index from SQLite, in one read transaction.
    ///
    /// # Errors
    ///
    /// SQLite errors (the previous index, if any, stays).
    pub fn build(&self, conn: &Connection) -> Result<Built> {
        let (generation, nodes, links) = store::load(conn)?;
        let snap = Snapshot::build(&self.pack, generation, &nodes, &links);
        let built = Built {
            nodes: snap.len(),
            links: snap.link_count(),
            generation,
        };
        if let Ok(mut current) = self.current.write() {
            *current = Some(Arc::new(snap));
        }
        self.stale.store(false, Ordering::SeqCst);
        Ok(built)
    }

    /// The current snapshot.
    ///
    /// # Errors
    ///
    /// [`Error::WarmingUp`] until the first build.
    pub fn snapshot(&self) -> Result<Arc<Snapshot>> {
        self.current
            .read()
            .ok()
            .and_then(|current| current.clone())
            .ok_or(Error::WarmingUp)
    }

    /// Built, and not known to be behind SQLite.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        !self.stale.load(Ordering::SeqCst) && self.snapshot().is_ok()
    }

    /// Applies a committed change set. One that skips a generation (a
    /// write published elsewhere, or lost to a crash) marks the index
    /// stale, for the host to rebuild; one it already holds is ignored.
    pub fn publish(&self, pending: &Pending) {
        let Ok(mut current) = self.current.write() else {
            return;
        };
        let Some(snap) = current.as_mut() else {
            return;
        };
        let have = snap.generation();
        if pending.generation <= have {
            return;
        }
        if pending.generation != have + 1 {
            self.stale.store(true, Ordering::SeqCst);
            return;
        }
        Arc::make_mut(snap).apply(&self.pack, pending);
    }

    /// Compares the index's generation with SQLite's: `true` when the
    /// index is current; otherwise it is marked stale.
    ///
    /// # Errors
    ///
    /// SQLite errors.
    pub fn check(&self, conn: &Connection) -> Result<bool> {
        let stored = store::generation(conn)?;
        let fresh = self
            .snapshot()
            .is_ok_and(|snap| snap.generation() == stored)
            && !self.stale.load(Ordering::SeqCst);
        if !fresh {
            self.stale.store(true, Ordering::SeqCst);
        }
        Ok(fresh)
    }

    /// Rebuilds when the index is missing or behind SQLite; `true` when it
    /// rebuilt.
    ///
    /// # Errors
    ///
    /// SQLite errors.
    pub fn ensure(&self, conn: &Connection) -> Result<bool> {
        if self.check(conn)? {
            return Ok(false);
        }
        self.build(conn)?;
        Ok(true)
    }

    /// Queues uses of these nodes (a context that included them): applied
    /// in the next writer hold by [`Self::flush_uses`], so a read never
    /// writes.
    pub fn touch(&self, nodes: &[NodeId]) {
        if let Ok(mut uses) = self.uses.lock() {
            for id in nodes {
                *uses.entry(*id).or_default() += 1;
            }
        }
    }

    /// Takes the queued uses.
    #[must_use]
    pub fn take_uses(&self) -> Vec<(NodeId, u32)> {
        let mut out: Vec<(NodeId, u32)> = self
            .uses
            .lock()
            .map(|mut uses| uses.drain().collect())
            .unwrap_or_default();
        out.sort_unstable();
        out
    }

    /// Applies the queued uses in one transaction, then publishes them.
    ///
    /// # Errors
    ///
    /// SQLite errors (the uses are dropped: they are counts, not content).
    pub fn flush_uses(&self, conn: &Connection, now: i64) -> Result<usize> {
        let uses = self.take_uses();
        if uses.is_empty() {
            return Ok(0);
        }
        let pending = {
            let _secure = SecureDelete::on(conn)?;
            in_transaction(conn, |tx| store::touch_in(tx, &uses, now))?
        };
        self.publish(&pending);
        Ok(uses.len())
    }

    /// Writes `batch` in its own transaction and publishes it (for a
    /// database of gyrus's own; a host with its own write gate calls
    /// [`store::stage`] in its transaction and [`Self::publish`] after).
    ///
    /// # Errors
    ///
    /// As [`store::stage`]; nothing is written then.
    pub fn write(&self, conn: &Connection, batch: &Batch, now: i64) -> Result<Pending> {
        self.write_observed(conn, batch, now, &mut |_| {})
    }

    /// [`Self::write`], telling `at` each step (a test stops the process
    /// there).
    ///
    /// # Errors
    ///
    /// As [`Self::write`].
    pub fn write_observed(
        &self,
        conn: &Connection,
        batch: &Batch,
        now: i64,
        at: &mut dyn FnMut(Step),
    ) -> Result<Pending> {
        let pending = {
            // Every write frees what it replaces zeroed (a keyword segment
            // merged, a page split), so no earlier copy of a label lingers
            // in free space for a later forget to miss.
            let _secure = SecureDelete::on(conn)?;
            in_transaction_observed(conn, |tx| store::stage(tx, &self.pack, batch, now), at)?
        };
        self.publish(&pending);
        at(Step::Published);
        Ok(pending)
    }

    /// Forgets the nodes with these keys, the whole procedure for a
    /// database of gyrus's own: `secure_delete` on, the nodes, links,
    /// aliases, vectors and keyword entries deleted and the buckets that
    /// held them rebuilt, in one transaction; then the WAL folded in and
    /// truncated; then the same nodes removed from the index.
    ///
    /// # Errors
    ///
    /// SQLite errors (nothing is removed unless all of it is).
    pub fn forget(&self, conn: &Connection, keys: &[&str]) -> Result<Erased> {
        let (pending, erased) = {
            let _secure = SecureDelete::on(conn)?;
            in_transaction(conn, |tx| store::forget_in(tx, keys))?
        };
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        self.publish(&pending);
        Ok(erased)
    }
}

/// `PRAGMA secure_delete` on while it lives (SQLite zeroes what each
/// delete frees), then back to what the connection had.
struct SecureDelete<'c> {
    conn: &'c Connection,
    before: i64,
}

impl<'c> SecureDelete<'c> {
    fn on(conn: &'c Connection) -> Result<Self> {
        let before: i64 = conn.query_row("PRAGMA secure_delete", [], |row| row.get(0))?;
        conn.pragma_update(None, "secure_delete", 1)?;
        Ok(Self { conn, before })
    }
}

impl Drop for SecureDelete<'_> {
    fn drop(&mut self) {
        let _ = self.conn.pragma_update(None, "secure_delete", self.before);
    }
}

fn in_transaction<T>(conn: &Connection, body: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
    in_transaction_observed(conn, body, &mut |_| {})
}

/// `BEGIN IMMEDIATE` (a write that reads first must hold the lock from
/// the start), the body, `COMMIT`; a failed body rolls back.
fn in_transaction_observed<T>(
    conn: &Connection,
    body: impl FnOnce(&Connection) -> Result<T>,
    at: &mut dyn FnMut(Step),
) -> Result<T> {
    conn.execute_batch("BEGIN IMMEDIATE")?;
    match body(conn) {
        Ok(value) => {
            at(Step::Staged);
            if let Err(error) = conn.execute_batch("COMMIT") {
                let _ = conn.execute_batch("ROLLBACK");
                return Err(error.into());
            }
            at(Step::Committed);
            Ok(value)
        }
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}
