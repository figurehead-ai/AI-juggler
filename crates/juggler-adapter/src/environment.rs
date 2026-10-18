//! Environment scanning and config import for smart onboarding (spec §12,
//! ADR-0006).
//!
//! The onboarding wizard assumes the user knows nothing about how harness
//! config directories work: adapters scan the machine for existing
//! installations so the wizard can offer "import what you already have"
//! instead of a blank form.
//!
//! Guardrails (ADR-0003/0006): scanning is **metadata-only** — file
//! presence, sizes, and timestamps, never contents; importing is
//! **read-only toward the source** — a plain byte copy into a fresh managed
//! dir, with the source left untouched and unmarked.

use std::path::PathBuf;

use crate::model::HarnessId;

/// Login evidence for a discovered install, probed without reading
/// credential contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoginProbe {
    /// Credentials are present for this config dir.
    LoggedIn,
    /// No credentials found for this config dir.
    #[default]
    LoggedOut,
}

/// How a discovered install's location came to exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOrigin {
    /// The harness's own default location (e.g. `~/.claude`).
    VendorDefault,
    /// Pointed at by the harness's env override (e.g. `$CLAUDE_CONFIG_DIR`)
    /// — typically another manager tool or a manual setup.
    EnvOverride,
    /// Matched a config-naming pattern in the user's home — created
    /// manually, by another tool, or by an AI assistant.
    HomePattern,
    /// Found under another manager tool's profile root (e.g. t3.codes'
    /// `ClaudeCodeProfiles/`) — managed elsewhere, imported read-only.
    ManagerTool,
    /// Pointed at by a shell rc assignment (`CLAUDE_CONFIG_DIR=` in
    /// `~/.zshrc`) — a forgotten manual setup (AJ-E302 S5).
    ShellAlias,
}

impl InstallOrigin {
    /// Wire name for the UI (`vendor-default` / `env-override` /
    /// `home-pattern` / `manager-tool`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VendorDefault => "vendor-default",
            Self::EnvOverride => "env-override",
            Self::HomePattern => "home-pattern",
            Self::ManagerTool => "manager-tool",
            Self::ShellAlias => "shell-alias",
        }
    }
}

/// One existing harness installation found on this machine.
#[derive(Debug, Clone)]
pub struct DiscoveredInstall {
    /// Harness this install belongs to.
    pub harness: HarnessId,
    /// Config dir root (e.g. `~/.claude`).
    pub path: PathBuf,
    /// How the location came to exist.
    pub origin: InstallOrigin,
    /// Login evidence for this exact path (metadata-only probe).
    pub login: LoginProbe,
    /// Approximate bytes on disk (bounded walk; caches excluded).
    pub bytes: u64,
    /// Most recent mtime among top-level entries, unix seconds.
    pub last_used_secs: Option<u64>,
    /// True when the dir already carries the AI Juggler management marker
    /// (a previous import or managed setup).
    pub managed: bool,
    /// Human-facing hints the wizard shows verbatim.
    pub notes: Vec<String>,
}

/// Result of importing an existing config dir into a managed account dir.
#[derive(Debug, Clone)]
pub struct ImportOutcome {
    /// Files copied into the managed dir.
    pub files_copied: u64,
    /// Bytes copied into the managed dir.
    pub bytes_copied: u64,
    /// Relative paths skipped as rebuildable cache/noise, with reasons.
    pub skipped: Vec<String>,
    /// Login state of the **destination** after the copy.
    pub login: LoginProbe,
    /// When login did not carry over, the plain-language reason and what
    /// the user should do (e.g. macOS keychain path-locking).
    pub login_note: Option<String>,
    /// AJ-E307 Req.3: per-top-level-entry bytes, largest first — the
    /// Confirm-source breakdown (and the Summary's copy receipt).
    pub folders: Vec<(String, u64)>,
    /// True when this was the settings-only degraded import (Req.4).
    pub degraded: bool,
}

/// AJ-E307 Req.3: the pre-copy source inspection (metadata-only —
/// nothing read beyond metadata, nothing written).
#[derive(Debug, Clone, Default)]
pub struct SourceInspect {
    /// Total importable bytes (after the rebuildable-cache skips).
    pub total_bytes: u64,
    /// Importable file count (bounded walk; `truncated` flags the cap).
    pub file_count: u64,
    /// Per top-level entry (folder or file) importable bytes,
    /// largest first — the Confirm-source breakdown.
    pub folders: Vec<(String, u64)>,
    /// True when the walk stopped at the bounded-file cap.
    pub truncated: bool,
    /// The adapter's copy ceiling in bytes (the UI offers the
    /// degraded path from this, never a raw ceiling string).
    pub ceiling_bytes: u64,
    /// Config evidence — false renders the plain-language rejection.
    pub is_config: bool,
    /// Already managed by AI Juggler.
    pub already_managed: bool,
    /// The source's login state.
    pub login: LoginProbe,
}

/// The vendor's own web sign-in page for this harness, when one exists.
/// Opening it is the maximum login aid allowed: the user completes the
/// vendor's flow; AI Juggler never touches credentials.
pub type LoginUrl = &'static str;

// ---------------------------------------------------------------------------
// AJ-E302: the widened scan — per-probe health, shell-alias hints,
// manager hints (S1–S10, pass2/03 §3). Every probe is
// metadata-only: existence, counts, names — never credential
// contents, never keychain values (existence only).

/// One probe's health (AJ-E302 Req.4): the scan NEVER fails whole —
/// a denied or partial probe reports itself and the rest continue.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ScanHealth {
    /// The probe ran to completion.
    #[default]
    Ok,
    /// The probe ran but some sources were unreadable/skipped — the
    /// reason is surfaced verbatim.
    Partial(String),
    /// The probe could not run (permissions, locked DB) — setups it
    /// would have found will not appear.
    Denied(String),
}

impl ScanHealth {
    /// Wire name (`ok` / `partial` / `denied`).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Partial(_) => "partial",
            Self::Denied(_) => "denied",
        }
    }

    /// The human reason, when not `Ok`.
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Ok => None,
            Self::Partial(reason) | Self::Denied(reason) => Some(reason),
        }
    }
}

/// A path-valued env assignment found in a shell rc file (S5):
/// `CLAUDE_CONFIG_DIR=~/work-claude` at `~/.zshrc:42`. Parsing is
/// line-scoped to the two known variable names — never general shell
/// evaluation, never other variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasHint {
    /// The variable name (`CLAUDE_CONFIG_DIR` / `CODEX_HOME`).
    pub variable: String,
    /// The assigned path (value only — a path, never other data).
    pub path: PathBuf,
    /// Source file (display path).
    pub source_file: String,
    /// 1-based line number.
    pub line: u32,
    /// False when the assigned dir doesn't exist (the dangling case
    /// — shown as a hint, never an import candidate).
    pub target_exists: bool,
}

/// Another manager tool's presence (S3/S4): metadata only —
/// existence, row counts, provider NAMES. Never tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagerHint {
    /// Manager id (`t3` / `cc-switch`).
    pub manager: String,
    /// Present on this machine.
    pub present: bool,
    /// Metadata-only detail (e.g. "3 providers: work, personal").
    pub detail: String,
    /// The probe's health.
    pub health: ScanHealth,
}
