//! AJ-E1702: pools — the user groups sessions, names the groups, and says
//! who leads. AJ-E1703: each pool's context cards ([`cards`]).
//!
//! - The user owns every pool: these commands are the only way to change
//!   pools, members, roles and access levels, and only the app's UI calls
//!   them (D-28).
//!   Nothing an agent runs can reach them.
//! - D-33: each project has its own default pool, "`<project>` · all
//!   sessions", holding every session of that project on any account;
//!   sessions with no project are in "No project · all sessions". A
//!   default pool always exists and can't be renamed or deleted, so its
//!   level, rules and hop limit are set per project.
//! - A pool can be for one project; a session from elsewhere is refused.
//! - Pool operations only read and write the pool tables. They never
//!   start, stop or type into a session (`tests/pools.rs` holds that).
//! - What they answer never grows with the history (I6): the pools with
//!   their counts and the working set's places; a pool's members come a
//!   page at a time ([`members`]).

pub mod access;
pub mod cards;
pub mod commands;
pub mod dto;
pub mod members;

use std::collections::HashMap;
use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
pub use dto::{
    AccessInfo, AgentRuleInfo, CardField, ContextCard, PoolCards, PoolInfo, PoolMemberInfo,
    PoolMemberRow, PoolsInfo,
};
use juggler_core::model::{ProjectId, SessionId};
use juggler_core::storage::{MemberRef, Pool, PoolKind, PoolRole, Storage, default_pool_name};

/// The longest pool name accepted (a name, not a note).
const NAME_MAX: usize = 48;

/// A pool name, trimmed, or a refusal in words. `except` is the pool
/// being renamed (its own name is fine).
fn checked_name(storage: &Storage, name: &str, except: Option<&str>) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Refused("Give the pool a name".to_owned()));
    }
    if name.chars().count() > NAME_MAX {
        return Err(AppError::Refused(format!(
            "A pool's name is at most {NAME_MAX} characters"
        )));
    }
    let lower = name.to_lowercase();
    let projects = project_names(storage)?;
    if let Some(other) = storage
        .pools()
        .list()?
        .into_iter()
        .map(|pool| (pool.id.clone(), shown_name(&pool, &projects)))
        .find(|(id, shown)| Some(id.as_str()) != except && shown.to_lowercase() == lower)
    {
        return Err(AppError::Refused(format!(
            "There's already a pool called «{}» — pick another name",
            other.1
        )));
    }
    Ok(name.to_owned())
}

/// A pool that isn't deleted, or a refusal in words.
fn live_pool(storage: &Storage, pool_id: &str) -> AppResult<juggler_core::storage::Pool> {
    storage
        .pools()
        .get(pool_id)
        .map_err(|_| AppError::Refused("That pool isn't there any more".to_owned()))
}

/// Each project's name as it is now, by id.
fn project_names(storage: &Storage) -> AppResult<HashMap<String, String>> {
    Ok(storage
        .projects()
        .list()?
        .into_iter()
        .map(|project| (project.id.as_str().to_owned(), project.name))
        .collect())
}

/// A pool's name on screen: the user's for theirs; for a default pool,
/// its project's name now (a renamed project renames it) and what it is.
fn shown_name(pool: &Pool, projects: &HashMap<String, String>) -> String {
    match pool.kind {
        PoolKind::User => pool.name.clone(),
        PoolKind::Default => default_pool_name(pool.project.as_ref().map(|project| {
            projects
                .get(project.as_str())
                .map_or(pool.name.as_str(), String::as_str)
        })),
    }
}

