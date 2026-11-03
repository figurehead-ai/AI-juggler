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
//! - USD is priced per API call (0.3.7, the usage audit): a vendor's
//!   price depends on each call's prompt size (long-context tiers), its
//!   1-hour cache writes and its speed, none of which a sum keeps. The
//!   store prices a call when it is read and keeps the cost beside it
//!   with the prices' version ([`PricingCatalog::version`]); a new table
//!   or a user override prices every stored call again, so repricing
//!   stays retroactive and the token rows never change. Rates are
//!   event-time selected (`effective_from ≤ ts` — the reading deep/10
//!   line 75 and deep/09 §Q7 share).

use crate::model::{Confidence, TokenCounts};
use crate::quota::{UsageDetail, UsageEvent};

/// The bundled, versioned seed (`cost_ledger_seed.json`; v2 added
/// cache rates, `effective_from`, and per-row `source_url` over the
/// AJ-E409 v1 table; v3 prices the current Claude and GPT models from
/// the vendors' pages — `docs/research/model-pricing.md`). Malformed ⇒ build-time bug the tests catch;
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
    seed().rows.clone()
}

/// The embedded seed, parsed once (the store prices every call it reads
/// with it).
#[allow(clippy::panic)]
fn seed() -> &'static SeedFile {
    static SEED: std::sync::OnceLock<SeedFile> = std::sync::OnceLock::new();
    SEED.get_or_init(|| {
        serde_json::from_str(PRICING_SEED)
            .unwrap_or_else(|error| panic!("embedded pricing seed is malformed: {error}"))
    })
}

/// When the bundled table's numbers were captured: the seed's `captured`
/// day, at midnight UTC (epoch ms). `None` only for a seed without a
/// readable date, which the tests rule out. The pricing warning reads it:
/// a build's prices age with the build, and no price sync exists.
#[must_use]
pub fn seed_captured_ms() -> Option<i64> {
    let seed: SeedFile = serde_json::from_str(PRICING_SEED).ok()?;
    let day = crate::renewal::CalendarDate::parse(&seed.captured)?;
    Some(day.civil_days() * 86_400_000)
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
    /// USD per 1k cache-write tokens (the 5-minute cache, where a vendor
    /// has two).
    #[serde(default)]
    pub cache_write_per_1k: f64,
    /// USD per 1k tokens written to the 1-hour cache (Anthropic: 2× base
    /// input, against 1.25× for the 5-minute one). `None`: priced as
    /// [`Self::cache_write_per_1k`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_1h_per_1k: Option<f64>,
    /// The higher rates a call pays when its prompt is longer than a
    /// threshold (GPT models over 272K input tokens; Claude Haiku 5.5 over
    /// 100,000). Each call is priced on its own prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_context: Option<LongContext>,
    /// The rates of the vendor's fast mode, for the whole prompt
    /// (Anthropic's fast mode on some Opus models).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fast: Option<TierRates>,
    /// What US-only inference multiplies every rate by (Anthropic: 1.1
    /// on Claude 4.6 and later). `None`: no such option.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_multiplier: Option<f64>,
    /// Where the numbers came from (auditable pricing law).
    pub source_url: String,
    /// When the numbers were captured (YYYY-MM-DD).
    pub captured: String,
}

/// One set of rates, USD per 1k tokens per class: a price tier.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct TierRates {
    /// Input.
    pub input_per_1k: f64,
    /// Output (reasoning included).
    pub output_per_1k: f64,
    /// Cache reads.
    #[serde(default)]
    pub cache_read_per_1k: f64,
    /// Cache writes (5-minute).
    #[serde(default)]
    pub cache_write_per_1k: f64,
    /// Cache writes (1-hour); `None`: as 5-minute ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_1h_per_1k: Option<f64>,
}

/// A long-context tier: the rates a call pays when its prompt (input,
/// cache reads and cache writes together) is over `over_prompt_tokens`.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct LongContext {
    /// The threshold: a prompt of more tokens than this pays these rates.
    pub over_prompt_tokens: u64,
    /// The rates.
    #[serde(flatten)]
    pub rates: TierRates,
}

impl LedgerPriceRow {
    /// The row's standard rates, as a tier.
    #[must_use]
    pub fn standard(&self) -> TierRates {
        TierRates {
            input_per_1k: self.input_per_1k,
            output_per_1k: self.output_per_1k,
            cache_read_per_1k: self.cache_read_per_1k,
            cache_write_per_1k: self.cache_write_per_1k,
            cache_write_1h_per_1k: self.cache_write_1h_per_1k,
        }
    }

