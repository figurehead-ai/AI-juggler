//! AJ-E609: the pricing catalog and the user's price overrides.

pub mod commands;

/// The catalog as loaded: bundled seed + user overrides + staleness.
#[must_use]
pub(crate) fn pricing_catalog(
    storage: &juggler_core::storage::Storage,
) -> juggler_core::cost_ledger::PricingCatalog {
    use juggler_core::cost_ledger::PricingCatalog;
    let mut catalog = PricingCatalog {
        bundled: juggler_core::cost_ledger::load_seed_rows(),
        overrides: storage.usage().pricing_overrides().unwrap_or_default(),
        last_synced_ms: None,
    };
    // How fresh the prices are: the bundled table's capture day, or a
    // later sync when one was recorded (0.1.8: the warning read only the
    // sync time, which nothing writes, so it could never show).
    let synced = storage
        .app_settings()
        .get("pricing_synced_at")
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok());
    catalog.last_synced_ms = synced.max(juggler_core::cost_ledger::seed_captured_ms());
    catalog
}
