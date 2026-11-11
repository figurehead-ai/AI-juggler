//! The board's session list and stopping a session.

use std::sync::Arc;

use juggler_core::model::SessionRecord;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::sessions::dto::InstanceInfo;
use crate::features::sessions::read_model::{
    LiveStatsMap, desktop_isolation_labels, session_infos_for, working_records,
};
use crate::state::AppState;

/// The read model with the LAST sampled live stats (no extra collector
/// refresh, so the 2 s sampler's CPU deltas stay honest) — what this
/// Mac shares with paired consoles over the Fleet link (AJ-E705).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on storage failures.
pub fn instances_shared_snapshot(state: &Arc<AppState>) -> AppResult<Vec<InstanceInfo>> {
    let records = working_records(&*lock_storage_arc(state)?)?;
    instances_of(state, records)
}

/// The rows of `records` as [`sessions_list`] gives them (from the last
/// resource sample, never a new one): a page of sessions.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on storage failures.
pub(crate) fn instances_of(
    state: &Arc<AppState>,
    records: Vec<SessionRecord>,
) -> AppResult<Vec<InstanceInfo>> {
    let live: LiveStatsMap = state
        .resources
        .lock()
        .map(|collector| {
            collector
                .stats_snapshot()
                .into_iter()
                .map(|(session, stats)| (session.to_string(), stats))
                .collect()
        })
        .unwrap_or_default();
    rows_with(state, records, &live)
}

/// The rows of `records` with `live` stats: counted subagents, the
/// runtime's words, the conversation's title and its resume refusal
/// (rc21) on each.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on
/// storage failures.
pub(crate) fn rows_with(
    state: &Arc<AppState>,
    records: Vec<SessionRecord>,
    live: &LiveStatsMap,
) -> AppResult<Vec<InstanceInfo>> {
    let labels = desktop_isolation_labels(&state.registry);
    let adopted = state.supervisor.alive_adopted_sessions();
    let storage = lock_storage_arc(state)?;
    let rows = session_infos_for(&storage, records, live, &labels, &adopted)?;
    drop(storage);
    let rows = crate::features::subagents::count::counted(state, rows);
    let rows = crate::features::runtime::described(state, rows);
    let rows = crate::features::launch::resume_gate::with_refusals(state, rows);
    crate::features::conversations::titled(state, rows)
}
