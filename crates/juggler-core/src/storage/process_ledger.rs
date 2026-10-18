//! Process ledger repository (AJ-E501, pass2 §5): the queryable
//! SQLite table that survives root exit, app exit, and reboot — the
//! basis for orphan detection. The JSONL journal stays a crash log;
//! THIS is the query store (the task's trap: never extend the journal
//! into one).
//!
//! Rows are written `Intent`-first (state `Starting`, no PID) BEFORE
//! the spawn, patched immediately after (PID/PGID/OS start time),
//! helpers upserted batched per reconciler tick, exits patched with
//! their kind. Exited rows retain 7 days; running rows are never
//! pruned.

use rusqlite::{Connection, params};

use crate::error::CoreError;

/// Ledger row state vocabulary (`Starting` → `Running` → `Exited`).
pub const STATE_STARTING: &str = "starting";
/// A live (or presumed-live) row.
pub const STATE_RUNNING: &str = "running";
/// The process ended; the row retains 7 days for audit/cleanup.
pub const STATE_EXITED: &str = "exited";
/// Identity verification failed (AJ-E503 §6.2): PERMANENT — the row
/// leaves every kill path and the candidate set forever, and the
/// prune never touches it (neither running nor exited).
pub const STATE_UNVERIFIABLE: &str = "unverifiable";
/// The Instance's root exited and the reconciler confirmed (grace +
/// two agreeing ticks, AJ-E504) the helper was left behind. Stays a
/// candidate (re-verified every tick); the Cleanup Centre surface.
pub const STATE_ORPHANED: &str = "orphaned";

/// The exec-settle window (AJ-E508): discovery can catch a helper
/// between `fork` and `exec`, when its recorded image is still the
/// wrapper (`/bin/sh` → python → the real tool). Within this many
/// milliseconds of `first_seen` the upsert REFINES the recorded
/// identity (exe/cmdline) toward the live kernel facts, and the
/// supervisor's reconciler withholds its exe-mismatch verdict — the
/// upsert is throttled to 2 s while the reconciler ticks faster, so a
/// verdict racing the refinement would flip the row `unverifiable`
/// before the correction ever lands (CI flake PR #83). After the
/// window the identity is frozen: an exe mismatch is then a PID-reuse
/// signal and flips as before. A start time NEVER legitimately
/// changes — that leg is verified at every age, window or not.
pub const EXEC_SETTLE_MS: i64 = 10_000;

/// Row role: the root harness process.
pub const ROLE_ROOT: &str = "root";
/// Row role: a descendant helper (dev server, watcher, …).
pub const ROLE_HELPER: &str = "helper";
/// Row role: a vendor login terminal (ADR-0003).
pub const ROLE_LOGIN: &str = "login";

