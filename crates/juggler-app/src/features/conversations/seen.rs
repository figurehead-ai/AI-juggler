//! 0.5.2 G45 (RW1 W1, W3): what the live look sees a session AI Juggler
//! didn't start do (one started in the user's own terminal, or one another
//! app runs), told on the observation bus as a session AI Juggler runs
//! tells its own (`notifications::session_status`), so a watch on it, on
//! its App or project, or on any session hears it: the same engine, no
//! second rule (ADR-0036 §2.3, the transitions `ToolObserves::backed_by`
//! waited for, AJ-E3705).
//!
//! What is told, by its key where a session id goes
//! ([`super::switch_source::source_key`]):
//!
//! - **Opened** (a look finds it open that the last look didn't):
//!   `Started`, and its state if it is working or asking. Idle at its
//!   start isn't a turn's end ("finishes" never fires on an open).
//! - **Its state moved** (working, idle, asking; "done, review it" is
//!   idle): `StateEntered`. Working, then idle, is the turn's end.
//! - **Closed** (no look finds it open): `Ended`.
//!
//! Never before the index was read whole and one look made after it: a
//! start sees every session already open, which neither opened nor moved
//! then (an idle timer armed before a restart keeps its time, G12). A
//! headless run (`claude -p`, `codex exec`) is never told (D-102), nor a
//! conversation AI Juggler's own session runs (its supervisor tells it).

use std::collections::HashMap;
use std::sync::Arc;

use super::live::Live;
use super::scan::{LiveEntry, State};
use crate::features::observe::{EndKind, Observed, SessionState};
use crate::state::AppState;

/// A state as a watch condition reads it; "done, review it" is idle (its
/// turn ended).
const fn observed(state: State) -> SessionState {
    match state {
        State::Working => SessionState::Working,
        State::AsksYou => SessionState::AsksYou,
        State::Idle | State::DoneReview => SessionState::Idle,
    }
}

/// What changed from one look (`was`) to the next (`now`), by key, in key
/// order.
#[must_use]
pub fn transitions<S: std::hash::BuildHasher, T: std::hash::BuildHasher>(
    was: &HashMap<Live, LiveEntry, S>,
    now: &HashMap<Live, LiveEntry, T>,
) -> Vec<(Live, Vec<Observed>)> {
    let interactive = |entry: &&LiveEntry| !entry.background;
    let mut out: Vec<(Live, Vec<Observed>)> = Vec::new();
    for (key, entry) in now.iter().filter(|(_, e)| interactive(e)) {
        let state = entry.state.map(observed);
        match was.get(key).filter(interactive) {
            None => {
                let mut events = vec![Observed::Started];
                if let Some(state @ (SessionState::Working | SessionState::AsksYou)) = state {
                    events.push(Observed::StateEntered { state });
                }
                out.push((key.clone(), events));
            }
            Some(before) => {
                if let Some(state) = state
                    && before.state.map(observed) != Some(state)
                {
                    out.push((key.clone(), vec![Observed::StateEntered { state }]));
                }
            }
        }
    }
    for (key, _) in was.iter().filter(|(_, e)| interactive(e)) {
        if now.get(key).is_none_or(|entry| entry.background) {
            out.push((
                key.clone(),
                vec![Observed::Ended {
                    how: EndKind::Clean,
                }],
            ));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Tells what changed, leaving out a conversation an AI Juggler session
/// runs (as every list reads it, `commands::stands_for`). Takes the
/// storage lock for a moment; the caller holds none.
pub fn tell(state: &Arc<AppState>, changes: Vec<(Live, Vec<Observed>)>, at_ms: i64) {
    if changes.is_empty() {
        return;
    }
    let links: HashMap<(String, String), juggler_core::storage::ConversationLink> =
        match crate::commands::lock_storage_arc(state)
            .map_err(drop)
            .and_then(|storage| storage.sessions().conversation_links().map_err(drop))
        {
            Ok(links) => links
                .into_iter()
                .map(|link| ((link.account.clone(), link.conversation.clone()), link))
                .collect(),
            // Unread: nothing is told rather than an own session twice.
            Err(()) => return,
        };
    for ((account, tool, id), events) in changes {
        let live = state.conversations.live_entry(&account, &tool, &id);
        let run = links.get(&(account.clone(), id.clone()));
        if super::commands::stands_for(run, live.as_ref()).is_some() {
            continue;
        }
        crate::features::observe::publish_for_session(
            state,
            &super::switch_source::source_key(&account, &id),
            events,
            at_ms,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(state: Option<State>) -> LiveEntry {
        LiveEntry {
            tool: "claude-code".into(),
            account_id: Some("acct-a".into()),
            session_id: "c-1".into(),
            places: Vec::new(),
            state,
            probably: false,
            viewed_ms: None,
            app_title: None,
            background: false,
        }
    }

    fn key() -> Live {
        ("acct-a".into(), "claude-code".into(), "c-1".into())
    }

    fn look(state: Option<State>) -> HashMap<Live, LiveEntry> {
        HashMap::from([(key(), entry(state))])
    }

    fn events(changes: &[(Live, Vec<Observed>)]) -> Vec<Observed> {
        changes.iter().flat_map(|(_, e)| e.clone()).collect()
    }

    #[test]
    fn an_open_is_a_start_and_idle_at_the_start_is_no_turn_ending() {
        let none = HashMap::new();
        assert_eq!(
            events(&transitions(&none, &look(Some(State::Idle)))),
            [Observed::Started]
        );
        assert_eq!(
            events(&transitions(&none, &look(Some(State::Working)))),
            [
                Observed::Started,
                Observed::StateEntered {
                    state: SessionState::Working
                }
            ]
        );
    }

    #[test]
    fn working_then_idle_is_told_and_the_same_state_again_is_not() {
        let working = look(Some(State::Working));
        assert_eq!(
            events(&transitions(&working, &look(Some(State::Idle)))),
            [Observed::StateEntered {
                state: SessionState::Idle
            }]
        );
        assert!(transitions(&working, &working).is_empty());
        // Done, review it is idle: no move from idle.
        assert!(transitions(&look(Some(State::Idle)), &look(Some(State::DoneReview))).is_empty());
        // A state no longer known says nothing.
        assert!(transitions(&working, &look(None)).is_empty());
    }

    #[test]
    fn a_close_is_an_ending_and_a_headless_run_is_never_told() {
        assert_eq!(
            events(&transitions(&look(Some(State::Idle)), &HashMap::new())),
            [Observed::Ended {
                how: EndKind::Clean
            }]
        );
        let mut headless = look(Some(State::Working));
        headless.get_mut(&key()).unwrap().background = true;
        assert!(transitions(&HashMap::new(), &headless).is_empty());
        assert!(transitions(&headless, &HashMap::new()).is_empty());
    }
}
