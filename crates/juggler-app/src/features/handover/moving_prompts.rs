//! 0.5.1 F93 (PAT-OSP, AJ-E1547): every system prompt a move can cause,
//! checked at its offer and asked on the Move screen, before anything
//! quits; never inside the move. In the PL1 walk the link sheet, asked
//! inside the move, held one with both Apps closed for as long as it
//! waited (600 s once).
//!
//! What each path of a move, its Undo, a repair and startup recovery can
//! make the system ask (the registry, `docs/design/os-prompts.md`):
//!
//! - **Quit** (`moving_quit`, by [`crate::features::ownership::ProcId`]):
//!   a signal to that exact process. Asks nothing.
//! - **Start** (`places::start`, `apps::open_app`): an app the system
//!   would ask about first (`first-open`) is refused, never started. Asks
//!   nothing.
//! - **The link** to the target after a move, and to the source after
//!   the user's Undo (`automation-app-links`, the system's Automation
//!   dialog, once per app): the move sends it only in the background
//!   ([`sent`]), so nothing is ever asked mid-move. The Move screen's
//!   Move button and each Undo button ask its sheet first, while nothing
//!   has quit (the offer's `prompts`, a move record's `prompts`).
//!   Declined (Not now), the move still runs, sends no link and says to
//!   pick the conversation in the App's sidebar or use Open. Once primed,
//!   the system's own dialog comes with that link, both Apps started.
//! - **Files**: the tools' own folders and the moves' own; a worktree's
//!   files stay where they are, so no guarded folder (`documents`,
//!   `desktop`, `downloads`, `app-data`) is read.
//! - **Repair**, **Finish** and **startup recovery** start Apps again and
//!   send no link. A move never calls Go to (`goto.rs`: Accessibility,
//!   System Events, terminal tabs); Open after it is the user's click,
//!   with its own sheets.
//! - The bell's notices a move leaves are background: a system
//!   notification waits for its own priming.

use std::sync::Arc;

use juggler_platform::os_prompt::{self, Gate, Gated, Prompt, Status};

use crate::os_prompts::registry;
use crate::state::AppState;

/// The prompts a move with desktop Apps can cause.
pub(super) const MOVE_PROMPTS: &[Prompt] = &[registry::APP_LINKS];

/// What a link the move may not send yet says: none was sent.
pub(super) const LINKS_WAIT: &str = "not sent: AI Juggler may not send links to apps yet";

/// The ids of [`MOVE_PROMPTS`] a move (`apps`: with desktop Apps) can
/// cause that the user hasn't been told of: not primed in `gate`, and
/// not decided by the system (`status`). Empty: nothing to ask first.
pub(super) fn unprimed(
    gate: Gate<'_>,
    status: &dyn Fn(Prompt) -> Status,
    apps: bool,
) -> Vec<String> {
    if !apps {
        return Vec::new();
    }
    MOVE_PROMPTS
        .iter()
        .filter(|prompt| {
            !gate.primed(prompt.id)
                && !matches!(status(**prompt), Status::Allowed | Status::NotAllowed)
        })
        .map(|prompt| prompt.id.to_owned())
        .collect()
}

/// [`unprimed`] on this computer: the app's ledger and the system's word.
pub(super) fn unprimed_now(state: &Arc<AppState>, apps: bool) -> Vec<String> {
    unprimed(
        os_prompt::gate(),
        &|prompt| crate::os_prompts::status_of(state, prompt),
        apps,
    )
}

/// A link sent in the background, as the move sends every one: never a
/// sheet mid-move. Not primed yet: none sent ([`LINKS_WAIT`]).
///
/// # Errors
///
/// The system's reason, or [`LINKS_WAIT`].
pub(super) fn sent(gated: Gated<std::io::Result<()>>) -> Result<(), String> {
    match gated {
        Gated::Called { value, .. } => value.map_err(|error| error.to_string()),
        Gated::NotNow | Gated::Waiting => Err(LINKS_WAIT.to_owned()),
    }
}
