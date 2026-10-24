//! D-28: what agents may do in each pool.
//!
//! - The user sets each pool's level and takes back rules, in the app;
//!   those are the only writes here, and only the app's own commands call
//!   them. An agent's request goes through [`allowed`] alone, which only
//!   decides (and writes the audit).
//! - Above the level is an ask: the user answers it with [`answer_impl`]
//!   (Allow once, Always allow in this pool, Deny); from the second
//!   identical approval the ask offers the rule.
//!
//! The hard lines are in what this module can't do: no action in the
//! catalog changes a pool, a level or a rule, starts a session, picks an
//! account or reads a credential (`tests/pools.rs` checks the source).

use std::collections::BTreeSet;
use std::sync::Arc;

use juggler_core::agent_access::{AccessLevel, AgentAction, Decision};
use juggler_core::model::SessionId;
use juggler_core::storage::{PoolRole, Storage};

use super::{AccessInfo, AgentRuleInfo, PoolsInfo, pools_info};
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// A pool's level and rules, as the app shows them.
///
/// # Errors
///
/// Storage failures.
pub fn info(storage: &Storage, pool_id: &str) -> AppResult<(AccessInfo, Vec<AgentRuleInfo>)> {
    let access = storage.agent_access().access(pool_id)?;
    let level = access.level.unwrap_or(AccessLevel::Read);
    let allowed = level.actions(&access.custom);
    let rules = storage
        .agent_access()
        .rules(Some(pool_id))?
        .into_iter()
        .map(|rule| AgentRuleInfo {
            id: rule.id,
            pool_id: rule.pool_id,
            action: rule.action.as_str().to_owned(),
            created_at: rule.created_at,
            made_by: rule.made_by,
        })
        .collect();
    Ok((
        AccessInfo {
            level: level.as_str().to_owned(),
            actions: AgentAction::ALL
                .into_iter()
                .filter(|action| allowed.contains(action))
                .map(|action| action.as_str().to_owned())
                .collect(),
        },
        rules,
    ))
}

fn action_of(word: &str) -> AppResult<AgentAction> {
    AgentAction::parse(word).ok_or_else(|| AppError::Validation {
        subject: "action",
        message: format!("{word} isn't something an agent can do"),
    })
}

/// The user sets a pool's level; `actions` is the Custom set (ignored
/// for a named level).
///
/// # Errors
///
/// [`AppError::Validation`] for an unknown level or action;
/// [`AppError::Refused`] for a missing pool.
pub fn set_level_impl(
    state: &Arc<AppState>,
    pool_id: &str,
    level: &str,
    actions: &[String],
) -> AppResult<PoolsInfo> {
    let level = AccessLevel::parse(level).ok_or_else(|| AppError::Validation {
        subject: "level",
        message: format!("{level} isn't a level (read, work together, full or custom)"),
    })?;
    let custom: BTreeSet<AgentAction> = actions
        .iter()
        .map(|word| action_of(word))
        .collect::<AppResult<_>>()?;
    let storage = lock_storage_arc(state)?;
    storage
        .agent_access()
        .set_level(pool_id, level, &custom)
        .map_err(|_| AppError::Refused("That pool isn't there any more".to_owned()))?;
    pools_info(&storage)
}

/// The user takes back an "always allow" rule.
///
/// # Errors
///
/// [`AppError::Refused`] for a rule that's gone already.
pub fn revoke_impl(state: &Arc<AppState>, rule_id: i64) -> AppResult<PoolsInfo> {
    let storage = lock_storage_arc(state)?;
    storage
        .agent_access()
        .revoke(rule_id)
        .map_err(|_| AppError::Refused("That rule isn't there any more".to_owned()))?;
    pools_info(&storage)
}

/// The user's answer to an agent's ask: `once` (counted: the next
/// identical ask offers a rule), `always` (a rule for this action in this
/// pool) or `deny` (nothing is kept).
///
/// # Errors
///
/// [`AppError::Validation`] for an unknown action or answer;
/// [`AppError::Refused`] for a missing pool.
pub fn answer_impl(
    state: &Arc<AppState>,
    pool_id: &str,
    action: &str,
    answer: &str,
) -> AppResult<PoolsInfo> {
    let action = action_of(action)?;
    let storage = lock_storage_arc(state)?;
    let repo = storage.agent_access();
    if repo.access(pool_id)?.level.is_none() {
        return Err(AppError::Refused(
            "That pool isn't there any more".to_owned(),
        ));
    }
    match answer {
        "once" => repo.allow_once(pool_id, action)?,
        "always" => {
            repo.always_allow(pool_id, action)?;
        }
        "deny" => {}
        other => {
            return Err(AppError::Validation {
                subject: "answer",
                message: format!("{other} isn't an answer (once, always or deny)"),
            });
        }
    }
    pools_info(&storage)
}

/// The one decision point for an agent's request (D-28): may the agent in
/// `session_id` do `action` in `pool_id`? It is a member when its session
/// is in that pool; an observer may only read without an ask. Every
/// request is audited. A failure to decide is a deny, never an allow.
#[must_use]
pub fn allowed(
    state: &Arc<AppState>,
    session_id: &str,
    pool_id: &str,
    action: AgentAction,
) -> Decision {
    let Ok(storage) = lock_storage_arc(state) else {
        return Decision::Deny;
    };
    let session = SessionId::new(session_id);
    let place = storage
        .sessions()
        .get(&session)
        .ok()
        .and_then(|_| pools_info(&storage).ok())
        .and_then(|info| {
            info.members
                .into_iter()
                .find(|row| row.session_id == session_id)
        });
    let member = place.as_ref().is_some_and(|row| row.pool_id == pool_id);
    let observer = place
        .as_ref()
        .is_some_and(|row| PoolRole::parse(&row.role) == Some(PoolRole::Observer));
    storage
        .agent_access()
        .allowed(
            pool_id,
            &format!("session:{session_id}"),
            member,
            observer,
            action,
        )
        .unwrap_or(Decision::Deny)
}
