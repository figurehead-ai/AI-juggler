//! D-62 decisions 8–16 (AJ-E3003–E3005): the pools model as the user
//! changes it — scope, member rules of every kind, a member's own level,
//! talk per role pair or connection, sealing — and a session's own answer
//! to "which pools, how, and why not" (its Pools tab).
//!
//! Only the app's UI calls these (D-28): nothing an agent runs reaches
//! them. Each refusal says why, and the step that would let it in.

use std::collections::BTreeSet;
use std::sync::Arc;

use juggler_core::agent_access::AccessLevel;
use juggler_core::model::{ProjectId, SessionId};
use juggler_core::pools::SkipReason;
use juggler_core::storage::{
    MemberKind, MemberLevel, MemberRef, Pool, PoolKind, PoolRole, Scope, Storage, TalkKey, TalkRule,
};

use super::dto::{
    EdgeTalkInfo, PoolRuleInfo, PoolRulesInfo, PoolTalkInfo, PoolsInfo, SessionPoolInfo,
    SessionPoolsInfo, SessionSkipInfo,
};
use super::{live_pool, members, pool_names, pools_info};
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Whether `session` may be named in `pool`: its scope holds the session's
/// project, and no sealed pool (other than this one) holds the session.
///
/// # Errors
///
/// [`AppError::Refused`] with the reason and the step that would let it
/// in; storage failures.
pub fn may_join(storage: &Storage, pool: &Pool, session: &SessionId) -> AppResult<()> {
    let facts = storage
        .sessions()
        .pool_facts_of(session)?
        .ok_or_else(|| AppError::Refused("That session isn't there any more".to_owned()))?;
    let names = pool_names(storage)?;
    let shown = |id: &str| {
        names
            .get(id)
            .cloned()
            .unwrap_or_else(|| "that pool".to_owned())
    };
    if !pool.scope.holds(facts.project.as_ref()) {
        let project_name = |project: Option<&ProjectId>| match project {
            Some(project) => storage
                .projects()
                .get(project)
                .map_or_else(|_| "another project".to_owned(), |row| row.name),
            None => "no project".to_owned(),
        };
        let here = project_name(facts.project.as_ref());
        let takes = match &pool.scope {
            Scope::Every => "every project".to_owned(),
            Scope::These(projects) => {
                let mut names: Vec<String> = projects
                    .iter()
                    .map(|project| match project {
                        Some(_) => project_name(project.as_ref()),
                        None => "sessions with no project".to_owned(),
                    })
                    .collect();
                names.sort();
                match names.as_slice() {
                    [one] => format!("{one} only"),
                    [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
                    [] => "no project yet".to_owned(),
                }
            }
        };
        let works = if facts.project.is_some() {
            format!("this one works in {here}")
        } else {
            "this one has no project".to_owned()
        };
        return Err(AppError::Refused(if pool.kind == PoolKind::Default {
            format!("«{}» holds {takes}; {works}.", shown(&pool.id))
        } else {
            format!(
                "«{}» takes {takes}; {works}. Add {here} to its scope to let it in.",
                shown(&pool.id)
            )
        }));
    }
    let resolution = members::memberships(storage)?.resolve(&facts);
    if let Some(seal) = resolution
        .placements
        .iter()
        .find(|place| place.sealed && place.pool_id != pool.id)
    {
        return Err(AppError::Refused(format!(
            "It's in «{}», which is sealed: its sessions are in no other pool. Take it out there first.",
            shown(&seal.pool_id)
        )));
    }
    Ok(())
}

/// A member rule from its wire form; a session's id becomes its Work
/// item's ref when it switched (so the switch keeps it).
fn member_from(storage: &Storage, wire: &str) -> AppResult<MemberRef> {
    let member = MemberRef::parse(wire).ok_or_else(|| AppError::Validation {
        subject: "member",
        message: format!("{wire} isn't a member"),
    })?;
    Ok(match member {
        MemberRef::Session(id) => super::member_of(storage, &id)?.0,
        // A stored Account only: the pools L2 on the replica found New
        // pool sending the Accounts list's opaque key, kept as a member
        // that could never match a session.
        MemberRef::Account(id) if storage.subscriptions().get(&id)?.is_none() => {
            return Err(AppError::Refused(
                "That Account isn't one AI Juggler has: read its sign-in first (Accounts)"
                    .to_owned(),
            ));
        }
        other => other,
    })
}

fn level_from(level: Option<&str>, actions: &[String]) -> AppResult<Option<MemberLevel>> {
    let Some(word) = level else {
        return Ok(None);
    };
    let level = AccessLevel::parse(word).ok_or_else(|| AppError::Validation {
        subject: "level",
        message: format!("{word} isn't a level (read, work_together, full or custom)"),
    })?;
    Ok(Some(MemberLevel {
        level: level.as_str().to_owned(),
        custom_actions: if level == AccessLevel::Custom {
            actions.join(",")
        } else {
            String::new()
        },
    }))
}

fn role_from(role: &str) -> AppResult<PoolRole> {
    PoolRole::parse(role).ok_or_else(|| AppError::Validation {
        subject: "role",
        message: format!("{role} isn't a role (lead, member, reviewer or observer)"),
    })
}

/// Adds a member rule to a pool, or changes its role and level there; its
/// other pools stay (D-62 decision 10). A session named on its own must
/// be one the pool may hold ([`may_join`]).
///
/// # Errors
///
/// [`AppError::Validation`] for an unknown member, role or level;
/// [`AppError::Refused`] as [`may_join`], or for a missing pool.
pub fn join_impl(
    state: &Arc<AppState>,
    pool_id: &str,
    member: &str,
    role: &str,
    level: Option<&str>,
    actions: &[String],
) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    let pool = live_pool(&storage, pool_id)?;
    let role = role_from(role)?;
    let level = level_from(level, actions)?;
    match MemberRef::parse(member) {
        Some(MemberRef::Session(session)) => may_join(&storage, &pool, &session)?,
        Some(MemberRef::Conversation { folder, id }) => {
            // A Watched member's project, from the conversation index.
            drop(storage);
            let project = crate::features::conversations::commands::list_indexed(state)?
                .into_iter()
                .find(|info| info.account_id == folder && info.id == id)
                .and_then(|info| info.project_id);
            if !pool.scope.holds(project.map(ProjectId::new).as_ref()) {
                return Err(AppError::Refused(
                    "That session works outside this pool's projects. Add its project to the pool's scope first.".to_owned(),
                ));
            }
            return join_checked(state, &pool, member, role, level.as_ref());
        }
        _ => {}
    }
    let member = member_from(&storage, member)?;
    let mut replaces = Vec::new();
    if member.kind() == MemberKind::Session {
        let works = storage.switch_graph().works_by_session()?;
        replaces = storage
            .pools()
            .members()?
            .into_iter()
            .filter(|row| row.pool_id == pool.id && row.member.anchor(&works) == member)
            .map(|row| row.member)
            .collect();
    }
    storage
        .pools()
        .join(&pool.id, &member, &replaces, role, level.as_ref())?;
    super::super::relations::changed(state, super::super::relations::Change::PoolsChanged);
    pools_info(&storage)
}

