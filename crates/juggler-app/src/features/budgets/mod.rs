//! AJ-E1514: monthly caps. A cap is a USD amount a month for one account
//! or a group of projects; this month's usage is priced by the cost
//! ledger, as Usage shows it, and the cap warns in Needs you at 80 % and
//! 100 %. It never stops anything: this module reads usage and writes
//! only its own settings.
//!
//! The month is the user's: the screen sends the start of its local
//! month (the backend keeps no time zone).

pub mod commands;
pub mod dto;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use juggler_core::budget::{self, BudgetCap, BudgetLevel, BudgetScope};

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
pub use dto::{BudgetInfo, BudgetInput};

/// Where the caps are kept.
const CAPS_KEY: &str = "budget_caps";
/// How many caps there may be.
const CAPS_MAX: usize = 50;
/// The longest a month can be, so a month start older than this is refused.
const MONTH_MS: i64 = 32 * 24 * 60 * 60 * 1000;

fn seen_key(id: &str) -> String {
    format!("budget_seen:{id}")
}

fn level_word(level: BudgetLevel) -> &'static str {
    match level {
        BudgetLevel::Under => "under",
        BudgetLevel::Near => "near",
        BudgetLevel::Over => "over",
    }
}

fn level_rank(word: &str) -> u8 {
    match word {
        "over" => 2,
        "near" => 1,
        _ => 0,
    }
}

fn invalid(message: &str) -> AppError {
    AppError::Validation {
        subject: "cap",
        message: message.to_owned(),
    }
}

/// The caps as saved (an unreadable list reads as none, never a guess).
fn caps(storage: &juggler_core::storage::Storage) -> AppResult<Vec<BudgetCap>> {
    Ok(storage
        .app_settings()
        .get(CAPS_KEY)?
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default())
}

fn save_caps(storage: &juggler_core::storage::Storage, caps: &[BudgetCap]) -> AppResult<()> {
    let text = if caps.is_empty() {
        String::new()
    } else {
        serde_json::to_string(caps).map_err(|e| AppError::Io {
            context: "save the caps",
            message: e.to_string(),
        })?
    };
    storage.app_settings().set(CAPS_KEY, &text)?;
    Ok(())
}

/// A month start the screen may send: within the last month, not ahead.
fn checked_month(month_start_ms: i64, now_ms: i64) -> AppResult<i64> {
    if month_start_ms > now_ms + 24 * 60 * 60 * 1000 || month_start_ms < now_ms - MONTH_MS {
        return Err(invalid("That isn't this month."));
    }
    Ok(month_start_ms)
}

/// Every cap with this month's estimate.
///
/// # Errors
///
/// [`AppError::Validation`] for a month start that isn't this month's;
/// storage errors.
pub fn list_impl(state: &Arc<AppState>, month_start_ms: i64) -> AppResult<Vec<BudgetInfo>> {
    let now = juggler_core::model::Timestamp::now().millis();
    let since = checked_month(month_start_ms, now)?;
    let saved = caps(&*lock_storage_arc(state)?)?;
    if saved.is_empty() {
        return Ok(Vec::new());
    }
    // Which project each conversation belongs to, as the rest of the app
    // folds them (worktrees into their repository).
    let projects: HashMap<(String, String), String> = if saved
        .iter()
        .any(|cap| matches!(cap.scope, BudgetScope::Projects { .. }))
    {
        crate::features::conversations::commands::list_impl(state)?
            .into_iter()
            .filter_map(|c| Some(((c.account_id, c.id), c.project_id?)))
            .collect()
    } else {
        HashMap::new()
    };
    let storage = lock_storage_arc(state)?;
    let catalog = crate::commands::pricing_catalog(&storage);
    let usage = storage.usage();
    let mut by_account: HashMap<String, Vec<juggler_core::quota::UsageEvent>> = HashMap::new();
    let mut events_of = |account: &str| -> Vec<juggler_core::quota::UsageEvent> {
        by_account
            .entry(account.to_owned())
            .or_insert_with(|| usage.events_for_account(account, since).unwrap_or_default())
            .clone()
    };
    let accounts = usage.usage_accounts().unwrap_or_default();
    let mut out = Vec::new();
    for cap in saved {
        let events: Vec<_> = match &cap.scope {
            BudgetScope::Account { account_id } => events_of(account_id),
            BudgetScope::Projects { project_ids } => {
                let wanted: HashSet<&str> = project_ids.iter().map(String::as_str).collect();
                accounts
                    .iter()
                    .flat_map(|account| events_of(account))
                    .filter(|event| {
                        event.session.as_ref().is_some_and(|session| {
                            projects
                                .get(&(event.account.clone(), session.clone()))
                                .is_some_and(|project| wanted.contains(project.as_str()))
                        })
                    })
                    .collect()
            }
        };
        let (spent_usd, unpriced) = budget::spent(&events, &catalog);
        let level = level_word(budget::level(spent_usd.unwrap_or(0.0), cap.monthly_usd));
        let seen = storage
            .app_settings()
            .get(&seen_key(&cap.id))?
            .and_then(|text| {
                let (month, seen_level) = text.split_once(':')?;
                Some(
                    month.parse::<i64>().ok()? == since
                        && level_rank(seen_level) >= level_rank(level),
                )
            })
            .unwrap_or(false);
        let (account_id, project_ids) = match cap.scope {
            BudgetScope::Account { account_id } => (Some(account_id), Vec::new()),
            BudgetScope::Projects { project_ids } => (None, project_ids),
        };
        out.push(BudgetInfo {
            id: cap.id,
            name: cap.name,
            monthly_usd: cap.monthly_usd,
            account_id,
            project_ids,
            spent_usd,
            unpriced,
            level: level.to_owned(),
            seen,
        });
    }
    Ok(out)
}

