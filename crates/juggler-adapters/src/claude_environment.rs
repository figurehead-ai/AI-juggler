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
use juggler_core::model::HarnessId;

use crate::claude_code::{CLAUDE_CODE_ID, is_managed_dir, prepare_managed_dir_for};

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
    ".credentials.json",
    "settings.json",
    "settings.local.json",
    ".claude.json",
    "projects",
    "keybinds.json",
];

/// What differs between Providers' config homes for the scan, the
/// inspection and the copy (AJ-E1302); the machinery in this module is
/// shared — Codex supplies its own [`Layout`].
pub(crate) struct Layout {
    /// The Provider id written into discovered installs and the marker.
    pub(crate) provider: &'static str,
    /// Entry names skipped as rebuildable state.
    pub(crate) skip_names: &'static [&'static str],
    /// File-name suffixes skipped as rebuildable state.
    pub(crate) skip_suffixes: &'static [&'static str],
    /// Top-level names that make a directory a real config home.
    pub(crate) config_evidence: &'static [&'static str],
    /// Top-level history entries the settings-only import leaves behind.
    pub(crate) history: &'static [&'static str],
    /// Files kept owner-only (0600) in the copy.
    pub(crate) private_files: &'static [&'static str],
    /// The plain words when a folder is not this Provider's config.
    pub(crate) not_a_config: &'static str,
    /// Signed in at the source, not in the copy.
    pub(crate) login_stays_note: &'static str,
    /// Signed in at both.
    pub(crate) login_moves_note: &'static str,
}

impl Layout {
    /// Whether `name` is rebuildable state, never copied.
    pub(crate) fn skips(&self, name: &str) -> bool {
        self.skip_names.contains(&name)
            || self
                .skip_suffixes
                .iter()
                .any(|suffix| name.ends_with(suffix))
    }

    /// Whether `dir` carries evidence of a real config home.
    pub(crate) fn is_config_dir(&self, dir: &Path) -> bool {
        self.config_evidence
            .iter()
            .any(|name| dir.join(name).exists())
    }
}

/// Claude Code's home layout.
pub(crate) const CLAUDE_LAYOUT: Layout = Layout {
    provider: CLAUDE_CODE_ID,
    skip_names: SKIP_ON_IMPORT,
    skip_suffixes: &[],
    config_evidence: CONFIG_EVIDENCE,
    history: HISTORY_DIRS,
    private_files: &[".credentials.json"],
    not_a_config: "this folder does not look like a Claude Code configuration (no settings, credentials, or session history found)",
    login_stays_note: "Your Claude login lives in the macOS Keychain, locked to the original folder — it can't (and shouldn't) be moved. Sign in once inside the imported account; it takes a few seconds.",
    login_moves_note: "Login carried over with the import.",
};

/// Default import ceiling: config dirs beyond this are almost entirely
/// transcript/cache bulk; the wizard offers a fresh account instead.
pub const MAX_IMPORT_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB

/// Walk ceiling for size estimation (bounds pathological trees).
const MAX_WALK_ENTRIES: usize = 20_000;

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

/// Turns candidate paths into discovered installs: canonical, deduped,
/// config-only, metadata-only, most useful first. Shared by every
/// Provider's scan.
pub(crate) fn installs_from(
    layout: &Layout,
    candidates: Vec<(PathBuf, InstallOrigin)>,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
    origin_notes: &dyn Fn(InstallOrigin, LoginProbe) -> Vec<String>,
) -> Vec<DiscoveredInstall> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut found: Vec<DiscoveredInstall> = Vec::new();
    for (path, origin) in candidates {
        let Ok(canonical) = path.canonicalize() else {
            continue;
        };
        if seen.contains(&canonical) || !layout.is_config_dir(&canonical) {
            continue;
        }
        seen.push(canonical.clone());
        let login = login_probe(&canonical);
        let managed = is_managed_dir(&canonical);
        let bytes = estimate_bytes(layout, &canonical);
        let last_used_secs = newest_mtime_secs(&canonical);
        let mut notes = origin_notes(origin, login);
        if managed {
            notes.push("Already managed by AI Juggler.".to_string());
        }
        found.push(DiscoveredInstall {
            harness: HarnessId::new(layout.provider),
            path: canonical,
            origin,
            login,
            bytes,
            last_used_secs,
            managed,
            notes,
        });
    }

    // Most useful first: the default location, then logged-in installs,
    // then most recently used.
    found.sort_by(|a, b| {
        origin_rank(a.origin)
            .cmp(&origin_rank(b.origin))
            .then(login_rank(a.login).cmp(&login_rank(b.login)))
            .then(b.last_used_secs.cmp(&a.last_used_secs))
    });
    found
}

