//! Codex environment scan + home import (AJ-E1302, spec §12, ADR-0006).
//!
//! Finds the Codex homes already on this machine and copies one into a
//! fresh managed home. Same rules as Claude's (`claude_environment`):
//! metadata-only probing, zero writes outside managed dirs, the source
//! never modified or marked.
//!
//! What is NOT copied, and why (codex-cli 0.163.0, research/codex.md
//! §10):
//! - `auth.json`, the sign-in. A `ChatGPT` sign-in renews its refresh
//!   token on every refresh and the old one is then refused ("your
//!   refresh token was already used. Please log out and sign in again",
//!   codex-rs `login/src/auth/manager.rs:207`, `:1688`; the new token
//!   replaces the stored one, `:1604-1618`). Two homes holding one token
//!   would sign each other out — so the copy signs in on its own.
//! - The SQLite files record ABSOLUTE rollout paths — a copied
//!   `state_5.sqlite` would point the new account at the original home's
//!   session files, and resuming would write into the original. Codex
//!   rebuilds that index from `sessions/` on its own (a filename scan that
//!   read-repairs the database), so every `*.sqlite*` stays behind, as do
//!   locks, logs, the per-install id, shell snapshots and the background
//!   server's state and package copy.
//!
//! Skips apply to the home's own top-level entries only: a `log/` or
//! `packages/` inside, say, a skill is the user's and is copied.

use std::path::{Path, PathBuf};

use juggler_adapter::environment::{
    DiscoveredInstall, ImportOutcome, InstallOrigin, LoginProbe, SourceInspect,
};

use crate::claude_environment::{
    Layout, MAX_IMPORT_BYTES, adopt_with_layout, import_with_layout, inspect_with_layout,
    installs_from,
};
use crate::codex::CODEX_ID;

/// Codex's home layout.
pub(crate) const CODEX_LAYOUT: Layout = Layout {
    provider: CODEX_ID,
    skip_names: &[
        ".tmp",
        "tmp",
        "log",
        "app-server-control",
        "app-server-daemon",
        "packages",
        ".DS_Store",
        "installation_id",
        "shell_snapshots",
        "thread-writer-locks",
        ".sandbox_migration",
    ],
    skip_suffixes: &[".sqlite", ".sqlite-wal", ".sqlite-shm", ".sqlite-journal"],
    top_level_skips_only: true,
    estimate_depth: 6,
    kept_back: &[(
        crate::sign_in::CODEX_AUTH,
        "your sign-in — it stays with the original; this account signs in once",
    )],
    config_evidence: &[
        "config.toml",
        crate::sign_in::CODEX_AUTH,
        "sessions",
        "history.jsonl",
        "version.json",
    ],
    history: &[
        "sessions",
        "archived_sessions",
        "history.jsonl",
        "session_index.jsonl",
    ],
    private_files: &[crate::sign_in::CODEX_AUTH],
    not_a_config: "this folder does not look like a Codex home (no config.toml, auth.json or sessions found)",
    login_stays_note: "Your Codex sign-in stays with the original folder: a copied sign-in would sign one of the two out the next time either renews it. Sign in once inside the imported account.",
    login_moves_note: "Sign-in found in the copy.",
};

/// Scans for existing Codex homes: `~/.codex`, `$CODEX_HOME`, and any
/// `~/.codex*` directory (manual setups and tools such as
/// codex-profiles keep `~/.codex-<name>`). Parameters exist so tests
/// can pass sandboxes and a deterministic probe.
pub fn scan_codex_environment(
    home: &Path,
    env_override: Option<&Path>,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Vec<DiscoveredInstall> {
    let mut candidates: Vec<(PathBuf, InstallOrigin)> =
        vec![(home.join(".codex"), InstallOrigin::VendorDefault)];
    if let Some(override_path) = env_override {
        candidates.push((override_path.to_path_buf(), InstallOrigin::EnvOverride));
    }
    if let Ok(entries) = std::fs::read_dir(home) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with(".codex") && name != ".codex" {
                candidates.push((entry.path(), InstallOrigin::HomePattern));
            }
        }
    }
    installs_from(&CODEX_LAYOUT, candidates, login_probe, &origin_notes)
}

/// The Confirm-source inspection for a Codex home (metadata only).
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] when the path is not a
/// readable directory.
pub fn inspect_codex_source(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<SourceInspect, juggler_adapter::AdapterError> {
    inspect_with_layout(&CODEX_LAYOUT, source, login_probe)
}

/// ADR-0018: use an existing Codex home where it is. Checks it is a
/// Codex home and not one of AI Juggler's own copies, and reports its
/// sign-in from `auth.json`'s shape — read-only: nothing is copied,
/// marked or written, and `auth.json` stays the one copy of its
/// single-use refresh token.
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] with a plain reason.
pub fn adopt_codex_home(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    adopt_with_layout(&CODEX_LAYOUT, source, login_probe)
}

/// Copy-imports a Codex home into a fresh managed home.
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] for a missing,
/// non-Codex, already-managed or oversize source, or a destination that
/// already exists; a mid-copy failure removes the partial copy first.
pub fn import_codex_home(
    source: &Path,
    dest: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    import_with_layout(
        &CODEX_LAYOUT,
        source,
        dest,
        login_probe,
        MAX_IMPORT_BYTES,
        &[],
    )
}

/// The settings-only import: the history (`sessions/`,
/// `archived_sessions/`, `history.jsonl`, `session_index.jsonl`) stays
/// behind, each skip listed with its size.
///
/// # Errors
///
/// As [`import_codex_home`].
pub fn import_codex_settings_only(
    source: &Path,
    dest: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    let mut outcome = import_with_layout(
        &CODEX_LAYOUT,
        source,
        dest,
        login_probe,
        MAX_IMPORT_BYTES,
        CODEX_LAYOUT.history,
    )?;
    outcome.degraded = true;
    Ok(outcome)
}

fn origin_notes(origin: InstallOrigin, login: LoginProbe) -> Vec<String> {
    let mut notes = vec![match origin {
        InstallOrigin::VendorDefault => {
            "Codex's standard location — what a plain `codex` command uses.".to_string()
        }
        InstallOrigin::EnvOverride => {
            "Your environment points CODEX_HOME here — likely another tool or a manual setup."
                .to_string()
        }
        InstallOrigin::HomePattern => {
            "A separate Codex home — created manually or by another tool.".to_string()
        }
        InstallOrigin::ManagerTool => {
            "Managed by another tool — importing copies it; the tool is never touched.".to_string()
        }
        InstallOrigin::ShellAlias => {
            "Your shell points CODEX_HOME here — a setup you may have forgotten about.".to_string()
        }
    }];
    if matches!(login, LoginProbe::LoggedIn) {
        notes.push("Signed in — importing keeps your settings and session history.".to_string());
    }
    notes
}
