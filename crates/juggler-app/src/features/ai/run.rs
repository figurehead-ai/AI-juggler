//! AJ-E3825: an AI job's engine, really started and used. 0.5.1's Models
//! page said "In use" from availability alone, and Bot never reached its
//! model; here the engine is started when a job first needs it (its
//! hello checked), every generation goes through it, and what happened is
//! kept: started, answering, idle again, or failed with a kind. Bot's
//! header and AJ-E3826's Models card read the same status
//! ([`AiEngineStatusInfo`], `ai_engine_status`), with today's answers and
//! tokens from the same counts.
//!
//! Every failure has a kind ([`AiFailure`]) the window says in one plain
//! line with its fix, and is logged by kind (the app log and the AI log),
//! never a silent fallback.

use std::time::{Duration, Instant};

use juggler_core::storage::memory::{Actor, NewEvent, Stream};
use serde::{Deserialize, Serialize};

use super::dto::AiJob;
use super::engine::{Draft, EngineError, Reason, Request, Started, Stop};
use super::models::Engine as ModelEngine;
use super::service::{Routed, Service, route_on};
use crate::state::AppState;

/// How long a start may take, the helper's hello and a model's load
/// included.
pub const START_WAIT: Duration = Duration::from_secs(20);

/// The app setting holding today's counts.
const TODAY_KEY: &str = "ai_usage_today";

/// Where a job's engine stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum AiEngineState {
    /// No AI is chosen for the job (its Beta is off, or nothing can run).
    #[default]
    Off,
    /// Chosen, not running now: not started yet, or stopped after its
    /// idle time. The next question starts it.
    Idle,
    /// Starting (the helper's hello, a model's load).
    Starting,
    /// Started and checked; ready for a question.
    Running,
    /// Writing an answer now.
    Answering,
    /// It failed: [`AiEngineStatusInfo::failure`] says why.
    Failed,
}

/// Why an engine couldn't serve, as the window says it (one plain line
/// with its fix, in the window's words module).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum AiFailure {
    /// The system's AI is turned off (System Settings turns it on).
    Off,
    /// The model isn't on this Mac yet (the system is still getting it,
    /// or the downloaded model is gone).
    NotDownloaded,
    /// AI Juggler's helper didn't start, broke, or kept failing.
    HelperDidntStart,
    /// It took too long and was stopped.
    TimedOut,
    /// The model declined to answer.
    Refused,
    /// The question and what came with it didn't fit the model's context.
    TooLong,
    /// This Mac or system can't run it.
    NotThisMac,
    /// Something else; the log has it.
    Unknown,
}

impl AiFailure {
    /// The failure an engine error is, for a model run by `engine`.
    #[must_use]
    pub const fn of(error: EngineError, engine: Option<ModelEngine>) -> Self {
        match error {
            EngineError::NotHere(Reason::TurnedOff) => Self::Off,
            EngineError::NotHere(Reason::GettingReady) => Self::NotDownloaded,
            EngineError::NotHere(Reason::HelperMissing | Reason::Unhealthy | Reason::Unknown)
            | EngineError::Crashed => Self::HelperDidntStart,
            EngineError::NotHere(
                Reason::NotThisComputer | Reason::NotThisSystem | Reason::NotBuilt,
            ) => Self::NotThisMac,
            EngineError::TimedOut | EngineError::Stopped => Self::TimedOut,
            EngineError::Refused => Self::Refused,
            EngineError::TooLong => Self::TooLong,
            // An engine of downloaded files says this of a model that
            // isn't installed.
            EngineError::Unsupported => {
                if matches!(engine, Some(engine) if engine.downloads()) {
                    Self::NotDownloaded
                } else {
                    Self::Unknown
                }
            }
        }
    }

    /// Its code in the logs.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::NotDownloaded => "not_downloaded",
            Self::HelperDidntStart => "helper_didnt_start",
            Self::TimedOut => "timed_out",
            Self::Refused => "refused",
            Self::TooLong => "too_long",
            Self::NotThisMac => "not_this_mac",
            Self::Unknown => "unknown",
        }
    }
}

/// A token count: the engine's own, or an estimate (labelled so).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AiCount {
    /// Tokens.
    pub n: u32,
    /// Estimated (3.5 characters a token, ADR-0037 §5.5), not counted by
    /// the engine.
    pub estimated: bool,
}

