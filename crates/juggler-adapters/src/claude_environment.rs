//! Claude Code environment scan + config import (spec §12, ADR-0006).
//!
//! The scan answers "what Claude setups already exist on this machine?"
//! for the onboarding wizard; the import turns one of them into a managed
//! account with a byte copy. Both are bound by ADR-0003: metadata-only
//! probing, zero writes outside managed dirs, sources never modified.

use std::path::{Path, PathBuf};

use juggler_adapter::environment::{
    DiscoveredInstall, ImportOutcome, InstallOrigin, LoginProbe, SourceInspect,
};

use crate::claude_code::CLAUDE_CODE_ID;
pub use crate::common::home::MAX_IMPORT_BYTES;
use crate::common::home::{
    adopt_with_layout, import_with_layout, inspect_with_layout, installs_from,
};
use crate::common::layout::Layout;

/// Directory/file names inside a Claude config dir that are rebuildable
/// cache or telemetry noise — skipped on import (`shell-snapshots/` alone
/// can dwarf the useful state).
const SKIP_ON_IMPORT: &[&str] = &[
    "shell-snapshots",
    "statsig",
    "telemetry",
    "cache",
    ".DS_Store",
];

/// Top-level names that make a directory look like a real Claude config —
/// a bare `.claude-something` dir with none of these is noise, not a setup.
const CONFIG_EVIDENCE: &[&str] = &[
    crate::sign_in::CLAUDE_CREDENTIALS,
    "settings.json",
    "settings.local.json",
    ".claude.json",
    "projects",
    "keybinds.json",
];

/// Claude Code's home layout.
pub(crate) const CLAUDE_LAYOUT: Layout = Layout {
    provider: CLAUDE_CODE_ID,
    skip_names: SKIP_ON_IMPORT,
    skip_suffixes: &[],
    top_level_skips_only: false,
    estimate_depth: 2,
    // AJ-E1521: a sign-in is never copied. Two folders holding one
    // OAuth refresh token sign each other out when either renews it.
    kept_back: &[(
        crate::sign_in::CLAUDE_CREDENTIALS,
        "your sign-in — it stays with the original; this account signs in once",
    )],
    config_evidence: CONFIG_EVIDENCE,
    history: HISTORY_DIRS,
    private_files: &[crate::sign_in::CLAUDE_CREDENTIALS],
    not_a_config: "this folder does not look like a Claude Code configuration (no settings, credentials, or session history found)",
    login_stays_note: "Your Claude sign-in stays with the original folder: the macOS Keychain locks it to that folder, and a copied sign-in file would sign one of the two out. Sign in once inside the imported account; it takes a few seconds.",
    login_moves_note: "Login carried over with the import.",
};

/// Scans for existing Claude Code configurations under `home`, the
/// `$CLAUDE_CONFIG_DIR` override when set, and any other manager tool's
/// profile roots (`manager_roots` — e.g. t3.codes' `ClaudeCodeProfiles`).
/// The trait method passes the real locations and the metadata-only login
/// probe; tests pass sandboxes and a deterministic probe — the parameters
/// exist precisely so behavior is observable without touching real state.
pub fn scan_claude_environment(
    home: &Path,
    env_override: Option<&Path>,
    manager_roots: &[PathBuf],
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Vec<DiscoveredInstall> {
    let mut candidates: Vec<(PathBuf, InstallOrigin)> =
        vec![(home.join(".claude"), InstallOrigin::VendorDefault)];
    if let Some(override_path) = env_override {
        candidates.push((override_path.to_path_buf(), InstallOrigin::EnvOverride));
    }
    // Other manager tools keep whole profile roots; each direct child that
    // looks like a config is one of their profiles.
    for root in manager_roots {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                candidates.push((entry.path(), InstallOrigin::ManagerTool));
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(home) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            // Manual / other-tool / AI-created setups: anything dot-prefixed
            // starting with `.claude`, except the default dir itself and
            // the vendor's `.claude.json` state file.
            if name.starts_with(".claude") && name != ".claude" && name != ".claude.json" {
                candidates.push((entry.path(), InstallOrigin::HomePattern));
            }
        }
    }

    installs_from(&CLAUDE_LAYOUT, candidates, login_probe, &origin_notes)
}

