//! What about tasks waits on the user, for Home's Needs you.
//!
//! - **An agent's change above its pool's level** (`ask`): held until the
//!   user answers with the same three choices as every agent ask (D-28):
//!   Allow once, Always allow in this pool (offered from the second
//!   approval), Deny. A yes applies the change as the agent's own.
//! - **An agent's request for a new session** (`launch`): only the user
//!   starts one (D-31), on the launch settings they pick
//!   ([`super::launch`]), or dismisses it.
//!
//! The answers are the user's only: [`answer`] and [`dismiss`] run from
//! the app's own commands.

use std::sync::Arc;

use juggler_core::agent_access::AgentAction;
use juggler_core::model::SessionId;
use juggler_core::storage::TaskNeed;

use super::Actor;
use super::agent::{Held, run};
use super::dto::TaskNeedInfo;
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The most listed.
const EACH: u32 = 40;

/// Everything about tasks that waits on the user, oldest first.
///
/// # Errors
///
/// Storage failures.
pub fn needs(state: &Arc<AppState>) -> AppResult<Vec<TaskNeedInfo>> {
    let storage = lock_storage_arc(state)?;
    let pools = crate::features::pools::shown_pools(&storage)?;
    let repo = storage.tasks();
    let mut out = Vec::new();
    for need in repo.open_needs(0, EACH)? {
        let pool = pools.iter().find(|(p, _)| p.id == need.pool_id);
        let task = need
            .task_id
            .as_deref()
            .and_then(|id| repo.get(id).ok().flatten());
        let offer_always = need.kind == "ask"
            && AgentAction::parse(&need.action).is_some_and(|action| {
                storage
                    .agent_access()
                    .approvals(&need.pool_id, action)
                    .unwrap_or(0)
                    >= 1
            });
        let detail = match need.kind.as_str() {
            "launch" => need.change.clone().map_or_else(
                || "You pick the account and start it; nothing starts by itself.".to_owned(),
                |note| format!("«{note}»"),
            ),
            _ => format!(
                "{} · above what the pool lets agents do",
                pool.map_or("", |(_, name)| name.as_str())
            ),
        };
        out.push(TaskNeedInfo {
            id: need.id,
            kind: need.kind,
            pool_name: pool.map(|(_, name)| name.clone()).unwrap_or_default(),
            project_id: pool.and_then(|(p, _)| p.project.as_ref().map(|id| id.as_str().to_owned())),
            pool_id: need.pool_id,
            task_title: task.as_ref().map(|t| t.title.clone()),
            task_id: need.task_id,
            title: need.summary,
            detail,
            offer_always,
            at_ms: need.created_at,
        });
    }
    Ok(out)
}

/// The agent a need came from, as an actor now (its role as it is now).
fn actor_of(state: &Arc<AppState>, need: &TaskNeed) -> Actor {
    let mut actor = Actor {
        session: None,
        member_ref: need.actor_ref.clone(),
        name: need.actor_name.clone(),
        role: juggler_core::tasks::Role::Member,
        refs: vec![need.actor_ref.clone()],
    };
    let Ok(storage) = lock_storage_arc(state) else {
        return actor;
    };
    // Its session now (its Work item's newest), else the one it asked from.
    let session = crate::features::messages::live_session_of(&storage, &need.actor_ref)
        .or_else(|| need.actor_ref.strip_prefix("session:").map(SessionId::new));
    if let Some(session) = &session {
        actor.refs = crate::features::messages::refs_of(&storage, session);
        if let Ok(Some((_, role))) = crate::features::pools::members::place_of(&storage, session) {
            actor.role = super::role_of(role.as_str());
        }
    }
    actor.session = session;
    actor
}

/// The user's answer to a held change: `once`, `always` (a rule for this
/// action in this pool, D-28) or `deny`. A yes applies it as the agent's.
///
/// # Errors
///
/// [`AppError::Refused`] for a need that isn't open, or a change that
/// can't be done any more (the task moved on); validation for an unknown
/// answer.
pub fn answer(state: &Arc<AppState>, need_id: &str, answer: &str) -> AppResult<()> {
    if !matches!(answer, "once" | "always" | "deny") {
        return Err(AppError::Validation {
            subject: "answer",
            message: format!("{answer} isn't an answer (once, always or deny)"),
        });
    }
    let need = lock_storage_arc(state)?
        .tasks()
        .open_need(need_id)?
        .filter(|need| need.kind == "ask")
        .ok_or_else(|| AppError::Refused("That isn't waiting any more".to_owned()))?;
    let held: Held = need
        .change
        .as_deref()
        .and_then(|change| serde_json::from_str(change).ok())
        .ok_or_else(|| AppError::Refused("That change can't be read any more".to_owned()))?;
    // The same answer as every agent ask (counted, or a rule).
    crate::features::pools::access::answer_impl(state, &need.pool_id, &need.action, answer)?;
    let you = Actor::user();
    {
        let storage = lock_storage_arc(state)?;
        storage.tasks().close_need(&need.id, answer, None)?;
        if let Some(task) = held.task() {
            let verb = if answer == "deny" {
                "refused"
            } else {
                "allowed"
            };
            let _ = storage
                .tasks()
                .log(&need.pool_id, task, you.who(), verb, Some(&need.summary));
        }
    }
    state.tasks.notify(&need.pool_id);
    if answer == "deny" {
        return Ok(());
    }
    let actor = actor_of(state, &need);
    run(state, &need.pool_id, &actor, &held).map(|_| ())
}

/// The user dismisses a request for a new session.
///
/// # Errors
///
/// [`AppError::Refused`] for a request that isn't open.
pub fn dismiss(state: &Arc<AppState>, need_id: &str) -> AppResult<()> {
    let storage = lock_storage_arc(state)?;
    let need = storage
        .tasks()
        .open_need(need_id)?
        .filter(|need| need.kind == "launch")
        .ok_or_else(|| AppError::Refused("That isn't waiting any more".to_owned()))?;
    storage.tasks().close_need(&need.id, "dismissed", None)?;
    if let Some(task) = &need.task_id {
        let _ = storage.tasks().log(
            &need.pool_id,
            task,
            Actor::user().who(),
            "refused",
            Some("a new session"),
        );
    }
    drop(storage);
    state.tasks.notify(&need.pool_id);
    Ok(())
}
