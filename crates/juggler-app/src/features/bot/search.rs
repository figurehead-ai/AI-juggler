//! Search's records (AJ-E3824, D-144): what ⌘K finds in AI Juggler's
//! records for what's typed — sessions, tasks, their pools and usage —
//! through Bot's one look-up ([`super::lookup::find_records`]), never a
//! copy of it. Nothing is written: no turn, no memory, no log.
//!
//! - **Only a real ask:** words that name nothing (a greeting, "can you",
//!   one letter) find no records, so a few letters never list everything.
//! - **Only matches:** "I don't know" (a subject nothing holds, something
//!   AI Juggler doesn't record) finds nothing here; Search says so itself.
//! - **Each record once:** a pool is listed once however many of its
//!   tasks matched.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::dto::{BotRowInfo, BotRowKind};
use super::lookup::{Found, look_up};
use super::question::{Reading, When, read};
use super::world::World;
use crate::error::AppResult;
use crate::state::AppState;

/// A pool some found tasks belong to (its page is the pool's board).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SearchPoolInfo {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// How many of the found tasks are in it.
    pub tasks: u32,
}

/// What Search found in the records, by kind, best first.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SearchRecordsInfo {
    /// Sessions (their page).
    pub sessions: Vec<BotRowInfo>,
    /// Tasks (their pool's board).
    pub tasks: Vec<BotRowInfo>,
    /// The found tasks' pools, once each.
    pub pools: Vec<SearchPoolInfo>,
    /// Accounts with their usage (their page).
    pub usage: Vec<BotRowInfo>,
}

/// Whether the words ask for records at all: a name, a state, a time or a
/// kind ("tasks", "usage"); not a greeting, an order or "remember".
fn asks_for_records(reading: &Reading) -> bool {
    let names_something = !reading.subject.is_empty()
        || !reading.states.is_empty()
        || reading.when != When::Any
        || reading.kinds.any()
        || reading.most_room;
    names_something
        && !reading.help
        && !reading.out_of_scope
        && reading.unknown.is_none()
        && reading.remember.is_none()
}

/// The look-up's rows by kind; nothing when it doesn't know.
fn by_kind(found: Found) -> SearchRecordsInfo {
    if found.unknown.is_some() {
        return SearchRecordsInfo::default();
    }
    let mut out = SearchRecordsInfo::default();
    for row in found.rows {
        match row.kind {
            BotRowKind::Session => out.sessions.push(row),
            BotRowKind::Account => out.usage.push(row),
            BotRowKind::Task => {
                if let (Some(id), Some(name)) = (row.pool_id.as_ref(), row.pool.as_ref()) {
                    match out.pools.iter_mut().find(|pool| &pool.id == id) {
                        Some(pool) => pool.tasks += 1,
                        None => out.pools.push(SearchPoolInfo {
                            id: id.clone(),
                            name: name.clone(),
                            tasks: 1,
                        }),
                    }
                }
                out.tasks.push(row);
            }
        }
    }
    out
}

/// What `text` finds in `world` (pure; the fixtures' way in).
#[must_use]
pub fn records_in(text: &str, world: &World) -> SearchRecordsInfo {
    let reading = read(text);
    if !asks_for_records(&reading) {
        return SearchRecordsInfo::default();
    }
    by_kind(look_up(&reading, world, None))
}

/// What `text` finds in the records now, through Bot's look-up.
///
/// # Errors
///
/// The records can't be read.
pub fn search_records_impl(state: &Arc<AppState>, text: &str) -> AppResult<SearchRecordsInfo> {
    if !asks_for_records(&read(text)) {
        return Ok(SearchRecordsInfo::default());
    }
    Ok(by_kind(super::lookup::find_records(state, text, None)?))
}

/// Search's records for what's typed (see [`search_records_impl`]).
///
/// # Errors
///
/// As [`search_records_impl`].
#[tauri::command]
pub async fn search_records(
    state: tauri::State<'_, Arc<AppState>>,
    text: String,
) -> AppResult<SearchRecordsInfo> {
    crate::ipc::blocking(state.inner(), "Search", move |state| {
        search_records_impl(state, &text)
    })
    .await
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
