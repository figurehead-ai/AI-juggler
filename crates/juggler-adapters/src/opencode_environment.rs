//! OpenCode setups on this machine, and bringing one in (AJ-E1602).
//!
//! A setup is OpenCode's **data folder**, `<XDG data>/opencode` — where
//! its sign-ins (`auth.json`) and sessions (`opencode.db`) live. The
//! default is `~/.local/share/opencode`; a user who exports
//! `XDG_DATA_HOME` has `$XDG_DATA_HOME/opencode` instead.
//!
//! Two ways in, as for Codex (ADR-0018):
//!
//! - **Where it is** — only the default folder, which then runs with every
//!   XDG variable unset, exactly as a plain `opencode` runs. Nothing is
//!   written to it. (Any other folder's settings and state folders can't
//!   be known from the data folder alone, so it comes in as a copy.)
//! - **A copy** into the account's own folders (`data/`, `config/`,
//!   `state/`). What stays behind, and why:
//!   - `auth.json` and `mcp-auth.json` — the sign-ins. An OAuth refresh
//!     token renewed in one folder is refused in the other, so the copy
//!     signs in on its own.
//!   - The database's sign-in tables (`credential`, `account`,
//!     `account_state`) and share secrets (`session_share`). The database
//!     itself is copied as a consistent snapshot (`VACUUM INTO` from a
//!     read-only connection, so a running OpenCode is never disturbed);
//!     those tables are then emptied in the copy and the copy vacuumed
//!     again, so no sign-in byte survives in a free page.
//!   - Logs, file snapshots (`snapshot/`, git stores of every edited
//!     tree — large and only used to undo old edits), cloned repos, and
//!     tool output: rebuildable.
//!
//!   The default folder's settings, `~/.config/opencode`, are copied too
//!   (installed plugin packages excepted: OpenCode reinstalls them), but
//!   never a settings file with a key written in it (a provider's
//!   `apiKey`, a server's token): it stays behind, named, and the account
//!   gets its key through its own sign-in. A `{env:NAME}` or
//!   `{file:path}` reference isn't a key.

use std::path::{Path, PathBuf};

use juggler_adapter::AdapterError;
use juggler_adapter::environment::{
    DiscoveredInstall, ImportOutcome, InstallOrigin, LoginProbe, SourceInspect,
};

use crate::claude_environment::{
    Layout, MAX_IMPORT_BYTES, adopt_with_layout, import_with_layout, inspect_with_layout,
    installs_from,
};
use crate::opencode::OPENCODE_ID;
use crate::opencode_store::DB_FILE;

/// Tables holding sign-ins or share secrets — emptied in a copy.
const PRIVATE_TABLES: &[&str] = &["credential", "account", "account_state", "session_share"];

/// OpenCode's data-folder layout.
pub(crate) const OPENCODE_LAYOUT: Layout = Layout {
    provider: OPENCODE_ID,
    skip_names: &[
        "log",
        "snapshot",
        "repos",
        "bin",
        "tool-output",
        "tmp",
        ".DS_Store",
    ],
    // The database is copied as a snapshot (see module docs), never as
    // raw files that may be mid-write.
    skip_suffixes: &[".db", ".db-wal", ".db-shm", ".db-journal"],
    top_level_skips_only: true,
    estimate_depth: 4,
    kept_back: &[
        (
            crate::sign_in::OPENCODE_AUTH,
            "your sign-ins — they stay with the original; this account signs in once",
        ),
        (
            crate::sign_in::OPENCODE_MCP_AUTH,
            "your MCP server sign-ins — they stay with the original",
        ),
    ],
    config_evidence: &[
        DB_FILE,
        crate::sign_in::OPENCODE_AUTH,
        "storage",
        "snapshot",
    ],
    history: &[DB_FILE, "storage"],
    private_files: &[],
    not_a_config: "this folder does not look like OpenCode's data folder (no opencode.db, auth.json or storage found)",
    login_stays_note: "Your OpenCode sign-ins stay with the original folder: a copied sign-in would sign one of the two out the next time either renews it. Sign in once inside the imported account.",
    login_moves_note: "Sign-in found in the copy.",
};

/// The account folder's data folder, `<account>/data/opencode`.
#[must_use]
pub fn managed_data_dir(account_dir: &Path) -> PathBuf {
    account_dir.join("data").join("opencode")
}

/// The account folder's settings folder, `<account>/config/opencode`.
#[must_use]
pub fn managed_config_dir(account_dir: &Path) -> PathBuf {
    account_dir.join("config").join("opencode")
}

