//! D-29: a delivered message typed as the next prompt of a session AI
//! Juggler runs in its terminal (rc31 walk: two Lab Claude Code sessions
//! in a pool at Full sat Idle with a message "Waiting in its inbox" for
//! minutes; delivery reached driven sessions only, and a Claude session
//! in a terminal heard its hooks' note only once it ran a turn, which an
//! idle one never does).
//!
//! - **Only a tool that says how** ([`juggler_adapter::ToolRuntime::typed_prompt`]):
//!   any other tool's terminal is never typed into.
//! - **Only at its idle prompt, as its hooks said** ([`at_its_prompt`]):
//!   never before its first report, at a question on its screen, mid-turn
//!   or once it ended. The delivery pump adds the rest of its checks (the
//!   user's typing, the pool's permission, the hop limit).
//! - **Through the one write to a terminal** (`terminal::type_prompt`).

use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::runtime::typed::TypedPrompt;
use juggler_core::model::SessionId;
use juggler_core::storage::Storage;

use crate::state::AppState;

/// How long the tool gets to take the text in before Return sends it.
const SETTLE: Duration = Duration::from_millis(150);

/// How `session`'s tool takes `text` as a typed prompt, when it can.
/// (`storage`: the caller's lock, never taken twice.)
fn prompt_of(
    state: &AppState,
    storage: &Storage,
    session: &SessionId,
    text: &str,
) -> Option<TypedPrompt> {
    let record = storage.sessions().get(session).ok()?;
    let profile = storage.profiles().get(&record.profile).ok()?;
    let harness = storage.accounts().get(&profile.account).ok()?.harness;
    state.registry.get(&harness)?.typed_prompt(text)
}

/// Whether a message can be typed into `session`: AI Juggler runs it in
/// its terminal, and its tool says how a prompt is typed (`storage`: the
/// caller's lock).
#[must_use]
pub fn typable(state: &AppState, storage: &Storage, session: &SessionId) -> bool {
    !state.runtime.is_driven(session)
        && state.supervisor.is_supervised(session)
        && !state.supervisor.is_finished(session)
        && prompt_of(state, storage, session, "").is_some()
}

/// Whether `session`'s tool waits at its idle prompt, as its own hooks
/// said last: a session that never ran a turn included (its
/// `SessionStart`), never one whose first report is still awaited or
/// overdue, or one that asks something.
#[must_use]
pub fn at_its_prompt(state: &AppState, session: &SessionId) -> bool {
    state
        .runtime
        .observed(session.as_str())
        .is_some_and(|seen| {
            seen.state == "idle" && seen.ask.is_none() && !seen.is_first_report_overdue()
        })
}

/// Types `text` as `session`'s next prompt.
///
/// # Errors
///
/// In words, when its tool can't take one or its terminal is gone (the
/// message then stays queued).
pub fn type_turn(state: &Arc<AppState>, session: &SessionId, text: &str) -> Result<(), String> {
    let typed = {
        let storage = state
            .storage
            .lock()
            .map_err(|_| "the store is busy".to_owned())?;
        prompt_of(state, &storage, session, text)
    }
    .ok_or_else(|| "its tool can't take a typed prompt".to_owned())?;
    crate::terminal::type_prompt(state, session, &typed.text).map_err(|e| e.to_string())?;
    std::thread::sleep(SETTLE);
    crate::terminal::type_prompt(state, session, &typed.submit).map_err(|e| e.to_string())
}
