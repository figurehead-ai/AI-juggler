//! D-62 decision 15 (AJ-E3006): sessions other apps run, as Watched
//! members. One is a conversation (its folder and id) named on its own in
//! a pool: shown on the canvas and in counts, in agents' rosters as
//! "can't reply"; no tools and no rules apply to it, and a message to it
//! waits with the reason. Continue in AI Juggler resumes it in a launch
//! that carries the tools; the new session then matches the member as
//! itself, so its membership carries on.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use juggler_core::storage::{MemberRef, Storage};

use super::dto::WatchedMemberInfo;
use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::state::AppState;

/// The conversations AI Juggler's own sessions carry, `folder/id`.
fn continued(storage: &Storage) -> AppResult<HashMap<String, String>> {
    Ok(storage
        .sessions()
        .pool_facts()?
        .into_iter()
        .filter_map(|facts| {
            let (folder, id) = facts.conversation?;
            Some((format!("{folder}/{id}"), facts.session.as_str().to_owned()))
        })
        .collect())
}

/// Each pool's Watched members that aren't continued yet, counted (they
/// count as members, D-62 decision 15).
///
/// # Errors
///
/// Storage failures.
pub fn counts(storage: &Storage) -> AppResult<HashMap<String, u32>> {
    let done: HashSet<String> = continued(storage)?.into_keys().collect();
    let mut out: HashMap<String, u32> = HashMap::new();
    for row in storage.pools().members()? {
        if let MemberRef::Conversation { folder, id } = &row.member
            && !done.contains(&format!("{folder}/{id}"))
        {
            *out.entry(row.pool_id).or_default() += 1;
        }
    }
    Ok(out)
}

/// A pool's Watched members, with what the conversation index says of
/// each (its title, tool, account), and the session that continued it in
/// AI Juggler when one did.
///
/// # Errors
///
/// [`crate::error::AppError::Refused`] for a pool that isn't there;
/// storage failures.
pub fn watched_impl(state: &Arc<AppState>, pool_id: &str) -> AppResult<Vec<WatchedMemberInfo>> {
    let (rows, done) = {
        let storage = lock_storage_arc(state)?;
        super::live_pool(&storage, pool_id)?;
        let rows: Vec<_> = storage
            .pools()
            .members()?
            .into_iter()
            .filter(|row| row.pool_id == pool_id && row.member.is_watched())
            .collect();
        (rows, continued(&storage)?)
    };
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let index: HashMap<(String, String), _> =
        crate::features::conversations::commands::list_indexed(state)?
            .into_iter()
            .map(|info| ((info.account_id.clone(), info.id.clone()), info))
            .collect();
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let MemberRef::Conversation { folder, id } = &row.member else {
                return None;
            };
            let info = index.get(&(folder.clone(), id.clone()));
            Some(WatchedMemberInfo {
                member: row.member.to_wire(),
                folder: folder.clone(),
                conversation_id: id.clone(),
                title: info.and_then(|info| info.title.clone()),
                tool: info.map(|info| info.tool.clone()),
                account_label: info.map(|info| info.account_label.clone()),
                project_id: info.and_then(|info| info.project_id.clone()),
                role: row.role.as_str().to_owned(),
                continued_session: done.get(&format!("{folder}/{id}")).cloned(),
            })
        })
        .collect())
}
