//! AJ-E1517: Export diagnostics — one zip a bug report can carry: the
//! versions, the database's schema, the settings and the logs AI Juggler
//! keeps, each scrubbed as a crash report is (no home folder, user name,
//! links or keys) and shown before it's saved. What the user saw is what
//! is written; a last scan refuses anything that still looks like a
//! secret. Never in it: transcripts, terminal output, credentials, the
//! Account folders, or settings that hold tokens, addresses or paths.

pub mod zip;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// One file in the zip, as it will be saved.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DiagnosticsFileInfo {
    /// Its path in the zip.
    pub name: String,
    /// What it is, in a few words.
    pub about: String,
    /// Its whole text, exactly as saved.
    pub text: String,
}

/// What the zip will hold, before it's saved.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DiagnosticsPreviewInfo {
    /// Names this preview: the export saves exactly it.
    pub id: String,
    /// The files, in order.
    pub files: Vec<DiagnosticsFileInfo>,
    /// Settings left out entirely (they hold tokens, addresses or paths).
    pub left_out: Vec<String>,
    /// The suggested file name.
    pub file_name: String,
}

/// Where the zip went.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DiagnosticsSavedInfo {
    /// The file, home-relative.
    pub path: String,
    /// Its size.
    #[ts(type = "number")]
    pub bytes: u64,
}

/// Settings never in diagnostics: what only means something on this Mac
/// (as a backup leaves out) and what holds tokens, addresses or paths.
const LEFT_OUT: &[&str] = &[
    "history_tokens",
    "restore_token",
    "machine_label",
    "local_machine_id",
    "usage_ingest_state",
    "juggler_worktrees",
    "vault_code_saved",
    "identity_pins",
    "parked_instances",
    "recovery_summary",
    "redaction_overrides",
    "projects.editor",
];
/// …and whole families of them.
const LEFT_OUT_PREFIXES: &[&str] = &[
    "figurehead_",
    "fleet_",
    "cloud_backup_",
    "credits:",
    "vault",
];

/// The logs AI Juggler keeps, from its data folder: where, its name in
/// the zip, and what it is. Terminal output and transcripts aren't logs
/// and never come.
const LOGS: &[(&str, &str, &str)] = &[
    (
        "supervision/supervision.jsonl",
        "logs/supervision.jsonl",
        "Sessions started and ended",
    ),
    (
        "stats/upload.log",
        "logs/usage-stats-upload.log",
        "Usage statistics sent",
    ),
    (
        "crashes/kept.log",
        "logs/crashes-kept.log",
        "Crashes kept on this Mac",
    ),
    (
        "funnel-events.jsonl",
        "logs/funnel-events.jsonl",
        "First-run steps",
    ),
    (
        crate::error_log::RELATIVE,
        "logs/errors.log",
        "What went wrong behind the messages you saw",
    ),
    ("last-run", "logs/last-run", "How the last run ended"),
    (
        "crash-relaunches",
        "logs/crash-relaunches",
        "Reopens after a crash",
    ),
];
/// The newest part of each log that comes (whole lines).
const LOG_TAIL: usize = 256 * 1024;

/// The last preview, saved as shown.
static PREPARED: Mutex<Option<DiagnosticsPreviewInfo>> = Mutex::new(None);

/// Scrubs as a crash report is: this Mac's home folder and user name.
struct Scrub {
    home: String,
    user: String,
}

impl Scrub {
    fn here() -> Self {
        Self {
            home: dirs::home_dir()
                .map(|home| home.display().to_string())
                .unwrap_or_default(),
            user: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_default(),
        }
    }

    fn text(&self, text: &str) -> String {
        // Line by line, so a log keeps its lines.
        text.split_inclusive('\n')
            .map(|line| crate::crash::scrub(line, &self.home, &self.user))
            .collect()
    }

    /// What still looks like a secret or this Mac's home or user.
    fn leaks(&self, text: &str) -> Vec<String> {
        let mut found: Vec<String> = juggler_handover::redaction::list_segments(&[text])
            .into_iter()
            .map(|hit| hit.kind.to_string())
            .collect();
        if text
            .split_whitespace()
            .any(|word| crate::crash::secret_assignment(word).is_some())
        {
            found.push("env-value".to_owned());
        }
        if self.home.len() > 1 && text.contains(&self.home) {
            found.push("home folder".to_owned());
        }
        if self.user.len() >= 3 && text.contains(&self.user) {
            found.push("user name".to_owned());
        }
        found
    }
}

