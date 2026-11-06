//! AJ-E2016: a tool's config home, whatever the tool: candidate folders
//! turned into discovered installs, a source inspected before a copy, the
//! copy itself (links kept inside, never followed out), adoption in place,
//! and the size and recency estimates. Bound by ADR-0003: metadata-only
//! probing, zero writes outside managed dirs, sources never modified.
//! Moved from `claude_environment.rs`; each tool passes its [`Layout`].

use std::path::{Path, PathBuf};

use juggler_adapter::environment::{
    DiscoveredInstall, ImportOutcome, InstallOrigin, LoginProbe, SourceInspect,
};
use juggler_core::model::HarnessId;

use super::layout::Layout;
use juggler_adapter::AdapterError;

/// Default import ceiling: config dirs beyond this are almost entirely
/// transcript/cache bulk; the wizard offers a fresh account instead.
pub const MAX_IMPORT_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB

/// Walk ceiling for size estimation (bounds pathological trees).
const MAX_WALK_ENTRIES: usize = 20_000;

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
        let top_level = dir == root;
        if layout.skips(&name, top_level) || layout.kept_back(&name, top_level).is_some() {
            continue;
        }
        // As the copy treats them: a linked file counts at its content's
        // size, a linked folder (kept as a link) counts nothing.
        let Some(meta) = as_copied(&entry, meta) else {
            continue;
        };
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
        links: Vec::new(),
        skipped: Vec::new(),
        outside: Vec::new(),
        total: 0,
        folders: Vec::new(),
    };
    collect_files(layout, &source, &source, &mut acc, max_bytes, extra_skips).map_err(dir_error)?;
    let CollectAccum {
        files,
        links,
        mut skipped,
        outside: mut left_outside,
        folders,
        ..
    } = acc;
    let mut to_scan = Vec::new();

    let source_login = login_probe(&source);

    // Marker first: even a partially imported dir is identifiable as ours.
    prepare_managed_dir_for(dest, layout.provider)?;

    let mut files_copied: u64 = 0;
    let mut bytes_copied: u64 = 0;
    for (relative, _) in &files {
        let from = source.join(relative);
        let to = dest.join(relative);
        if let Some(parent) = to.parent()
            && let Err(e) = create_private_dir(parent)
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
            let _ = restrict_to_owner(&to);
        } else {
            // AJ-E2006: the rest goes to the app's redaction scan.
            to_scan.push(to.clone());
        }
        files_copied += 1;
        bytes_copied += std::fs::metadata(&to).map(|m| m.len()).unwrap_or_default();
    }

    recreate_links(&source, dest, &links, &mut skipped, &mut left_outside);

    let (login, login_note) = login_carryover(layout, source_login, login_probe(dest));

    Ok(ImportOutcome {
        files_copied,
        bytes_copied,
        skipped,
        login,
        login_note,
        folders,
        degraded: false,
        left_outside,
        to_scan,
    })
}

/// Recreate each collected symlink in the copy as a link — never followed,
/// never copied through. A link into the setup is pointed at the copy.
fn recreate_links(
    source: &Path,
    dest: &Path,
    links: &[(PathBuf, PathBuf)],
    skipped: &mut Vec<String>,
    left_outside: &mut Vec<String>,
) {
    let resolved_source = std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    for (relative, target) in links {
        let to = dest.join(relative);
        // Where the link really points, from its own folder, with `..`
        // collapsed.
        let link_dir = source.join(relative);
        let link_dir = link_dir.parent().unwrap_or(source);
        let absolute = lexical_normal(&if target.is_absolute() {
            target.clone()
        } else {
            link_dir.join(target)
        });
        let resolved = std::fs::canonicalize(&absolute).unwrap_or_else(|_| absolute.clone());
        let inside = absolute
            .strip_prefix(source)
            .or_else(|_| absolute.strip_prefix(&resolved_source))
            .or_else(|_| resolved.strip_prefix(&resolved_source))
            .ok()
            .map(Path::to_path_buf);
        // Inside the setup: point at the same place in the copy (a
        // relative link keeps working as it is). Outside: the real place,
        // absolute, so it still resolves from the copy.
        // AJ-E2006: a link out of the folder isn't made in the copy; the
        // user is told where it pointed and can link it again.
        let retargeted = match inside {
            Some(_) if !target.is_absolute() => target.clone(),
            Some(inside) => dest.join(inside),
            None => {
                left_outside.push(format!("{}/ → {}", relative.display(), resolved.display()));
                continue;
            }
        };
        if let Some(parent) = to.parent() {
            let _ = create_private_dir(parent);
        }
        // A link that can't be made (no permission for links, on Windows)
        // leaves that folder out rather than failing the whole import.
        if let Err(e) = make_link(&retargeted, &to) {
            skipped.push(format!(
                "{}/ (a linked folder — left out: {e})",
                relative.display()
            ));
            continue;
        }
        skipped.push(format!(
            "{} (kept as a link to {})",
            relative.display(),
            retargeted.display()
        ));
    }
}

