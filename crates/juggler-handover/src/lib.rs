//! Handover & session-sync engine (spec §7).
//!
//! Moves real session continuity — not hand-off prompts — between accounts and
//! harnesses: native resume (T1), cross-account session transplant (T2),
//! cross-harness transcript replay (T3), and the versioned context-pack format
//! (T4), plus lineage tracking and project-context sync. Per the automation
//! policy (ADR-0003), everything here *prepares*; only an explicit user action
//! executes.

#![cfg_attr(
    test,
    allow(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)
)]

pub mod door;
pub mod fidelity;
pub mod manifest;
pub mod pack;
pub mod pipeline;
pub mod ranking;
pub mod reconcile;
pub mod redaction;
pub mod switch_back;
pub mod transplant;
pub mod validation;
