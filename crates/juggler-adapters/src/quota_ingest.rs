//! Quota ingest pipeline (AJ-E607, deep/09 §Q1 + §Q8): feeds the
//! AJ-E606 engine from every sanctioned LOCAL source — the Claude
//! statusline spool (Measured quota samples), Claude transcript
//! `usage` blocks (Measured tokens), Codex rollout
//! `token_usage_record` lines (Measured tokens), and the `ZCode` SQLite
//! `model_usage` table (Computed credits via the published formula).
//! Gemini has NO emitter — AJ-E605 verified no source at any grade;
//! an absent emitter with honest `unknown` beats a speculative parser.
//!
//! Laws this module lives by:
//! - PASSIVE: no Provider process is launched, signalled, or written
//!   to. The `ZCode` DB opens READ-ONLY (WAL-safe concurrent reads,
//!   SPEC §4.3). The ONE sanctioned write is the managed statusline
//!   wrapper inside AI Juggler's own account dir — a pass-through
//!   that chains any pre-existing user statusline byte-for-byte in
//!   behaviour and uninstalls to the exact prior setting.
//! - OFFLINE: no HTTP to any Provider host exists anywhere in this
//!   crate (test-pinned); no quota data leaves the machine.
//! - IDEMPOTENT: re-tailing the same segment inserts zero new events
//!   — `UNIQUE(account, msg_uuid)` at the store (AJ-E606) plus
//!   offset-tracked tails here. File truncation/rotation resets the
//!   offset (full reparse; the budget allows it) and dedupe absorbs
//!   the replay.

use std::io::Read;
use std::path::{Path, PathBuf};

use juggler_core::model::Confidence;
use juggler_core::model::TokenCounts;
use juggler_core::quota::{QuotaSample, UsageEvent, UsageSource, WindowKind};
use juggler_core::renewal::CalendarDate;

/// Spool rotation threshold (Req.6): 10 MB per spool file.
pub const SPOOL_ROTATE_BYTES: u64 = 10 * 1024 * 1024;

/// The managed statusline wrapper's path inside an account dir.
const WRAPPER_NAME: &str = "juggler-statusline.sh";
/// The spool's managed directory (never outside the account dir).
const SPOOL_DIR_NAME: &str = "quota-spool";
/// The spool file name.
const SPOOL_FILE_NAME: &str = "statusline.jsonl";
/// Where the pre-existing user statusline is preserved verbatim.
const BACKUP_NAME: &str = "juggler-statusline-backup.json";

/// Per-source health (Req.5): the observable state the AJ-E610
/// "Feed stale" banner binds to. A source silent >10 min demotes its
/// Measured readings to Stale in AJ-E606 terms — data is never
/// deleted.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SourceHealth {
    /// Last successfully ingested event time, epoch milliseconds.
    pub last_event_at: Option<i64>,
    /// Malformed lines/rows dropped since startup (honest damage
    /// count; never resets to look healthy).
    pub parse_errors: u64,
}

/// Result of one incremental tail pass.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct TailOutcome {
    /// Events ready for the store (already uuid-tagged for dedupe).
    pub events: Vec<UsageEvent>,
    /// Quota samples ready for the store (Claude's statusline spool;
    /// Codex's rollout `token_count` lines).
    pub samples: Vec<(WindowKind, QuotaSample)>,
    /// The newest Codex credits snapshot seen this pass, when any.
    pub credits: Option<CreditsSnapshot>,
    /// The model the slice's last Codex `turn_context` named. A rollout's
    /// usage lines carry no model: the next slice's lines before their
    /// own `turn_context` ran on this one (the pipeline carries it).
    pub model: Option<String>,
    /// Bytes consumed — the next pass starts here.
    pub offset: u64,
    /// Malformed lines seen this pass.
    pub parse_errors: u64,
}

// ---------------------------------------------------------------------------
// Managed statusline capture (Req.1)
// ---------------------------------------------------------------------------

