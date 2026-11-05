//! AJ-E2902: the cost cuts Usage shows beside its charts — what a finished
//! task costs, what went on sessions that crashed or were probably stuck
//! when they ended, each Account's cost with its top project, and the
//! price list the estimates use. From the same summary as the charts
//! (`super::summarize`): never a sum of what's on screen, never a
//! transcript read.
//!
//! A finished task (D-77, the Q1 ruling): a session AI Juggler ran that
//! ended on its own as finished — not stopped, crashed or handed over.
//! Sessions other apps run carry no ending, so they count in neither cut
//! and the figure says how many it knows of.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use juggler_core::model::SessionStatus;
use serde::Serialize;
use ts_rs::TS;

use super::dto::{UsageQueryInput, UsageSessionInfo};
use super::fold::Folded;
use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::paging::{self, Page, PageRequest, SortKey};
use crate::state::AppState;

/// How an AI Juggler session ended, for the cuts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Ended on its own as finished: a finished task.
    Finished,
    /// Exited abnormally.
    Crashed,
    /// Stopped while it was probably stuck (inferred: "probably").
    Stuck,
}

impl Outcome {
    fn wasted(self) -> bool {
        matches!(self, Self::Crashed | Self::Stuck)
    }
}

/// Each ended AI Juggler session's outcome, by its id.
pub type Outcomes = HashMap<String, Outcome>;
/// Project names by id.
pub type ProjectNames = HashMap<String, String>;

/// Each ended AI Juggler session's outcome, from its status and its last
/// status change (one read each).
///
/// # Errors
///
/// Storage errors.
pub fn outcomes(storage: &juggler_core::storage::Storage) -> AppResult<HashMap<String, Outcome>> {
    let last = storage.sessions().last_transitions()?;
    let mut out = HashMap::new();
    for record in storage.sessions().list(None)? {
        let id = record.id.to_string();
        let from_stuck = last
            .get(&id)
            .is_some_and(|(from, _, _)| from.as_deref() == Some(SessionStatus::Stuck.as_str()));
        let outcome = match record.status {
            SessionStatus::Finished => Some(Outcome::Finished),
            SessionStatus::Failed => Some(Outcome::Crashed),
            SessionStatus::Stopped if from_stuck => Some(Outcome::Stuck),
            _ => None,
        };
        if let Some(outcome) = outcome {
            out.insert(id, outcome);
        }
    }
    Ok(out)
}

/// A cut's sum: how many sessions, their new tokens and their estimate.
#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageCutInfo {
    /// Sessions in the cut with usage in the range.
    pub count: u32,
    /// Their new tokens (the charts' count).
    #[ts(type = "number")]
    pub io: u64,
    /// Their estimate at list prices; `None` when none has a price.
    pub usd: Option<f64>,
    /// Some of them used a model without a list price.
    pub unpriced: bool,
}

impl UsageCutInfo {
    fn take(&mut self, row: &UsageSessionInfo) {
        self.count = self.count.saturating_add(1);
        self.io = self.io.saturating_add(row.io);
        match row.usd {
            Some(usd) => self.usd = Some(self.usd.unwrap_or(0.0) + usd),
            None if row.io > 0 => self.unpriced = true,
            None => {}
        }
    }
}

/// An Account's top project in the range.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageTopProjectInfo {
    /// The project's id.
    pub id: String,
    /// Its name.
    pub label: String,
    /// Its estimate on this Account; `None` when unpriced.
    pub usd: Option<f64>,
    /// Its new tokens on this Account.
    #[ts(type = "number")]
    pub io: u64,
}

/// One Account's cost in the range (its folders together; a session that
/// moved Accounts counts on each for its own part).
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageAccountCostInfo {
    /// The Account's key (a folder's id when no Account lists it).
    pub key: String,
    /// The Account's stored id, for its page.
    pub account_id: Option<String>,
    /// Its name.
    pub label: String,
    /// Its tool's id.
    pub tool: String,
    /// New tokens.
    #[ts(type = "number")]
    pub io: u64,
    /// Cache reads.
    #[ts(type = "number")]
    pub cache: u64,
    /// The estimate at list prices; `None` when nothing has a price.
    pub usd: Option<f64>,
    /// Some usage was on a model without a list price.
    pub unpriced: bool,
    /// Sessions with usage.
    pub sessions: u32,
    /// The project it went on most (by estimate, else tokens).
    pub top_project: Option<UsageTopProjectInfo>,
}

