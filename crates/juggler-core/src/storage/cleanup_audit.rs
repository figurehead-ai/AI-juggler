//! Cleanup audit log (AJ-E509, pass2 §11.1): the append-only
//! accountability record for EVERY kill — manual or automatic. The
//! intent row is written BEFORE the first signal and completed with
//! outcomes after the ladder; a crash mid-ladder leaves the truthful
//! pending record ("TERM'd, outcome unknown" at read time).
//!
//! APPEND-ONLY BY CONSTRUCTION: the repo exposes insert + complete +
//! read + prune — no update-delete surface exists, and no UI in this
//! epic may add one (Req.5). Command lines persist HASHED + truncated
//! only: args may carry tokens; hash+truncate is the privacy boundary
//! and full cmdlines never outlive the confirm sheet (Req.2).

use rusqlite::{Connection, params};

use crate::error::CoreError;

/// The actor vocabulary: `user:{command}` for manual kills,
/// `auto:tier-1` / `auto:tier-2` for the opted-in tiers.
pub const ACTOR_PREFIX_USER: &str = "user:";
/// The Tier-1 automatic actor (AJ-E508).
pub const ACTOR_AUTO_TIER1: &str = "auto:tier-1";
/// The Tier-2 automatic actor (AJ-E508): crash-exit tidies.
pub const ACTOR_AUTO_TIER2: &str = "auto:tier-2";

/// The outcome while the ladder is still running (or after a crash
/// mid-ladder — read as "TERM'd, outcome unknown").
pub const OUTCOME_PENDING: &str = "pending";

/// One audit record (insert shape; `id` assigned by the store).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CleanupAuditRecord {
    /// Row id (0 until inserted).
    pub id: i64,
    /// When the INTENT was written, epoch ms (before the first
    /// signal — the whole point).
    pub at_ms: i64,
    /// `user:{command}` | `auto:tier-1` | `auto:tier-2`.
    pub actor: String,
    /// The candidate process.
    pub pid: u32,
    /// Its process group, when scoped.
    pub pgid: Option<u32>,
    /// Executable path.
    pub exe_path: String,
    /// Hashed full command line (the matching key; never the args).
    pub cmdline_hash: String,
    /// Truncated command line for display (the privacy boundary).
    pub cmdline_trunc: String,
    /// Owning Instance.
    pub session_id: String,
    /// Provider id.
    pub provider: String,
    /// Account id.
    pub account_id: Option<String>,
    /// Project id.
    pub project_id: Option<String>,
    /// Machine id (D-13).
    pub machine_id: String,
    /// Identity-check results at verify time (serialized).
    pub identity: String,
    /// `pid` | `pgid` | `tree`.
    pub scope: String,
    /// `pending` until completed; then the per-PID outcomes JSON.
    pub outcome: String,
    /// Ports the candidate held before the ladder.
    pub ports_before: Option<String>,
    /// Ports still held after (the honest success check).
    pub ports_after: Option<String>,
}

/// A history row (the read shape — pending rows read as the truthful
/// unknown outcome).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AuditOutcomeInfo {
    /// Row id.
    pub id: i64,
    /// Intent time, epoch ms.
    pub at_ms: i64,
    /// The actor (UI distinguishes user/auto by icon+text).
    pub actor: String,
    /// True when the actor is an automatic tier.
    pub automatic: bool,
    /// The candidate pid.
    pub pid: u32,
    /// Display command line (truncated).
    pub cmdline_trunc: String,
    /// Owning Instance.
    pub session_id: String,
    /// Provider id.
    pub provider: String,
    /// Account id.
    pub account_id: Option<String>,
    /// Machine id.
    pub machine_id: String,
    /// The outcome in the AJ-E507 vocabulary, with `outcome unknown`
    /// for crash-pending records.
    pub outcome: String,
    /// Ports before the ladder.
    pub ports_before: Option<String>,
    /// Ports after (None while pending).
    pub ports_after: Option<String>,
}

impl CleanupAuditRecord {
    /// The history read shape: pending records surface as the
    /// truthful `TERM'd, outcome unknown` (a crash mid-ladder is a
    /// fact to report, never to hide).
    #[must_use]
    pub fn to_history(&self) -> AuditOutcomeInfo {
        AuditOutcomeInfo {
            id: self.id,
            at_ms: self.at_ms,
            actor: self.actor.clone(),
            automatic: self.actor.starts_with("auto:"),
            pid: self.pid,
            cmdline_trunc: self.cmdline_trunc.clone(),
            session_id: self.session_id.clone(),
            provider: self.provider.clone(),
            account_id: self.account_id.clone(),
            machine_id: self.machine_id.clone(),
            outcome: if self.outcome == OUTCOME_PENDING {
                "TERM'd, outcome unknown".to_string()
            } else {
                self.outcome.clone()
            },
            ports_before: self.ports_before.clone(),
            ports_after: self.ports_after.clone(),
        }
    }
}

/// Append-only access to `cleanup_audit` over its OWN connection
/// (WAL: the intent write must not queue behind the storage mutex —
/// ≤5 ms p95 in front of the ladder).
#[derive(Debug)]
pub struct CleanupAuditRepo {
    conn: std::sync::Mutex<Connection>,
}

