//! The user's own Muse Code setup, and bringing it in (AJ-E1608).
//!
//! Muse's setup is its config folder, `~/.config/muse` (settings, the
//! sign-in record, skills), with its sessions in `~/.local/share/muse`.
//! One way in (ADR-0018): **where it is**, the user's own `~/.config/muse`
//! at its exact path, run as a plain `muse` runs it; nothing is written to
//! it. No copy is offered: Muse's sign-in is one per Mac (its Keychain
//! item), so a copy would sign in to nothing of its own, and any other
//! account starts fresh with its own API key ([`crate::muse::ONE_SIGN_IN`]).

use std::path::Path;

use juggler_adapter::environment::{
    DiscoveredInstall, ImportOutcome, InstallOrigin, LoginProbe, SourceInspect,
};

use crate::{
    common::{home, layout::Layout},
    muse::MUSE_ID,
};

/// Muse Code's config folder layout.
pub(crate) const MUSE_LAYOUT: Layout = Layout {
    provider: MUSE_ID,
    skip_names: &["locks", ".DS_Store"],
    skip_suffixes: &[".lock", ".tmp"],
    top_level_skips_only: true,
    estimate_depth: 3,
    kept_back: &[(
        crate::sign_in::MUSE_AUTH,
        "your sign-in record — it stays with the original",
    )],
    config_evidence: &[
        "settings.json",
        "trust.json",
        crate::sign_in::MUSE_AUTH,
        "skills",
    ],
    history: &[],
    private_files: &[],
    not_a_config: "this folder does not look like Muse Code's (no settings.json, trust.json, sign-in record or skills found)",
    login_stays_note: "Muse Code's sign-in is one per computer: only your own Muse uses it.",
    login_moves_note: "Signed in.",
};

/// Scans for the user's own Muse Code setup, `~/.config/muse` (and the
/// folder `$XDG_CONFIG_HOME/muse` when that variable is set).
pub fn scan_muse_environment(
    home: &Path,
    env_override: Option<&Path>,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Vec<DiscoveredInstall> {
    let config = env_override.map(|root| root.join("muse"));
    let candidates = home::default_candidates(home, ".config/muse", config.as_deref());
    home::installs_from(&MUSE_LAYOUT, candidates, login_probe, &origin_notes)
}

/// The Confirm-source inspection for a Muse config folder (metadata only).
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] when the path is not a
/// readable directory.
pub fn inspect_muse_source(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<SourceInspect, juggler_adapter::AdapterError> {
    home::inspect_with_layout(&MUSE_LAYOUT, source, login_probe)
}

/// ADR-0018: use the user's own `~/.config/muse` where it is — checked,
/// its sign-in told by its record's presence, never written. Any other
/// folder is refused: AI Juggler runs the user's own Muse as a plain
/// `muse` does, and only that one.
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] with a plain reason.
pub fn adopt_muse_config(
    source: &Path,
    own: Option<&Path>,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if own.is_none_or(|own| canonical(own) != canonical(source)) {
        return Err(juggler_adapter::AdapterError::AccountDir {
            path: source.to_path_buf(),
            message: "only your own Muse Code setup (~/.config/muse) can be used where it is: \
                      Muse's sign-in is one per computer"
                .to_owned(),
        });
    }
    home::adopt_with_layout(&MUSE_LAYOUT, source, login_probe)
}

fn origin_notes(origin: InstallOrigin, login: LoginProbe) -> Vec<String> {
    let mut notes = vec![match origin {
        InstallOrigin::VendorDefault => {
            "Muse Code's standard location — what a plain `muse` command uses.".to_string()
        }
        InstallOrigin::EnvOverride => {
            "Your environment points XDG_CONFIG_HOME here — likely another tool or a manual setup."
                .to_string()
        }
        InstallOrigin::HomePattern | InstallOrigin::ManagerTool | InstallOrigin::ShellAlias => {
            "A separate Muse Code setup — created manually or by another tool.".to_string()
        }
    }];
    if matches!(login, LoginProbe::LoggedIn) {
        notes.push("Signed in — use it where it is.".to_string());
    }
    notes
}
