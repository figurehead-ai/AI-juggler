//! AJ-E1009: backup and restore of AI Juggler's own data (ADR-0016).
//!
//! **Export** writes one `.ajbackup` file (gzip tar): `manifest.json`
//! first, then a scrubbed database snapshot, `heuristics.json` and the
//! parked packs. Only those paths are ever read (an allowlist), so
//! vendor sign-ins in the Account folders and the Fleet identity in
//! `fleet/` cannot travel. The snapshot drops what only means something
//! on this Mac (process ledger, Fleet sync, confirmation tokens) and
//! blanks every Launch settings environment value, listing them as
//! withheld. The redaction engine then scans every entry, and anything
//! it finds stops the export.
//!
//! **Restore** first verifies: the archive is read in full into a
//! staging folder, every checksum must match, nothing may be missing or
//! extra, and the format and schema must be readable by this build.
//! Applying the restore needs the verify step's one-time token and no
//! live Instance. The current database and settings are kept beside the
//! restored ones, and this Mac's Fleet identity is never touched.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use juggler_core::model::Timestamp;
use juggler_core::storage::Storage;
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The manifest's `format` value.
pub const FORMAT: &str = "ai-juggler-backup";
/// The archive layout this build writes and reads.
pub const FORMAT_VERSION: u32 = 1;
const STAGING_DIR: &str = "restore-staging";
const TOKEN_KEY: &str = "restore_token";
/// App settings that only mean something on the Mac that wrote them:
/// never in a backup, and on restore this Mac keeps its own. Exact keys…
const MACHINE_ONLY_SETTINGS: &[&str] = &[
    "history_tokens",
    "restore_token",
    "machine_label",
    "usage_ingest_state",
    "juggler_worktrees",
    "vault_code_saved",
    "update_check_on_launch",
];
/// …and whole families: this Mac's Figurehead link and cloud choices,
/// its Fleet state, its install telemetry, its Cloud backup schedule, its
/// usage-stats and crash-report choices (a restore never opts another
/// Mac in).
const MACHINE_ONLY_PREFIXES: &[&str] = &[
    "figurehead_",
    "fleet_",
    "funnel_",
    "cloud_backup_",
    "usage_stats",
    "crash_reports",
];

/// The SQL condition matching a machine-only setting key.
fn machine_only_where(column: &str) -> String {
    let exact = MACHINE_ONLY_SETTINGS
        .iter()
        .map(|key| format!("'{key}'"))
        .collect::<Vec<_>>()
        .join(",");
    let families = MACHINE_ONLY_PREFIXES
        .iter()
        .fold(String::new(), |mut out, prefix| {
            use std::fmt::Write as _;
            let _ = write!(out, " OR {column} LIKE '{prefix}%'");
            out
        });
    format!("({column} IN ({exact}){families})")
}
/// Tables whose rows only mean something on the Mac that wrote them.
const MACHINE_ONLY_TABLES: &[&str] = &[
    "process_ledger",
    "fleet_outbox",
    "fleet_replica",
    "fleet_replica_hw",
];

fn io_error(context: &'static str, error: &impl std::fmt::Display) -> AppError {
    AppError::Io {
        context,
        message: error.to_string(),
    }
}

fn validation(subject: &'static str, message: impl Into<String>) -> AppError {
    AppError::Validation {
        subject,
        message: message.into(),
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// One file in the archive.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BackupFileInfo {
    /// Its path inside the archive.
    pub path: String,
    /// Its size in bytes.
    #[ts(type = "number")]
    pub bytes: u64,
    /// Its sha256.
    pub sha256: String,
}

/// Where a backup came from.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BackupOriginInfo {
    /// The writing Mac's Fleet fingerprint (to recognise it, not to be it).
    pub fingerprint: String,
    /// Its host name.
    pub label: String,
}

