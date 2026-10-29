//! AJ-E2306 (D-40, ADR-0022 decision 4): a terminal session's exact state
//! from the tool's own extension points (Claude Code's hooks and status
//! line), added per launch through a file in AI Juggler's own folder, never
//! the user's setup.
//!
//! The app says what to add ([`ReportsLaunch`]: the report program, this
//! launch's folder, and AI Juggler's overlays folder); each adapter says
//! how its tool takes it ([`crate::HarnessAdapter::state_reports`]): the
//! launch's whole new argument list (it may replace an AI Juggler overlay
//! file the launch already names, since a tool may take only one) and the
//! files to write. An adapter that returns one declares
//! [`crate::Capabilities::STATE_REPORTS`]; the conformance kit holds both.

use std::path::PathBuf;

use crate::agent_bridge::BridgeFile;

/// What AI Juggler adds to one terminal launch for its state reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportsLaunch {
    /// The program the tool's hooks run (`juggler-agent`, whose `hook
    /// <event>` reports), by its full path.
    pub program: PathBuf,
    /// This launch's folder for files, in AI Juggler's overlays folder.
    pub dir: PathBuf,
    /// AI Juggler's overlays folder: a settings file the launch names from
    /// inside it is AI Juggler's own, and may be folded into the new one.
    pub overlays: PathBuf,
}

/// How one tool takes the reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportsInjection {
    /// The launch's whole argument list, with the reports in.
    pub args: Vec<String>,
    /// Files for [`ReportsLaunch::dir`].
    pub files: Vec<BridgeFile>,
}
