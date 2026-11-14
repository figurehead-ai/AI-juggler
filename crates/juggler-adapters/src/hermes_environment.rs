//! The user's own Hermes Agent setups, and bringing one in (AJ-E1607).
//!
//! A Hermes home is one folder (`$HERMES_HOME`, else `~/.hermes`): its
//! `config.yaml`, `SOUL.md`, memories, skills, sessions (`state.db`) and
//! sign-ins (`auth.json`, keys in `.env`). Its named profiles are homes of
//! their own under `~/.hermes/profiles/<name>/` (research `hermes.md` §2).
//! Two ways in (ADR-0018): **where it is** (run as a plain `hermes` runs
//! it, nothing of it written but AI Juggler's own guard folder,
//! [`crate::hermes_guard`]), or **a copy** without its sign-ins, keys and
//! sessions (a copied refresh token would be a second holder of one
//! single-use token: Hermes' own profile docs warn of it).

use crate::common::{home, layout::Layout};
use crate::hermes::HERMES_ID;
use crate::hermes_sessions::STORE_FILE;
use juggler_adapter::environment::{
    DiscoveredInstall, ImportOutcome, InstallOrigin, LoginProbe, SourceInspect,
};
use std::path::Path;

/// What a copy keeps as settings.
const SETTINGS: &[&str] = &["config.yaml", "SOUL.md"];

const SESSIONS_STAY: &str =
    "your sessions — they stay with the original, which you can use where it is";
const SIGN_INS_STAY: &str =
    "your sign-ins and keys — they stay with the original; this account signs in once";

/// A Hermes home's layout.
pub(crate) const HERMES_LAYOUT: Layout = Layout {
    provider: HERMES_ID,
    skip_names: &[
        "logs",
        "cache",
        "checkpoints",
        "hermes-agent",
        "profiles",
        "gateway.pid",
        "gateway.lock",
        "gateway_state.json",
        "spawn-ledger.json",
        "active_profile",
        crate::hermes_guard::OWN_DIR,
        ".DS_Store",
    ],
    skip_suffixes: &[".lock", ".tmp", "-wal", "-shm"],
    top_level_skips_only: true,
    estimate_depth: 3,
    kept_back: &[
        (crate::sign_in::HERMES_AUTH, SIGN_INS_STAY),
        (crate::sign_in::HERMES_ENV, SIGN_INS_STAY),
        (crate::sign_in::HERMES_SHARED_AUTH, SIGN_INS_STAY),
        (STORE_FILE, SESSIONS_STAY),
    ],
    config_evidence: &[
        "config.yaml",
        "SOUL.md",
        STORE_FILE,
        crate::sign_in::HERMES_AUTH,
    ],
    history: &[STORE_FILE, "memories"],
    private_files: &[],
    not_a_config: "this folder does not look like a Hermes Agent home (no config.yaml, SOUL.md, state.db or sign-ins found)",
    login_stays_note: "Your Hermes sign-ins stay with the original folder. Sign in once inside the imported account.",
    login_moves_note: "Sign-in found in the copy.",
};

/// Scans for the user's own Hermes homes: `~/.hermes`, the folder
/// `$HERMES_HOME` names, and each named profile under `~/.hermes/profiles`.
pub fn scan_hermes_environment(
    home: &Path,
    env_override: Option<&Path>,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Vec<DiscoveredInstall> {
    let mut candidates = home::default_candidates(home, ".hermes", env_override);
    if let Ok(entries) = std::fs::read_dir(home.join(".hermes/profiles")) {
        candidates.extend(
            entries
                .flatten()
                .filter(|entry| entry.path().is_dir())
                .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
                .map(|entry| (entry.path(), InstallOrigin::HomePattern)),
        );
    }
    home::installs_from(&HERMES_LAYOUT, candidates, login_probe, &origin_notes)
}

/// The Confirm-source inspection for a Hermes home (metadata only).
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] when the path is not a
/// readable directory.
pub fn inspect_hermes_source(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<SourceInspect, juggler_adapter::AdapterError> {
    home::inspect_with_layout(&HERMES_LAYOUT, source, login_probe)
}

/// ADR-0018: use a Hermes home where it is, checked, never copied.
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] with a plain reason.
pub fn adopt_hermes_home(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    home::adopt_with_layout(&HERMES_LAYOUT, source, login_probe)
}

/// A copy of a Hermes home's settings, without its sign-ins, keys and
/// sessions.
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] when the copy can't be
/// made (the destination is there, it's too big, a file can't be read).
pub fn import_hermes_home(
    source: &Path,
    dest: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
    settings_only: bool,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    // Settings only: its memories and sessions stay behind too.
    let left: &[&str] = if settings_only {
        HERMES_LAYOUT.history
    } else {
        &[]
    };
    home::copy_home(
        &HERMES_LAYOUT,
        (source, dest),
        login_probe,
        SETTINGS,
        left,
        settings_only,
    )
}

/// What the scan says of where a home came from, and of its sign-in.
fn origin_notes(origin: InstallOrigin, login: LoginProbe) -> Vec<String> {
    let place = match origin {
        InstallOrigin::VendorDefault => {
            "Hermes Agent's standard home — what a plain `hermes` command uses."
        }
        InstallOrigin::EnvOverride => {
            "Your environment points HERMES_HOME here — likely another tool or a manual setup."
        }
        InstallOrigin::HomePattern | InstallOrigin::ManagerTool | InstallOrigin::ShellAlias => {
            "One of your Hermes profiles (`hermes -p <name>`)."
        }
    };
    let signed_in = matches!(login, LoginProbe::LoggedIn)
        .then_some("Signed in — bringing it in keeps your settings.");
    std::iter::once(place)
        .chain(signed_in)
        .map(str::to_owned)
        .collect()
}
