//! AJ-E609: the cost commands: the per-session cost matrix, its CSV
//! export and the pricing catalog with the user's overrides. Each runs
//! off the main thread.

use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

use super::lock_storage_arc;

/// AJ-E609: the account's cost rollups (session/account/project/day
/// dimensions share one shape — the UI matrix and the CSV export read
/// the SAME rows, so reconciliation is exact) plus the dual
/// subscription view and the pricing-staleness flag.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
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
    pub pricing_last_synced_ms: Option<i64>,
    /// The grand-total row (ALL bucket) for the scope.
    pub total_usd: Option<f64>,
    /// The scope the query ran over (account id or `ALL`).
    pub scope: String,
}

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
    catalog.last_synced_ms = storage
        .app_settings()
        .get("pricing_synced_at")
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok());
    catalog
}

/// The scoped events (one account, or every account with usage) plus
/// the account's optional user-entered flat fee.
fn scoped_cost_events(
    storage: &juggler_core::storage::Storage,
    account: Option<&str>,
) -> (String, Vec<juggler_core::quota::UsageEvent>, Option<f64>) {
    let account = account.map_or_else(|| "ALL".to_owned(), str::to_owned);
    let usage = storage.usage();
    let mut events = Vec::new();
    if account == "ALL" {
        for account_id in usage.usage_accounts().unwrap_or_default() {
            events.extend(usage.events_for_account(&account_id, 0).unwrap_or_default());
        }
    } else {
        events = usage.events_for_account(&account, 0).unwrap_or_default();
    }
    let fee = storage
        .app_settings()
        .get(&format!("subscription_fee:{account}"))
        .ok()
        .flatten()
        .and_then(|value| value.parse::<f64>().ok());
    (account, events, fee)
}

/// AJ-E609 Req.3: the per-session cost matrix (session × model × day).
///
/// # Errors
///
/// The work could not finish off the main thread.
#[tauri::command]
pub async fn cost_rollups(
    state: tauri::State<'_, Arc<AppState>>,
    account: Option<String>,
) -> AppResult<CostRollupsInfo> {
    crate::ipc::blocking(state.inner(), "cost rollups", move |state| {
        Ok(cost_rollups_impl(state, account.as_deref()))
    })
    .await
}

/// [`cost_rollups`]'s work.
#[must_use]
pub fn cost_rollups_impl(state: &Arc<AppState>, account: Option<&str>) -> CostRollupsInfo {
    let Ok(storage) = state.storage.lock() else {
        return CostRollupsInfo::default_cost();
    };
    let catalog = pricing_catalog(&storage);
    let (scope, events, fee) = scoped_cost_events(&storage, account);
    let rows = juggler_core::cost_ledger::rollup(
        &events,
        &catalog,
        juggler_core::cost_ledger::RollupDimension::Session,
    );
    let all = juggler_core::cost_ledger::rollup(
        &events,
        &catalog,
        juggler_core::cost_ledger::RollupDimension::All,
    );
    let total_usd = all.first().and_then(|row| row.usd.usd);
    let view = juggler_core::cost_ledger::subscription_view(total_usd, fee);
    let now = juggler_core::model::Timestamp::now().millis();
    CostRollupsInfo {
        rows,
        subscription: view,
        pricing_stale: catalog.is_stale(now),
        pricing_last_synced_ms: catalog.last_synced_ms,
        total_usd,
        scope,
    }
}

impl CostRollupsInfo {
    /// The poison/error fallback: an empty, honest matrix.
    #[must_use]
    fn default_cost() -> Self {
        Self {
            rows: Vec::new(),
            subscription: juggler_core::cost_ledger::subscription_view(None, None),
            pricing_stale: false,
            pricing_last_synced_ms: None,
            total_usd: None,
            scope: String::new(),
        }
    }
}

/// AJ-E609 Req.6: streams the CSV to a temp file (chunked month
/// windows — never a full-table read) and returns its path. The rows
/// are the SAME rollup shape the UI displays.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage or I/O failure.
#[tauri::command]
pub async fn cost_csv_export(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    account: Option<String>,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "cost csv export", move |state| {
        cost_csv_export_impl(state, account.as_deref())
    })
    .await
}

/// [`cost_csv_export`]'s work: the CSV file's path.
///
/// # Errors
///
/// As [`cost_csv_export`].
pub fn cost_csv_export_impl(state: &Arc<AppState>, account: Option<&str>) -> AppResult<String> {
    let storage = lock_storage_arc(state)?;
    let catalog = pricing_catalog(&storage);
    let usage = storage.usage();
    let accounts: Vec<String> = match account {
        Some(named) => vec![named.to_owned()],
        None => usage.usage_accounts()?,
    };
    let month_ms = 30 * 24 * 60 * 60 * 1000_i64;
    let now = juggler_core::model::Timestamp::now().millis();
    let path = std::env::temp_dir().join(format!("ai-juggler-costs-{now}.csv"));
    let mut file =
        std::fs::File::create(&path).map_err(|error| AppError::io("create cost csv", &error))?;
    // Header + chunked month windows, merged into the day map as we
    // go (bounded memory: the map is day-bucketed, the events chunk
    // never accumulates).
    let mut merged: Vec<juggler_core::cost_ledger::CostRollup> = Vec::new();
    for account_id in &accounts {
        let mut hi = now;
        loop {
            let lo = hi.saturating_sub(month_ms);
            let chunk = usage.events_between(account_id, lo, hi)?;
            if !chunk.is_empty() {
                let mut rows = juggler_core::cost_ledger::rollup(
                    &chunk,
                    &catalog,
                    juggler_core::cost_ledger::RollupDimension::Session,
                );
                merged.append(&mut rows);
            }
            if lo <= 0 {
                break;
            }
            hi = lo;
        }
    }
    juggler_core::cost_ledger::write_csv(&merged, &mut file)
        .map_err(|error| AppError::io("write cost csv", &error))?;
    Ok(path.to_string_lossy().to_string())
}

