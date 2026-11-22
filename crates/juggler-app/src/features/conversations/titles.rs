//! AJ-E1536: AI Juggler's sessions named by their conversations' titles
//! (`titled`), wherever a session row shows: its own conversation's, or
//! the one a switch carried into it (SK-11).

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::AppResult;
use crate::state::AppState;

/// Every session's row as the store has it now, titled ([`titled`]);
/// none when a lock is poisoned.
#[must_use]
pub fn titled_snapshot(state: &Arc<AppState>) -> Vec<crate::features::sessions::dto::InstanceInfo> {
    let rows = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| {
            crate::features::sessions::read_model::session_infos_snapshot(&storage).ok()
        })
        .unwrap_or_default();
    titled(state, rows).unwrap_or_default()
}

/// AJ-E1536: AI Juggler's sessions with their conversations' titles (the
/// tool's summary, else the first thing asked), so a session reads as
/// what it was about wherever it shows, as one started outside does.
///
/// # Errors
///
/// [`crate::error::AppError::StatePoisoned`] when the index's lock is poisoned.
pub fn titled(
    state: &Arc<AppState>,
    mut rows: Vec<crate::features::sessions::dto::InstanceInfo>,
) -> AppResult<Vec<crate::features::sessions::dto::InstanceInfo>> {
    // AJ-E805: only these rows' conversations (Needs you reads the rows
    // every 5 s; this copied every conversation on the Mac each time).
    let titles: HashMap<(String, String), String> = {
        let wanted: std::collections::HashSet<(&str, &str)> = rows
            .iter()
            .flat_map(|row| [own_conversation(row), carried_conversation(row)])
            .flatten()
            .collect();
        state
            .conversations
            .all_where(|c| wanted.contains(&(c.account_id.as_str(), c.summary.id.as_str())))?
            .into_iter()
            .filter_map(|c| Some(((c.account_id, c.summary.id), c.summary.title?)))
            .collect()
    };
    let title = |key: Option<(&str, &str)>| {
        key.and_then(|(account, id)| titles.get(&(account.to_owned(), id.to_owned())).cloned())
    };
    // Over the title the row came with (its task's, `sessions::list`), only
    // when its own conversation, or the one a switch carried in (SK-11),
    // has one: an unread index keeps the task's.
    for row in &mut rows {
        let (own, carried) = (own_conversation(row), carried_conversation(row));
        if let Some(found) = title(own).or_else(|| title(carried)) {
            row.title = Some(found);
        }
    }
    Ok(rows)
}

/// A session row's own conversation: `(account, id)`.
fn own_conversation(row: &crate::features::sessions::dto::InstanceInfo) -> Option<(&str, &str)> {
    Some((row.account_id.as_deref()?, row.conversation_id.as_deref()?))
}

/// SK-11: the conversation a switch carried into a session row. Its copy
/// is the same conversation, so until the copy is read the row goes by
/// that one's title (never by its facts: "web · started 9:40 AM").
fn carried_conversation(
    row: &crate::features::sessions::dto::InstanceInfo,
) -> Option<(&str, &str)> {
    let from = row.continues.as_ref()?;
    Some((from.account_id.as_str(), from.conversation_id.as_deref()?))
}
