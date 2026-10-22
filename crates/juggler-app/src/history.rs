//! AJ-E1007: an Instance's lifecycle — archive, unarchive, a confirmed
//! delete, transcript search, read-only replay, and per-Account
//! retention with storage accounting.
//!
//! - **Archive and unarchive** are lifecycle events (the V26 journal). The
//!   Instance's data is never touched, so unarchiving restores it exactly.
//! - **Delete** is separate and irreversible, and it takes two calls: the
//!   preview names every record it removes and everything it keeps, then
//!   returns a one-time token that the delete must present. It removes
//!   only AI Juggler's own records. The Provider's transcript file, the
//!   process and cleanup audit, and usage history all stay. An Instance
//!   that is still live, or part of a switch history, is refused.
//! - **Retention** is a per-Account policy (keep forever by default).
//!   It never runs by itself: a dry run lists what it would delete, and
//!   applying it needs that dry run's token.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_core::model::SessionId;
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// One-time confirmation tokens, by what they confirm.
const TOKENS_KEY: &str = "history_tokens";
/// Per-Account retention, account id → days (absent = keep forever).
const RETENTION_KEY: &str = "retention_days";
/// A token is good for this long after its preview.
const TOKEN_TTL_MS: i64 = 10 * 60 * 1000;
/// Replay returns at most this much transcript text.
const REPLAY_CAP_BYTES: u64 = 16 * 1024 * 1024;
/// Transcript lines read per indexing step (bounded memory).
const INDEX_CHUNK_BYTES: u64 = 8 * 1024 * 1024;

fn validation(subject: &'static str, message: impl Into<String>) -> AppError {
    AppError::Validation {
        subject,
        message: message.into(),
    }
}

fn io_error(context: &'static str, error: &impl std::fmt::Display) -> AppError {
    AppError::Io {
        context,
        message: error.to_string(),
    }
}

// --- archive / unarchive ---------------------------------------------------

/// Core: archives an ended Instance (hidden from active views; nothing
/// deleted). Returns when.
///
/// # Errors
///
/// Returns [`AppError::Validation`] for a live Instance, or the typed
/// storage error for an unknown one.
pub fn instance_archive_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<i64> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let id = SessionId::new(session_id);
    let record = storage.sessions().get(&id)?;
    if !crate::commands::is_terminal(record.status) {
        return Err(validation(
            "archive",
            "only a session that has ended can be archived — stop it first",
        ));
    }
    Ok(storage.sessions().record_lifecycle(&id, "archived")?)
}

/// Core: unarchives an Instance — back exactly as it was.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when it is not archived, or the typed
/// storage error for an unknown one.
pub fn instance_unarchive_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<i64> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let id = SessionId::new(session_id);
    storage.sessions().get(&id)?;
    let latest = storage.sessions().lifecycle_latest()?;
    if latest.get(session_id).map(|(event, _)| event.as_str()) != Some("archived") {
        return Err(validation("unarchive", "this session is not archived"));
    }
    Ok(storage.sessions().record_lifecycle(&id, "unarchived")?)
}

// --- delete ----------------------------------------------------------------

/// What a delete would do, shown before it can happen.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DeletePreviewInfo {
    /// The Instance.
    pub session_id: String,
    /// Its Account (for the dialog's title).
    pub account_label: String,
    /// Why it cannot be deleted, when it cannot.
    pub refused: Option<String>,
    /// Exactly what is removed, in plain words.
    pub removes: Vec<String>,
    /// What stays, in plain words.
    pub keeps: Vec<String>,
    /// The one-time token the delete must present (absent when refused).
    pub token: Option<String>,
}

/// Lowercase hex of `bytes`.
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

fn new_token() -> AppResult<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| io_error("confirmation token", &e))?;
    Ok(hex(&bytes))
}

