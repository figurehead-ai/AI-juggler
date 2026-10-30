//! AJ-E1703: context cards — what each member of a pool is working on,
//! built only from what AI Juggler knows (D-06: a field it doesn't know
//! says "Unavailable" and why, never a guess).
//!
//! - Name, place, tool, state and last activity: the read model (the
//!   sessions list, without any account). The place is the rows' place
//!   line (D-36): project · app · profile.
//! - The files changed most recently and the step the session's own task
//!   list has in progress: read from each live session's transcript on
//!   the read model's 2 s tick, only what was appended since the last
//!   read (`juggler_adapters::activity`, each tool's parser). A session
//!   that ended keeps what was read while it ran, until AI Juggler quits.
//!   A live session whose tool didn't name its file is linked to it the
//!   way a switch finds it (never a guess), looked for every 10 s.
//! - The task it's on (E19): the one it holds in progress, in review or
//!   blocked, through its Work item (so a switch keeps it).
//! - What the agent said of itself: its `set_status` line through the
//!   agent bridge (AJ-E1704) when it set one, else the step its own task
//!   list has in progress.
//!
//! Of the account, a card carries only its profile's label (D-35), never
//! an email, a plan or any other detail (D-33): it is what an agent will
//! be shown (AJ-E1704). Nothing here starts, stops or types into a session.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_adapters::{ActivityLog, ActivityTail};
use juggler_core::model::{ProjectId, SessionId};
use juggler_core::storage::{MemberRef, Memberships, PoolRole};

use super::members::role_rank;
use super::{CardField, ContextCard, PoolCards};
use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::sessions::dto::InstanceInfo;
use crate::state::AppState;

/// The app AI Juggler's own sessions run in, as the rail names it.
const OWN_APP: &str = "AI Juggler";
/// Why a card's activity isn't known.
const NOT_READ: &str = "read only while it runs in AI Juggler";
/// What a card says when its member holds no task (known: none).
const NO_TASK: &str = "None";
/// Why a card's task can't be read.
const NO_TASKS: &str = "AI Juggler is busy: its tasks couldn't be read";

/// Why a live session's activity isn't known yet.
const NOT_YET: &str = "its transcript hasn't been read yet";
/// How long a live session whose transcript isn't linked waits before the
/// tick looks for it again.
const LINK_EVERY: Duration = Duration::from_secs(10);

/// Each session's activity, read on the tick. The tick reads transcripts
/// into its own map; readers see the last published view.
#[derive(Debug, Default)]
pub struct Activity {
    /// The tick's own: each live session's transcript, as read so far.
    work: Mutex<HashMap<SessionId, (PathBuf, ActivityTail)>>,
    /// What the tick last published, per session.
    view: Mutex<HashMap<SessionId, ActivityLog>>,
    /// When the tick last looked for an unlinked session's transcript.
    looked: Mutex<HashMap<SessionId, Instant>>,
}

impl Activity {
    /// What was read of a session's transcript; `None` when it hasn't
    /// been (it isn't running here, or has no transcript yet).
    #[must_use]
    pub fn of(&self, session: &SessionId) -> Option<ActivityLog> {
        self.view.lock().ok()?.get(session).cloned()
    }

    /// Whether the tick may look for `session`'s transcript now (at most
    /// once every [`LINK_EVERY`]).
    fn may_look(&self, session: &SessionId, live: &[SessionId]) -> bool {
        let Ok(mut looked) = self.looked.lock() else {
            return false;
        };
        looked.retain(|id, _| live.contains(id));
        let now = Instant::now();
        if looked
            .get(session)
            .is_some_and(|at| now.duration_since(*at) < LINK_EVERY)
        {
            return false;
        }
        looked.insert(session.clone(), now);
        true
    }
}

/// A live session's transcript: its link, else the file its tool made,
/// found as a switch finds it (by its conversation's id when the tool
/// named it, else the one file made in its folder since it started; never
/// a guess) and linked, so the next tick and every other reader have it.
///
/// 0.2 lab: the card said "read only while it runs in AI Juggler" for
/// sessions running here. A headless Claude Code session names its
/// conversation but not its file; a driven Codex thread names its rollout
/// before Codex writes it (so the link wasn't kept); a terminal session
/// names it only through its hooks; and a link whose file is gone was
/// never looked for again.
fn transcript_of(
    state: &Arc<AppState>,
    session: &SessionId,
    linked: Option<PathBuf>,
    live: &[SessionId],
) -> Option<PathBuf> {
    if let Some(path) = linked.filter(|path| path.exists()) {
        return Some(path);
    }
    if !state.pool_activity.may_look(session, live) {
        return None;
    }
    crate::commands::handover_source_linked(state, session.as_str())
        .ok()?
        .0
        .transcript_ref
        .map(|reference| reference.path)
}