/// One `process_ledger` row (pass2 §5 columns, typed).
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerRow {
    /// OS process id (`None` on Intent rows).
    pub pid: Option<u32>,
    /// Process-group id (Unix; `None` until captured — AJ-E502).
    pub pgid: Option<u32>,
    /// OS process start time, epoch seconds (`None` until patched —
    /// NOT wall-clock spawn time; E503's identity key).
    pub started_at: Option<i64>,
    /// Executable path (the real harness binary).
    pub exe_path: String,
    /// Stable hash of the full command line (matching key; the
    /// 64-bit `DefaultHasher` — identity, not cryptography).
    pub cmdline_hash: String,
    /// Truncated command line for display.
    pub cmdline_trunc: String,
    /// Boot identity (OS boot time, epoch seconds) — makes
    /// `(pid, started_at)` unique across reboots.
    pub boot_id: String,
    /// Owning Instance (supervisor session) id.
    pub session_id: String,
    /// Instance row id, when the read model links one.
    pub instance_id: Option<i64>,
    /// Provider id.
    pub provider: String,
    /// Owning Account id (`None` for login roots before an account).
    pub account_id: Option<String>,
    /// Owning project id.
    pub project_id: Option<String>,
    /// Machine identity (D-13: fleet-shaped from day one).
    pub machine_id: String,
    /// Root process pid of the tree (`None` on the root row itself).
    pub root_pid: Option<u32>,
    /// Direct parent pid (`None` on the root row itself).
    pub parent_pid: Option<u32>,
    /// Tree depth (root = 0).
    pub depth: u32,
    /// `root` | `helper` | `login`.
    pub role: String,
    /// `starting` | `running` | `exited`.
    pub state: String,
    /// Listening ports (AJ-E505; `None` until then).
    pub listens_on: Option<String>,
    /// Working directory.
    pub cwd: String,
    /// Peak RSS bytes observed (upserted by the reconciler tick).
    pub peak_rss: Option<i64>,
    /// Peak CPU percent observed.
    pub peak_cpu: Option<f64>,
    /// Exit classification (`code:N` | `signal:N`; `None` while live).
    pub exit_kind: Option<String>,
    /// True when Juggler reaped the exit (waited the root).
    pub reaped_by_us: Option<bool>,
    /// True when the helper left the root's process group (live PGID ≠
    /// root PGID at reconcile time — AJ-E502 Req.4). Sticky once set:
    /// containment is void for this row, forever.
    pub escaped_group: bool,
    /// True when the row was orphaned by CRASH RECOVERY (AJ-E510
    /// Req.2): its root died while Juggler was closed — the distinct
    /// `left behind while Juggler was closed` marker. Sticky.
    pub down_period: bool,
    /// When the Intent row was first written, epoch ms.
    pub first_seen: i64,
    /// Last update, epoch ms.
    pub last_seen: i64,
}

/// Typed read/write access to the `process_ledger` table over its OWN
/// connection (WAL: concurrent with the main store), so the
/// supervisor can write from spawn threads and reconciler ticks
/// without the app's storage mutex.
/// The connection sits behind a mutex: the repo is shared across
/// spawn threads and reconciler ticks (`Send + Sync`).
#[derive(Debug)]
pub struct ProcessLedgerRepo {
    conn: std::sync::Mutex<Connection>,
}

