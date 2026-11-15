//! AJ-E3013 (D-116, D-119): what a desktop app's session gets through AI
//! Juggler's connector besides its tools: the inbox note its hooks print,
//! and a wake when a pool message may start its turn.
//!
//! The connector's hooks (`juggler-agent hook <event> --connector`) send a
//! report with no key. The caller is attested from the system exactly as a
//! keyless call is ([`super::attest`]); then, on the events that take a
//! note ([`juggler_agentbridge::hook::NOTE_EVENTS`]), the app answers with
//! the facts only: how many unread pool messages and from whom, never a
//! message's words (`messages::delivery::note_for`). Nothing else of the
//! session is recorded from it, and an unattested hook gets a refusal
//! (the hook prints nothing and never fails).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use juggler_agentbridge::wire::{self, Refusal, Response};
use juggler_core::agent_access::{AgentAction, Decision};
use juggler_core::model::SessionId;
use juggler_core::storage::{RecipientState, SENDER_USER, ThreadState};
use juggler_platform::local_ipc::Peer;

use crate::state::AppState;

/// D-119: the most wake waits held open at once (each a connector of a
/// desktop session; never one of the calls' slots).
pub const WAITS_MAX: usize = 32;
/// How often a wait looks for a message that may wake its session.
pub const LOOK_EVERY: Duration = Duration::from_secs(5);
/// Our wake budget, on top of Claude Code's own: at most one wake a
/// session in this long.
pub const WAKE_EVERY: Duration = Duration::from_secs(60);

/// Per desktop session: when it was last woken, and the newest message
/// (epoch ms) a wake or a check already accounted for.
type Woken = HashMap<String, (Option<Instant>, i64)>;

fn woken() -> &'static Mutex<Woken> {
    static WOKEN: OnceLock<Mutex<Woken>> = OnceLock::new();
    WOKEN.get_or_init(Mutex::default)
}

fn waits() -> &'static Arc<tokio::sync::Semaphore> {
    static WAITS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    WAITS.get_or_init(|| Arc::new(tokio::sync::Semaphore::new(WAITS_MAX)))
}

/// The session a message's sender ref stands for (a session, a Work
/// item's live session, or an attested desktop session's conversation).
fn sender_session(storage: &juggler_core::storage::Storage, sender: &str) -> Option<SessionId> {
    if let Some((folder, conversation)) = sender
        .strip_prefix("conversation:")
        .and_then(|rest| rest.split_once('/'))
    {
        return Some(juggler_core::storage::attested::desktop_id(
            folder,
            conversation,
        ));
    }
    crate::features::messages::live_session_of(storage, sender)
}

/// D-119: the facts-only notice that wakes `session` now, if a message
/// came that may: an agent's (a person's message waits for its turn), in
/// an open thread of a pool neither paused nor stopped, that the sender's
/// pool lets it deliver into a member's session within the hop limit
/// (`allowed(deliver_to_session)`: the pool at Full, or a rule the user
/// made), and at most once in [`WAKE_EVERY`].
#[must_use]
pub fn wake_for(state: &Arc<AppState>, session: &SessionId) -> Option<String> {
    let now = Instant::now();
    let (last, seen) = woken()
        .lock()
        .ok()?
        .get(session.as_str())
        .copied()
        .unwrap_or((None, 0));
    if last.is_some_and(|at| now.duration_since(at) < WAKE_EVERY) {
        return None;
    }
    let (rows, newest) = {
        let storage = state.storage.lock().ok()?;
        let refs = crate::features::messages::refs_of(&storage, session);
        let repo = storage.messages();
        let rows: Vec<_> = repo
            .inbox(&refs, &[RecipientState::Unread], None, None, 50)
            .ok()?
            .into_iter()
            .filter(|row| row.message.created_at > seen)
            .collect();
        let newest = rows
            .iter()
            .map(|row| row.message.created_at)
            .max()
            .unwrap_or(seen);
        let open: Vec<_> = rows
            .into_iter()
            .filter(|row| row.message.sender_kind != SENDER_USER)
            .filter(|row| {
                let pool = &row.message.pool_id;
                repo.pool_pause(pool).ok().flatten().is_none()
                    && !storage
                        .agent_access()
                        .access(pool)
                        .is_ok_and(|access| access.delivery_stopped)
                    && repo
                        .thread(&row.message.thread_id)
                        .ok()
                        .flatten()
                        .is_some_and(|thread| thread.state == ThreadState::Open)
            })
            .filter_map(|row| Some((sender_session(&storage, &row.message.sender_ref)?, row)))
            .collect();
        (open, newest)
    };
    let mut senders: Vec<String> = Vec::new();
    let mut count = 0;
    for (sender, row) in &rows {
        let taken = row.message.hop.saturating_sub(1);
        if crate::features::pools::access::allowed(
            state,
            sender.as_str(),
            &row.message.pool_id,
            AgentAction::DeliverToSession,
            Some(taken),
        ) == Decision::Allow
        {
            count += 1;
            if !senders.contains(&row.message.sender_name) {
                senders.push(row.message.sender_name.clone());
            }
        }
    }
    let notice = juggler_core::messaging::unread_note(count, &senders);
    if let Ok(mut woken) = woken().lock() {
        let entry = woken
            .entry(session.as_str().to_owned())
            .or_insert((None, 0));
        entry.1 = entry.1.max(newest);
        if notice.is_some() {
            entry.0 = Some(now);
        }
    }
    notice
}