/// One shell word: single-quoted, with `'` escaped. Claude runs a status
/// line `command` through the shell, and AI Juggler's data folder is
/// under "Application Support" — unquoted, the path split at the space
/// and the wrapper never ran (found in the ADR-0018 review).
#[must_use]
pub fn shell_word(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// A status line command to chain — never one of AI Juggler's own
/// wrappers (quoted or not): chaining one would run the wrapper inside
/// itself on every refresh (a mixed-version backup can hold one).
fn not_ours(command: Option<String>) -> Option<String> {
    command.filter(|command| !command.contains(WRAPPER_NAME))
}

/// Writes `bytes` only when the file differs, atomically — running
/// sessions read the wrapper every few hundred milliseconds.
fn write_if_changed(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if std::fs::read(path).is_ok_and(|current| current == bytes) {
        return Ok(());
    }
    juggler_core::fs::atomic_write(path, bytes)
}

/// Renders the pass-through wrapper. `chain` is the user's own
/// statusline command, verbatim, when one pre-exists — it runs AFTER
/// the spool append on the SAME stdin bytes, so the user's display is
/// unchanged. The timestamp is second-granularity (`date +%s` +
/// `000`) — ample for 10-minute staleness and window maths.
#[must_use]
pub fn statusline_script(spool_path: &Path, chain: Option<&str>) -> String {
    let chain_line = match chain {
        Some(command) => format!(
            "CHAIN='{}'\nif [ -n \"$CHAIN\" ]; then printf '%s' \"$payload\" | sh -c \"$CHAIN\"; fi",
            command.replace('\'', "'\\''")
        ),
        None => String::new(),
    };
    format!(
        "#!/bin/sh\n\
         # AI Juggler managed quota capture (AJ-E607) — PASS-THROUGH wrapper.\n\
         # Appends the statusline stdin JSON to Juggler's local spool, then\n\
         # chains the user's own statusline with the SAME stdin bytes.\n\
         # Uninstalling (AI Juggler) restores the prior statusline exactly.\n\
         SPOOL={spool}\n\
         payload=$(cat)\n\
         printf '%s\\t%s\\n' \"$(date +%s)000\" \"$payload\" >> \"$SPOOL\"\n\
         {chain_line}\n",
        spool = shell_word(&spool_path.to_string_lossy()),
        chain_line = chain_line
    )
}

/// Installs (idempotently) the managed statusline wrapper into a
/// Claude account's `settings.json`. A pre-existing user statusline is
/// backed up verbatim and CHAINED, never replaced. Re-running with the
/// wrapper already active is a no-op.
///
/// # Errors
///
/// Returns a message when `settings.json` cannot be read/parsed or
/// the wrapper file cannot be written — the account's own statusline
/// is never left half-configured (settings are only rewritten after
/// the wrapper + backup exist on disk).
pub fn install_statusline(config_dir: &Path) -> Result<bool, String> {
    let settings_path = config_dir.join("settings.json");
    let mut settings: serde_json::Value = match std::fs::read(&settings_path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("settings.json unparseable: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            serde_json::json!({}) // no settings yet — a fresh object is honest
        }
        Err(error) => return Err(format!("settings.json unreadable: {error}")),
    };
    let wrapper = config_dir.join(WRAPPER_NAME);
    let spool_dir = config_dir.join(SPOOL_DIR_NAME);
    std::fs::create_dir_all(&spool_dir).map_err(|error| format!("spool dir: {error}"))?;
    let spool = spool_dir.join(SPOOL_FILE_NAME);

    let existing = settings.get("statusLine").cloned();
    let wrapper_str = wrapper.to_string_lossy().into_owned();
    let wrapper_command = shell_word(&wrapper_str);
    let existing_command = existing.as_ref().and_then(|value| {
        value
            .get("command")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    });
    if existing_command.as_deref() == Some(wrapper_command.as_str()) {
        return Ok(false); // idempotent — nothing to do
    }
    // An older install wrote the path unquoted (it never ran from a path
    // with a space). It's ours: re-point it, and keep the backup — that
    // still holds the user's own status line.
    let stale_ours = existing_command.as_deref() == Some(wrapper_str.as_str());

    // Backup BEFORE any mutation; the backup is the uninstall truth.
    if stale_ours {
        // keep the existing backup as it is
    } else if existing.is_some() {
        let backup = serde_json::json!({ "statusLine": existing });
        std::fs::write(
            config_dir.join(BACKUP_NAME),
            serde_json::to_vec_pretty(&backup).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("backup write: {error}"))?;
    } else {
        let _ = std::fs::remove_file(config_dir.join(BACKUP_NAME));
    }

    let chain = if stale_ours {
        std::fs::read(config_dir.join(BACKUP_NAME))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .and_then(|backup| {
                backup
                    .get("statusLine")
                    .and_then(|line| line.get("command"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
    } else {
        existing_command
    };
    let chain = not_ours(chain);
    write_if_changed(
        &wrapper,
        statusline_script(&spool, chain.as_deref()).as_bytes(),
    )
    .map_err(|error| format!("wrapper write: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755));
    }

    let object = settings
        .as_object_mut()
        .ok_or("settings.json is not an object")?;
    object.insert(
        "statusLine".into(),
        serde_json::json!({ "type": "command", "command": wrapper_command }),
    );
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("settings write: {error}"))?;
    Ok(true)
}

/// Uninstalls the wrapper and restores the pre-existing statusline
/// EXACTLY (the backed-up value verbatim; the key removed when none
/// existed). Returns whether anything was removed.
///
/// # Errors
///
/// Returns a message on I/O or parse failures; the wrapper is only
/// deleted after `settings.json` carries the restored value.
pub fn uninstall_statusline(config_dir: &Path) -> Result<bool, String> {
    let settings_path = config_dir.join("settings.json");
    let mut settings: serde_json::Value = match std::fs::read(&settings_path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("settings.json unparseable: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("settings.json unreadable: {error}")),
    };
    let wrapper = config_dir.join(WRAPPER_NAME);
    if !wrapper.exists() {
        return Ok(false);
    }
    let object = settings
        .as_object_mut()
        .ok_or("settings.json is not an object")?;
    match std::fs::read(config_dir.join(BACKUP_NAME)) {
        Ok(bytes) => {
            let backup: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|error| format!("backup: {error}"))?;
            let restored = backup.get("statusLine").cloned();
            if let Some(value) = restored {
                object.insert("statusLine".into(), value);
            } else {
                object.remove("statusLine");
            }
            let _ = std::fs::remove_file(config_dir.join(BACKUP_NAME));
        }
        Err(_) => {
            // No backup ⇒ we installed over nothing: remove the key.
            object.remove("statusLine");
        }
    }
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("settings write: {error}"))?;
    let _ = std::fs::remove_file(&wrapper);
    Ok(true)
}

// ---------------------------------------------------------------------------
// Spool tail + rotation (Req.1/3/6)
// ---------------------------------------------------------------------------

/// Rotates the spool at [`SPOOL_ROTATE_BYTES`] (keep one generation;
/// the previous `.1` is replaced). A no-op under the threshold.
pub fn rotate_spool_if_needed(spool: &Path) {
    let Ok(meta) = std::fs::metadata(spool) else {
        return;
    };
    if meta.len() < SPOOL_ROTATE_BYTES {
        return;
    }
    let rotated = spool.with_extension("jsonl.1");
    let _ = std::fs::remove_file(&rotated);
    let _ = std::fs::rename(spool, rotated);
}

/// Tails the spool from `offset` (Req.3: idempotent — the caller
/// persists the returned offset). Truncation (size < offset) resets
/// to 0 and reparses the whole file; dedupe absorbs the replay.
/// Each line is `<epoch-ms>\t<statusline JSON>`; payloads without
/// `rate_limits` yield NO samples (the honest absence — tokens come
/// from the transcript source, not the statusline).
#[must_use]
pub fn tail_spool(spool: &Path, offset: u64) -> TailOutcome {
    let mut outcome = TailOutcome {
        offset,
        ..TailOutcome::default()
    };
    let Ok(mut file) = std::fs::File::open(spool) else {
        return outcome;
    };
    let Ok(meta) = file.metadata() else {
        return outcome;
    };
    let len = meta.len();
    if len < offset {
        outcome.offset = 0; // rotated/truncated — full reparse
    }
    if outcome.offset >= len {
        return outcome;
    }
    if let Ok(seeked) = std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(outcome.offset)) {
        outcome.offset = seeked;
    }
    let mut text = String::new();
    let _ = file.read_to_string(&mut text);
    // A trailing partial line stays for the next pass: only consume
    // through the last newline.
    let consumed_upto = text.rfind('\n').map_or(0, |index| index + 1);
    outcome.offset += consumed_upto as u64;
    for line in text[..consumed_upto].lines() {
        match parse_spool_line(line) {
            Some((ts, payload)) => {
                for (kind, sample) in statusline_samples(ts, &payload) {
                    outcome.samples.push((kind, sample));
                }
                if outcome.samples.is_empty() && payload.get("rate_limits").is_some() {
                    // rate_limits present but unparseable shapes — count it.
                    outcome.parse_errors += 1;
                }
            }
            None => {
                if !line.trim().is_empty() {
                    outcome.parse_errors += 1;
                }
            }
        }
    }
    outcome
}

/// One spool line: `<epoch-ms>\t<json>`.
#[must_use]
fn parse_spool_line(line: &str) -> Option<(i64, serde_json::Value)> {
    let (ts, payload) = line.split_once('\t')?;
    let ts: i64 = ts.parse().ok()?;
    let payload: serde_json::Value = serde_json::from_str(payload).ok()?;
    Some((ts, payload))
}

/// Extracts Measured quota samples from a statusline payload — ONLY
/// from `rate_limits` (the AJ-E602 candidate shape
/// `rate_limits.five_hour/seven_day.{used_percentage, resets_at}`).
/// Fields absent ⇒ no sample, never a derived number.
#[must_use]
pub fn statusline_samples(ts: i64, payload: &serde_json::Value) -> Vec<(WindowKind, QuotaSample)> {
    let Some(limits) = payload.get("rate_limits") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut push = |key: &str, kind: WindowKind| {
        let entry = limits.get(key)?;
        let used = entry
            .get("used_percentage")
            .and_then(serde_json::Value::as_f64)?;
        let resets_at = entry.get("resets_at").and_then(serde_json::Value::as_i64);
        out.push((
            kind,
            QuotaSample {
                ts,
                used_pct: used.clamp(0.0, 100.0),
                resets_at,
                confidence: Confidence::Measured,
            },
        ));
        Some(())
    };
    push("five_hour", WindowKind::FiveHour);
    push("seven_day", WindowKind::Weekly { anchor_ms: None });
    out
}

// ---------------------------------------------------------------------------
// ISO-timestamp → epoch-ms (line-level, no chrono dependency)
// ---------------------------------------------------------------------------

/// Parses `YYYY-MM-DDTHH:MM:SS(.fff)?Z` into epoch milliseconds.
/// Returns `None` for anything else — an unparseable timestamp is
/// honest damage (counted), never a fabricated one.
#[must_use]
pub fn iso_to_epoch_ms(raw: &str) -> Option<i64> {
    let date_end = raw.find('T')?;
    let date = CalendarDate::parse(&raw[..date_end])?;
    let time = &raw[date_end + 1..];
    let time = time.strip_suffix('Z').unwrap_or(time);
    let (hms, millis) = match time.split_once('.') {
        Some((hms, frac)) => (hms, frac.parse::<i64>().ok()?),
        None => (time, 0),
    };
    let mut parts = hms.split(':');
    let hours: i64 = parts.next()?.parse().ok()?;
    let minutes: i64 = parts.next()?.parse().ok()?;
    let seconds: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some()
        || !(0..24).contains(&hours)
        || !(0..60).contains(&minutes)
        || !(0..60).contains(&seconds)
    {
        return None;
    }
    Some(date.civil_days() * 86_400_000 + (hours * 3_600 + minutes * 60 + seconds) * 1000 + millis)
}

// ---------------------------------------------------------------------------
// Source: Claude transcript usage (Measured tokens)
// ---------------------------------------------------------------------------

/// Extracts Measured token events from Claude transcript JSONL bytes
/// (an incremental slice is fine — lines are independent). One event
/// per API response, keyed `message.id/requestId`: Claude Code writes
/// one line per content block (thinking, text, each tool call), each
/// with its own line `uuid` but the same message and the same `usage`,
/// so keying on the line counted one response two or three times (the
/// 0.0.12 QA walk: ×1.8–2.5). Lines of one response in this slice
/// collapse to one event, each class its largest value; resumed files
/// re-emit history and the store's `UNIQUE(account, msg_uuid)` absorbs
/// it. A line with no message id falls back to its `uuid`; a line with
/// neither is skipped: unkeyed usage cannot dedupe, and double-counting
/// is the worse failure.
#[must_use]
pub fn transcript_events(account: &str, bytes: &[u8]) -> TailOutcome {
    let text = String::from_utf8_lossy(bytes);
    let mut outcome = TailOutcome::default();
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            outcome.parse_errors += 1;
            continue;
        };
        let Some(usage) = value
            .get("message")
            .and_then(|message| message.get("usage"))
        else {
            continue;
        };
        let Some(key) = response_key(&value) else {
            continue;
        };
        let ts = value
            .get("timestamp")
            .and_then(serde_json::Value::as_str)
            .and_then(iso_to_epoch_ms);
        let read = |key: &str| {
            usage
                .get(key)
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default()
        };
        let tokens = TokenCounts {
            input: read("input_tokens"),
            output: read("output_tokens"),
            cache_read: read("cache_read_input_tokens"),
            cache_write: read("cache_creation_input_tokens"),
            reasoning: 0,
        };
        if tokens == TokenCounts::default() {
            continue; // an empty usage block is not an event
        }
        if let Some(&at) = seen.get(&key) {
            let kept = &mut outcome.events[at].tokens;
            kept.input = kept.input.max(tokens.input);
            kept.output = kept.output.max(tokens.output);
            kept.cache_read = kept.cache_read.max(tokens.cache_read);
            kept.cache_write = kept.cache_write.max(tokens.cache_write);
            continue;
        }
        seen.insert(key.clone(), outcome.events.len());
        outcome.events.push(UsageEvent {
            account: account.to_owned(),
            session: value
                .get("sessionId")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            ts: ts.unwrap_or_default(),
            model: value
                .get("message")
                .and_then(|message| message.get("model"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            tokens,
            credits: None,
            usd_est: None,
            source: UsageSource::Transcript,
            confidence: Confidence::Measured,
            msg_uuid: Some(key),
        });
    }
    outcome
}

/// One Claude API response's key: `message.id/requestId` (either alone
/// when the other is missing), else the line's `uuid`.
fn response_key(value: &serde_json::Value) -> Option<String> {
    let id = value
        .get("message")
        .and_then(|message| message.get("id"))
        .and_then(serde_json::Value::as_str);
    let request = value.get("requestId").and_then(serde_json::Value::as_str);
    match (id, request) {
        (Some(id), Some(request)) => Some(format!("{id}/{request}")),
        (Some(only), None) | (None, Some(only)) => Some(only.to_owned()),
        (None, None) => value
            .get("uuid")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    }
}

// ---------------------------------------------------------------------------
// Source: Codex rollout token_usage_record (Measured tokens)
// ---------------------------------------------------------------------------

/// Extracts Measured token events from Codex rollout JSONL bytes: the
/// `token_usage_record` payload's RESPONSE-level `usage` object (the
/// per-turn/per-thread rollups on the same line are aggregation views
/// — reading only the response level keeps the sum exact). Dedupe key
/// `thread/turn/response` survives re-tails and resumed rollouts.
///
/// Codex's classes nest: `input_tokens` includes `cached_input_tokens`
/// and `output_tokens` includes `reasoning_output_tokens` (its own
/// `non_cached_input()`; `total_tokens` = input + output). Stored as
/// disjoint classes — input without the cached part, output without
/// the reasoning — so a sum of classes counts each token once (the
/// 0.0.12 QA walk: exactly ×2 before). The model is the one the
/// latest `turn_context` named.
#[must_use]
pub fn rollout_events(account: &str, bytes: &[u8]) -> TailOutcome {
    let text = String::from_utf8_lossy(bytes);
    let mut outcome = TailOutcome::default();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            outcome.parse_errors += 1;
            continue;
        };
        if take_token_count(&value, &mut outcome) {
            continue;
        }
        if value.get("type").and_then(serde_json::Value::as_str) == Some("turn_context") {
            if let Some(model) = value
                .get("payload")
                .and_then(|payload| payload.get("model"))
                .and_then(serde_json::Value::as_str)
            {
                outcome.model = Some(model.to_owned());
            }
            continue;
        }
        if value.get("type").and_then(serde_json::Value::as_str) != Some("token_usage_record") {
            continue;
        }
        let Some(payload) = value.get("payload") else {
            outcome.parse_errors += 1;
            continue;
        };
        let Some(usage) = payload.get("usage") else {
            outcome.parse_errors += 1;
            continue;
        };
        let key = |field: &str| {
            payload
                .get(field)
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        };
        let (Some(thread), Some(turn), Some(response)) =
            (key("thread_id"), key("turn_id"), key("response_id"))
        else {
            outcome.parse_errors += 1;
            continue;
        };
        let tokens = codex_token_counts(usage);
        if tokens == TokenCounts::default() {
            continue;
        }
        let ts = value
            .get("timestamp")
            .and_then(serde_json::Value::as_str)
            .and_then(iso_to_epoch_ms)
            .unwrap_or_default();
        outcome.events.push(UsageEvent {
            account: account.to_owned(),
            session: Some(thread.clone()),
            ts,
            model: outcome.model.clone(),
            tokens,
            credits: None,
            usd_est: None,
            source: UsageSource::Rollout,
            confidence: Confidence::Measured,
            msg_uuid: Some(format!("{thread}/{turn}/{response}")),
        });
    }
    outcome
}

/// One Codex `usage` object as disjoint classes, the one place this split
/// is made (the usage pipeline and the inspector's quality scan both call
/// it). Codex's classes nest: `input_tokens` includes the cached input and
/// the cache writes, and `output_tokens` includes the reasoning; its
/// `total_tokens` is input + output. Stored apart, they sum to that total,
/// so each token counts once (P14).
#[must_use]
pub fn codex_token_counts(usage: &serde_json::Value) -> TokenCounts {
    let read = |field: &str| {
        usage
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default()
    };
    let cached = read("cached_input_tokens");
    let written = read("cache_write_input_tokens");
    let reasoning = read("reasoning_output_tokens");
    TokenCounts {
        input: read("input_tokens")
            .saturating_sub(cached)
            .saturating_sub(written),
        output: read("output_tokens").saturating_sub(reasoning),
        cache_read: cached,
        cache_write: written,
        reasoning,
    }
}

/// A Codex `event_msg` `token_count` line: its `rate_limits` are the
/// vendor's own limit readings (Measured samples; no tokens here). True
/// when `value` was such a line, whatever it held.
fn take_token_count(value: &serde_json::Value, outcome: &mut TailOutcome) -> bool {
    let is_event = value.get("type").and_then(serde_json::Value::as_str) == Some("event_msg");
    let Some(payload) = value.get("payload").filter(|payload| {
        is_event && payload.get("type").and_then(serde_json::Value::as_str) == Some("token_count")
    }) else {
        return false;
    };
    let Some(limits) = payload
        .get("rate_limits")
        .filter(|limits| !limits.is_null())
    else {
        return true;
    };
    let Some(ts) = value
        .get("timestamp")
        .and_then(serde_json::Value::as_str)
        .and_then(iso_to_epoch_ms)
    else {
        outcome.parse_errors += 1;
        return true;
    };
    let (samples, credits) = codex_rate_limits(ts, limits);
    // An odd reading (a window of width 0, a null primary) is skipped
    // quietly; only a window without its figure is a broken line.
    let broken = ["primary", "secondary"].iter().any(|key| {
        limits.get(key).is_some_and(|window| {
            !window.is_null()
                && window
                    .get("used_percent")
                    .and_then(serde_json::Value::as_f64)
                    .is_none()
        })
    });
    if broken {
        outcome.parse_errors += 1;
    }
    outcome.samples.extend(samples);
    if let Some(credits) = credits
        && outcome
            .credits
            .as_ref()
            .is_none_or(|seen| seen.ts <= credits.ts)
    {
        outcome.credits = Some(credits);
    }
    true
}

/// Codex's credits, as its own rollout reports them (no value is read
/// anywhere else; `auth.json` is never opened).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreditsSnapshot {
    /// When Codex wrote it, epoch ms.
    pub ts: i64,
    /// The plan has credits at all.
    pub has_credits: bool,
    /// Credits are unlimited.
    pub unlimited: bool,
    /// The balance as Codex wrote it (a string in its protocol).
    pub balance: Option<String>,
}