fn load_tokens(storage: &juggler_core::storage::Storage) -> BTreeMap<String, (String, i64)> {
    storage
        .app_settings()
        .get(TOKENS_KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn issue_token(storage: &juggler_core::storage::Storage, subject: &str) -> AppResult<String> {
    let token = new_token()?;
    let now = juggler_core::model::Timestamp::now().millis();
    let mut tokens = load_tokens(storage);
    tokens.retain(|_, (_, at)| now - *at < TOKEN_TTL_MS);
    tokens.insert(subject.to_owned(), (token.clone(), now));
    let raw = serde_json::to_string(&tokens).map_err(|e| io_error("confirmation token", &e))?;
    storage.app_settings().set(TOKENS_KEY, &raw)?;
    Ok(token)
}

/// Spends the token for `subject`: it must match and be fresh. One use.
fn spend_token(
    storage: &juggler_core::storage::Storage,
    subject: &str,
    token: &str,
) -> AppResult<()> {
    let now = juggler_core::model::Timestamp::now().millis();
    let mut tokens = load_tokens(storage);
    let valid = tokens
        .get(subject)
        .is_some_and(|(expected, at)| expected == token && now - *at < TOKEN_TTL_MS);
    tokens.remove(subject);
    let raw = serde_json::to_string(&tokens).map_err(|e| io_error("confirmation token", &e))?;
    storage.app_settings().set(TOKENS_KEY, &raw)?;
    if valid {
        Ok(())
    } else {
        Err(validation(
            "delete",
            "the confirmation has expired or does not match — preview it again",
        ))
    }
}

fn count(storage: &juggler_core::storage::Storage, sql: &str, id: &str) -> AppResult<i64> {
    Ok(storage
        .raw_conn()
        .query_row(sql, [id], |row| row.get(0))
        .map_err(juggler_core::CoreError::from)?)
}

/// Why `session` cannot be deleted, if it cannot.
fn delete_refusal(
    storage: &juggler_core::storage::Storage,
    record: &juggler_core::model::InstanceRecord,
) -> AppResult<Option<String>> {
    if !crate::commands::is_terminal(record.status) {
        return Ok(Some(
            "It is still live — stop it first. Only an ended session can be deleted.".to_owned(),
        ));
    }
    let lineage = count(
        storage,
        "SELECT COUNT(*) FROM epochs WHERE session_ref = ?1",
        record.id.as_str(),
    )?;
    if lineage > 0 {
        return Ok(Some(
            "It is part of a switch history — deleting it would break that lineage. Archive it instead."
                .to_owned(),
        ));
    }
    Ok(None)
}

fn plural(n: i64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Core: what deleting `session_id` would remove and keep, with the
/// one-time token (absent when refused).
///
/// # Errors
///
/// Returns the typed storage error for an unknown Instance.
pub fn instance_delete_preview_impl(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<DeletePreviewInfo> {
    let account_label = crate::commands::instances_shared_snapshot(state)?
        .into_iter()
        .find(|row| row.id == session_id)
        .map(|row| row.account_label)
        .unwrap_or_default();
    let storage = crate::commands::lock_storage_arc(state)?;
    let id = SessionId::new(session_id);
    let record = storage.sessions().get(&id)?;
    let refused = delete_refusal(&storage, &record)?;
    let events = count(
        &storage,
        "SELECT (SELECT COUNT(*) FROM instance_transitions WHERE session_id = ?1)
              + (SELECT COUNT(*) FROM instance_lifecycle WHERE session_id = ?1)",
        session_id,
    )?;
    let lines = storage.transcript_index().line_count(&id)?;
    let mut removes = vec![
        "Its record in AI Juggler (account, project, status, model, usage numbers)".to_owned(),
        format!("Its history: {}", plural(events, "event", "events")),
        format!(
            "Its search index: {}",
            plural(i64::try_from(lines).unwrap_or(i64::MAX), "line", "lines")
        ),
    ];
    let checks = count(
        &storage,
        "SELECT COUNT(*) FROM identity_assertions WHERE session_id = ?1",
        session_id,
    )?;
    if checks > 0 {
        removes.push(format!(
            "Its account checks: {}",
            plural(checks, "record", "records")
        ));
    }
    let mut keeps = Vec::new();
    if let Some(transcript) = &record.transcript_ref {
        keeps.push(format!(
            "The Provider's own transcript file — AI Juggler never deletes Provider data: {}",
            transcript.path.display()
        ));
    }
    keeps.push("The process and cleanup audit (the record of what ran and was stopped)".to_owned());
    keeps.push("Its usage in your usage totals".to_owned());
    let token = if refused.is_none() {
        Some(issue_token(&storage, &format!("delete:{session_id}"))?)
    } else {
        None
    };
    Ok(DeletePreviewInfo {
        session_id: session_id.to_owned(),
        account_label,
        refused,
        removes,
        keeps,
        token,
    })
}

/// Removes AI Juggler's records for one Instance (caller checked and
/// confirmed).
fn delete_records(storage: &juggler_core::storage::Storage, id: &SessionId) -> AppResult<()> {
    storage.transcript_index().forget(id)?;
    let conn = storage.raw_conn();
    let tx = conn
        .unchecked_transaction()
        .map_err(juggler_core::CoreError::from)?;
    for sql in [
        "DELETE FROM instance_transitions WHERE session_id = ?1",
        "DELETE FROM instance_lifecycle WHERE session_id = ?1",
        "DELETE FROM identity_assertions WHERE session_id = ?1",
        "DELETE FROM reset_queue WHERE session = ?1",
        "DELETE FROM instances WHERE id = ?1",
    ] {
        tx.execute(sql, [id.as_str()])
            .map_err(juggler_core::CoreError::from)?;
    }
    tx.commit().map_err(juggler_core::CoreError::from)?;
    Ok(())
}

/// Core: deletes an Instance's AI Juggler records — only with the token
/// its preview issued.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when refused (live, in a switch
/// history) or the token is wrong or expired.
pub fn instance_delete_impl(state: &Arc<AppState>, session_id: &str, token: &str) -> AppResult<()> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let id = SessionId::new(session_id);
    let record = storage.sessions().get(&id)?;
    spend_token(&storage, &format!("delete:{session_id}"), token)?;
    if let Some(reason) = delete_refusal(&storage, &record)? {
        return Err(validation("delete", reason));
    }
    delete_records(&storage, &id)
}

// --- transcript search -----------------------------------------------------

/// One transcript hit.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TranscriptHitInfo {
    /// The Instance.
    pub session_id: String,
    /// The matching text around the first match.
    pub snippet: String,
}

/// What one indexing pass did.
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct IndexReportInfo {
    /// Sessions whose transcript had new text.
    pub sessions: u32,
    /// Lines added.
    pub lines: u32,
}

/// Indexed lines `(offset, len, text)` and the offset consumed up to.
type LineBatch = (Vec<(u64, u64, String)>, u64);

/// Reads complete lines of `path` from `start` (at most one chunk):
/// `(offset, len, text)` for lines with searchable text, and the offset
/// up to which the file was consumed.
fn read_lines(path: &Path, start: u64, size: u64) -> std::io::Result<LineBatch> {
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(start))?;
    let take = (size - start).min(INDEX_CHUNK_BYTES);
    let mut buf = Vec::new();
    file.take(take).read_to_end(&mut buf)?;
    // Only complete lines; a partial last line waits for its newline.
    let Some(end) = buf.iter().rposition(|byte| *byte == b'\n') else {
        // One line longer than a chunk: index the chunk as a whole.
        return Ok(if take == INDEX_CHUNK_BYTES {
            let text = juggler_core::storage::line_text(&String::from_utf8_lossy(&buf));
            (vec![(start, take, text)], start + take)
        } else {
            (Vec::new(), start)
        });
    };
    let mut out = Vec::new();
    let mut offset = 0usize;
    for line in buf[..end].split(|byte| *byte == b'\n') {
        let text = juggler_core::storage::line_text(&String::from_utf8_lossy(line));
        if !text.trim().is_empty() {
            out.push((start + offset as u64, line.len() as u64, text));
        }
        offset += line.len() + 1;
    }
    Ok((out, start + end as u64 + 1))
}

/// Core: one incremental indexing pass over every Instance's transcript.
/// Files are read outside the storage lock; each session's new lines are
/// written in one transaction.
///
/// # Errors
///
/// Returns the typed storage error on SQLite failures (an unreadable
/// transcript is skipped, not an error).
pub fn index_transcripts(state: &Arc<AppState>) -> AppResult<IndexReportInfo> {
    // One pass at a time: the background pass and a search never index
    // the same transcript twice.
    let _pass = state
        .indexing
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (targets, states) = {
        let storage = crate::commands::lock_storage_arc(state)?;
        let targets: Vec<(SessionId, PathBuf)> = storage
            .sessions()
            .list(None)?
            .into_iter()
            .filter_map(|record| record.transcript_ref.map(|tr| (record.id, tr.path)))
            .collect();
        (targets, storage.transcript_index().states()?)
    };
    let mut report = IndexReportInfo::default();
    for (id, path) in targets {
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        let size = meta.len();
        let path_text = path.to_string_lossy().into_owned();
        let mut start = match states.get(id.as_str()) {
            Some(done) if done.path == path_text && done.indexed_bytes <= size => {
                done.indexed_bytes
            }
            Some(_) => {
                // Moved or rewritten: start again.
                crate::commands::lock_storage_arc(state)?
                    .transcript_index()
                    .forget(&id)?;
                0
            }
            None => 0,
        };
        let mut touched = false;
        while start < size {
            let Ok((lines, next)) = read_lines(&path, start, size) else {
                break;
            };
            if next == start {
                break; // a partial line; wait for the rest
            }
            report.lines += u32::try_from(lines.len()).unwrap_or(u32::MAX);
            touched |= !lines.is_empty();
            crate::commands::lock_storage_arc(state)?
                .transcript_index()
                .append(&id, &path_text, &lines, next)?;
            start = next;
        }
        if touched {
            report.sessions += 1;
        }
    }
    Ok(report)
}

/// The text around the first query word in a transcript line.
fn snippet(path: &Path, offset: u64, len: u64, query: &str) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = Vec::new();
    file.take(len.min(1024 * 1024)).read_to_end(&mut buf).ok()?;
    let text = juggler_core::storage::line_text(&String::from_utf8_lossy(&buf));
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = text.to_lowercase();
    let at = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .filter_map(|word| lower.find(&word.to_lowercase()))
        .min()
        .unwrap_or(0);
    let chars: Vec<char> = text.chars().collect();
    let at_char = text[..at.min(text.len())].chars().count();
    let from = at_char.saturating_sub(60);
    let to = (at_char + 120).min(chars.len());
    let mut out: String = chars[from..to].iter().collect();
    if from > 0 {
        out.insert(0, '…');
    }
    if to < chars.len() {
        out.push('…');
    }
    Some(out)
}