fn left_out(key: &str) -> bool {
    LEFT_OUT.contains(&key)
        || LEFT_OUT_PREFIXES
            .iter()
            .any(|prefix| key.starts_with(prefix))
}

/// The newest `LOG_TAIL` bytes of a log, from a line's start.
fn tail(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let from = bytes.len().saturating_sub(LOG_TAIL);
    let text = String::from_utf8_lossy(&bytes[from..]).into_owned();
    Some(match (from, text.find('\n')) {
        (0, _) | (_, None) => text,
        (_, Some(at)) => text[at + 1..].to_owned(),
    })
}

fn about(
    state: &Arc<AppState>,
    storage: &juggler_core::storage::Storage,
) -> AppResult<serde_json::Value> {
    let app = crate::commands::AppInfo::current();
    let tools: std::collections::BTreeMap<String, String> = state
        .provider_versions
        .lock()
        .map(|versions| {
            versions
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default();
    let sessions = storage.sessions().list(None)?;
    let mut by_status = std::collections::BTreeMap::<&str, usize>::new();
    for session in &sessions {
        *by_status.entry(session.status.as_str()).or_default() += 1;
    }
    let accounts = storage.accounts().list()?;
    let launch_settings: usize = accounts
        .iter()
        .map(|account| {
            storage
                .profiles()
                .list_by_account(&account.id)
                .map_or(0, |list| list.len())
        })
        .sum();
    Ok(serde_json::json!({
        "app": { "name": app.name, "version": app.version, "edition": app.edition,
                 "channel": crate::updates::current_channel().as_str() },
        "system": { "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
                    "os_version": sysinfo::System::os_version() },
        "database": { "schema_this_build": juggler_core::storage::Storage::schema_version(),
                      "schema_on_disk": storage.user_version()? },
        "tools": tools,
        "counts": { "accounts": accounts.len(), "launch_settings": launch_settings,
                    "projects": storage.projects().list()?.len(),
                    "sessions": sessions.len(), "sessions_by_status": by_status },
    }))
}

/// Core: what the zip will hold. Every file is scrubbed; the export
/// saves exactly this.
///
/// # Errors
///
/// Returns the typed storage error.
pub fn preview_impl(state: &Arc<AppState>) -> AppResult<DiagnosticsPreviewInfo> {
    let scrub = Scrub::here();
    let mut files = Vec::new();
    let mut left = Vec::new();
    let mut reads = vec![state.data_dir.join("juggler.sqlite")];
    {
        let storage = crate::commands::lock_storage_arc(state)?;
        let about = serde_json::to_string_pretty(&about(state, &storage)?).unwrap_or_default();
        files.push(file(
            "about.json",
            "Versions, the database's schema and counts",
            scrub.text(&about),
        ));
        let mut settings = serde_json::Map::new();
        for (key, value) in storage.app_settings().list()? {
            // Names too: a key can carry an id or a name.
            if left_out(&key) {
                left.push(scrub.text(&key));
            } else {
                settings.insert(
                    scrub.text(&key),
                    serde_json::Value::String(scrub.text(&value)),
                );
            }
        }
        let settings = serde_json::to_string_pretty(&serde_json::json!({
            "settings": settings,
            "left_out": left,
        }))
        .unwrap_or_default();
        files.push(file(
            "app-settings.json",
            "Your settings, without what's left out",
            settings,
        ));
    }
    for (path, name, what) in LOGS {
        let path = state.data_dir.join(path);
        if let Some(text) = tail(&path) {
            files.push(file(name, what, scrub.text(&text)));
            reads.push(path);
        }
    }
    for folder in ["asking", "queued"] {
        let dir = state.data_dir.join("crashes").join(folder);
        let mut reports: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map(|read| {
                read.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|x| x == "json"))
                    .collect()
            })
            .unwrap_or_default();
        reports.sort();
        for path in reports {
            if let (Some(text), Some(stem)) = (tail(&path), path.file_stem()) {
                let name = format!("crashes/{}.json", stem.to_string_lossy());
                files.push(file(
                    &name,
                    "A crash report, as it would be sent",
                    scrub.text(&text),
                ));
                reads.push(path);
            }
        }
    }
    if let Ok(mut recorded) = state.export_reads.lock() {
        *recorded = reads;
    }
    let mut id = [0u8; 8];
    let _ = getrandom::fill(&mut id);
    let preview = DiagnosticsPreviewInfo {
        id: crate::crash::hex(&id),
        files,
        left_out: left,
        file_name: format!("AI Juggler diagnostics {}.zip", today()),
    };
    if let Ok(mut prepared) = PREPARED.lock() {
        *prepared = Some(preview.clone());
    }
    Ok(preview)
}