    /// The tier one call pays: fast mode's when the call ran fast and the
    /// model has it (it covers the whole context), else the long-context
    /// tier's when the prompt is over its threshold, else the standard one.
    #[must_use]
    pub fn tier_for(&self, prompt_tokens: u64, detail: &UsageDetail) -> TierRates {
        if detail.speed.as_deref() == Some("fast")
            && let Some(fast) = &self.fast
        {
            return fast.clone();
        }
        match &self.long_context {
            Some(long) if prompt_tokens > long.over_prompt_tokens => long.rates.clone(),
            _ => self.standard(),
        }
    }
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

/// The bundled table's version (`v4`), as the seed names it.
///
/// # Panics
///
/// Panics when the embedded seed is malformed (as [`load_seed_rows`]).
#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn seed_version() -> String {
    seed().version.clone()
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

/// `claude-haiku-4-5-20251001` → `claude-haiku-4-5`: a model id without
/// its trailing `-YYYYMMDD` snapshot date; `None` when it has none.
fn undated(model: &str) -> Option<&str> {
    let (alias, date) = model.rsplit_once('-')?;
    (date.len() == 8 && date.bytes().all(|byte| byte.is_ascii_digit())).then_some(alias)
}

impl PricingCatalog {
    /// The bundled table with the user's `overrides` (no staleness
    /// metadata).
    #[must_use]
    pub fn with_overrides(overrides: Vec<LedgerPriceRow>) -> PricingCatalog {
        PricingCatalog {
            bundled: load_seed_rows(),
            overrides,
            last_synced_ms: None,
        }
    }

    /// Which prices a stored cost was made with: the bundled table's
    /// version, and a fingerprint of the user's overrides when there are
    /// any (`v4`, `v4+u1a2b3c4d5e6f7a8b`). A stored cost made with another
    /// is priced again ([`crate::storage::UsageRepo::reprice`]).
    #[must_use]
    pub fn version(&self) -> String {
        let seed = seed_version();
        if self.overrides.is_empty() {
            return seed;
        }
        // FNV-1a over the overrides' JSON: stable across builds and runs.
        let text = serde_json::to_string(&self.overrides).unwrap_or_default();
        let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
        format!("{seed}+u{hash:016x}")
    }

    /// The rate row for `model` at `ts`: the latest row
    /// (`effective_from ≤ ts`) among overrides (preferred) and
    /// bundled rows; `None` when unpriced at that instant. A dated
    /// snapshot id (`claude-haiku-4-5-20251001`) falls back to its
    /// dateless alias's row when it has none of its own.
    #[must_use]
    pub fn row_for(&self, model: &str, ts_ms: i64) -> Option<&LedgerPriceRow> {
        self.row_exact(model, ts_ms)
            .or_else(|| self.row_exact(undated(model)?, ts_ms))
    }

    fn row_exact(&self, model: &str, ts_ms: i64) -> Option<&LedgerPriceRow> {
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

    /// Prices tokens at event-time standard rates, as one call with no
    /// detail would be (a projection's or a summed bucket's price: no
    /// tier can be told from a sum). `None` when the model is unpriced AT
    /// THAT INSTANT (the honest `—`). Stored usage is priced per call
    /// instead ([`Self::price_event`]).
    #[must_use]
    pub fn price_tokens(&self, model: &str, ts_ms: i64, tokens: TokenCounts) -> Option<f64> {
        let row = self.row_for(model, ts_ms)?;
        Some(cost_at(&row.standard(), tokens, 0, 1.0))
    }

    /// Prices ONE API call at its event-time list rates and its own tier
    /// (0.3.7, the usage audit): the long-context rates when its prompt is
    /// over the model's threshold, fast mode's when it ran fast, 1-hour
    /// cache writes at their own rate, and US-only inference's multiplier.
    /// Reasoning tokens are output the vendor bills at the output rate
    /// (stored apart from `output`, never inside it). `None` when the
    /// model is unpriced at that instant.
    #[must_use]
    pub fn price_event(
        &self,
        model: &str,
        ts_ms: i64,
        tokens: TokenCounts,
        detail: &UsageDetail,
    ) -> Option<f64> {
        let row = self.row_for(model, ts_ms)?;
        let tier = row.tier_for(detail.prompt_of(&tokens), detail);
        let geo = match (detail.geo.as_deref(), row.us_multiplier) {
            (Some("us"), Some(multiplier)) => multiplier,
            _ => 1.0,
        };
        Some(cost_at(&tier, tokens, detail.cache_write_1h, geo))
    }
}

/// `tokens` at `rates`, `one_hour` of the cache writes at the 1-hour
/// rate, all times `multiplier`. The u64→f64 casts lose cents only past
/// 2^53 tokens — beyond any real bucket.
#[allow(clippy::cast_precision_loss)]
fn cost_at(rates: &TierRates, tokens: TokenCounts, one_hour: u64, multiplier: f64) -> f64 {
    let per_1k = |count: u64, rate: f64| (count as f64 / 1000.0) * rate;
    let one_hour = one_hour.min(tokens.cache_write);
    let five_minute = tokens.cache_write - one_hour;
    (per_1k(tokens.input, rates.input_per_1k)
        + per_1k(
            tokens.output.saturating_add(tokens.reasoning),
            rates.output_per_1k,
        )
        + per_1k(tokens.cache_read, rates.cache_read_per_1k)
        + per_1k(five_minute, rates.cache_write_per_1k)
        + per_1k(
            one_hour,
            rates
                .cache_write_1h_per_1k
                .unwrap_or(rates.cache_write_per_1k),
        ))
        * multiplier
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
        // Event-time pricing, per call: the cost stored when the call was
        // read (its own tier), else priced here. Unpriced at THIS instant ⇒
        // the bucket's USD becomes None (— never a partial $).
        let priced = event.usd_est.or_else(|| {
            event
                .model
                .as_deref()
                .and_then(|model| catalog.price_event(model, event.ts, event.tokens, &event.detail))
        });
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