/// Core: transcript text search (the index is brought up to date first).
///
/// # Errors
///
/// Returns the typed storage error on SQLite failures.
pub fn transcript_search_impl(
    state: &Arc<AppState>,
    query: &str,
) -> AppResult<Vec<TranscriptHitInfo>> {
    // Bring the index up to date unless the background pass is already
    // doing it; then search what is indexed now.
    if state.indexing.try_lock().is_ok() {
        index_transcripts(state)?;
    }
    let (hits, paths) = {
        let storage = crate::commands::lock_storage_arc(state)?;
        let hits = storage.transcript_index().search(query, 100)?;
        let paths: BTreeMap<String, String> = storage
            .transcript_index()
            .states()?
            .into_iter()
            .map(|(id, done)| (id, done.path))
            .collect();
        (hits, paths)
    };
    Ok(hits
        .into_iter()
        .filter_map(|hit| {
            let path = paths.get(&hit.session)?;
            Some(TranscriptHitInfo {
                snippet: snippet(Path::new(path), hit.offset, hit.len, query)?,
                session_id: hit.session,
            })
        })
        .collect())
}

// --- replay ----------------------------------------------------------------

/// One transcript message, for display.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ReplayEntryInfo {
    /// `user` | `assistant` | `system` | the line's own type.
    pub role: String,
    /// Its text.
    pub text: String,
}

