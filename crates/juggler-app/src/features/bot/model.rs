//! AJ-E3825: Bot's model, as Bot sees it: the engine's status (its name,
//! version and window) and one generation. [`Live`] goes through the AI
//! service's started engine ([`crate::features::ai::run`]); the tests use
//! a fake that counts its calls, so a reply made without one fails them.

use std::sync::Arc;

use crate::features::ai::dto::AiJob;
use crate::features::ai::engine::{Request, Stop};
use crate::features::ai::run::{self, AiEngineStatusInfo, AiFailure, Generated};
use crate::features::ai::service::Service;
use crate::state::AppState;

/// Bot's model.
pub trait BotModel {
    /// The engine now, starting nothing.
    fn status(&self) -> AiEngineStatusInfo;

    /// One generation, the engine started first when it isn't running.
    ///
    /// # Errors
    ///
    /// The [`AiFailure`], logged by kind.
    fn generate(&self, request: &Request, stop: &Stop) -> Result<Generated, AiFailure>;
}

/// The app's model: Bot's job on `service`'s routed engine.
pub struct Live<'a> {
    /// The AI service (the app's own, or a test's over fake helpers).
    pub service: &'a Service,
    /// The app.
    pub state: &'a Arc<AppState>,
}

impl BotModel for Live<'_> {
    fn status(&self) -> AiEngineStatusInfo {
        run::status_on(self.service, self.state, AiJob::Bot)
    }

    fn generate(&self, request: &Request, stop: &Stop) -> Result<Generated, AiFailure> {
        run::generate_on(self.service, self.state, AiJob::Bot, request, stop)
    }
}

/// AJ-E3829's seam: one question through Bot's real routing on
/// `service`'s engine (the app's, or a check's own over the real helper),
/// kept like any turn; the turn, its outcome and the model that wrote it.
#[derive(Debug, Clone)]
pub struct CheckedTurn {
    /// The turn as the window gets it.
    pub turn: super::dto::BotTurnInfo,
    /// Its outcome's code (`answered`, `general`, `failed`, …).
    pub outcome: &'static str,
    /// The model that wrote it, from the engine's own name.
    pub model_name: Option<String>,
}

/// See [`CheckedTurn`].
///
/// # Errors
///
/// As [`super::commands::bot_ask_with`].
pub fn ask_for_check(
    state: &Arc<AppState>,
    service: &Service,
    question: &str,
) -> crate::error::AppResult<CheckedTurn> {
    let live = Live { service, state };
    let turn = super::commands::bot_ask_with(
        state,
        &super::dto::BotAskInput {
            question: question.to_owned(),
            scope: None,
            again: None,
        },
        &live,
    )?;
    let model_name = turn
        .written
        .as_ref()
        .map(|w| w.model_name.clone())
        .or_else(|| turn.general.as_ref().map(|g| g.model_name.clone()))
        .or_else(|| turn.details.as_ref().map(|d| d.model_name.clone()));
    Ok(CheckedTurn {
        outcome: turn.outcome.code(),
        model_name,
        turn,
    })
}