/// Adds a member that needs no session check (a Watched conversation)
/// after its own check.
fn join_checked(
    state: &Arc<AppState>,
    pool: &Pool,
    member: &str,
    role: PoolRole,
    level: Option<&MemberLevel>,
) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    let member = member_from(&storage, member)?;
    storage.pools().join(&pool.id, &member, &[], role, level)?;
    super::super::relations::changed(state, super::super::relations::Change::PoolsChanged);
    pools_info(&storage)
}

/// Ends a member rule in a pool (and a session's rows from before its
/// switch there). A project's own pool keeps its everyone.
///
/// # Errors
///
/// [`AppError::Refused`] for a missing pool, or the everyone of a
/// project's own pool.
pub fn leave_impl(state: &Arc<AppState>, pool_id: &str, member: &str) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    let pool = live_pool(&storage, pool_id)?;
    let member = member_from(&storage, member)?;
    if pool.kind == PoolKind::Default && member == MemberRef::Everyone {
        return Err(AppError::Refused(
            "A project's own pool always holds everyone in its project.".to_owned(),
        ));
    }
    let works = storage.switch_graph().works_by_session()?;
    let rows: Vec<MemberRef> = storage
        .pools()
        .members()?
        .into_iter()
        .filter(|row| row.pool_id == pool.id && row.member.anchor(&works) == member.anchor(&works))
        .map(|row| row.member)
        .collect();
    storage.pools().leave(&pool.id, &rows)?;
    super::super::relations::changed(state, super::super::relations::Change::PoolsChanged);
    pools_info(&storage)
}

