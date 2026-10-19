//! `stats/counters.sqlite`: this week's counters, gauges and active days,
//! plus a little state. The folder exists only while the mode isn't off.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

/// Storage failures (logged, never surfaced to callers of `record`).
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// SQLite.
    #[error("usage-stats store: {0}")]
    Sql(#[from] rusqlite::Error),
    /// The filesystem.
    #[error("usage-stats store: {0}")]
    Io(#[from] std::io::Error),
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS counters (week TEXT NOT NULL, metric TEXT NOT NULL, dims TEXT NOT NULL, value INTEGER NOT NULL,
                                     PRIMARY KEY (week, metric, dims));
CREATE TABLE IF NOT EXISTS gauges   (week TEXT NOT NULL, metric TEXT NOT NULL, dims TEXT NOT NULL, value TEXT NOT NULL,
                                     PRIMARY KEY (week, metric, dims));
CREATE TABLE IF NOT EXISTS active_days (day TEXT PRIMARY KEY);
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
";

/// The open store.
#[derive(Debug)]
pub struct Store {
    dir: PathBuf,
    conn: Connection,
}

/// Creates `dir` readable only by this user.
fn private_dir(dir: &Path) -> Result<(), StoreError> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

impl Store {
    /// Opens (creating) the store in `dir` — only call when the mode isn't
    /// off.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on filesystem or SQLite failures.
    pub fn open(dir: &Path) -> Result<Self, StoreError> {
        private_dir(dir)?;
        for sub in ["pending", "sent", "skipped"] {
            private_dir(&dir.join(sub))?;
        }
        let conn = Connection::open(dir.join("counters.sqlite"))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // WAL keeps the store consistent at NORMAL; counters don't need an
        // fsync per flush.
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(2))?;
        conn.execute_batch(SCHEMA)?;
        let store = Self {
            dir: dir.to_path_buf(),
            conn,
        };
        if store.meta("salt")?.is_none() {
            let mut salt = [0u8; 32];
            getrandom::fill(&mut salt).map_err(|e| std::io::Error::other(e.to_string()))?;
            store.set_meta("salt", &hex(&salt))?;
        }
        Ok(store)
    }

    /// Runs `f` inside one transaction (a flush is one write).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures; the transaction is
    /// rolled back.
    pub fn transaction<T>(
        &self,
        f: impl FnOnce(&Self) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match f(self) {
            Ok(value) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(value)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    /// The store's folder.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Adds `by` to a counter.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures.
    pub fn add(&self, week: &str, metric: &str, dims: &str, by: u64) -> Result<(), StoreError> {
        let by = i64::try_from(by).unwrap_or(i64::MAX);
        self.conn
            .prepare_cached(
                "INSERT INTO counters (week, metric, dims, value) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(week, metric, dims) DO UPDATE SET value = MIN(value + ?4, 9223372036854775807)",
            )?
            .execute(params![week, metric, dims, by])?;
        Ok(())
    }

    /// Sets a gauge (last value wins).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures.
    pub fn set_gauge(
        &self,
        week: &str,
        metric: &str,
        dims: &str,
        value: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .prepare_cached(
                "INSERT INTO gauges (week, metric, dims, value) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(week, metric, dims) DO UPDATE SET value = ?4",
            )?
            .execute(params![week, metric, dims, value])?;
        Ok(())
    }

    /// Records a UTC date the app ran (`2026-10-18`; days line up with the
    /// UTC week, so a report's `days_active` is 0–7).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures.
    pub fn mark_active(&self, day: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO active_days (day) VALUES (?1)",
            params![day],
        )?;
        Ok(())
    }

    /// Every counter of `week`: `(metric, dims json, value)`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures.
    pub fn counters(&self, week: &str) -> Result<Vec<(String, String, u64)>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT metric, dims, value FROM counters WHERE week = ?1 ORDER BY metric, dims",
        )?;
        let rows = statement.query_map(params![week], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get::<_, i64>(2)?.unsigned_abs(),
            ))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Every gauge of `week`: `(metric, dims json, value)`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures.
    pub fn gauges(&self, week: &str) -> Result<Vec<(String, String, String)>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT metric, dims, value FROM gauges WHERE week = ?1 ORDER BY metric, dims",
        )?;
        let rows = statement.query_map(params![week], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Weeks that have counters or gauges.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures.
    pub fn weeks(&self) -> Result<Vec<String>, StoreError> {
        let mut statement = self
            .conn
            .prepare("SELECT week FROM counters UNION SELECT week FROM gauges ORDER BY week")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// UTC dates the app ran between `from` and `to` (inclusive).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures.
    pub fn active_days(&self, from: &str, to: &str) -> Result<u32, StoreError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM active_days WHERE day >= ?1 AND day <= ?2",
            params![from, to],
            |row| row.get(0),
        )?;
        Ok(u32::try_from(n).unwrap_or(0))
    }

    /// Drops a week's counters and gauges, and active days up to `before`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures.
    pub fn forget_week(&self, week: &str, days_before: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM counters WHERE week = ?1", params![week])?;
        self.conn
            .execute("DELETE FROM gauges WHERE week = ?1", params![week])?;
        self.conn.execute(
            "DELETE FROM active_days WHERE day < ?1",
            params![days_before],
        )?;
        Ok(())
    }

    /// A state value.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures.
    pub fn meta(&self, key: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Sets a state value.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures.
    pub fn set_meta(&self, key: &str, value: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    /// Removes a state value.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures.
    pub fn delete_meta(&self, key: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM meta WHERE key = ?1", params![key])?;
        Ok(())
    }

    /// The sampling salt (32 random bytes, never sent).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on SQLite failures.
    pub fn salt(&self) -> Result<Vec<u8>, StoreError> {
        let hex = self.meta("salt")?.unwrap_or_default();
        Ok((0..hex.len())
            .step_by(2)
            .filter_map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
            .collect())
    }
}

/// Lowercase hex.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// Deletes the whole stats folder (turning stats off): counters, reports,
/// salt and the upload log.
///
/// # Errors
///
/// Returns the filesystem error.
pub fn delete_all(dir: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}
