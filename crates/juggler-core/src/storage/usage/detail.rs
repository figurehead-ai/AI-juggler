//! Each stored call's detail and cost (0.3.7, the usage audit; split from
//! the usage store: the size budget): the derived `usage_detail` table is
//! written with every call stored, and priced again when the prices change.

use rusqlite::{Connection, OptionalExtension, params};

use super::{UsageRepo, as_sql, from_sql, write_transaction};
use crate::cost_ledger::PricingCatalog;
use crate::error::CoreError;
use crate::model::TokenCounts;
use crate::quota::{CallKind, UsageDetail, UsageEvent, UsageSource};

use super::parse_confidence;

impl UsageRepo<'_> {
    /// Whether every stored per-call cost was made with today's prices
    /// (the bundled table and the user's overrides).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn prices_current(&self) -> Result<bool, CoreError> {
        let catalog = PricingCatalog::with_overrides(self.pricing_overrides()?);
        let stored: Option<String> = self
            .conn
            .query_row(
                "SELECT version FROM usage_pricing WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        Ok(stored.as_deref() == Some(catalog.version().as_str()))
    }

    /// Prices stored calls again, up to `limit` of them in key order after
    /// `cursor` (`None`: from the start), with today's prices. A pass that
    /// reaches the end with the prices it started with records them, so
    /// [`Self::prices_current`] holds. A detail whose call is gone goes.
    /// Run in batches, so the store isn't held for long.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; the batch rolls
    /// back then.
    pub fn reprice(
        &self,
        cursor: Option<RepriceCursor>,
        limit: usize,
    ) -> Result<Option<RepriceCursor>, CoreError> {
        let tx = write_transaction(self.conn)?;
        let catalog = PricingCatalog::with_overrides(UsageRepo { conn: &tx }.pricing_overrides()?);
        let version = catalog.version();
        // Prices that changed mid-pass start the pass again.
        let after = cursor
            .filter(|cursor| cursor.version == version)
            .map(|cursor| cursor.after);
        let (after_account, after_key) = after.unwrap_or_default();
        let rows = {
            let mut stmt = tx.prepare_cached(
                "SELECT d.account, d.msg_uuid, d.cache_w_1h, d.prompt_tokens, d.speed, d.geo,
                        d.price_version, d.usd, e.model, e.ts, e.input, e.output, e.cache_r,
                        e.cache_w, e.reasoning, e.id
                 FROM usage_detail d
                 LEFT JOIN usage_events e ON e.account = d.account AND e.msg_uuid = d.msg_uuid
                 WHERE (d.account, d.msg_uuid) > (?1, ?2)
                 ORDER BY d.account, d.msg_uuid
                 LIMIT ?3",
            )?;
            stmt.query_map(
                params![
                    after_account,
                    after_key,
                    i64::try_from(limit).unwrap_or(i64::MAX)
                ],
                |row| {
                    let detail = UsageDetail {
                        cache_write_1h: from_sql(row.get(2)?),
                        prompt_tokens: from_sql(row.get(3)?),
                        speed: row.get(4)?,
                        geo: row.get(5)?,
                        ..UsageDetail::default()
                    };
                    let tokens = TokenCounts {
                        input: row.get::<_, Option<i64>>(10)?.map_or(0, from_sql),
                        output: row.get::<_, Option<i64>>(11)?.map_or(0, from_sql),
                        cache_read: row.get::<_, Option<i64>>(12)?.map_or(0, from_sql),
                        cache_write: row.get::<_, Option<i64>>(13)?.map_or(0, from_sql),
                        reasoning: row.get::<_, Option<i64>>(14)?.map_or(0, from_sql),
                    };
                    Ok((
                        (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                        detail,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<f64>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, Option<i64>>(9)?,
                        tokens,
                        row.get::<_, Option<i64>>(15)?.is_some(),
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?
        };
        let done = rows.len() < limit;
        let last = rows.last().map(|row| row.0.clone());
        {
            let mut update = tx.prepare_cached(
                "UPDATE usage_detail SET usd = ?3, price_version = ?4
                 WHERE account = ?1 AND msg_uuid = ?2",
            )?;
            let mut orphan =
                tx.prepare_cached("DELETE FROM usage_detail WHERE account = ?1 AND msg_uuid = ?2")?;
            for ((account, key), detail, priced_with, _, model, ts, tokens, has_event) in rows {
                if !has_event {
                    orphan.execute(params![account, key])?;
                    continue;
                }
                if priced_with.as_deref() == Some(version.as_str()) {
                    continue;
                }
                let now = model.as_deref().and_then(|model| {
                    catalog.price_event(model, ts.unwrap_or_default(), tokens, &detail)
                });
                update.execute(params![account, key, now, version])?;
            }
        }
        let next = if done {
            tx.execute(
                "INSERT OR REPLACE INTO usage_pricing (id, version) VALUES (1, ?1)",
                params![version],
            )?;
            None
        } else {
            last.map(|after| RepriceCursor { version, after })
        };
        tx.commit()?;
        Ok(next)
    }
}

/// Where a [`UsageRepo::reprice`] pass stands: the prices it is making
/// costs with, and the last call it priced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepriceCursor {
    version: String,
    after: (String, String),
}

/// Writes each stored call's [`UsageDetail`] and its cost (see
/// `usage_detail` in [`DERIVED`]).
pub(super) struct Details<'c> {
    catalog: PricingCatalog,
    version: String,
    find: rusqlite::CachedStatement<'c>,
    write: rusqlite::CachedStatement<'c>,
}

impl<'c> Details<'c> {
    pub(super) fn new(conn: &'c Connection) -> Result<Self, CoreError> {
        let catalog = PricingCatalog::with_overrides(UsageRepo { conn }.pricing_overrides()?);
        Ok(Details {
            version: catalog.version(),
            catalog,
            find: conn.prepare_cached(
                "SELECT cache_w_1h, prompt_tokens, speed, geo, kind, root_session, usd, price_version
                 FROM usage_detail WHERE account = ?1 AND msg_uuid = ?2",
            )?,
            write: conn.prepare_cached(
                "INSERT OR REPLACE INTO usage_detail
                     (account, msg_uuid, cache_w_1h, prompt_tokens, speed, geo, kind,
                      root_session, usd, price_version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?,
        })
    }

    /// Keeps `event`'s detail for its stored row, whose tokens are now
    /// `tokens`: merged with what is stored (each figure its largest, as
    /// the tokens are; a one-time kind or conversation is kept), and
    /// priced at the call's own tier.
    pub(super) fn keep(
        &mut self,
        event: &UsageEvent,
        tokens: TokenCounts,
        model: Option<&str>,
    ) -> Result<(), CoreError> {
        let Some(key) = &event.msg_uuid else {
            return Ok(());
        };
        let was = self
            .find
            .query_row(params![event.account, key], |row| {
                Ok((
                    UsageDetail {
                        cache_write_1h: from_sql(row.get(0)?),
                        prompt_tokens: from_sql(row.get(1)?),
                        speed: row.get(2)?,
                        geo: row.get(3)?,
                        kind: CallKind::parse(&row.get::<_, String>(4)?),
                        root_session: row.get(5)?,
                    },
                    row.get::<_, Option<f64>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .optional()?;
        let detail = match &was {
            Some((stored, _, _)) => UsageDetail {
                cache_write_1h: stored.cache_write_1h.max(event.detail.cache_write_1h),
                prompt_tokens: stored.prompt_tokens.max(event.detail.prompt_tokens),
                speed: stored.speed.clone().or_else(|| event.detail.speed.clone()),
                geo: stored.geo.clone().or_else(|| event.detail.geo.clone()),
                kind: stored.kind,
                root_session: stored
                    .root_session
                    .clone()
                    .or_else(|| event.detail.root_session.clone()),
            },
            None => event.detail.clone(),
        };
        let usd =
            model.and_then(|model| self.catalog.price_event(model, event.ts, tokens, &detail));
        if let Some((stored, stored_usd, version)) = &was
            && *stored == detail
            && *stored_usd == usd
            && version.as_deref() == Some(self.version.as_str())
        {
            return Ok(());
        }
        self.write.execute(params![
            event.account,
            key,
            as_sql(detail.cache_write_1h),
            as_sql(detail.prompt_tokens),
            detail.speed,
            detail.geo,
            detail.kind.as_str(),
            detail.root_session,
            usd,
            self.version,
        ])?;
        // A settled call's new cost marks its hour for the summaries (the
        // store's `usage_rollup_detail_*` triggers).
        Ok(())
    }
}

/// What an event read selects: its row and its detail (the stored
/// per-call cost, when it has one, as its `usd_est`). Followed by the
/// `WHERE` over `e`.
pub(in crate::storage) const EVENT_COLUMNS: &str =
    "e.account, e.session, e.ts, e.model, e.input, e.output,
        e.cache_r, e.cache_w, e.reasoning, e.credits, COALESCE(d.usd, e.usd_est), e.source,
        e.confidence, e.msg_uuid, d.cache_w_1h, d.prompt_tokens, d.speed, d.geo, d.kind,
        d.root_session
    FROM usage_events e
    LEFT JOIN usage_detail d ON d.account = e.account AND d.msg_uuid = e.msg_uuid";

/// One event as [`EVENT_COLUMNS`] reads it.
pub(in crate::storage) fn event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UsageEvent> {
    Ok(UsageEvent {
        account: row.get(0)?,
        session: row.get(1)?,
        ts: row.get(2)?,
        model: row.get(3)?,
        tokens: TokenCounts {
            input: from_sql(row.get(4)?),
            output: from_sql(row.get(5)?),
            cache_read: from_sql(row.get(6)?),
            cache_write: from_sql(row.get(7)?),
            reasoning: from_sql(row.get(8)?),
        },
        credits: row.get(9)?,
        usd_est: row.get(10)?,
        source: UsageSource::parse(&row.get::<_, String>(11)?).unwrap_or(UsageSource::Transcript),
        confidence: parse_confidence(&row.get::<_, String>(12)?),
        msg_uuid: row.get(13)?,
        detail: UsageDetail {
            cache_write_1h: row.get::<_, Option<i64>>(14)?.map_or(0, from_sql),
            prompt_tokens: row.get::<_, Option<i64>>(15)?.map_or(0, from_sql),
            speed: row.get(16)?,
            geo: row.get(17)?,
            kind: row
                .get::<_, Option<String>>(18)?
                .map_or(CallKind::Main, |kind| CallKind::parse(&kind)),
            root_session: row.get(19)?,
        },
    })
}