/// D-119: a connector's wake wait. The caller is attested (or refused);
/// its App must be one whose sessions can be woken; then it's held until
/// a message may wake it ([`wake_for`]) or [`wire::WAIT_MAX_SECS`] pass.
pub async fn wait(state: Arc<AppState>, wait: wire::Wait, peer: Peer) -> Response {
    if wait.v != wire::VERSION || !wait.token.is_empty() {
        return Response::refused(Refusal::Bad, "AI Juggler couldn't read that request.");
    }
    let Ok(_slot) = Arc::clone(waits()).try_acquire_owned() else {
        return Response::ok(serde_json::json!({ "wake": null }));
    };
    let attested = {
        let state = Arc::clone(&state);
        tauri::async_runtime::spawn_blocking(move || super::attest::attest(&state, peer)).await
    };
    let caller = match attested {
        Ok(Ok(caller)) => caller,
        Ok(Err(why)) => return Response::refused(Refusal::Unattested, why),
        Err(_) => return Response::refused(Refusal::Bad, "AI Juggler couldn't answer that"),
    };
    if !caller.can_wake {
        return Response::refused(Refusal::Off, "This app's sessions can't be woken.");
    }
    // What came before the first wait isn't a wake (its note says it).
    if let Ok(mut woken) = woken().lock() {
        woken
            .entry(caller.session.as_str().to_owned())
            .or_insert((None, juggler_core::model::Timestamp::now().millis()));
    }
    let until =
        Instant::now() + Duration::from_secs(u64::from(wait.wait_secs.min(wire::WAIT_MAX_SECS)));
    loop {
        let found = {
            let state = Arc::clone(&state);
            let session = caller.session.clone();
            tauri::async_runtime::spawn_blocking(move || wake_for(&state, &session))
                .await
                .ok()
                .flatten()
        };
        if let Some(notice) = found {
            return Response::ok(serde_json::json!({ "wake": notice }));
        }
        if Instant::now() + LOOK_EVERY > until {
            return Response::ok(serde_json::json!({ "wake": null }));
        }
        tokio::time::sleep(LOOK_EVERY).await;
    }
}

/// Answers a connector hook's keyless report.
#[must_use]
pub fn report(state: &Arc<AppState>, report: &wire::Report, peer: Option<Peer>) -> Response {
    if report.v != wire::VERSION || !wire::HOOK_EVENTS.contains(&report.hook.event.as_str()) {
        return Response::refused(Refusal::Bad, "AI Juggler couldn't read that report.");
    }
    // L7: limited by its peer before any attestation's work.
    if !super::calls::peer_within_rate(state, peer) {
        return Response::refused(Refusal::Limit, "Too many reports in a short time.");
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
        super::calls::whose(&storage, &caller.session, Some(&caller.facts)).is_ok_and(
            |(project, account, _)| {
                super::settings::allows(
                    &super::settings::read(&storage),
                    project.as_deref(),
                    &account,
                )
                .is_ok()
            },
        )
    });
    let note = (allowed
        && juggler_agentbridge::hook::NOTE_EVENTS.contains(&report.hook.event.as_str()))
    .then(|| crate::features::messages::delivery::note_for(state, &caller.session))
    .flatten();
    Response::ok(serde_json::json!({ "note": note }))
}