/// What a backup holds (ADR-0016).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BackupManifestInfo {
    /// Always [`FORMAT`].
    pub format: String,
    /// The archive layout.
    pub format_version: u32,
    /// The database schema of the snapshot.
    #[ts(type = "number")]
    pub schema_version: i64,
    /// The app version that wrote it.
    pub app_version: String,
    /// When, epoch ms.
    #[ts(type = "number")]
    pub created_at_ms: i64,
    /// The writing Mac.
    pub origin: BackupOriginInfo,
    /// Record counts (the restore is checked against them).
    #[ts(type = "Record<string, number>")]
    pub counts: BTreeMap<String, u64>,
    /// Every other entry, with its checksum.
    pub files: Vec<BackupFileInfo>,
    /// Left out on purpose; re-supply after a restore.
    pub withheld: Vec<String>,
    /// Never in a backup, and why.
    pub excluded: Vec<String>,
}

/// What an export wrote.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BackupExportInfo {
    /// The archive.
    pub path: String,
    /// Its size.
    #[ts(type = "number")]
    pub bytes: u64,
    /// Its manifest.
    pub manifest: BackupManifestInfo,
}

/// The verified restore plan.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RestorePlanInfo {
    /// The archive.
    pub path: String,
    /// Its manifest (every checksum verified).
    pub manifest: BackupManifestInfo,
    /// It came from a different Mac: it carries that Mac's data, never
    /// its identity.
    pub other_machine: bool,
    /// Plain-language notes the user should read before restoring.
    pub notes: Vec<String>,
    /// The one-time token that applies this restore.
    pub token: String,
}

/// What a restore did.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RestoreResultInfo {
    /// Counts after the restore (they match the manifest).
    #[ts(type = "Record<string, number>")]
    pub counts: BTreeMap<String, u64>,
    /// Where the data from before the restore was kept.
    pub kept_before: Vec<String>,
    /// What to do next, in plain words.
    pub next_steps: Vec<String>,
}

/// Progress callback: `(bytes done, bytes total)`.
pub type Progress<'a> = &'a mut dyn FnMut(u64, u64);

fn counts(storage: &Storage) -> AppResult<BTreeMap<String, u64>> {
    let mut out = BTreeMap::new();
    for (name, sql) in [
        ("accounts", "SELECT COUNT(*) FROM accounts"),
        ("launch_settings", "SELECT COUNT(*) FROM launch_recipes"),
        ("projects", "SELECT COUNT(*) FROM projects"),
        ("instances", "SELECT COUNT(*) FROM instances"),
        (
            "history_events",
            "SELECT (SELECT COUNT(*) FROM instance_transitions) + (SELECT COUNT(*) FROM instance_lifecycle)",
        ),
        ("switch_edges", "SELECT COUNT(*) FROM switch_edges"),
        ("epochs", "SELECT COUNT(*) FROM epochs"),
    ] {
        let n: i64 = storage
            .raw_conn()
            .query_row(sql, [], |row| row.get(0))
            .map_err(juggler_core::CoreError::from)?;
        out.insert(name.to_owned(), n.unsigned_abs());
    }
    Ok(out)
}

/// Removes what only means something on this Mac and blanks environment
/// values; returns what was withheld.
fn scrub(snapshot: &Path) -> AppResult<Vec<String>> {
    let storage = Storage::open(snapshot)?;
    let conn = storage.raw_conn();
    let core = juggler_core::CoreError::from;
    for table in MACHINE_ONLY_TABLES {
        conn.execute(&format!("DELETE FROM {table}"), [])
            .map_err(core)?;
    }
    conn.execute(
        &format!(
            "DELETE FROM app_settings WHERE {}",
            machine_only_where("key")
        ),
        [],
    )
    .map_err(core)?;
    for sql in [
        "DELETE FROM transcript_fts",
        "DELETE FROM transcript_lines",
        "DELETE FROM transcript_index_state",
    ] {
        conn.execute(sql, []).map_err(core)?;
    }
    let mut withheld = Vec::new();
    for account in storage.accounts().list()? {
        for mut recipe in storage.profiles().list_by_account(&account.id)? {
            if recipe.settings_overlay.env.is_empty() {
                continue;
            }
            let names: Vec<String> = recipe.settings_overlay.env.keys().cloned().collect();
            for value in recipe.settings_overlay.env.values_mut() {
                value.clear();
            }
            storage.profiles().update(&recipe)?;
            withheld.push(format!(
                "Environment values of the Launch settings \u{201c}{}\u{201d} ({}): {}",
                recipe.name,
                account.label,
                names.join(", ")
            ));
        }
    }
    // Fold the WAL back in and compact, so the file alone is the
    // snapshot and no deleted row survives in a free page.
    storage
        .raw_conn()
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode = DELETE; VACUUM;")
        .map_err(core)?;
    drop(storage);
    Ok(withheld)
}