/// One lifecycle step in the replay's history.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct HistoryEventInfo {
    /// `status` (a status change) or `lifecycle` (reviewed / archived …).
    pub kind: String,
    /// The new status or the lifecycle step.
    pub what: String,
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// One switch the Instance took part in.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LineageStepInfo {
    /// `in` (work arrived here) or `out` (work left from here).
    pub direction: String,
    /// The other Account.
    pub other_account: String,
    /// The other Provider.
    pub other_provider: String,
    /// The edge's state.
    pub state: String,
    /// The fidelity shown at the time, when recorded.
    pub fidelity: Option<String>,
}

/// One process from the spawn ledger (kept 7 days after it exits).
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProcessRecordInfo {
    /// Its pid.
    #[ts(type = "number | null")]
    pub pid: Option<i64>,
    /// `root` | `helper` | …
    pub role: String,
    /// Its last ledger state.
    pub state: String,
    /// How it ended, when known.
    pub exit_kind: Option<String>,
    /// First seen, epoch ms.
    #[ts(type = "number")]
    pub first_seen_ms: i64,
    /// Last seen, epoch ms.
    #[ts(type = "number")]
    pub last_seen_ms: i64,
}

/// A read-only replay of an ended or archived Instance.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ReplayInfo {
    /// The Instance.
    pub session_id: String,
    /// The transcript file, when there is one.
    pub transcript_path: Option<String>,
    /// The transcript exactly as stored (the last 16 MiB when larger).
    pub transcript: String,
    /// Whether the start of a large transcript was left out.
    pub truncated: bool,
    /// Its size in bytes.
    #[ts(type = "number")]
    pub bytes: u64,
    /// sha256 of the whole file — proof it is the stored transcript.
    pub sha256: Option<String>,
    /// The messages, for display.
    pub entries: Vec<ReplayEntryInfo>,
    /// Status changes and lifecycle steps, oldest first.
    pub history: Vec<HistoryEventInfo>,
    /// The switches it took part in.
    pub lineage: Vec<LineageStepInfo>,
    /// Its processes, from the spawn ledger.
    pub processes: Vec<ProcessRecordInfo>,
}

