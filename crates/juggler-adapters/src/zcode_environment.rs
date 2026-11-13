//! `ZCode` trees on this machine, and bringing one in (AJ-E1605 Req.5).
//!
//! A tree is the folder `ZCode` keeps everything in: `~/.zcode` by
//! default (its settings under `cli/`, its sign-in and the desktop app's
//! state under `v2/`, its rules and skills at the top). Two ways in
//! (ADR-0018):
//!
//! - **Where it is** — `ZCode`'s own `~/.zcode` only, which then runs with
//!   every variable unset, exactly as a plain `zcode` runs. Nothing is
//!   written to it. (Another tree, the beta's `~/.zcode-beta`, keeps its
//!   sign-in where the engine's defaults can't tell, so it comes in as a
//!   copy.)
//! - **A copy** into the App's own tree (`<App>/.zcode`). What stays
//!   behind, and why:
//!   - `v2/credentials.json` — the sign-in (Z.ai's tokens and any
//!     provider keys). One sign-in in two trees would be one device twice;
//!     the copy signs in on its own.
//!   - `cli/db/` — the session store (sessions and usage): its rows name
//!     the original's folders, and the original keeps them, usable where
//!     it is. Any other database (`*.sqlite`, `*.db`: the desktop app's
//!     task index) stays too.
//!   - `cli/rollout/` and `cli/debug/` — the record of every model call,
//!     which can hold the sign-in's request headers.
//!   - `cli/exec/`, `cli/artifacts/`, `cli/shell-snapshots/` — each
//!     session's working files: they stay with the sessions.
//!   - Logs (`cli/log/`, `v2/logs/`, `v2/crash/`), the plugins' download
//!     cache and the desktop app's certificates: rebuildable, or the
//!     original's own.
//!   - `cli/config.json`, `AGENTS.md`, `skills/` and `commands/` — the
//!     engine reads these from the home folder for every App whatever its
//!     tree (research `docs/research/zcode.md`, round-2 addendum), so a
//!     copy would sit unread; the App has them already.
//!   - A settings file with a key written in it (`v2/provider_config.json`
//!     with a provider's key, `v2/setting.json`): never copied, named.

use std::path::Path;

use juggler_adapter::AdapterError;
use juggler_adapter::environment::{
    DiscoveredInstall, ImportOutcome, InstallOrigin, LoginProbe, SourceInspect,
};

use crate::common::{home, layout::Layout};
use crate::zcode::{TREE_NAME, ZCODE_ID};

/// The settings files checked for a key before a copy keeps them.
const SETTINGS: &[&str] = &["v2/provider_config.json", "v2/setting.json"];

/// Why the session store and each session's files stay.
const SESSIONS_STAY: &str = "your sessions and usage — they stay with the original, which you can use where it is: their records name its folders";
/// Why the files every App reads from the home folder aren't copied.
const READ_FROM_HOME: &str =
    "ZCode reads this from your home folder for every App, so the App has it already";

/// `ZCode`'s tree layout.
pub(crate) const ZCODE_LAYOUT: Layout = Layout {
    provider: ZCODE_ID,
    skip_names: &[".DS_Store"],
    // Any database, wherever it is (the session store, the desktop app's
    // task index): never copied as raw files that may be mid-write.
    skip_suffixes: &[
        ".sqlite",
        ".sqlite-wal",
        ".sqlite-shm",
        ".db",
        ".db-wal",
        ".db-shm",
        ".lock",
        ".tmp",
    ],
    top_level_skips_only: false,
    estimate_depth: 5,
    kept_back: &[
        (
            crate::sign_in::ZCODE_SIGN_IN_PATH,
            "your sign-in — it stays with the original; this App signs in once",
        ),
        ("cli/db", SESSIONS_STAY),
        ("cli/exec", SESSIONS_STAY),
        ("cli/artifacts", SESSIONS_STAY),
        ("cli/shell-snapshots", SESSIONS_STAY),
        (
            "cli/rollout",
            "the record of every model call, which can hold your sign-in's headers — it stays with the original",
        ),
        (
            "cli/debug",
            "the record of every model call, which can hold your sign-in's headers — it stays with the original",
        ),
        ("cli/log", "logs — rebuildable"),
        ("v2/logs", "logs — rebuildable"),
        ("v2/crash", "crash reports — they stay with the original"),
        (
            "v2/certs",
            "the desktop app's certificates — they stay with the original",
        ),
        (
            "cli/plugins/cache",
            "the plugins' download cache — rebuildable",
        ),
        ("cli/config.json", READ_FROM_HOME),
        ("AGENTS.md", READ_FROM_HOME),
        ("skills", READ_FROM_HOME),
        ("commands", READ_FROM_HOME),
    ],
    config_evidence: &["cli/config.json", "cli/db", "v2"],
    history: &["cli/db"],
    private_files: &[],
    not_a_config: "this folder does not look like a ZCode folder (no cli/config.json, session store or v2 folder found)",
    login_stays_note: "Your ZCode sign-in stays with the original folder: each folder signs in as its own device. Sign in once inside the new App.",
    login_moves_note: "Sign-in found in the copy.",
};

