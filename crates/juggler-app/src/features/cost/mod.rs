//! AJ-E609: the per-session cost matrix and its CSV export.

pub mod commands;
pub mod dto;

use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::cost::dto::{CostRollupsInfo, CostRowInfo};
use crate::features::pricing::pricing_catalog;
use crate::state::AppState;

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
    // D-62: the fee its Account is set with (the folder's own until moved).
    let fee = storage
        .terms()
        .effective(&account)
        .ok()
        .and_then(|terms| terms.fee_usd);
    (account, events, fee)
}

/// [`cost_rollups`](crate::features::cost::commands::cost_rollups)'s work.
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
    // 0.1.8: the total was the first ALL bucket only (one model's one
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

/// [`cost_csv_export`](crate::features::cost::commands::cost_csv_export)'s work: the CSV file's path.
///
/// # Errors
///
/// As [`cost_csv_export`](crate::features::cost::commands::cost_csv_export).
pub fn cost_csv_export_impl(
    state: &Arc<AppState>,
    account: Option<&str>,
    span: Option<(i64, i64)>,
) -> AppResult<String> {
    let storage = lock_storage_arc(state)?;
    let catalog = pricing_catalog(&storage);
    let usage = storage.usage();
    let accounts: Vec<String> = match account {
        Some(named) => vec![named.to_owned()],
        None => usage.usage_accounts()?,
    };
    let month_ms = 30 * 24 * 60 * 60 * 1000_i64;
    let now = juggler_core::model::Timestamp::now().millis();
    // AJ-E2902: a span (the weekly review's week) exports just its days.
    let (start, end) = span.unwrap_or((0, now));
    let path = std::env::temp_dir().join(format!("ai-juggler-costs-{now}.csv"));
    let mut file =
        std::fs::File::create(&path).map_err(|error| AppError::io("create cost csv", &error))?;
    // Header + chunked month windows, merged into the day map as we
    // go (bounded memory: the map is day-bucketed, the events chunk
    // never accumulates).
    let mut merged: Vec<juggler_core::cost_ledger::CostRollup> = Vec::new();
    for account_id in &accounts {
        let mut hi = end;
        loop {
            let lo = hi.saturating_sub(month_ms).max(start);
            let chunk = usage.events_between(account_id, lo, hi)?;
            if !chunk.is_empty() {
                let mut rows = juggler_core::cost_ledger::rollup(
                    &chunk,
                    &catalog,
                    juggler_core::cost_ledger::RollupDimension::Session,
                );
                merged.append(&mut rows);
            }
            if lo <= start {
                break;
            }
            hi = lo;
        }
    }
    juggler_core::cost_ledger::write_csv(&merged, &mut file)
        .map_err(|error| AppError::io("write cost csv", &error))?;
    Ok(path.to_string_lossy().to_string())
}

/// [`cost_rows_page`](crate::features::cost::commands::cost_rows_page)'s work.
///
/// # Errors
///
/// As [`cost_rows_page`](crate::features::cost::commands::cost_rows_page).
pub fn cost_rows_page_impl(
    state: &Arc<AppState>,
    account: Option<&str>,
    request: &crate::paging::PageRequest,
) -> AppResult<crate::paging::Page<CostRowInfo>> {
    use std::collections::HashMap;

    use juggler_core::cost_ledger::{CostRollup, RollupDimension, day_of, rollup};

    use crate::paging::{Page, SortKey};
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
        let labels: HashMap<(String, String), String> = storage
            .sessions()
            .conversation_labels()?
            .into_iter()
            .map(|(account, conversation, label)| ((account, conversation), label))
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
        let account_key = key(&row.session);
        let fact = account_key.as_ref().and_then(|key| facts.get(key));
        row.title = account_key
            .as_ref()
            .and_then(|key| labels.get(key))
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
