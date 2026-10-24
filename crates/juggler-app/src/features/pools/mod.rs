//! AJ-E1702: pools — the user groups sessions, names the groups, and says
//! who leads. AJ-E1703: each pool's context cards ([`cards`]).
//!
//! - The user owns every pool: these commands are the only way to change
//!   pools, members, roles and access levels, and only the app's UI calls
//!   them (D-28).
//!   Nothing an agent runs can reach them.
//! - "All sessions" always exists; it can't be renamed or deleted.
//! - A pool can be for one project; a session from elsewhere is refused.
//! - Pool operations only read and write the pool tables. They never
//!   start, stop or type into a session (`tests/pools.rs` holds that).

pub mod access;
pub mod cards;
pub mod commands;
pub mod dto;

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
pub use dto::{
    AccessInfo, AgentRuleInfo, CardField, ContextCard, PoolCards, PoolInfo, PoolMemberInfo,
    PoolsInfo,
};
use juggler_core::model::{AccountId, ProjectId, SessionId};
use juggler_core::storage::{
    DEFAULT_POOL_ID, DEFAULT_POOL_NAME, DefaultScope, MemberRef, PoolKind, PoolRole, Storage,
    default_pool_scope, resolve_memberships,
};

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
    if let Some(other) = storage
        .pools()
        .list()?
        .into_iter()
        .find(|pool| Some(pool.id.as_str()) != except && pool.name.to_lowercase() == lower)
    {
        return Err(AppError::Refused(format!(
            "There's already a pool called «{}» — pick another name",
            other.name
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

/// The default pool's scope of every session (D-33, through the one
/// function that decides it).
fn scopes_of(storage: &Storage) -> AppResult<HashMap<SessionId, DefaultScope>> {
    let mut accounts: HashMap<juggler_core::model::ProfileId, Option<AccountId>> = HashMap::new();
    let mut out = HashMap::new();
    for record in storage.sessions().list(None)? {
        let account = accounts
            .entry(record.profile.clone())
            .or_insert_with(|| {
                storage
                    .profiles()
                    .get(&record.profile)
                    .ok()
                    .map(|recipe| recipe.account)
            })
            .clone();
        let scope = default_pool_scope(record.project.as_ref(), account.as_ref());
        out.insert(record.id, scope);
    }
    Ok(out)
}

/// Builds the whole picture the UI shows.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
pub fn pools_info(storage: &Storage) -> AppResult<PoolsInfo> {
    let scopes = scopes_of(storage)?;
    let mut sessions: Vec<SessionId> = scopes.keys().cloned().collect();
    sessions.sort();
    let works = storage.switch_graph().works_by_session()?;
    let members = storage.pools().members()?;
    let places = resolve_memberships(&sessions, &members, &works);
    let repo = storage.pools();
    let mut pools = Vec::new();
    for pool in repo.list()? {
        let (access, rules) = access::info(storage, &pool.id)?;
        let groups: BTreeSet<&DefaultScope> = places
            .iter()
            .filter(|(_, (pool_id, _))| *pool_id == pool.id)
            .filter_map(|(session, _)| scopes.get(session))
            .collect();
        pools.push(PoolInfo {
            id: pool.id,
            name: pool.name,
            kind: pool.kind.as_str().to_owned(),
            project_id: pool.project.map(|project| project.as_str().to_owned()),
            access,
            rules,
            scopes: u32::try_from(groups.len()).unwrap_or(u32::MAX),
        });
    }
    let members = sessions
        .iter()
        .map(|session| {
            let (pool_id, role) = places
                .get(session)
                .cloned()
                .unwrap_or_else(|| (DEFAULT_POOL_ID.to_owned(), PoolRole::Member));
            PoolMemberInfo {
                session_id: session.as_str().to_owned(),
                pool_id,
                role: role.as_str().to_owned(),
            }
        })
        .collect();
    Ok(PoolsInfo { pools, members })
}

/// Every pool and every session's place.
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

/// Renames a pool ("All sessions" keeps its name).
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
            "«{DEFAULT_POOL_NAME}» keeps its name"
        )));
    }
    let name = checked_name(&storage, name, Some(pool_id))?;
    storage.pools().rename(pool_id, &name)?;
    pools_info(&storage)
}

/// Deletes a pool; its sessions go back to "All sessions". Nothing else
/// changes: the sessions keep running as they were.
///
/// # Errors
///
/// [`AppError::Refused`] for the default pool or a missing pool.
pub fn pool_delete_impl(state: &Arc<AppState>, pool_id: &str) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    let pool = live_pool(&storage, pool_id)?;
    if pool.kind == PoolKind::Default {
        return Err(AppError::Refused(format!(
            "«{DEFAULT_POOL_NAME}» can't be deleted — it holds every session that isn't in another pool"
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

/// Moves a session into a pool, as a member. A pool for one project
/// takes only that project's sessions.
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
            pool.name
        )));
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
    storage
        .sessions()
        .get(&session)
        .map_err(|_| AppError::Refused("That session isn't there any more".to_owned()))?;
    let info = pools_info(&storage)?;
    let pool_id = info
        .members
        .iter()
        .find(|row| row.session_id == session_id)
        .map_or_else(|| DEFAULT_POOL_ID.to_owned(), |row| row.pool_id.clone());
    let (member, replaces) = member_of(&storage, &session)?;
    storage.pools().place(&member, &replaces, &pool_id, role)?;
    pools_info(&storage)
}
