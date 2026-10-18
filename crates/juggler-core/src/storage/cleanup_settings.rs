//! Per-Account cleanup tier settings (AJ-E508, D-08): `tier1_opt_in`
//! (`Automatically tidy left-behind dev servers`) and `tier2_opt_in`
//! (`Also tidy after a crash (with notification)`) — TWO separate
//! toggles, both off by default, never bundled (ADR-0010's E508
//! amendment). The E410 prototype's single `accounts.cleanup_opt_in`
//! migrates to Tier 1 only; Tier 2 starts off for everyone.

use std::sync::Mutex;

use rusqlite::Connection;

use super::CoreError;

/// A poisoned settings lock is an internal invariant breach — the
/// honest typed failure (never a panic).
fn poisoned() -> CoreError {
    CoreError::Storage(rusqlite::Error::InvalidParameterCount(0, 1))
}

/// One Account's tier opt-ins (defaults are the product's promise).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct CleanupTierSettings {
    /// Tier 1 — tidy verified orphans after CLEAN exits.
    pub tier1_opt_in: bool,
    /// Tier 2 — tidy after crash roots (5 min + notification with
    /// an Undo window). Separate consent, never implied by Tier 1.
    pub tier2_opt_in: bool,
}

/// Typed access to `cleanup_settings` over its OWN WAL connection
/// (the sweep's per-tick read must not queue behind the app storage
/// mutex; writes are rare — two toggles per Account). The connection
/// sits behind a mutex so the shared `AppState` stays `Send + Sync`.
#[derive(Debug)]
pub struct CleanupSettingsRepo {
    conn: Mutex<Connection>,
}

impl CleanupSettingsRepo {
    /// Opens (creating if needed) the settings table at `path`.
    ///
    /// # Errors
    ///
    /// [`CoreError::Storage`] when the file cannot be opened or the
    /// table cannot be created.
    pub fn open(path: &std::path::Path) -> Result<Self, CoreError> {
        let conn = Connection::open(path).map_err(CoreError::Storage)?;
        Self::init(conn)
    }

    /// The in-memory test seam.
    ///
    /// # Errors
    ///
    /// [`CoreError::Storage`] on table-creation failure.
    pub fn open_in_memory() -> Result<Self, CoreError> {
        Self::init(Connection::open_in_memory().map_err(CoreError::Storage)?)
    }

    fn init(conn: Connection) -> Result<Self, CoreError> {
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(CoreError::Storage)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cleanup_settings (
                account_id TEXT PRIMARY KEY,
                tier1_opt_in INTEGER NOT NULL DEFAULT 0,
                tier2_opt_in INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .map_err(CoreError::Storage)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// One Account's settings — absent rows read as the honest
    /// default (both off), never an error.
    ///
    /// # Errors
    ///
    /// [`CoreError::Storage`] on query failure.
    pub fn get(&self, account_id: &str) -> Result<CleanupTierSettings, CoreError> {
        // A poisoned lock reads as not-opted-in (fail closed) — the
        // sweep must never treat a lock failure as consent.
        let Ok(conn) = self.conn.lock() else {
            return Ok(CleanupTierSettings::default());
        };
        let mut stmt = conn
            .prepare(
                "SELECT tier1_opt_in, tier2_opt_in FROM cleanup_settings WHERE account_id = ?1",
            )
            .map_err(CoreError::Storage)?;
        match stmt.query_row([account_id], |row| {
            Ok(CleanupTierSettings {
                tier1_opt_in: row.get::<_, i64>(0)? != 0,
                tier2_opt_in: row.get::<_, i64>(1)? != 0,
            })
        }) {
            Ok(settings) => Ok(settings),
            // An absent row is the both-off default, never an error.
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(CleanupTierSettings::default()),
            Err(e) => Err(CoreError::Storage(e)),
        }
    }

    /// Sets one Account's two toggles (upsert).
    ///
    /// # Errors
    ///
    /// [`CoreError::Storage`] on write failure (a poisoned lock
    /// included).
    pub fn set(&self, account_id: &str, settings: &CleanupTierSettings) -> Result<(), CoreError> {
        let conn = self.conn.lock().map_err(|_| poisoned())?;
        conn.execute(
            "INSERT INTO cleanup_settings (account_id, tier1_opt_in, tier2_opt_in)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(account_id) DO UPDATE SET
                tier1_opt_in = excluded.tier1_opt_in,
                tier2_opt_in = excluded.tier2_opt_in",
            rusqlite::params![
                account_id,
                i64::from(settings.tier1_opt_in),
                i64::from(settings.tier2_opt_in)
            ],
        )
        .map_err(CoreError::Storage)?;
        Ok(())
    }

    /// Every explicitly-set row (fixture/diagnostic reads; the sweep
    /// resolves per Account).
    ///
    /// # Errors
    ///
    /// [`CoreError::Storage`] on query failure.
    pub fn all(&self) -> Result<Vec<(String, CleanupTierSettings)>, CoreError> {
        let conn = self.conn.lock().map_err(|_| poisoned())?;
        let mut stmt = conn
            .prepare("SELECT account_id, tier1_opt_in, tier2_opt_in FROM cleanup_settings")
            .map_err(CoreError::Storage)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    CleanupTierSettings {
                        tier1_opt_in: row.get::<_, i64>(1)? != 0,
                        tier2_opt_in: row.get::<_, i64>(2)? != 0,
                    },
                ))
            })
            .map_err(CoreError::Storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn absent_rows_read_as_both_off() {
        let repo = CleanupSettingsRepo::open_in_memory().unwrap();
        assert_eq!(
            repo.get("no-such-account").unwrap(),
            CleanupTierSettings::default()
        );
    }

    #[test]
    fn set_upserts_both_toggles_independently() {
        let repo = CleanupSettingsRepo::open_in_memory().unwrap();
        repo.set(
            "a1",
            &CleanupTierSettings {
                tier1_opt_in: true,
                tier2_opt_in: false,
            },
        )
        .unwrap();
        assert!(repo.get("a1").unwrap().tier1_opt_in);
        assert!(!repo.get("a1").unwrap().tier2_opt_in);
        // Tier 2 is a SEPARATE consent: setting it never moves Tier 1.
        repo.set(
            "a1",
            &CleanupTierSettings {
                tier1_opt_in: true,
                tier2_opt_in: true,
            },
        )
        .unwrap();
        let all = repo.all().unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].1.tier1_opt_in && all[0].1.tier2_opt_in);
    }
}