/// The limit readings on one Codex `token_count` line: `rate_limits`
/// `{limit_id, primary, secondary, credits}`, each window
/// `{used_percent, window_minutes, resets_at}` (`resets_at` in Unix
/// seconds; older builds wrote `resets_in_seconds`). The window comes
/// from `window_minutes` (300 = the 5-hour pool, 10080 = the week); a
/// window of any other width is skipped, never guessed. `secondary` may
/// be null.
#[must_use]
pub fn codex_rate_limits(
    ts: i64,
    limits: &serde_json::Value,
) -> (Vec<(WindowKind, QuotaSample)>, Option<CreditsSnapshot>) {
    let mut samples = Vec::new();
    for key in ["primary", "secondary"] {
        let Some(window) = limits.get(key).filter(|window| !window.is_null()) else {
            continue;
        };
        let Some(used) = window
            .get("used_percent")
            .and_then(serde_json::Value::as_f64)
        else {
            continue;
        };
        let kind = match window
            .get("window_minutes")
            .and_then(serde_json::Value::as_i64)
        {
            Some(300) => WindowKind::FiveHour,
            Some(10_080) => WindowKind::Weekly { anchor_ms: None },
            _ => continue,
        };
        let resets_at = window
            .get("resets_at")
            .and_then(serde_json::Value::as_i64)
            // Seconds (Codex's protocol); a value already in ms passes.
            .map(|at| {
                if at < 100_000_000_000 {
                    at.saturating_mul(1000)
                } else {
                    at
                }
            })
            .or_else(|| {
                window
                    .get("resets_in_seconds")
                    .and_then(serde_json::Value::as_i64)
                    .map(|secs| ts.saturating_add(secs.saturating_mul(1000)))
            });
        samples.push((
            kind,
            QuotaSample {
                ts,
                used_pct: used.clamp(0.0, 100.0),
                resets_at,
                confidence: Confidence::Measured,
            },
        ));
    }
    let credits = limits
        .get("credits")
        .filter(|credits| !credits.is_null())
        .and_then(|credits| {
            Some(CreditsSnapshot {
                ts,
                has_credits: credits.get("has_credits")?.as_bool()?,
                unlimited: credits
                    .get("unlimited")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                balance: credits.get("balance").and_then(|balance| match balance {
                    serde_json::Value::String(text) => Some(text.clone()),
                    serde_json::Value::Number(number) => Some(number.to_string()),
                    _ => None,
                }),
            })
        });
    (samples, credits)
}

