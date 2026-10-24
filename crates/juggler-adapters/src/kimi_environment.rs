//! Kimi Code homes on this machine, and bringing one in (AJ-E1603).
//!
//! A home is `KIMI_CODE_HOME`: `~/.kimi-code` by default, holding
//! everything Kimi Code keeps (research `docs/research/kimi.md` §2). Two
//! ways in, as for Codex (ADR-0018):
//!
//! - **Where it is** — any home, at its exact path; `~/.kimi-code` then
//!   runs with `KIMI_CODE_HOME` unset, as a plain `kimi` runs it. Nothing
//!   is written to it.
//! - **A copy** into the account's own folder. What stays behind, and why:
//!   - `credentials/` — the sign-ins (and its MCP servers'). One sign-in
//!     in two homes would be one device twice, and a renewed token in one
//!     could sign the other out; the copy signs in on its own.
//!   - `server.token` — the local server's key.
//!   - `sessions/` and `session_index.jsonl` — the list may name the
//!     original's folders, so a copy could resume into the original
//!     [unverified: the schema of `sessionDir` isn't documented; on the
//!     live checklist]. The original keeps them, and can be used where
//!     it is.
//!   - Logs, locks, the bundled tools, update state, the local server's
//!     state and the per-home device id: rebuildable.
//!   - A settings file with a key written in it (`config.toml`'s
//!     providers, `mcp.json`, `tui.toml`): never copied, named.

use std::path::{Path, PathBuf};

use juggler_adapter::environment::{
    DiscoveredInstall, ImportOutcome, InstallOrigin, LoginProbe, SourceInspect,
};

use crate::claude_environment::{
    Layout, MAX_IMPORT_BYTES, adopt_with_layout, import_with_layout, inspect_with_layout,
    installs_from,
};
use crate::kimi_code::KIMI_CODE_ID;
use crate::kimi_sessions::{INDEX_FILE, SESSIONS_DIR};

/// The settings files checked for a key before a copy keeps them.
const SETTINGS: &[&str] = &["config.toml", "tui.toml", "mcp.json"];

/// Why the sessions stay with the original.
const SESSIONS_STAY: &str = "your sessions — they stay with the original, which you can use where it is: \
     their list may name its folders, so a copy could change them";

/// Kimi Code's home layout.
pub(crate) const KIMI_LAYOUT: Layout = Layout {
    provider: KIMI_CODE_ID,
    skip_names: &[
        "logs",
        "locks",
        "bin",
        "updates",
        "server",
        "device_id",
        ".DS_Store",
    ],
    skip_suffixes: &[".lock", ".tmp"],
    top_level_skips_only: true,
    estimate_depth: 5,
    kept_back: &[
        (
            crate::sign_in::KIMI_CREDENTIALS,
            "your sign-ins — they stay with the original; this account signs in once",
        ),
        (
            "server.token",
            "the local server's key — it stays with the original",
        ),
        (SESSIONS_DIR, SESSIONS_STAY),
        (INDEX_FILE, SESSIONS_STAY),
    ],
    config_evidence: &[
        "config.toml",
        "tui.toml",
        crate::sign_in::KIMI_CREDENTIALS,
        SESSIONS_DIR,
        INDEX_FILE,
    ],
    history: &[SESSIONS_DIR, INDEX_FILE, "user-history"],
    private_files: &[],
    not_a_config: "this folder does not look like a Kimi Code home (no config.toml, credentials or sessions found)",
    login_stays_note: "Your Kimi Code sign-in stays with the original folder: each folder signs in as its own device. Sign in once inside the imported account.",
    login_moves_note: "Sign-in found in the copy.",
};

/// Scans for existing Kimi Code homes: `~/.kimi-code`, `$KIMI_CODE_HOME`,
/// and any `~/.kimi-code*` folder. The legacy Kimi CLI's `~/.kimi` is not
/// one (its own format, no longer supported by Kimi).
pub fn scan_kimi_environment(
    home: &Path,
    env_override: Option<&Path>,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Vec<DiscoveredInstall> {
    let mut candidates: Vec<(PathBuf, InstallOrigin)> =
        vec![(home.join(".kimi-code"), InstallOrigin::VendorDefault)];
    if let Some(path) = env_override {
        candidates.push((path.to_path_buf(), InstallOrigin::EnvOverride));
    }
    if let Ok(entries) = std::fs::read_dir(home) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with(".kimi-code") && name != ".kimi-code" {
                candidates.push((entry.path(), InstallOrigin::HomePattern));
            }
        }
    }
    installs_from(&KIMI_LAYOUT, candidates, login_probe, &origin_notes)
}

/// The Confirm-source inspection for a Kimi Code home (metadata only).
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] when the path is not a
/// readable directory.
pub fn inspect_kimi_source(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<SourceInspect, juggler_adapter::AdapterError> {
    inspect_with_layout(&KIMI_LAYOUT, source, login_probe)
}

/// ADR-0018: use an existing Kimi Code home where it is — checked, its
/// sign-in told by its files' presence, never written.
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] with a plain reason.
pub fn adopt_kimi_home(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    adopt_with_layout(&KIMI_LAYOUT, source, login_probe)
}

/// Copies a Kimi Code home's settings into a fresh managed home (see the
/// module docs for what stays). `degraded` marks the settings-only form,
/// which is the same copy here.
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] for a missing, non-Kimi,
/// already-managed or oversize source, or a destination that already
/// exists; a mid-copy failure removes the partial copy first.
pub fn import_kimi_home(
    source: &Path,
    dest: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
    degraded: bool,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    let mut outcome = import_with_layout(
        &KIMI_LAYOUT,
        source,
        dest,
        login_probe,
        MAX_IMPORT_BYTES,
        &[],
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
            "Kimi Code's standard location — what a plain `kimi` command uses.".to_string()
        }
        InstallOrigin::EnvOverride => {
            "Your environment points KIMI_CODE_HOME here — likely another tool or a manual setup."
                .to_string()
        }
        InstallOrigin::HomePattern | InstallOrigin::ManagerTool | InstallOrigin::ShellAlias => {
            "A separate Kimi Code home — created manually or by another tool.".to_string()
        }
    }];
    if matches!(login, LoginProbe::LoggedIn) {
        notes.push("Signed in — bringing it in keeps your settings.".to_string());
    }
    notes
}
