//! The cost table a page at a time (0.1.5 review: Usage read every
//! usage event and drew every session × model × day row at once).
//!
//! 0.1.7: a row is one session's UTC day, its models folded together
//! (the maintainer saw near-duplicate rows, one per model a session used
//! that day). Pages are keyset ones: newest day first, then the session
//! id, so a row added meanwhile never shifts or repeats what was shown.

use rusqlite::params;

use super::{UsageRepo, parse_confidence};
use crate::error::CoreError;
use crate::quota::{UsageEvent, UsageSource};

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
        let mut stmt = self.conn.prepare(
            "SELECT account, session, ts, model, input, output, cache_r, cache_w,
                    reasoning, credits, usd_est, source, confidence, msg_uuid
               FROM usage_events
              WHERE (?1 IS NULL OR account = ?1) AND ts >= ?2 AND ts < ?3
              ORDER BY ts",
        )?;
        let rows = stmt.query_map(
            params![
                account,
                first.saturating_mul(DAY_MS),
                last.saturating_add(1).saturating_mul(DAY_MS)
            ],
            |row| {
                Ok(UsageEvent {
                    account: row.get(0)?,
                    session: row.get(1)?,
                    ts: row.get(2)?,
                    model: row.get(3)?,
                    tokens: crate::model::TokenCounts {
                        input: u64::try_from(row.get::<_, i64>(4)?).unwrap_or(0),
                        output: u64::try_from(row.get::<_, i64>(5)?).unwrap_or(0),
                        cache_read: u64::try_from(row.get::<_, i64>(6)?).unwrap_or(0),
                        cache_write: u64::try_from(row.get::<_, i64>(7)?).unwrap_or(0),
                        reasoning: u64::try_from(row.get::<_, i64>(8)?).unwrap_or(0),
                    },
                    credits: row.get(9)?,
                    usd_est: row.get(10)?,
                    source: UsageSource::parse(&row.get::<_, String>(11)?)
                        .unwrap_or(UsageSource::Transcript),
                    confidence: parse_confidence(&row.get::<_, String>(12)?),
                    msg_uuid: row.get(13)?,
                })
            },
        )?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }
}
