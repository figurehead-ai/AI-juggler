//! Profile repository.

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::CoreError;
use crate::model::{AccountId, Profile, ProfileId};

/// Converts a stored-JSON parse failure into a row-mapping failure.
fn json_err(e: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
}

/// Typed read/write access to the `profiles` table.
#[derive(Debug)]
pub struct ProfilesRepo<'a> {
    conn: &'a Connection,
}

impl ProfilesRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> ProfilesRepo<'_> {
        ProfilesRepo { conn }
    }

    /// Inserts a new profile.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on I/O or constraint violations
    /// (unknown account, duplicate id).
    pub fn insert(&self, profile: &Profile) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO launch_recipes (id, account_id, name, settings_json, default_cwd, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                profile.id.as_str(),
                profile.account.as_str(),
                profile.name,
                serde_json::to_string(&profile.settings_overlay)?,
                cwd_column(profile.default_cwd.as_deref()),
                crate::model::Timestamp::now().millis(),
            ],
        )?;
        Ok(())
    }

    /// Updates an existing profile by id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the id does not exist, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn update(&self, profile: &Profile) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE launch_recipes SET account_id = ?2, name = ?3, settings_json = ?4, default_cwd = ?5
             WHERE id = ?1",
            params![
                profile.id.as_str(),
                profile.account.as_str(),
                profile.name,
                serde_json::to_string(&profile.settings_overlay)?,
                cwd_column(profile.default_cwd.as_deref()),
            ],
        )?;
        super::accounts::require_changed(changed, "profile", profile.id.as_str())
    }

    /// Fetches one profile by id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing, [`CoreError::Storage`]
    /// on SQLite failures, or [`CoreError::Serialization`] for corrupt
    /// stored settings.
    pub fn get(&self, id: &ProfileId) -> Result<Profile, CoreError> {
        self.conn
            .query_row(
                "SELECT * FROM launch_recipes WHERE id = ?1",
                params![id.as_str()],
                map_profile,
            )
            .optional()?
            .ok_or_else(|| super::accounts::not_found("profile", id.as_str()))
    }

    /// Lists profiles, optionally filtered by account, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures or
    /// [`CoreError::Serialization`] for corrupt stored settings.
    pub fn list_by_account(&self, account: &AccountId) -> Result<Vec<Profile>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM launch_recipes WHERE account_id = ?1 ORDER BY created_at, id",
        )?;
        let rows = stmt.query_map(params![account.as_str()], map_profile)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Deletes a profile by id (cascades to sessions).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn delete(&self, id: &ProfileId) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "DELETE FROM launch_recipes WHERE id = ?1",
            params![id.as_str()],
        )?;
        super::accounts::require_changed(changed, "profile", id.as_str())
    }
}

/// Maps a row to a [`Profile`].
fn map_profile(row: &Row<'_>) -> rusqlite::Result<Profile> {
    let settings_json: String = row.get("settings_json")?;
    Ok(Profile {
        id: ProfileId::new(row.get::<_, String>("id")?),
        name: row.get("name")?,
        account: AccountId::new(row.get::<_, String>("account_id")?),
        settings_overlay: serde_json::from_str(&settings_json).map_err(json_err)?,
        default_cwd: row
            .get::<_, Option<String>>("default_cwd")?
            .map(std::path::PathBuf::from),
    })
}

/// Converts an optional path into its nullable TEXT column value.
fn cwd_column(path: Option<&std::path::Path>) -> Option<String> {
    path.map(std::path::Path::to_string_lossy)
        .map(std::borrow::Cow::into_owned)
}