// ---------------------------------------------------------------------------
// Source: ZCode SQLite model_usage (Computed credits) — Req.4 passive
// ---------------------------------------------------------------------------

/// Published per-model credit multipliers (in, cached, out) —
/// docs.z.ai/devpack/overview revised 2026-07-30, graded in
/// `zcode-captures.md`. Unknown models carry NO credits (honest
/// absence) and mark the event Estimated.
#[must_use]
pub fn zcode_multiplier(model: &str) -> Option<(f64, f64, f64)> {
    match model {
        "GLM-5.3" => Some((6.9, 1.7, 24.0)),
        "GLM-5.3-Flash" => Some((2.3, 0.56, 8.0)),
        _ => None,
    }
}

/// True inside a PEAK window: weekdays 14:00–18:00 UTC+8 (off-peak
/// bills at 50%). Pure integer calendar maths from epoch ms.
#[must_use]
pub fn zcode_is_peak(utc_ms: i64) -> bool {
    let days = utc_ms.div_euclid(86_400_000);
    let day_ms = utc_ms.rem_euclid(86_400_000);
    let hour_utc8 = day_ms / 3_600_000 + 8;
    // 1970-01-01 was a Thursday; Monday=0 ⇒ offset 3.
    let weekday = (days + 3).rem_euclid(7);
    weekday < 5 && (14..18).contains(&hour_utc8)
}

