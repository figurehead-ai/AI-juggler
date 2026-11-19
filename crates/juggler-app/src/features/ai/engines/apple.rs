//! ADR-0033 §4.1 (AJ-E3802): Apple's on-device model, through AI
//! Juggler's Swift helper `juggler-apple-ai`
//! (`crates/juggler-app/helpers/juggler-apple-ai.swift`, built by
//! `scripts/build_apple_ai_helper.sh`).
//!
//! **It works only when Apple Intelligence is available and on.** The OS
//! seam answers first ([`juggler_platform::ai::system_model`]): on an
//! Intel Mac, a macOS before 26, Windows or Linux the helper is never
//! started. Otherwise the helper's `hello` carries the model's own
//! availability (`SystemLanguageModel.default.availability`) and context
//! size; a probe starts it, reads that line and lets it go. No model is
//! loaded until a job asks ([`AiEngine::respond`]); the helper then stays
//! for the next request and exits by itself after 60 s idle.
//!
//! It serves the `models` section's `apple_builtin` entry and nothing
//! else: no download, `Basic` tier, look-ups through guided steps.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use juggler_platform::ai::SystemModel;

use super::helper::Helper;
use crate::features::ai::engine::{
    AiEngine, Card, Draft, EngineError, Finish, Readiness, Reason, Request, Stop, Tier,
};
use crate::features::ai::models::recommend::AppleAi;
use crate::features::ai::models::{Engine as ModelEngine, ModelEntry};

/// The helper's file name, beside AI Juggler's own executable.
pub const PROGRAM: &str = "juggler-apple-ai";

/// The identifier the helper is signed as (its embedded Info.plist,
/// `helpers/juggler-apple-ai.Info.plist`); the app runs no helper signed
/// as anything else.
pub const IDENTIFIER: &str = "ai.figurehead.juggler.apple-ai";

/// The protocol version this build speaks.
pub const PROTOCOL: u64 = 1;

/// How long a starting helper may take to say hello.
const HELLO_WAIT: Duration = Duration::from_secs(5);

/// ADR-0033 §3's health rule: this many failures within [`HEALTH_WINDOW`]
/// mark the engine unhealthy until the next probe that finds it fine.
const HEALTH_FAILURES: usize = 3;
const HEALTH_WINDOW: Duration = Duration::from_secs(3_600);

/// The context the model is assumed to have when the helper doesn't say
/// (macOS 26's figure).
const DEFAULT_CONTEXT: u32 = 4_096;

/// The model's own answer, from the helper's hello.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// On and ready.
    Available,
    /// This Mac can't run it.
    NotEligible,
    /// Apple Intelligence is turned off.
    NotEnabled,
    /// The system is still getting the model.
    NotReady,
    /// The helper ran on a macOS without the model.
    NotThisOs,
    /// A word this build doesn't know: unknown fails closed.
    Unknown,
}

impl Availability {
    fn from_word(word: &str) -> Self {
        match word {
            "available" => Self::Available,
            "device_not_eligible" => Self::NotEligible,
            "apple_intelligence_not_enabled" => Self::NotEnabled,
            "model_not_ready" => Self::NotReady,
            "not_this_os" => Self::NotThisOs,
            _ => Self::Unknown,
        }
    }
}

/// What the engine last saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seen {
    /// Not asked yet.
    NotAsked,
    /// The OS seam said no; the helper wasn't started.
    Os(SystemModel),
    /// The helper isn't in the app (or isn't safe to run: its owner or
    /// its code signature).
    Missing,
    /// The helper said this.
    Model {
        /// The model's availability.
        availability: Availability,
        /// Its context, in tokens.
        context_tokens: u32,
    },
    /// The helper didn't answer properly.
    Failed,
}