/// Reads what each live session's transcript gained and publishes it.
/// A session that ended keeps its last view; its reader is dropped.
pub fn tick(state: &Arc<AppState>) {
    let live: Vec<(SessionId, Option<PathBuf>)> = {
        let Ok(storage) = state.storage.lock() else {
            return;
        };
        // Only the sessions that haven't ended: never the whole history.
        storage
            .sessions()
            .live()
            .unwrap_or_default()
            .into_iter()
            .map(|record| (record.id, record.transcript_ref.map(|r| r.path)))
            .collect()
    };
    let ids: Vec<SessionId> = live.iter().map(|(id, _)| id.clone()).collect();
    let live: Vec<(SessionId, PathBuf)> = live
        .into_iter()
        .filter_map(|(session, linked)| {
            let path = transcript_of(state, &session, linked, &ids)?;
            Some((session, path))
        })
        .collect();
    // Only the tick takes `work` (one tick at a time); readers use `view`.
    let Ok(mut work) = state.pool_activity.work.lock() else {
        return;
    };
    let mut previous = std::mem::take(&mut *work);
    let mut updates: Vec<(SessionId, ActivityLog)> = Vec::new();
    for (session, path) in live {
        let first = !previous.contains_key(&session);
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
        // Read for the first time (even an empty file): what it says is
        // known now, if only that nothing changed yet.
        if read || first || old_path != path {
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

/// One pool's cards for its members in the working set (what an agent
/// is shown, AJ-E1704): the lead first, then members, then observers, the
/// most recently active first within each. Archived sessions are left out
/// (they're history). The pool page reads all its members a page at a
/// time instead ([`super::members::members_page`]).
///
/// # Errors
///
/// [`crate::error::AppError::Refused`] for a pool that isn't there;
/// storage failures.
pub fn cards_impl(state: &Arc<AppState>, pool_id: &str) -> AppResult<PoolCards> {
    let (memberships, projects) = {
        let storage = lock_storage_arc(state)?;
        super::live_pool(&storage, pool_id)?;
        (
            super::members::memberships(&storage)?,
            super::project_names(&storage)?,
        )
    };
    let rows = crate::commands::instances_shared_snapshot(state)?;
    let mut cards: Vec<(PoolRole, ContextCard)> = rows
        .into_iter()
        .filter(|row| row.lifecycle.as_deref() != Some("archived"))
        .filter_map(|row| {
            let project = row.project_id.as_deref().map(ProjectId::new);
            let session = SessionId::new(row.id.as_str());
            let (place, role) = memberships.place(&session, project.as_ref());
            (place == pool_id).then(|| (role, card_for(state, &row, role, &projects, &memberships)))
        })
        .collect();
    cards.sort_by(|(a_role, a), (b_role, b)| {
        role_rank(*a_role)
            .cmp(&role_rank(*b_role))
            .then(b.last_activity_ms.cmp(&a.last_activity_ms))
    });
    Ok(PoolCards {
        pool_id: pool_id.to_owned(),
        cards: cards.into_iter().map(|(_, card)| card).collect(),
    })
}

/// A member's card: its row, what its transcript said, and what it set
/// with `set_status` (AJ-E1704: that takes the said line's place, kept
/// per member, so across switches).
pub(super) fn card_for(
    state: &Arc<AppState>,
    row: &InstanceInfo,
    role: PoolRole,
    projects: &HashMap<String, String>,
    memberships: &Memberships,
) -> ContextCard {
    let session = SessionId::new(row.id.as_str());
    let activity = state.pool_activity.of(&session);
    let member = MemberRef::of_session(&session, memberships.work_of(&session)).to_wire();
    let status = state.agent_bridge.status_of(&member);
    card_of(
        state,
        row,
        role.as_str().to_owned(),
        projects,
        activity.as_ref(),
        status,
    )
}

/// The task a session's member is on (E19), as `title (in progress)`
/// (D-25: the card is on screen, never its id).
fn task_of(state: &Arc<AppState>, session: &str) -> CardField {
    let Ok(storage) = state.storage.lock() else {
        return CardField::unavailable(NO_TASKS);
    };
    let refs = crate::features::messages::refs_of(&storage, &SessionId::new(session));
    match storage.tasks().held_by(&refs) {
        Ok(Some(task)) => CardField::known(format!(
            "{} ({})",
            task.title,
            task.status.words().to_lowercase()
        )),
        Ok(None) => CardField::known(NO_TASK),
        Err(_) => CardField::unavailable(NO_TASKS),
    }
}

/// Whether a row's state (its wire word) says the session hasn't ended.
fn is_live(status: &str) -> bool {
    juggler_core::model::SessionStatus::parse(status)
        .is_some_and(crate::features::messages::is_live)
}

/// One card from a session's row and what its transcript said. Of the
/// row's account only its profile's label goes in, and not when it is an
/// email: in the place line and as the card's account.
fn card_of(
    state: &Arc<AppState>,
    row: &InstanceInfo,
    role: String,
    projects: &HashMap<String, String>,
    activity: Option<&ActivityLog>,
    status: Option<crate::features::agent_bridge::Said>,
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
    let profile = Some(row.account_label.trim())
        .filter(|label| !label.is_empty() && !label.contains('@'))
        .map(str::to_owned);
    // D-36: the rows' place line, project · app · profile.
    let place = [Some(project.as_str()), Some(OWN_APP), profile.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    // Not read: a session running here is read on the tick once its
    // transcript is found; one that ended was read only while it ran.
    let why = if is_live(&row.status) {
        NOT_YET
    } else {
        NOT_READ
    };
    let (said, files, files_unavailable) = activity.map_or_else(
        || {
            (
                CardField::unavailable(why),
                Vec::new(),
                Some(format!("Unavailable — {why}")),
            )
        },
        |log| {
            (
                match log.step() {
                    Some(step) => CardField::known(step),
                    None if log.has_plan() => {
                        CardField::unavailable("nothing on its task list is in progress")
                    }
                    None => CardField::unavailable("it hasn't said"),
                },
                log.files().to_vec(),
                None,
            )
        },
    );
    let said = status.map_or(said, |status| CardField::known(status.text));
    ContextCard {
        session_id: row.id.clone(),
        name,
        place,
        tool: crate::features::tools::display_name(&state.registry, &row.provider),
        account: profile,
        state: row.status.clone(),
        role,
        task: task_of(state, &row.id),
        said,
        last_activity_ms: row.last_activity_ms,
        files,
        files_unavailable,
    }
}