/// Saves a new cap or changes one; returns its id.
///
/// # Errors
///
/// [`AppError::Validation`] in the form's words; [`AppError::NotFound`]
/// for a cap, account or project that isn't there; storage errors.
pub fn save_impl(state: &Arc<AppState>, input: BudgetInput) -> AppResult<String> {
    let storage = lock_storage_arc(state)?;
    let scope = match input.account_id.filter(|id| !id.is_empty()) {
        Some(account_id) => {
            storage
                .accounts()
                .get(&juggler_core::model::AccountId::new(account_id.as_str()))?;
            BudgetScope::Account { account_id }
        }
        None => {
            let mut project_ids = input.project_ids;
            project_ids.sort();
            project_ids.dedup();
            for id in &project_ids {
                storage
                    .projects()
                    .get(&juggler_core::model::ProjectId::new(id.as_str()))?;
            }
            BudgetScope::Projects { project_ids }
        }
    };
    let mut saved = caps(&storage)?;
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| format!("cap-{}", crate::commands::unique_suffix()));
    let cap = BudgetCap {
        id: id.clone(),
        name: input.name.trim().to_owned(),
        monthly_usd: input.monthly_usd,
        scope,
    };
    if let Some(message) = budget::invalid(&cap) {
        return Err(invalid(message));
    }
    match (input.id, saved.iter().position(|kept| kept.id == id)) {
        (Some(_), Some(at)) => saved[at] = cap,
        (Some(_), None) => {
            return Err(AppError::NotFound {
                entity: "cap",
                id: id.clone(),
            });
        }
        (None, _) if saved.len() >= CAPS_MAX => {
            return Err(invalid("You have 50 caps already: remove one first"));
        }
        (None, _) => saved.push(cap),
    }
    save_caps(&storage, &saved)?;
    // A changed cap warns afresh.
    storage.app_settings().set(&seen_key(&id), "")?;
    Ok(id)
}

/// Removes a cap. Removing one that isn't there is not an error.
///
/// # Errors
///
/// Storage errors.
pub fn remove_impl(state: &Arc<AppState>, id: &str) -> AppResult<()> {
    let storage = lock_storage_arc(state)?;
    let mut saved = caps(&storage)?;
    saved.retain(|cap| cap.id != id);
    save_caps(&storage, &saved)?;
    storage.app_settings().set(&seen_key(id), "")?;
    Ok(())
}

/// The user has seen a cap's warning: Needs you leaves it out until it
/// reaches the next level, or the month ends.
///
/// # Errors
///
/// [`AppError::Validation`] for a month that isn't this one or a level
/// that isn't a warning; [`AppError::NotFound`] for an unknown cap.
pub fn seen_impl(
    state: &Arc<AppState>,
    id: &str,
    month_start_ms: i64,
    level: &str,
) -> AppResult<()> {
    let month = checked_month(
        month_start_ms,
        juggler_core::model::Timestamp::now().millis(),
    )?;
    if !matches!(level, "near" | "over") {
        return Err(invalid("That isn't a warning."));
    }
    let storage = lock_storage_arc(state)?;
    if !caps(&storage)?.iter().any(|cap| cap.id == id) {
        return Err(AppError::NotFound {
            entity: "cap",
            id: id.to_owned(),
        });
    }
    storage
        .app_settings()
        .set(&seen_key(id), &format!("{month}:{level}"))?;
    Ok(())
}