fn process_records(
    storage: &juggler_core::storage::Storage,
    session_id: &str,
) -> AppResult<Vec<ProcessRecordInfo>> {
    let conn = storage.raw_conn();
    let mut statement = conn
        .prepare(
            "SELECT pid, role, state, exit_kind, first_seen, last_seen FROM process_ledger
              WHERE session_id = ?1 ORDER BY first_seen",
        )
        .map_err(juggler_core::CoreError::from)?;
    let rows = statement
        .query_map([session_id], |row| {
            Ok(ProcessRecordInfo {
                pid: row.get(0)?,
                role: row.get(1)?,
                state: row.get(2)?,
                exit_kind: row.get(3)?,
                first_seen_ms: row.get(4)?,
                last_seen_ms: row.get(5)?,
            })
        })
        .map_err(juggler_core::CoreError::from)?;
    Ok(rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(juggler_core::CoreError::from)?)
}

fn role_of(line: &str) -> String {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(|value| {
            value
                .pointer("/message/role")
                .or_else(|| value.get("role"))
                .or_else(|| value.get("type"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "output".to_owned())
}

fn read_transcript(path: &Path) -> AppResult<(String, bool, u64, String)> {
    let mut file = std::fs::File::open(path).map_err(|e| io_error("replay transcript", &e))?;
    let size = file
        .metadata()
        .map_err(|e| io_error("replay transcript", &e))?
        .len();
    let mut hasher = Sha256::new();
    let mut chunk = vec![0u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut chunk)
            .map_err(|e| io_error("replay transcript", &e))?;
        if read == 0 {
            break;
        }
        hasher.update(&chunk[..read]);
    }
    let sha = hex(&hasher.finalize());
    let skip = size.saturating_sub(REPLAY_CAP_BYTES);
    file.seek(SeekFrom::Start(skip))
        .map_err(|e| io_error("replay transcript", &e))?;
    let mut raw = Vec::new();
    file.read_to_end(&mut raw)
        .map_err(|e| io_error("replay transcript", &e))?;
    Ok((
        String::from_utf8_lossy(&raw).into_owned(),
        skip > 0,
        size,
        sha,
    ))
}

/// Core: the read-only replay of an ended or archived Instance.
///
/// # Errors
///
/// Returns [`AppError::Validation`] for a live Instance (open its
/// terminal instead), or the typed storage/IO error.
pub fn instance_replay_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<ReplayInfo> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let id = SessionId::new(session_id);
    let record = storage.sessions().get(&id)?;
    if !crate::commands::is_terminal(record.status) {
        return Err(validation(
            "replay",
            "it is still live — its terminal shows it as it runs",
        ));
    }
    let mut history: Vec<HistoryEventInfo> = storage
        .sessions()
        .transitions(&id)?
        .into_iter()
        .map(|(_, to, at_ms)| HistoryEventInfo {
            kind: "status".to_owned(),
            what: to,
            at_ms,
        })
        .collect();
    history.extend(
        storage
            .sessions()
            .lifecycle_events(&id)?
            .into_iter()
            .map(|(what, at_ms)| HistoryEventInfo {
                kind: "lifecycle".to_owned(),
                what,
                at_ms,
            }),
    );
    history.sort_by_key(|event| event.at_ms);
    let lineage = storage
        .switch_graph()
        .edges_for_session(session_id)?
        .into_iter()
        .map(|(edge, outgoing)| LineageStepInfo {
            direction: if outgoing { "out" } else { "in" }.to_owned(),
            other_account: if outgoing {
                edge.to_account.to_string()
            } else {
                edge.from_account.to_string()
            },
            other_provider: if outgoing {
                edge.to_provider.to_string()
            } else {
                edge.from_provider.to_string()
            },
            state: format!("{:?}", edge.state).to_lowercase(),
            fidelity: edge.fidelity_shown,
        })
        .collect();
    let processes = process_records(&storage, session_id)?;
    drop(storage);
    let (transcript, truncated, bytes, sha256, path) = match &record.transcript_ref {
        Some(tr) if tr.path.exists() => {
            let (text, truncated, bytes, sha) = read_transcript(&tr.path)?;
            (
                text,
                truncated,
                bytes,
                Some(sha),
                Some(tr.path.display().to_string()),
            )
        }
        Some(tr) => (
            String::new(),
            false,
            0,
            None,
            Some(tr.path.display().to_string()),
        ),
        None => (String::new(), false, 0, None, None),
    };
    let entries = transcript
        .lines()
        .filter_map(|line| {
            let text = juggler_core::storage::line_text(line);
            (!text.trim().is_empty()).then(|| ReplayEntryInfo {
                role: role_of(line),
                text,
            })
        })
        .collect();
    Ok(ReplayInfo {
        session_id: session_id.to_owned(),
        transcript_path: path,
        transcript,
        truncated,
        bytes,
        sha256,
        entries,
        history,
        lineage,
        processes,
    })
}

// --- retention and storage -------------------------------------------------

/// One Account's retention and storage.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AccountStorageInfo {
    /// The Account.
    pub account_id: String,
    /// Its label.
    pub label: String,
    /// Ended Instances older than this many days may be cleaned up;
    /// `null` keeps them forever (the default).
    pub retention_days: Option<u32>,
    /// Instances AI Juggler holds records for.
    pub instances: u32,
    /// History events (status changes and lifecycle steps).
    pub history_events: u32,
    /// Search index lines.
    #[ts(type = "number")]
    pub index_lines: u64,
    /// The Provider's transcript files for those Instances, in bytes
    /// (measured; AI Juggler never deletes them).
    #[ts(type = "number")]
    pub transcript_bytes: u64,
}