impl ProcessLedgerRepo {
    /// Opens (or creates) a ledger at `path`. The table comes from
    /// schema v12; a fresh file gets it applied directly (the repo is
    /// also the standalone creator for supervisor-side tests).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn open(path: &std::path::Path) -> Result<Self, CoreError> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(conn)
    }

    /// In-memory ledger (tests).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn open_in_memory() -> Result<Self, CoreError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, CoreError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS process_ledger (
                pid INTEGER,
                pgid INTEGER,
                started_at INTEGER,
                exe_path TEXT NOT NULL,
                cmdline_hash TEXT NOT NULL,
                cmdline_trunc TEXT NOT NULL,
                boot_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                instance_id INTEGER,
                provider TEXT NOT NULL,
                account_id TEXT,
                project_id TEXT,
                machine_id TEXT NOT NULL,
                root_pid INTEGER,
                parent_pid INTEGER,
                depth INTEGER NOT NULL DEFAULT 0,
                role TEXT NOT NULL,
                state TEXT NOT NULL,
                listens_on TEXT,
                cwd TEXT NOT NULL,
                peak_rss INTEGER,
                peak_cpu REAL,
                exit_kind TEXT,
                reaped_by_us INTEGER,
                escaped_group INTEGER NOT NULL DEFAULT 0,
                down_period INTEGER NOT NULL DEFAULT 0,
                first_seen INTEGER NOT NULL,
                last_seen INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS process_ledger_session_state
                ON process_ledger(session_id, state);
            CREATE INDEX IF NOT EXISTS process_ledger_pid_start
                ON process_ledger(pid, started_at, boot_id);
            CREATE INDEX IF NOT EXISTS process_ledger_state_last_seen
                ON process_ledger(state, last_seen);",
        )?;
        // v13's `escaped_group` and v16's `down_period` columns on a
        // LEGACY file are added by the main store's migration
        // (`Storage::open` always runs before this repo in the app; two
        // writers for one ALTER would race to "duplicate column"). This
        // CREATE only builds standalone ledgers (supervisor tests),
        // already in the v16 shape.
        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }

    /// Writes the Intent row (state `Starting`, no PID) BEFORE the
    /// spawn — the ownership record that survives a crash between
    /// intent and spawn.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn intent(&self, row: &LedgerRow) -> Result<(), CoreError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO process_ledger
                 (pid, pgid, started_at, exe_path, cmdline_hash, cmdline_trunc, boot_id,
                  session_id, instance_id, provider, account_id, project_id, machine_id,
                  root_pid, parent_pid, depth, role, state, listens_on, cwd,
                  peak_rss, peak_cpu, exit_kind, reaped_by_us, first_seen, last_seen)
             VALUES (NULL, ?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                     ?12, ?13, ?14, ?15, ?16, ?17, ?18, NULL, NULL, NULL, NULL, ?19, ?19)",
            params![
                row.pgid,
                row.exe_path,
                row.cmdline_hash,
                row.cmdline_trunc,
                row.boot_id,
                row.session_id,
                row.instance_id,
                row.provider,
                row.account_id,
                row.project_id,
                row.machine_id,
                row.root_pid,
                row.parent_pid,
                row.depth,
                row.role,
                STATE_STARTING,
                row.listens_on,
                row.cwd,
                row.first_seen,
            ],
        )?;
        Ok(())
    }

    /// Patches the newest Intent row of `session_id` with the spawn
    /// result: PID, PGID, OS start time; state → `running`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    #[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
    pub fn spawned(
        &self,
        session_id: &str,
        pid: u32,
        pgid: Option<u32>,
        started_at: Option<i64>,
        now_ms: i64,
    ) -> Result<(), CoreError> {
        let conn = self.lock()?;
        let changed = conn.execute(
            "UPDATE process_ledger SET
                 pid = ?2, pgid = COALESCE(?3, pgid), started_at = COALESCE(?4, started_at),
                 state = ?5, last_seen = ?6
             WHERE rowid IN (
                 SELECT rowid FROM process_ledger
                 WHERE session_id = ?1 AND state = ?7
                 ORDER BY first_seen DESC LIMIT 1
             )",
            params![
                session_id,
                pid,
                pgid,
                started_at,
                STATE_RUNNING,
                now_ms,
                STATE_STARTING
            ],
        )?;
        let _ = changed; // 0 → the spawn raced an exit patch; exit wins.
        Ok(())
    }

    /// Batched helper upsert — ONE transaction for the whole tick
    /// (≤500 rows ≤20 ms budget). Helpers key on
    /// `(session_id, pid, boot_id)`: new pids insert, known pids
    /// refresh `last_seen` + peaks; pids no longer present stay
    /// untouched (exit patch owns their fate).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn upsert_helpers(&self, session_id: &str, helpers: &[LedgerRow]) -> Result<(), CoreError> {
        let conn = self.lock()?;
        let tx = conn.unchecked_transaction()?;
        {
            // No unique index exists (append-first by design), so the
            // upsert is: refresh a known running row; insert only when
            // nothing matched. One transaction for the whole tick.
            let mut update = tx.prepare(&format!(
                "UPDATE process_ledger SET
                     last_seen = ?2,
                     peak_rss = MAX(COALESCE(peak_rss, 0), COALESCE(?3, 0)),
                     peak_cpu = MAX(COALESCE(peak_cpu, 0.0), COALESCE(?4, 0.0)),
                     pgid = COALESCE(?8, pgid),
                     escaped_group = MAX(escaped_group, ?9),
                     started_at = COALESCE(started_at, ?10),
                     -- Identity refinement only inside the exec-settle
                     -- window (AJ-E508, EXEC_SETTLE_MS): a helper
                     -- discovered pre-exec legitimately changes image
                     -- (sh → python → vite); after the window the
                     -- identity is frozen and a mismatch flips
                     -- unverifiable (PID-reuse guard). The supervisor's
                     -- reconciler withholds its exe verdict for the
                     -- same window — one shared constant.
                     exe_path = CASE
                         WHEN ?2 - first_seen < {EXEC_SETTLE_MS} AND ?11 LIKE '/%' THEN ?11
                         ELSE exe_path END,
                     cmdline_hash = CASE
                         WHEN ?2 - first_seen < {EXEC_SETTLE_MS} AND ?12 != '' THEN ?12
                         ELSE cmdline_hash END,
                     cmdline_trunc = CASE
                         WHEN ?2 - first_seen < {EXEC_SETTLE_MS} AND ?13 != '' THEN ?13
                         ELSE cmdline_trunc END
                 WHERE session_id = ?1 AND pid = ?5 AND boot_id = ?6 AND state = ?7",
            ))?;
            let mut insert = tx.prepare(
                "INSERT INTO process_ledger
                     (pid, pgid, started_at, exe_path, cmdline_hash, cmdline_trunc, boot_id,
                      session_id, instance_id, provider, account_id, project_id, machine_id,
                      root_pid, parent_pid, depth, role, state, listens_on, cwd,
                      peak_rss, peak_cpu, exit_kind, reaped_by_us, escaped_group,
                      first_seen, last_seen)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                         ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, NULL, NULL, ?23,
                         ?24, ?24)",
            )?;
            for row in helpers {
                let changed = update.execute(params![
                    session_id,
                    row.last_seen,
                    row.peak_rss,
                    row.peak_cpu,
                    row.pid,
                    row.boot_id,
                    STATE_RUNNING,
                    row.pgid,
                    i64::from(row.escaped_group),
                    row.started_at,
                    row.exe_path,
                    row.cmdline_hash,
                    row.cmdline_trunc,
                ])?;
                if changed == 0 {
                    insert.execute(params![
                        row.pid,
                        row.pgid,
                        row.started_at,
                        row.exe_path,
                        row.cmdline_hash,
                        row.cmdline_trunc,
                        row.boot_id,
                        session_id,
                        row.instance_id,
                        row.provider,
                        row.account_id,
                        row.project_id,
                        row.machine_id,
                        row.root_pid,
                        row.parent_pid,
                        row.depth,
                        ROLE_HELPER,
                        STATE_RUNNING,
                        row.listens_on,
                        row.cwd,
                        row.peak_rss,
                        row.peak_cpu,
                        i64::from(row.escaped_group),
                        row.first_seen,
                    ])?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Patches one row's exit (classification + reaped-by-us).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn mark_exited(
        &self,
        pid: u32,
        boot_id: &str,
        exit_kind: &str,
        reaped_by_us: bool,
        now_ms: i64,
    ) -> Result<(), CoreError> {
        let conn = self.lock()?;
        // `(pid, boot_id)` is the identity key (E503); the newest
        // running row for it is the one that ended. An ORPHANED row
        // may also exit (post-confirmation) — it resolves to `exited`
        // and leaves the Cleanup Centre.
        conn.execute(
            "UPDATE process_ledger SET
                 state = ?2, exit_kind = ?3, reaped_by_us = ?4, last_seen = ?5
             WHERE rowid IN (
                 SELECT rowid FROM process_ledger
                 WHERE pid = ?1 AND boot_id = ?6 AND state IN (?7, ?8)
                 ORDER BY first_seen DESC LIMIT 1
             )",
            params![
                pid,
                STATE_EXITED,
                exit_kind,
                i64::from(reaped_by_us),
                now_ms,
                boot_id,
                STATE_RUNNING,
                STATE_ORPHANED
            ],
        )?;
        Ok(())
    }

    /// Flips the newest RUNNING row for `(pid, boot_id)` to the
    /// permanent `unverifiable` state (AJ-E503 Req.5) — the row
    /// exits `candidates` (running-only) and every kill path, and
    /// the exited-only prune can never remove it.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn mark_unverifiable(&self, pid: u32, boot_id: &str, now_ms: i64) -> Result<(), CoreError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE process_ledger SET
                 state = ?2, last_seen = ?3
             WHERE rowid IN (
                 SELECT rowid FROM process_ledger
                 WHERE pid = ?1 AND boot_id = ?4 AND state = ?5
                 ORDER BY first_seen DESC LIMIT 1
             )",
            params![pid, STATE_UNVERIFIABLE, now_ms, boot_id, STATE_RUNNING],
        )?;
        Ok(())
    }

    /// Batched listener join (AJ-E505 Req.2): sets `listens_on` for
    /// the newest running-or-orphaned row of each `(pid, boot_id)` —
    /// ONE transaction for the whole tick; a pid absent from the
    /// table is NOT cleared (its ports simply stop refreshing; the
    /// exit patch owns the row's fate).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn apply_ports(&self, entries: &[(u32, &str, &str)]) -> Result<(), CoreError> {
        let conn = self.lock()?;
        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "UPDATE process_ledger SET listens_on = ?3
                 WHERE rowid IN (
                     SELECT rowid FROM process_ledger
                     WHERE pid = ?1 AND boot_id = ?2 AND state IN (?4, ?5)
                     ORDER BY first_seen DESC LIMIT 1
                 )",
            )?;
            for (pid, boot_id, ports) in entries {
                stmt.execute(params![pid, boot_id, ports, STATE_RUNNING, STATE_ORPHANED])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Flips a RUNNING helper row to `orphaned` (AJ-E504 Req.3):
    /// root exited, grace elapsed, two ticks agreed. The row stays a
    /// candidate; an orphan that later exits flips to `exited` via
    /// [`ProcessLedgerRepo::mark_exited`].
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    /// Crash recovery's variant (AJ-E510 Req.2): same state flip,
    /// plus the sticky down-period marker.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn mark_orphaned_down(
        &self,
        pid: u32,
        boot_id: &str,
        now_ms: i64,
    ) -> Result<(), CoreError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE process_ledger SET
                 state = ?2, last_seen = ?3, down_period = 1
             WHERE rowid IN (
                 SELECT rowid FROM process_ledger
                 WHERE pid = ?1 AND boot_id = ?4 AND state = ?5 AND role = ?6
                 ORDER BY first_seen DESC LIMIT 1
             )",
            params![
                pid,
                STATE_ORPHANED,
                now_ms,
                boot_id,
                STATE_RUNNING,
                ROLE_HELPER
            ],
        )?;
        Ok(())
    }

    /// Flips a RUNNING helper row to `orphaned` (AJ-E504 Req.3):
    /// root exited, grace elapsed, two ticks agreed. The row stays a
    /// candidate; an orphan that later exits flips to `exited` via
    /// [`ProcessLedgerRepo::mark_exited`].
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn mark_orphaned(&self, pid: u32, boot_id: &str, now_ms: i64) -> Result<(), CoreError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE process_ledger SET
                 state = ?2, last_seen = ?3
             WHERE rowid IN (
                 SELECT rowid FROM process_ledger
                 WHERE pid = ?1 AND boot_id = ?4 AND state = ?5 AND role = ?6
                 ORDER BY first_seen DESC LIMIT 1
             )",
            params![
                pid,
                STATE_ORPHANED,
                now_ms,
                boot_id,
                STATE_RUNNING,
                ROLE_HELPER
            ],
        )?;
        Ok(())
    }

    /// Candidate rows for orphan reconciliation (AJ-E504's input):
    /// every `running` **or** `orphaned` row on a machine — both are
    /// re-verified each tick; `exited`/`unverifiable` never return.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn candidates(&self, machine_id: &str) -> Result<Vec<LedgerRow>, CoreError> {
        let conn = self.lock()?;
        let mut out = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT * FROM process_ledger
             WHERE state IN (?1, ?3) AND machine_id = ?2",
        )?;
        let rows = stmt.query_map(params![STATE_RUNNING, machine_id, STATE_ORPHANED], map_row)?;
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// All rows for one session (tests + the Processes view later).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn rows_for_session(&self, session_id: &str) -> Result<Vec<LedgerRow>, CoreError> {
        let conn = self.lock()?;
        let mut out = Vec::new();
        let mut stmt =
            conn.prepare("SELECT * FROM process_ledger WHERE session_id = ?1 ORDER BY depth, pid")?;
        let rows = stmt.query_map(params![session_id], map_row)?;
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Every LIVE (session, machine) composite key (AJ-E511): the
    /// all-machines inventory walks these, then reads each key's
    /// rows — sessions on two machines appear as two keys, never
    /// merged (the D-13 join rule).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn live_session_machines(&self) -> Result<Vec<(String, String)>, CoreError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT session_id, machine_id FROM process_ledger
             WHERE state IN ('running', 'orphaned', 'unverifiable')",
        )?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The FLEET composite-key read (AJ-E511, D-13): one session's
    /// rows ON ONE MACHINE. `session_id` alone is NOT unique once
    /// machines sync — every cross-machine join goes through
    /// `session_id+machine_id`. Rides the v17 index.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn rows_for_session_on_machine(
        &self,
        session_id: &str,
        machine_id: &str,
    ) -> Result<Vec<LedgerRow>, CoreError> {
        let conn = self.lock()?;
        let mut out = Vec::new();
        let mut stmt = conn
            .prepare("SELECT * FROM process_ledger WHERE session_id = ?1 AND machine_id = ?2 ORDER BY depth, pid")?;
        let rows = stmt.query_map(params![session_id, machine_id], map_row)?;
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Prunes `exited` rows last seen before `before_ms`. Running
    /// rows are NEVER pruned. Returns the removed count.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn prune(&self, before_ms: i64) -> Result<usize, CoreError> {
        let conn = self.lock()?;
        let removed = conn.execute(
            "DELETE FROM process_ledger WHERE state = ?1 AND last_seen < ?2",
            params![STATE_EXITED, before_ms],
        )?;
        Ok(removed)
    }
}