/// The pools that aren't deleted, each with its name on screen, in the
/// UI's order: the user's (oldest first), then each project's default
/// pool (by name, "No project" last). Never reads a session: what looks
/// a pool up by its id (its name, its project) reads this.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
pub fn shown_pools(storage: &Storage) -> AppResult<Vec<(Pool, String)>> {
    let projects = project_names(storage)?;
    let mut listed: Vec<(Pool, String)> = storage
        .pools()
        .list()?
        .into_iter()
        .map(|pool| {
            let name = shown_name(&pool, &projects);
            (pool, name)
        })
        .collect();
    listed.sort_by_key(|(pool, name)| match pool.kind {
        PoolKind::User => (0, pool.created_at, String::new()),
        PoolKind::Default if pool.project.is_none() => (2, 0, String::new()),
        PoolKind::Default => (1, 0, name.to_lowercase()),
    });
    Ok(listed)
}

/// One pool that isn't deleted, with its name on screen.
///
/// # Errors
///
/// [`AppError::Refused`] for a pool that isn't there; storage failures.
pub fn named_pool(storage: &Storage, pool_id: &str) -> AppResult<(Pool, String)> {
    let pool = live_pool(storage, pool_id)?;
    let name = shown_name(&pool, &project_names(storage)?);
    Ok((pool, name))
}

/// Each pool's name on screen, by id.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
pub fn pool_names(storage: &Storage) -> AppResult<HashMap<String, String>> {
    Ok(shown_pools(storage)?
        .into_iter()
        .map(|(pool, name)| (pool.id, name))
        .collect())
}

/// Builds what the UI holds of the pools: every pool with its level,
/// rules and how many sessions it holds, and the places of the working
/// set's sessions that aren't their project's default (bounded by what
/// the rail holds, never the whole history: [`members`]).
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
pub fn pools_info(storage: &Storage) -> AppResult<PoolsInfo> {
    let memberships = members::memberships(storage)?;
    let tally = members::Tally::read(storage, &memberships)?;
    let mut pools = Vec::new();
    for (pool, name) in shown_pools(storage)? {
        let (access, rules) = access::info(storage, &pool.id)?;
        pools.push(PoolInfo {
            name,
            // How many projects its sessions come from (D-33).
            scopes: tally.scopes(&pool.id),
            members: tally.members(&pool.id),
            kind: pool.kind.as_str().to_owned(),
            project_id: pool.project.map(|project| project.as_str().to_owned()),
            access,
            rules,
            spans_macs: pool.spans_macs,
            id: pool.id,
        });
    }
    Ok(PoolsInfo {
        pools,
        placements: members::placements(storage, &memberships)?,
    })
}

/// Every pool, and the working set's places.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
pub fn pools_list_impl(state: &Arc<AppState>) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    pools_info(&storage)
}

/// Makes a pool, optionally for one project.
///
/// # Errors
///
/// [`AppError::Refused`] for an empty, too long or taken name, or an
/// unknown project; storage failures otherwise.
pub fn pool_create_impl(
    state: &Arc<AppState>,
    name: &str,
    project_id: Option<&str>,
) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    let name = checked_name(&storage, name, None)?;
    let project = match project_id {
        Some(id) => {
            let project = ProjectId::new(id);
            storage
                .projects()
                .get(&project)
                .map_err(|_| AppError::Refused("That project isn't there any more".to_owned()))?;
            Some(project)
        }
        None => None,
    };
    storage.pools().create(&name, project.as_ref())?;
    pools_info(&storage)
}

/// Renames a pool (a project's default pool keeps its name: the
/// project's).
///
/// # Errors
///
/// [`AppError::Refused`] for the default pool, a bad name or a missing
/// pool.
pub fn pool_rename_impl(state: &Arc<AppState>, pool_id: &str, name: &str) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    let pool = live_pool(&storage, pool_id)?;
    if pool.kind == PoolKind::Default {
        return Err(AppError::Refused(format!(
            "«{}» keeps its name: rename its project instead",
            shown_name(&pool, &project_names(&storage)?)
        )));
    }
    let name = checked_name(&storage, name, Some(pool_id))?;
    storage.pools().rename(pool_id, &name)?;
    pools_info(&storage)
}

