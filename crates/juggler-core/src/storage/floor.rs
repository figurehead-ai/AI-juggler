//! The Floor's saved choices (SPEC §9.7, AJ-E3503, schema v46): per App,
//! its place in the order the Floor first saw it, its family and look, its
//! size and since when it has fitted a smaller one. The office seed lives
//! in `app_settings` ([`SEED_KEY`]).
//!
//! - A row is written the first time the Floor sees an App and then only
//!   when its size or look changes; nothing here decides either (the
//!   `juggler-floor` model does).
//! - The family and layout are stored as words: storage knows no tool and
//!   no family.
//! - An App removed takes its row with it; Re-create rewrites every row's
//!   look and the seed in one transaction and keeps the order.
//!
//! Nothing outside the Floor reads or writes these rows.

use rusqlite::{Connection, params};

use crate::error::CoreError;

/// v46: the Floor's areas.
pub(super) const MIGRATION: &str = "
CREATE TABLE floor_areas (
    app_id      TEXT PRIMARY KEY REFERENCES apps(id) ON DELETE CASCADE,
    seen_order  INTEGER NOT NULL CHECK (seen_order >= 0),
    family      TEXT NOT NULL,
    layout      TEXT NOT NULL,
    seed        INTEGER NOT NULL CHECK (seed >= 0 AND seed <= 4294967295),
    desks       INTEGER NOT NULL CHECK (desks > 0),
    fits_smaller_since INTEGER,
    hidden      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_floor_areas_order ON floor_areas(seen_order);
";

/// The `app_settings` key of the office seed.
pub const SEED_KEY: &str = "floor.seed";

/// The most areas read at once: far beyond any real office, so a damaged
/// table can't make the Floor read without end.
pub const FLOOR_AREAS_LIMIT: u32 = 2000;

/// One App's saved area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloorArea {
    /// The App.
    pub app_id: String,
    /// Its place in the order the Floor first saw Apps.
    pub order: u32,
    /// Its tool's family word when it was first seen.
    pub family: String,
    /// Its layout word.
    pub layout: String,
    /// Its look's seed.
    pub seed: u32,
    /// The desks it holds.
    pub desks: u32,
    /// Since when it has fitted fewer desks (epoch ms).
    pub fits_smaller_since: Option<i64>,
    /// Left off the Floor by the user (Settings › The Floor › More).
    pub hidden: bool,
}

/// Read/write access to `floor_areas`.
#[derive(Debug)]
pub struct FloorRepo<'a> {
    conn: &'a Connection,
}

impl FloorRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> FloorRepo<'_> {
        FloorRepo { conn }
    }

    /// Every saved area, in order (at most [`FLOOR_AREAS_LIMIT`]).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn areas(&self) -> Result<Vec<FloorArea>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT app_id, seen_order, family, layout, seed, desks, fits_smaller_since, hidden
             FROM floor_areas ORDER BY seen_order, app_id LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![FLOOR_AREAS_LIMIT], |row| {
            Ok(FloorArea {
                app_id: row.get(0)?,
                order: to_u32(row.get(1)?),
                family: row.get(2)?,
                layout: row.get(3)?,
                seed: to_u32(row.get(4)?),
                desks: to_u32(row.get(5)?),
                fits_smaller_since: row.get(6)?,
                hidden: row.get::<_, i64>(7)? != 0,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Saves the areas that changed (insert or update), in one
    /// transaction; `hidden` is the user's and only [`Self::set_hidden`]
    /// changes it.
    ///
    /// # Errors
    ///
    /// [`CoreError::Validation`] for an area with no desks;
    /// [`CoreError::Storage`] on SQLite failures (an unknown App among
    /// them), and then nothing is saved.
    pub fn save(&self, areas: &[FloorArea]) -> Result<(), CoreError> {
        if areas.iter().any(|area| area.desks == 0) {
            return Err(CoreError::Validation {
                subject: "floor area",
                message: "an area holds at least one desk".to_owned(),
            });
        }
        let tx = crate::storage::write_transaction(self.conn)?;
        for area in areas {
            tx.execute(
                "INSERT INTO floor_areas
                     (app_id, seen_order, family, layout, seed, desks, fits_smaller_since)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (app_id) DO UPDATE SET seen_order = ?2, family = ?3, layout = ?4,
                     seed = ?5, desks = ?6, fits_smaller_since = ?7",
                params![
                    area.app_id,
                    area.order,
                    area.family,
                    area.layout,
                    area.seed,
                    area.desks,
                    area.fits_smaller_since
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Re-create: the new office seed and every App's new look, in one
    /// transaction; the order, sizes and what's hidden stay.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures, and then nothing
    /// changes.
    pub fn recreate(&self, seed: u32, looks: &[(String, String, u32)]) -> Result<(), CoreError> {
        let tx = crate::storage::write_transaction(self.conn)?;
        tx.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SEED_KEY, seed.to_string()],
        )?;
        for (app_id, layout, look_seed) in looks {
            tx.execute(
                "UPDATE floor_areas SET layout = ?2, seed = ?3 WHERE app_id = ?1",
                params![app_id, layout, look_seed],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Leaves an App off the Floor, or puts it back.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn set_hidden(&self, app_id: &str, hidden: bool) -> Result<bool, CoreError> {
        Ok(self.conn.execute(
            "UPDATE floor_areas SET hidden = ?2 WHERE app_id = ?1",
            params![app_id, i64::from(hidden)],
        )? > 0)
    }
}

fn to_u32(n: i64) -> u32 {
    u32::try_from(n.max(0)).unwrap_or(u32::MAX)
}
