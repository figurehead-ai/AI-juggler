//! Usage history (0.1.5): what the Usage, project, account and session
//! pages chart. The store's summary queries (`juggler_core::storage::
//! UsageHistoryRepo`) sum each profile's usage per hour; this module names
//! the groups (profiles, tools, models, projects, sessions), prices them at
//! list prices and folds them into the screen's buckets. It reads only.
//!
//! Sessions other apps run count when their transcripts on this Mac carry
//! token counts; the ones whose tool keeps none are counted as not
//! measured, never guessed.

pub mod commands;
pub mod dto;
pub mod fold;
mod limits;
mod settled;

use std::sync::Arc;

use juggler_core::model::TokenCounts;
use juggler_core::storage::{HourRow, Storage};

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::paging::{self, Page, PageRequest, SortKey};
use crate::state::AppState;
pub use dto::{UsageHistoryInfo, UsageLimitsInfo, UsageQueryInput, UsageSessionDetailInfo};
use fold::{Folded, Lookups, ProfileFacts, SessionFacts};
pub use limits::{limits_impl, session_impl};

/// The most buckets a range is cut into.
pub const MAX_BUCKETS: usize = 400;
/// The most hourly rows read per profile for one range (years of heavy
/// use; past it the oldest hours are left out and the summary says so).
const MAX_ROWS_PER_PROFILE: usize = 60_000;
/// The longest range: twenty years.
const MAX_SPAN_MS: i64 = 20 * 366 * 86_400_000;

fn invalid(message: &str) -> AppError {
    AppError::Validation {
        subject: "usage range",
        message: message.to_owned(),
    }
}

/// A query the screen may send: 2 to 401 ascending edges no more than
/// twenty years apart, a known grouping.
///
/// # Errors
///
/// [`AppError::Validation`] in plain words.
pub fn checked(input: &UsageQueryInput) -> AppResult<()> {
    let edges = &input.edges_ms;
    if edges.len() < 2 || edges.len() > MAX_BUCKETS + 1 {
        return Err(invalid("A range needs 2 to 401 edges."));
    }
    if edges.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(invalid("A range's edges must rise."));
    }
    if edges[edges.len() - 1] - edges[0] > MAX_SPAN_MS {
        return Err(invalid("A range is at most twenty years."));
    }
    if !matches!(
        input.group_by.as_str(),
        "profile" | "tool" | "model" | "project"
    ) {
        return Err(invalid("Usage splits by profile, tool, model or project."));
    }
    Ok(())
}

/// What the summary knows about the store's profiles, sessions and
/// projects, and the conversations in scope (for their starts and the
/// unmeasured count).
struct Scope {
    lookups: Lookups,
    profiles: Vec<String>,
    /// Conversations in scope: (start, last activity, measured).
    conversations: Vec<(Option<i64>, Option<i64>, bool)>,
}

fn scope_of(state: &Arc<AppState>, input: &UsageQueryInput) -> AppResult<Scope> {
    // The conversations index names each session's project, tool and
    // title, as the rail and the project pages fold them.
    let conversations = crate::features::conversations::commands::list_impl(state)?;
    let storage = lock_storage_arc(state)?;
    let mut lookups = Lookups::default();
    for account in storage.accounts().list()? {
        lookups.profiles.insert(
            account.id.to_string(),
            ProfileFacts {
                label: account.label,
                tool: account.harness.to_string(),
            },
        );
    }
    for project in storage.projects().list()? {
        lookups
            .projects
            .insert(project.id.to_string(), project.name);
    }
    let wanted = |profile: &str, project: Option<&String>| {
        input.profile.as_deref().is_none_or(|only| only == profile)
            && input
                .project
                .as_ref()
                .is_none_or(|only| project == Some(only))
    };
    let mut in_scope = Vec::new();
    for conversation in conversations {
        if wanted(&conversation.account_id, conversation.project_id.as_ref()) {
            let measured = conversation.tokens.is_some_and(|t| t.input + t.output > 0);
            in_scope.push((
                conversation.started_ms,
                conversation.last_activity_ms,
                measured,
            ));
        }
        lookups.sessions.insert(
            (conversation.account_id, conversation.id),
            SessionFacts {
                project: conversation.project_id,
                tool: Some(conversation.tool),
                title: conversation.title,
                juggler: conversation.session_id,
            },
        );
    }
    let mut profiles: Vec<String> = lookups
        .profiles
        .keys()
        .filter(|id| {
            input
                .profile
                .as_deref()
                .is_none_or(|only| only == id.as_str())
        })
        .cloned()
        .collect();
    profiles.sort();
    Ok(Scope {
        lookups,
        profiles,
        conversations: in_scope,
    })
}

/// Every profile's hourly rows in `[start, end)`, the project's only when
/// one is asked for. Says whether any profile's rows were cut.
fn rows_between(
    storage: &Storage,
    data_dir: &std::path::Path,
    scope: &Scope,
    project: Option<&str>,
    start_ms: i64,
    end_ms: i64,
) -> AppResult<(Vec<(String, HourRow)>, bool)> {
    let now = juggler_core::model::Timestamp::now().millis();
    let mut rows = Vec::new();
    let mut cut = false;
    for profile in &scope.profiles {
        let (mine, was_cut) = settled::hourly(
            storage,
            data_dir,
            profile,
            start_ms,
            end_ms,
            MAX_ROWS_PER_PROFILE,
            now,
        )?;
        cut |= was_cut;
        rows.extend(
            mine.into_iter()
                .filter(|row| {
                    project.is_none_or(|only| {
                        row.session.as_ref().is_some_and(|session| {
                            scope
                                .lookups
                                .sessions
                                .get(&(profile.clone(), session.clone()))
                                .and_then(|facts| facts.project.as_deref())
                                == Some(only)
                        })
                    })
                })
                .map(|row| (profile.clone(), row)),
        );
    }
    Ok((rows, cut))
}