/// Computes one row's credits:
/// `(input×in + cached×cached + output×out) / 10000`, half off-peak.
/// Bucket mapping per the E604 packet (graded `inferred`): the row's
/// `input_tokens` counts ALL prompt tokens, so fresh = input −
/// `cache_read` − `cache_creation`; cached = `cache_read` + `cache_creation`.
#[must_use]
pub fn zcode_credits(
    model: &str,
    input: u64,
    cache_read: u64,
    cache_creation: u64,
    output: u64,
    utc_ms: i64,
) -> Option<f64> {
    let (in_mult, cached_mult, out_mult) = zcode_multiplier(model)?;
    #[allow(clippy::cast_precision_loss)]
    let fresh = input
        .saturating_sub(cache_read)
        .saturating_sub(cache_creation) as f64;
    #[allow(clippy::cast_precision_loss)]
    let cached = cache_read.saturating_add(cache_creation) as f64;
    #[allow(clippy::cast_precision_loss)]
    let out = output as f64;
    let raw = (fresh * in_mult + cached * cached_mult + out * out_mult) / 10_000.0;
    Some(if zcode_is_peak(utc_ms) {
        raw
    } else {
        raw * 0.5
    })
}

/// Reads the WHOLE `model_usage` table (completed rows only) from the
/// vendor DB, opened READ-ONLY (WAL-safe). Full-table each pass is
/// the deliberate choice: rows carry no vendor uuid, so the stable
/// `rowid` is the dedupe key and completeness — not a high-water mark
/// — keeps ids honest. A day is ~1k rows; the read is <10 ms.
///
/// # Errors
///
/// Returns a message when the DB cannot be opened or the table is
/// missing (never a partial/invented read).
pub fn zcode_events(account: &str, db_path: &Path) -> Result<Vec<UsageEvent>, String> {
    use rusqlite::OpenFlags;
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| format!("zcode db open (read-only): {error}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT rowid, model_id, input_tokens, output_tokens,
                    cache_creation_input_tokens, cache_read_input_tokens, started_at
             FROM model_usage WHERE status = 'completed'",
        )
        .map_err(|error| format!("zcode model_usage query: {error}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(|error| format!("zcode model_usage rows: {error}"))?;
    let mut events = Vec::new();
    for row in rows {
        let (rowid, model, input, output, cache_creation, cache_read, started_at) =
            row.map_err(|error| format!("zcode row: {error}"))?;
        #[allow(clippy::cast_sign_loss)]
        let (input, output, cache_creation, cache_read) = (
            input.max(0) as u64,
            output.max(0) as u64,
            cache_creation.max(0) as u64,
            cache_read.max(0) as u64,
        );
        let credits = zcode_credits(
            &model,
            input,
            cache_read,
            cache_creation,
            output,
            started_at,
        );
        events.push(UsageEvent {
            account: account.to_owned(),
            session: None,
            ts: started_at,
            model: Some(model),
            tokens: TokenCounts {
                input,
                output,
                cache_read,
                cache_write: cache_creation,
                reasoning: 0,
            },
            credits,
            usd_est: None,
            source: UsageSource::Sqlite,
            confidence: if credits.is_some() {
                Confidence::Computed
            } else {
                Confidence::Estimated
            },
            msg_uuid: Some(format!("zcode:{rowid}")),
        });
    }
    Ok(events)
}

// ---------------------------------------------------------------------------
// Source: Gemini — deliberately absent (AJ-E605)
// ---------------------------------------------------------------------------

/// Gemini has NO emitter: AJ-E605 verified no quota source at any
/// grade (legacy CLI sunset for individuals 2026-06-18; the successor
/// Antigravity CLI's `/usage` is documented but ungraded live). Quota
/// renders unavailable-with-reason (D-06) and this function exists so
/// the absence is a typed, test-pinned decision — not an omission.
#[must_use]
pub fn gemini_events() -> Vec<UsageEvent> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// Generic file tail (transcripts/rollouts): offset-tracked, truncation-aware
// ---------------------------------------------------------------------------

/// Tails a JSONL source file from `offset`, returning the new bytes'
/// parse outcome. Truncation (size < offset) resets to 0 — a full
/// reparse whose duplicate events the store dedupes by uuid.
#[must_use]
pub fn tail_jsonl(
    path: &Path,
    account: &str,
    offset: u64,
    parse: fn(&str, &[u8]) -> TailOutcome,
) -> TailOutcome {
    let Ok(mut file) = std::fs::File::open(path) else {
        return TailOutcome {
            offset,
            ..TailOutcome::default()
        };
    };
    let len = file.metadata().map_or(offset, |meta| meta.len());
    let mut start = offset;
    if len < start {
        start = 0;
    }
    if start >= len {
        return TailOutcome {
            offset: start,
            ..TailOutcome::default()
        };
    }
    if let Ok(seeked) = std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(start)) {
        start = seeked;
    }
    let mut bytes = Vec::new();
    let _ = file.read_to_end(&mut bytes);
    // Consume only through the last newline (a partial trailing line
    // stays for the next pass).
    let consumed = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |index| index + 1);
    let mut outcome = parse(account, &bytes[..consumed]);
    outcome.offset = start + consumed as u64;
    outcome
}

