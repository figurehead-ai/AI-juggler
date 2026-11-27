//! 0.5.2 G42, G44: sessions named by the one rule (the rail's row,
//! [`crate::features::notifications::needs::session_name`] over
//! `naming::short_name`) for a reader the window may not hold them for:
//! the feed's posts and the Reminders list. Their rows are made as the
//! rail makes its own ([`super::list::stored_rows`], their tasks' and
//! conversations' titles), so a post or a reminder says the name the rail
//! says, never a second rule (label, else the folder).

use std::collections::HashMap;
use std::sync::Arc;

use juggler_core::model::SessionId;

use super::read_model::LiveStatsMap;
use crate::commands::lock_storage_arc;
use crate::features::notifications::needs::session_name;
use crate::state::AppState;

/// The name of each of `ids` that storage holds, by id, as the rail
/// names it at `now_ms`; none for an id it doesn't hold or on a failed
/// read (the caller keeps what it had). Takes the storage lock for a
/// moment, then the conversation index's, never both; the caller holds
/// neither.
#[must_use]
pub fn names_of(state: &Arc<AppState>, ids: &[String], now_ms: i64) -> HashMap<String, String> {
    if ids.is_empty() {
        return HashMap::new();
    }
    // 0.5.2 G57 (RW1 F-RW2-1): a session AI Juggler didn't start (a Codex
    // thread), by its key, named as its rail row names it (G45).
    let (outside, own): (Vec<&String>, Vec<&String>) = ids
        .iter()
        .partition(|id| super::seen::parse_source_key(id).is_some());
    let mut out: HashMap<String, String> = if outside.is_empty() {
        HashMap::new()
    } else {
        super::seen::outside_of(state, outside.iter().map(|id| id.as_str()))
            .into_iter()
            .map(|(key, row)| {
                (
                    key,
                    crate::features::notifications::needs::conversation_name(&row, now_ms),
                )
            })
            .collect()
    };
    if own.is_empty() {
        return out;
    }
    out.extend(own_names(state, &own, now_ms));
    out
}

/// [`names_of`] for AI Juggler's own sessions.
fn own_names(state: &Arc<AppState>, ids: &[&String], now_ms: i64) -> HashMap<String, String> {
    let mut wanted: Vec<SessionId> = ids.iter().map(|id| SessionId::new(id.as_str())).collect();
    wanted.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    wanted.dedup();
    let Ok(records) = lock_storage_arc(state).and_then(|storage| {
        storage
            .sessions()
            .get_many(&wanted)
            .map_err(crate::error::AppError::from)
    }) else {
        return HashMap::new();
    };
    let Ok(rows) = super::list::stored_rows(state, records, &LiveStatsMap::new()) else {
        return HashMap::new();
    };
    let rows = crate::features::conversations::titled(state, rows).unwrap_or_default();
    rows.iter()
        .map(|row| (row.id.clone(), session_name(row, now_ms)))
        .collect()
}
