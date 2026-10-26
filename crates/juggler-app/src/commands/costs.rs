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

/// The scope's total at list prices and the models it leaves out: the
/// priced buckets summed (the ALL rollup is still one bucket per model
/// and day), each unpriced model named once.
#[must_use]
pub fn priced_total(
    rows: &[juggler_core::cost_ledger::CostRollup],
) -> (Option<f64>, Vec<Option<String>>) {
    let mut total: Option<f64> = None;
    let mut unpriced = std::collections::BTreeSet::new();
    for row in rows {
        match row.usd.usd {
            Some(usd) => total = Some(total.unwrap_or(0.0) + usd),
            None => {
                unpriced.insert(row.model.clone());
            }
        }
    }
    (total, unpriced.into_iter().collect())
}

/// One row of the cost table (0.1.7): one session's UTC day, its models
/// folded together and named as the rest of the app names a session.
/// Each model's own part rides along for the row's breakdown.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
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
    fn of(
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
    // 0.1.7: the total was the first ALL bucket only (one model's one
    // day), and none at all once any model had no list price.
    let (total_usd, unpriced_models) = priced_total(&all);
    let view = juggler_core::cost_ledger::subscription_view(total_usd, fee);
    let now = juggler_core::model::Timestamp::now().millis();
    CostRollupsInfo {
        rows,
        subscription: view,
        pricing_stale: catalog.is_stale(now),
        pricing_last_synced_ms: catalog.last_synced_ms,
        total_usd,
        unpriced_models,
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
            unpriced_models: Vec::new(),
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

/// The cost table (one row per session and UTC day, its models folded
/// together), a page at a time: newest day first, then by session. Each
/// row's models are [`cost_rollups`]' rows (the same rollup, so the two
/// reconcile); the totals, the subscription view and the pricing flags
/// stay there. The whole table is never read: a page reads its session
/// days, then only the events of the days they're in, and names only
/// its own sessions.
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
) -> AppResult<crate::paging::Page<CostRowInfo>> {
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
) -> AppResult<crate::paging::Page<CostRowInfo>> {
    use crate::paging::{Page, SortKey};
    use juggler_core::cost_ledger::{CostRollup, RollupDimension, day_of, rollup};
    use std::collections::HashMap;
    const DAY_MS: i64 = 86_400_000;

    let limit = request.limit();
    // Newest day first: a row's rank is its day, negated.
    let after = request
        .start_after()?
        .map(|key| (-key.ranks.first().copied().unwrap_or_default(), key.id));
    let (days, more, total, events, catalog) = {
        let storage = lock_storage_arc(state)?;
        let usage = storage.usage();
        let mut days = usage.cost_days_page(
            account,
            after
                .as_ref()
                .map(|(day, session)| (*day, session.as_str())),
            limit + 1,
        )?;
        // The one past the page only says another page follows.
        let more = days.len() > limit;
        days.truncate(limit);
        let first = days.iter().map(|row| row.day).min();
        let last = days.iter().map(|row| row.day).max();
        let events = match (first, last) {
            (Some(first), Some(last)) => usage.events_in_days(account, first, last)?,
            _ => Vec::new(),
        };
        let total = usage.cost_day_count(account).ok();
        (days, more, total, events, pricing_catalog(&storage))
    };

    let mut models: HashMap<(String, String), Vec<CostRollup>> = HashMap::new();
    for row in rollup(&events, &catalog, RollupDimension::Session) {
        let day = row.day.clone().unwrap_or_default();
        models.entry((day, row.key.clone())).or_default().push(row);
    }
    let next_cursor = days
        .last()
        .filter(|_| more)
        .map(|row| SortKey::new([-row.day], row.session.clone()).cursor());
    let mut items: Vec<CostRowInfo> = days
        .into_iter()
        .filter_map(|row| {
            let day = day_of(row.day * DAY_MS);
            let parts = models.remove(&(day.clone(), row.session.clone()))?;
            Some(CostRowInfo::of(row.session, day, parts))
        })
        .collect();
    if !items.is_empty() {
        // Each session's account, from its own usage (a conversation is
        // known by both).
        let accounts: HashMap<&str, &str> = events
            .iter()
            .filter_map(|event| Some((event.session.as_deref()?, event.account.as_str())))
            .collect();
        name_rows(state, &mut items, &accounts)?;
    }
    Ok(Page {
        items,
        next_cursor,
        total,
    })
}

/// Names a page's sessions as the rail and History do: the user's label,
/// else the conversation's title, with its project. The conversation
/// index is the warm one Usage already read; a session it doesn't know
/// yet has no title (the row says "Untitled session").
fn name_rows(
    state: &Arc<AppState>,
    items: &mut [CostRowInfo],
    accounts: &std::collections::HashMap<&str, &str>,
) -> AppResult<()> {
    use std::collections::{HashMap, HashSet};
    let (labels, projects, homes) = {
        let storage = lock_storage_arc(state)?;
        // The user's labels: only labelled sessions, a few rows.
        let labels: HashMap<String, String> = storage
            .sessions()
            .conversation_labels()?
            .into_iter()
            .collect();
        let projects: HashMap<std::path::PathBuf, String> = storage
            .projects()
            .list()?
            .into_iter()
            .map(|project| (project.root, project.name))
            .collect();
        (labels, projects, storage.projects().worktree_homes()?)
    };
    let key = |session: &str| {
        accounts
            .get(session)
            .map(|account| ((*account).to_owned(), session.to_owned()))
    };
    let wanted: HashSet<(String, String)> =
        items.iter().filter_map(|row| key(&row.session)).collect();
    let facts = state.conversations.facts_of(&wanted);
    for row in items {
        let fact = key(&row.session).and_then(|key| facts.get(&key));
        row.title = labels
            .get(&row.session)
            .cloned()
            .or_else(|| fact.and_then(|(title, _)| title.clone()))
            .filter(|title| !title.trim().is_empty());
        row.project = fact
            .and_then(|(_, folder)| folder.as_deref())
            .map(|folder| juggler_core::storage::project_root_with(folder, &homes))
            .and_then(|root| projects.get(&root).cloned());
    }
    Ok(())
}