/// The price list the estimates use.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsagePricesInfo {
    /// The bundled list's version.
    pub version: String,
    /// The day its prices were captured (`YYYY-MM-DD`).
    pub captured: String,
    /// The prices are over 30 days old.
    pub stale: bool,
    /// The user set some prices of their own.
    pub overridden: bool,
}

/// The cuts for one range.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageCostCutsInfo {
    /// Finished tasks (D-77).
    pub finished: UsageCutInfo,
    /// Sessions that crashed or were probably stuck when they ended.
    pub wasted: UsageCutInfo,
    /// Of `wasted`, how many crashed.
    pub crashed: u32,
    /// Of `wasted`, how many were probably stuck.
    pub stuck: u32,
    /// Sessions with usage whose ending isn't known (other apps run them).
    pub unknown_endings: u32,
    /// Each Account, the costliest first.
    pub by_account: Vec<UsageAccountCostInfo>,
    /// The price list in use.
    pub prices: UsagePricesInfo,
}

/// The Accounts the folders belong to: folder id → (key, account id,
/// name, tool).
pub type AccountOf = HashMap<String, (String, Option<String>, String, String)>;

fn accounts_of(state: &Arc<AppState>) -> AccountOf {
    let mut out = HashMap::new();
    for account in crate::features::subscriptions::list_impl(state).unwrap_or_default() {
        for profile in &account.profiles {
            out.insert(
                profile.id.clone(),
                (
                    account.key.clone(),
                    account.account_id.clone(),
                    account.display_name.clone(),
                    account.tool.clone(),
                ),
            );
        }
    }
    out
}

/// The finished and wasted cuts: (finished, wasted, crashed, stuck,
/// endings not known).
fn endings(now: &Folded, outcomes: &Outcomes) -> (UsageCutInfo, UsageCutInfo, u32, u32, u32) {
    let mut finished = UsageCutInfo::default();
    let mut wasted = UsageCutInfo::default();
    let (mut crashed, mut stuck, mut unknown) = (0_u32, 0_u32, 0_u32);
    for row in &now.sessions {
        match row.session_id.as_ref().and_then(|id| outcomes.get(id)) {
            Some(Outcome::Finished) => finished.take(row),
            Some(outcome @ (Outcome::Crashed | Outcome::Stuck)) => {
                wasted.take(row);
                if *outcome == Outcome::Crashed {
                    crashed += 1;
                } else {
                    stuck += 1;
                }
            }
            None if row.session_id.is_none() => unknown += 1,
            None => {}
        }
    }
    (finished, wasted, crashed, stuck, unknown)
}

/// Each Account, the costliest first.
fn by_account(
    now: &Folded,
    account_of: &AccountOf,
    projects: &ProjectNames,
) -> Vec<UsageAccountCostInfo> {
    // Each Account: its folders' shares summed, its top project from its
    // sessions' parts.
    let mut accounts: BTreeMap<String, UsageAccountCostInfo> = BTreeMap::new();
    for share in &now.by_profile {
        let (key, account_id, label, tool) = account_of
            .get(&share.key)
            .cloned()
            .unwrap_or_else(|| (share.key.clone(), None, share.label.clone(), String::new()));
        let entry = accounts
            .entry(key.clone())
            .or_insert_with(|| UsageAccountCostInfo {
                key,
                account_id,
                label,
                tool,
                io: 0,
                cache: 0,
                usd: None,
                unpriced: false,
                sessions: 0,
                top_project: None,
            });
        entry.io = entry.io.saturating_add(share.io);
        entry.cache = entry.cache.saturating_add(share.cache);
        if let Some(usd) = share.usd {
            entry.usd = Some(entry.usd.unwrap_or(0.0) + usd);
        }
        entry.sessions = entry.sessions.saturating_add(share.sessions);
    }
    let mut parts: HashMap<(String, String), (u64, Option<f64>)> = HashMap::new();
    for row in &now.sessions {
        let key = account_of
            .get(&row.profile)
            .map_or_else(|| row.profile.clone(), |found| found.0.clone());
        if row.usd.is_none()
            && row.io > 0
            && let Some(entry) = accounts.get_mut(&key)
        {
            entry.unpriced = true;
        }
        let Some(project) = row.project_id.clone() else {
            continue;
        };
        let part = parts.entry((key, project)).or_insert((0, None));
        part.0 = part.0.saturating_add(row.io);
        if let Some(usd) = row.usd {
            part.1 = Some(part.1.unwrap_or(0.0) + usd);
        }
    }
    for ((key, project), (io, usd)) in parts {
        let Some(entry) = accounts.get_mut(&key) else {
            continue;
        };
        let better = entry
            .top_project
            .as_ref()
            .is_none_or(|top| (usd.unwrap_or(0.0), io) > (top.usd.unwrap_or(0.0), top.io));
        if better {
            entry.top_project = Some(UsageTopProjectInfo {
                label: projects
                    .get(&project)
                    .cloned()
                    .unwrap_or_else(|| project.clone()),
                id: project,
                usd,
                io,
            });
        }
    }
    let mut by_account: Vec<_> = accounts.into_values().collect();
    by_account.sort_by(|a, b| {
        b.usd
            .unwrap_or(0.0)
            .total_cmp(&a.usd.unwrap_or(0.0))
            .then(b.io.cmp(&a.io))
            .then(a.label.cmp(&b.label))
    });
    by_account
}