/// Every allowlisted source file: `(archive path, file on disk)`.
fn sources(data_dir: &Path, snapshot: &Path) -> Vec<(String, PathBuf)> {
    let mut out = vec![("db/juggler.sqlite".to_owned(), snapshot.to_path_buf())];
    let heuristics = data_dir.join("heuristics.json");
    if heuristics.is_file() {
        out.push(("settings/heuristics.json".to_owned(), heuristics));
    }
    let parked = data_dir.join("parked");
    let mut stack = vec![parked.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(std::fs::DirEntry::path);
        for entry in entries {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file()
                && let Ok(relative) = path.strip_prefix(&parked)
            {
                let relative = relative
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push((format!("parked/{relative}"), path));
            }
        }
    }
    out
}

fn sha256_file(path: &Path, progress: &mut dyn FnMut(u64)) -> AppResult<(u64, String)> {
    let mut file = std::fs::File::open(path).map_err(|e| io_error("backup read", &e))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    let mut total = 0u64;
    loop {
        let read = file
            .read(&mut buf)
            .map_err(|e| io_error("backup read", &e))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
        total += read as u64;
        progress(read as u64);
    }
    Ok((total, hex(&hasher.finalize())))
}

/// The redaction engine over one entry: masked findings, if any.
fn scan(path: &Path) -> AppResult<Vec<String>> {
    let bytes = std::fs::read(path).map_err(|e| io_error("backup scan", &e))?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(juggler_handover::redaction::list_segments(&[text.as_ref()])
        .into_iter()
        .map(|found| format!("{} ({})", found.preview, found.kind))
        .collect())
}

fn origin(state: &Arc<AppState>) -> BackupOriginInfo {
    BackupOriginInfo {
        fingerprint: crate::fleet::this_machine_fingerprint(state).unwrap_or_default(),
        label: sysinfo::System::host_name().unwrap_or_else(|| "this Mac".to_owned()),
    }
}

/// Core: writes a backup of AI Juggler's own data to `dest`.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when something that looks like a
/// secret is found (named, masked) or `dest` exists, or the typed
/// storage or IO error.
pub fn backup_export_impl(
    state: &Arc<AppState>,
    dest: &Path,
    progress: Progress<'_>,
) -> AppResult<BackupExportInfo> {
    if dest.exists() {
        return Err(validation(
            "backup",
            format!("{} already exists — choose a new file name", dest.display()),
        ));
    }
    let work = state
        .data_dir
        .join(format!("backup-work-{}", Timestamp::now().millis()));
    std::fs::create_dir_all(&work).map_err(|e| io_error("backup staging", &e))?;
    let result = export_from(state, dest, &work, progress);
    let _ = std::fs::remove_dir_all(&work);
    result
}

/// Every file an export opens comes from `sources` (scan, hash, write)
/// or is the live database it snapshotted: recorded for the AJ-E1207
/// credential guardrail.
fn record_reads(state: &Arc<AppState>, sources: &[(String, PathBuf)]) {
    if let Ok(mut reads) = state.export_reads.lock() {
        *reads = std::iter::once(state.data_dir.join("juggler.sqlite"))
            .chain(sources.iter().map(|(_, path)| path.clone()))
            .collect();
    }
}

