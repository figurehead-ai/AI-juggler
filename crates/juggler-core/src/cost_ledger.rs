//! Cost ledger (AJ-E609, deep/09 §Q7, deep/10 §6): prices every
//! `usage_event` per model into an auditable USD ledger — versioned
//! pricing with event-time rate selection, per-session/account/
//! project/day rollups, a streaming CSV writer, and the dual
//! subscription view. Label discipline IS the feature:
//!
//! - USD is ALWAYS `Estimate` — except a future vendor
//!   `cost.total_cost_usd` passthrough (labelled `vendor-reported`;
//!   no passthrough is ingested until the statusline capture verifies
//!   live — AJ-E602's candidate).
//! - An unpriced model yields USD `None` (rendered `—`), never
//!   `$0.00`.
//! - `ZCode` credits display ALONGSIDE USD and are never converted at
//!   an invented rate — separate column, always.
//! - USD is joined at QUERY time (repricing is retroactive; the
//!   stored events never change), with event-time rate selection
//!   (`effective_from ≤ ts`, day-bucketed — the reading deep/10 line
//!   75 and deep/09 §Q7 share, recorded in the task's PR).

use crate::model::{Confidence, TokenCounts};
use crate::quota::UsageEvent;

/// The bundled, versioned seed (`cost_ledger_seed.json`, v2 — adds
/// cache rates, `effective_from`, and per-row `source_url` over the
/// AJ-E409 v1 table). Malformed ⇒ build-time bug the tests catch;
/// never a silent fallback.
pub const PRICING_SEED: &str = include_str!("cost_ledger_seed.json");

/// The seed's deserialized shape.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SeedFile {
    /// Table version (`v2`).
    pub version: String,
    /// Capture date of the newest row (YYYY-MM-DD).
    pub captured: String,
    /// The rows.
    pub rows: Vec<LedgerPriceRow>,
}

/// Loads the bundled seed as a catalog's `bundled` rows.
///
/// # Panics
///
/// Panics when the embedded seed is malformed — an in-tree,
/// test-validated file; there is no silent fallback.
#[allow(clippy::missing_panics_doc, clippy::panic)]
#[must_use]
pub fn load_seed_rows() -> Vec<LedgerPriceRow> {
    let seed: SeedFile = serde_json::from_str(PRICING_SEED)
        .unwrap_or_else(|error| panic!("embedded pricing seed is malformed: {error}"));
    seed.rows
}

/// USD per 1k tokens, per class, for one model from an instant on.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct LedgerPriceRow {
    /// Model identifier exactly as usage events carry it.
    pub model: String,
    /// Effective from (epoch ms); rows before this instant do not
    /// apply. `0` = always effective (the bundled default).
    pub effective_from_ms: i64,
    /// USD per 1k input tokens.
    pub input_per_1k: f64,
    /// USD per 1k output tokens.
    pub output_per_1k: f64,
    /// USD per 1k cache-read tokens.
    #[serde(default)]
    pub cache_read_per_1k: f64,
    /// USD per 1k cache-write tokens.
    #[serde(default)]
    pub cache_write_per_1k: f64,
    /// Where the numbers came from (auditable pricing law).
    pub source_url: String,
    /// When the numbers were captured (YYYY-MM-DD).
    pub captured: String,
}

/// The priced USD view of one aggregated bucket.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct UsdEstimate {
    /// The USD number, when every model row in the bucket priced.
    /// `None` ⇒ render `—` (never `$0.00`).
    pub usd: Option<f64>,
    /// Always `Estimate` (list pricing — the permanent badge copy:
    /// "Estimated — list pricing, not your subscription fee").
    pub label: Confidence,
}

/// One rollup bucket: tokens by class + USD estimate + credits +
/// confidence, keyed by the caller's dimension (the single rollup
/// shape behind the UI matrix AND the CSV export — reconciliation to
/// the cent is structural).
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct CostRollup {
    /// First dimension key (session id, account id, project id, or
    /// `ALL` for account-wide rows).
    pub key: String,
    /// Model attribution — survives mid-session model switches.
    pub model: Option<String>,
    /// The day bucket (UTC YYYY-MM-DD), for day rollups.
    pub day: Option<String>,
    /// Tokens by class.
    pub tokens: TokenCounts,
    /// Priced USD (list rates, event-time selected).
    pub usd: UsdEstimate,
    /// Vendor credits (`ZCode`), never converted to USD.
    pub credits: Option<f64>,
    /// Lowest confidence among the folded events.
    pub confidence: Confidence,
}

/// The catalog: bundled rows (versioned, ship with the app) plus
/// user overrides (same shape; a user row supersedes every bundled
/// row for the same model FROM its effective instant).
#[derive(Clone, PartialEq, Debug, Default, serde::Serialize)]
pub struct PricingCatalog {
    /// Bundled, versioned rows.
    pub bundled: Vec<LedgerPriceRow>,
    /// User-entered overrides.
    pub overrides: Vec<LedgerPriceRow>,
    /// `last synced` metadata (epoch ms) for the bundled set.
    pub last_synced_ms: Option<i64>,
}