/// Prices a row at list prices, at its hour's rates.
fn pricer(
    catalog: &juggler_core::cost_ledger::PricingCatalog,
) -> impl Fn(Option<&str>, i64, TokenCounts) -> Option<f64> + '_ {
    move |model, at_ms, tokens| catalog.price_tokens(model?, at_ms, tokens)
}

/// The range's summary, every session included (the history's and the
/// sessions page's common work).
fn summarize(
    state: &Arc<AppState>,
    input: &UsageQueryInput,
) -> AppResult<(Folded, Folded, bool, Scope)> {
    checked(input)?;
    let scope = scope_of(state, input)?;
    let storage = lock_storage_arc(state)?;
    let catalog = crate::commands::pricing_catalog(&storage);
    let price = pricer(&catalog);
    let (start, end) = (input.edges_ms[0], input.edges_ms[input.edges_ms.len() - 1]);
    let project = input.project.as_deref();
    let (rows, cut) = rows_between(&storage, &state.data_dir, &scope, project, start, end)?;
    let now = fold::fold(
        &rows,
        &input.edges_ms,
        &input.group_by,
        &scope.lookups,
        &price,
    );
    // The same span just before: its totals for the tiles' change.
    let span = end - start;
    let (before, _) = rows_between(
        &storage,
        &state.data_dir,
        &scope,
        project,
        start - span,
        start,
    )?;
    let previous = fold::fold(
        &before,
        &[start - span, start],
        "profile",
        &scope.lookups,
        &price,
    );
    Ok((now, previous, cut, scope))
}

/// A range's usage, summarised for the charts.
///
/// # Errors
///
/// [`AppError::Validation`] for a range the screen can't ask for; storage
/// errors.
pub fn history_impl(state: &Arc<AppState>, input: &UsageQueryInput) -> AppResult<UsageHistoryInfo> {
    let (now, previous, truncated, scope) = summarize(state, input)?;
    let edges = &input.edges_ms;
    let (start, end) = (edges[0], edges[edges.len() - 1]);
    let top = input.top.unwrap_or(10).min(50) as usize;
    let switches = {
        let storage = lock_storage_arc(state)?;
        storage
            .usage_history()
            .switches_between(start, end, input.project.as_deref(), 200)?
            .into_iter()
            .filter(|row| {
                input
                    .profile
                    .as_deref()
                    .is_none_or(|only| row.from_account == only || row.to_account == only)
            })
            .map(|row| dto::UsageSwitchInfo {
                at_ms: row.at_ms,
                project_id: row.project,
                from_profile: row.from_account,
                to_profile: row.to_account,
                state: row.state,
            })
            .collect()
    };
    let in_range = |at: Option<i64>| at.is_some_and(|at| at >= start && at < end);
    Ok(UsageHistoryInfo {
        edges_ms: edges.clone(),
        group_by: input.group_by.clone(),
        session_count: u32::try_from(now.sessions.len()).unwrap_or(u32::MAX),
        top_sessions: now.sessions.into_iter().take(top).collect(),
        series: now.series,
        totals: now.totals,
        previous: previous.totals,
        by_profile: now.by_profile,
        by_tool: now.by_tool,
        by_model: now.by_model,
        by_project: now.by_project,
        sessions_per_bucket: now.sessions_per_bucket,
        started_per_bucket: fold::started_per_bucket(
            edges,
            scope.conversations.iter().filter_map(|c| c.0),
        ),
        active_hours: now.active_hours,
        switches,
        unmeasured_sessions: u32::try_from(
            scope
                .conversations
                .iter()
                .filter(|(_, last, measured)| !measured && in_range(*last))
                .count(),
        )
        .unwrap_or(u32::MAX),
        truncated,
        reading: state.usage_reading.so_far(),
    })
}

/// A page of the range's sessions, the biggest first (the shared keyset
/// paging: a session's tokens, then its last use, then its ids).
///
/// # Errors
///
/// As [`history_impl`]; [`AppError::Validation`] for a cursor this app
/// didn't make.
pub fn sessions_page_impl(
    state: &Arc<AppState>,
    input: &UsageQueryInput,
    request: &PageRequest,
) -> AppResult<Page<dto::UsageSessionInfo>> {
    let (now, _, _, _) = summarize(state, input)?;
    paging::page(
        now.sessions,
        |row| {
            SortKey::new(
                [-i64::try_from(row.io).unwrap_or(i64::MAX), -row.last_ms],
                format!("{}\u{0}{}", row.profile, row.id),
            )
        },
        request,
    )
}

/// The first usage in scope, epoch ms: where "All" starts.
///
/// # Errors
///
/// Storage errors.
pub fn first_impl(state: &Arc<AppState>, profile: Option<&str>) -> AppResult<Option<i64>> {
    let storage = lock_storage_arc(state)?;
    let history = storage.usage_history();
    let mut first: Option<i64> = None;
    for account in storage.accounts().list()? {
        let id = account.id.to_string();
        if profile.is_some_and(|only| only != id) {
            continue;
        }
        if let Some(at) = history.first_ts(&id)? {
            first = Some(first.map_or(at, |seen| seen.min(at)));
        }
    }
    Ok(first)
}

#[cfg(test)]
mod tests;