fn export_from(
    state: &Arc<AppState>,
    dest: &Path,
    work: &Path,
    progress: Progress<'_>,
) -> AppResult<BackupExportInfo> {
    let snapshot = work.join("juggler.sqlite");
    crate::commands::lock_storage_arc(state)?.snapshot_to(&snapshot)?;
    let withheld = scrub(&snapshot)?;
    let counts = counts(&Storage::open(&snapshot)?)?;

    let sources = sources(&state.data_dir, &snapshot);
    record_reads(state, &sources);
    let mut findings = Vec::new();
    for (name, path) in &sources {
        for found in scan(path)? {
            findings.push(format!("{name}: {found}"));
        }
    }
    if !findings.is_empty() {
        return Err(validation(
            "backup",
            format!(
                "stopped — something that looks like a secret is in {}; nothing was written",
                findings.join("; ")
            ),
        ));
    }

    let total: u64 = sources
        .iter()
        .map(|(_, path)| std::fs::metadata(path).map_or(0, |m| m.len()))
        .sum::<u64>()
        * 2; // hashed once, then written
    let mut done = 0u64;
    let mut files = Vec::new();
    for (name, path) in &sources {
        let (bytes, sha256) = sha256_file(path, &mut |n| {
            done += n;
            progress(done, total);
        })?;
        files.push(BackupFileInfo {
            path: name.clone(),
            bytes,
            sha256,
        });
    }
    let manifest = BackupManifestInfo {
        format: FORMAT.to_owned(),
        format_version: FORMAT_VERSION,
        schema_version: Storage::schema_version(),
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        created_at_ms: Timestamp::now().millis(),
        origin: origin(state),
        counts,
        files,
        withheld,
        excluded: vec![
            "Sign-ins to Claude, Codex and other Providers (the Account folders) — sign in again after a restore".to_owned(),
            "This Mac's Fleet identity and paired machines — a restored Mac keeps its own and pairs again".to_owned(),
            "Process records — their process ids belong to the Mac that wrote them".to_owned(),
            "The Providers' own transcripts — they stay where each Provider keeps them".to_owned(),
        ],
    };

    let partial = dest.with_extension("ajbackup.partial");
    let file = std::fs::File::create(&partial).map_err(|e| io_error("backup write", &e))?;
    let mut archive = tar::Builder::new(flate2::write::GzEncoder::new(
        file,
        flate2::Compression::fast(),
    ));
    let manifest_json =
        serde_json::to_vec_pretty(&manifest).map_err(|e| io_error("backup manifest", &e))?;
    append_bytes(&mut archive, "manifest.json", &manifest_json)?;
    for (name, path) in &sources {
        let mut source = std::fs::File::open(path).map_err(|e| io_error("backup read", &e))?;
        let len = source
            .metadata()
            .map_err(|e| io_error("backup read", &e))?
            .len();
        let mut header = tar::Header::new_gnu();
        header.set_size(len);
        header.set_mode(0o600);
        header.set_mtime(0);
        header.set_cksum();
        let mut counted = CountingReader {
            inner: &mut source,
            on_read: &mut |n| {
                done += n;
                progress(done, total);
            },
        };
        archive
            .append_data(&mut header, name, &mut counted)
            .map_err(|e| io_error("backup write", &e))?;
    }
    let encoder = archive
        .into_inner()
        .map_err(|e| io_error("backup write", &e))?;
    let mut file = encoder.finish().map_err(|e| io_error("backup write", &e))?;
    file.flush().map_err(|e| io_error("backup write", &e))?;
    drop(file);
    std::fs::rename(&partial, dest).map_err(|e| io_error("backup write", &e))?;
    progress(total, total);
    Ok(BackupExportInfo {
        path: dest.display().to_string(),
        bytes: std::fs::metadata(dest).map_or(0, |m| m.len()),
        manifest,
    })
}

fn append_bytes<W: Write>(
    archive: &mut tar::Builder<W>,
    name: &str,
    bytes: &[u8],
) -> AppResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o600);
    header.set_mtime(0);
    header.set_cksum();
    archive
        .append_data(&mut header, name, bytes)
        .map_err(|e| io_error("backup write", &e))
}

struct CountingReader<'a, R: Read> {
    inner: &'a mut R,
    on_read: &'a mut dyn FnMut(u64),
}

impl<R: Read> Read for CountingReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        (self.on_read)(read as u64);
        Ok(read)
    }
}

/// A safe relative path inside the staging folder, or `None`.
fn staged_path(staging: &Path, name: &str) -> Option<PathBuf> {
    let relative = Path::new(name);
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None; // absolute, `..` or `.` — never written
    }
    let allowed = name == "db/juggler.sqlite"
        || name == "settings/heuristics.json"
        || name.starts_with("parked/");
    allowed.then(|| staging.join(relative))
}

