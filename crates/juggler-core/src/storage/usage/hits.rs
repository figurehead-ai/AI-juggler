//! D-103.10: limit hits an earlier version credited to the wrong Account.
//! A conversation moved to another Account (ADR-0031) takes its transcript
//! along, and before 0.4.4 the hits in it were read where it landed. Which
//! Account a hit happened on is the move journal's to say, which the app
//! reads; here a hit's row is only found and moved.

use rusqlite::params;

use super::UsageRepo;
use crate::error::CoreError;
use crate::quota::{QuotaSample, UsageSource, WindowKind};
use crate::storage::write_transaction;

/// One limit hit to give back: the Account holding it wrongly, the one it
/// happened on (`None`: that one was removed, so it is no one's), and the
/// hit as its transcript records it.
pub type HitMove = (String, Option<String>, WindowKind, QuotaSample);

/// A hit's own row (`?1` window, `?2` instant, `?3` reset, `?4` the
/// transcript source): stored from its transcript at its instant, in its
/// window, with its reset.
const HIT_ROW: &str = "q.window = ?1 AND q.ts = ?2 AND q.resets_at IS ?3
    AND EXISTS (SELECT 1 FROM quota_sample_sources s
        WHERE s.account = q.account AND s.window = q.window AND s.ts = q.ts
          AND s.source = ?4)";

impl UsageRepo<'_> {
    /// The Accounts holding the limit hit `(kind, hit)` as a row of its
    /// own: stored from a transcript ([`UsageSource::ApiRetry`]) at its
    /// instant, in its window, with its reset.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn limit_hit_holders(
        &self,
        kind: WindowKind,
        hit: &QuotaSample,
    ) -> Result<Vec<String>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT q.account FROM quota_samples q WHERE {HIT_ROW} ORDER BY q.account"
        ))?;
        let rows = stmt.query_map(
            params![
                kind.as_str(),
                hit.ts,
                hit.resets_at,
                UsageSource::ApiRetry.as_str()
            ],
            |row| row.get(0),
        )?;
        rows.collect::<Result<_, _>>().map_err(CoreError::Storage)
    }

    /// Gives limit hits back (D-103.10), in ONE transaction: each hit's row
    /// taken off the Account holding it wrongly and put on the one it
    /// happened on, whole (the span it held too), unless that one holds
    /// that instant already; on none when that one was removed. A row
    /// that isn't the hit's own ([`Self::limit_hit_holders`]) is never
    /// touched. Returns how many rows were taken off.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; the transaction
    /// rolls back atomically, so nothing moved.
    pub fn give_back_limit_hits(&self, moves: &[HitMove]) -> Result<usize, CoreError> {
        let tx = write_transaction(self.conn)?;
        let source = UsageSource::ApiRetry.as_str();
        let mut taken = 0;
        for (from, to, kind, hit) in moves {
            let window = kind.as_str();
            let held: bool = tx.query_row(
                &format!(
                    "SELECT EXISTS (SELECT 1 FROM quota_samples q
                         WHERE q.account = ?5 AND {HIT_ROW})"
                ),
                params![window, hit.ts, hit.resets_at, source, from],
                |row| row.get(0),
            )?;
            if !held {
                continue;
            }
            let key = params![from, window, hit.ts];
            if let Some(to) = to {
                let put = tx.execute(
                    "INSERT OR IGNORE INTO quota_samples
                         (account, window, ts, used_pct, resets_at, confidence, last_seen)
                     SELECT ?4, window, ts, used_pct, resets_at, confidence, last_seen
                     FROM quota_samples WHERE account = ?1 AND window = ?2 AND ts = ?3",
                    params![from, window, hit.ts, to],
                )?;
                if put == 1 {
                    tx.execute(
                        "INSERT OR REPLACE INTO quota_sample_sources (account, window, ts, source)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![to, window, hit.ts, source],
                    )?;
                }
            }
            tx.execute(
                "DELETE FROM quota_sample_sources WHERE account = ?1 AND window = ?2 AND ts = ?3",
                key,
            )?;
            taken += tx.execute(
                "DELETE FROM quota_samples WHERE account = ?1 AND window = ?2 AND ts = ?3",
                key,
            )?;
        }
        tx.commit()?;
        Ok(taken)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Confidence;
    use crate::storage::Storage;

    fn hit(ts: i64) -> QuotaSample {
        QuotaSample {
            ts,
            used_pct: 100.0,
            resets_at: Some(ts + 3_600_000),
            confidence: Confidence::Measured,
            held_until: None,
        }
    }

    #[test]
    fn a_hit_goes_whole_to_the_account_it_happened_on_and_nothing_else_moves() {
        let storage = Storage::open_in_memory().unwrap();
        let usage = storage.usage();
        let five = WindowKind::FiveHour;
        // The hit, hit again a minute on (one held row), on `b`; another
        // reading at the same instant from elsewhere on `c`.
        let again = QuotaSample {
            ts: 61_000,
            ..hit(1_000)
        };
        usage
            .insert_sourced_samples(&[
                ("b".into(), five, hit(1_000), UsageSource::ApiRetry),
                ("b".into(), five, again, UsageSource::ApiRetry),
                ("c".into(), five, hit(1_000), UsageSource::Statusline),
            ])
            .unwrap();
        assert_eq!(usage.limit_hit_holders(five, &hit(1_000)).unwrap(), ["b"]);
        let given = [("b".to_owned(), Some("a".to_owned()), five, hit(1_000))];
        assert_eq!(usage.give_back_limit_hits(&given).unwrap(), 1);
        assert_eq!(usage.limit_hit_holders(five, &hit(1_000)).unwrap(), ["a"]);
        let on_a = usage.sourced_samples_for_account("a", five).unwrap();
        assert_eq!(
            on_a.iter()
                .map(|(sample, source)| (sample.ts, sample.held_until, *source))
                .collect::<Vec<_>>(),
            [
                (1_000, Some(61_000), Some(UsageSource::ApiRetry)),
                (61_000, None, Some(UsageSource::ApiRetry))
            ],
            "whole, the span it held too"
        );
        assert!(
            usage
                .sourced_samples_for_account("b", five)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            usage.sourced_samples_for_account("c", five).unwrap().len(),
            1
        );
        // Again: nothing is held wrongly, nothing moves.
        assert_eq!(usage.give_back_limit_hits(&given).unwrap(), 0);
        // A hit whose Account was removed is taken off, onto no one.
        usage
            .insert_sourced_samples(&[("b".into(), five, hit(9_000_000), UsageSource::ApiRetry)])
            .unwrap();
        let gone = [("b".to_owned(), None, five, hit(9_000_000))];
        assert_eq!(usage.give_back_limit_hits(&gone).unwrap(), 1);
        assert!(
            usage
                .limit_hit_holders(five, &hit(9_000_000))
                .unwrap()
                .is_empty()
        );
    }
}
