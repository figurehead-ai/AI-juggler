//! AJ-E2903: how an Account's sessions went — D-07's measurable signals
//! per Account over the last 7 days, each with its source, and no
//! composite score anywhere.
//!
//! A session's errors, turns and context at its end come from its own
//! transcript, read once soon after it ends ([`settle_soon`]) and kept on
//! its usage record ([`EndQuality`]); an Account's page and the session's
//! Usage tab read only what's kept. Stuck episodes are AI Juggler's own
//! recorded state changes, so sessions it didn't watch aren't counted.

pub mod commands;
pub mod transcript;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use juggler_core::model::{EndQuality, SessionStatus, Timestamp};
use serde::Serialize;
use ts_rs::TS;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::state::AppState;

/// The section's window: the last 7 days (wf07).
pub const WINDOW_MS: i64 = 7 * 86_400_000;
/// Sessions ended longer ago than this aren't settled (their figures
/// would never show).
const SETTLE_WITHIN_MS: i64 = 30 * 86_400_000;
/// At most this many transcripts read per settling.
const SETTLE_BATCH: usize = 20;

static SETTLING: AtomicBool = AtomicBool::new(false);

/// Whether a status is an ending.
#[must_use]
pub fn ended(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Finished
            | SessionStatus::Failed
            | SessionStatus::Stopped
            | SessionStatus::HandedOver
            | SessionStatus::QuotaBlocked
    )
}

/// Keeps the end figures of sessions that ended without them, the newest
/// first, a batch at a time. Returns how many it kept.
///
/// # Errors
///
/// Storage errors.
pub fn settle_ended(state: &Arc<AppState>) -> AppResult<usize> {
    let now = Timestamp::now().millis();
    let due = {
        let storage = lock_storage_arc(state)?;
        let last = storage.sessions().last_transitions()?;
        let mut due: Vec<_> = storage
            .sessions()
            .list(None)?
            .into_iter()
            .filter(|record| ended(record.status) && record.usage.at_end.is_none())
            .filter_map(|record| {
                let at = last.get(record.id.as_str()).map_or(0, |entry| entry.2);
                (at >= now - SETTLE_WITHIN_MS).then_some((at, record))
            })
            .collect();
        due.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        due.truncate(SETTLE_BATCH);
        due
    };
    let mut kept = 0;
    for (_, record) in due {
        // The transcript is read without the store's lock held.
        let scan = crate::features::quality::transcript::scan_transcript(&record, now);
        let quality = EndQuality {
            errors: scan.errors,
            turns: scan.turns,
            context_used: scan.context_used,
            context_window: scan.context_window,
        };
        let storage = lock_storage_arc(state)?;
        let Ok(mut fresh) = storage.sessions().get(&record.id) else {
            continue;
        };
        if fresh.usage.at_end.is_none() && ended(fresh.status) {
            fresh.usage.at_end = Some(quality);
            storage.sessions().update(&fresh)?;
            kept += 1;
        }
    }
    Ok(kept)
}

/// Settles in the background unless a settling already runs (called when
/// the sessions change: an ending is a change).
pub fn settle_soon(state: &Arc<AppState>) {
    if SETTLING.swap(true, Ordering::AcqRel) {
        return;
    }
    let state = Arc::clone(state);
    let spawned = std::thread::Builder::new()
        .name("aij-quality-settle".into())
        .spawn(move || {
            let _ = settle_ended(&state);
            SETTLING.store(false, Ordering::Release);
        });
    if spawned.is_err() {
        SETTLING.store(false, Ordering::Release);
    }
}