/// Scans for `ZCode` trees: `~/.zcode`, `$ZCODE_STORAGE_DIR`, and any
/// `~/.zcode-*` folder (the beta keeps `~/.zcode-beta`).
pub fn scan_zcode_environment(
    home: &Path,
    env_override: Option<&Path>,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Vec<DiscoveredInstall> {
    let mut candidates = home::default_candidates(home, TREE_NAME, env_override);
    candidates.extend(home::home_variants(home, &|name| {
        name.starts_with(".zcode-")
    }));
    home::installs_from(&ZCODE_LAYOUT, candidates, login_probe, &origin_notes)
}

/// The Confirm-source inspection for a `ZCode` tree (metadata only).
///
/// # Errors
///
/// [`AdapterError::AccountDir`] when the path is not a readable directory.
pub fn inspect_zcode_source(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<SourceInspect, AdapterError> {
    home::inspect_with_layout(&ZCODE_LAYOUT, source, login_probe)
}

/// ADR-0018: use `ZCode`'s own tree (`default_tree`, `~/.zcode`) where it
/// is — checked, its sign-in told by its file's presence, never written.
/// Any other tree is refused with why: it comes in as a copy.
///
/// # Errors
///
/// [`AdapterError::AccountDir`] with a plain reason.
pub fn adopt_zcode_tree(
    source: &Path,
    default_tree: Option<&Path>,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, AdapterError> {
    let canonical = |path: &Path| path.canonicalize().ok();
    let is_default = default_tree
        .and_then(canonical)
        .is_some_and(|default| canonical(source).as_ref() == Some(&default));
    if !is_default {
        return Err(AdapterError::AccountDir {
            path: source.to_path_buf(),
            message: "only ZCode's own ~/.zcode can be used where it is (ZCode keeps another folder's sign-in where AI Juggler can't point it) — bring this one in as a copy".to_owned(),
        });
    }
    home::adopt_with_layout(&ZCODE_LAYOUT, source, login_probe)
}

/// Copies a `ZCode` tree's settings into the new App folder `dest`, as
/// its tree `<dest>/.zcode` (see the module docs for what stays).
/// `degraded` marks the settings-only form, which is the same copy here.
///
/// # Errors
///
/// [`AdapterError::AccountDir`] for a missing, non-`ZCode`,
/// already-managed or oversize source, or a destination that already
/// exists; a failure after the copy began removes it first.
pub fn import_zcode_tree(
    source: &Path,
    dest: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
    degraded: bool,
) -> Result<ImportOutcome, AdapterError> {
    home::refuse_existing(dest)?;
    let tree = crate::zcode::tree_of(dest);
    let outcome = match home::import_with_layout(
        &ZCODE_LAYOUT,
        source,
        &tree,
        login_probe,
        home::MAX_IMPORT_BYTES,
        &[],
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            // Refused before anything was written, or a partial copy that
            // `import_with_layout` removed: the App folder goes too.
            if dest.exists() {
                let _ = std::fs::remove_dir_all(dest);
            }
            return Err(error);
        }
    };
    // The App folder is AI Juggler's (its marker), as `prepare_account`
    // leaves it.
    home::prepare_managed_dir_for(dest, ZCODE_ID).map_err(|error| home::undo_copy(dest, &error))?;
    home::finish_copy(dest, &tree, SETTINGS, outcome, degraded)
}

fn origin_notes(origin: InstallOrigin, login: LoginProbe) -> Vec<String> {
    let mut notes = vec![match origin {
        InstallOrigin::VendorDefault => {
            "ZCode's standard folder — what ZCode itself uses.".to_owned()
        }
        InstallOrigin::EnvOverride => {
            "Your environment points ZCODE_STORAGE_DIR here — likely another tool or a manual setup."
                .to_owned()
        }
        InstallOrigin::HomePattern | InstallOrigin::ManagerTool | InstallOrigin::ShellAlias => {
            "A separate ZCode folder — ZCode's beta, or made by hand or by another tool.".to_owned()
        }
    }];
    if matches!(login, LoginProbe::LoggedIn) {
        notes.push("Signed in — bringing it in keeps your settings.".to_owned());
    }
    notes
}
