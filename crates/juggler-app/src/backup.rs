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

use crate::events::{AppEvent, EmitEvent as _};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_core::backup_archive::{self, hex, stage_entries};
// The archive format and its reader live in juggler-core (the fuzz
// target reads through them without the app); the app's API keeps them.
pub use juggler_core::backup_archive::{
    BackupFileInfo, BackupManifestInfo, BackupOriginInfo, FORMAT, FORMAT_VERSION,
};
pub(crate) use juggler_core::backup_archive::{MAX_FILE_BYTES, MAX_FILES, MAX_TOTAL_BYTES};
use juggler_core::model::Timestamp;
use juggler_core::storage::Storage;
use juggler_core::storage::memory::backup::Destination as MemoryDestination;
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

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
/// values; returns what was withheld. The memory store's rows go by
/// D-133's backup rule for `memory` (ADR-0037 §1.8).
fn scrub(snapshot: &Path, memory: MemoryDestination) -> AppResult<Vec<String>> {
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
    juggler_core::storage::memory::backup::scrub_snapshot(conn, memory)?;
    let mut withheld = Vec::new();
    // Every account's, removed ones too (AJ-E1501): their launch
    // settings are still in the database.
    for account in storage.accounts().list_all()? {
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
        label: sysinfo::System::host_name().unwrap_or_else(|| "this computer".to_owned()),
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
    backup_write(state, dest, progress, Purpose::LeavesTheMac)
}

/// The backup written before an update (and kept for Go back to an
/// earlier version): it stays in AI Juggler's own folder on this Mac, as
/// the database it copies does, so the check that stops a secret leaving
/// the Mac in an export doesn't apply. A look-alike in a project's name
/// stopped every update (0.1.1, 2026-10-25); an update never waits on it.
///
/// # Errors
///
/// As [`backup_export_impl`], without the secret check.
pub fn backup_before_update(
    state: &Arc<AppState>,
    dest: &Path,
    progress: Progress<'_>,
) -> AppResult<BackupExportInfo> {
    backup_write(state, dest, progress, Purpose::StaysOnThisMac)
}

/// The backup for the encrypted cloud copy: as [`backup_export_impl`],
/// but of Chat's memory only the watches and preferences the user stated
/// (ADR-0037 §1.8, D-133): Chat's history, remembered answers, notes and
/// inferred memories stay in this Mac's own backups.
///
/// # Errors
///
/// As [`backup_export_impl`].
pub fn backup_for_cloud(
    state: &Arc<AppState>,
    dest: &Path,
    progress: Progress<'_>,
) -> AppResult<BackupExportInfo> {
    backup_write(state, dest, progress, Purpose::Cloud)
}

/// Where a backup goes, which decides whether a secret look-alike stops it
/// and which of Chat's memory goes with it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Purpose {
    /// An export: anything secret-looking stops it.
    LeavesTheMac,
    /// The cloud copy: anything secret-looking stops it, and Chat's words
    /// stay behind.
    Cloud,
    /// The copy before an update, in AI Juggler's own folder.
    StaysOnThisMac,
}

impl Purpose {
    /// The memory store's destination class (ADR-0037 §1.8).
    const fn memory(self) -> MemoryDestination {
        match self {
            Self::Cloud => MemoryDestination::Cloud,
            Self::LeavesTheMac => MemoryDestination::Local,
            // A copy AI Juggler keeps on its own: a later forget couldn't
            // reach it, so it holds no memory (ADR-0037 §1.8).
            Self::StaysOnThisMac => MemoryDestination::OwnCopy,
        }
    }
}

