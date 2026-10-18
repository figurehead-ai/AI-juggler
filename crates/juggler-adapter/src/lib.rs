//! The `HarnessAdapter` contract — the only place harness specifics are allowed
//! to live (ADR-0002).
//!
//! This crate defines the [`HarnessAdapter`] trait, capability flags,
//! discovery reports, launch specs, and session-watching interfaces that every
//! harness integration (Claude Code, Codex, `ZCode`, future community
//! adapters) implements. It is semver-protected public API: external adapters
//! build against it, so breaking changes are guarded by `cargo-semver-checks`
//! in CI. Requirements: `docs/SPEC.md` §4.

#![cfg_attr(
    test,
    allow(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)
)]

pub mod adapter;
pub mod capabilities;
pub mod discovery;
pub mod environment;
pub mod error;
pub mod handover;
pub mod launch;
pub mod registry;
pub mod watcher;

/// Re-exported so adapter authors need only depend on this crate for types
/// (they still come from `juggler-core`, the single source of truth).
pub use juggler_core::model;

pub use adapter::{HarnessAdapter, SharedAdapter};
pub use capabilities::Capabilities;
pub use discovery::{DiscoveryReport, InstallStatus, VerificationStatus};
pub use environment::{DiscoveredInstall, ImportOutcome, InstallOrigin, LoginProbe, LoginUrl};
pub use error::AdapterError;
pub use launch::{DesktopLaunch, EnvMap, LaunchMode, LaunchSpec};
pub use registry::AdapterRegistry;
pub use watcher::{SessionEvent, SessionWatcher};