/// Stale-pricing threshold: a bundled table older than 30 days warns
/// inline (deep/10 §6) rather than silently repricing history.
pub const STALE_PRICING_MS: i64 = 30 * 24 * 60 * 60 * 1000;

/// Day bucket for an epoch-ms timestamp (UTC `YYYY-MM-DD`) — the
/// event-time rate-selection granularity.
#[must_use]
pub fn day_of(ts_ms: i64) -> String {
    let days = ts_ms.div_euclid(86_400_000);
    // civil_from_days (Hinnant) — day granularity only.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!("{year:04}-{m:02}-{d:02}")
}

impl PricingCatalog {
    /// The rate row for `model` at `ts`: the latest row
    /// (`effective_from ≤ ts`) among overrides (preferred) and
    /// bundled rows; `None` when unpriced at that instant.
    #[must_use]
    pub fn row_for(&self, model: &str, ts_ms: i64) -> Option<&LedgerPriceRow> {
        fn best<'a>(
            rows: &'a [LedgerPriceRow],
            model: &str,
            ts_ms: i64,
        ) -> Option<&'a LedgerPriceRow> {
            rows.iter()
                .filter(|row| row.model == model && row.effective_from_ms <= ts_ms)
                .max_by_key(|row| row.effective_from_ms)
        }
        best(&self.overrides, model, ts_ms).or_else(|| best(&self.bundled, model, ts_ms))
    }

    /// True when the bundled table's newest capture is older than
    /// [`STALE_PRICING_MS`] (the inline-warning state).
    #[must_use]
    pub fn is_stale(&self, now_ms: i64) -> bool {
        let Some(synced) = self.last_synced_ms else {
            return false; // no metadata ⇒ cannot claim staleness
        };
        now_ms.saturating_sub(synced) > STALE_PRICING_MS
    }

    /// Prices one event's tokens at event-time rates. `None` when the
    /// model is unpriced AT THAT INSTANT (the honest `—`). The u64→f64
    /// casts lose cents only past 2^53 tokens — beyond any real bucket.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn price_tokens(&self, model: &str, ts_ms: i64, tokens: TokenCounts) -> Option<f64> {
        let row = self.row_for(model, ts_ms)?;
        Some(
            (tokens.input as f64 / 1000.0) * row.input_per_1k
                + (tokens.output as f64 / 1000.0) * row.output_per_1k
                + (tokens.cache_read as f64 / 1000.0) * row.cache_read_per_1k
                + (tokens.cache_write as f64 / 1000.0) * row.cache_write_per_1k,
        )
    }
}

/// Groups events into rollup buckets. `dimension` picks the key:
/// session id, account id, `ALL`, or a caller-supplied project map
/// (session → project). Day buckets apply event-time rate selection
/// per event BEFORE summing, so a rate change mid-window reprices
/// exactly the events after it.
#[must_use]
pub fn rollup(
    events: &[UsageEvent],
    catalog: &PricingCatalog,
    dimension: RollupDimension<'_>,
) -> Vec<CostRollup> {
    /// One working bucket keyed by (key, model, day).
    struct Bucket {
        tokens: TokenCounts,
        usd: Option<f64>,
        credits: f64,
        has_credits: bool,
        confidence: Option<Confidence>,
        unpriced: bool,
    }
    let mut buckets: std::collections::BTreeMap<(String, Option<String>, String), Bucket> =
        std::collections::BTreeMap::new();
    for event in events {
        let key = match dimension {
            RollupDimension::Account => event.account.clone(),
            RollupDimension::Session => event.session.clone().unwrap_or_default(),
            RollupDimension::Project(map) => event
                .session
                .as_deref()
                .and_then(|session| map.get(session))
                .cloned()
                .unwrap_or_else(|| event.session.clone().unwrap_or_default()),
            RollupDimension::All => "ALL".to_owned(),
        };
        let day = day_of(event.ts);
        let entry = buckets
            .entry((key, event.model.clone(), day))
            .or_insert(Bucket {
                tokens: TokenCounts::default(),
                usd: Some(0.0),
                credits: 0.0,
                has_credits: false,
                confidence: None,
                unpriced: false,
            });
        entry.tokens.input = entry.tokens.input.saturating_add(event.tokens.input);
        entry.tokens.output = entry.tokens.output.saturating_add(event.tokens.output);
        entry.tokens.cache_read = entry
            .tokens
            .cache_read
            .saturating_add(event.tokens.cache_read);
        entry.tokens.cache_write = entry
            .tokens
            .cache_write
            .saturating_add(event.tokens.cache_write);
        entry.tokens.reasoning = entry
            .tokens
            .reasoning
            .saturating_add(event.tokens.reasoning);
        // Event-time pricing: unpriced at THIS instant ⇒ the bucket's
        // USD becomes None (— never a partial $).
        let priced = event
            .model
            .as_deref()
            .and_then(|model| catalog.price_tokens(model, event.ts, event.tokens));
        if let (Some(usd), Some(total)) = (priced, entry.usd) {
            entry.usd = Some(total + usd);
        } else {
            entry.usd = None;
        }
        if let Some(credits) = event.credits {
            entry.credits += credits;
            entry.has_credits = true;
        }
        entry.confidence = Some(entry.confidence.map_or(event.confidence, |existing| {
            if rank(event.confidence) < rank(existing) {
                event.confidence
            } else {
                existing
            }
        }));
        let _ = entry.unpriced;
    }
    buckets
        .into_iter()
        .map(|((key, model, day), bucket)| CostRollup {
            key,
            model,
            day: Some(day),
            tokens: bucket.tokens,
            usd: UsdEstimate {
                usd: bucket.usd,
                label: Confidence::Estimated,
            },
            credits: bucket.has_credits.then_some(bucket.credits + 0.0),
            confidence: bucket.confidence.unwrap_or_default(),
        })
        .collect()
}