/// Convenience: [`tail_jsonl`] with the Claude transcript parser.
#[must_use]
pub fn tail_transcript(path: &Path, account: &str, offset: u64) -> TailOutcome {
    tail_jsonl(path, account, offset, transcript_events)
}

/// Convenience: [`tail_jsonl`] with the Codex rollout parser.
#[must_use]
pub fn tail_rollout(path: &Path, account: &str, offset: u64) -> TailOutcome {
    tail_jsonl(path, account, offset, rollout_events)
}

/// The spool path for an account config dir (managed-dir-only law).
#[must_use]
pub fn spool_path(config_dir: &Path) -> PathBuf {
    config_dir.join(SPOOL_DIR_NAME).join(SPOOL_FILE_NAME)
}

// ---------------------------------------------------------------------------
// ADR-0018: in-place accounts — the capture lives in AI Juggler's data
// ---------------------------------------------------------------------------

/// The settings file AI Juggler passes with `claude --settings` for an
/// in-place account.
const OVERLAY_SETTINGS_NAME: &str = "launch-settings.json";

/// Where AI Juggler keeps an in-place account's own files (wrapper,
/// spool, launch settings): `<overlays root>/<account id>/`. Never the
/// user's folder.
#[must_use]
pub fn overlay_dir(overlays_root: &Path, account_id: &str) -> PathBuf {
    overlays_root.join(account_id)
}