/// Deletes a pool; its sessions go back to their projects' default
/// pools. Nothing else changes: the sessions keep running as they were.
///
/// # Errors
///
/// [`AppError::Refused`] for the default pool or a missing pool.
pub fn pool_delete_impl(state: &Arc<AppState>, pool_id: &str) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    let pool = live_pool(&storage, pool_id)?;
    if pool.kind == PoolKind::Default {
        return Err(AppError::Refused(format!(
            "«{}» can't be deleted — it holds every session of its project that isn't in another pool",
            shown_name(&pool, &project_names(&storage)?)
        )));
    }
    storage.pools().delete(pool_id)?;
    pools_info(&storage)
}

/// The session's member ref now, and every active row that stands for
/// the same member (its own, a row from before it switched).
fn member_of(storage: &Storage, session: &SessionId) -> AppResult<(MemberRef, Vec<MemberRef>)> {
    let works = storage.switch_graph().works_by_session()?;
    let member = MemberRef::of_session(session, works.get(session));
    let replaces = storage
        .pools()
        .members()?
        .into_iter()
        .map(|row| row.member)
        .filter(|row| row.anchor(&works) == member)
        .collect();
    Ok((member, replaces))
}

/// Moves a session into a pool, as a member. A pool for one project (a
/// project's default pool too) takes only that project's sessions; the
/// no-project default pool only sessions with no project.
///
/// # Errors
///
/// [`AppError::Refused`] for a session from another project;
/// [`AppError::Refused`] for a missing session or pool too.
pub fn pool_move_impl(
    state: &Arc<AppState>,
    session_id: &str,
    pool_id: &str,
) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    let session = SessionId::new(session_id);
    let record = storage
        .sessions()
        .get(&session)
        .map_err(|_| AppError::Refused("That session isn't there any more".to_owned()))?;
    let pool = live_pool(&storage, pool_id)?;
    if let Some(project) = &pool.project
        && record.project.as_ref() != Some(project)
    {
        let name = storage
            .projects()
            .get(project)
            .map_or_else(|_| "one project".to_owned(), |row| row.name);
        return Err(AppError::Refused(format!(
            "«{}» is for {name} only — this session works somewhere else",
            shown_name(&pool, &project_names(&storage)?)
        )));
    }
    if pool.kind == PoolKind::Default && pool.project.is_none() && record.project.is_some() {
        return Err(AppError::Refused(
            "«No project · all sessions» is for sessions with no project — this one has one"
                .to_owned(),
        ));
    }
    let (member, replaces) = member_of(&storage, &session)?;
    storage
        .pools()
        .place(&member, &replaces, &pool.id, PoolRole::Member)?;
    pools_info(&storage)
}

/// Sets a session's role in the pool it's in.
///
/// # Errors
///
/// [`AppError::Validation`] for an unknown role; [`AppError::Refused`]
/// for a missing session.
pub fn pool_set_role_impl(
    state: &Arc<AppState>,
    session_id: &str,
    role: &str,
) -> AppResult<PoolsInfo> {
    let role = PoolRole::parse(role).ok_or_else(|| AppError::Validation {
        subject: "role",
        message: format!("{role} isn't a role (lead, member or observer)"),
    })?;
    let storage = lock_storage_arc(state)?;
    let session = SessionId::new(session_id);
    let (pool_id, _) = members::place_of(&storage, &session)?
        .ok_or_else(|| AppError::Refused("That session isn't there any more".to_owned()))?;
    let (member, replaces) = member_of(&storage, &session)?;
    storage.pools().place(&member, &replaces, &pool_id, role)?;
    pools_info(&storage)
}

/// D-32: lets a pool span the user's paired Macs, or not.
///
/// # Errors
///
/// [`AppError::Refused`] for a missing pool.
pub fn pool_set_spans_macs_impl(
    state: &Arc<AppState>,
    pool_id: &str,
    spans: bool,
) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    live_pool(&storage, pool_id)?;
    storage.pools().set_spans_macs(pool_id, spans)?;
    pools_info(&storage)
}
