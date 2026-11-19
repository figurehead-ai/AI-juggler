//! AI features on this Mac (ADR-0033, ADR-0034, ADR-0037). Each job and
//! store is its own module; this one only groups them.
//!
//! The router (AJ-E3802): [`engine`] is the seam (jobs' needs, engines'
//! cards), [`jobs`] the routed jobs and their flags, [`router`] the pure
//! choice, [`engines`] the only place that names an engine, and
//! [`service`] the one query the window reads ([`commands::ai_jobs`]).

pub mod commands;
pub mod dto;
pub mod engine;
pub mod engines;
pub mod jobs;
pub mod memory;
pub mod models;
pub mod router;
pub mod service;

#[cfg(test)]
mod router_tests;