impl ProcessLedgerRepo {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, CoreError> {
        self.conn
            .lock()
            .map_err(|_| CoreError::Storage(rusqlite::Error::ExecuteReturnedResults))
    }
}

/// Maps a row (values unchanged; ADR-0008 A-1). Unreadable values
/// degrade to errors, never panics.
fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LedgerRow> {
    Ok(LedgerRow {
        pid: row.get::<_, Option<u32>>("pid")?,
        pgid: row.get::<_, Option<u32>>("pgid")?,
        started_at: row.get::<_, Option<i64>>("started_at")?,
        exe_path: row.get("exe_path")?,
        cmdline_hash: row.get("cmdline_hash")?,
        cmdline_trunc: row.get("cmdline_trunc")?,
        boot_id: row.get("boot_id")?,
        session_id: row.get("session_id")?,
        instance_id: row.get("instance_id")?,
        provider: row.get("provider")?,
        account_id: row.get("account_id")?,
        project_id: row.get("project_id")?,
        machine_id: row.get("machine_id")?,
        root_pid: row.get::<_, Option<u32>>("root_pid")?,
        parent_pid: row.get::<_, Option<u32>>("parent_pid")?,
        depth: row.get::<_, u32>("depth")?,
        role: row.get("role")?,
        state: row.get("state")?,
        listens_on: row.get("listens_on")?,
        cwd: row.get("cwd")?,
        peak_rss: row.get("peak_rss")?,
        peak_cpu: row.get("peak_cpu")?,
        exit_kind: row.get("exit_kind")?,
        reaped_by_us: row.get::<_, Option<i64>>("reaped_by_us")?.map(|v| v != 0),
        escaped_group: row.get::<_, i64>("escaped_group")? != 0,
        down_period: row.get::<_, i64>("down_period")? != 0,
        first_seen: row.get("first_seen")?,
        last_seen: row.get("last_seen")?,
    })
}
