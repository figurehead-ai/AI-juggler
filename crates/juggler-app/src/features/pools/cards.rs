//! AJ-E1703: context cards — what each member of a pool is working on,
//! built only from what AI Juggler knows (D-06: a field it doesn't know
//! says "Unavailable" and why, never a guess).
//!
//! - Name, place, tool, state and last activity: the read model (the
//!   sessions list, without any account).
//! - The files changed most recently and the step the session's own task
//!   list has in progress: read from each live session's transcript on
//!   the read model's 2 s tick, only what was appended since the last
//!   read (`juggler_adapters::activity`, each tool's parser). A session
//!   that ended keeps what was read while it ran, until AI Juggler quits.
//! - The task it claimed: tasks are E19, so it's unavailable here.
//!
//! A card never carries an account: it is what an agent will be shown
//! (AJ-E1704, D-33). Nothing here starts, stops or types into a session.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use juggler_adapters::{ActivityLog, ActivityTail};
use juggler_core::model::SessionId;

use super::{CardField, ContextCard, PoolCards};
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::sessions::dto::InstanceInfo;
use crate::state::AppState;

/// Why a card's activity isn't known.
const NOT_READ: &str = "read only while it runs in AI Juggler";
/// Why a card's task isn't known.
const NO_TASKS: &str = "AI Juggler doesn't keep tasks yet";

/// Each session's activity, read on the tick. The tick reads transcripts
/// into its own map; readers see the last published view.
#[derive(Debug, Default)]
pub struct Activity {
    /// The tick's own: each live session's transcript, as read so far.
    work: Mutex<HashMap<SessionId, (PathBuf, ActivityTail)>>,
    /// What the tick last published, per session.
    view: Mutex<HashMap<SessionId, ActivityLog>>,
}

impl Activity {
    /// What was read of a session's transcript; `None` when it hasn't
    /// been (it isn't running here, or has no transcript yet).
    #[must_use]
    pub fn of(&self, session: &SessionId) -> Option<ActivityLog> {
        self.view.lock().ok()?.get(session).cloned()
    }
}

/// Reads what each live session's transcript gained and publishes it.
/// A session that ended keeps its last view; its reader is dropped.
pub fn tick(state: &Arc<AppState>) {
    let live: Vec<(SessionId, PathBuf)> = {
        let Ok(storage) = state.storage.lock() else {
            return;
        };
        // Only the sessions that haven't ended: never the whole history.
        storage
            .sessions()
            .live()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|record| Some((record.id, record.transcript_ref?.path)))
            .collect()
    };
    // Only the tick takes `work` (one tick at a time); readers use `view`.
    let Ok(mut work) = state.pool_activity.work.lock() else {
        return;
    };
    let mut previous = std::mem::take(&mut *work);
    let mut updates: Vec<(SessionId, ActivityLog)> = Vec::new();
    for (session, path) in live {
        let (old_path, mut tail) = previous
            .remove(&session)
            .unwrap_or_else(|| (path.clone(), ActivityTail::default()));
        if old_path != path {
            tail = ActivityTail::default();
        }
        // No transcript (yet, or any more): nothing to publish.
        let Ok(read) = tail.read(&path) else {
            continue;
        };
        if read || old_path != path {
            updates.push((session.clone(), tail.log().clone()));
        }
        work.insert(session, (path, tail));
    }
    drop(work);
    if !updates.is_empty()
        && let Ok(mut view) = state.pool_activity.view.lock()
    {
        view.extend(updates);
    }
}

/// One pool's cards: the lead first, then members, then observers, the
/// most recently active first within each. Archived sessions are left out
/// (they're history).
///
/// # Errors
///
/// [`AppError::Refused`] for a pool that isn't there; storage failures.
pub fn cards_impl(state: &Arc<AppState>, pool_id: &str) -> AppResult<PoolCards> {
    let (members, projects) = {
        let storage = lock_storage_arc(state)?;
        storage
            .pools()
            .get(pool_id)
            .map_err(|_| AppError::Refused("That pool isn't there any more".to_owned()))?;
        let info = super::pools_info(&storage)?;
        let members: HashMap<String, String> = info
            .members
            .into_iter()
            .filter(|row| row.pool_id == pool_id)
            .map(|row| (row.session_id, row.role))
            .collect();
        let projects: HashMap<String, String> = storage
            .projects()
            .list()?
            .into_iter()
            .map(|project| (project.id.as_str().to_owned(), project.name))
            .collect();
        (members, projects)
    };
    let rows = crate::commands::instances_shared_snapshot(state)?;
    let mut cards: Vec<ContextCard> = rows
        .into_iter()
        .filter(|row| row.lifecycle.as_deref() != Some("archived"))
        .filter_map(|row| {
            let role = members.get(&row.id)?.clone();
            let activity = state.pool_activity.of(&SessionId::new(row.id.as_str()));
            Some(card_of(state, &row, role, &projects, activity.as_ref()))
        })
        .collect();
    cards.sort_by(|a, b| {
        role_rank(&a.role)
            .cmp(&role_rank(&b.role))
            .then(b.last_activity_ms.cmp(&a.last_activity_ms))
    });
    Ok(PoolCards {
        pool_id: pool_id.to_owned(),
        cards,
    })
}

fn role_rank(role: &str) -> u8 {
    match role {
        "lead" => 0,
        "member" => 1,
        _ => 2,
    }
}

/// One card from a session's row and what its transcript said. Nothing
/// of the row's account goes in.
fn card_of(
    state: &Arc<AppState>,
    row: &InstanceInfo,
    role: String,
    projects: &HashMap<String, String>,
    activity: Option<&ActivityLog>,
) -> ContextCard {
    let folder = row
        .cwd
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("its folder")
        .to_owned();
    let name = row
        .label
        .clone()
        .or_else(|| row.title.clone())
        .or_else(|| row.git_branch.clone())
        .unwrap_or_else(|| folder.clone());
    let project = row
        .project_id
        .as_ref()
        .and_then(|id| projects.get(id))
        .cloned()
        .unwrap_or(folder);
    let place = match &row.git_branch {
        Some(branch) => format!("{project} · {branch}"),
        None => project,
    };
    let (said, files, files_unavailable) = match activity {
        Some(log) => (
            match log.step() {
                Some(step) => CardField::known(step),
                None if log.has_plan() => {
                    CardField::unavailable("nothing on its task list is in progress")
                }
                None => CardField::unavailable("it hasn't said"),
            },
            log.files().to_vec(),
            None,
        ),
        None => (
            CardField::unavailable(NOT_READ),
            Vec::new(),
            Some(format!("Unavailable — {NOT_READ}")),
        ),
    };
    ContextCard {
        session_id: row.id.clone(),
        name,
        place,
        tool: crate::features::tools::display_name(&state.registry, &row.provider),
        state: row.status.clone(),
        role,
        task: CardField::unavailable(NO_TASKS),
        said,
        last_activity_ms: row.last_activity_ms,
        files,
        files_unavailable,
    }
}