/// Sets a user pool's scope: every project, or the ones named (`None`
/// for sessions with no project). A pool needs at least one.
///
/// # Errors
///
/// [`AppError::Refused`] for an empty scope, an unknown project, a
/// project's own pool or a missing pool.
pub fn set_scope_impl(
    state: &Arc<AppState>,
    pool_id: &str,
    every: bool,
    projects: &[Option<String>],
) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    let pool = live_pool(&storage, pool_id)?;
    if pool.kind == PoolKind::Default {
        return Err(AppError::Refused(
            "A project's own pool keeps its project.".to_owned(),
        ));
    }
    let scope = if every {
        Scope::Every
    } else {
        let mut set = BTreeSet::new();
        for project in projects {
            if let Some(id) = project {
                storage
                    .projects()
                    .get(&ProjectId::new(id.as_str()))
                    .map_err(|_| {
                        AppError::Refused("That project isn't there any more".to_owned())
                    })?;
            }
            set.insert(project.as_deref().map(ProjectId::new));
        }
        if set.is_empty() {
            return Err(AppError::Refused(
                "Pick at least one project, or every project.".to_owned(),
            ));
        }
        Scope::These(set)
    };
    storage.pools().set_scope(&pool.id, &scope)?;
    super::super::relations::changed(state, super::super::relations::Change::PoolsChanged);
    pools_info(&storage)
}

/// Seals a pool or opens it (D-62 decision 14). A sealed pool has no
/// links: sealing ends them.
///
/// # Errors
///
/// [`AppError::Refused`] for a project's own pool or a missing pool.
pub fn set_sealed_impl(state: &Arc<AppState>, pool_id: &str, sealed: bool) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    let pool = live_pool(&storage, pool_id)?;
    if pool.kind == PoolKind::Default {
        return Err(AppError::Refused(
            "A project's own pool can't be sealed: make a pool for the work you want kept apart."
                .to_owned(),
        ));
    }
    storage.pools().set_sealed(&pool.id, sealed)?;
    if sealed {
        let messages = storage.messages();
        for other in messages.links(&pool.id)? {
            messages.unlink(&pool.id, &other)?;
        }
    }
    super::super::relations::changed(state, super::super::relations::Change::PoolsChanged);
    pools_info(&storage)
}

/// Whether a pool's broad members skip the sessions another pool names.
///
/// # Errors
///
/// [`AppError::Refused`] for a missing pool.
pub fn set_skip_held_impl(
    state: &Arc<AppState>,
    pool_id: &str,
    skip: bool,
) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    live_pool(&storage, pool_id)?;
    storage.pools().set_skip_held(pool_id, skip)?;
    super::super::relations::changed(state, super::super::relations::Change::PoolsChanged);
    pools_info(&storage)
}

/// A connection's end: a session named by id stands for its Work item
/// (so a switch keeps the rule), as its member rows do.
fn talk_end(storage: &Storage, wire: &str) -> AppResult<MemberRef> {
    member_from(storage, wire)
}

fn talk_key(storage: &Storage, from: &str, to: &str) -> AppResult<TalkKey> {
    let bad = || AppError::Validation {
        subject: "talk",
        message: format!("{from} → {to} isn't a role pair or a connection"),
    };
    match (from.strip_prefix("role:"), to.strip_prefix("role:")) {
        (Some(from), Some(to)) => Ok(TalkKey::Roles(
            PoolRole::parse(from).ok_or_else(bad)?,
            PoolRole::parse(to).ok_or_else(bad)?,
        )),
        (None, None) => Ok(TalkKey::Connection(
            talk_end(storage, from).map_err(|_| bad())?,
            talk_end(storage, to).map_err(|_| bad())?,
        )),
        _ => Err(bad()),
    }
}