fn load_retention(storage: &juggler_core::storage::Storage) -> BTreeMap<String, u32> {
    storage
        .app_settings()
        .get(RETENTION_KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Session ids per account id (via the launch recipes).
fn sessions_by_account(
    storage: &juggler_core::storage::Storage,
) -> AppResult<BTreeMap<String, Vec<juggler_core::model::InstanceRecord>>> {
    let mut recipe_account = BTreeMap::new();
    for account in storage.accounts().list()? {
        for recipe in storage.profiles().list_by_account(&account.id)? {
            recipe_account.insert(recipe.id.to_string(), account.id.to_string());
        }
    }
    let mut out: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for record in storage.sessions().list(None)? {
        if let Some(account) = recipe_account.get(record.profile.as_str()) {
            out.entry(account.clone()).or_default().push(record);
        }
    }
    Ok(out)
}

/// Core: every Account's retention policy and storage used.
///
/// # Errors
///
/// Returns the typed storage error on SQLite failures.
pub fn account_storage_impl(state: &Arc<AppState>) -> AppResult<Vec<AccountStorageInfo>> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let retention = load_retention(&storage);
    let by_account = sessions_by_account(&storage)?;
    let mut out = Vec::new();
    for account in storage.accounts().list()? {
        let records = by_account
            .get(account.id.as_str())
            .map_or(&[][..], Vec::as_slice);
        let mut history_events = 0i64;
        let mut index_lines = 0u64;
        let mut transcript_bytes = 0u64;
        for record in records {
            history_events += count(
                &storage,
                "SELECT (SELECT COUNT(*) FROM instance_transitions WHERE session_id = ?1)
                      + (SELECT COUNT(*) FROM instance_lifecycle WHERE session_id = ?1)",
                record.id.as_str(),
            )?;
            index_lines += storage.transcript_index().line_count(&record.id)?;
            if let Some(tr) = &record.transcript_ref {
                transcript_bytes += std::fs::metadata(&tr.path).map_or(0, |meta| meta.len());
            }
        }
        out.push(AccountStorageInfo {
            account_id: account.id.to_string(),
            label: account.label.clone(),
            retention_days: retention.get(account.id.as_str()).copied(),
            instances: u32::try_from(records.len()).unwrap_or(u32::MAX),
            history_events: u32::try_from(history_events).unwrap_or(u32::MAX),
            index_lines,
            transcript_bytes,
        });
    }
    Ok(out)
}

/// Core: sets an Account's retention (`None` = keep forever).
///
/// # Errors
///
/// Returns [`AppError::Validation`] for zero days, or the typed storage
/// error.
pub fn retention_set_impl(
    state: &Arc<AppState>,
    account_id: &str,
    days: Option<u32>,
) -> AppResult<()> {
    if days == Some(0) {
        return Err(validation(
            "retention",
            "keep at least one day, or keep forever",
        ));
    }
    let storage = crate::commands::lock_storage_arc(state)?;
    let mut retention = load_retention(&storage);
    match days {
        Some(days) => retention.insert(account_id.to_owned(), days),
        None => retention.remove(account_id),
    };
    let raw = serde_json::to_string(&retention).map_err(|e| io_error("retention", &e))?;
    storage.app_settings().set(RETENTION_KEY, &raw)?;
    Ok(())
}

/// One Instance a retention cleanup would delete, or keeps.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RetentionRowInfo {
    /// The Instance.
    pub session_id: String,
    /// Its project folder's name, when tracked.
    pub project: Option<String>,
    /// When it was last active, epoch ms.
    #[ts(type = "number")]
    pub last_active_ms: i64,
    /// Why it is kept, when it is.
    pub kept_because: Option<String>,
}