/// The bounded walk cap (perf budget: ≤ 2 s for ≤ 50k files).
const INSPECT_FILE_CAP: u64 = 50_000;

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

/// [`inspect_source`] for any Provider's [`Layout`].
///
/// # Errors
///
/// As [`inspect_source`].
pub(crate) fn inspect_with_layout(
    layout: &Layout,
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<SourceInspect, juggler_adapter::AdapterError> {
    let mut inspect = SourceInspect {
        ceiling_bytes: MAX_IMPORT_BYTES,
        ..SourceInspect::default()
    };
    if !source.is_dir() {
        return Err(juggler_adapter::AdapterError::AccountDir {
            path: source.to_path_buf(),
            message: "folder does not exist (it may have moved)".to_string(),
        });
    }
    let canonical =
        source
            .canonicalize()
            .map_err(|e| juggler_adapter::AdapterError::AccountDir {
                path: source.to_path_buf(),
                message: format!("cannot read folder: {e}"),
            })?;
    inspect.is_config = layout.is_config_dir(&canonical);
    inspect.already_managed = is_managed_dir(&canonical);
    inspect.login = login_probe(&canonical);
    let mut entries: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    inspect_walk(layout, &canonical, &canonical, &mut entries, &mut inspect).map_err(|e| {
        juggler_adapter::AdapterError::AccountDir {
            path: source.to_path_buf(),
            message: format!("cannot read folder: {e}"),
        }
    })?;
    let mut folders: Vec<(String, u64)> = entries.into_iter().collect();
    folders.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    inspect.folders = folders;
    Ok(inspect)
}

fn inspect_walk(
    layout: &Layout,
    root: &Path,
    dir: &Path,
    entries: &mut std::collections::BTreeMap<String, u64>,
    inspect: &mut SourceInspect,
) -> Result<(), String> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Ok(()); // unreadable subtree: counts as nothing
    };
    for entry in read.flatten() {
        if inspect.file_count >= INSPECT_FILE_CAP {
            inspect.truncated = true;
            return Ok(());
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let name = entry.file_name();
        let name = name.to_string_lossy().into_owned();
        if layout.skips(&name) {
            continue;
        }
        if meta.is_dir() {
            inspect_walk(layout, root, &entry.path(), entries, inspect)?;
        } else {
            inspect.file_count += 1;
            inspect.total_bytes = inspect.total_bytes.saturating_add(meta.len());
            let relative = entry.path().strip_prefix(root).ok().map(Path::to_path_buf);
            let is_top_level_file = relative
                .as_deref()
                .is_some_and(|rel| rel.components().count() == 1);
            let top = match relative.as_deref().and_then(|rel| rel.components().next()) {
                Some(component) => component.as_os_str().to_string_lossy().into_owned(),
                None => name.clone(),
            };
            let label = if is_top_level_file {
                top
            } else {
                format!("{top}/")
            };
            let slot = entries.entry(label).or_insert(0);
            *slot = slot.saturating_add(meta.len());
        }
    }
    Ok(())
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

/// [`import_with_cap_skips`] for any Provider's [`Layout`] — the one
/// copy path (read-only toward `source`, session-scoped cleanup).
///
/// # Errors
///
/// As [`import_with_cap_skips`].
pub(crate) fn import_with_layout(
    layout: &Layout,
    source: &Path,
    dest: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
    max_bytes: u64,
    extra_skips: &[&str],
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    use juggler_adapter::AdapterError;

    let dir_error = |message: String| AdapterError::AccountDir {
        path: source.to_path_buf(),
        message,
    };

    if !source.is_dir() {
        return Err(dir_error(format!(
            "{} is not a directory",
            source.display()
        )));
    }
    let source = source
        .canonicalize()
        .map_err(|e| dir_error(e.to_string()))?;
    if !layout.is_config_dir(&source) {
        return Err(dir_error(layout.not_a_config.to_string()));
    }
    if is_managed_dir(&source) {
        return Err(dir_error(
            "this folder is already managed by AI Juggler — launch it from Profiles instead of importing".to_string(),
        ));
    }
    if dest.exists() {
        return Err(AdapterError::AccountDir {
            path: dest.to_path_buf(),
            message: "destination already exists".to_string(),
        });
    }

    // Collect first, copy second: fail before writing anything when the
    // tree exceeds the ceiling.
    let mut acc = CollectAccum {
        files: Vec::new(),
        skipped: Vec::new(),
        total: 0,
        folders: Vec::new(),
    };
    collect_files(layout, &source, &source, &mut acc, max_bytes, extra_skips).map_err(dir_error)?;
    let CollectAccum {
        files,
        skipped,
        folders,
        ..
    } = acc;

    let source_login = login_probe(&source);

    // Marker first: even a partially imported dir is identifiable as ours.
    prepare_managed_dir_for(dest, layout.provider)?;

    let mut files_copied: u64 = 0;
    let mut bytes_copied: u64 = 0;
    for (relative, _) in &files {
        let from = source.join(relative);
        let to = dest.join(relative);
        if let Some(parent) = to.parent()
            && let Err(e) = crate::claude_code::create_private_dir(parent)
        {
            cleanup_partial(dest);
            return Err(AdapterError::AccountDir {
                path: to.clone(),
                message: format!(
                    "cannot create {} — the partial import was removed; retry or pick a different folder ({e})",
                    parent.display()
                ),
            });
        }
        if let Err(e) = std::fs::copy(&from, &to) {
            cleanup_partial(dest);
            return Err(AdapterError::AccountDir {
                path: to.clone(),
                message: format!(
                    "cannot copy {} — the partial import was removed; retry or pick a different folder ({e})",
                    relative.display()
                ),
            });
        }
        if relative
            .file_name()
            .is_some_and(|n| layout.private_files.iter().any(|f| n == *f))
        {
            // Defense in depth: credentials stay owner-only in the copy.
            let _ = crate::claude_code::restrict_to_owner(&to);
        }
        files_copied += 1;
        bytes_copied += std::fs::metadata(&to).map(|m| m.len()).unwrap_or_default();
    }

    let (login, login_note) = login_carryover(layout, source_login, login_probe(dest));

    Ok(ImportOutcome {
        files_copied,
        bytes_copied,
        skipped,
        login,
        login_note,
        folders,
        degraded: false,
    })
}

/// The post-copy login carryover pair (destination probe + the
/// plain-language note when the login could not move).
fn login_carryover(
    layout: &Layout,
    source: LoginProbe,
    dest: LoginProbe,
) -> (LoginProbe, Option<String>) {
    let note = match (source, dest) {
        (LoginProbe::LoggedIn, LoginProbe::LoggedOut) => Some(layout.login_stays_note.to_string()),
        (LoginProbe::LoggedIn, LoginProbe::LoggedIn) => Some(layout.login_moves_note.to_string()),
        _ => None,
    };
    (dest, note)
}

/// Sizes a skipped subtree (bounded by the walk cap; metadata only).
fn size_tree(dir: &Path, total: &mut u64) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            size_tree(&entry.path(), total);
        } else {
            *total = total.saturating_add(meta.len());
        }
    }
}

