//! Grok Build homes on this machine, and bringing one in (AJ-E1604).
//!
//! A home is `GROK_HOME`: `~/.grok` by default, holding its settings
//! (`config.toml`, `pager.toml`), sign-ins (`auth.json`,
//! `mcp_credentials.json`), sessions and rules (research
//! `docs/research/grok.md` §2–§4). The installer also keeps the binary,
//! its downloads and completions in `~/.grok`. The unofficial
//! `grok-cli` uses `~/.grok/user-settings.json`: a folder with only that
//! isn't a Grok Build home.
//!
//! Two ways in, as for Codex (ADR-0018):
//! - **Where it is** — any home; `~/.grok` runs with `GROK_HOME` unset.
//!   Nothing is written to it.
//! - **A copy** — settings, rules, hooks and sessions (each session's
//!   folder names its working folder, never the home), but never the
//!   sign-ins (a renewed token in one home could sign the other out), the
//!   installer's binary, downloads and completions, the user guide it
//!   unpacks on each start, or a settings file with a key written in it.

use std::path::{Path, PathBuf};

use juggler_adapter::environment::{
    DiscoveredInstall, ImportOutcome, InstallOrigin, LoginProbe, SourceInspect,
};

use crate::claude_environment::{
    Layout, MAX_IMPORT_BYTES, adopt_with_layout, import_with_layout, inspect_with_layout,
    installs_from,
};
use crate::grok_build::GROK_BUILD_ID;
use crate::grok_sessions::SESSIONS_DIR;

/// The settings files checked for a key before a copy keeps them.
const SETTINGS: &[&str] = &["config.toml", "pager.toml"];

/// Grok Build's home layout.
pub(crate) const GROK_LAYOUT: Layout = Layout {
    provider: GROK_BUILD_ID,
    skip_names: &[
        "bin",
        "downloads",
        "completions",
        "docs",
        "logs",
        "cache",
        ".DS_Store",
    ],
    skip_suffixes: &[".lock", ".tmp"],
    top_level_skips_only: true,
    estimate_depth: 4,
    kept_back: &[
        (
            crate::sign_in::GROK_AUTH,
            "your sign-in — it stays with the original; this account signs in once",
        ),
        (
            crate::sign_in::GROK_MCP_AUTH,
            "your MCP server sign-ins — they stay with the original",
        ),
    ],
    config_evidence: &[
        "config.toml",
        "pager.toml",
        crate::sign_in::GROK_AUTH,
        SESSIONS_DIR,
    ],
    history: &[SESSIONS_DIR],
    private_files: &[],
    not_a_config: "this folder does not look like a Grok Build home (no config.toml, auth.json or sessions found)",
    login_stays_note: "Your Grok sign-in stays with the original folder: a copied sign-in could sign one of the two out when either renews it. Sign in once inside the imported account.",
    login_moves_note: "Sign-in found in the copy.",
};

/// Scans for Grok Build homes: `~/.grok`, `$GROK_HOME`, and any
/// `~/.grok-*` folder.
pub fn scan_grok_environment(
    home: &Path,
    env_override: Option<&Path>,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Vec<DiscoveredInstall> {
    let mut candidates: Vec<(PathBuf, InstallOrigin)> =
        vec![(home.join(".grok"), InstallOrigin::VendorDefault)];
    if let Some(path) = env_override {
        candidates.push((path.to_path_buf(), InstallOrigin::EnvOverride));
    }
    if let Ok(entries) = std::fs::read_dir(home) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with(".grok-") {
                candidates.push((entry.path(), InstallOrigin::HomePattern));
            }
        }
    }
    installs_from(&GROK_LAYOUT, candidates, login_probe, &origin_notes)
}

/// The Confirm-source inspection for a Grok Build home (metadata only).
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] when the path is not a
/// readable directory.
pub fn inspect_grok_source(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<SourceInspect, juggler_adapter::AdapterError> {
    inspect_with_layout(&GROK_LAYOUT, source, login_probe)
}

/// ADR-0018: a Grok Build home used where it is — checked, never written.
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] with a plain reason.
pub fn adopt_grok_home(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    adopt_with_layout(&GROK_LAYOUT, source, login_probe)
}

/// Copies a Grok Build home into a fresh managed home (see the module
/// docs for what stays); `degraded` leaves the sessions behind too.
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] for a missing, non-Grok,
/// already-managed or oversize source, or a destination that already
/// exists; a mid-copy failure removes the partial copy first.
pub fn import_grok_home(
    source: &Path,
    dest: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
    degraded: bool,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    let skips: &[&str] = if degraded { GROK_LAYOUT.history } else { &[] };
    let mut outcome = import_with_layout(
        &GROK_LAYOUT,
        source,
        dest,
        login_probe,
        MAX_IMPORT_BYTES,
        skips,
    )?;
    if let Err(error) = crate::settings_keys::keep_back_keyed(dest, SETTINGS, &mut outcome.skipped)
    {
        let _ = std::fs::remove_dir_all(dest);
        return Err(juggler_adapter::AdapterError::AccountDir {
            path: dest.to_path_buf(),
            message: format!("couldn't finish the copy — the partial copy was removed ({error})"),
        });
    }
    outcome.degraded = degraded;
    Ok(outcome)
}

fn origin_notes(origin: InstallOrigin, login: LoginProbe) -> Vec<String> {
    let mut notes = vec![match origin {
        InstallOrigin::VendorDefault => {
            "Grok Build's standard location — what a plain `grok` command uses.".to_string()
        }
        InstallOrigin::EnvOverride => {
            "Your environment points GROK_HOME here — likely another tool or a manual setup."
                .to_string()
        }
        InstallOrigin::HomePattern | InstallOrigin::ManagerTool | InstallOrigin::ShellAlias => {
            "A separate Grok Build home — created manually or by another tool.".to_string()
        }
    }];
    if matches!(login, LoginProbe::LoggedIn) {
        notes.push("Signed in — bringing it in keeps your settings and sessions.".to_string());
    }
    notes
}