/// The spool of an in-place account (see [`overlay_dir`]).
#[must_use]
pub fn overlay_spool_path(overlays_root: &Path, account_id: &str) -> PathBuf {
    spool_path(&overlay_dir(overlays_root, account_id))
}

/// Writes the in-place account's status line capture into AI Juggler's
/// own folder and returns the settings file to pass with `--settings`.
/// The user's folder is only READ: their own status line (from their
/// `settings.json`) is chained, so what they see is unchanged.
///
/// # Errors
///
/// Returns a message when AI Juggler's overlay folder can't be written.
pub fn ensure_launch_overlay(
    overlays_root: &Path,
    account_id: &str,
    config_dir: &Path,
) -> Result<PathBuf, String> {
    let dir = overlay_dir(overlays_root, account_id);
    let spool_dir = dir.join(SPOOL_DIR_NAME);
    std::fs::create_dir_all(&spool_dir).map_err(|error| format!("overlay dir: {error}"))?;
    let chain = std::fs::read(config_dir.join("settings.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|settings| {
            settings
                .get("statusLine")
                .and_then(|line| line.get("command"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
    let chain = not_ours(chain);
    let wrapper = dir.join(WRAPPER_NAME);
    write_if_changed(
        &wrapper,
        statusline_script(&spool_dir.join(SPOOL_FILE_NAME), chain.as_deref()).as_bytes(),
    )
    .map_err(|error| format!("wrapper write: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755));
    }
    let settings = serde_json::json!({
        "statusLine": { "type": "command", "command": shell_word(&wrapper.to_string_lossy()) }
    });
    let path = dir.join(OVERLAY_SETTINGS_NAME);
    write_if_changed(
        &path,
        &serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("settings write: {error}"))?;
    Ok(path)
}