impl Seen {
    /// The card's readiness for what was seen.
    #[must_use]
    pub const fn readiness(self) -> Readiness {
        match self {
            Self::Model {
                availability: Availability::Available,
                ..
            } => Readiness::Ready,
            Self::Model {
                availability: Availability::NotReady,
                ..
            } => Readiness::Preparing(Reason::GettingReady),
            Self::Model {
                availability: Availability::NotEnabled,
                ..
            } => Readiness::NotHere(Reason::TurnedOff),
            Self::Model {
                availability: Availability::NotEligible,
                ..
            }
            | Self::Os(SystemModel::NotThisChip | SystemModel::NotThisVersion) => {
                Readiness::NotHere(Reason::NotThisComputer)
            }
            Self::Model {
                availability: Availability::NotThisOs,
                ..
            }
            | Self::Os(SystemModel::NotThisSystem) => Readiness::NotHere(Reason::NotThisSystem),
            Self::Missing => Readiness::NotHere(Reason::HelperMissing),
            Self::Os(SystemModel::MayHave)
            | Self::NotAsked
            | Self::Failed
            | Self::Model {
                availability: Availability::Unknown,
                ..
            } => Readiness::NotHere(Reason::Unknown),
        }
    }

    /// Apple Intelligence for the recommendation (AJ-E3811).
    #[must_use]
    pub const fn apple_ai(self) -> AppleAi {
        match self {
            Self::Model {
                availability: Availability::Available,
                ..
            } => AppleAi::On,
            Self::Model {
                availability: Availability::NotEnabled | Availability::NotReady,
                ..
            } => AppleAi::Off,
            _ => AppleAi::Unavailable,
        }
    }

    const fn context_tokens(self) -> u32 {
        match self {
            Self::Model { context_tokens, .. } if context_tokens > 0 => context_tokens,
            _ => DEFAULT_CONTEXT,
        }
    }
}

/// The Apple engine.
#[derive(Debug)]
pub struct Apple {
    /// The helper, when a test names it; otherwise beside the app.
    program: Option<PathBuf>,
    seen: Mutex<Seen>,
    helper: Mutex<Option<Helper>>,
    failures: Mutex<Vec<Instant>>,
    next_id: Mutex<u64>,
}

impl Apple {
    /// The engine, its helper beside AI Juggler's executable.
    #[must_use]
    pub fn new() -> Self {
        Self::with_program(None)
    }

    /// The engine with its helper at `program` (tests).
    #[must_use]
    pub fn with_program(program: Option<PathBuf>) -> Self {
        Self {
            program,
            seen: Mutex::new(Seen::NotAsked),
            helper: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            next_id: Mutex::new(0),
        }
    }

    /// What it last saw.
    #[must_use]
    pub fn seen(&self) -> Seen {
        self.seen.lock().map_or(Seen::Failed, |seen| *seen)
    }

    fn set_seen(&self, seen: Seen) {
        if let Ok(mut slot) = self.seen.lock() {
            *slot = seen;
        }
    }

    /// The helper to run: a test's, or the app's own through the one
    /// resolver ([`juggler_platform::code_trust::own_helper`]): in the
    /// app's bundle (or beside it in a development build), ours or
    /// root's and writable by no one else, and signed as [`IDENTIFIER`]
    /// by the app's own team. Anything else is missing (fails closed).
    fn program(&self) -> Option<PathBuf> {
        if let Some(program) = &self.program {
            return program.is_file().then(|| program.clone());
        }
        juggler_platform::code_trust::own_helper(PROGRAM, IDENTIFIER)
    }

    /// Starts the helper and reads what its hello says.
    fn start(&self, program: &std::path::Path) -> Result<Helper, EngineError> {
        let helper = Helper::start(program, HELLO_WAIT)?;
        let hello = &helper.hello;
        if hello["protocol"].as_u64() != Some(PROTOCOL) {
            self.set_seen(Seen::Failed);
            return Err(EngineError::Crashed);
        }
        let availability = Availability::from_word(hello["available"].as_str().unwrap_or(""));
        let context_tokens = hello["context_size"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0);
        self.set_seen(Seen::Model {
            availability,
            context_tokens,
        });
        Ok(helper)
    }

    fn unhealthy(&self) -> bool {
        self.failures.lock().is_ok_and(|failures| {
            failures
                .iter()
                .filter(|at| at.elapsed() < HEALTH_WINDOW)
                .count()
                >= HEALTH_FAILURES
        })
    }

    fn failed(&self) {
        if let Ok(mut failures) = self.failures.lock() {
            failures.retain(|at| at.elapsed() < HEALTH_WINDOW);
            failures.push(Instant::now());
        }
    }

    fn running(&self) -> bool {
        self.helper
            .lock()
            .is_ok_and(|mut helper| helper.as_mut().is_some_and(Helper::alive))
    }
}

impl Default for Apple {
    fn default() -> Self {
        Self::new()
    }
}

