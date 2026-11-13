//! AJ-E2016: `ZCode`'s usage, as credits (spec §6, Req.4 passive), moved out
//! of `quota_ingest.rs`: the published per-model multipliers, the peak
//! window, one row's credits, and the read of its `model_usage` table.

use std::path::Path;

use juggler_core::model::{Confidence, TokenCounts};
use juggler_core::quota::{UsageEvent, UsageSource};

// ---------------------------------------------------------------------------
// Source: ZCode SQLite model_usage (Computed credits) — Req.4 passive
// ---------------------------------------------------------------------------

/// Published per-model credit multipliers (in, cached, out) —
/// docs.z.ai/devpack/overview revised 2026-07-30, graded in
/// `zcode-captures.md`. Unknown models carry NO credits (honest
/// absence) and mark the event Estimated.
#[must_use]
pub fn zcode_multiplier(model: &str) -> Option<(f64, f64, f64)> {
    match model {
        "GLM-5.3" => Some((6.9, 1.7, 24.0)),
        "GLM-5.3-Flash" => Some((2.3, 0.56, 8.0)),
        _ => None,
    }
}

/// True inside a PEAK window: weekdays 14:00–18:00 UTC+8 (off-peak
/// bills at 50%). Pure integer calendar maths from epoch ms.
#[must_use]
pub fn zcode_is_peak(utc_ms: i64) -> bool {
    let days = utc_ms.div_euclid(86_400_000);
    let day_ms = utc_ms.rem_euclid(86_400_000);
    let hour_utc8 = day_ms / 3_600_000 + 8;
    // 1970-01-01 was a Thursday; Monday=0 ⇒ offset 3.
    let weekday = (days + 3).rem_euclid(7);
    weekday < 5 && (14..18).contains(&hour_utc8)
}

/// Computes one row's credits:
/// `(input×in + cached×cached + output×out) / 10000`, half off-peak.
/// Bucket mapping per the E604 packet (graded `inferred`): the row's
/// `input_tokens` counts ALL prompt tokens, so fresh = input −
/// `cache_read` − `cache_creation`; cached = `cache_read` + `cache_creation`.
#[must_use]
pub fn zcode_credits(
    model: &str,
    input: u64,
    cache_read: u64,
    cache_creation: u64,
    output: u64,
    utc_ms: i64,
) -> Option<f64> {
    let (in_mult, cached_mult, out_mult) = zcode_multiplier(model)?;
    #[allow(
        clippy::cast_precision_loss,
        reason = "token counts far below 2^52, where f64 is exact"
    )]
    let fresh = input
        .saturating_sub(cache_read)
        .saturating_sub(cache_creation) as f64;
    #[allow(
        clippy::cast_precision_loss,
        reason = "token counts far below 2^52, where f64 is exact"
    )]
    let cached = cache_read.saturating_add(cache_creation) as f64;
    #[allow(
        clippy::cast_precision_loss,
        reason = "token counts far below 2^52, where f64 is exact"
    )]
    let out = output as f64;
    let raw = (fresh * in_mult + cached * cached_mult + out * out_mult) / 10_000.0;
    Some(if zcode_is_peak(utc_ms) {
        raw
    } else {
        raw * 0.5
    })
}

/// Reads the WHOLE `model_usage` table (completed rows only) from the
/// vendor DB, read-only and never written beside (`open_tool_store`:
/// the log joined while `ZCode` runs, `immutable` when it is closed, where
/// a plain read-only open failed). Full-table each pass is
/// the deliberate choice: rows carry no vendor uuid, so the stable
/// `rowid` is the dedupe key and completeness — not a high-water mark
/// — keeps ids honest. A day is ~1k rows; the read is <10 ms.
///
/// # Errors
///
/// Returns a message when the DB cannot be opened or the table is
/// missing (never a partial/invented read).
pub fn zcode_events(account: &str, db_path: &Path) -> Result<Vec<UsageEvent>, String> {
    let conn = crate::common::sqlite::open_tool_store(db_path, "ZCode")?;
    let mut stmt = conn
        .prepare(
            "SELECT rowid, model_id, input_tokens, output_tokens,
                    cache_creation_input_tokens, cache_read_input_tokens, started_at
             FROM model_usage WHERE status = 'completed'",
        )
        .map_err(|error| format!("zcode model_usage query: {error}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(|error| format!("zcode model_usage rows: {error}"))?;
    let mut events = Vec::new();
    for row in rows {
        let (rowid, model, input, output, cache_creation, cache_read, started_at) =
            row.map_err(|error| format!("zcode row: {error}"))?;
        #[allow(
            clippy::cast_sign_loss,
            reason = "clamped at 0 first: a negative count reads as none"
        )]
        let (input, output, cache_creation, cache_read) = (
            input.max(0) as u64,
            output.max(0) as u64,
            cache_creation.max(0) as u64,
            cache_read.max(0) as u64,
        );
        let credits = zcode_credits(
            &model,
            input,
            cache_read,
            cache_creation,
            output,
            started_at,
        );
        events.push(UsageEvent {
            account: account.to_owned(),
            session: None,
            ts: started_at,
            model: Some(model),
            tokens: TokenCounts {
                input,
                output,
                cache_read,
                cache_write: cache_creation,
                reasoning: 0,
            },
            credits,
            usd_est: None,
            source: UsageSource::Sqlite,
            confidence: if credits.is_some() {
                Confidence::Computed
            } else {
                Confidence::Estimated
            },
            detail: juggler_core::quota::UsageDetail::default(),
            msg_uuid: Some(format!("zcode:{rowid}")),
        });
    }
    Ok(events)
}