/// Req.6: best-effort removal of THIS import's partial managed dir
/// (safe by construction — see [`import_with_cap_skips`]).
fn cleanup_partial(dest: &Path) {
    let _ = std::fs::remove_dir_all(dest);
}

// ---- internals -----------------------------------------------------------

pub(crate) fn origin_rank(origin: InstallOrigin) -> u8 {
    match origin {
        InstallOrigin::VendorDefault => 0,
        InstallOrigin::EnvOverride => 1,
        InstallOrigin::HomePattern => 2,
        InstallOrigin::ManagerTool | InstallOrigin::ShellAlias => 3,
    }
}

fn login_rank(login: LoginProbe) -> u8 {
    match login {
        LoginProbe::LoggedIn => 0,
        LoginProbe::LoggedOut => 1,
    }
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

/// Sums file sizes to depth 2 (config dirs keep bulk under `projects/`),
/// bounded by [`MAX_WALK_ENTRIES`]; skips the same noise the import skips.
pub(crate) fn estimate_bytes(layout: &Layout, dir: &Path) -> u64 {
    let mut total = 0u64;
    let mut entries_seen = 0usize;
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    while let Some((current, depth)) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in read.flatten() {
            entries_seen += 1;
            if entries_seen > MAX_WALK_ENTRIES {
                return total;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy().into_owned();
            if layout.skips(&name) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() && depth < 2 {
                stack.push((entry.path(), depth + 1));
            } else {
                total += meta.len();
            }
        }
    }
    total
}

pub(crate) fn newest_mtime_secs(dir: &Path) -> Option<u64> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|entry| entry.metadata().ok())
        .filter_map(|meta| meta.modified().ok())
        .map(|time| {
            time.duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or_default()
        })
        .max()
}

