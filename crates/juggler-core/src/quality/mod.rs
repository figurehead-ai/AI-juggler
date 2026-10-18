//! Quality signals without a score (D-07, AJ-E409): individually
//! labelled, numerator/denominator shown, tier tags adjacent — **no
//! composite 0-100 quality/health/stuck score exists in type, payload,
//! or UI** (grep-gate test enforces it).

pub mod pricing;
pub mod signals;

pub use pricing::{PricingTable, load_pricing_table, session_cost};
pub use signals::{
    ContextReport, ErrorRate, QualitySignals, TokensPerMin, UsageEvent, quota_forecast_label,
};
