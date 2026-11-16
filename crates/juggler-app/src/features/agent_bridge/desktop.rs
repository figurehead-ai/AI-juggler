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
use juggler_core::storage::{RecipientState, SENDER_USER};
use juggler_platform::local_ipc::Peer;

use crate::state::AppState;

/// D-119: the most wake waits held open at once (each a connector of a
/// desktop session, one a session; never one of the calls' slots).
pub const WAITS_MAX: usize = 64;
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

/// D-119 (M2): the wake waits held now: a slot each, one a session.
#[derive(Debug)]
pub struct Waits {
    slots: Arc<tokio::sync::Semaphore>,
    waiting: Mutex<std::collections::HashSet<String>>,
}

impl Default for Waits {
    fn default() -> Self {
        Self {
            slots: Arc::new(tokio::sync::Semaphore::new(WAITS_MAX)),
            waiting: Mutex::default(),
        }
    }
}

/// One session's wait, held until it ends (then another may start).
struct Held<'w> {
    waits: &'w Waits,
    session: String,
}

impl<'w> Held<'w> {
    /// The session's one wait, unless it already has one.
    fn take(waits: &'w Waits, session: &SessionId) -> Option<Self> {
        let mut waiting = waits.waiting.lock().ok()?;
        waiting.insert(session.as_str().to_owned()).then(|| Self {
            waits,
            session: session.as_str().to_owned(),
        })
    }
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        if let Ok(mut waiting) = self.waits.waiting.lock() {
            waiting.remove(&self.session);
        }
    }
}

/// Who a wait's peer is (the attestation; a test's stand-in).
pub type Attestor =
    Arc<dyn Fn(&Arc<AppState>, Peer) -> Result<super::attest::DesktopCaller, String> + Send + Sync>;

/// D-119: the facts-only notice that wakes `session` now, if a message
/// came that may: an agent's (a person's message waits for its turn), in
/// an open thread of a pool neither paused nor stopped, that the sender's
/// pool lets it deliver into a member's session within the hop limit
/// (`allowed(deliver_to_session)`: the pool at Full, or a rule the user
/// made), and at most once in [`WAKE_EVERY`]. Marks what it read as
/// accounted for (and the budget spent, when it wakes).
#[must_use]
pub fn wake_for(state: &Arc<AppState>, session: &SessionId) -> Option<String> {
    let look = look_for(state, session)?;
    mark(session, &look);
    look.notice
}

/// What one look found for a session, before anything is marked.
#[derive(Clone, Debug)]
pub(crate) struct Look {
    /// The notice that may wake it now.
    pub notice: Option<String>,
    /// The newest message (epoch ms) the look read.
    newest: i64,
}

/// L8 (round 2): marks `look` as accounted for: what it read is seen,
/// and when it woke the session, the wake budget is spent now. Only after
/// the caller was attested again, when it wakes.
pub(crate) fn mark(session: &SessionId, look: &Look) {
    if let Ok(mut woken) = woken().lock() {
        let entry = woken
            .entry(session.as_str().to_owned())
            .or_insert((None, 0));
        entry.1 = entry.1.max(look.newest);
        if look.notice.is_some() {
            entry.0 = Some(Instant::now());
        }
    }
}