/// Sets who may tell whom (D-62 decision 13): a role pair
/// (`role:lead` → `role:member`) or one connection (two members' refs);
/// `rule` `None` clears it back to the pair's, or to allowed.
///
/// # Errors
///
/// [`AppError::Validation`] for a bad pair or rule; a missing pool.
pub fn set_talk_impl(
    state: &Arc<AppState>,
    pool_id: &str,
    from: &str,
    to: &str,
    rule: Option<&str>,
) -> AppResult<PoolRulesInfo> {
    let storage = lock_storage_arc(state)?;
    live_pool(&storage, pool_id)?;
    let key = talk_key(&storage, from, to)?;
    let rule = match rule {
        None => None,
        Some(word) => Some(TalkRule::parse(word).ok_or_else(|| AppError::Validation {
            subject: "rule",
            message: format!("{word} isn't a rule (allowed, ask or blocked)"),
        })?),
    };
    storage.pools().set_talk(pool_id, &key, rule)?;
    rules_of(&storage, pool_id)
}

/// The rule along one line between two of a pool's sessions: its own,
/// else its roles' pair's (D-62 decision 13).
///
/// # Errors
///
/// [`AppError::Refused`] for a session that isn't in the pool; storage
/// failures.
pub fn edge_talk_impl(
    state: &Arc<AppState>,
    pool_id: &str,
    from: &str,
    to: &str,
) -> AppResult<EdgeTalkInfo> {
    use juggler_core::pools::talk::PoolTalk;
    let storage = lock_storage_arc(state)?;
    live_pool(&storage, pool_id)?;
    let gone = || AppError::Refused("That session isn't in this pool any more".to_owned());
    let (from_session, to_session) = (SessionId::new(from), SessionId::new(to));
    let from_role = members::role_in(&storage, &from_session, pool_id)?.ok_or_else(gone)?;
    let to_role = members::role_in(&storage, &to_session, pool_id)?.ok_or_else(gone)?;
    let (from_ref, _) = super::member_of(&storage, &from_session)?;
    let (to_ref, _) = super::member_of(&storage, &to_session)?;
    let rows = storage.pools().talk()?;
    let talk = PoolTalk::new(
        rows.iter()
            .filter(|(pool, ..)| pool == pool_id)
            .map(|(_, key, rule)| (key, *rule)),
    );
    let own = talk.connection(&from_ref, &to_ref);
    let pair = talk.pair(from_role, to_role);
    Ok(EdgeTalkInfo {
        rule: own.unwrap_or(pair).as_str().to_owned(),
        pair_rule: pair.as_str().to_owned(),
        own: own.is_some(),
        from_role: from_role.as_str().to_owned(),
        to_role: to_role.as_str().to_owned(),
    })
}

/// The most sessions a rule names for its lines (the canvas draws 60).
const CANVAS_IDS: usize = 60;

/// A pool's member rules (most specific first) and talk settings.
///
/// # Errors
///
/// Storage failures.
pub fn rules_of(storage: &Storage, pool_id: &str) -> AppResult<PoolRulesInfo> {
    // How many sessions each member brings in now.
    let memberships = members::memberships(storage)?;
    let mut brought: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut ids: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let archived = members::archived(storage)?;
    for facts in storage.sessions().pool_facts()? {
        if archived.contains(facts.session.as_str()) {
            continue;
        }
        if let Some(place) = memberships.resolve(&facts).in_pool(pool_id) {
            *brought.entry(place.member.to_wire()).or_default() += 1;
            let some = ids.entry(place.member.to_wire()).or_default();
            if some.len() < CANVAS_IDS {
                some.push(facts.session.as_str().to_owned());
            }
        }
    }
    let mut rows: Vec<PoolRuleInfo> = storage
        .pools()
        .members()?
        .into_iter()
        .filter(|row| row.pool_id == pool_id)
        .map(|row| PoolRuleInfo {
            sessions: brought.get(&row.member.to_wire()).copied().unwrap_or(0),
            session_ids: ids.remove(&row.member.to_wire()).unwrap_or_default(),
            member: row.member.to_wire(),
            kind: row.member.kind().as_str().to_owned(),
            role: row.role.as_str().to_owned(),
            level: row.level.map(|level| level.level),
            joined_at: row.joined_at,
        })
        .collect();
    let rank = |kind: &str| MemberKind::ALL.iter().position(|one| one.as_str() == kind);
    rows.sort_by(|a, b| {
        rank(&a.kind)
            .cmp(&rank(&b.kind))
            .then(a.joined_at.cmp(&b.joined_at))
    });
    let talk = storage
        .pools()
        .talk()?
        .into_iter()
        .filter(|(pool, ..)| pool == pool_id)
        .map(|(_, key, rule)| {
            let (from, to) = match key {
                TalkKey::Roles(from, to) => (
                    format!("role:{}", from.as_str()),
                    format!("role:{}", to.as_str()),
                ),
                TalkKey::Connection(from, to) => (from.to_wire(), to.to_wire()),
            };
            PoolTalkInfo {
                from,
                to,
                rule: rule.as_str().to_owned(),
            }
        })
        .collect();
    Ok(PoolRulesInfo {
        pool_id: pool_id.to_owned(),
        members: rows,
        talk,
    })
}

