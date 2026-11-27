//! Reading a turn for its review (AJ-E3820): the turn's own transcript,
//! through the readers already built (the feed's turn reader for what
//! changed, the page's last-messages reader for the reply, redacted),
//! the agent's reply through Bot's input guard. Nothing here runs a
//! command or reads git.
//!
//! **R5: no review for a turn whose end AI Juggler can't see.** Only an
//! App whose tool reports each turn's end ([`Observes::TURN_ENDS`] for its
//! kind) and whose transcripts the turn reader reads gets one; a desktop
//! app's conversation, or a tool that says only that it is busy, does
//! not. [`covered_tools`] is the same rule for the router's query.
//!
//! Read only while Last messages is on (the agent's words are read, as
//! the page's conversation): off, nothing is read.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::Observes;
use juggler_adapters::turn_changes::{TurnChanges, turn_changes};
use juggler_core::model::{HarnessId, SessionId};

use super::dto::{ReviewAsk, TurnReviewInfo};
use super::facts::{Facts, review};
use crate::features::bot::guard::guard;
use crate::features::conversations::last_messages;
use crate::state::AppState;

/// A turn read for its review: the code-only review, and what the
/// model's lines need (the guarded reply and what the guard removed).
#[derive(Clone, Debug, Default)]
pub struct Read {
    /// The code-only review (or why there is none, in its `state`).
    pub review: TurnReviewInfo,
    /// The agent's reply after the guard.
    pub reply: String,
    /// What the guard took out (for the echo check; never shown).
    pub removed: Vec<String>,
    /// The turn ended failed.
    pub failed: bool,
}

impl Read {
    fn without(state: &str) -> Self {
        Self {
            review: TurnReviewInfo {
                state: state.to_owned(),
                ..TurnReviewInfo::default()
            },
            ..Self::default()
        }
    }
}

/// Whether a tool's App of this kind tells each turn's end, and the turn
/// reader reads its transcripts.
pub(crate) fn sees_turn_ends(state: &AppState, tool: &str, desktop: bool) -> bool {
    state
        .registry
        .get(&HarnessId::new(tool))
        .is_some_and(|adapter| {
            let observes = adapter.spec().observes;
            let kind = if desktop {
                observes.desktop
            } else {
                observes.cli
            };
            kind.contains(Observes::TURN_ENDS) && adapter.reports_activity()
        })
}

/// The tools whose turns are reviewed (R5, the router's query): their
/// command-line Apps tell each turn's end and their transcripts are read.
/// Tool ids, in the registry's order.
#[must_use]
pub fn covered_tools(state: &AppState) -> Vec<String> {
    state
        .registry
        .ids()
        .into_iter()
        .filter(|id| sees_turn_ends(state, id.as_str(), false))
        .map(|id| id.as_str().to_owned())
        .collect()
}

/// The transcript, its tool and whether it is a desktop app's.
fn locate(state: &Arc<AppState>, ask: &ReviewAsk) -> Option<(PathBuf, String, bool)> {
    if let (Some(account), Some(id)) = (ask.account_id.as_deref(), ask.conversation_id.as_deref())
        && let Some(found) = state.conversations.conversation_of(account, id)
    {
        let desktop = found.summary.started_in.is_some();
        return Some((found.path, found.tool, desktop));
    }
    let session = ask.session_id.as_deref()?;
    let storage = state.storage.lock().ok()?;
    let record = storage.sessions().get(&SessionId::new(session)).ok()?;
    let path = record.transcript_ref.clone()?.path;
    let profile = storage.profiles().get(&record.profile).ok()?;
    let account = storage.accounts().get(&profile.account).ok()?;
    // AI Juggler's own sessions run in a terminal.
    Some((path, account.harness.as_str().to_owned(), false))
}

/// The turn's key: the feed's (its file and start, hashed), so a review
/// and a post's diff stat name one turn alike.
fn key_of(path: &Path, changes: Option<&TurnChanges>, until: Option<i64>) -> String {
    if let Some(changes) = changes {
        return crate::features::feed::changes::info_of(path, changes.clone()).turn;
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}:t{}", until.unwrap_or(0))
}

/// Reads the turn `ask` names and builds its code-only review.
#[must_use]
pub fn read(state: &Arc<AppState>, ask: &ReviewAsk) -> Read {
    if !last_messages::enabled(state) {
        return Read::without("off");
    }
    let Some((path, tool, desktop)) = locate(state, ask) else {
        return Read::without("none");
    };
    if !sees_turn_ends(state, &tool, desktop) {
        return Read::without("unseen");
    }
    read_at(&path, ask)
}

/// [`read`](fn@read) past the gates: the transcript at `path`.
#[must_use]
pub fn read_at(path: &Path, ask: &ReviewAsk) -> Read {
    let Ok(changes) = turn_changes(path, ask.until_ms) else {
        return Read::without("unreadable");
    };
    // G48: the shared reading (results first); G49: the reply guarded
    // before any fact is read from it.
    let turn = match super::turn::read(path, ask.until_ms) {
        Ok(turn) => turn,
        Err(why) => return Read::without(why),
    };
    let guarded = guard(turn.reply.as_deref().unwrap_or(""));
    let failed = matches!(ask.ended.as_deref(), Some("failed" | "crashed"));
    let key = key_of(path, changes.as_ref(), ask.until_ms);
    let reply = guarded.cleaned;
    let review = review(&Facts {
        turn: &key,
        changes: changes.as_ref(),
        reply: Some(reply.as_str()).filter(|r| !r.is_empty()),
        failed,
        runs: Some(&turn.runs),
    });
    Read {
        review,
        reply,
        removed: guarded.removed,
        failed,
    }
}