impl AiCount {
    /// The engine's count, else an estimate over `text`.
    #[must_use]
    pub fn counted_or_estimated(counted: Option<u32>, text: &str) -> Self {
        counted.map_or_else(
            || Self {
                n: estimate(text),
                estimated: true,
            },
            |n| Self {
                n,
                estimated: false,
            },
        )
    }

    /// The sum; estimated when either is.
    #[must_use]
    pub const fn plus(self, other: Self) -> Self {
        Self {
            n: self.n.saturating_add(other.n),
            estimated: self.estimated || other.estimated,
        }
    }
}

/// ADR-0037 §5.5's estimate: 3.5 characters a token, rounded up.
#[must_use]
pub fn estimate(text: &str) -> u32 {
    let chars = u32::try_from(text.chars().count()).unwrap_or(u32::MAX);
    chars.saturating_mul(2).div_ceil(7)
}

/// Today's use of a job's engine (AJ-E3826's Models card reads it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AiUsageInfo {
    /// Answers written today.
    pub answers: u32,
    /// Tokens read today.
    pub tokens_in: AiCount,
    /// Tokens written today.
    pub tokens_out: AiCount,
}

/// A job's engine now: the one status Bot's header and the Models page
/// read.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AiEngineStatusInfo {
    /// Where it stands.
    pub state: AiEngineState,
    /// Why it failed (or, while it can't start, why not).
    pub failure: Option<AiFailure>,
    /// The model's id in the `models` section.
    pub model_id: Option<String>,
    /// Its display name, from the section ("Apple's on-device model").
    pub model_name: Option<String>,
    /// Its short name, for Bot's header ("Apple on-device").
    pub short_name: Option<String>,
    /// Its engine: `apple_builtin` or `gguf_local`.
    pub engine: Option<String>,
    /// The model's version as the engine says (the system's version for
    /// the system's own model).
    pub version: Option<String>,
    /// The context the engine runs it with, in tokens (its own figure).
    pub context_tokens: Option<u32>,
    /// When it last started.
    #[ts(type = "number | null")]
    pub started_ms: Option<i64>,
    /// When it last answered.
    #[ts(type = "number | null")]
    pub answered_ms: Option<i64>,
    /// Today's answers and tokens.
    pub today: AiUsageInfo,
}

/// What the service keeps of a job's engine between calls.
#[derive(Debug, Default)]
pub struct RunState {
    phase: Phase,
    failure: Option<AiFailure>,
    started: Option<Started>,
    started_ms: Option<i64>,
    answered_ms: Option<i64>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    Starting,
    Answering,
    Failed,
}

impl RunState {
    /// Nothing started.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            phase: Phase::Idle,
            failure: None,
            started: None,
            started_ms: None,
            answered_ms: None,
        }
    }
}

fn now_ms(state: &AppState) -> i64 {
    state.notifier.clock().now
}

/// Logs a failure by kind: the app log and the AI log.
fn log_failure(state: &AppState, failure: AiFailure, step: &str, cause: EngineError) {
    tracing::warn!(
        kind = failure.code(),
        step,
        error = ?cause,
        "AI engine failed"
    );
    log_event(
        state,
        "engine_failed",
        serde_json::json!({ "kind": failure.code(), "stage": step }),
    );
}

fn log_event(state: &AppState, what: &'static str, payload: serde_json::Value) {
    if let Ok(storage) = state.storage.lock() {
        let mut event = NewEvent::new(now_ms(state), Stream::Chat, what, Actor::App);
        event.payload = payload;
        let _ = storage.ai_log().append(&event, None);
    }
}