impl AiEngine for Apple {
    fn cards(&self, engine: usize, models: &[ModelEntry]) -> Vec<Card> {
        let Some(entry) = models
            .iter()
            .find(|m| m.engine == ModelEngine::AppleBuiltin)
        else {
            return Vec::new();
        };
        let seen = self.seen();
        let ready = match seen.readiness() {
            Readiness::Ready if self.unhealthy() => Readiness::NotHere(Reason::Unhealthy),
            ready => ready,
        };
        vec![Card {
            engine,
            model: entry.id.clone(),
            context_tokens: seen.context_tokens(),
            tools: true,
            tier: Tier::Basic,
            uses_download: false,
            running: self.running(),
            ready,
        }]
    }

    fn probe(&self) {
        let os = juggler_platform::ai::system_model();
        if os != SystemModel::MayHave && self.program.is_none() {
            self.set_seen(Seen::Os(os));
            return;
        }
        let Some(program) = self.program() else {
            self.set_seen(Seen::Missing);
            return;
        };
        match self.start(&program) {
            Ok(helper) => {
                if self.seen().readiness() == Readiness::Ready
                    && let Ok(mut failures) = self.failures.lock()
                {
                    failures.clear();
                }
                drop(helper);
            }
            Err(_) => self.set_seen(Seen::Failed),
        }
    }

    fn respond(&self, model: &str, request: &Request, stop: &Stop) -> Result<Draft, EngineError> {
        // It runs one model, the one its card named; the router sends no other.
        if model.is_empty() {
            return Err(EngineError::Unsupported);
        }
        if self.unhealthy() {
            return Err(EngineError::NotHere(Reason::Unhealthy));
        }
        if let Readiness::NotHere(why) | Readiness::Preparing(why) = self.seen().readiness() {
            return Err(EngineError::NotHere(why));
        }
        let program = self
            .program()
            .ok_or(EngineError::NotHere(Reason::HelperMissing))?;
        let mut slot = self.helper.lock().map_err(|_| EngineError::Crashed)?;
        if !slot.as_mut().is_some_and(Helper::alive) {
            *slot = Some(self.start(&program).inspect_err(|_| self.failed())?);
        }
        if let Readiness::NotHere(why) | Readiness::Preparing(why) = self.seen().readiness() {
            *slot = None;
            return Err(EngineError::NotHere(why));
        }
        let id = self.next_id.lock().map_or(0, |mut next| {
            *next += 1;
            *next
        });
        let mut line = serde_json::json!({
            "id": id,
            "op": "respond",
            "instructions": request.instructions,
            "prompt": request.prompt,
            "max_tokens": request.max_tokens,
        });
        if let Some(shape) = &request.shape {
            line["schema"] = shape.clone();
        }
        let Some(helper) = slot.as_mut() else {
            return Err(EngineError::Crashed);
        };
        let answer = match helper.ask(&line, stop) {
            Ok(answer) if answer["id"].as_u64() == Some(id) => answer,
            Ok(_) | Err(EngineError::Crashed) => {
                *slot = None;
                self.failed();
                return Err(EngineError::Crashed);
            }
            Err(error) => {
                // Stopped, timed out or too long: never wait on it again.
                *slot = None;
                return Err(error);
            }
        };
        draft(&answer).inspect_err(|error| {
            if *error == EngineError::NotHere(Reason::Unknown) {
                *slot = None;
                self.set_seen(Seen::Failed);
            }
        })
    }

    fn release(&self) {
        if let Ok(mut slot) = self.helper.lock() {
            *slot = None;
        }
    }
}

/// The helper's answer as a draft, or its error.
fn draft(answer: &serde_json::Value) -> Result<Draft, EngineError> {
    match answer["error"].as_str() {
        None => Ok(Draft {
            text: answer["text"].as_str().unwrap_or_default().to_owned(),
            finish: if answer["finish"].as_str() == Some("length") {
                Finish::Length
            } else {
                Finish::Stop
            },
        }),
        Some("refused") => Err(EngineError::Refused),
        Some("too_long") => Err(EngineError::TooLong),
        Some("unavailable") => Err(EngineError::NotHere(Reason::Unknown)),
        Some(_) => Err(EngineError::Crashed),
    }
}

#[cfg(test)]
#[path = "apple_tests.rs"]
mod tests;