fn refuse(path: &Path, reason: impl std::fmt::Display) -> AppError {
    validation(
        "restore",
        format!("{} can't be restored: {reason}", path.display()),
    )
}

/// Staged entries: path → (bytes, sha256).
type Staged = BTreeMap<String, (u64, String)>;

/// Reads the whole archive into `staging`: its manifest (checked for
/// format and schema) and every entry's size and sha256.
fn stage_entries(path: &Path, staging: &Path) -> AppResult<(BackupManifestInfo, Staged)> {
    let file = std::fs::File::open(path).map_err(|e| refuse(path, e))?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let mut manifest: Option<BackupManifestInfo> = None;
    let mut seen: BTreeMap<String, (u64, String)> = BTreeMap::new();
    for entry in archive.entries().map_err(|e| refuse(path, e))? {
        let mut entry = entry.map_err(|e| refuse(path, format!("damaged archive ({e})")))?;
        if !entry.header().entry_type().is_file() {
            return Err(refuse(path, "it holds something other than plain files"));
        }
        let name = entry
            .path()
            .map_err(|e| refuse(path, e))?
            .to_string_lossy()
            .into_owned();
        if manifest.is_none() {
            if name != "manifest.json" {
                return Err(refuse(path, "it is not an AI Juggler backup (no manifest)"));
            }
            let mut raw = Vec::new();
            entry.read_to_end(&mut raw).map_err(|e| refuse(path, e))?;
            let parsed: BackupManifestInfo = serde_json::from_slice(&raw)
                .map_err(|e| refuse(path, format!("its manifest can't be read ({e})")))?;
            if parsed.format != FORMAT {
                return Err(refuse(path, "it is not an AI Juggler backup"));
            }
            if parsed.format_version != FORMAT_VERSION {
                return Err(refuse(
                    path,
                    format!(
                        "it uses backup format {} and this build reads format {FORMAT_VERSION} — update AI Juggler",
                        parsed.format_version
                    ),
                ));
            }
            if parsed.schema_version > Storage::schema_version() {
                return Err(refuse(
                    path,
                    format!(
                        "it was made by a newer AI Juggler (database schema {} > {}) — update first",
                        parsed.schema_version,
                        Storage::schema_version()
                    ),
                ));
            }
            manifest = Some(parsed);
            continue;
        }
        let target = staged_path(staging, &name)
            .ok_or_else(|| refuse(path, format!("unexpected entry {name}")))?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| io_error("restore staging", &e))?;
        }
        let mut out =
            std::fs::File::create(&target).map_err(|e| io_error("restore staging", &e))?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 1024 * 1024];
        let mut bytes = 0u64;
        loop {
            let read = entry
                .read(&mut buf)
                .map_err(|e| refuse(path, format!("damaged archive ({e})")))?;
            if read == 0 {
                break;
            }
            hasher.update(&buf[..read]);
            out.write_all(&buf[..read])
                .map_err(|e| io_error("restore staging", &e))?;
            bytes += read as u64;
        }
        seen.insert(name, (bytes, hex(&hasher.finalize())));
    }
    let manifest = manifest.ok_or_else(|| refuse(path, "it is empty"))?;
    Ok((manifest, seen))
}