/// The dry run of a retention cleanup.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RetentionPreviewInfo {
    /// The Account.
    pub account_id: String,
    /// Its policy.
    pub retention_days: Option<u32>,
    /// What it would delete (AI Juggler's records only).
    pub deletes: Vec<RetentionRowInfo>,
    /// Old enough, but kept (live, or part of a switch history).
    pub kept: Vec<RetentionRowInfo>,
    /// What stays for every one of them.
    pub keeps: Vec<String>,
    /// The token the cleanup must present (absent when nothing to do).
    pub token: Option<String>,
}

fn retention_plan(
    storage: &juggler_core::storage::Storage,
    account_id: &str,
    now: i64,
) -> AppResult<(Option<u32>, Vec<RetentionRowInfo>, Vec<RetentionRowInfo>)> {
    let days = load_retention(storage).get(account_id).copied();
    let Some(days) = days else {
        return Ok((None, Vec::new(), Vec::new()));
    };
    let cutoff = now - i64::from(days) * 24 * 3_600_000;
    let records = sessions_by_account(storage)?
        .remove(account_id)
        .unwrap_or_default();
    let projects: BTreeMap<String, String> = storage
        .projects()
        .list()
        .unwrap_or_default()
        .into_iter()
        .map(|project| (project.id.to_string(), project.name))
        .collect();
    let mut deletes = Vec::new();
    let mut kept = Vec::new();
    for record in records {
        let last = record.last_activity.map_or(
            record.started_at.millis(),
            juggler_core::model::Timestamp::millis,
        );
        if last >= cutoff {
            continue;
        }
        let row = |kept_because| RetentionRowInfo {
            session_id: record.id.to_string(),
            project: record
                .project
                .as_ref()
                .and_then(|id| projects.get(id.as_str()).cloned()),
            last_active_ms: last,
            kept_because,
        };
        match delete_refusal(storage, &record)? {
            Some(reason) => kept.push(row(Some(reason))),
            None => deletes.push(row(None)),
        }
    }
    Ok((Some(days), deletes, kept))
}

/// Core: the retention dry run for one Account (nothing is deleted).
///
/// # Errors
///
/// Returns the typed storage error on SQLite failures.
pub fn retention_preview_impl(
    state: &Arc<AppState>,
    account_id: &str,
) -> AppResult<RetentionPreviewInfo> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let now = juggler_core::model::Timestamp::now().millis();
    let (retention_days, deletes, kept) = retention_plan(&storage, account_id, now)?;
    let token = if deletes.is_empty() {
        None
    } else {
        Some(issue_token(&storage, &format!("retention:{account_id}"))?)
    };
    Ok(RetentionPreviewInfo {
        account_id: account_id.to_owned(),
        retention_days,
        deletes,
        kept,
        keeps: vec![
            "The Provider's own transcript files".to_owned(),
            "The process and cleanup audit".to_owned(),
            "Usage in your usage totals".to_owned(),
        ],
        token,
    })
}