/// [`rules_of`], for the UI.
///
/// # Errors
///
/// [`AppError::Refused`] for a missing pool; storage failures.
pub fn rules_impl(state: &Arc<AppState>, pool_id: &str) -> AppResult<PoolRulesInfo> {
    let storage = lock_storage_arc(state)?;
    live_pool(&storage, pool_id)?;
    rules_of(&storage, pool_id)
}

/// A session's pools, how it got into each, and the pools that left it
/// out with the reason and the step that would let it in.
///
/// # Errors
///
/// [`AppError::Refused`] for a session that isn't there; storage failures.
pub fn session_pools_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<SessionPoolsInfo> {
    let storage = lock_storage_arc(state)?;
    let resolution = members::resolution_of(&storage, &SessionId::new(session_id))?
        .ok_or_else(|| AppError::Refused("That session isn't there any more".to_owned()))?;
    let names = pool_names(&storage)?;
    let name = |id: &str| {
        names
            .get(id)
            .cloned()
            .unwrap_or_else(|| "A pool".to_owned())
    };
    let pools = resolution
        .placements
        .iter()
        .map(|place| SessionPoolInfo {
            pool_id: place.pool_id.clone(),
            pool_name: name(&place.pool_id),
            role: place.role.as_str().to_owned(),
            via: place.via().as_str().to_owned(),
            member: place.member.to_wire(),
            level: place.level.as_ref().map(|level| level.level.clone()),
            sealed: place.sealed,
        })
        .collect();
    let skipped = resolution
        .skipped
        .iter()
        .map(|skip| {
            let (reason, by) = match &skip.reason {
                SkipReason::OutOfScope => ("out_of_scope", None),
                SkipReason::SealedElsewhere { sealed_pool } => ("sealed_elsewhere", Some(sealed_pool.clone())),
                SkipReason::HeldElsewhere { by_pool } => ("held_elsewhere", Some(by_pool.clone())),
            };
            let by_name = by.as_deref().map(name);
            let words = match (&skip.reason, &by_name) {
                (SkipReason::OutOfScope, _) => format!(
                    "«{}» doesn't take its project. Add the project to its scope to let it in.",
                    name(&skip.pool_id)
                ),
                (SkipReason::SealedElsewhere { .. }, Some(by)) => format!(
                    "It's in «{by}» (sealed), so it's in no other pool. Take it out of «{by}» to let it in."
                ),
                (SkipReason::HeldElsewhere { .. }, Some(by)) => format!(
                    "You put it in «{by}», so «{}» leaves it out.",
                    name(&skip.pool_id)
                ),
                _ => "Left out.".to_owned(),
            };
            SessionSkipInfo {
                pool_id: skip.pool_id.clone(),
                pool_name: name(&skip.pool_id),
                reason: reason.to_owned(),
                by_pool: by,
                by_pool_name: by_name,
                words,
            }
        })
        .collect();
    Ok(SessionPoolsInfo {
        session_id: session_id.to_owned(),
        pools,
        skipped,
        account_unknown: resolution.account_unknown,
    })
}