/// Trust ranking for confidence inheritance (mirrors the forecast's).
#[must_use]
fn rank(confidence: Confidence) -> u8 {
    match confidence {
        Confidence::Estimated => 0,
        Confidence::Computed => 1,
        Confidence::Measured => 2,
    }
}

/// Which dimension a rollup groups by.
#[derive(Clone, Copy)]
pub enum RollupDimension<'a> {
    /// Per account.
    Account,
    /// Per vendor session id.
    Session,
    /// Per project (a session→project map; unmapped sessions key by
    /// their own id — never a fabricated project).
    Project(&'a std::collections::BTreeMap<String, String>),
    /// One bucket for everything (the grand total row).
    All,
}

/// The dual subscription view: API-equivalent value vs the
/// user-entered flat fee. The ratio exists ONLY when a fee exists —
/// no fee ⇒ no ratio, no guessed fee (Req.4).
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct SubscriptionView {
    /// "≈$X at API prices" (the ledger's Estimate-labelled sum).
    pub api_equivalent_usd: Option<f64>,
    /// The user-entered flat fee (AJ-E610's account form owns entry).
    pub flat_fee_usd: Option<f64>,
    /// `api_equivalent ÷ fee`, only when BOTH exist.
    pub value_ratio: Option<f64>,
}

/// Builds the dual view from a USD total and an optional fee.
#[must_use]
pub fn subscription_view(
    api_equivalent_usd: Option<f64>,
    flat_fee_usd: Option<f64>,
) -> SubscriptionView {
    SubscriptionView {
        api_equivalent_usd,
        flat_fee_usd,
        value_ratio: match (api_equivalent_usd, flat_fee_usd) {
            (Some(value), Some(fee)) if fee > 0.0 => Some(value / fee),
            _ => None,
        },
    }
}

/// Streams rollups as CSV to `out` — no in-memory materialisation
/// beyond the caller's rows (1M-event exports stay ≤ the memory
/// budget because the rows arrive day-bucketed and chunked). The
/// header set is the UI matrix's columns, verbatim — export-vs-screen
/// reconciliation is exact by construction.
///
/// # Errors
///
/// Returns the writer's error type on I/O failure.
pub fn write_csv<W: std::io::Write>(rows: &[CostRollup], out: &mut W) -> std::io::Result<()> {
    writeln!(
        out,
        "key,model,day,input,output,cache_read,cache_write,reasoning,usd_estimate,usd_label,credits,confidence"
    )?;
    for row in rows {
        writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            csv_field(&row.key),
            csv_field(row.model.as_deref().unwrap_or("unknown")),
            csv_field(row.day.as_deref().unwrap_or("")),
            row.tokens.input,
            row.tokens.output,
            row.tokens.cache_read,
            row.tokens.cache_write,
            row.tokens.reasoning,
            match row.usd.usd {
                Some(usd) => format!("{usd:.2}"),
                None => "—".to_owned(),
            },
            USD_LABEL, // the permanent USD label (deep/10 §6 copy)
            row.credits
                .map(|credits| format!("{credits:.4}"))
                .unwrap_or_default(),
            match row.confidence {
                Confidence::Measured => "Measured",
                Confidence::Computed => "Computed",
                Confidence::Estimated => "Estimate",
            },
        )?;
    }
    Ok(())
}

/// The permanent USD label (deep/10 §6): USD is always an Estimate of
/// list pricing, never the subscription fee.
pub const USD_LABEL: &str = "Estimate";

/// Minimal CSV field quoting (quote only when needed; doubles inside).
fn csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}