/// [`wake_for`]'s look, marking nothing: `None` within the wake budget
/// or when the messages can't be read.
pub(crate) fn look_for(state: &Arc<AppState>, session: &SessionId) -> Option<Look> {
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
    let (rows, newest, delivered) = {
        let storage = state.storage.lock().ok()?;
        let refs = crate::features::messages::refs_of(&storage, session);
        let repo = storage.messages();
        let rows: Vec<_> = repo
            .inbox(
                &refs,
                &[RecipientState::Unread, RecipientState::Queued],
                None,
                None,
                50,
            )
            .ok()?
            .into_iter()
            .filter(|row| when(row) > seen)
            .collect();
        let newest = rows.iter().map(when).max().unwrap_or(seen);
        // AJ-E3013 (the A9 lab walk): the person's Deliver (their compose
        // with Deliver, or the Deliver on a waiting copy) is their click,
        // the permission: it wakes the member, whoever sent it.
        let (delivered, rows): (Vec<_>, Vec<_>) =
            rows.into_iter().partition(by_the_user);
        let delivered: Vec<String> = delivered
            .into_iter()
            .map(|row| row.message.sender_name)
            .collect();
        let open: Vec<_> = rows
            .into_iter()
            .filter(|row| row.recipient.state == RecipientState::Unread)
            .filter(|row| row.message.sender_kind != SENDER_USER)
            .filter(|row| {
                crate::features::messages::delivery::may_deliver_now(&storage, &row.message)
            })
            .filter_map(|row| {
                let sender =
                    crate::features::messages::sender_session(&storage, &row.message.sender_ref)?;
                Some((sender, row))
            })
            .collect();
        (open, newest, delivered)
    };
    let mut senders: Vec<String> = Vec::new();
    let mut count = 0;
    for name in delivered {
        count += 1;
        if !senders.contains(&name) {
            senders.push(name);
        }
    }
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
    Some(Look { notice, newest })
}

/// When a copy became wake-worthy: a delivered one when the person
/// delivered it, any other when it was written.
fn when(row: &juggler_core::storage::InboxRow) -> i64 {
    if by_the_user(row) {
        row.recipient.state_at
    } else {
        row.message.created_at
    }
}

/// A copy the person delivered (queued by their click).
fn by_the_user(row: &juggler_core::storage::InboxRow) -> bool {
    row.recipient.state == RecipientState::Queued
        && row.recipient.via.as_deref() == Some(crate::features::messages::delivery::BY_USER)
}

/// D-119: a connector's wake wait. The caller is attested (or refused);
/// its App must be one whose sessions can be woken; then it's held until
/// a message may wake it ([`wake_for`]) or [`wire::WAIT_MAX_SECS`] pass.
pub async fn wait(state: Arc<AppState>, wait: wire::Wait, peer: Peer) -> Response {
    let attestor: Attestor = Arc::new(super::attest::attest);
    wait_with(state, wait, peer, attestor, LOOK_EVERY).await
}