/// How an Account's sessions went in the window.
#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AccountQualityInfo {
    /// Its tool's id (for "Not reported by Codex").
    pub tool: String,
    /// AI Juggler's sessions on it that ended in the window.
    pub sessions: u32,
    /// Of them, the ones whose figures aren't kept yet.
    pub pending: u32,
    /// Errors and retries, over the sessions whose tool counts turns.
    #[ts(type = "number")]
    pub errors: u64,
    /// Turns, over the same sessions.
    #[ts(type = "number")]
    pub turns: u64,
    /// Some session's tool counted turns (else errors can't be counted).
    pub errors_measured: bool,
    /// Times its sessions went probably stuck in the window.
    pub stuck_episodes: u32,
    /// The average share of the context window in use at the end, where
    /// the tool reported it.
    pub context_avg_pct: Option<f64>,
    /// How many sessions reported their context.
    pub context_sessions: u32,
    /// Finished tasks (D-77) in the window.
    pub finished: u32,
    /// Their new tokens on average.
    #[ts(type = "number | null")]
    pub tokens_per_finished: Option<u64>,
}

/// New tokens, as Usage counts them.
fn io(tokens: &juggler_core::model::TokenCounts) -> u64 {
    tokens
        .input
        .saturating_add(tokens.cache_write)
        .saturating_add(tokens.output)
        .saturating_add(tokens.reasoning)
}

/// How the sessions on `folders` went in the 7 days before `now`.
///
/// # Errors
///
/// Storage errors.
pub fn account_quality_impl(
    state: &Arc<AppState>,
    folders: &[String],
    now: i64,
) -> AppResult<AccountQualityInfo> {
    let since = now - WINDOW_MS;
    let wanted: HashSet<&str> = folders.iter().map(String::as_str).collect();
    let storage = lock_storage_arc(state)?;
    // Every launch setting on the wanted folders (removed ones too: their
    // sessions are still the folder's).
    let mut folder_of: HashMap<String, String> = HashMap::new();
    for folder in folders {
        let account = juggler_core::model::AccountId::new(folder.as_str());
        for recipe in storage.profiles().list_all_by_account(&account)? {
            folder_of.insert(recipe.id.to_string(), folder.clone());
        }
    }
    let tool = storage
        .accounts()
        .list()?
        .into_iter()
        .find(|account| wanted.contains(account.id.as_str()))
        .map(|account| account.harness.to_string())
        .unwrap_or_default();
    let last = storage.sessions().last_transitions()?;
    let stuck = storage.sessions().stuck_episodes_since(since)?;
    let mut out = AccountQualityInfo {
        tool,
        ..AccountQualityInfo::default()
    };
    let (mut pct_sum, mut finished_tokens) = (0.0_f64, 0_u64);
    for record in storage.sessions().list(None)? {
        let mine = folder_of
            .get(record.profile.as_str())
            .is_some_and(|folder| wanted.contains(folder.as_str()));
        if !mine {
            continue;
        }
        out.stuck_episodes += stuck.get(record.id.as_str()).copied().unwrap_or(0);
        let ended_at = last.get(record.id.as_str()).map_or(0, |entry| entry.2);
        if !ended(record.status) || ended_at < since {
            continue;
        }
        out.sessions += 1;
        match record.usage.at_end {
            Some(at_end) => {
                if at_end.turns > 0 {
                    out.errors_measured = true;
                    out.errors += at_end.errors;
                    out.turns += at_end.turns;
                }
                if let (Some(used), Some(window)) = (at_end.context_used, at_end.context_window)
                    && window > 0
                {
                    #[allow(clippy::cast_precision_loss)] // token counts far below 2^52
                    let share = used as f64 / window as f64;
                    pct_sum += (share * 100.0).min(100.0);
                    out.context_sessions += 1;
                }
            }
            None => out.pending += 1,
        }
        if record.status == SessionStatus::Finished {
            out.finished += 1;
            finished_tokens = finished_tokens.saturating_add(io(&record.usage.tokens));
        }
    }
    if out.context_sessions > 0 {
        out.context_avg_pct = Some(pct_sum / f64::from(out.context_sessions));
    }
    if out.finished > 0 {
        out.tokens_per_finished = Some(finished_tokens / u64::from(out.finished));
    }
    Ok(out)
}
