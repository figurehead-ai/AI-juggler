//! The in-repo pricing table (AJ-E409): DATA, not code constants — a
//! versioned JSON file with per-model USD per 1k tokens plus source and
//! date. Every derived cost is labelled `Est · value-equiv.`; models
//! without an entry render tokens only — never `$0.00`.

use serde::Deserialize;

/// One model's pricing row.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PriceRow {
    /// Provider id (`claude-code`, `zcode`, …).
    pub provider: String,
    /// Model identifier as it appears in usage events.
    pub model: String,
    /// USD per 1k input tokens.
    pub input_per_1k: f64,
    /// USD per 1k output tokens.
    pub output_per_1k: f64,
}

/// The versioned table (v1). Bump `PRICING_VERSION` + add a CHANGELOG
/// line when rows change (task gate).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PricingTable {
    /// Table version (`v1`).
    pub version: String,
    /// Source URL the numbers were taken from.
    pub source: String,
    /// Date the numbers were captured (YYYY-MM-DD).
    pub captured: String,
    /// Rows, keyed lookup by `provider|model`.
    pub rows: Vec<PriceRow>,
}

/// The compiled-in table (`include_str` — data file, single source).
pub const PRICING_DATA: &str = include_str!("pricing.v1.json");
/// Table version constant — changelog line required on change.
pub const PRICING_VERSION: &str = "v1";

/// Loads the built-in table; a malformed file is a build-time bug the
/// tests catch — this never falls back silently.
///
/// # Panics
///
/// Panics when the embedded `pricing.v1.json` fails to parse. The file
/// ships in-tree and is validated by the test suite, so a panic here is
/// a build-time bug, not a runtime condition — there is no silent
/// fallback.
#[allow(clippy::panic)] // names the malformed embedded table instead of a silent fallback
#[must_use]
pub fn load_pricing_table() -> PricingTable {
    serde_json::from_str(PRICING_DATA)
        .unwrap_or_else(|error| panic!("embedded pricing table is malformed: {error}"))
}

impl PricingTable {
    /// O(1)-ish lookup by provider + model (linear over a tiny table;
    /// the per-model lookup budget holds by construction).
    #[must_use]
    pub fn price(&self, provider: &str, model: &str) -> Option<&PriceRow> {
        self.rows
            .iter()
            .find(|row| row.provider == provider && row.model == model)
    }
}

/// Per-turn cost in USD (AJ-E412 Req.4): prices each event by the
/// model OBSERVED on that turn; turns without a recorded model are
/// counted by the caller but never priced. Returns the summed USD and
/// the distinct priced model identifiers — `None` cost when no turn
/// carried a priceable model (the honest `tokens only` state).
#[must_use]
pub fn session_cost_per_turn(
    table: &PricingTable,
    provider: &str,
    events: &[crate::quality::UsageEvent],
) -> Option<(f64, Vec<String>)> {
    let mut total = 0.0_f64;
    let mut models: Vec<String> = Vec::new();
    let mut any_priced = false;
    for event in events {
        let Some(model) = event.model.as_deref() else {
            continue; // `unknown model`: tokens counted, excluded from $
        };
        let Some(cost) = session_cost(table, provider, model, event.tokens) else {
            continue; // no pricing row: excluded from $ (never $0.00)
        };
        any_priced = true;
        total += cost;
        if !models.iter().any(|entry| entry == model) {
            models.push(model.to_string());
        }
    }
    any_priced.then_some((total, models))
}

/// Session cost in USD for the given token counts; `None` when the
/// model has no pricing row (callers render tokens only).
// Token counts are u64; crossing into f64 for USD math loses precision
// only past 2^53 tokens — far beyond any real session.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn session_cost(
    table: &PricingTable,
    provider: &str,
    model: &str,
    tokens: crate::model::TokenCounts,
) -> Option<f64> {
    let row = table.price(provider, model)?;
    Some(
        (tokens.input as f64 / 1000.0) * row.input_per_1k
            + (tokens.output as f64 / 1000.0) * row.output_per_1k,
    )
}
