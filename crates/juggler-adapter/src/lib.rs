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
pub mod agent_bridge;
pub mod capabilities;
pub mod conversation;
pub mod discovery;
pub mod endpoint;
pub mod environment;
pub mod error;
pub mod file_trust;
pub mod folder_trust;
pub mod handover;
pub mod identity;
pub mod inventory;
pub mod jsonc;
pub mod launch;
pub mod live;
pub mod registry;
pub mod reports;
pub mod runtime;
pub mod spec;
pub mod system_tools;
pub mod user_path;
pub mod watcher;

/// Re-exported so adapter authors need only depend on this crate for types
/// (they still come from `juggler-core`, the single source of truth).
pub use juggler_core::model;

pub use adapter::{HarnessAdapter, LimitsCapture, SharedAdapter};
pub use agent_bridge::{BridgeFile, BridgeInjection, BridgeLaunch};
pub use capabilities::Capabilities;
pub use discovery::{DiscoveryReport, InstallStatus, VerificationStatus};
pub use environment::{
    DiscoveredInstall, ImportOutcome, InstallOrigin, LoginProbe, LoginState, LoginUrl,
};
pub use error::AdapterError;
pub use identity::IdentityProbe;
pub use launch::{DesktopLaunch, EnvMap, LaunchMode, LaunchSpec};
pub use registry::AdapterRegistry;
pub use runtime::{
    DrivenLaunch, DrivenRuntime, DrivenSession, DrivenSpec, Maturity, Release, ReleaseOutcome,
    Runtime, RuntimeEvent, TerminalView, Transport,
};
pub use spec::{ReleaseFeed, SetupGroup, SignInWords, ToolFlags, ToolKind, ToolSpec, ToolTerms};
pub use watcher::{SessionEvent, SessionWatcher};