/// Core: applies the retention cleanup its dry run listed — only with
/// the dry run's token, and only what is still past the cutoff. Returns
/// how many Instances' records were deleted.
///
/// # Errors
///
/// Returns [`AppError::Validation`] for a wrong or expired token.
pub fn retention_apply_impl(
    state: &Arc<AppState>,
    account_id: &str,
    token: &str,
) -> AppResult<u32> {
    let storage = crate::commands::lock_storage_arc(state)?;
    spend_token(&storage, &format!("retention:{account_id}"), token)?;
    let now = juggler_core::model::Timestamp::now().millis();
    let (_, deletes, _) = retention_plan(&storage, account_id, now)?;
    for row in &deletes {
        delete_records(&storage, &SessionId::new(row.session_id.clone()))?;
    }
    Ok(u32::try_from(deletes.len()).unwrap_or(u32::MAX))
}

// --- commands --------------------------------------------------------------

/// AJ-E1007: archives an ended Instance.
///
/// # Errors
///
/// Per [`instance_archive_impl`].
#[tauri::command]
pub fn instance_archive(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<i64> {
    let at = instance_archive_impl(state.inner(), &session_id)?;
    crate::commands::emit_sessions(&app, state.inner())?;
    Ok(at)
}

/// AJ-E1007: unarchives an Instance.
///
/// # Errors
///
/// Per [`instance_unarchive_impl`].
#[tauri::command]
pub fn instance_unarchive(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<i64> {
    let at = instance_unarchive_impl(state.inner(), &session_id)?;
    crate::commands::emit_sessions(&app, state.inner())?;
    Ok(at)
}

/// AJ-E1007: what deleting an Instance would remove and keep.
///
/// # Errors
///
/// Per [`instance_delete_preview_impl`].
#[tauri::command]
pub fn instance_delete_preview(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<DeletePreviewInfo> {
    instance_delete_preview_impl(state.inner(), &session_id)
}

/// AJ-E1007: deletes an Instance's AI Juggler records (with its
/// preview's token).
///
/// # Errors
///
/// Per [`instance_delete_impl`].
#[tauri::command]
pub fn instance_delete(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    token: String,
) -> AppResult<()> {
    instance_delete_impl(state.inner(), &session_id, &token)?;
    crate::commands::emit_sessions(&app, state.inner())?;
    Ok(())
}

/// AJ-E1007: transcript text search.
///
/// # Errors
///
/// Per [`transcript_search_impl`].
#[tauri::command]
pub fn transcript_search(
    state: tauri::State<'_, Arc<AppState>>,
    query: String,
) -> AppResult<Vec<TranscriptHitInfo>> {
    transcript_search_impl(state.inner(), &query)
}

/// AJ-E1007: the read-only replay of an ended or archived Instance.
///
/// # Errors
///
/// Per [`instance_replay_impl`].
#[tauri::command]
pub fn instance_replay(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<ReplayInfo> {
    instance_replay_impl(state.inner(), &session_id)
}

/// AJ-E1007: every Account's retention and storage used.
///
/// # Errors
///
/// Per [`account_storage_impl`].
#[tauri::command]
pub fn account_storage(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<AccountStorageInfo>> {
    account_storage_impl(state.inner())
}

/// AJ-E1007: sets an Account's retention.
///
/// # Errors
///
/// Per [`retention_set_impl`].
#[tauri::command]
pub fn retention_set(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    days: Option<u32>,
) -> AppResult<()> {
    retention_set_impl(state.inner(), &account_id, days)
}

/// AJ-E1007: the retention dry run.
///
/// # Errors
///
/// Per [`retention_preview_impl`].
#[tauri::command]
pub fn retention_preview(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<RetentionPreviewInfo> {
    retention_preview_impl(state.inner(), &account_id)
}

/// AJ-E1007: applies the retention cleanup (with its dry run's token).
///
/// # Errors
///
/// Per [`retention_apply_impl`].
#[tauri::command]
pub fn retention_apply(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    token: String,
) -> AppResult<u32> {
    let deleted = retention_apply_impl(state.inner(), &account_id, &token)?;
    crate::commands::emit_sessions(&app, state.inner())?;
    Ok(deleted)
}