/// The account folder's state folder, `<account>/state/opencode`.
#[must_use]
pub fn managed_state_dir(account_dir: &Path) -> PathBuf {
    account_dir.join("state").join("opencode")
}

/// Scans for OpenCode data folders: the default, and `$XDG_DATA_HOME`'s
/// when that variable points elsewhere. Parameters exist so tests can
/// pass sandboxes and a deterministic probe.
pub fn scan_opencode_environment(
    home: &Path,
    xdg_data_home: Option<&Path>,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Vec<DiscoveredInstall> {
    let mut candidates = vec![(
        home.join(".local/share/opencode"),
        InstallOrigin::VendorDefault,
    )];
    if let Some(data) = xdg_data_home {
        candidates.push((data.join("opencode"), InstallOrigin::EnvOverride));
    }
    installs_from(&OPENCODE_LAYOUT, candidates, login_probe, &origin_notes)
}

/// The Confirm-source inspection (metadata only); the database counts
/// at its file size, as its snapshot will.
///
/// # Errors
///
/// [`AdapterError::AccountDir`] when the path is not a readable folder.
pub fn inspect_opencode_source(
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<SourceInspect, AdapterError> {
    let mut inspect = inspect_with_layout(&OPENCODE_LAYOUT, source, login_probe)?;
    if let Ok(meta) = std::fs::metadata(source.join(DB_FILE)) {
        inspect.total_bytes = inspect.total_bytes.saturating_add(meta.len());
        inspect.file_count += 1;
        inspect.folders.push((DB_FILE.to_owned(), meta.len()));
        inspect
            .folders
            .sort_by_key(|entry| std::cmp::Reverse(entry.1));
    }
    Ok(inspect)
}

/// ADR-0018: OpenCode's default data folder used where it is — checked,
/// probed, never written. Any other folder is refused with the reason.
///
/// # Errors
///
/// [`AdapterError::AccountDir`] with a plain reason.
pub fn adopt_opencode_data(
    source: &Path,
    default: Option<&Path>,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, AdapterError> {
    let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if default.is_none_or(|default| canonical(default) != canonical(source)) {
        return Err(AdapterError::AccountDir {
            path: source.to_path_buf(),
            message: "only OpenCode's standard folder (~/.local/share/opencode) can be used where it is — bring this one in as a copy".into(),
        });
    }
    adopt_with_layout(&OPENCODE_LAYOUT, source, login_probe)
}

/// Copies a data folder (and, for the default one, its settings) into a
/// fresh account folder `dest`. See the module docs for what stays.
///
/// # Errors
///
/// [`AdapterError::AccountDir`] for a missing, non-OpenCode,
/// already-managed or oversize source, an existing destination, or a
/// failed copy (the partial copy is removed first).
pub fn import_opencode_data(
    source: &Path,
    dest: &Path,
    settings: Option<&Path>,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
    degraded: bool,
) -> Result<ImportOutcome, AdapterError> {
    if dest.exists() {
        return Err(AdapterError::AccountDir {
            path: dest.to_path_buf(),
            message: "destination already exists".to_string(),
        });
    }
    let data = managed_data_dir(dest);
    let skips: &[&str] = if degraded {
        OPENCODE_LAYOUT.history
    } else {
        &[]
    };
    let result = import_with_layout(
        &OPENCODE_LAYOUT,
        source,
        &data,
        login_probe,
        MAX_IMPORT_BYTES,
        skips,
    )
    .and_then(|mut outcome| {
        // The marker belongs on the account folder, not inside it.
        let _ = std::fs::remove_file(data.join(crate::claude_code::MANAGED_MARKER));
        crate::claude_code::prepare_managed_dir_for(dest, OPENCODE_ID)?;
        for dir in [managed_config_dir(dest), managed_state_dir(dest)] {
            crate::claude_code::create_private_dir(&dir).map_err(|error| account_error(dest, &error))?;
        }
        let db = source.join(DB_FILE);
        if !degraded && db.is_file() {
            let bytes = snapshot_without_sign_ins(&db, &data.join(DB_FILE))
                .map_err(|message| AdapterError::AccountDir {
                    path: dest.to_path_buf(),
                    message: format!("couldn't copy OpenCode's sessions — the partial copy was removed ({message})"),
                })?;
            outcome.files_copied += 1;
            outcome.bytes_copied = outcome.bytes_copied.saturating_add(bytes);
            outcome
                .skipped
                .push(format!("{DB_FILE}: sign-in tables — they stay with the original"));
        }
        if let Some(settings) = settings.filter(|dir| dir.is_dir()) {
            let mut copied = Copied::default();
            copy_settings(settings, &managed_config_dir(dest), &mut copied)
                .map_err(|error| account_error(dest, &error))?;
            outcome.files_copied += copied.files;
            outcome.bytes_copied = outcome.bytes_copied.saturating_add(copied.bytes);
            outcome.skipped.extend(copied.kept_back);
        }
        outcome.degraded = degraded;
        Ok(outcome)
    });
    if result.is_err() {
        // `dest` didn't exist before this call: all of it is this copy's.
        let _ = std::fs::remove_dir_all(dest);
    }
    result
}

fn account_error(dest: &Path, error: &std::io::Error) -> AdapterError {
    AdapterError::AccountDir {
        path: dest.to_path_buf(),
        message: format!("couldn't finish the copy — the partial copy was removed ({error})"),
    }
}

/// A consistent copy of `db` at `to` with every [`PRIVATE_TABLES`] row
/// gone. Returns the copy's size.
fn snapshot_without_sign_ins(db: &Path, to: &Path) -> Result<u64, String> {
    let source = crate::opencode_store::open_read_only(db)?;
    source
        .execute("VACUUM INTO ?1", [to.to_string_lossy().as_ref()])
        .map_err(|error| format!("snapshot: {error}"))?;
    drop(source);
    let copy = rusqlite::Connection::open(to).map_err(|error| format!("open copy: {error}"))?;
    copy.pragma_update(None, "secure_delete", "ON")
        .map_err(|error| format!("secure delete: {error}"))?;
    for table in PRIVATE_TABLES {
        if crate::opencode_store::has_table(&copy, table) {
            // A literal from the list above, never input.
            copy.execute(&format!("DELETE FROM {table}"), [])
                .map_err(|error| format!("empty {table}: {error}"))?;
        }
    }
    copy.execute_batch("VACUUM")
        .map_err(|error| format!("vacuum copy: {error}"))?;
    drop(copy);
    crate::claude_code::restrict_to_owner(to).map_err(|error| error.to_string())?;
    std::fs::metadata(to)
        .map(|meta| meta.len())
        .map_err(|error| error.to_string())
}

/// What a settings copy did.
#[derive(Default)]
struct Copied {
    files: u64,
    bytes: u64,
    /// The files left behind, each with why.
    kept_back: Vec<String>,
}

/// Copies the settings folder, installed plugin packages excepted, and
/// never a settings file with a key written in it.
fn copy_settings(from: &Path, to: &Path, copied: &mut Copied) -> std::io::Result<()> {
    crate::claude_code::create_private_dir(to)?;
    for entry in std::fs::read_dir(from)?.flatten() {
        let name = entry.file_name();
        let text_name = name.to_string_lossy();
        if ["node_modules", ".DS_Store"].contains(&text_name.as_ref()) {
            continue;
        }
        let kind = entry.file_type()?;
        let target = to.join(&name);
        if kind.is_dir() {
            copy_settings(&entry.path(), &target, copied)?;
        } else if kind.is_file() {
            let keyed = crate::settings_keys::format_of(&text_name)
                .is_some_and(|format| crate::settings_keys::file_holds_key(&entry.path(), format));
            if keyed {
                copied
                    .kept_back
                    .push(crate::settings_keys::kept_back_note(&text_name));
                continue;
            }
            copied.bytes += std::fs::copy(entry.path(), &target)?;
            copied.files += 1;
        }
    }
    Ok(())
}

fn origin_notes(origin: InstallOrigin, login: LoginProbe) -> Vec<String> {
    let mut notes = vec![match origin {
        InstallOrigin::VendorDefault => {
            "OpenCode's standard folder — what a plain `opencode` command uses.".to_string()
        }
        InstallOrigin::EnvOverride => {
            "Your environment points XDG_DATA_HOME here — likely another tool or a manual setup."
                .to_string()
        }
        InstallOrigin::HomePattern | InstallOrigin::ManagerTool | InstallOrigin::ShellAlias => {
            "A separate OpenCode folder — created manually or by another tool.".to_string()
        }
    }];
    if matches!(login, LoginProbe::LoggedIn) {
        notes.push("Signed in — bringing it in keeps your settings and sessions.".to_string());
    }
    notes
}
