//! E19 (D-28, D-31): the tasks a pool shares — epics, stories and tasks,
//! with honest states and atomic claims.
//!
//! - **Agents write the tasks; the user sees and tracks them** (D-44,
//!   2026-10-27). The app makes no task, and edits, assigns or links
//!   none: the user manages tasks through the agents. Its only changes
//!   are the user's four ([`oversight`]): approve or send back a task in
//!   review, free one whose holder's session ended, cancel one; and it
//!   starts a session for one ([`launch`]).
//! - **One set of operations** ([`apply`]) for an agent's call and the
//!   user's answer to an agent's held ask: create, claim, change, give
//!   back, assign, and ask for a session. Each
//!   checks the task rules (`juggler_core::tasks`: the lead plans and
//!   assigns; a member works what it holds) and writes through the store,
//!   which logs every change.
//! - **The agent's tools** ([`agent`]): every call goes through
//!   `allowed(pool, action)` (D-28), which decides and audits. A change
//!   above the pool's level is held for the user's OK in Needs you
//!   ([`needs`]); nothing is refused silently.
//! - **No agent starts a session or picks an account** (D-31). An agent
//!   may only ask; the user's click in Needs you or on the task starts
//!   one, on the launch settings the user picks ([`launch`]).
//! - **Tasks follow the work**: a task's holder is a member ref (its Work
//!   item once it has one), so a switch keeps it, and every session that
//!   worked it is linked (the switch history shows it); a switch is a line
//!   in its history ([`switched`]).
//! - The user's reads ([`views`]), the issue a task comes from
//!   ([`issues`]) and the **commands** ([`commands`]) the window calls.
//!
//! Hard lines (D-28): nothing here changes a pool, a level or a rule for
//! an agent, starts a session for an agent, picks an account, reads a
//! credential or quota, or answers a tool's own prompt.

pub mod agent;
pub mod apply;
pub mod commands;
pub mod dto;
pub mod issues;
pub mod launch;
pub mod needs;
pub mod oversight;
pub mod switched;
pub mod views;

use std::sync::{Arc, Mutex};

use juggler_core::storage::{Storage, TASK_USER};
use juggler_core::tasks::Role;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Event: tasks changed (payload: the pools' ids).
pub const EVENT_CHANGED: &str = "tasks://changed";

/// The user's name in a task's history.
pub const YOU: &str = "You";

/// Tells the window which pools' tasks changed.
pub type Notifier = Arc<dyn Fn(&str) + Send + Sync>;

/// The tasks' state in the app: what tells the window, and where the
/// user's `gh` is when a test says. Everything else is in storage.
#[derive(Default)]
pub struct Tasks {
    notifier: Mutex<Option<Notifier>>,
    pub(crate) gh: Mutex<Option<std::path::PathBuf>>,
}

impl std::fmt::Debug for Tasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tasks").finish_non_exhaustive()
    }
}

impl Tasks {
    /// Sets what tells the window.
    pub fn set_notifier(&self, notifier: Notifier) {
        if let Ok(mut slot) = self.notifier.lock() {
            *slot = Some(notifier);
        }
    }

    /// Uses `path` as the user's `gh` (tests; `None` goes back to the
    /// usual places).
    pub fn set_gh(&self, path: Option<std::path::PathBuf>) {
        if let Ok(mut slot) = self.gh.lock() {
            *slot = path;
        }
    }

    /// Tells the window `pool`'s tasks changed.
    pub fn notify(&self, pool: &str) {
        let notifier = self.notifier.lock().ok().and_then(|slot| slot.clone());
        if let Some(notifier) = notifier {
            notifier(pool);
        }
    }
}

/// Starts telling the window about changes (at most every 250 ms, one
/// event a burst).
pub fn start_emitter(state: &Arc<AppState>, app: tauri::AppHandle) {
    use tauri::Emitter;
    let dirty: Arc<Mutex<std::collections::BTreeSet<String>>> = Arc::default();
    let marks = Arc::clone(&dirty);
    state.tasks.set_notifier(Arc::new(move |pool| {
        if let Ok(mut dirty) = marks.lock() {
            dirty.insert(pool.to_owned());
        }
    }));
    let weak = Arc::downgrade(state);
    // poll: coalesces a burst of task changes into one event for the window;
    // ends with the app.
    crate::ticks::every_while(
        "tasks-emitter",
        std::time::Duration::from_millis(250),
        move || {
            if weak.upgrade().is_none() {
                return false;
            }
            let pools: Vec<String> = dirty
                .lock()
                .map(|mut d| std::mem::take(&mut *d).into_iter().collect())
                .unwrap_or_default();
            if !pools.is_empty() {
                let _ = app.emit(EVENT_CHANGED, pools);
            }
            true
        },
    );
}

/// Who acts on a task: an agent of the pool, or the user.
#[derive(Clone, Debug)]
pub struct Actor {
    /// Its session (none for the user).
    pub session: Option<juggler_core::model::SessionId>,
    /// Its member ref (`work:…`, `session:…`), or `user`.
    pub member_ref: String,
    /// Its name, as the pool shows it.
    pub name: String,
    /// Its role in the pool (the user owns every pool: the lead's rights).
    pub role: Role,
    /// Every ref a task it holds may be under (its Work item's sessions).
    pub refs: Vec<String>,
}

impl Actor {
    /// The user, in the app.
    #[must_use]
    pub fn user() -> Self {
        Self {
            session: None,
            member_ref: TASK_USER.to_owned(),
            name: YOU.to_owned(),
            role: Role::Lead,
            refs: vec![TASK_USER.to_owned()],
        }
    }

    /// Whether this is the user.
    #[must_use]
    pub fn is_user(&self) -> bool {
        self.member_ref == TASK_USER
    }

    /// (ref, name), as the store takes them.
    #[must_use]
    pub fn who(&self) -> (&str, &str) {
        (&self.member_ref, &self.name)
    }
}

/// A pool role's word as the task rules see it.
#[must_use]
pub fn role_of(word: &str) -> Role {
    match juggler_core::storage::PoolRole::parse(word) {
        Some(juggler_core::storage::PoolRole::Lead) => Role::Lead,
        Some(juggler_core::storage::PoolRole::Observer) => Role::Observer,
        _ => Role::Member,
    }
}

/// A free short id (`aj-` and 4 hex digits; 6, then 8, when those are
/// taken).
pub(crate) fn new_short_id(storage: &Storage) -> AppResult<String> {
    for len in [2_usize, 2, 2, 3, 3, 4, 4, 6] {
        let mut bytes = vec![0_u8; len];
        if getrandom::fill(&mut bytes).is_err() {
            bytes = juggler_core::model::Timestamp::now().millis().to_le_bytes()[..len].to_vec();
        }
        let id = juggler_core::tasks::short_id(&bytes);
        if storage.tasks().id_free(&id)? {
            return Ok(id);
        }
    }
    Err(AppError::Refused(
        "AI Juggler couldn't find a free id for the task: try again".to_owned(),
    ))
}

/// A pool's name as the app shows it.
pub(crate) fn pool_name(storage: &Storage, pool_id: &str) -> AppResult<String> {
    crate::features::messages::pool_name(storage, pool_id)
}