/// Depth-first file collection relative to `root`; noise entries are
/// skipped and recorded. Fails (as a string) once the running total
/// exceeds `max_bytes` — before any copying happens.
/// The collect pass's accumulators (one bundle — keeps the walker's
/// argument list inside clippy's budget).
struct CollectAccum {
    files: Vec<(PathBuf, u64)>,
    skipped: Vec<String>,
    total: u64,
    folders: Vec<(String, u64)>,
}

fn collect_files(
    layout: &Layout,
    root: &Path,
    dir: &Path,
    acc: &mut CollectAccum,
    max_bytes: u64,
    extra_skips: &[&str],
) -> Result<(), String> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Ok(()); // unreadable subtree: skip it, don't fail the import
    };
    for entry in read.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy().into_owned();
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let relative = match entry.path().strip_prefix(root) {
            Ok(relative) => relative.to_path_buf(),
            Err(_) => entry.path(),
        };
        if meta.is_dir() {
            if layout.skips(&name) {
                acc.skipped
                    .push(format!("{}/ (rebuildable cache)", relative.display()));
                continue;
            }
            if extra_skips.contains(&name.as_str()) {
                // Size the skipped tree so the degraded import's skip
                // note can say how much history stayed behind.
                let mut skipped_bytes = 0u64;
                size_tree(&entry.path(), &mut skipped_bytes);
                #[allow(clippy::cast_precision_loss)]
                let gib = skipped_bytes as f64 / 1_073_741_824.0;
                acc.skipped.push(format!(
                    "{}/ ({gib:.1} GiB history — skipped at your choice)",
                    relative.display(),
                ));
                continue;
            }
            collect_files(layout, root, &entry.path(), acc, max_bytes, extra_skips)?;
        } else if layout.skips(&name) {
            acc.skipped
                .push(format!("{} (rebuildable)", relative.display()));
        } else if relative.components().count() == 1 && extra_skips.contains(&name.as_str()) {
            // A top-level history FILE (Codex's `history.jsonl`) stays
            // behind with its size, like a history folder.
            #[allow(clippy::cast_precision_loss)]
            let mib = meta.len() as f64 / 1_048_576.0;
            acc.skipped.push(format!(
                "{} ({mib:.1} MiB history — skipped at your choice)",
                relative.display(),
            ));
        } else {
            acc.total += meta.len();
            if acc.total > max_bytes {
                return Err(format!(
                    "configuration is larger than the {} GiB import ceiling — add a fresh account instead",
                    max_bytes / (1024 * 1024 * 1024)
                ));
            }
            // Per-top-level-entry accounting (the breakdown list) —
            // computed before `relative` moves into the file list.
            let top = match relative.components().next() {
                Some(component) => component.as_os_str().to_string_lossy().into_owned(),
                None => name.clone(),
            };
            let label = if relative.components().count() > 1 {
                format!("{top}/")
            } else {
                top
            };
            match acc
                .folders
                .iter_mut()
                .find(|(existing, _)| *existing == label)
            {
                Some(slot) => slot.1 += meta.len(),
                None => acc.folders.push((label, meta.len())),
            }
            acc.files.push((relative, meta.len()));
        }
    }
    Ok(())
}
