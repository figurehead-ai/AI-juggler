//! D-62 (P5 step 4): an App signs in to another Account. The App's open
//! sessions re-resolve: they leave the pools the old Account's member
//! held them in and join the new Account's. Each such pool says so in
//! its own event log, which its canvas's live stream reads ("«name» left:
//! its account is now Other"): the pools model's edge case "An App signs
//! in to another account".
//!
//! - [`set_folder`] points a folder at the Account its sign-in was read
//!   as, and works out who moved, under the storage lock the caller
//!   holds (nothing named yet: Account names are worked out after).
//! - [`tell`] writes each move into the pools' logs, by the names the
//!   session and the Account have now, and says the pools changed.
//! - Only open sessions are told: an App's ended ones are history, and a
//!   line for each would bury the pool's stream.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use juggler_core::model::SessionId;
use juggler_core::pools::{Memberships, SessionFacts};
use juggler_core::storage::{MEMBER_JOINED, MEMBER_LEFT, Storage};

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::state::AppState;

/// One open session a change of sign-in moved between pools.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moved {
    /// The session.
    pub session: SessionId,
    /// The Account it's on now (`None`: the sign-in can't be read).
    pub account: Option<String>,
    /// The pools it left.
    pub left: Vec<String>,
    /// The pools it joined.
    pub joined: Vec<String>,
}

/// Points folder `folder` at `account` (its sign-in as read now) and
/// says which open sessions that moved between pools; `None` when the
/// folder's Account didn't change. The folder is pointed even when who
/// moved can't be worked out (then no one is told).
///
/// # Errors
///
/// Storage failures pointing the folder.
pub fn set_folder(
    storage: &Storage,
    folder: &str,
    account: Option<&str>,
) -> AppResult<Option<Vec<Moved>>> {
    // Read on every Accounts read: the sessions' facts only when the
    // folder's Account changes, and some pool has an Account member (only
    // that can move a session on a sign-in change).
    let changes = storage
        .subscriptions()
        .folders()?
        .into_iter()
        .any(|(id, was)| id == folder && was.as_deref() != account);
    let before = changes
        .then(|| super::members::memberships(storage).ok())
        .flatten()
        .filter(|memberships| memberships.has_account_members())
        .and_then(|memberships| Some((memberships, super::members::facts(storage).ok()?)));
    if !storage.subscriptions().set_folder(folder, account)? {
        return Ok(None);
    }
    let Some((memberships, before)) = before else {
        return Ok(Some(Vec::new()));
    };
    Ok(Some(
        moved(storage, &memberships, &before).unwrap_or_default(),
    ))
}

/// The open sessions whose pools differ from `before`'s.
fn moved(
    storage: &Storage,
    memberships: &Memberships,
    before: &HashMap<SessionId, SessionFacts>,
) -> AppResult<Vec<Moved>> {
    let archived = super::members::archived(storage)?;
    let mut moved = Vec::new();
    for after in super::members::facts(storage)?.into_values() {
        let Some(was) = before.get(&after.session) else {
            continue;
        };
        if was.account == after.account || archived.contains(after.session.as_str()) {
            continue;
        }
        let pools = |facts| -> BTreeSet<String> {
            memberships
                .resolve(facts)
                .placements
                .into_iter()
                .map(|place| place.pool_id)
                .collect()
        };
        let (then, now) = (pools(was), pools(&after));
        if then == now {
            continue;
        }
        moved.push(Moved {
            session: after.session.clone(),
            account: after.account.clone(),
            left: then.difference(&now).cloned().collect(),
            joined: now.difference(&then).cloned().collect(),
        });
    }
    if moved.is_empty() {
        return Ok(moved);
    }
    // Open ones only.
    let ids: Vec<SessionId> = moved.iter().map(|m| m.session.clone()).collect();
    let open: BTreeSet<String> = storage
        .sessions()
        .get_many(&ids)?
        .into_iter()
        .filter(|record| crate::features::messages::is_live(record.status))
        .map(|record| record.id.as_str().to_owned())
        .collect();
    moved.retain(|m| open.contains(m.session.as_str()));
    moved.sort_by(|a, b| a.session.as_str().cmp(b.session.as_str()));
    Ok(moved)
}

/// Writes each move into the logs of the pools it left and joined:
/// `account_name` names an Account by id as the app does. The pools'
/// member lists and canvases hear the change.
pub fn tell(state: &Arc<AppState>, moved: &[Moved], account_name: impl Fn(&str) -> String) {
    if moved.is_empty() {
        return;
    }
    let names = session_names(state, moved);
    let Ok(storage) = lock_storage_arc(state) else {
        return;
    };
    let log = storage.messages();
    for m in moved {
        let session = format!("session:{}", m.session.as_str());
        let name = names
            .get(m.session.as_str())
            .cloned()
            .unwrap_or_else(|| "A session".to_owned());
        let account = m.account.as_deref().map(&account_name);
        let object = format!("account:{}", m.account.as_deref().unwrap_or_default());
        let detail = serde_json::json!([name, account]).to_string();
        for (pools, verb) in [(&m.left, MEMBER_LEFT), (&m.joined, MEMBER_JOINED)] {
            for pool in pools {
                if let Err(error) = log.log(pool, &session, verb, &object, Some(&detail)) {
                    tracing::warn!(%error, pool, verb, "pool membership line not written");
                }
            }
        }
    }
    drop(storage);
    super::super::relations::changed(state, super::super::relations::Change::PoolsChanged);
    super::super::relations::changed(state, super::super::relations::Change::MessagesChanged);
}

/// Each moved session's name, as its card says it.
fn session_names(state: &Arc<AppState>, moved: &[Moved]) -> HashMap<String, String> {
    let records = {
        let Ok(storage) = lock_storage_arc(state) else {
            return HashMap::new();
        };
        let ids: Vec<SessionId> = moved.iter().map(|m| m.session.clone()).collect();
        storage.sessions().get_many(&ids).unwrap_or_default()
    };
    crate::features::sessions::list::instances_of(state, records)
        .unwrap_or_default()
        .iter()
        .map(|row| (row.id.clone(), super::cards::name_of(row)))
        .collect()
}