/// Folds the range's sessions into the cuts.
#[must_use]
pub fn cuts_of(
    now: &Folded,
    outcomes: &Outcomes,
    account_of: &AccountOf,
    projects: &ProjectNames,
    prices: UsagePricesInfo,
) -> UsageCostCutsInfo {
    let (finished, wasted, crashed, stuck, unknown) = endings(now, outcomes);
    UsageCostCutsInfo {
        finished,
        wasted,
        crashed,
        stuck,
        unknown_endings: unknown,
        by_account: by_account(now, account_of, projects),
        prices,
    }
}

fn prices(storage: &juggler_core::storage::Storage) -> UsagePricesInfo {
    let catalog = crate::commands::pricing_catalog(storage);
    let (version, captured) = juggler_core::cost_ledger::seed_label();
    UsagePricesInfo {
        version: version.to_owned(),
        captured: captured.to_owned(),
        stale: catalog.is_stale(juggler_core::model::Timestamp::now().millis()),
        overridden: !catalog.overrides.is_empty(),
    }
}

/// The range's cost cuts.
///
/// # Errors
///
/// As [`super::history_impl`].
pub fn cuts_impl(state: &Arc<AppState>, input: &UsageQueryInput) -> AppResult<UsageCostCutsInfo> {
    let (now, _, _, scope) = super::summarize(state, input)?;
    let account_of = accounts_of(state);
    let storage = lock_storage_arc(state)?;
    let outcomes = outcomes(&storage)?;
    let prices = prices(&storage);
    Ok(cuts_of(
        &now,
        &outcomes,
        &account_of,
        &scope.lookups.projects,
        prices,
    ))
}

/// One session in the wasted list, and why.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageWastedInfo {
    /// The session.
    pub session: UsageSessionInfo,
    /// `crashed` | `stuck` (probably).
    pub why: String,
}

/// A page of the range's wasted sessions, the costliest first.
///
/// # Errors
///
/// As [`super::sessions_page_impl`].
pub fn wasted_page_impl(
    state: &Arc<AppState>,
    input: &UsageQueryInput,
    request: &PageRequest,
) -> AppResult<Page<UsageWastedInfo>> {
    let (now, _, _, _) = super::summarize(state, input)?;
    let outcomes = {
        let storage = lock_storage_arc(state)?;
        outcomes(&storage)?
    };
    let rows: Vec<UsageWastedInfo> = now
        .sessions
        .into_iter()
        .filter_map(|session| {
            let outcome = *session
                .session_id
                .as_ref()
                .and_then(|id| outcomes.get(id))?;
            outcome.wasted().then(|| UsageWastedInfo {
                why: if outcome == Outcome::Crashed {
                    "crashed"
                } else {
                    "stuck"
                }
                .to_owned(),
                session,
            })
        })
        .collect();
    paging::page(
        rows,
        |row| {
            // A session's estimate in cents fits an i64 by far.
            #[allow(clippy::cast_possible_truncation)]
            let cents = row
                .session
                .usd
                .map_or(0, |usd| (usd * 100.0).round() as i64);
            SortKey::new(
                [-cents, -row.session.last_ms],
                format!("{}\u{0}{}", row.session.profile, row.session.id),
            )
        },
        request,
    )
}
