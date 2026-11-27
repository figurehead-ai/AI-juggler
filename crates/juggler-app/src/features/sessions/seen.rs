//! 0.5.2 G45 (RW1 W1, W3): every session AI Juggler sees, wherever it was
//! started, as one read for every reader that isn't the window: the
//! watches (the picker's targets, a group target's open sessions, the
//! observations about one) and Bot's look-up. The window's own read is the
//! same two lists (`lib/rail-items`' `itemsOf`): AI Juggler's sessions
//! (the sessions read model) and the sessions it didn't start (the
//! conversation index's rows with no AI Juggler session on them).
//!
//! A session AI Juggler didn't start (a `claude` in the user's own
//! terminal, a Codex thread, a Claude app's Code session) is a Session of
//! its App (the entity model): seeing it run adds nothing to the user's
//! Apps (D-84, "No automatic adding"), and its App is the one presence
//! decides (ADR-0032: `ConversationInfo::app`); an App the user never
//! added is only named, never stored. It stands where a session id goes
//! by its key, `conversation:<account>:<id>`
//! ([`crate::features::conversations::switch_source::source_key`]), as the
//! switch and the window already read it.

use std::collections::HashMap;
use std::sync::Arc;

pub use crate::features::conversations::switch_source::{parse_source_key, source_key};

use crate::features::conversations::dto::ConversationInfo;
use crate::state::AppState;

/// The most sessions AI Juggler didn't start that one read gives (the
/// open ones always, then the newest).
pub const MOST: usize = 200;

/// The key a session AI Juggler didn't start stands by where a session id
/// goes.
#[must_use]
pub fn key_of(row: &ConversationInfo) -> String {
    source_key(&row.account_id, &row.id)
}

/// A conversation the window lists as a session of its own, by the rail's
/// rule (`itemsOf`, `conversationItem`): no AI Juggler session runs it
/// (that session's row is it), and neither AI Juggler, its app nor its
/// App's records put it away (archived, or a script's run, D-102).
#[must_use]
pub fn listed(row: &ConversationInfo) -> bool {
    row.session_id.is_none() && !row.app_archived && !row.archived_here && !row.app_unlisted
}

/// Open now, by the rail's rule (`openNow`): a process runs it, and not
/// as a headless run.
#[must_use]
pub const fn open(row: &ConversationInfo) -> bool {
    row.live && !row.background
}

/// The sessions AI Juggler didn't start that the window's working set
/// holds (open, idle in an app, or active this week), the open ones
/// first, then the newest, at most [`MOST`]. Takes the storage lock for a
/// moment; the caller holds none.
#[must_use]
pub fn outside(state: &Arc<AppState>, now_ms: i64) -> Vec<ConversationInfo> {
    let mut rows: Vec<ConversationInfo> =
        crate::features::conversations::commands::working_impl(state, now_ms)
            .unwrap_or_default()
            .into_iter()
            .filter(listed)
            .collect();
    rows.sort_by_key(|row| {
        (
            std::cmp::Reverse(open(row)),
            std::cmp::Reverse(row.last_activity_ms.or(row.started_ms).unwrap_or(0)),
        )
    });
    rows.truncate(MOST);
    rows
}

/// The sessions AI Juggler didn't start that are open now (the live ones
/// only: a few, never the working set). Takes the storage lock for a
/// moment; the caller holds none.
#[must_use]
pub fn outside_open(state: &Arc<AppState>) -> Vec<ConversationInfo> {
    crate::features::conversations::commands::live_rows(state)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| listed(row) && open(row))
        .collect()
}

/// The sessions AI Juggler didn't start among `ids` (each a key; any
/// other id is AI Juggler's own and left out), by key, open or not. Takes
/// the storage lock for a moment; the caller holds none.
#[must_use]
pub fn outside_of<'a>(
    state: &Arc<AppState>,
    ids: impl IntoIterator<Item = &'a str>,
) -> HashMap<String, ConversationInfo> {
    let keys: Vec<(String, String)> = ids
        .into_iter()
        .filter_map(parse_source_key)
        .map(|(account, id)| (account.to_owned(), id.to_owned()))
        .collect();
    crate::features::conversations::commands::rows_by_key(state, &keys)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.session_id.is_none())
        .map(|row| (key_of(&row), row))
        .collect()
}