/// [`wait`], with its attestation and how often it looks given (tests).
/// M2: a caller is limited by its peer first (L7), then attested; a
/// session in no pool gets no wait (Deny), a session holds one wait at a
/// time, and a slot is taken only after all that (Limit when none is
/// free): every refusal makes the connector back off.
pub(crate) async fn wait_with(
    state: Arc<AppState>,
    wait: wire::Wait,
    peer: Peer,
    attestor: Attestor,
    look_every: Duration,
) -> Response {
    if wait.v != wire::VERSION || !wait.token.is_empty() {
        return Response::refused(Refusal::Bad, "AI Juggler couldn't read that request.");
    }
    if let Some(refused) = super::calls::keyless_refused(
        &state,
        super::calls::Counted::peer(Some(peer)),
        "Too many waits in a short time.",
    ) {
        return refused;
    }
    let attested = {
        let (state, attestor) = (Arc::clone(&state), Arc::clone(&attestor));
        tauri::async_runtime::spawn_blocking(move || attestor(&state, peer)).await
    };
    let caller = match attested {
        Ok(Ok(caller)) => {
            super::calls::note_attested(&state, peer, caller.tool_pid);
            caller
        }
        Ok(Err(why)) => return Response::refused(Refusal::Unattested, why),
        Err(_) => return Response::refused(Refusal::Bad, "AI Juggler couldn't answer that"),
    };
    if !caller.can_wake {
        return Response::refused(Refusal::Off, "This app's sessions can't be woken.");
    }
    let in_pool = state.storage.lock().is_ok_and(|storage| {
        crate::features::pools::members::resolution_of(&storage, &caller.session)
            .ok()
            .flatten()
            .is_some_and(|resolution| !resolution.placements.is_empty())
    });
    if !in_pool {
        return Response::refused(
            Refusal::Deny,
            "This session is in no pool: nothing would wake it.",
        );
    }
    let waits = &state.agent_bridge.waits;
    let Some(_held) = Held::take(waits, &caller.session) else {
        return Response::refused(Refusal::Limit, "This session already waits.");
    };
    let Ok(_slot) = Arc::clone(&waits.slots).try_acquire_owned() else {
        return Response::refused(Refusal::Limit, "Too many sessions wait now.");
    };
    // What came before its Claude Code started isn't a wake (its note
    // says it). Not the first wait's time (the A9 lab walk: a message that
    // came while the connector's first waits were refused, the start race,
    // was taken for an old one and never woke it).
    if let Ok(mut woken) = woken().lock() {
        woken
            .entry(caller.session.as_str().to_owned())
            .or_insert((None, caller.tool_start.saturating_mul(1_000)));
    }
    let until =
        Instant::now() + Duration::from_secs(u64::from(wait.wait_secs.min(wire::WAIT_MAX_SECS)));
    loop {
        let found = {
            let state = Arc::clone(&state);
            let session = caller.session.clone();
            tauri::async_runtime::spawn_blocking(move || look_for(&state, &session))
                .await
                .ok()
                .flatten()
        };
        match found {
            Some(look) if look.notice.is_some() => {
                // L8 (round 2): still the same conversation? Asked before
                // anything is marked: a /clear or a resume during the wait
                // gets nothing, and the old conversation's messages stay
                // due and its wake budget unspent.
                let again = {
                    let (state, attestor) = (Arc::clone(&state), Arc::clone(&attestor));
                    tauri::async_runtime::spawn_blocking(move || attestor(&state, peer)).await
                };
                let same = matches!(&again, Ok(Ok(now)) if now.session == caller.session);
                if same {
                    mark(&caller.session, &look);
                }
                let wake = look.notice.filter(|_| same);
                return Response::ok(serde_json::json!({ "wake": wake }));
            }
            // Nothing that wakes: what it read is accounted for.
            Some(look) => mark(&caller.session, &look),
            None => {}
        }
        if Instant::now() + look_every > until {
            // Held as an attested wait until now: its next wait (and its
            // session's hooks) stay out of the common cap.
            super::calls::note_attested(&state, peer, caller.tool_pid);
            return Response::ok(serde_json::json!({ "wake": null }));
        }
        tokio::time::sleep(look_every).await;
    }
}

/// Who a hook's report counts as (the review's global-cap finding): a
/// hook is a new process each time, so it counts as its session's process
/// above it: its parent, or (a hook under `sh`) its grandparent when that
/// one was attested recently; its parent otherwise; itself when its parent
/// can't be read.
fn counted_report(state: &Arc<AppState>, peer: Option<Peer>) -> super::calls::Counted {
    use crate::features::conversations::procs;
    let counted = |pid: u32| super::calls::Counted {
        pid: Some(pid),
        version: juggler_proc_list::pid_version(pid),
    };
    let above = |pid: u32| procs::parent_of(pid).filter(|parent| *parent > 1);
    let Some(parent) = peer.and_then(|peer| peer.pid).and_then(above) else {
        return super::calls::Counted::peer(peer);
    };
    let first = counted(parent);
    if super::calls::recently_attested(state, first) {
        return first;
    }
    above(parent)
        .map(counted)
        .filter(|grandparent| super::calls::recently_attested(state, *grandparent))
        .unwrap_or(first)
}

/// Answers a connector hook's keyless report.
#[must_use]
pub fn report(state: &Arc<AppState>, report: &wire::Report, peer: Option<Peer>) -> Response {
    if report.v != wire::VERSION || !wire::HOOK_EVENTS.contains(&report.hook.event.as_str()) {
        return Response::refused(Refusal::Bad, "AI Juggler couldn't read that report.");
    }
    // L7: limited by its session's process before any attestation's work.
    if let Some(refused) = super::calls::keyless_refused(
        state,
        counted_report(state, peer),
        "Too many reports in a short time.",
    ) {
        return refused;
    }
    let caller = match peer
        .ok_or_else(|| "the system didn't name the caller".to_owned())
        .and_then(|peer| {
            let caller = super::attest::attest(state, peer)?;
            super::calls::note_attested(state, peer, caller.tool_pid);
            Ok(caller)
        }) {
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
