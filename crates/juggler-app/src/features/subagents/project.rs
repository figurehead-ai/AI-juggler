//! AJ-E1520 (0.3): a project's subagents — the same view as a session's
//! tab, over its newest sessions together (each group says which
//! session it is in). Each session is read by its own reader as its tab
//! would read it (its saved index first), within a share of the tab's
//! budget, so a project with many long sessions fills in over a few asks.

use std::sync::Arc;

use juggler_adapter::subagents::{SubagentStatus, TokensCounted};

use super::dto::{SubagentSessionInfo, SubagentsInfo, SubagentsQuery};
use super::view::{self, Found};
use super::{TAB_BUDGET, found, without};
use crate::error::AppResult;
use crate::paging::PageRequest;
use crate::state::AppState;

/// The project's newest sessions read (the rest: in each session's own
/// tab).
pub const SESSIONS_MAX: usize = 24;

/// The least of the budget one session gets.
const LEAST: u64 = 8 * 1024 * 1024;

/// A project's subagents, a page of rows at a time.
///
/// # Errors
///
/// Storage errors; a cursor this app didn't make.
pub fn project_impl(
    state: &Arc<AppState>,
    project_id: &str,
    query: &SubagentsQuery,
    request: &PageRequest,
) -> AppResult<SubagentsInfo> {
    let (root, homes) = {
        let storage = crate::commands::lock_storage_arc(state)?;
        let project = storage
            .projects()
            .get(&juggler_core::model::ProjectId::new(project_id.to_owned()))?;
        (project.root, storage.projects().worktree_homes()?)
    };
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    let sessions: Vec<_> = state
        .conversations
        .all()?
        .into_iter()
        .filter(|conversation| {
            conversation.folder.as_deref().is_some_and(|folder| {
                let folder_root = juggler_core::storage::project_root_with(folder, &homes);
                std::fs::canonicalize(&folder_root).unwrap_or(folder_root) == root
            })
        })
        .take(SESSIONS_MAX)
        .collect();
    let share = (TAB_BUDGET / sessions.len().max(1) as u64).max(LEAST);
    let mut parts: Vec<(SubagentSessionInfo, Found)> = Vec::new();
    for conversation in sessions {
        if let Ok(found) = found(
            state,
            &conversation.account_id,
            &conversation.summary.id,
            share,
        ) {
            parts.push((
                SubagentSessionInfo {
                    account_id: conversation.account_id.clone(),
                    conversation_id: conversation.summary.id.clone(),
                    title: conversation.summary.title.clone(),
                },
                found,
            ));
        }
    }
    match merged(parts) {
        Some(found) => view::listed(&found, query, request),
        None => Ok(without(super::Missing::None)),
    }
}

/// Several sessions' subagents as one: a subagent running in a session
/// that has ended is unknown (D-06) before they are put together.
#[must_use]
pub fn merged(parts: Vec<(SubagentSessionInfo, Found)>) -> Option<Found> {
    let mut parts = parts.into_iter();
    let (first_session, mut out) = parts.next()?;
    let tag = |found: &mut Found, session: &SubagentSessionInfo| {
        for agent in &mut found.agents {
            if !found.live && agent.status == SubagentStatus::Running {
                agent.status = SubagentStatus::Unknown;
            }
            found.sessions.insert(agent.id.clone(), session.clone());
        }
    };
    tag(&mut out, &first_session);
    let mut tools: Vec<String> = vec![out.tool.clone()];
    for (session, mut found) in parts {
        tag(&mut found, &session);
        out.reading |= found.reading;
        out.session_tokens = match (out.session_tokens, found.session_tokens) {
            (Some(a), Some(b)) => Some(a.plus(b)),
            (a, b) => a.or(b),
        };
        if out.counted == TokensCounted::NotRecorded {
            out.counted = found.counted;
        }
        if !tools.contains(&found.tool) {
            tools.push(found.tool.clone());
        }
        out.agents.extend(found.agents);
        out.cleans.extend(found.cleans);
        out.sessions.extend(found.sessions);
    }
    out.live = true;
    out.tool = tools.join(", ");
    Some(out)
}
