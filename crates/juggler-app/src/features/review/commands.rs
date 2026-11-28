//! The turn review over IPC (AJ-E3820): the code-only review at once,
//! the model's lines after it when the Beta is on (never blocking the
//! page), and "This is wrong" about them.

use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use juggler_core::storage::memory::{Actor, NewEvent, NewSidecar, Stream};

use super::dto::{ReviewAsk, TurnReviewInfo};
use super::lines::{Written, write};
use super::read::read;
use crate::error::{AppError, AppResult};
use crate::features::ai::dto::AiJob;
use crate::features::ai::engine::Stop;
use crate::features::ai::kept::Kept;
use crate::features::ai::models::Job;
use crate::features::ai::run::LiveJob;
use crate::features::ai::service;
use crate::state::AppState;

/// The most time the model's lines take, from the ask to the lines,
/// every attempt included (0.5.2 G28): the router's line budget, ADR-0033
/// §7's "≤ 6 s cold" for a line on Apple's model (warm, ≤ 3 s p95), well
/// inside ADR-0034 §4.3's 15 s a job. The code-only review is on screen
/// first and a cue says lines are coming, so nothing waits on them; a
/// regeneration is asked for only when it can end in time
/// ([`super::lines::write`]). Measured on Apple's model (the real-model
/// test's fixture turns, 2026-11-26): one draft 1.7–2.3 s warm, 3.1 s
/// with the engine's start.
pub const LINES_WAIT: Duration = Duration::from_secs(6);
/// Model results kept (a turn's lines are written once).
const KEPT_MAX: usize = 128;
/// How long a "This is wrong" keeps the lines it was about.
const WRONG_KEPT_MS: i64 = 30 * 86_400_000;

/// What the model wrote for a turn, by the turn and its reply: the lines
/// and the model's name, or `None` when every draft was refused.
type Lines = Kept<(String, u64), Option<(Written, String)>>;

static KEPT: LazyLock<Lines> = LazyLock::new(|| Kept::new(KEPT_MAX));

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
    let asked = Instant::now();
    let read = read(state, ask);
    let mut review = read.review;
    if review.state != "shown" || read.reply.is_empty() {
        return review;
    }
    let key = (review.turn.clone(), hash(&read.reply));
    let kept = KEPT.get(&key);
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
            let stop = Stop::at(asked + LINES_WAIT);
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
                    took_ms = u64::try_from(attempt.took.as_millis()).unwrap_or(u64::MAX),
                    verdict = attempt.refused.as_deref().unwrap_or("kept"),
                    "turn review lines"
                );
            }
            let done = written.map(|written| (written, routed.name));
            KEPT.keep(key, done.clone());
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
    record_wrong(state, "turn_review", turn, lines, wrong)
}

/// "This is wrong" (or its Undo) about a model's lines on a turn, whose
/// they are by `kind` (`turn_review`, `feed_line`): the AI log on this
/// Mac only, the lines in a sidecar for the next harness round.
///
/// # Errors
///
/// Storage failures.
pub(crate) fn record_wrong(
    state: &Arc<AppState>,
    kind: &'static str,
    turn: &str,
    lines: &str,
    wrong: bool,
) -> AppResult<()> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let now = state.notifier.clock().now;
    let what = if wrong { "wrong" } else { "wrong_undone" };
    let mut event = NewEvent::new(now, Stream::Feedback, what, Actor::User);
    event.subject = Some((kind, turn.to_owned()));
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
    // Nothing waits on it: the code-only review shows first, with a cue.
    crate::ipc::blocking_with(
        state.inner(),
        "turn review lines",
        crate::ipc::Wait::Job(LINES_WAIT),
        move |state| Ok(review_turn_ai_impl(state, &ask)),
    )
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
