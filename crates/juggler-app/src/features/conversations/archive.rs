//! 0.4.1 (main's ruling on the rc7 walk): AI Juggler's own Archive for a
//! session another app holds. AI Juggler never writes another app's
//! records, so archiving one of its ended sessions hides it here only
//! (the rail, the App's list; History lists it under Archived) and the
//! app still lists it. No adapter offers a sanctioned way to archive in
//! the app itself yet; when one does, an open app's session goes that way.
//!
//! Kept as one setting: the archived conversations' keys
//! (`<account>/<conversation>`), at most 5,000, the oldest dropped.

use std::collections::HashSet;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The setting holding the keys.
const KEY: &str = "conversations.archived_here";
/// The most kept.
const MAX: usize = 5_000;

/// A conversation's key in the set.
#[must_use]
pub fn key_of(account: &str, conversation: &str) -> String {
    format!("{account}/{conversation}")
}

/// The archived conversations' keys (empty when unreadable).
#[must_use]
pub fn archived(state: &Arc<AppState>) -> HashSet<String> {
    read(state).into_iter().collect()
}

fn read(state: &Arc<AppState>) -> Vec<String> {
    let Ok(storage) = state.storage.lock() else {
        return Vec::new();
    };
    storage
        .app_settings()
        .get(KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .unwrap_or_default()
}

/// Archives (or unarchives) a conversation here. One that's open isn't
/// archived: it's still working, so the words say why.
///
/// # Errors
///
/// [`AppError::Refused`] for an open one; storage failures.
pub fn set(state: &Arc<AppState>, account: &str, conversation: &str, on: bool) -> AppResult<()> {
    let open = state
        .conversations
        .all()
        .ok()
        .and_then(|all| {
            all.into_iter()
                .find(|c| c.account_id == account && c.summary.id == conversation)
        })
        .and_then(|c| {
            state
                .conversations
                .live_entry(&c.account_id, &c.tool, &c.summary.id)
        })
        .is_some();
    if on && open {
        return Err(AppError::Refused(
            "It's open in its app: archive it once it has ended.".to_owned(),
        ));
    }
    let key = key_of(account, conversation);
    let mut keys = read(state);
    keys.retain(|kept| *kept != key);
    if on {
        keys.push(key);
    }
    if keys.len() > MAX {
        let extra = keys.len() - MAX;
        keys.drain(..extra);
    }
    let raw = serde_json::to_string(&keys).map_err(|e| AppError::Refused(e.to_string()))?;
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage.app_settings().set(KEY, &raw)?;
    }
    // The counts (History's "Archived (N)") are read again.
    crate::features::relations::changed(
        state,
        crate::features::relations::Change::ConversationsLive,
    );
    // Its Needs you row leaves (or comes back) at once, not on a later
    // tick.
    crate::features::notifications::needs::refresh(state);
    Ok(())
}

/// Archives or unarchives a session another app holds, here only; the
/// lists hear it at once.
///
/// # Errors
///
/// As [`set`].
#[tauri::command]
pub async fn conversations_archive(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
    archived: bool,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the archive", move |state| {
        set(state, &account_id, &conversation_id, archived)?;
        crate::events::EmitEvent::emit_event(
            &app,
            crate::events::AppEvent::ConversationsChanged,
            super::CHANGED_INDEX,
        );
        Ok(())
    })
    .await
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a test (QUALITY.md §2)")]
    use super::{archived, key_of, set};

    #[test]
    fn an_ended_session_is_archived_here_and_back_and_the_app_is_never_written() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::bare_state(dir.path().join("data"));
        assert!(archived(&state).is_empty());
        set(&state, "acc", "c-1", true).unwrap();
        set(&state, "acc", "c-1", true).unwrap();
        set(&state, "acc", "c-2", true).unwrap();
        assert_eq!(archived(&state).len(), 2, "each once");
        set(&state, "acc", "c-1", false).unwrap();
        assert_eq!(
            archived(&state),
            std::collections::HashSet::from([key_of("acc", "c-2")])
        );
    }
}
