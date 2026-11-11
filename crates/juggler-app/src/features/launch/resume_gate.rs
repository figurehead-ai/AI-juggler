//! rc21: the one check every door that resumes a conversation asks
//! (Resume, Continue in AI Juggler, Copy the resume command, a restart,
//! an epoch's Resume, Run in AI Juggler, a switch back): its tool's
//! adapter says in words when it wouldn't open that conversation by its
//! id (`ToolLaunch::resume_refusal`). The launch is refused before any
//! process starts, no command is handed out, and the rows carry the same
//! words so a greyed control says why, with the way out beside it.

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::{HarnessAdapter, LaunchMode, LaunchSpec};
use juggler_core::model::{Account, HarnessId, Profile};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Why a conversation can't be resumed by its id, and the way out.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ResumeRefusalInfo {
    /// Why, in its tool's adapter's words.
    pub why: String,
    /// The conversation it was copied from, when a switch recorded it.
    #[ts(optional = nullable)]
    pub original: Option<OriginalConversationInfo>,
    /// How to find that one, when AI Juggler doesn't know it.
    #[ts(optional = nullable)]
    pub find: Option<String>,
}

/// The conversation a copy was made from.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OriginalConversationInfo {
    /// The account (App) it's on.
    pub account_id: String,
    /// That account's name.
    pub account_label: String,
    /// Its id.
    pub id: String,
}

/// How to find the original when no switch recorded it.
const FIND_IT: &str = "Find the conversation it was copied from in History: it has the same title, on the App it was switched from.";

/// The adapter's refusal to resume `conversation_id`, as an error in its
/// words; `Ok` when it opens it by that id.
///
/// # Errors
///
/// [`AppError::Refused`] with the adapter's words.
pub fn refuse(adapter: &dyn HarnessAdapter, conversation_id: &str) -> AppResult<()> {
    adapter
        .resume_refusal(conversation_id)
        .map_or(Ok(()), |why| Err(AppError::Refused(why)))
}

/// `adapter.launch_spec`, asked through [`refuse`] first when `mode`
/// resumes a conversation: every launch the app builds goes through here,
/// so no door resumes an id its tool won't open.
///
/// # Errors
///
/// [`refuse`]'s refusal; the adapter's launch errors.
pub fn launch_spec(
    adapter: &dyn HarnessAdapter,
    account: &Account,
    profile: &Profile,
    cwd: &Path,
    mode: &LaunchMode,
) -> AppResult<LaunchSpec> {
    if let LaunchMode::Resume { session_id, .. } = mode {
        refuse(adapter, session_id)?;
    }
    Ok(adapter.launch_spec(account, profile, cwd, mode)?)
}

/// The launch that resumes `conversation_id` on `account` (no turn
/// added), through [`launch_spec`].
///
/// # Errors
///
/// As [`launch_spec`]; an unknown tool.
pub fn resume_spec(
    state: &Arc<AppState>,
    account: &Account,
    profile: &Profile,
    cwd: &Path,
    conversation_id: &str,
) -> AppResult<LaunchSpec> {
    let adapter = adapter_of(state, account)?;
    let mode = LaunchMode::Resume {
        session_id: conversation_id.to_owned(),
        delta_prompt: None,
    };
    launch_spec(adapter.as_ref(), account, profile, cwd, &mode)
}

fn adapter_of(
    state: &Arc<AppState>,
    account: &Account,
) -> AppResult<juggler_adapter::SharedAdapter> {
    state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))
}

/// The copyable line that resumes `conversation_id` on `account` from
/// `cwd`, built by its tool's adapter; refused in words rather than a
/// line its tool won't open (Copy the resume command, a session's and a
/// conversation's alike).
///
/// # Errors
///
/// [`refuse`]'s refusal; [`AppError::Refused`] when its tool can't resume
/// from a terminal; an unknown tool.
pub fn resume_command(
    state: &Arc<AppState>,
    account: &Account,
    conversation_id: &str,
    cwd: &Path,
) -> AppResult<String> {
    let adapter = adapter_of(state, account)?;
    refuse(adapter.as_ref(), conversation_id)?;
    juggler_adapter::adapter::shell_resume_snippet(
        adapter.as_ref(),
        &account.state_dir,
        conversation_id,
        cwd,
    )
    .ok_or_else(|| {
        AppError::Refused(format!(
            "{} can't resume a session from a terminal",
            adapter.spec().display_name
        ))
    })
}

/// Why `tool` won't resume `conversation_id`, with the conversation it
/// was copied from (a switch's record) or how to find it; `None` when it
/// resumes it (or the tool isn't known).
#[must_use]
pub fn refusal_info(
    state: &Arc<AppState>,
    tool: &str,
    conversation_id: &str,
) -> Option<ResumeRefusalInfo> {
    let adapter = state.registry.get(&HarnessId::new(tool))?;
    let why = adapter.resume_refusal(conversation_id)?;
    let original = original_of(state, conversation_id);
    let find = original.is_none().then(|| FIND_IT.to_owned());
    Some(ResumeRefusalInfo {
        why,
        original,
        find,
    })
}

/// The conversation the copy `conversation_id` was made from, as the
/// switch that installed it recorded it.
fn original_of(state: &Arc<AppState>, conversation_id: &str) -> Option<OriginalConversationInfo> {
    let storage = state.storage.lock().ok()?;
    let (account, id) = storage
        .switch_graph()
        .copied_from(conversation_id)
        .ok()
        .flatten()?;
    let account_label = storage
        .accounts()
        .get(&account)
        .map(|found| found.label)
        .unwrap_or_default();
    Some(OriginalConversationInfo {
        account_id: account.to_string(),
        account_label,
        id,
    })
}

/// Session rows with their conversation's refusal, where its tool won't
/// resume it by its id.
#[must_use]
pub(crate) fn with_refusals(
    state: &Arc<AppState>,
    rows: Vec<crate::features::sessions::dto::InstanceInfo>,
) -> Vec<crate::features::sessions::dto::InstanceInfo> {
    rows.into_iter()
        .map(|row| crate::features::sessions::dto::InstanceInfo {
            resume_refusal: row
                .conversation_id
                .as_deref()
                .and_then(|id| refusal_info(state, &row.provider, id)),
            ..row
        })
        .collect()
}
