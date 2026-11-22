//! ADR-0034 §2 (AJ-E3802): the seam between AI jobs and the engines that
//! run them. **Jobs declare three needs; engines declare three
//! capabilities against the same three,** plus the facts the router
//! orders by. Neither side knows the other: a new engine is one
//! [`AiEngine`] in `engines/`, a new job is one [`super::jobs::JobSpec`],
//! and adding either changes nothing in the other.
//!
//! The router ([`super::router`]) sees only [`Card`]s: no engine, tool or
//! model name lives outside the engines' own modules and the `models`
//! section's typed records (the architecture lint,
//! `scripts/tool_literal_lint.py`).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use super::models::ModelEntry;

/// The least quality a job accepts, and what an engine's model gives.
/// Ordered: a job accepts its tier or better.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// A model we can't measure (the user's own server).
    Unknown,
    /// A 3B to 4B on-device model.
    Basic,
    /// A larger local model, if one is ever approved.
    Capable,
    /// A cloud engine (not built).
    Strong,
}

/// What a job needs (ADR-0034 §2: three needs, no more).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Needs {
    /// The least context, in tokens, the job can work in.
    pub context_tokens: u32,
    /// It drives look-ups (Chat's step loop, ADR-0034 §6.3).
    pub tools: bool,
    /// The least quality it accepts.
    pub tier: Tier,
}

/// Why an engine can't serve now: a key into the words module, never
/// text. Each is said in plain words by the surface that shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reason {
    /// This operating system has no such engine (yet).
    NotThisSystem,
    /// This computer can't run it (its chip, its memory).
    NotThisComputer,
    /// The system's AI is turned off; the user can turn it on.
    TurnedOff,
    /// The system is still getting its model ready.
    GettingReady,
    /// AI Juggler's own helper for it is missing from this build.
    HelperMissing,
    /// It isn't built in this version of AI Juggler.
    NotBuilt,
    /// It failed three times within an hour (ADR-0033 §3's health rule).
    Unhealthy,
    /// Its state couldn't be read; unknown fails closed.
    Unknown,
}

/// Whether an engine can take a job now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    /// It can.
    Ready,
    /// It will be able to soon, by itself (a model being prepared).
    Preparing(Reason),
    /// It can't, here and now.
    NotHere(Reason),
}

/// One model an engine can run, as the router sees it: the three
/// capabilities, and the facts it orders by (never matches on).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// The engine's place in the registry: opaque to the router, the
    /// registry's own order as the last tie-break.
    pub engine: usize,
    /// The `models` section's id of the model it runs (its verdicts).
    pub model: String,
    /// The context it runs that model with, in tokens.
    pub context_tokens: u32,
    /// It can drive look-ups (guided steps, ADR-0034 §6.3).
    pub tools: bool,
    /// The model's quality.
    pub tier: Tier,
    /// It runs a file AI Juggler downloaded (D-121: zero disk first).
    pub uses_download: bool,
    /// Its helper is alive or its model loaded already.
    pub running: bool,
    /// Whether it can take a job now.
    pub ready: Readiness,
}

/// The shape guided output must take: a schema in the helpers' small
/// form (`{"type":"object","name":…,"properties":[{"name","description",
/// "schema"}]}` with `string`, `enum`, `bool`, `integer` and `array`), the
/// harness's form (AJ-E3709 v4). Every engine constrains its output to it
/// (ADR-0034 §2: guided output is a floor, not a need).
pub type Shape = serde_json::Value;

/// One generation a job asks of an engine.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// The job's instructions (the frame, ADR-0033 §1).
    pub instructions: String,
    /// The framed prompt.
    pub prompt: String,
    /// The output's shape; `None` for plain text.
    pub shape: Option<Shape>,
    /// The most output tokens.
    pub max_tokens: u32,
}

/// How a generation ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// The model finished.
    Stop,
    /// It hit the token limit (the job's gate rejects a cut answer).
    Length,
}

/// What an engine wrote: data for the job's own check, never shown as is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// The text, or the guided output's JSON.
    pub text: String,
    /// How it ended.
    pub finish: Finish,
}

/// Why a generation gave nothing. Closed, and never carries text: the job
/// falls back to its plain path (ADR-0033 §3: one model attempt, never a
/// chain of engines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineError {
    /// The engine can't serve now ([`Readiness::NotHere`]).
    NotHere(Reason),
    /// The model refused, or its guardrails stopped it.
    Refused,
    /// The prompt didn't fit the context.
    TooLong,
    /// The deadline passed; the helper was stopped.
    TimedOut,
    /// The job asked it to stop.
    Stopped,
    /// The helper died or broke the protocol.
    Crashed,
    /// The engine doesn't run this model.
    Unsupported,
}

/// A job's stop: a shared flag and a deadline (ADR-0034 §4.2).
#[derive(Debug, Clone)]
pub struct Stop {
    flag: Arc<AtomicBool>,
    deadline: Instant,
}

impl Stop {
    /// A stop that fires at `deadline`, or when [`Stop::stop`] is called.
    #[must_use]
    pub fn at(deadline: Instant) -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            deadline,
        }
    }

    /// The same stop with another deadline: a step with its own time
    /// limit (loading a model), still stopped with the job.
    #[must_use]
    pub fn until(&self, deadline: Instant) -> Self {
        Self {
            flag: Arc::clone(&self.flag),
            deadline,
        }
    }

    /// Asks the job to stop.
    pub fn stop(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    /// Whether it was asked to stop.
    #[must_use]
    pub fn stopped(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    /// When it must end.
    #[must_use]
    pub const fn deadline(&self) -> Instant {
        self.deadline
    }
}

/// An engine: what it can serve now ([`AiEngine::cards`], pure), asking
/// the system again ([`AiEngine::probe`], lazily), and one generation
/// ([`AiEngine::respond`]). Nothing is started or loaded until a job asks.
pub trait AiEngine: Send + Sync {
    /// One card per model in `models` this engine can run, from its last
    /// probe: pure, no I/O, microseconds. An engine that runs none of
    /// them gives none.
    fn cards(&self, engine: usize, models: &[ModelEntry]) -> Vec<Card>;

    /// Asks the system again whether this engine can run: may start a
    /// short-lived helper, never loads a model. Off the window's thread.
    fn probe(&self);

    /// One generation with `model` (a card's), lazily starting what it
    /// needs.
    ///
    /// # Errors
    ///
    /// [`EngineError`]: the job then shows its plain path.
    fn respond(&self, model: &str, request: &Request, stop: &Stop) -> Result<Draft, EngineError>;

    /// Lets go of what it holds (a helper, a loaded model).
    fn release(&self);
}
