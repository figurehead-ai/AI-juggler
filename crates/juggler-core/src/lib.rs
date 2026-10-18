//! Domain model, storage, and usage accounting for AI Juggler.
//!
//! This crate is the pure heart of the system: entities ([`Account`],
//! [`Profile`], [`SessionRecord`], usage snapshots and windows), the `SQLite`
//! persistence layer, and crash-safe filesystem helpers. It knows nothing
//! about specific harnesses, the app shell, or the UI, and must stay fully
//! unit-testable (architecture rules in `docs/plan/QUALITY.md` §4;
//! requirements in `docs/SPEC.md` §3, §6).

#![cfg_attr(
    test,
    allow(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)
)]

pub mod alerts;
pub mod cost_ledger;
pub mod error;
pub mod fs;
pub mod funnel_events;
pub mod model;
pub mod quality;
pub mod quota;
pub mod quota_forecast;
pub mod ranking;
pub mod recommendation;
pub mod renewal;
pub mod storage;

pub use error::CoreError;
pub use model::{
    Account, AccountId, Confidence, HarnessId, MachineId, PlanTier, Profile, ProfileId,
    QuotaWindow, SessionId, SessionRecord, SessionStatus, SettingsOverlay, Timestamp, TokenCounts,
    TranscriptRef, UsageSnapshot,
};
