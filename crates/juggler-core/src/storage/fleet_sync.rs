//! Fleet sync storage (AJ-E708, schema v22): the node's durable outbox
//! of syncable record changes and the console's durable replica of what
//! each paired machine reported.
//!
//! Ownership is single-writer by construction (ADR-0013): every record
//! belongs to the machine that created it (`origin`), and only that
//! machine changes it. So the replica never merges two writers — the
//! newest per-origin **sequence** for a record wins, and the wall clock
//! is display-only (a skewed clock can never reorder anything).
//! Applying the same delta twice, or deltas out of order, converges to
//! the same state.

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::CoreError;

/// One outbox (or replica) entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncEntry {
    /// The origin's monotonic sequence (never reused).
    pub seq: u64,
    /// Record class (`instance`).
    pub kind: String,
    /// Record id within its class.
    pub record_id: String,
    /// The record's encoded state (the link's wire bytes).
    pub payload: Vec<u8>,
    /// The origin's wall clock when it changed, epoch ms (display only).
    pub wall_ms: i64,
}

/// Typed access to the sync tables.
#[derive(Debug)]
pub struct FleetSyncRepo<'a> {
    conn: &'a Connection,
}

fn to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

impl FleetSyncRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> FleetSyncRepo<'_> {
        FleetSyncRepo { conn }
    }

    /// Appends a record's new state to this node's outbox and drops its
    /// older entries (a reconnecting console needs each record's latest
    /// state, not its history). Returns the new sequence.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn outbox_append(
        &self,
        kind: &str,
        record_id: &str,
        payload: &[u8],
        wall_ms: i64,
    ) -> Result<u64, CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO fleet_outbox (kind, record_id, payload, wall_ms) VALUES (?1, ?2, ?3, ?4)",
            params![kind, record_id, payload, wall_ms],
        )?;
        let seq = tx.last_insert_rowid();
        tx.execute(
            "DELETE FROM fleet_outbox WHERE kind = ?1 AND record_id = ?2 AND seq < ?3",
            params![kind, record_id, seq],
        )?;
        tx.commit()?;
        Ok(to_u64(seq))
    }

    /// The latest outbox entry for a record, if any.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn outbox_latest(
        &self,
        kind: &str,
        record_id: &str,
    ) -> Result<Option<SyncEntry>, CoreError> {
        self.conn
            .query_row(
                "SELECT seq, kind, record_id, payload, wall_ms FROM fleet_outbox
                 WHERE kind = ?1 AND record_id = ?2 ORDER BY seq DESC LIMIT 1",
                params![kind, record_id],
                entry_from_row,
            )
            .optional()
            .map_err(CoreError::Storage)
    }

    /// Outbox entries after `since` (oldest first, at most `limit`).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn outbox_since(&self, since: u64, limit: usize) -> Result<Vec<SyncEntry>, CoreError> {
        let mut statement = self.conn.prepare(
            "SELECT seq, kind, record_id, payload, wall_ms FROM fleet_outbox
             WHERE seq > ?1 ORDER BY seq LIMIT ?2",
        )?;
        let rows = statement.query_map(
            params![to_i64(since), i64::try_from(limit).unwrap_or(i64::MAX)],
            entry_from_row,
        )?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// The replica's high-water mark for `origin` under `epoch` (0 when
    /// unknown or the epoch changed — a fresh full sync is needed).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn replica_high_water(&self, origin: &str, epoch: &str) -> Result<u64, CoreError> {
        let stored: Option<(String, i64)> = self
            .conn
            .query_row(
                "SELECT epoch, seq FROM fleet_replica_hw WHERE origin = ?1",
                params![origin],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        Ok(match stored {
            Some((stored_epoch, seq)) if stored_epoch == epoch => to_u64(seq),
            _ => 0,
        })
    }

    /// Starts `origin` afresh under `epoch` when it changed (a reset or
    /// reinstalled machine): its replica rows go, so stale sequences can
    /// never shadow new ones.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn replica_set_epoch(&self, origin: &str, epoch: &str) -> Result<(), CoreError> {
        let current: Option<String> = self
            .conn
            .query_row(
                "SELECT epoch FROM fleet_replica_hw WHERE origin = ?1",
                params![origin],
                |row| row.get(0),
            )
            .optional()?;
        if current.as_deref() == Some(epoch) {
            return Ok(());
        }
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM fleet_replica WHERE origin = ?1",
            params![origin],
        )?;
        tx.execute(
            "INSERT INTO fleet_replica_hw (origin, epoch, seq) VALUES (?1, ?2, 0)
             ON CONFLICT(origin) DO UPDATE SET epoch = excluded.epoch, seq = 0",
            params![origin, epoch],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Applies one delta from `origin`: kept only if its sequence is
    /// newer than what the replica holds for that record (idempotent,
    /// order-independent). Returns whether it changed anything.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn replica_apply(
        &self,
        origin: &str,
        entry: &SyncEntry,
        received_ms: i64,
    ) -> Result<bool, CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        let changed = tx.execute(
            "INSERT INTO fleet_replica (origin, kind, record_id, seq, payload, wall_ms, received_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(origin, kind, record_id) DO UPDATE SET
                 seq = excluded.seq, payload = excluded.payload,
                 wall_ms = excluded.wall_ms, received_ms = excluded.received_ms
             WHERE excluded.seq > fleet_replica.seq",
            params![
                origin,
                entry.kind,
                entry.record_id,
                to_i64(entry.seq),
                entry.payload,
                entry.wall_ms,
                received_ms
            ],
        )?;
        tx.execute(
            "UPDATE fleet_replica_hw SET seq = MAX(seq, ?2) WHERE origin = ?1",
            params![origin, to_i64(entry.seq)],
        )?;
        tx.commit()?;
        Ok(changed > 0)
    }

    /// Every replicated record of `origin` and `kind`, with when each
    /// arrived (newest sequence first).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn replica_list(
        &self,
        origin: &str,
        kind: &str,
    ) -> Result<Vec<(SyncEntry, i64)>, CoreError> {
        let mut statement = self.conn.prepare(
            "SELECT seq, kind, record_id, payload, wall_ms, received_ms FROM fleet_replica
             WHERE origin = ?1 AND kind = ?2 ORDER BY seq DESC",
        )?;
        let rows = statement.query_map(params![origin, kind], |row| {
            Ok((entry_from_row(row)?, row.get::<_, i64>(5)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// Forgets everything replicated from `origin` (unpaired).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn replica_forget(&self, origin: &str) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM fleet_replica WHERE origin = ?1",
            params![origin],
        )?;
        tx.execute(
            "DELETE FROM fleet_replica_hw WHERE origin = ?1",
            params![origin],
        )?;
        tx.commit()?;
        Ok(())
    }
}

fn entry_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncEntry> {
    Ok(SyncEntry {
        seq: to_u64(row.get(0)?),
        kind: row.get(1)?,
        record_id: row.get(2)?,
        payload: row.get(3)?,
        wall_ms: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    fn entry(seq: u64, id: &str, state: &str) -> SyncEntry {
        SyncEntry {
            seq,
            kind: "instance".into(),
            record_id: id.into(),
            payload: state.as_bytes().to_vec(),
            wall_ms: 0,
        }
    }

    #[test]
    fn the_outbox_keeps_each_records_latest_state_with_monotonic_sequences() {
        let storage = Storage::open_in_memory().unwrap();
        let sync = storage.fleet_sync();
        let a1 = sync.outbox_append("instance", "a", b"starting", 1).unwrap();
        let b1 = sync.outbox_append("instance", "b", b"running", 2).unwrap();
        let a2 = sync.outbox_append("instance", "a", b"running", 3).unwrap();
        assert!(a1 < b1 && b1 < a2);
        let all = sync.outbox_since(0, 100).unwrap();
        assert_eq!(all.len(), 2, "a's older state is compacted away");
        assert_eq!(all[1].payload, b"running".to_vec());
        assert_eq!(
            sync.outbox_since(b1, 100).unwrap().len(),
            1,
            "only what's newer"
        );
        // Deleting rows never lets a sequence be reused.
        let next = sync.outbox_append("instance", "c", b"x", 4).unwrap();
        assert!(next > a2);
    }

    #[test]
    fn shuffled_and_duplicated_delivery_converges_and_never_goes_backwards() {
        let deltas: Vec<SyncEntry> = (1..=60)
            .map(|seq| entry(seq, &format!("i{}", seq % 7), &format!("state-{seq}")))
            .collect();
        // Expected: per record, the highest sequence.
        let mut expected = std::collections::BTreeMap::new();
        for delta in &deltas {
            expected.insert(delta.record_id.clone(), delta.payload.clone());
        }
        // A deterministic shuffle with duplicates.
        let mut order: Vec<usize> = (0..deltas.len()).chain(0..deltas.len()).collect();
        let mut state = 0x2545_F491_4F6C_DD1D_u64;
        for i in (1..order.len()).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let j = usize::try_from(state % u64::try_from(i + 1).unwrap()).unwrap();
            order.swap(i, j);
        }
        let storage = Storage::open_in_memory().unwrap();
        let sync = storage.fleet_sync();
        sync.replica_set_epoch("mac-a", "e1").unwrap();
        for index in order {
            sync.replica_apply("mac-a", &deltas[index], 0).unwrap();
        }
        let got: std::collections::BTreeMap<_, _> = sync
            .replica_list("mac-a", "instance")
            .unwrap()
            .into_iter()
            .map(|(entry, _)| (entry.record_id, entry.payload))
            .collect();
        assert_eq!(got, expected);
        assert_eq!(sync.replica_high_water("mac-a", "e1").unwrap(), 60);
        // Re-applying an old delta changes nothing.
        assert!(!sync.replica_apply("mac-a", &deltas[0], 0).unwrap());
    }

    #[test]
    fn a_new_epoch_starts_the_origin_afresh() {
        let storage = Storage::open_in_memory().unwrap();
        let sync = storage.fleet_sync();
        sync.replica_set_epoch("mac-a", "e1").unwrap();
        sync.replica_apply("mac-a", &entry(500, "x", "old"), 0)
            .unwrap();
        assert_eq!(sync.replica_high_water("mac-a", "e1").unwrap(), 500);
        assert_eq!(
            sync.replica_high_water("mac-a", "e2").unwrap(),
            0,
            "another epoch knows nothing"
        );
        // The machine was reset: its sequences start again at 1.
        sync.replica_set_epoch("mac-a", "e2").unwrap();
        assert!(sync.replica_list("mac-a", "instance").unwrap().is_empty());
        assert!(
            sync.replica_apply("mac-a", &entry(1, "x", "new"), 0)
                .unwrap()
        );
        assert_eq!(
            sync.replica_list("mac-a", "instance").unwrap()[0].0.payload,
            b"new".to_vec()
        );
        sync.replica_forget("mac-a").unwrap();
        assert!(sync.replica_list("mac-a", "instance").unwrap().is_empty());
    }
}