fn backup_write(
    state: &Arc<AppState>,
    dest: &Path,
    progress: Progress<'_>,
    purpose: Purpose,
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
    let result = export_from(state, dest, &work, progress, purpose);
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

/// A backup that leaves the Mac stops on anything that looks like a
/// secret; the one before an update stays here and isn't checked.
fn secret_check(sources: &[(String, PathBuf)], purpose: Purpose) -> AppResult<()> {
    if purpose == Purpose::StaysOnThisMac {
        return Ok(());
    }
    let mut findings = Vec::new();
    for (name, path) in sources {
        for found in scan(path)? {
            findings.push(format!("{name}: {found}"));
        }
    }
    if findings.is_empty() {
        return Ok(());
    }
    Err(validation(
        "backup",
        format!(
            "stopped — something that looks like a secret is in {}; nothing was written",
            findings.join("; ")
        ),
    ))
}

fn export_from(
    state: &Arc<AppState>,
    dest: &Path,
    work: &Path,
    progress: Progress<'_>,
    purpose: Purpose,
) -> AppResult<BackupExportInfo> {
    let snapshot = work.join("juggler.sqlite");
    crate::commands::lock_storage_arc(state)?.snapshot_to(&snapshot)?;
    let withheld = scrub(&snapshot, purpose.memory())?;
    let counts = counts(&Storage::open(&snapshot)?)?;

    let sources = sources(&state.data_dir, &snapshot);
    record_reads(state, &sources);
    secret_check(&sources, purpose)?;

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
            "Sign-ins to Claude, Codex and other tools (the account folders) — sign in again after a restore".to_owned(),
            "This computer's Fleet identity and paired machines — a restored computer keeps its own and pairs again".to_owned(),
            "Process records — their process ids belong to the computer that wrote them".to_owned(),
            "Each tool's own transcripts — they stay where the tool keeps them".to_owned(),
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

fn refuse(path: &Path, reason: impl std::fmt::Display) -> AppError {
    backup_archive::refuse(path, reason).into()
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
    // AJ-E2006: its database checked before anything opens it for real.
    juggler_core::storage::foreign::vet(
        &staging.join("db/juggler.sqlite"),
        manifest.schema_version,
    )
    .map_err(|e| refuse(path, e))?;
    let here = origin(state);
    let other_machine = manifest.origin.fingerprint != here.fingerprint;
    let mut notes = vec![
        "Restoring replaces AI Juggler's data on this computer. What is here now is kept beside it, so it can be put back by hand.".to_owned(),
        "Sign in to each Account again afterwards — sign-ins are never in a backup.".to_owned(),
    ];
    if other_machine {
        notes.insert(
            0,
            format!(
                "This backup comes from {}. It carries that computer's data, never its identity: this computer keeps its own and must pair again with your other computers.",
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
    // The staged backup goes on every exit, a failed one too.
    let _staging = RemoveOnDrop(staging.clone());
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
            .any(|record| !crate::features::sessions::read_model::is_terminal(record.status))
        {
            return Err(validation(
                "restore",
                "stop every running session first — a restore replaces their records",
            ));
        }
    }
    let manifest = read_manifest(path)?;
    let stamp = Timestamp::now().millis();
    let data = &state.data_dir;
    let mut kept_before = Vec::new();
    let before_db = data.join(format!("{RESTORE_COPY_PREFIX}{stamp}"));
    let staged_db = staging.join("db/juggler.sqlite");
    prune_aged_copies(data, stamp);
    let counts = {
        // One hold from the copy to the end (the re-review's N5): a forget
        // asked for meanwhile waits, then applies to the restored data.
        let mut storage = crate::commands::lock_storage_arc(state)?;
        storage.snapshot_to(&before_db)?;
        // However this ends, the copy keeps no memory (ADR-0037 §1.5),
        // scrubbed before the hold ends; a crash leaves it to the sweep at
        // start ([`sweep_kept_copies`]).
        let _kept_copy = ScrubOnDrop(before_db.clone());
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
        // What this Mac forgot is dropped from the staged copy before it
        // replaces anything (and again after, as a check).
        apply_tombstones(&staged_db, &before_db)?;
        storage.restore_from(&staged_db)?;
        keep_this_macs_rows(&storage, &before_db)?;
        let verified = counts(&storage)?;
        // Instances that were live on the writing Mac are not live here
        // (each one's stop is journalled, after the counts are checked).
        for mut record in storage.sessions().list(None)? {
            if !crate::features::sessions::read_model::is_terminal(record.status) {
                record.status = juggler_core::model::SessionStatus::Stopped;
                storage.sessions().update(&record)?;
            }
        }
        verified
    };
    for (name, expected) in &manifest.counts {
        if counts.get(name) != Some(expected) {
            // Kept under its own name: the way back isn't pruned.
            let kept = data.join(format!("juggler.sqlite.before-failed-restore-{stamp}"));
            let kept = std::fs::rename(&before_db, &kept).map_or(before_db, |()| kept);
            return Err(validation(
                "restore",
                format!(
                    "restored {name} = {} but the backup lists {expected} — the data from before is at {}",
                    counts.get(name).copied().unwrap_or(0),
                    kept.display()
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

/// Reads only a backup's manifest (its first entry) — what it holds,
/// which version and schema wrote it — without checking the rest.
///
/// # Errors
///
/// Returns [`AppError::Validation`] naming the file when it can't be read
/// or has no manifest first.
pub fn read_manifest(path: &Path) -> AppResult<BackupManifestInfo> {
    Ok(backup_archive::read_manifest(path)?)
}

/// Puts back what belongs to this Mac, not to the backup: its process
/// ledger, its Fleet sync state, its machine-only settings and its memory
/// forget tombstones (the backup carries none of them).
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
    // What this Mac forgot stays forgotten (ADR-0037 §1.5): its
    // tombstones are kept and drop those subjects from what came back.
    let copied = conn.execute_batch(&sql).map_err(core).and_then(|()| {
        juggler_core::storage::memory::backup::after_restore(conn, "prior").map(drop)
    });
    conn.execute("DETACH DATABASE prior", []).map_err(core)?;
    copied?;
    Ok(())
}

/// Drops the backup a verify staged and its confirmation (the second
/// re-check's L5): a forget calls this under the same storage hold, so a
/// staged backup made before the forget never outlives it; the user
/// verifies again to restore.
///
/// # Errors
///
/// The typed storage error clearing the confirmation.
pub(crate) fn drop_staged(storage: &Storage, data_dir: &Path) -> AppResult<()> {
    storage.app_settings().set(TOKEN_KEY, "")?;
    let staging = data_dir.join(STAGING_DIR);
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| io_error("forget", &e))?;
    }
    Ok(())
}

/// The name a database copy kept before a restore starts with.
const RESTORE_COPY_PREFIX: &str = "juggler.sqlite.before-restore-";
/// How long a database copy kept before a restore is kept: a day to go
/// back if the restore was the wrong one. It holds no memory from the
/// moment the restore ends, so this bounds its other data only.
pub const RESTORE_COPY_KEEP_MS: i64 = 24 * 3_600_000;

/// Removes a folder however the function holding it ends.
struct RemoveOnDrop(PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Empties the memory from a database copy however the restore ends.
struct ScrubOnDrop(PathBuf);

impl Drop for ScrubOnDrop {
    fn drop(&mut self) {
        if let Err(error) = juggler_core::storage::memory::backup::scrub_copy(&self.0) {
            tracing::warn!(%error, "the copy kept before a restore couldn't be scrubbed");
        }
    }
}

/// Drops from the staged database every subject this Mac forgot (the
/// tombstones in `before`, the live database's copy), before the staged
/// database replaces the live one (the security review's M1).
fn apply_tombstones(staged: &Path, before: &Path) -> AppResult<()> {
    let storage = Storage::open(staged)?; // migrates it, as the restore would
    let conn = storage.raw_conn();
    let core = juggler_core::CoreError::from;
    conn.execute(
        "ATTACH DATABASE ?1 AS prior",
        [before.to_string_lossy().as_ref()],
    )
    .map_err(core)?;
    let applied = juggler_core::storage::memory::backup::after_restore(conn, "prior");
    conn.execute("DETACH DATABASE prior", []).map_err(core)?;
    applied?;
    Ok(())
}

/// The folders a backup or a restore works in, and the name a database
/// copy kept before a restore starts with: a crash leaves them behind.
const WORK_DIRS: [&str; 3] = ["backup-work-", "cloud-backup-work-", STAGING_DIR];
/// The database copies AI Juggler keeps beside the live one (restores):
/// `(name prefix, how long it is kept)`; `None` is never pruned.
const KEPT_COPIES: [(&str, Option<i64>); 5] = [
    (RESTORE_COPY_PREFIX, Some(RESTORE_COPY_KEEP_MS)),
    ("juggler.sqlite.before-failed-restore-", None),
    // Going back (`newer_data`): the data as it was, the newer data an
    // older build moved aside, and a copy a failed going-back tried. Kept
    // (the user's way forward), emptied of memory only (ADR-0037 §1.5).
    ("juggler.sqlite.from-v", None),
    ("juggler.sqlite.from-newer-", None),
    ("juggler.sqlite.unused-", None),
];
const DB_SUFFIXES: [&str; 3] = ["-wal", "-shm", "-journal"];

/// The database file a name belongs to (its `-wal`, `-shm` or `-journal`
/// mapped to it).
fn base_name(name: &str) -> &str {
    DB_SUFFIXES
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .unwrap_or(name)
}

/// At start (the re-review's N3 and L5): what a crash or a quit left
/// behind goes. The work folders of a backup or a restore (an unscrubbed
/// snapshot, a staged backup) are removed; every database copy AI Juggler
/// keeps is emptied of memory ([`juggler_core::storage::memory::backup::scrub_copy`]),
/// then removed once past its age.
pub fn sweep_kept_copies(data_dir: &Path, now_ms: i64) {
    let Ok(entries) = std::fs::read_dir(data_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if path.is_dir() && WORK_DIRS.iter().any(|dir| name.starts_with(dir)) {
            let _ = std::fs::remove_dir_all(&path);
        } else if base_name(&name) == name
            && KEPT_COPIES
                .iter()
                .any(|(prefix, _)| name.starts_with(prefix))
            && let Err(error) = juggler_core::storage::memory::backup::scrub_copy(&path)
        {
            tracing::warn!(%error, "a kept copy couldn't be scrubbed");
        }
    }
    prune_aged_copies(data_dir, now_ms);
}

/// Removes each kept database copy once it is older than its keep (by the
/// stamp its name ends with); a failed restore's copy stays.
pub fn prune_aged_copies(data_dir: &Path, now_ms: i64) {
    let Ok(entries) = std::fs::read_dir(data_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let base = base_name(&name);
        let Some(keep) = KEPT_COPIES
            .iter()
            .find_map(|(prefix, keep)| base.starts_with(prefix).then_some(*keep))
            .flatten()
        else {
            continue;
        };
        let stamp = base.rsplit('-').next().and_then(|s| s.parse::<i64>().ok());
        if stamp.is_some_and(|stamp| now_ms.saturating_sub(stamp) > keep) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
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
    use crate::picked::{PickKind, picked_as, vouch};

    let dest = vouch(&dest, picked_as(&[PickKind::BackupDest]), |_| false)?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut last = 0u64;
        backup_export_impl(&state, &dest, &mut |done, total| {
            // At least every 1 %: never a silent wait.
            if total == 0 || done == total || (done - last) * 100 >= total {
                last = done;
                app.emit_event(AppEvent::BackupProgress, ProgressEvent { done, total });
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
    use crate::picked::{PickKind, picked_as, vouch};
    let path = vouch(&path, picked_as(&[PickKind::BackupFile]), |_| false)?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || backup_verify_impl(&state, &path))
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
    use crate::picked::{PickKind, picked_as, vouch};
    let path = vouch(&path, picked_as(&[PickKind::BackupFile]), |_| false)?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || backup_restore_impl(&state, &path, &token))
        .await
        .map_err(|e| io_error("restore", &e))?
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a test: a failed step fails the test")]

    use super::*;
    use juggler_core::storage::memory::{self, Actor};

    const CANARY: &str = "qzxwvrestorecanary";

    /// A note holding [`CANARY`], written as the store writes it.
    fn canary_note(storage: &Storage) {
        storage
            .raw_conn()
            .execute(
                "INSERT INTO memory_records (id, subject_key, subject_id, version, kind, scope_kind,
                     tier, priority, authority, untrusted, source_kind, confidence, as_of_ms, text,
                     created_ms)
                 VALUES ('r1', 'note:gone', 'sub-gone', 1, 'note', 'global', 'long', 1, 'user', 0,
                         'settings', 1, 1, ?1, 1)",
                [format!("remember {CANARY} please")],
            )
            .unwrap();
    }

    fn holds(path: &Path) -> bool {
        ["", "-wal", "-shm"].iter().any(|suffix| {
            let mut name = path.as_os_str().to_owned();
            name.push(suffix);
            std::fs::read(PathBuf::from(name))
                .unwrap_or_default()
                .windows(CANARY.len())
                .any(|w| w == CANARY.as_bytes())
        })
    }

    /// ADR-0037 §1.5, the security review's H3 and M1: the staged backup
    /// loses what this Mac forgot before it replaces anything; the copy
    /// kept before the restore keeps no memory however the restore ends,
    /// and goes after a day.
    #[test]
    fn a_restore_drops_the_forgotten_and_keeps_no_copy_holding_it() {
        let dir = tempfile::tempdir().unwrap();
        let live_path = dir.path().join("juggler.sqlite");
        let live = Storage::open(&live_path).unwrap();
        live.raw_conn().execute_batch(memory::V54).unwrap();
        canary_note(&live);
        // A backup made before the forget, staged for a restore.
        let staged = dir.path().join("staged.sqlite");
        live.snapshot_to(&staged).unwrap();
        // The copy kept before a restore holds what is remembered now.
        let stamp = 10 * 86_400_000;
        let before = dir.path().join(format!("{RESTORE_COPY_PREFIX}{stamp}"));
        live.snapshot_to(&before).unwrap();
        assert!(
            holds(&before),
            "the kept copy holds the words before its scrub"
        );
        live.memory().forget("note:gone", &Actor::User, 2).unwrap();
        let after_forget = dir.path().join("after-forget.sqlite");
        live.snapshot_to(&after_forget).unwrap();
        assert!(holds(&staged));

        apply_tombstones(&staged, &after_forget).unwrap();
        assert!(!holds(&staged), "the staged backup still holds it");
        let restored = Storage::open(&staged).unwrap();
        assert!(restored.memory().current("note:gone").unwrap().is_none());
        drop(restored);

        // However the restore ends, the kept copy keeps no memory.
        drop(ScrubOnDrop(before.clone()));
        assert!(!holds(&before), "the kept copy still holds the words");
        // And it goes after a day; a failed restore's copy stays.
        let failed = dir
            .path()
            .join(format!("juggler.sqlite.before-failed-restore-{stamp}"));
        std::fs::write(&failed, b"kept").unwrap();
        prune_aged_copies(dir.path(), stamp + RESTORE_COPY_KEEP_MS / 2);
        assert!(before.exists());
        prune_aged_copies(dir.path(), stamp + RESTORE_COPY_KEEP_MS + 1);
        assert!(!before.exists() && failed.exists());

        // The staging folder goes on every exit.
        let staging = dir.path().join("staging");
        std::fs::create_dir_all(staging.join("db")).unwrap();
        drop(RemoveOnDrop(staging.clone()));
        assert!(!staging.exists());
    }

    /// The re-review's N3 and L5: a crash between taking the copy and its
    /// scrub, or mid-backup, leaves nothing unscrubbed past the next start.
    #[test]
    fn the_sweep_at_start_leaves_no_unscrubbed_copy_or_work_folder() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let live = Storage::open(&data.join("juggler.sqlite")).unwrap();
        live.raw_conn().execute_batch(memory::V54).unwrap();
        canary_note(&live);
        let now = 100 * 86_400_000;
        // A crashed restore's copy, unscrubbed; a crashed backup's and a
        // verified-but-never-confirmed restore's folders.
        let crashed = data.join(format!("{RESTORE_COPY_PREFIX}{now}"));
        live.snapshot_to(&crashed).unwrap();
        let failed = data.join(format!(
            "juggler.sqlite.before-failed-restore-{}",
            now - 90 * 86_400_000
        ));
        live.snapshot_to(&failed).unwrap();
        for work in [
            format!("backup-work-{now}"),
            format!("cloud-backup-work-{now}"),
            STAGING_DIR.to_owned(),
        ] {
            std::fs::create_dir_all(data.join(&work)).unwrap();
            live.snapshot_to(&data.join(&work).join("juggler.sqlite"))
                .unwrap();
        }
        assert!(holds(&crashed) && holds(&failed));

        sweep_kept_copies(data, now + 1);
        assert!(
            !holds(&crashed),
            "the crashed restore's copy still holds the words"
        );
        assert!(
            !holds(&failed),
            "the failed restore's copy still holds the words"
        );
        assert!(failed.exists(), "the way back after a failed restore stays");
        for work in ["backup-work-", "cloud-backup-work-", STAGING_DIR] {
            assert!(
                !std::fs::read_dir(data)
                    .unwrap()
                    .flatten()
                    .any(|e| e.file_name().to_string_lossy().starts_with(work)),
                "{work} is still there"
            );
        }
        sweep_kept_copies(data, now + RESTORE_COPY_KEEP_MS + 1);
        assert!(!crashed.exists(), "a day on, the copy is gone");
    }

    /// The re-review's N2 and the second re-check's N6: the data kept aside
    /// when going back (by this build, or by an older one that knows
    /// nothing of memory) is emptied of memory at the next start, and is
    /// otherwise kept: never deleted, however old.
    #[test]
    fn the_sweep_scrubs_and_keeps_the_going_back_copies() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let live = Storage::open(&data.join("juggler.sqlite")).unwrap();
        live.raw_conn().execute_batch(memory::V54).unwrap();
        canary_note(&live);
        let now = 100 * 86_400_000;
        let copies = [
            data.join(format!("juggler.sqlite.from-v54-{now}")),
            data.join(format!("juggler.sqlite.from-newer-{now}")),
            data.join(format!("juggler.sqlite.unused-{now}")),
        ];
        for copy in &copies {
            live.snapshot_to(copy).unwrap();
            assert!(holds(copy));
        }
        sweep_kept_copies(data, now + 1);
        for copy in &copies {
            assert!(copy.exists() && !holds(copy), "{}", copy.display());
        }
        sweep_kept_copies(data, now + 3_650 * 86_400_000);
        assert!(
            copies.iter().all(|copy| copy.exists()),
            "ten years on, they're still kept"
        );
    }

    /// The re-review's N5 (its second re-check's appendix): a forget asked
    /// for while a restore holds the storage waits, then applies to the
    /// restored data; the forgotten words come back nowhere in the data
    /// folder. The forget starts only once the copy before the restore
    /// exists (inside the hold), so the order is fixed. Fails when the hold
    /// is split, or when `ScrubOnDrop` does nothing.
    #[test]
    fn a_forget_during_a_restore_applies_to_the_restored_data() {
        let dir = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let data = dir.path().to_path_buf();
        let state = Arc::new(
            AppState::init_with_registry(data.clone(), juggler_adapter::AdapterRegistry::new())
                .unwrap(),
        );
        {
            let storage = state.storage.lock().unwrap();
            storage.raw_conn().execute_batch(memory::V54).unwrap();
            canary_note(&storage);
        }
        let archive = out.path().join("before.ajbackup");
        backup_export_impl(&state, &archive, &mut |_, _| {}).unwrap();
        // Verify stages it, then refuses V54's tables until V54 is
        // registered; the confirmation is set as verify sets it.
        let _ = backup_verify_impl(&state, &archive);
        assert!(data.join(STAGING_DIR).join("db/juggler.sqlite").exists());
        state
            .storage
            .lock()
            .unwrap()
            .app_settings()
            .set(TOKEN_KEY, &format!("tok:{}", archive.display()))
            .unwrap();

        let forgetter = {
            let state = Arc::clone(&state);
            let data = data.clone();
            std::thread::spawn(move || {
                let start = std::time::Instant::now();
                while !std::fs::read_dir(&data).unwrap().flatten().any(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with(RESTORE_COPY_PREFIX)
                }) {
                    assert!(start.elapsed().as_secs() < 30, "no restore started");
                    // poll: a test waiting for the restore's copy to appear.
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                crate::commands::lock_storage_arc(&state)
                    .unwrap()
                    .memory()
                    .forget("note:gone", &Actor::User, 5)
                    .unwrap()
            })
        };
        backup_restore_impl(&state, &archive, "tok").unwrap();
        let forgot = forgetter.join().unwrap();
        assert_eq!(forgot.subjects, 1, "the forget found the restored subject");
        {
            let storage = state.storage.lock().unwrap();
            assert!(storage.memory().current("note:gone").unwrap().is_none());
            storage
                .raw_conn()
                .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
                .unwrap();
        }
        let mut stack = vec![data.clone()];
        let mut holding = Vec::new();
        while let Some(at) = stack.pop() {
            for entry in std::fs::read_dir(&at).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if std::fs::read(&path)
                    .unwrap_or_default()
                    .windows(CANARY.len())
                    .any(|w| w == CANARY.as_bytes())
                {
                    holding.push(path);
                }
            }
        }
        assert!(holding.is_empty(), "these still hold it: {holding:?}");
    }

    /// AI Juggler's own copy (before an update) holds no memory; the
    /// user's export and the cloud copy go by ADR-0037 §1.8.
    #[test]
    fn each_purpose_takes_its_memory_class() {
        assert_eq!(Purpose::StaysOnThisMac.memory(), MemoryDestination::OwnCopy);
        assert_eq!(Purpose::LeavesTheMac.memory(), MemoryDestination::Local);
        assert_eq!(Purpose::Cloud.memory(), MemoryDestination::Cloud);
    }
}
