//! AJ-E1702: who is where, never the whole history at once (I6).
//!
//! - A session's place is what a member row says (the user put it, or
//!   its Work item, there), else its project's default pool, as a member
//!   (D-33). [`Memberships`] answers it per session from the rows alone.
//! - The pools' answer counts each pool's sessions in one narrow storage
//!   pass ([`Tally`]) and names only the places of the working set's
//!   sessions (what the rail holds) that aren't that default
//!   ([`placements`]). The UI falls back to the default for the rest, and
//!   reads a session past the working set alone ([`place_impl`]).
//! - A pool's members come a page at a time ([`members_page`]): its lead
//!   first, then members, then observers, the most recently active first
//!   within each. Only the page's sessions are built in full.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use juggler_core::model::{ProjectId, SessionId};
use juggler_core::storage::{Memberships, PoolRole, SessionBrief, Storage};

use super::dto::{PoolMemberInfo, PoolMemberRow};
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::paging::{Page, PageRequest, SortKey};
use crate::state::AppState;

/// Who is where now, from the member rows and the sessions' Work items.
///
/// # Errors
///
/// Storage failures.
pub fn memberships(storage: &Storage) -> AppResult<Memberships> {
    Ok(Memberships::new(
        &storage.pools().members()?,
        storage.switch_graph().works_by_session()?,
    ))
}

/// One session's pool and role; `None` when the session isn't there.
///
/// # Errors
///
/// Storage failures.
pub fn place_of(storage: &Storage, session: &SessionId) -> AppResult<Option<(String, PoolRole)>> {
    let Ok(record) = storage.sessions().get(session) else {
        return Ok(None);
    };
    Ok(Some(
        memberships(storage)?.place(session, record.project.as_ref()),
    ))
}

/// The sessions archived now (they're history: no pool lists them).
fn archived(storage: &Storage) -> AppResult<BTreeSet<String>> {
    Ok(storage
        .sessions()
        .lifecycle_latest()?
        .into_iter()
        .filter(|(_, (event, _))| event == "archived")
        .map(|(id, _)| id)
        .collect())
}

/// Each pool's sessions, counted in one narrow pass.
#[derive(Debug, Default)]
pub struct Tally {
    /// Sessions in each pool, archived ones aside.
    members: HashMap<String, u32>,
    /// The projects each pool's sessions come from (archived ones too:
    /// they're still its sessions).
    projects: HashMap<String, BTreeSet<Option<ProjectId>>>,
}

impl Tally {
    /// Counts every session's place.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn read(storage: &Storage, memberships: &Memberships) -> AppResult<Self> {
        let archived = archived(storage)?;
        let mut tally = Self::default();
        for brief in storage.sessions().briefs()? {
            let (pool_id, _) = memberships.place(&brief.id, brief.project.as_ref());
            if !archived.contains(brief.id.as_str()) {
                *tally.members.entry(pool_id.clone()).or_default() += 1;
            }
            tally
                .projects
                .entry(pool_id)
                .or_default()
                .insert(brief.project);
        }
        Ok(tally)
    }

    /// How many sessions `pool_id` holds, archived ones aside.
    #[must_use]
    pub fn members(&self, pool_id: &str) -> u32 {
        self.members.get(pool_id).copied().unwrap_or_default()
    }

    /// How many projects `pool_id`'s sessions come from.
    #[must_use]
    pub fn scopes(&self, pool_id: &str) -> u32 {
        self.projects.get(pool_id).map_or(0, |projects| {
            u32::try_from(projects.len()).unwrap_or(u32::MAX)
        })
    }
}

/// The places of the working set's sessions that a row sets apart from
/// their project's default pool as a member, by session id.
///
/// # Errors
///
/// Storage failures.
pub fn placements(storage: &Storage, memberships: &Memberships) -> AppResult<Vec<PoolMemberInfo>> {
    let mut placements: Vec<PoolMemberInfo> = crate::commands::working_records(storage)?
        .into_iter()
        .filter_map(|record| {
            let (pool_id, role) = memberships.placed(&record.id)?;
            let default = juggler_core::storage::default_pool_id(record.project.as_ref());
            (pool_id != default || role != PoolRole::Member).then(|| PoolMemberInfo {
                session_id: record.id.as_str().to_owned(),
                pool_id: pool_id.to_owned(),
                role: role.as_str().to_owned(),
            })
        })
        .collect();
    placements.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    Ok(placements)
}

/// One session's place, any session (one past the working set too).
///
/// # Errors
///
/// [`AppError::Refused`] for a session that isn't there; storage failures.
pub fn place_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<PoolMemberInfo> {
    let storage = lock_storage_arc(state)?;
    let (pool_id, role) = place_of(&storage, &SessionId::new(session_id))?
        .ok_or_else(|| AppError::Refused("That session isn't there any more".to_owned()))?;
    Ok(PoolMemberInfo {
        session_id: session_id.to_owned(),
        pool_id,
        role: role.as_str().to_owned(),
    })
}

/// A role's place in a pool's list: the lead first, observers last.
pub(super) fn role_rank(role: PoolRole) -> i64 {
    match role {
        PoolRole::Lead => 0,
        PoolRole::Member => 1,
        PoolRole::Observer => 2,
    }
}

/// One of a pool's sessions, before its page is built.
struct Candidate {
    brief: SessionBrief,
    role: PoolRole,
}

impl Candidate {
    /// The lead first, then members, then observers; the most recently
    /// active first within each; then by id.
    fn sort_key(&self) -> SortKey {
        SortKey::new(
            [role_rank(self.role), -self.brief.last_ms],
            self.brief.id.as_str(),
        )
    }
}

/// One page of a pool's members (archived sessions aside): each one's
/// session row and card.
///
/// # Errors
///
/// [`AppError::Refused`] for a pool that isn't there; a cursor this app
/// didn't make; storage failures.
pub fn members_page(
    state: &Arc<AppState>,
    pool_id: &str,
    request: &PageRequest,
) -> AppResult<Page<PoolMemberRow>> {
    let (page, records, memberships, projects) = {
        let storage = lock_storage_arc(state)?;
        super::live_pool(&storage, pool_id)?;
        let memberships = memberships(&storage)?;
        let archived = archived(&storage)?;
        let candidates: Vec<Candidate> = storage
            .sessions()
            .briefs()?
            .into_iter()
            .filter(|brief| !archived.contains(brief.id.as_str()))
            .filter_map(|brief| {
                let (place, role) = memberships.place(&brief.id, brief.project.as_ref());
                (place == pool_id).then_some(Candidate { brief, role })
            })
            .collect();
        let page = crate::paging::page(candidates, Candidate::sort_key, request)?;
        let ids: Vec<SessionId> = page.items.iter().map(|c| c.brief.id.clone()).collect();
        let records = storage.sessions().get_many(&ids)?;
        (page, records, memberships, super::project_names(&storage)?)
    };
    // Only this page's sessions are built in full.
    let mut built: HashMap<String, _> = crate::commands::instances_of(state, records)?
        .into_iter()
        .map(|row| (row.id.clone(), row))
        .collect();
    let rows = page
        .items
        .iter()
        .filter_map(|candidate| {
            let session = built.remove(candidate.brief.id.as_str())?;
            let card =
                super::cards::card_for(state, &session, candidate.role, &projects, &memberships);
            Some(PoolMemberRow { session, card })
        })
        .collect();
    Ok(Page {
        items: rows,
        next_cursor: page.next_cursor,
        total: page.total,
    })
}