fn status_of(
    service: &Service,
    state: &AppState,
    routed: Result<&Routed, EngineError>,
) -> AiEngineStatusInfo {
    let today = today(state);
    let routed = match routed {
        Ok(routed) => routed,
        // No AI chosen (Beta off, flag off, nothing to run on).
        Err(EngineError::Unsupported) => {
            return AiEngineStatusInfo {
                today,
                ..AiEngineStatusInfo::default()
            };
        }
        // Chosen, and it can't serve now: say why.
        Err(error) => {
            return AiEngineStatusInfo {
                state: AiEngineState::Failed,
                failure: Some(AiFailure::of(error, None)),
                today,
                ..AiEngineStatusInfo::default()
            };
        }
    };
    let run = service.run.lock().map(|run| {
        (
            run.phase,
            run.failure,
            run.started.clone(),
            run.started_ms,
            run.answered_ms,
        )
    });
    let (phase, failure, started, started_ms, answered_ms) =
        run.unwrap_or((Phase::Failed, Some(AiFailure::Unknown), None, None, None));
    let running = service.engines().running(&routed.card);
    let state_now = match phase {
        Phase::Starting => AiEngineState::Starting,
        Phase::Answering => AiEngineState::Answering,
        Phase::Failed => AiEngineState::Failed,
        Phase::Idle if running => AiEngineState::Running,
        Phase::Idle => AiEngineState::Idle,
    };
    AiEngineStatusInfo {
        state: state_now,
        failure: if phase == Phase::Failed {
            failure
        } else {
            None
        },
        model_id: Some(routed.card.model.clone()),
        model_name: Some(routed.name.clone()),
        engine: routed.engine.map(|engine| engine.name().to_owned()),
        short_name: Some(routed.short_name.clone()),
        version: started.as_ref().and_then(|s| s.version.clone()),
        context_tokens: Some(
            started
                .as_ref()
                .map_or(routed.card.context_tokens, |s| s.context_tokens),
        ),
        started_ms,
        answered_ms,
        today,
    }
}

/// An AI job is starting or answering right now: ADR-0037 §2.3's
/// maintenance (the world model's idle pass among it) waits for it.
#[must_use]
pub fn busy(service: &Service) -> bool {
    service
        .run
        .lock()
        .is_ok_and(|run| matches!(run.phase, Phase::Starting | Phase::Answering))
}

/// `job`'s engine now, starting nothing.
#[must_use]
pub fn status_on(service: &Service, state: &AppState, job: AiJob) -> AiEngineStatusInfo {
    service.ensure_probed(&state.data_dir);
    let routed = route_on(service, state, job.job());
    status_of(service, state, routed.as_ref().map_err(|e| *e))
}

fn set(service: &Service, change: impl FnOnce(&mut RunState)) {
    if let Ok(mut run) = service.run.lock() {
        change(&mut run);
    }
}

/// Starts `routed`'s engine when it isn't running (the helper's hello
/// checked), and keeps what it said.
fn ensure_started(service: &Service, state: &AppState, routed: &Routed) -> Result<(), AiFailure> {
    if service.engines().running(&routed.card) {
        return Ok(());
    }
    set(service, |run| {
        run.phase = Phase::Starting;
        run.failure = None;
    });
    let stop = Stop::at(Instant::now() + START_WAIT);
    match service.engines().start(&routed.card, &stop) {
        Ok(started) => {
            log_event(
                state,
                "engine_started",
                serde_json::json!({
                    "engine": routed.engine.map_or("", ModelEngine::name),
                    "context_tokens": started.context_tokens,
                }),
            );
            let at = now_ms(state);
            set(service, |run| {
                run.phase = Phase::Idle;
                run.started = Some(started);
                run.started_ms = Some(at);
            });
            Ok(())
        }
        Err(error) => {
            let failure = AiFailure::of(error, routed.engine);
            log_failure(state, failure, "start", error);
            set(service, |run| {
                run.phase = Phase::Failed;
                run.failure = Some(failure);
            });
            Err(failure)
        }
    }
}

/// Starts `job`'s engine (Bot opening with an AI chosen, its first
/// question, the Models page's check): lazily, only when it isn't
/// running. The status after.
#[must_use]
pub fn start_on(service: &Service, state: &AppState, job: AiJob) -> AiEngineStatusInfo {
    service.ensure_probed(&state.data_dir);
    let routed = route_on(service, state, job.job());
    match &routed {
        Ok(routed) => {
            let _ = ensure_started(service, state, routed);
        }
        Err(error) if *error != EngineError::Unsupported => {
            log_failure(state, AiFailure::of(*error, None), "route", *error);
        }
        Err(_) => {}
    }
    status_of(service, state, routed.as_ref().map_err(|e| *e))
}

/// A job's model as the job sees it: one generation (D-149's jobs: the
/// feed line's pick, the digest's lines, Bot's watch drafts). [`LiveJob`]
/// goes through the router's started engine; the tests use fakes.
pub trait JobModel {
    /// One generation, the engine started first when it isn't running.
    ///
    /// # Errors
    ///
    /// The [`AiFailure`] (the job then keeps its plain path).
    fn generate(&self, request: &Request, stop: &Stop) -> Result<Draft, AiFailure>;
}

