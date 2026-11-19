//! ADR-0033 §4.3 (AJ-E3802): the user's own model server (Ollama or LM
//! Studio, off by default and last to build), a **stub**: it sees no
//! server and runs nothing, so it gives no card. When it is built, a
//! running server with a model loaded becomes one card (no download,
//! `Unknown` tier), found through the process table, never a port scan.

use crate::features::ai::engine::{AiEngine, Card, Draft, EngineError, Reason, Request, Stop};
use crate::features::ai::models::ModelEntry;

/// The user's own server (a stub).
#[derive(Debug, Default)]
pub struct Server;

impl AiEngine for Server {
    fn cards(&self, _engine: usize, _models: &[ModelEntry]) -> Vec<Card> {
        Vec::new()
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