/// An entry as the copy sees it: a plain entry as is; a link to a FILE
/// as its target (copied by content); a link to a folder or a broken
/// link as nothing (kept as a link, or left out) — so sizes and the
/// inspection agree with what the import does.
fn as_copied(entry: &std::fs::DirEntry, meta: std::fs::Metadata) -> Option<std::fs::Metadata> {
    if !meta.file_type().is_symlink() {
        return Some(meta);
    }
    std::fs::metadata(entry.path())
        .ok()
        .filter(std::fs::Metadata::is_file)
}

/// AJ-E2006: `link → where it really points` when the link at `path`
/// resolves outside `root`.
fn outside_link(path: &Path, root: &Path, relative: &Path) -> Option<String> {
    let target = std::fs::canonicalize(path).ok()?;
    (!target.starts_with(root)).then(|| format!("{} → {}", relative.display(), target.display()))
}

/// `path` with `.` and `..` collapsed, without touching the disk.
fn lexical_normal(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// [`adopt_claude_config`] for any Provider's layout (Codex too): the
/// folder is checked against the layout's evidence and AI Juggler's
/// marker, then probed for its sign-in — read-only.
///
/// # Errors
///
/// [`juggler_adapter::AdapterError::AccountDir`] with a plain reason.
pub(crate) fn adopt_with_layout(
    layout: &Layout,
    source: &Path,
    login_probe: &dyn Fn(&Path) -> LoginProbe,
) -> Result<ImportOutcome, juggler_adapter::AdapterError> {
    let dir_error = |message: &str| juggler_adapter::AdapterError::AccountDir {
        path: source.to_path_buf(),
        message: message.to_owned(),
    };
    if !source.is_dir() {
        return Err(dir_error("that folder isn't there any more — rescan"));
    }
    let canonical = source
        .canonicalize()
        .map_err(|error| dir_error(&error.to_string()))?;
    if !layout.is_config_dir(&canonical) {
        return Err(dir_error(layout.not_a_config));
    }
    if is_managed_dir(&canonical) {
        return Err(dir_error(
            "this is one of AI Juggler's own copies — it's already in AI Juggler",
        ));
    }
    Ok(ImportOutcome {
        files_copied: 0,
        bytes_copied: 0,
        skipped: Vec::new(),
        login: login_probe(source),
        login_note: None,
        folders: Vec::new(),
        degraded: false,
        left_outside: Vec::new(),
        to_scan: Vec::new(),
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
        let Some(meta) = as_copied(&entry, meta) else {
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

fn origin_rank(origin: InstallOrigin) -> u8 {
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

/// Sums FILE sizes down to the layout's `estimate_depth` (a folder's own
/// metadata size is never counted — it differs by file system), bounded
/// by [`MAX_WALK_ENTRIES`]; skips the same noise and links the import
/// skips.
fn estimate_bytes(layout: &Layout, dir: &Path) -> u64 {
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
            let top_level = depth == 0;
            if layout.skips(&name, top_level) || layout.kept_back(&name, top_level).is_some() {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let Some(meta) = as_copied(&entry, meta) else {
                continue;
            };
            if meta.is_dir() {
                if depth < layout.estimate_depth {
                    stack.push((entry.path(), depth + 1));
                }
            } else {
                total += meta.len();
            }
        }
    }
    total
}

fn newest_mtime_secs(dir: &Path) -> Option<u64> {
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
    /// Symlinks in the source, as (relative path, link target). They're
    /// recreated as links in the copy — never followed, so nothing
    /// outside the setup is copied (0.0.7 dogfood: a symlinked
    /// `skills/<name>` made the whole import fail).
    links: Vec<(PathBuf, PathBuf)>,
    skipped: Vec<String>,
    /// AJ-E2006: links that point outside the folder (never followed).
    outside: Vec<String>,
    total: u64,
    folders: Vec<(String, u64)>,
}

#[expect(
    clippy::too_many_lines,
    reason = "one bounded walk (AJ-E2006's caps and its outside-link check); split, its counters would pass through every helper"
)]
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
        let top_level = relative.components().count() == 1;
        // Kept back first (Codex's `auth.json`): never copied, linked or not.
        if let Some(why) = layout.kept_back(&name, top_level) {
            acc.skipped.push(format!("{} ({why})", relative.display()));
            continue;
        }
        // A linked FILE is copied by content, as before (the copy never
        // shares a file with the original). A linked FOLDER is kept as a
        // link — never followed or copied through (0.0.7 dogfood: a
        // `skills/` folder linked to dotfiles failed the whole import).
        let meta = if meta.file_type().is_symlink() {
            if layout.skips(&name, top_level) || extra_skips.contains(&name.as_str()) {
                continue;
            }
            // AJ-E2006: a link reaching outside the folder is never
            // followed: it is listed for the user instead.
            if let Some(line) = outside_link(&entry.path(), root, &relative) {
                acc.outside.push(line);
                continue;
            }
            match std::fs::metadata(entry.path()) {
                Ok(target) if target.is_file() => target,
                Ok(target) if target.is_dir() => {
                    if let Ok(link) = std::fs::read_link(entry.path()) {
                        acc.links.push((relative, link));
                    }
                    continue;
                }
                _ => {
                    acc.skipped
                        .push(format!("{} (a broken link — left out)", relative.display()));
                    continue;
                }
            }
        } else {
            meta
        };
        if meta.is_dir() {
            if layout.skips(&name, top_level) {
                acc.skipped
                    .push(format!("{}/ (rebuildable cache)", relative.display()));
                continue;
            }
            if extra_skips.contains(&name.as_str()) {
                // Size the skipped tree so the degraded import's skip
                // note can say how much history stayed behind.
                let mut skipped_bytes = 0u64;
                size_tree(&entry.path(), &mut skipped_bytes);
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "a size shown to one decimal of a GiB"
                )]
                let gib = skipped_bytes as f64 / 1_073_741_824.0;
                acc.skipped.push(format!(
                    "{}/ ({gib:.1} GiB history — skipped at your choice)",
                    relative.display(),
                ));
                continue;
            }
            collect_files(layout, root, &entry.path(), acc, max_bytes, extra_skips)?;
        } else if layout.skips(&name, top_level) {
            acc.skipped
                .push(format!("{} (rebuildable)", relative.display()));
        } else if relative.components().count() == 1 && extra_skips.contains(&name.as_str()) {
            // A top-level history FILE (Codex's `history.jsonl`) stays
            // behind with its size, like a history folder.
            #[allow(
                clippy::cast_precision_loss,
                reason = "a size shown to one decimal of a MiB"
            )]
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

