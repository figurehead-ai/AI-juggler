//! The turn review over IPC (AJ-E3820): the code-only review at once,
//! the model's lines after it when the Beta is on (never blocking the
//! page), and "This is wrong" about them.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use juggler_core::storage::memory::{Actor, NewEvent, NewSidecar, Stream};

use super::dto::{ReviewAsk, TurnReviewInfo};
use super::lines::{Written, write};
use super::read::read;
use crate::error::{AppError, AppResult};
use crate::features::ai::dto::AiJob;
use crate::features::ai::engine::Stop;
use crate::features::ai::models::Job;
use crate::features::ai::run::LiveJob;
use crate::features::ai::service;
use crate::state::AppState;

/// The most time the model's lines take, every attempt included.
pub const LINES_WAIT: Duration = Duration::from_secs(45);
/// Model results kept (a turn's lines are written once).
const KEPT_MAX: usize = 128;
/// How long a "This is wrong" keeps the lines it was about.
const WRONG_KEPT_MS: i64 = 30 * 86_400_000;

/// What the model wrote for a turn, by the turn and its reply: the lines
/// and the model's name, or `None` when every draft was refused.
type Kept = HashMap<(String, u64), Option<(Written, String)>>;

static KEPT: LazyLock<Mutex<Kept>> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn hash(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// The code-only review of the turn `ask` names (R1; under 100 ms).
#[must_use]
pub fn review_turn_impl(state: &Arc<AppState>, ask: &ReviewAsk) -> TurnReviewInfo {
    read(state, ask).review
}

/// The review with the model's lines (R2), when the router serves the
/// review job (its flag, a `beta` or `ships` verdict, its Beta on, an
/// engine ready); otherwise, or when every draft is refused, the
/// code-only review unchanged.
#[must_use]
pub fn review_turn_ai_impl(state: &Arc<AppState>, ask: &ReviewAsk) -> TurnReviewInfo {
    let read = read(state, ask);
    let mut review = read.review;
    if review.state != "shown" || read.reply.is_empty() {
        return review;
    }
    let key = (review.turn.clone(), hash(&read.reply));
    let kept = KEPT.lock().ok().and_then(|kept| kept.get(&key).cloned());
    let done = if let Some(done) = kept {
        done
    } else {
        {
            let service = service::shared();
            let live = LiveJob {
                service,
                state,
                job: AiJob::Review,
            };
            let Ok(routed) = service::route_on(service, state, Job::Review) else {
                return review;
            };
            let stop = Stop::at(Instant::now() + LINES_WAIT);
            let (written, attempts) = write(
                &live,
                &review,
                &read.reply,
                &read.removed,
                read.failed,
                &stop,
            );
            for (n, attempt) in attempts.iter().enumerate() {
                tracing::info!(
                    target: "review",
                    attempt = n,
                    refused = attempt.refused.as_deref().unwrap_or("passed"),
                    "turn review lines"
                );
            }
            let done = written.map(|written| (written, routed.name));
            if let Ok(mut kept) = KEPT.lock() {
                if kept.len() >= KEPT_MAX {
                    kept.clear();
                }
                kept.insert(key, done.clone());
            }
            done
        }
    };
    if let Some((written, model)) = done {
        written.apply(&mut review, &model, &read.reply);
    }
    review
}

/// "This is wrong" about a turn review's model lines, or its Undo:
/// kept on this Mac only, in the AI log (ADR-0036 §1.8), the lines in a
/// sidecar for the next harness round.
///
/// # Errors
///
/// Storage failures.
pub fn review_wrong_impl(
    state: &Arc<AppState>,
    turn: &str,
    lines: &str,
    wrong: bool,
) -> AppResult<()> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let now = state.notifier.clock().now;
    let what = if wrong { "wrong" } else { "wrong_undone" };
    let mut event = NewEvent::new(now, Stream::Feedback, what, Actor::User);
    event.subject = Some(("turn_review", turn.to_owned()));
    let sidecar = NewSidecar {
        body: lines,
        untrusted: true,
        expires_ms: now + WRONG_KEPT_MS,
    };
    storage
        .ai_log()
        .append(
            &event,
            (wrong && !lines.is_empty()).then_some((
                &sidecar,
                &crate::features::ai::memory::RedactionSecrets
                    as &dyn juggler_core::storage::memory::SecretScan,
            )),
        )
        .map_err(|err| AppError::Io {
            context: "turn review",
            message: err.to_string(),
        })?;
    Ok(())
}

/// The code-only review (see [`review_turn_impl`]).
///
/// # Errors
///
/// None of its own; a task that can't finish.
#[tauri::command]
pub async fn review_turn(
    state: tauri::State<'_, Arc<AppState>>,
    ask: ReviewAsk,
) -> AppResult<TurnReviewInfo> {
    crate::ipc::blocking(state.inner(), "turn review", move |state| {
        Ok(review_turn_impl(state, &ask))
    })
    .await
}

/// The review with the model's lines (see [`review_turn_ai_impl`]).
///
/// # Errors
///
/// None of its own; a task that can't finish.
#[tauri::command]
pub async fn review_turn_ai(
    state: tauri::State<'_, Arc<AppState>>,
    ask: ReviewAsk,
) -> AppResult<TurnReviewInfo> {
    crate::ipc::blocking(state.inner(), "turn review lines", move |state| {
        Ok(review_turn_ai_impl(state, &ask))
    })
    .await
}

/// "This is wrong" (see [`review_wrong_impl`]).
///
/// # Errors
///
/// As [`review_wrong_impl`].
#[tauri::command]
pub async fn review_wrong(
    state: tauri::State<'_, Arc<AppState>>,
    turn: String,
    lines: String,
    wrong: Option<bool>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "turn review", move |state| {
        review_wrong_impl(state, &turn, &lines, wrong.unwrap_or(true))
    })
    .await
}
