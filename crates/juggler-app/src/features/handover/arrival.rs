//! 0.3.2 — after a switch launches, did the conversation arrive? The
//! maintainer's walk: the new session opened on Claude Code's setup screen
//! and AI Juggler said "switched", because a spawned process was taken as
//! proof. The launch is now watched for up to [`WATCH_MS`]: the target's
//! tool lists its live sessions ([`juggler_adapter::live`]); the switch
//! holds when the new process runs the conversation that was installed.
//! Only evidence says otherwise: the process ended in that time, or it
//! runs a different conversation. Then a notice says so, with the way on;
//! the original is never touched. A tool that lists nothing is no
//! evidence either way, and nothing is said.

use std::sync::Arc;
use std::time::Duration;

use juggler_core::model::{Account, SessionId};
use juggler_core::storage::NewNotice;

use crate::features::notifications::centre::{self, NoticeFacts, NoticeTarget};
use crate::state::AppState;

/// How long a launch is watched.
pub const WATCH_MS: u64 = 30_000;
const STEP_MS: u64 = 1_000;

/// What the watch saw.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Arrival {
    /// The new process runs the installed conversation.
    Arrived,
    /// The new process runs another conversation (a fresh one, or setup).
    Elsewhere,
    /// The new process ended before it was seen with the conversation.
    Ended,
    /// No evidence either way.
    Unknown,
}

/// One look: the tool's live sessions in the target folder, by pid.
#[must_use]
pub fn look(
    live: &[juggler_adapter::live::LiveSession],
    pid: u32,
    native_id: &str,
    ended: bool,
) -> Arrival {
    if live.iter().any(|session| session.session_id == native_id) {
        return Arrival::Arrived;
    }
    if live.iter().any(|session| session.pid == Some(pid)) {
        return Arrival::Elsewhere;
    }
    if ended {
        Arrival::Ended
    } else {
        Arrival::Unknown
    }
}

/// Watches a transplant's launch: the conversation it installed.
pub fn watch_switch(
    state: &Arc<AppState>,
    target: &Account,
    session: &SessionId,
    pid: u32,
    record: &juggler_handover::pipeline::SwitchRecord,
) {
    let installed = record
        .install
        .reference
        .native_id
        .as_deref()
        .unwrap_or_default();
    watch(state, target, session, pid, installed);
}

/// Watches the launch in the background; tells only on evidence.
pub fn watch(
    state: &Arc<AppState>,
    target: &Account,
    session: &SessionId,
    pid: u32,
    native_id: &str,
) {
    let (state, target, session, native_id) = (
        Arc::clone(state),
        target.clone(),
        session.clone(),
        native_id.to_owned(),
    );
    std::thread::spawn(move || {
        let Some(adapter) = state.registry.get(&target.harness) else {
            return;
        };
        let mut seen = Arrival::Unknown;
        for _ in 0..(WATCH_MS / STEP_MS) {
            let ended = state.supervisor.is_finished(&session);
            seen = look(
                &adapter.live_sessions(&target.state_dir),
                pid,
                &native_id,
                ended,
            );
            if seen != Arrival::Unknown {
                break;
            }
            std::thread::sleep(Duration::from_millis(STEP_MS));
        }
        eprintln!("ai-juggler: switch {session} arrival: {seen:?}");
        if matches!(seen, Arrival::Elsewhere | Arrival::Ended) {
            tell(&state, &target.label, &session, &seen);
        }
    });
}

/// The notice: what happened and the way on.
fn tell(state: &Arc<AppState>, target: &str, session: &SessionId, seen: &Arrival) {
    let detail = match seen {
        Arrival::Ended => format!(
            "The new session on {target} closed before it opened the conversation. The original is untouched: switch again, or continue it where it was."
        ),
        _ => format!(
            "The new session on {target} opened without the conversation (its tool may be asking to be set up). The original is untouched: open the new session to check, or switch again."
        ),
    };
    let notice = NewNotice {
        key: format!("switch-unconfirmed:{session}"),
        kind: "failed".to_owned(),
        title: format!("The switch to {target} didn't carry the conversation"),
        detail,
        data: centre::data_of(
            &NoticeTarget {
                session_id: Some(session.to_string()),
                ..NoticeTarget::default()
            },
            &NoticeFacts::default(),
        ),
        action: Some("open".to_owned()),
        ..NewNotice::default()
    };
    let now = crate::commands::now_millis();
    let recorded = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| centre::record(&storage, &notice, now).ok().flatten());
    if recorded.is_some() {
        crate::features::notifications::changed(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_adapter::live::LiveSession;

    fn live(id: &str, pid: u32) -> LiveSession {
        LiveSession {
            session_id: id.to_owned(),
            pid: Some(pid),
            ..LiveSession::default()
        }
    }

    #[test]
    fn only_evidence_says_it_didnt_arrive() {
        assert_eq!(
            look(&[live("conv-1", 40)], 40, "conv-1", false),
            Arrival::Arrived
        );
        assert_eq!(
            look(&[live("fresh", 40)], 40, "conv-1", false),
            Arrival::Elsewhere,
            "its process runs another conversation"
        );
        assert_eq!(look(&[], 40, "conv-1", true), Arrival::Ended);
        assert_eq!(
            look(&[], 40, "conv-1", false),
            Arrival::Unknown,
            "a tool that lists nothing is no evidence"
        );
        assert_eq!(
            look(&[live("other", 77)], 40, "conv-1", false),
            Arrival::Unknown,
            "another process's session says nothing"
        );
    }
}