/// Core: verifies `path` in full into the staging folder and returns the
/// restore plan with its one-time token. Nothing about this Mac's data
/// changes.
///
/// # Errors
///
/// Returns [`AppError::Validation`] naming the file and the reason for a
/// damaged, incomplete, foreign or too-new archive.
pub fn backup_verify_impl(state: &Arc<AppState>, path: &Path) -> AppResult<RestorePlanInfo> {
    let staging = state.data_dir.join(STAGING_DIR);
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| io_error("restore staging", &e))?;
    let (manifest, mut seen) = stage_entries(path, &staging)?;
    for file in &manifest.files {
        match seen.remove(&file.path) {
            None => return Err(refuse(path, format!("{} is missing", file.path))),
            Some((bytes, sha)) if bytes != file.bytes || sha != file.sha256 => {
                return Err(refuse(
                    path,
                    format!(
                        "{} does not match its checksum — the file is damaged",
                        file.path
                    ),
                ));
            }
            Some(_) => {}
        }
    }
    if let Some(extra) = seen.keys().next() {
        return Err(refuse(
            path,
            format!("{extra} is not listed in its manifest"),
        ));
    }
    let here = origin(state);
    let other_machine = manifest.origin.fingerprint != here.fingerprint;
    let mut notes = vec![
        "Restoring replaces AI Juggler's data on this Mac. What is here now is kept beside it, so it can be put back by hand.".to_owned(),
        "Sign in to each Account again afterwards — sign-ins are never in a backup.".to_owned(),
    ];
    if other_machine {
        notes.insert(
            0,
            format!(
                "This backup comes from {}. It carries that Mac's data, never its identity: this Mac keeps its own and must pair again with your other Macs.",
                manifest.origin.label
            ),
        );
    }
    if !manifest.withheld.is_empty() {
        notes.push(format!(
            "Re-enter what was withheld: {}.",
            manifest.withheld.join("; ")
        ));
    }
    let token = {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|e| io_error("restore token", &e))?;
        hex(&bytes)
    };
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(TOKEN_KEY, &format!("{token}:{}", path.display()))?;
    Ok(RestorePlanInfo {
        path: path.display().to_string(),
        manifest,
        other_machine,
        notes,
        token,
    })
}

/// Core: applies the verified restore.
///
/// # Errors
///
/// Returns [`AppError::Validation`] for a wrong token, a live Instance,
/// or counts that do not match the manifest; or the typed error.
pub fn backup_restore_impl(
    state: &Arc<AppState>,
    path: &Path,
    token: &str,
) -> AppResult<RestoreResultInfo> {
    let staging = state.data_dir.join(STAGING_DIR);
    let expected = format!("{token}:{}", path.display());
    {
        let storage = crate::commands::lock_storage_arc(state)?;
        let stored = storage.app_settings().get(TOKEN_KEY)?;
        storage.app_settings().set(TOKEN_KEY, "")?;
        if stored.as_deref() != Some(expected.as_str()) {
            return Err(validation(
                "restore",
                "verify the backup again first — the confirmation does not match",
            ));
        }
        if storage
            .sessions()
            .list(None)?
            .iter()
            .any(|record| !crate::commands::is_terminal(record.status))
        {
            return Err(validation(
                "restore",
                "stop every running Instance first — a restore replaces their records",
            ));
        }
    }
    let manifest: BackupManifestInfo = {
        let file = std::fs::File::open(path).map_err(|e| refuse(path, e))?;
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
        let mut first = archive
            .entries()
            .map_err(|e| refuse(path, e))?
            .next()
            .ok_or_else(|| refuse(path, "it is empty"))?
            .map_err(|e| refuse(path, e))?;
        let mut raw = Vec::new();
        first.read_to_end(&mut raw).map_err(|e| refuse(path, e))?;
        serde_json::from_slice(&raw).map_err(|e| refuse(path, e))?
    };
    let stamp = Timestamp::now().millis();
    let data = &state.data_dir;
    let mut kept_before = Vec::new();
    let before_db = data.join(format!("juggler.sqlite.before-restore-{stamp}"));
    crate::commands::lock_storage_arc(state)?.snapshot_to(&before_db)?;
    kept_before.push(before_db.display().to_string());
    for (name, live) in [
        ("settings/heuristics.json", data.join("heuristics.json")),
        ("parked", data.join("parked")),
    ] {
        if live.exists() {
            let aside = data.join(format!(
                "{}.before-restore-{stamp}",
                live.file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
            ));
            std::fs::rename(&live, &aside).map_err(|e| io_error("restore", &e))?;
            kept_before.push(aside.display().to_string());
        }
        let staged = staging.join(name);
        if staged.exists() {
            std::fs::rename(&staged, &live).map_err(|e| io_error("restore", &e))?;
        }
    }
    let counts = {
        let mut storage = crate::commands::lock_storage_arc(state)?;
        storage.restore_from(&staging.join("db/juggler.sqlite"))?;
        keep_this_macs_rows(&storage, &before_db)?;
        let verified = counts(&storage)?;
        // Instances that were live on the writing Mac are not live here
        // (each one's stop is journalled, after the counts are checked).
        for mut record in storage.sessions().list(None)? {
            if !crate::commands::is_terminal(record.status) {
                record.status = juggler_core::model::SessionStatus::Stopped;
                storage.sessions().update(&record)?;
            }
        }
        verified
    };
    let _ = std::fs::remove_dir_all(&staging);
    for (name, expected) in &manifest.counts {
        if counts.get(name) != Some(expected) {
            return Err(validation(
                "restore",
                format!(
                    "restored {name} = {} but the backup lists {expected} — the data from before is at {}",
                    counts.get(name).copied().unwrap_or(0),
                    before_db.display()
                ),
            ));
        }
    }
    let mut next_steps = vec![
        "Restart AI Juggler to finish.".to_owned(),
        "Sign in to each Account again.".to_owned(),
    ];
    if !manifest.withheld.is_empty() {
        next_steps.push("Re-enter the withheld environment values in Launch settings.".to_owned());
    }
    Ok(RestoreResultInfo {
        counts,
        kept_before,
        next_steps,
    })
}

