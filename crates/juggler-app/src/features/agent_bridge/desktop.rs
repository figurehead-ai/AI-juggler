//! AJ-E3013 (D-116): what a desktop app's session gets through AI
//! Juggler's connector besides its tools: the inbox note its hooks print.
//!
//! The connector's hooks (`juggler-agent hook <event> --connector`) send a
//! report with no key. The caller is attested from the system exactly as a
//! keyless call is ([`super::attest`]); then, on the events that take a
//! note ([`juggler_agentbridge::hook::NOTE_EVENTS`]), the app answers with
//! the facts only: how many unread pool messages and from whom, never a
//! message's words (`messages::delivery::note_for`). Nothing else of the
//! session is recorded from it, and an unattested hook gets a refusal
//! (the hook prints nothing and never fails).

use std::sync::Arc;

use juggler_agentbridge::wire::{self, Refusal, Response};
use juggler_platform::local_ipc::Peer;

use crate::state::AppState;

/// Answers a connector hook's keyless report.
#[must_use]
pub fn report(state: &Arc<AppState>, report: &wire::Report, peer: Option<Peer>) -> Response {
    if report.v != wire::VERSION || !wire::HOOK_EVENTS.contains(&report.hook.event.as_str()) {
        return Response::refused(Refusal::Bad, "AI Juggler couldn't read that report.");
    }
    let caller = match peer
        .ok_or_else(|| "the system didn't name the caller".to_owned())
        .and_then(|peer| super::attest::attest(state, peer))
    {
        Ok(caller) => caller,
        Err(why) => return Response::refused(Refusal::Unattested, why),
    };
    // The tools' own switch (Settings › Agent access) governs the note too.
    let allowed = crate::commands::lock_storage_arc(state).is_ok_and(|storage| {
        super::calls::whose(&storage, &caller.session).is_ok_and(|(project, account, _)| {
            super::settings::allows(
                &super::settings::read(&storage),
                project.as_deref(),
                &account,
            )
            .is_ok()
        })
    });
    let note = (allowed
        && juggler_agentbridge::hook::NOTE_EVENTS.contains(&report.hook.event.as_str()))
    .then(|| crate::features::messages::delivery::note_for(state, &caller.session))
    .flatten();
    Response::ok(serde_json::json!({ "note": note }))
}