fn file(name: &str, about: &str, text: String) -> DiagnosticsFileInfo {
    DiagnosticsFileInfo {
        name: name.to_owned(),
        about: about.to_owned(),
        text,
    }
}

fn today() -> String {
    let days = crate::commands::now_millis().div_euclid(86_400_000);
    let (y, m, d) = zip::civil_day(days);
    format!("{y}-{m:02}-{d:02}")
}

/// Core: saves the preview `id` to `dest` (a `.zip`), exactly as shown.
///
/// # Errors
///
/// [`AppError::Validation`] when the preview is gone (shown again first),
/// `dest` isn't a `.zip`, or a file still looks like it holds a secret
/// (nothing is written); the IO error otherwise.
pub fn export_impl(id: &str, dest: &Path) -> AppResult<DiagnosticsSavedInfo> {
    let refuse = |subject: &'static str, message: String| AppError::Validation { subject, message };
    if dest
        .extension()
        .is_none_or(|ext| !ext.eq_ignore_ascii_case("zip"))
    {
        return Err(refuse("diagnostics", "save it as a .zip file".to_owned()));
    }
    let preview = PREPARED
        .lock()
        .ok()
        .and_then(|prepared| prepared.clone())
        .filter(|prepared| prepared.id == id)
        .ok_or_else(|| {
            refuse(
                "diagnostics",
                "that preview is gone: show it again, then save".to_owned(),
            )
        })?;
    let scrub = Scrub::here();
    for file in &preview.files {
        let leaks = scrub.leaks(&file.text);
        if !leaks.is_empty() {
            return Err(refuse(
                "diagnostics",
                format!(
                    "{} still looks like it holds {}: nothing was saved",
                    file.name,
                    leaks.join(", ")
                ),
            ));
        }
    }
    let entries: Vec<zip::Entry<'_>> = preview
        .files
        .iter()
        .map(|file| zip::Entry {
            name: &file.name,
            data: file.text.as_bytes(),
        })
        .collect();
    let bytes = zip::write(&entries, crate::commands::now_millis())
        .map_err(|e| io("diagnostics zip", &e))?;
    let partial = dest.with_extension("zip.partial");
    std::fs::write(&partial, &bytes).map_err(|e| io("saving diagnostics", &e))?;
    std::fs::rename(&partial, dest).map_err(|e| {
        let _ = std::fs::remove_file(&partial);
        io("saving diagnostics", &e)
    })?;
    Ok(DiagnosticsSavedInfo {
        path: crate::features::quota::capture::home_relative(dest),
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
    })
}

fn io(context: &'static str, error: &std::io::Error) -> AppError {
    AppError::Io {
        context,
        message: error.to_string(),
    }
}

/// What a diagnostics zip will hold, scrubbed, to look over first.
///
/// # Errors
///
/// As [`preview_impl`].
#[tauri::command]
pub async fn app_diagnostics_preview(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<DiagnosticsPreviewInfo> {
    crate::ipc::blocking(state.inner(), "diagnostics", preview_impl).await
}

/// Saves the diagnostics zip shown in preview `id` to `dest`.
///
/// # Errors
///
/// As [`export_impl`].
#[tauri::command]
pub async fn app_diagnostics_export(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    dest: String,
) -> AppResult<DiagnosticsSavedInfo> {
    crate::ipc::blocking(state.inner(), "diagnostics", move |_| {
        export_impl(&id, Path::new(&dest))
    })
    .await
}

/// Where a bug is reported: the repository's bug form.
pub const REPORT_BUG_URL: &str =
    "https://github.com/figurehead-ai/AI-juggler/issues/new?template=bug_report.yml";

/// Opens the bug form in the browser.
///
/// # Errors
///
/// When the system can't open the link.
#[tauri::command]
pub async fn app_report_bug_open(app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt as _;
    app.opener()
        .open_url(REPORT_BUG_URL, None::<&str>)
        .map_err(|e| AppError::Io {
            context: "bug report page",
            message: e.to_string(),
        })
}

#[cfg(test)]
mod tests;
