//! F94 (the AJ-E2317 class): the App a switch's source is in, by its id
//! (ADR-0032: `app:<host>:<instance folder>` or `cli:<folder>`), decided
//! where every list decides it. A conversation of a desktop App's Code tab
//! is that desktop App's, never its folder's CLI App found again from the
//! account: the switch dialog names the source by it, and the targets
//! leave out only that App (each App a row by its own id).

use std::sync::Arc;

use juggler_core::model::{Account, InstanceKind, InstanceRecord};

use crate::features::conversations::switch_source::parse_source_key;
use crate::state::AppState;

/// A switch's source App: its id and its name, as the rail says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceApp {
    /// Its App id.
    pub id: String,
    /// "Claude app · Work", "Claude Code CLI · Work".
    pub name: String,
}

/// The App `session_id` (an AI Juggler session, or a conversation's source
/// key) is in, `source` and `account` being its row and folder.
#[must_use]
pub(crate) fn source_app_of(
    state: &Arc<AppState>,
    session_id: &str,
    source: &InstanceRecord,
    account: &Account,
) -> SourceApp {
    let folder = account.id.as_str();
    let id = match parse_source_key(session_id) {
        // Another app's conversation: the App its row is under.
        Some((account_id, id)) => {
            crate::features::conversations::commands::app_of_conversation(state, account_id, id)
                .unwrap_or_else(|_| juggler_core::storage::cli_app_id(folder))
        }
        // AI Juggler's own: the desktop App it launched in, else its CLI App.
        None => {
            let launched = state
                .storage
                .lock()
                .ok()
                .and_then(|storage| storage.apps().session_apps().ok())
                .and_then(|apps| apps.get(session_id).cloned());
            crate::features::places::app_of(
                &crate::features::places::desktop_apps(state),
                launched.as_deref(),
                Some(folder),
                None,
                source.kind != InstanceKind::Desktop,
            )
        }
    };
    let name = name_of(state, &id, account);
    SourceApp { id, name }
}

/// App `id`'s name as the rail says it: a desktop App's identity, else its
/// folder's CLI App ("Claude Code CLI · Work").
fn name_of(state: &Arc<AppState>, id: &str, account: &Account) -> String {
    id.starts_with("app:")
        .then(|| crate::features::places::commands::identities(state).remove(id))
        .flatten()
        .map_or_else(
            || {
                crate::features::presence::cli_label(
                    &crate::features::tools::display_name(
                        &state.registry,
                        account.harness.as_str(),
                    ),
                    &account.label,
                )
            },
            |identity| identity.label,
        )
}

#[cfg(test)]
#[path = "source_app_tests.rs"]
mod tests;
