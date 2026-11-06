//! The cost types on the wire (exported to TypeScript).

/// AJ-E609: the account's cost rollups (session/account/project/day
/// dimensions share one shape — the UI matrix and the CSV export read
/// the SAME rows, so reconciliation is exact) plus the dual
/// subscription view and the pricing-staleness flag.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CostRollupsInfo {
    /// Day-bucketed rollup rows (`key` = session id in the session
    /// dimension).
    pub rows: Vec<juggler_core::cost_ledger::CostRollup>,
    /// "≈$X at API prices · you pay flat" — ratio only when a fee
    /// exists (AJ-E610's form owns entry).
    pub subscription: juggler_core::cost_ledger::SubscriptionView,
    /// The bundled pricing table is >30 d old (inline warn).
    pub pricing_stale: bool,
    /// The bundled table's `last synced`, epoch ms.
    #[ts(type = "number | null")]
    pub pricing_last_synced_ms: Option<i64>,
    /// The scope's USD at list prices: every model that has a list price,
    /// summed; `None` when none has one. Models without one are in
    /// `unpriced_models`, never counted as $0.
    pub total_usd: Option<f64>,
    /// The models the scope used that have no list price, by name
    /// (`None` for usage whose model wasn't recorded), sorted.
    pub unpriced_models: Vec<Option<String>>,
    /// The scope the query ran over (account id or `ALL`).
    pub scope: String,
}

/// One row of the cost table (0.1.8): one session's UTC day, its models
/// folded together and named as the rest of the app names a session.
/// Each model's own part rides along for the row's breakdown.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CostRowInfo {
    /// The tool's id for the session (`""` when the usage names none).
    /// The row never shows it; its tooltip and the CSV carry it.
    pub session: String,
    /// What the session is called: the user's label, else its
    /// conversation's title; `None` when it has neither.
    pub title: Option<String>,
    /// Its project's name, when AI Juggler knows it.
    pub project: Option<String>,
    /// The UTC day, `YYYY-MM-DD`.
    pub day: String,
    /// Its tokens, every model together.
    pub tokens: juggler_core::model::TokenCounts,
    /// USD at list prices of the models that have one, summed; `None`
    /// when none has. Which models have none: `models` with no USD.
    pub usd: Option<f64>,
    /// Vendor credits, never converted.
    pub credits: Option<f64>,
    /// The lowest confidence among its models.
    pub confidence: juggler_core::model::Confidence,
    /// Each model's part, most tokens first.
    pub models: Vec<juggler_core::cost_ledger::CostRollup>,
}

impl CostRowInfo {
    /// One session day from its models' rollup rows (at least one).
    pub(crate) fn of(
        session: String,
        day: String,
        mut models: Vec<juggler_core::cost_ledger::CostRollup>,
    ) -> Self {
        use juggler_core::model::{Confidence, TokenCounts};
        let io = |row: &juggler_core::cost_ledger::CostRollup| row.tokens.input + row.tokens.output;
        models.sort_by(|a, b| io(b).cmp(&io(a)).then_with(|| a.model.cmp(&b.model)));
        let mut tokens = TokenCounts::default();
        let mut usd: Option<f64> = None;
        let mut credits: Option<f64> = None;
        let mut confidence: Option<Confidence> = None;
        for row in &models {
            tokens.input = tokens.input.saturating_add(row.tokens.input);
            tokens.output = tokens.output.saturating_add(row.tokens.output);
            tokens.cache_read = tokens.cache_read.saturating_add(row.tokens.cache_read);
            tokens.cache_write = tokens.cache_write.saturating_add(row.tokens.cache_write);
            tokens.reasoning = tokens.reasoning.saturating_add(row.tokens.reasoning);
            if let Some(value) = row.usd.usd {
                usd = Some(usd.unwrap_or(0.0) + value);
            }
            if let Some(value) = row.credits {
                credits = Some(credits.unwrap_or(0.0) + value);
            }
            // Measured > worked out > estimated: the row is as sure as
            // its least sure model.
            let rank = |c: Confidence| match c {
                Confidence::Estimated => 0,
                Confidence::Computed => 1,
                Confidence::Measured => 2,
            };
            confidence = Some(match confidence {
                Some(kept) if rank(kept) <= rank(row.confidence) => kept,
                _ => row.confidence,
            });
        }
        Self {
            session,
            title: None,
            project: None,
            day,
            tokens,
            usd,
            credits,
            confidence: confidence.unwrap_or_default(),
            models,
        }
    }
}

impl CostRollupsInfo {
    /// The poison/error fallback: an empty, honest matrix.
    #[must_use]
    pub(crate) fn default_cost() -> Self {
        Self {
            rows: Vec::new(),
            subscription: juggler_core::cost_ledger::subscription_view(None, None),
            pricing_stale: false,
            pricing_last_synced_ms: None,
            total_usd: None,
            unpriced_models: Vec::new(),
            scope: String::new(),
        }
    }
}
