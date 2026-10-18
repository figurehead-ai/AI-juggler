//! App-level settings repository (AJ-E414, schema v10): a string kv
//! for user preferences that belong to the app rather than one Account
//! or launch recipe — today the global "Go to instance…" hotkey
//! binding; future features namespace their own keys. Values are opaque
//! strings to storage; validation lives with the owning feature.

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::CoreError;

/// Typed read/write access to the `app_settings` table.
#[derive(Debug)]
pub struct AppSettingsRepo<'a> {
    conn: &'a Connection,
}

impl AppSettingsRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> AppSettingsRepo<'_> {
        AppSettingsRepo { conn }
    }

    /// Reads one setting; `Ok(None)` = never set (the caller applies
    /// its feature default).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn get(&self, key: &str) -> Result<Option<String>, CoreError> {
        self.conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>("value"),
            )
            .optional()
            .map_err(CoreError::Storage)
    }

    /// Writes one setting (upsert). An empty value deletes the row so
    /// "reset to default" is a real state, never a stored blank.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn set(&self, key: &str, value: &str) -> Result<(), CoreError> {
        if value.is_empty() {
            self.conn
                .execute("DELETE FROM app_settings WHERE key = ?1", params![key])?;
        } else {
            self.conn.execute(
                "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
        }
        Ok(())
    }

    /// Lists every setting (diagnostics/export; ordered by key).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn list(&self) -> Result<Vec<(String, String)>, CoreError> {
        let mut out = Vec::new();
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM app_settings ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>("key")?, row.get::<_, String>("value")?))
        })?;
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}
