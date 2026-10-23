//! Reopening AI Juggler from inside it — first for Screen Recording, which
//! macOS reports to a process only once (the 0.0.12 report: allowed in
//! System Settings, it reads "not allowed" until the app reopens).
//!
//! A reopen ends the sessions in AI Juggler's own terminal, so with
//! sessions live it says so first and needs a second "Reopen anyway"; a
//! switch or a bring-in in progress makes it wait. AJ-E1503's quit plan
//! (release/0.1) says the same for ⌘Q; when it reaches `main`, the check
//! here becomes `quit::quit_plan_impl`. Until then it is the updater's
//! `busy_at` (switch, bring-in, live sessions).

// Tauri command signatures take State by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use crate::error::AppResult;
use crate::state::AppState;

/// Why reopening AI Juggler has to be confirmed or wait, when it does.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ReopenBlockInfo {
    /// What reopening now would cut short, in words.
    pub reason: String,
    /// True when the user may reopen anyway (live sessions); false when
    /// it has to wait (a switch or a bring-in under way).
    pub can_reopen_anyway: bool,
}

/// What a Reopen click does: relaunch now, or say why not first.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReopenDecision {
    /// Nothing would be cut short, or the user said reopen anyway.
    Relaunch,
    /// Ask (or wait) first.
    Hold(ReopenBlockInfo),
}

/// The decision for what runs now (`busy`) and whether the user already
/// said "Reopen anyway". Live sessions can be overridden — a terminal
/// session ends when AI Juggler quits, so the user hears that first; a
/// switch or a bring-in can't be cut short.
#[must_use]
pub fn reopen_decision(busy: Option<crate::updates::Busy>, anyway: bool) -> ReopenDecision {
    use crate::updates::Busy;
    match busy {
        None => ReopenDecision::Relaunch,
        Some(Busy::Sessions(_)) if anyway => ReopenDecision::Relaunch,
        Some(Busy::Sessions(live)) => ReopenDecision::Hold(ReopenBlockInfo {
            reason: format!(
                "{live} {} running. Reopening AI Juggler ends the ones in its terminal; sessions in their own app keep running.",
                if live == 1 {
                    "session is"
                } else {
                    "sessions are"
                }
            ),
            can_reopen_anyway: true,
        }),
        Some(other) => ReopenDecision::Hold(ReopenBlockInfo {
            reason: other.words("Reopen AI Juggler"),
            can_reopen_anyway: false,
        }),
    }
}

/// Reopens AI Juggler: at once when nothing would be cut short, or with
/// `anyway` after the live-session warning; otherwise returns why,
/// without relaunching.
///
/// # Errors
///
/// Returns the typed storage error reading what runs now.
#[tauri::command]
pub async fn app_relaunch(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    anyway: bool,
) -> AppResult<Option<ReopenBlockInfo>> {
    // What runs now is read off the IPC thread (ENGINEERING-GUIDE T1).
    let busy = crate::ipc::blocking(state.inner(), "what runs now", |state| {
        crate::updates::busy_at(state, juggler_core::model::Timestamp::now().millis())
    })
    .await?;
    match reopen_decision(busy, anyway) {
        ReopenDecision::Hold(block) => Ok(Some(block)),
        ReopenDecision::Relaunch => {
            // AJ-E1509: restart() skips the exit event: clean, watchdog ended.
            crate::features::app::crash::before_exit(state.inner());
            app.restart()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The block a held decision carries (`None` for a relaunch).
    fn held(decision: ReopenDecision) -> Option<ReopenBlockInfo> {
        match decision {
            ReopenDecision::Hold(block) => Some(block),
            ReopenDecision::Relaunch => None,
        }
    }

    #[test]
    fn reopening_asks_first_when_sessions_are_live_and_waits_for_a_switch() {
        use crate::updates::Busy;
        assert_eq!(reopen_decision(None, false), ReopenDecision::Relaunch);
        let two = held(reopen_decision(Some(Busy::Sessions(2)), false))
            .expect("live sessions are said first");
        assert!(two.can_reopen_anyway);
        assert!(
            two.reason.starts_with("2 sessions are running."),
            "{}",
            two.reason
        );
        let one = held(reopen_decision(Some(Busy::Sessions(1)), false))
            .expect("one live session is said first");
        assert!(
            one.reason.starts_with("1 session is running."),
            "{}",
            one.reason
        );
        assert_eq!(
            reopen_decision(Some(Busy::Sessions(2)), true),
            ReopenDecision::Relaunch,
            "Reopen anyway, after the warning"
        );
        for busy in [Busy::Switch, Busy::BringIn] {
            let block = held(reopen_decision(Some(busy), true))
                .expect("a switch or a bring-in can't be cut short, even anyway");
            assert!(!block.can_reopen_anyway);
            assert!(
                block.reason.contains("Reopen AI Juggler"),
                "{}",
                block.reason
            );
        }
    }
}