impl CleanupAuditRepo {
    /// Opens (or creates) the audit log at `path`; a fresh file gets
    /// the v14 table applied directly (the standalone creator for
    /// tests; the app's file gets it via migration).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn open(path: &std::path::Path) -> Result<Self, CoreError> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(conn)
    }

    /// In-memory audit log (tests).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn open_in_memory() -> Result<Self, CoreError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, CoreError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cleanup_audit (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                at_ms INTEGER NOT NULL,
                actor TEXT NOT NULL,
                pid INTEGER NOT NULL,
                pgid INTEGER,
                exe_path TEXT NOT NULL,
                cmdline_hash TEXT NOT NULL,
                cmdline_trunc TEXT NOT NULL,
                session_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                account_id TEXT,
                project_id TEXT,
                machine_id TEXT NOT NULL,
                identity TEXT NOT NULL,
                scope TEXT NOT NULL,
                outcome TEXT NOT NULL,
                ports_before TEXT,
                ports_after TEXT
            );
            CREATE INDEX IF NOT EXISTS cleanup_audit_at ON cleanup_audit(at_ms);
            CREATE INDEX IF NOT EXISTS cleanup_audit_account
                ON cleanup_audit(account_id, at_ms);",
        )?;
        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, CoreError> {
        self.conn
            .lock()
            .map_err(|_| CoreError::Storage(rusqlite::Error::ExecuteReturnedResults))
    }

    /// Writes the INTENT row (outcome `pending`) BEFORE any signal —
    /// returns the row id the ladder completes later.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures — callers
    /// REFUSE the kill (no audit, no kill).
    pub fn intent(&self, record: &CleanupAuditRecord) -> Result<i64, CoreError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO cleanup_audit
                 (at_ms, actor, pid, pgid, exe_path, cmdline_hash, cmdline_trunc,
                  session_id, provider, account_id, project_id, machine_id,
                  identity, scope, outcome, ports_before, ports_after)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                     ?14, ?15, ?16, NULL)",
            params![
                record.at_ms,
                record.actor,
                record.pid,
                record.pgid,
                record.exe_path,
                record.cmdline_hash,
                record.cmdline_trunc,
                record.session_id,
                record.provider,
                record.account_id,
                record.project_id,
                record.machine_id,
                record.identity,
                record.scope,
                OUTCOME_PENDING,
                record.ports_before,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Completes the record with the ladder's per-PID outcomes JSON
    /// and the honest post-ladder port state. The ONLY mutation after
    /// insert — there is no update/delete surface (append-only law).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn complete(
        &self,
        id: i64,
        outcome: &str,
        ports_after: Option<&str>,
        now_ms: i64,
    ) -> Result<(), CoreError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE cleanup_audit SET outcome = ?2, ports_after = ?3
             WHERE id = ?1 AND outcome = ?4",
            params![id, outcome, ports_after, OUTCOME_PENDING],
        )?;
        let _ = now_ms;
        Ok(())
    }

    /// History read: records within the retention window (7 days),
    /// newest first, mapped to the outcome vocabulary (pending reads
    /// as `TERM'd, outcome unknown`).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn history(&self, since_ms: i64) -> Result<Vec<AuditOutcomeInfo>, CoreError> {
        let conn = self.lock()?;
        let mut out = Vec::new();
        let mut stmt =
            conn.prepare("SELECT * FROM cleanup_audit WHERE at_ms >= ?1 ORDER BY at_ms DESC")?;
        let rows = stmt.query_map(params![since_ms], map_row)?;
        for row in rows {
            out.push(row?.to_history());
        }
        Ok(out)
    }

    /// Full-fidelity rows within the window (the EXPORT source —
    /// hashed cmdlines only, per the privacy boundary).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn rows_since(&self, since_ms: i64) -> Result<Vec<CleanupAuditRecord>, CoreError> {
        let conn = self.lock()?;
        let mut out = Vec::new();
        let mut stmt =
            conn.prepare("SELECT * FROM cleanup_audit WHERE at_ms >= ?1 ORDER BY at_ms ASC")?;
        let rows = stmt.query_map(params![since_ms], map_row)?;
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 7-day retention prune (mirrors the ledger's; the audit has no
    /// running rows — every record is a completed or crashed kill).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn prune(&self, before_ms: i64) -> Result<usize, CoreError> {
        let conn = self.lock()?;
        let removed = conn.execute(
            "DELETE FROM cleanup_audit WHERE at_ms < ?1",
            params![before_ms],
        )?;
        Ok(removed)
    }
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CleanupAuditRecord> {
    Ok(CleanupAuditRecord {
        id: row.get("id")?,
        at_ms: row.get("at_ms")?,
        actor: row.get("actor")?,
        pid: u32::try_from(row.get::<_, i64>("pid")?).unwrap_or_default(),
        pgid: row
            .get::<_, Option<i64>>("pgid")?
            .and_then(|v| u32::try_from(v).ok()),
        exe_path: row.get("exe_path")?,
        cmdline_hash: row.get("cmdline_hash")?,
        cmdline_trunc: row.get("cmdline_trunc")?,
        session_id: row.get("session_id")?,
        provider: row.get("provider")?,
        account_id: row.get("account_id")?,
        project_id: row.get("project_id")?,
        machine_id: row.get("machine_id")?,
        identity: row.get("identity")?,
        scope: row.get("scope")?,
        outcome: row.get("outcome")?,
        ports_before: row.get("ports_before")?,
        ports_after: row.get("ports_after")?,
    })
}
