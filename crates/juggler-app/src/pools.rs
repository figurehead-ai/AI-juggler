//! AJ-E1702: pools — the user groups sessions, names the groups, and says
//! who leads.
//!
//! - The user owns every pool: these commands are the only way to change
//!   pools, members, roles and grants, and only the app's UI calls them.
//!   Nothing an agent runs can reach them.
//! - "All sessions" always exists; it can't be renamed or deleted.
//! - A pool can be for one project; a session from elsewhere is refused.
//! - Pool operations only read and write the pool tables. They never
//!   start, stop or type into a session (`tests/pools.rs` holds that).

// Tauri command arguments arrive owned (deserialized).
#![allow(clippy::needless_pass_by_value)]

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use juggler_core::model::{AccountId, ProjectId, SessionId};
use juggler_core::storage::{
    Capability, DEFAULT_POOL_ID, DEFAULT_POOL_NAME, DefaultScope, MemberRef, PoolKind, PoolRole,
    Storage, default_pool_scope, resolve_memberships,
};
use serde::Serialize;
use ts_rs::TS;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The longest pool name accepted (a name, not a note).
const NAME_MAX: usize = 48;

/// Every pool, every session's place in one, and what roles may do.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolsInfo {
    /// The pools: "All sessions" first, then the user's, oldest first.
    pub pools: Vec<PoolInfo>,
    /// Each session's pool and role.
    pub members: Vec<PoolMemberInfo>,
}

/// One pool.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolInfo {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// `default` | `user`.
    pub kind: String,
    /// The one project it's for, when it is.
    pub project_id: Option<String>,
    /// What each role may do here, capability by capability.
    pub grants: Vec<GrantInfo>,
    /// How many separate groups its sessions make under D-33 (one project
    /// and one account each): in the default pool agents will see only
    /// their own group; in a user pool more than one means the user put
    /// work from different projects or accounts together.
    pub scopes: u32,
}

/// One session's place.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolMemberInfo {
    /// The session.
    pub session_id: String,
    /// Its pool.
    pub pool_id: String,
    /// `lead` | `member` | `observer`.
    pub role: String,
}

/// What one capability allows, per role.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct GrantInfo {
    /// The capability (`message`, `broadcast`, …).
    pub capability: String,
    /// Every role always has it; it can't be changed.
    pub fixed: bool,
    /// Leads may.
    pub lead: bool,
    /// Members may.
    pub member: bool,
    /// Observers may.
    pub observer: bool,
}

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

/// A pool that isn't deleted, or "not found".
fn live_pool(storage: &Storage, pool_id: &str) -> AppResult<juggler_core::storage::Pool> {
    storage.pools().get(pool_id).map_err(|_| AppError::NotFound {
        entity: "pool",
        id: pool_id.to_owned(),
    })
}

