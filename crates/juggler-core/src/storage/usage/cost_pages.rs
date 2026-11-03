//! The cost table a page at a time (0.1.5 review: Usage read every
//! usage event and drew every session × model × day row at once).
//!
//! 0.1.8: a row is one session's UTC day, its models folded together
//! (the maintainer saw near-duplicate rows, one per model a session used
//! that day). Pages are keyset ones: newest day first, then the session
//! id, so a row added meanwhile never shifts or repeats what was shown.

use rusqlite::params;

use super::{EVENT_COLUMNS, UsageRepo, event_row};
use crate::error::CoreError;
use crate::quota::UsageEvent;

const DAY_MS: i64 = 86_400_000;

/// One row's place: its UTC day (days since the epoch) and its session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostDay {
    /// Days since 1970-01-01, UTC.
    pub day: i64,
    /// The session it's in (`""` when the events name none).
    pub session: String,
}

impl UsageRepo<'_> {
    /// Up to `limit` session days of `account` (every account when
    /// `None`), newest day first, after `after` (`(day, session)` of the
    /// last one shown).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn cost_days_page(
        &self,
        account: Option<&str>,
        after: Option<(i64, &str)>,
        limit: usize,
    ) -> Result<Vec<CostDay>, CoreError> {
        let (after_day, after_session) =
            after.map_or((None, None), |(day, session)| (Some(-day), Some(session)));
        let mut stmt = self.conn.prepare(&format!(
            "SELECT DISTINCT -(ts / {DAY_MS}) AS dayn, COALESCE(session, '') AS skey
               FROM usage_events
              WHERE (?1 IS NULL OR account = ?1)
                AND (?2 IS NULL OR dayn > ?2 OR (dayn = ?2 AND skey > ?3))
              ORDER BY dayn, skey
              LIMIT ?4"
        ))?;
        let rows = stmt.query_map(
            params![
                account,
                after_day,
                after_session,
                i64::try_from(limit).unwrap_or(i64::MAX)
            ],
            |row| {
                Ok(CostDay {
                    day: -row.get::<_, i64>(0)?,
                    session: row.get(1)?,
                })
            },
        )?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// How many session days `account` (every account when `None`) has.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn cost_day_count(&self, account: Option<&str>) -> Result<u64, CoreError> {
        let count: i64 = self.conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM (SELECT DISTINCT ts / {DAY_MS}, COALESCE(session, '')
                   FROM usage_events WHERE (?1 IS NULL OR account = ?1))"
            ),
            params![account],
            |row| row.get(0),
        )?;
        Ok(u64::try_from(count).unwrap_or(0))
    }

    /// The events of `account` (every account when `None`) in the UTC
    /// days `first..=last`, oldest first: what one page's buckets fold.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn events_in_days(
        &self,
        account: Option<&str>,
        first: i64,
        last: i64,
    ) -> Result<Vec<UsageEvent>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {EVENT_COLUMNS}
                  WHERE (?1 IS NULL OR e.account = ?1) AND e.ts >= ?2 AND e.ts < ?3
                  ORDER BY e.ts"
        ))?;
        let rows = stmt.query_map(
            params![
                account,
                first.saturating_mul(DAY_MS),
                last.saturating_add(1).saturating_mul(DAY_MS)
            ],
            event_row,
        )?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }
}