/// AJ-E609 Req.1: set one user pricing override (the editor shows
/// `source_url` + effective date before save — AJ-E610's surface).
///
/// # Errors
///
/// Returns the typed [`AppError`] on validation or storage failure.
#[tauri::command]
pub async fn pricing_override_set(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    row: juggler_core::cost_ledger::LedgerPriceRow,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "pricing override set", move |state| {
        if row.model.trim().is_empty() || row.source_url.trim().is_empty() {
            return Err(AppError::Validation {
                subject: "pricing override",
                message: "model and source_url are required (auditable pricing law)".into(),
            });
        }
        let storage = lock_storage_arc(state)?;
        storage
            .usage()
            .pricing_override_upsert(&row)
            .map_err(AppError::from)?;
        Ok(())
    })
    .await
}

/// AJ-E609: clear one override (history reprices back to bundled).
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
#[tauri::command]
pub async fn pricing_override_clear(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    model: String,
    effective_from_ms: i64,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "pricing override clear", move |state| {
        let storage = lock_storage_arc(state)?;
        storage
            .usage()
            .pricing_override_delete(&model, effective_from_ms)
            .map_err(AppError::from)?;
        Ok(())
    })
    .await
}

/// AJ-E609: catalog metadata for the pricing editor (staleness, last
/// synced, the bundled + override rows).
#[tauri::command]
///
/// # Errors
///
/// The work could not finish off the main thread.
pub async fn pricing_catalog_info(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> AppResult<juggler_core::cost_ledger::PricingCatalog> {
    crate::ipc::blocking(state.inner(), "pricing catalog info", move |state| {
        Ok(match state.storage.lock() {
            Ok(storage) => pricing_catalog(&storage),
            Err(_) => juggler_core::cost_ledger::PricingCatalog::default(),
        })
    })
    .await
}

/// The cost table (session × model × UTC day), a page at a time: newest
/// day first, then by session and model. Each row is one of
/// [`cost_rollups`]'s rows (the same rollup, so the two reconcile); the
/// totals, the subscription view and the pricing flags stay there. The
/// whole table is never read: a page reads its buckets, then only the
/// events of the days they're in.
///
/// # Errors
///
/// [`AppError::Validation`] for a cursor this app didn't make; storage
/// errors.
#[tauri::command]
pub async fn cost_rows_page(
    state: tauri::State<'_, Arc<AppState>>,
    account: Option<String>,
    page: crate::paging::PageRequest,
) -> AppResult<crate::paging::Page<juggler_core::cost_ledger::CostRollup>> {
    crate::ipc::blocking(state.inner(), "the cost table", move |state| {
        cost_rows_page_impl(state, account.as_deref(), &page)
    })
    .await
}

/// [`cost_rows_page`]'s work.
///
/// # Errors
///
/// As [`cost_rows_page`].
pub fn cost_rows_page_impl(
    state: &Arc<AppState>,
    account: Option<&str>,
    request: &crate::paging::PageRequest,
) -> AppResult<crate::paging::Page<juggler_core::cost_ledger::CostRollup>> {
    use crate::paging::{Page, SortKey};
    use juggler_core::cost_ledger::{RollupDimension, day_of, rollup};
    const DAY_MS: i64 = 86_400_000;

    let limit = request.limit();
    // Newest day first: a bucket's rank is its day, negated.
    let after = request
        .start_after()?
        .map(|key| (-key.ranks.first().copied().unwrap_or_default(), key.id));
    let (buckets, total, events, catalog) = {
        let storage = lock_storage_arc(state)?;
        let usage = storage.usage();
        let mut buckets = usage.cost_buckets_page(
            account,
            after.as_ref().map(|(day, key)| (*day, key.as_str())),
            limit + 1,
        )?;
        // The one past the page only says another page follows.
        let more = buckets.len() > limit;
        buckets.truncate(limit);
        let first = buckets.iter().map(|bucket| bucket.day).min();
        let last = buckets.iter().map(|bucket| bucket.day).max();
        let events = match (first, last) {
            (Some(first), Some(last)) => usage.events_in_days(account, first, last)?,
            _ => Vec::new(),
        };
        let total = usage.cost_bucket_count(account).ok();
        ((buckets, more), total, events, pricing_catalog(&storage))
    };
    let (buckets, more) = buckets;
    let mut rows: std::collections::HashMap<(String, String, Option<String>), _> =
        rollup(&events, &catalog, RollupDimension::Session)
            .into_iter()
            .map(|row| {
                (
                    (
                        row.day.clone().unwrap_or_default(),
                        row.key.clone(),
                        row.model.clone(),
                    ),
                    row,
                )
            })
            .collect();
    let next_cursor = buckets
        .last()
        .filter(|_| more)
        .map(|bucket| SortKey::new([-bucket.day], bucket.key()).cursor());
    let items = buckets
        .into_iter()
        .filter_map(|bucket| {
            rows.remove(&(day_of(bucket.day * DAY_MS), bucket.session, bucket.model))
        })
        .collect();
    Ok(Page {
        items,
        next_cursor,
        total,
    })
}