/// The history-bearing top-level dirs the settings-only degraded
/// import skips (AJ-E307 Req.4) — session transcripts and plans.
const HISTORY_DIRS: &[&str] = &["projects", "todos"];

/// Inspects a candidate source BEFORE any copy (Confirm-source step).
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] when the path is not a directory.
pub fn inspect_source(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<SourceInspect, juggler_adapter::AdapterError> {
    inspect_with_layout(&CLAUDE_LAYOUT, source, login_probe)
}

/// AJ-E307 Req.4: the degraded import — settings only, history dirs
/// skipped by name, each skip recorded with its size so the wizard
/// shows exactly what stayed behind.
/// # Errors
///
/// Same conditions as [`import_claude_config`] (the history dirs are
/// skipped before any ceiling check applies to them).
pub fn import_settings_only(
    source: &Path,
    dest: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    let mut outcome =
        import_with_cap_skips(source, dest, login_probe, MAX_IMPORT_BYTES, HISTORY_DIRS)?;
    outcome.degraded = true;
    Ok(outcome)
}

/// Copy-imports an existing Claude Code configuration into a managed
/// account dir (the default and ONLY default-path mode — Req.7: no
/// adopt-in-place).
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] for a missing,
/// non-config, already-managed, or oversize source, or a destination
/// that already exists.
pub fn import_claude_config(
    source: &Path,
    dest: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    import_with_cap(source, dest, login_probe, MAX_IMPORT_BYTES)
}

/// Copy-imports with an explicit byte ceiling — the testable core of
/// [`import_claude_config`].
///
/// # Errors
///
/// Same conditions as [`import_claude_config`], with the ceiling
/// taken from `max_bytes` instead of [`MAX_IMPORT_BYTES`].
pub fn import_with_cap(
    source: &Path,
    dest: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
    max_bytes: u64,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    import_with_cap_skips(source, dest, login_probe, max_bytes, &[])
}

/// [`import_with_cap`] with extra top-level skips (the degraded
/// import's history dirs), a per-folder breakdown in the outcome,
/// and SESSION-SCOPED cleanup on mid-copy failure: the import
/// refuses a `dest` that exists on entry, so anything at `dest`
/// mid-call is definitionally ours — removing it can never touch a
/// foreign dir (Req.6: orphans never accumulate silently).
///
/// # Errors
///
/// Same conditions as [`import_claude_config`]; a mid-copy failure
/// removes the partial managed dir before the error returns.
pub fn import_with_cap_skips(
    source: &Path,
    dest: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
    max_bytes: u64,
    extra_skips: &[&str],
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    import_with_layout(
        &CLAUDE_LAYOUT,
        source,
        dest,
        login_probe,
        max_bytes,
        extra_skips,
    )
}

/// ADR-0018: use an existing setup where it is. Checks it is a Claude
/// Code folder and not one of AI Juggler's own copies, and reports its
/// sign-in — read-only: nothing is copied, marked or written. The login
/// is probed at `source` exactly as given (Claude's Keychain name hashes
/// that path string).
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] with a plain reason.
pub fn adopt_claude_config(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    adopt_with_layout(&CLAUDE_LAYOUT, source, login_probe)
}

fn origin_notes(origin: InstallOrigin, login: LoginProbe) -> Vec<String> {
    let mut notes = vec![match origin {
        InstallOrigin::VendorDefault => {
            "Claude Code's standard location — what a plain `claude` command uses.".to_string()
        }
        InstallOrigin::EnvOverride => {
            "Your environment points CLAUDE_CONFIG_DIR here — likely another manager tool or a manual setup.".to_string()
        }
        InstallOrigin::HomePattern => {
            "A separate Claude configuration — created manually, by another tool, or by an AI assistant.".to_string()
        }
        InstallOrigin::ManagerTool => {
            "Managed by another tool (t3.codes) — importing copies it; the tool and its sessions are never touched.".to_string()
        }
        InstallOrigin::ShellAlias => {
            "Your shell points CLAUDE_CONFIG_DIR here — a setup you may have forgotten about.".to_string()
        }
    }];
    if matches!(login, LoginProbe::LoggedIn) {
        notes.push("Signed in — importing keeps your settings and session history.".to_string());
    }
    notes
}