/// `job` on `service`'s routed engine.
pub struct LiveJob<'a> {
    /// The AI service.
    pub service: &'a Service,
    /// The app.
    pub state: &'a AppState,
    /// The job.
    pub job: AiJob,
}

impl LiveJob<'_> {
    /// The router serves the job now (its flag, its verdict and Beta, an
    /// engine that can run it).
    #[must_use]
    pub fn served(&self) -> bool {
        route_on(self.service, self.state, self.job.job()).is_ok()
    }
}

impl JobModel for LiveJob<'_> {
    fn generate(&self, request: &Request, stop: &Stop) -> Result<Draft, AiFailure> {
        let made = generate_on(self.service, self.state, self.job, request, stop)?;
        let tokens_in = AiCount::counted_or_estimated(made.draft.usage.input, &request.prompt);
        let tokens_out = AiCount::counted_or_estimated(made.draft.usage.output, &made.draft.text);
        note_usage(self.state, tokens_in, tokens_out);
        Ok(made.draft)
    }
}

/// One generation for `job`, through its started engine.
#[derive(Debug, Clone)]
pub struct Generated {
    /// What it wrote.
    pub draft: Draft,
    /// The model's display name.
    pub model_name: String,
    /// The model's version, when the engine said.
    pub version: Option<String>,
    /// The context it runs with, in tokens.
    pub context_tokens: u32,
    /// How long the generation took, in milliseconds.
    pub ms: u64,
}

/// Asks `job`'s engine: started first when it isn't running (lazily, on
/// first use), then one generation. A failure is logged by kind and kept
/// in the status.
///
/// # Errors
///
/// The [`AiFailure`].
pub fn generate_on(
    service: &Service,
    state: &AppState,
    job: AiJob,
    request: &Request,
    stop: &Stop,
) -> Result<Generated, AiFailure> {
    service.ensure_probed(&state.data_dir);
    let routed = route_on(service, state, job.job()).map_err(|error| {
        let failure = AiFailure::of(error, None);
        log_failure(state, failure, "route", error);
        failure
    })?;
    ensure_started(service, state, &routed)?;
    set(service, |run| {
        run.phase = Phase::Answering;
        run.failure = None;
    });
    let began = Instant::now();
    let result = service.engines().respond(&routed.card, request, stop);
    let ms = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX);
    match result {
        Ok(draft) => {
            let at = now_ms(state);
            let (version, context) = service.run.lock().map_or((None, None), |run| {
                (
                    run.started.as_ref().and_then(|s| s.version.clone()),
                    run.started.as_ref().map(|s| s.context_tokens),
                )
            });
            set(service, |run| {
                run.phase = Phase::Idle;
                run.answered_ms = Some(at);
            });
            Ok(Generated {
                draft,
                model_name: routed.name,
                version,
                context_tokens: context.unwrap_or(routed.card.context_tokens),
                ms,
            })
        }
        Err(error) => {
            let failure = AiFailure::of(error, routed.engine);
            log_failure(state, failure, "answer", error);
            set(service, |run| {
                run.phase = Phase::Failed;
                run.failure = Some(failure);
            });
            Err(failure)
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Today {
    day: i64,
    #[serde(flatten)]
    usage: AiUsageInfo,
}

fn day_of(state: &AppState) -> i64 {
    let clock = state.notifier.clock();
    (clock.now + clock.utc_offset_hours.unwrap_or(0) * 3_600_000).div_euclid(86_400_000)
}

/// Today's answers and tokens (zero on a new day).
#[must_use]
pub fn today(state: &AppState) -> AiUsageInfo {
    let day = day_of(state);
    state
        .app_setting(TODAY_KEY)
        .and_then(|text| serde_json::from_str::<Today>(&text).ok())
        .filter(|kept| kept.day == day)
        .map(|kept| kept.usage)
        .unwrap_or_default()
}

/// Adds one answer's tokens to today's.
pub fn note_usage(state: &AppState, tokens_in: AiCount, tokens_out: AiCount) {
    let now = today(state);
    let kept = Today {
        day: day_of(state),
        usage: AiUsageInfo {
            answers: now.answers.saturating_add(1),
            tokens_in: now.tokens_in.plus(tokens_in),
            tokens_out: now.tokens_out.plus(tokens_out),
        },
    };
    if let (Ok(storage), Ok(text)) = (state.storage.lock(), serde_json::to_string(&kept)) {
        let _ = storage.app_settings().set(TODAY_KEY, &text);
    }
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