/// Puts back what belongs to this Mac, not to the backup: its process
/// ledger, its Fleet sync state and its machine-only settings (the
/// backup carries none of them).
fn keep_this_macs_rows(storage: &Storage, before: &Path) -> AppResult<()> {
    use std::fmt::Write as _;
    let conn = storage.raw_conn();
    let core = juggler_core::CoreError::from;
    conn.execute(
        "ATTACH DATABASE ?1 AS prior",
        [before.to_string_lossy().as_ref()],
    )
    .map_err(core)?;
    let mut sql = String::new();
    for table in MACHINE_ONLY_TABLES {
        let _ = write!(
            sql,
            "DELETE FROM main.{table}; INSERT INTO main.{table} SELECT * FROM prior.{table};"
        );
    }
    // Whatever an older backup carried for these is dropped; this Mac's
    // own values come back.
    let _ = write!(
        sql,
        "DELETE FROM main.app_settings WHERE {cond};
         INSERT OR REPLACE INTO main.app_settings SELECT * FROM prior.app_settings WHERE {cond};",
        cond = machine_only_where("key")
    );
    let copied = conn.execute_batch(&sql).map_err(core);
    conn.execute("DETACH DATABASE prior", []).map_err(core)?;
    copied?;
    Ok(())
}

/// One progress update on the `backup://progress` event.
#[derive(Clone, serde::Serialize)]
struct ProgressEvent {
    done: u64,
    total: u64,
}

/// AJ-E1009: exports a backup to `dest`, emitting `backup://progress`.
///
/// # Errors
///
/// Per [`backup_export_impl`].
#[tauri::command]
pub async fn backup_export(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    dest: String,
) -> AppResult<BackupExportInfo> {
    use tauri::Emitter as _;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut last = 0u64;
        backup_export_impl(&state, Path::new(&dest), &mut |done, total| {
            // At least every 1 %: never a silent wait.
            if total == 0 || done == total || (done - last) * 100 >= total {
                last = done;
                let _ = app.emit("backup://progress", ProgressEvent { done, total });
            }
        })
    })
    .await
    .map_err(|e| io_error("backup", &e))?
}

/// AJ-E1009: verifies a backup and returns the restore plan.
///
/// # Errors
///
/// Per [`backup_verify_impl`].
#[tauri::command]
pub async fn backup_verify(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> AppResult<RestorePlanInfo> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || backup_verify_impl(&state, Path::new(&path)))
        .await
        .map_err(|e| io_error("restore", &e))?
}

/// AJ-E1009: applies a verified restore.
///
/// # Errors
///
/// Per [`backup_restore_impl`].
#[tauri::command]
pub async fn backup_restore(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
    token: String,
) -> AppResult<RestoreResultInfo> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        backup_restore_impl(&state, Path::new(&path), &token)
    })
    .await
    .map_err(|e| io_error("restore", &e))?
}