/// The default pool's scope of every session (D-33, through the one
/// function that decides it).
fn scopes_of(storage: &Storage) -> AppResult<HashMap<SessionId, DefaultScope>> {
    let mut accounts: HashMap<juggler_core::model::ProfileId, Option<AccountId>> = HashMap::new();
    let mut out = HashMap::new();
    for record in storage.sessions().list(None)? {
        let account = accounts
            .entry(record.profile.clone())
            .or_insert_with(|| storage.profiles().get(&record.profile).ok().map(|recipe| recipe.account))
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
        let overrides = repo.grant_overrides(&pool.id)?;
        let allowed = |capability: Capability, role: PoolRole| {
            capability.fixed()
                || overrides.get(&(capability, role)).copied().unwrap_or_else(|| {
                    juggler_core::storage::default_grant(capability, role)
                })
        };
        let grants = Capability::ALL
            .into_iter()
            .map(|capability| GrantInfo {
                capability: capability.as_str().to_owned(),
                fixed: capability.fixed(),
                lead: allowed(capability, PoolRole::Lead),
                member: allowed(capability, PoolRole::Member),
                observer: allowed(capability, PoolRole::Observer),
            })
            .collect();
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
            grants,
            scopes: u32::try_from(groups.len()).unwrap_or(u32::MAX),
        });
    }
    let members = sessions
        .iter()
        .map(|session| {
            let (pool_id, role) = places.get(session).cloned().unwrap_or_else(|| {
                (DEFAULT_POOL_ID.to_owned(), PoolRole::Member)
            });
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
/// [`AppError::Refused`] for the default pool or a bad name;
/// [`AppError::NotFound`] for a missing pool.
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
/// [`AppError::Refused`] for the default pool; [`AppError::NotFound`]
/// for a missing pool.
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
/// [`AppError::NotFound`] for a missing session or pool.
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
        .map_err(|_| AppError::NotFound {
            entity: "session",
            id: session_id.to_owned(),
        })?;
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
/// [`AppError::Validation`] for an unknown role; [`AppError::NotFound`]
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
        .map_err(|_| AppError::NotFound {
            entity: "session",
            id: session_id.to_owned(),
        })?;
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

/// Sets what one role may do in a pool.
///
/// # Errors
///
/// [`AppError::Refused`] for a fixed capability; [`AppError::Validation`]
/// for an unknown capability or role; [`AppError::NotFound`] for a
/// missing pool.
pub fn pool_set_grant_impl(
    state: &Arc<AppState>,
    pool_id: &str,
    capability: &str,
    role: &str,
    allowed: bool,
) -> AppResult<PoolsInfo> {
    let capability = Capability::parse(capability).ok_or_else(|| AppError::Validation {
        subject: "capability",
        message: format!("{capability} isn't something a role can do"),
    })?;
    let role = PoolRole::parse(role).ok_or_else(|| AppError::Validation {
        subject: "role",
        message: format!("{role} isn't a role (lead, member or observer)"),
    })?;
    if capability.fixed() {
        return Err(AppError::Refused(
            "Every role can always see the pool and read its own messages".to_owned(),
        ));
    }
    let storage = lock_storage_arc(state)?;
    live_pool(&storage, pool_id)?;
    storage
        .pools()
        .set_grant(pool_id, capability, role, allowed)?;
    pools_info(&storage)
}

/// Every pool and every session's place.
///
/// # Errors
///
/// Storage failures.
#[tauri::command]
pub fn pools_list(state: tauri::State<'_, Arc<AppState>>) -> AppResult<PoolsInfo> {
    pools_list_impl(state.inner())
}

/// Make a pool (optionally for one project).
///
/// # Errors
///
/// A refused name or project, or storage failures.
#[tauri::command]
pub fn pool_create(
    state: tauri::State<'_, Arc<AppState>>,
    name: String,
    project_id: Option<String>,
) -> AppResult<PoolsInfo> {
    pool_create_impl(state.inner(), &name, project_id.as_deref())
}

/// Rename a pool.
///
/// # Errors
///
/// A refused name, the default pool, or storage failures.
#[tauri::command]
pub fn pool_rename(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    name: String,
) -> AppResult<PoolsInfo> {
    pool_rename_impl(state.inner(), &pool_id, &name)
}

/// Delete a pool; its sessions go back to "All sessions".
///
/// # Errors
///
/// The default pool, or storage failures.
#[tauri::command]
pub fn pool_delete(state: tauri::State<'_, Arc<AppState>>, pool_id: String) -> AppResult<PoolsInfo> {
    pool_delete_impl(state.inner(), &pool_id)
}

/// Move a session into a pool.
///
/// # Errors
///
/// A session from another project, or storage failures.
#[tauri::command]
pub fn pool_move(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    pool_id: String,
) -> AppResult<PoolsInfo> {
    pool_move_impl(state.inner(), &session_id, &pool_id)
}

/// Set a session's role in its pool.
///
/// # Errors
///
/// An unknown role, or storage failures.
#[tauri::command]
pub fn pool_set_role(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    role: String,
) -> AppResult<PoolsInfo> {
    pool_set_role_impl(state.inner(), &session_id, &role)
}

/// Set what a role may do in a pool.
///
/// # Errors
///
/// A fixed capability, or storage failures.
#[tauri::command]
pub fn pool_set_grant(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    capability: String,
    role: String,
    allowed: bool,
) -> AppResult<PoolsInfo> {
    pool_set_grant_impl(state.inner(), &pool_id, &capability, &role, allowed)
}
