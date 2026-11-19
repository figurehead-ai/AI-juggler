//! ADR-0033 §4.2 (AJ-E3802): the local engine for downloaded GGUF files,
//! a **stub** for now. Its runtime crate (`llama-cpp-2`, in its own helper
//! binary `juggler-local-ai`, never in the app) is a new dependency that
//! waits for the maintainer's yes, so it is not in this build.
//!
//! Until then it runs nothing: it gives a card for each installed model
//! it would run, saying [`Reason::NotBuilt`], so the router's choice over
//! a local model is testable and every job it alone could serve stays
//! hidden. With no model installed (the store, AJ-E3803, isn't built
//! either) it gives none.

use crate::features::ai::engine::{
    AiEngine, Card, Draft, EngineError, Readiness, Reason, Request, Stop, Tier,
};
use crate::features::ai::models::{Engine as ModelEngine, ModelEntry};

/// The local engine (a stub).
#[derive(Debug, Default)]
pub struct Local {
    /// The ids of the models installed on this Mac (none until the store
    /// exists).
    installed: Vec<String>,
}

impl Local {
    /// The stub, with the models installed on this Mac.
    #[must_use]
    pub const fn new(installed: Vec<String>) -> Self {
        Self { installed }
    }
}

impl AiEngine for Local {
    fn cards(&self, engine: usize, models: &[ModelEntry]) -> Vec<Card> {
        models
            .iter()
            .filter(|m| m.engine == ModelEngine::GgufLocal && self.installed.contains(&m.id))
            .map(|m| Card {
                engine,
                model: m.id.clone(),
                context_tokens: m
                    .sampling
                    .context_tokens
                    .and_then(|n| u32::try_from(n).ok())
                    .unwrap_or(4_096),
                tools: true,
                tier: Tier::Basic,
                uses_download: true,
                running: false,
                ready: Readiness::NotHere(Reason::NotBuilt),
            })
            .collect()
    }

    fn probe(&self) {}

    fn respond(
        &self,
        _model: &str,
        _request: &Request,
        _stop: &Stop,
    ) -> Result<Draft, EngineError> {
        Err(EngineError::NotHere(Reason::NotBuilt))
    }

    fn release(&self) {}
}