#[cfg(unix)]
fn make_link(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn make_link(target: &Path, link: &Path) -> std::io::Result<()> {
    // Only linked folders are kept as links (files are copied).
    std::os::windows::fs::symlink_dir(target, link)
}

/// The file that marks a folder as managed by AI Juggler (AJ-E308
/// isolation row 2 reads it).
pub const MANAGED_MARKER: &str = ".ai-juggler-managed";

/// True when the directory carries the AI Juggler management marker.
#[must_use]
pub fn is_managed_dir(state_dir: &Path) -> bool {
    state_dir.join(MANAGED_MARKER).is_file()
}

/// Creates a managed account folder (0700) and writes the AI Juggler
/// marker (0600) naming the tool whose state it holds: the single
/// folder-initialization path shared by `prepare_account` and import.
///
/// # Errors
///
/// Returns [`AdapterError::AccountDir`] when the dir or marker cannot be
/// written.
pub(crate) fn prepare_managed_dir_for(
    state_dir: &Path,
    provider: &str,
) -> Result<(), AdapterError> {
    // A failure after creating the folder removes it again — only when
    // this call created it (an existing folder is never removed).
    let created_here = !state_dir.exists();
    let undo = || {
        if created_here {
            let _ = std::fs::remove_dir_all(state_dir);
        }
    };
    create_private_dir(state_dir).map_err(|error| {
        undo();
        AdapterError::AccountDir {
            path: state_dir.to_path_buf(),
            message: error.to_string(),
        }
    })?;
    // Marker identifies the directory as ours — guardrail primitive for
    // "never touch unmanaged dirs" checks (ADR-0003).
    let marker = state_dir.join(MANAGED_MARKER);
    std::fs::write(
        &marker,
        format!("managed-by: ai-juggler\nharness: {provider}\n"),
    )
    .and_then(|()| restrict_to_owner(&marker))
    .map_err(|error| {
        undo();
        AdapterError::AccountDir {
            path: state_dir.to_path_buf(),
            message: format!("cannot write marker: {error}"),
        }
    })
}

/// Creates `dir` (and its parents) readable by its owner only (0700).
pub(crate) fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::fs::DirBuilder;
        use std::os::unix::fs::DirBuilderExt;
        DirBuilder::new().recursive(true).mode(0o700).create(dir)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir)
    }
}

/// Makes `path` readable and writable by its owner only (0600).
pub(crate) fn restrict_to_owner(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}
