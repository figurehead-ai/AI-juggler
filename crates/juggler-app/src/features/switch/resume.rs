//! AJ-E209, AJ-E2908: resuming an epoch, or an ended session on its own
//! account.

use std::sync::Arc;

use juggler_adapter::LaunchMode;
use juggler_core::model::SessionId;

use crate::commands::{lock_storage_arc, not_found_or_core};
use crate::error::{AppError, AppResult};
use crate::features::launch::dto::LaunchResult;
use crate::ids::unique_suffix;
use crate::state::AppState;

/// Finds one epoch (any project) with its session row + account —
/// the graph actions' shared read side.
pub(crate) fn find_epoch_with_session(
    state: &Arc<AppState>,
    epoch_id: &str,
) -> AppResult<(
    juggler_core::model::Epoch,
    juggler_core::model::InstanceRecord,
    juggler_core::model::Account,
)> {
    let storage = lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let wanted = juggler_core::model::EpochId::new(epoch_id.to_owned());
    let mut found = None;
    for project in storage.projects().list()? {
        for work in graph.works_for_project(&project.id)? {
            if let Some(epoch) = graph
                .epochs_for_work(&work.id)?
                .into_iter()
                .find(|row| row.id == wanted)
            {
                found = Some(epoch);
            }
        }
    }
    let epoch = found.ok_or_else(|| AppError::Io {
        context: "graph action",
        message: "That point in the work isn't recorded any more".to_owned(),
    })?;
    let account = storage
        .accounts()
        .get(&epoch.account)
        .map_err(|error| not_found_or_core(error, "account", &epoch.account.to_string()))?;
    let session = epoch
        .session_ref
        .as_ref()
        .and_then(|id| storage.sessions().get(id).ok());
    drop(storage);
    let session = match (session, &epoch.native_id) {
        (Some(session), _) => session,
        // AJ-E2108: work that began as a session another app runs: that
        // conversation as a switch reads it (from the index; no row).
        (None, Some(native)) if epoch.session_ref.is_none() => {
            crate::features::conversations::switch_source::as_switch_source(
                state,
                account.id.as_str(),
                native,
                false,
            )?
            .0
        }
        (None, _) => {
            return Err(AppError::Io {
                context: "graph action",
                message: "The epoch has no session row".to_owned(),
            });
        }
    };
    Ok((epoch, session, account))
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn epoch_resume_impl(state: &Arc<AppState>, epoch_id: &str) -> AppResult<LaunchResult> {
    let (epoch, session_row, account) = find_epoch_with_session(state, epoch_id)?;
    // AJ-E2108: a conversation another app runs is continued here as
    // "Continue in AI Juggler" does (refused while it's open there).
    if crate::features::conversations::switch_source::is_source_key(session_row.id.as_str())
        && let Some(native) = epoch.native_id.as_deref()
    {
        return crate::features::conversations::continue_impl(state, account.id.as_str(), native);
    }
    let native_id = epoch
        .native_id
        .clone()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| AppError::Io {
            context: "resume epoch",
            message: "That point in the work has no conversation AI Juggler can resume".to_owned(),
        })?;
    resume_conversation(state, &session_row, &account, native_id, "resume")
}

/// Resumes `native_id`, `session_row`'s conversation, as a new session on
/// `account` with the row's launch settings and folder: the one resume
/// path (an epoch's Resume, an ended session's Resume).
fn resume_conversation(
    state: &Arc<AppState>,
    session_row: &juggler_core::model::InstanceRecord,
    account: &juggler_core::model::Account,
    native_id: String,
    origin: &'static str,
) -> AppResult<LaunchResult> {
    let profile = {
        let storage = lock_storage_arc(state)?;
        storage
            .profiles()
            .get(&session_row.profile)
            .map_err(|error| {
                not_found_or_core(error, "profile", &session_row.profile.to_string())
            })?
    };
    let (new_session, pid) = relaunch(
        state,
        &Relaunch {
            account,
            profile: &profile,
            from: session_row,
            mode: LaunchMode::Resume {
                session_id: native_id.clone(),
                delta_prompt: None,
            },
            conversation: native_id,
            origin,
        },
    )?;
    Ok(LaunchResult {
        session_id: new_session.to_string(),
        pid,
    })
}

/// A conversation started again as a new session (Resume, Undo, switch
/// back): on `account` with `profile`'s launch settings, in `from`'s
/// folder and project, its transcript link carried.
pub(crate) struct Relaunch<'a> {
    pub account: &'a juggler_core::model::Account,
    pub profile: &'a juggler_core::model::Profile,
    pub from: &'a juggler_core::model::InstanceRecord,
    pub mode: LaunchMode,
    /// The conversation's native id.
    pub conversation: String,
    pub origin: &'static str,
}

/// Starts `r` the one way (ADR-0010 §1: the row first, then the bridge and
/// the account's key in the launch, under the supervisor). Returns the new
/// session and its pid.
///
/// # Errors
///
/// An unknown tool, the launch's or the spawn's error.
pub(crate) fn relaunch(state: &Arc<AppState>, r: &Relaunch<'_>) -> AppResult<(SessionId, u32)> {
    let adapter = state
        .registry
        .get(&r.account.harness)
        .ok_or_else(|| AppError::UnknownHarness(r.account.harness.to_string()))?;
    let new_session = SessionId::new(format!("sess-{}", unique_suffix()));
    let spec = adapter.launch_spec(r.account, r.profile, &r.from.cwd, &r.mode)?;
    let started = crate::features::sessions::spawn::start_session(
        state,
        &crate::features::sessions::spawn::NewSession {
            id: &new_session,
            account: r.account,
            profile: &r.profile.id,
            project: r.from.project.clone(),
            cwd: r.from.cwd.clone(),
            conversation: Some(juggler_core::model::ConversationId::new(
                r.conversation.clone(),
            )),
            transcript_ref: r.from.transcript_ref.clone(),
            origin: r.origin,
        },
        crate::features::sessions::spawn::Runs::Terminal(spec),
    )?;
    Ok((new_session, started.spawned.pid))
}

/// AJ-E2908: Resume on an ended session's page — the same conversation,
/// on the same account and launch settings, in its folder.
///
/// # Errors
///
/// [`AppError::Refused`] while it still runs, or when it has no
/// conversation to resume; as [`epoch_resume_impl`] otherwise.
pub fn session_resume_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<LaunchResult> {
    let (row, account) = {
        let storage = lock_storage_arc(state)?;
        let row = storage
            .sessions()
            .get(&SessionId::new(session_id.to_owned()))
            .map_err(|error| not_found_or_core(error, "session", session_id))?;
        let profile = storage
            .profiles()
            .get(&row.profile)
            .map_err(|error| not_found_or_core(error, "profile", &row.profile.to_string()))?;
        let account = storage
            .accounts()
            .get(&profile.account)
            .map_err(|error| not_found_or_core(error, "account", &profile.account.to_string()))?;
        (row, account)
    };
    {
        use juggler_core::model::SessionStatus as S;
        if !matches!(
            row.status,
            S::Finished | S::Stopped | S::Failed | S::HandedOver
        ) {
            return Err(AppError::Refused(
                "It's still running. Go to it instead".into(),
            ));
        }
    }
    let native = row
        .conversation
        .as_ref()
        .map(|id| id.as_str().to_owned())
        .or_else(|| {
            row.transcript_ref
                .as_ref()
                .and_then(|reference| reference.native_id.clone())
        })
        .filter(|id| !id.is_empty())
        .ok_or_else(|| AppError::Refused("It has no conversation to resume yet".into()))?;
    resume_conversation(state, &row, &account, native, "resume")
}
