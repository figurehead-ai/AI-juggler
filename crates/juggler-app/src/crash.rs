//! AJ-E1107: crash reports, behind their own setting (separate from
//! usage statistics — D-19).
//!
//! - **The setting:** `never` (the default) | `ask` | `always`, stored per
//!   Mac; `DO_NOT_TRACK` forces `never`.
//! - **What is kept:** when AI Juggler panics, a report is written under
//!   `crashes/` — only if the setting isn't `never`, and at most one per
//!   crash signature a day and five a day in total. It is scrubbed before
//!   it is written: the home directory becomes `~`, the user name goes,
//!   URLs and query strings go, secret-shaped strings are masked, and it
//!   carries no environment, host name or user.
//! - **`ask`:** the next launch shows the report in full and asks: Send,
//!   Don't send, or Always send.
//! - **Sending** needs the reporting service's address, compiled into
//!   release builds only. This build has none, so a report the user
//!   chose to send stays queued on this Mac until one that does.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The setting key (per Mac; never in a backup).
pub const SETTING_KEY: &str = "crash_reports";
/// At most this many reports a day.
pub const MAX_PER_DAY: usize = 5;
/// The most backtrace lines a report keeps.
pub const MAX_FRAMES: usize = 50;

/// The reporting service's address: compiled into release builds only,
/// and only when the release pipeline sets it.
#[must_use]
pub fn service_address() -> Option<&'static str> {
    if cfg!(debug_assertions) {
        None
    } else {
        option_env!("AI_JUGGLER_CRASH_ENDPOINT")
    }
}

/// One kept crash report — exactly what would be sent.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CrashReport {
    /// A random id.
    pub id: String,
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// Where it happened (`crate/src/file.rs:line`) and the kind of
    /// failure, hashed: the daily limit's key.
    pub signature: String,
    /// The panic message, scrubbed.
    pub message: String,
    /// Where in AI Juggler's code, scrubbed.
    pub location: String,
    /// The backtrace, scrubbed, at most 50 lines.
    pub frames: Vec<String>,
    /// The app version.
    pub app_version: String,
    /// `beta` | `stable`.
    pub channel: String,
    /// `macos` | `linux` | `windows`.
    pub os: String,
}

/// The setting and the reports waiting on this Mac.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CrashInfo {
    /// The user's choice: `never` | `ask` | `always`.
    pub setting: String,
    /// What applies (`never` under `DO_NOT_TRACK`).
    pub effective: String,
    /// Why it differs, in plain words.
    pub reason: Option<String>,
    /// Reports waiting for the user's answer (`ask`).
    pub asking: Vec<CrashReport>,
    /// Reports the user chose to send, waiting for a build that can.
    pub queued: u32,
    /// This build can send them.
    pub can_send: bool,
}

/// What the panic hook applies now (0 never, 1 ask, 2 always): set at
/// launch and whenever the setting changes.
static APPLIED: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

fn apply(state: &AppState) {
    let value = match effective(state).as_str() {
        "ask" => 1,
        "always" => 2,
        _ => 0,
    };
    APPLIED.store(value, std::sync::atomic::Ordering::Relaxed);
}

fn dir(state: &AppState) -> PathBuf {
    state.data_dir.join("crashes")
}

fn do_not_track() -> bool {
    std::env::var("DO_NOT_TRACK").is_ok_and(|v| !v.trim().is_empty() && v.trim() != "0")
}

/// The stored choice (`never` when unset).
#[must_use]
pub fn setting(state: &AppState) -> String {
    let stored = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(SETTING_KEY).ok().flatten());
    match stored.as_deref() {
        Some("ask") => "ask".to_owned(),
        Some("always") => "always".to_owned(),
        _ => "never".to_owned(),
    }
}

/// What applies.
#[must_use]
pub fn effective(state: &AppState) -> String {
    if do_not_track() {
        "never".to_owned()
    } else {
        setting(state)
    }
}

/// Scrubs one string: the home directory becomes `~`, the user name and
/// URLs (with any query string) go, secret-shaped strings are masked.
#[must_use]
pub fn scrub(text: &str, home: &str, user: &str) -> String {
    let mut out = text.to_owned();
    if home.len() > 1 {
        out = out.replace(home, "~");
    }
    if user.len() >= 3 {
        out = out.replace(user, "[user]");
    }
    out = out
        .split_inclusive(char::is_whitespace)
        .map(|word| {
            let trimmed = word.trim_end();
            if trimmed.contains("://") {
                format!("[url]{}", &word[trimmed.len()..])
            } else if let Some(at) = trimmed
                .find('?')
                .filter(|at| *at > 0 && trimmed[..*at].contains('/'))
            {
                format!("{}{}", &trimmed[..at], &word[trimmed.len()..])
            } else {
                word.to_owned()
            }
        })
        .collect();
    let (stripped, _) = juggler_handover::redaction::strip_segments(
        &[out],
        true,
        &std::collections::BTreeSet::new(),
    );
    stripped.into_iter().next().unwrap_or_default()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// Builds a scrubbed report from a panic's parts.
#[must_use]
pub fn report_of(message: &str, location: &str, backtrace: &str, at_ms: i64) -> CrashReport {
    use sha2::Digest as _;
    let home = dirs::home_dir()
        .map(|h| h.display().to_string())
        .unwrap_or_default();
    let user = std::env::var("USER").unwrap_or_default();
    let location = scrub(location, &home, &user);
    // The signature: where, plus the message's first word (its kind),
    // never the message itself.
    let kind = message.split_whitespace().next().unwrap_or_default();
    let signature = hex(&sha2::Sha256::digest(format!("{location}|{kind}").as_bytes())[..8]);
    let mut id = [0u8; 8];
    let _ = getrandom::fill(&mut id);
    CrashReport {
        id: hex(&id),
        at_ms,
        signature,
        message: scrub(message, &home, &user),
        location,
        frames: backtrace
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .take(MAX_FRAMES)
            .map(|line| scrub(line, &home, &user))
            .collect(),
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        channel: crate::updates::current_channel().as_str().to_owned(),
        os: std::env::consts::OS.to_owned(),
    }
}

fn day_of(ms: i64) -> i64 {
    ms.div_euclid(86_400_000)
}

fn kept(dir: &Path, folder: &str) -> Vec<CrashReport> {
    let mut out: Vec<CrashReport> = std::fs::read_dir(dir.join(folder))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| serde_json::from_slice(&std::fs::read(entry.path()).ok()?).ok())
        .collect();
    out.sort_by_key(|report: &CrashReport| report.at_ms);
    out
}

/// Keeps `report` under `dir` if the limits allow: one per signature a
/// day, five a day in total (counting every report kept that day).
/// `ask` waits for the user; `always` is queued to send.
///
/// # Errors
///
/// Returns the filesystem error.
pub fn keep(dir: &Path, report: &CrashReport, setting: &str) -> std::io::Result<bool> {
    let folder = match setting {
        "ask" => "asking",
        "always" => "queued",
        _ => return Ok(false),
    };
    let today = day_of(report.at_ms);
    let log_path = dir.join("kept.log");
    let log = std::fs::read_to_string(&log_path).unwrap_or_default();
    let todays: Vec<&str> = log
        .lines()
        .filter_map(|line| line.split_once(' '))
        .filter(|(day, _)| day.parse::<i64>().is_ok_and(|day| day == today))
        .map(|(_, signature)| signature)
        .collect();
    if todays.len() >= MAX_PER_DAY || todays.contains(&report.signature.as_str()) {
        return Ok(false);
    }
    std::fs::create_dir_all(dir.join(folder))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    std::fs::write(
        dir.join(folder).join(format!("{}.json", report.id)),
        serde_json::to_vec_pretty(report).unwrap_or_default(),
    )?;
    // The log keeps only days and signatures (for the limits).
    let recent: Vec<&str> = log
        .lines()
        .filter(|line| {
            line.split_once(' ')
                .is_some_and(|(day, _)| day.parse::<i64>().is_ok_and(|day| today - day < 7))
        })
        .collect();
    let mut next = recent.join("\n");
    if !next.is_empty() {
        next.push('\n');
    }
    {
        use std::fmt::Write as _;
        let _ = writeln!(next, "{today} {}", report.signature);
    }
    std::fs::write(log_path, next)?;
    Ok(true)
}

/// Installs the panic hook (at launch). In `never` it keeps nothing; the
/// default hook still prints.
pub fn install(state: &Arc<AppState>) {
    let dir = dir(state);
    apply(state);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let setting = match APPLIED.load(std::sync::atomic::Ordering::Relaxed) {
            1 => "ask",
            2 => "always",
            _ => "never",
        };
        if setting != "never" {
            let message = info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_default();
            let location = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_default();
            let backtrace = std::backtrace::Backtrace::force_capture().to_string();
            let report = report_of(
                &message,
                &location,
                &backtrace,
                juggler_core::model::Timestamp::now().millis(),
            );
            let _ = keep(&dir, &report, setting);
        }
        previous(info);
    }));
}

/// Core: the setting and the reports waiting.
#[must_use]
pub fn crash_info_impl(state: &Arc<AppState>) -> CrashInfo {
    let dir = dir(state);
    let effective = effective(state);
    CrashInfo {
        setting: setting(state),
        reason: do_not_track().then(|| "Never, because DO_NOT_TRACK is set".to_owned()),
        asking: if effective == "never" {
            Vec::new()
        } else {
            kept(&dir, "asking")
        },
        queued: u32::try_from(kept(&dir, "queued").len()).unwrap_or(u32::MAX),
        can_send: service_address().is_some(),
        effective,
    }
}

/// Core: sets `never` | `ask` | `always` (`never` deletes every kept
/// report).
///
/// # Errors
///
/// Returns [`AppError::Validation`] for anything else.
pub fn crash_set_impl(state: &Arc<AppState>, setting: &str) -> AppResult<CrashInfo> {
    if !matches!(setting, "never" | "ask" | "always") {
        return Err(AppError::Validation {
            subject: "crash reports",
            message: format!("unknown setting {setting:?}"),
        });
    }
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage.app_settings().set(SETTING_KEY, setting)?;
    }
    apply(state);
    if setting == "never" {
        match std::fs::remove_dir_all(dir(state)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(AppError::io("crash reports", &e));
            }
            _ => {}
        }
    }
    Ok(crash_info_impl(state))
}

/// Core: the user's answer to the next-launch question for one report:
/// `send` | `dont_send` | `always_send`.
///
/// # Errors
///
/// Returns [`AppError::Validation`] for an unknown answer or report.
pub fn crash_answer_impl(state: &Arc<AppState>, id: &str, answer: &str) -> AppResult<CrashInfo> {
    if !id.bytes().all(|b| b.is_ascii_hexdigit()) || id.is_empty() {
        return Err(AppError::Validation {
            subject: "crash report",
            message: "unknown report".to_owned(),
        });
    }
    let dir = dir(state);
    let from = dir.join("asking").join(format!("{id}.json"));
    if !from.exists() {
        return Err(AppError::Validation {
            subject: "crash report",
            message: "that report isn't waiting any more".to_owned(),
        });
    }
    match answer {
        "dont_send" => {
            std::fs::remove_file(&from).map_err(|e| AppError::io("crash reports", &e))?;
        }
        "send" | "always_send" => {
            std::fs::create_dir_all(dir.join("queued"))
                .map_err(|e| AppError::io("crash reports", &e))?;
            std::fs::rename(&from, dir.join("queued").join(format!("{id}.json")))
                .map_err(|e| AppError::io("crash reports", &e))?;
            if answer == "always_send" {
                {
                    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
                    storage.app_settings().set(SETTING_KEY, "always")?;
                }
                apply(state);
            }
        }
        _ => {
            return Err(AppError::Validation {
                subject: "crash report",
                message: format!("unknown answer {answer:?}"),
            });
        }
    }
    Ok(crash_info_impl(state))
}

/// AJ-E1107: the crash-report setting and waiting reports.
#[tauri::command]
#[must_use]
pub fn crash_info(state: tauri::State<'_, Arc<AppState>>) -> CrashInfo {
    crash_info_impl(state.inner())
}

/// AJ-E1107: sets the crash-report setting.
///
/// # Errors
///
/// Per [`crash_set_impl`].
#[tauri::command]
pub fn crash_set(state: tauri::State<'_, Arc<AppState>>, setting: String) -> AppResult<CrashInfo> {
    crash_set_impl(state.inner(), &setting)
}

/// AJ-E1107: answers the next-launch question for one report.
///
/// # Errors
///
/// Per [`crash_answer_impl`].
#[tauri::command]
pub fn crash_answer(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    answer: String,
) -> AppResult<CrashInfo> {
    crash_answer_impl(state.inner(), &id, &answer)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_seeded_crash_is_scrubbed_before_it_is_kept() {
        let home = "/Users/someone-q9";
        let user = "someone-q9";
        let secret = "sk-ant-api03-c2VjcmV0LXNlY3JldC1zZWNyZXQtc2VjcmV0";
        let message = format!(
            "called `Result::unwrap()` on {home}/projects/acme with {secret} for someone-q9"
        );
        let frames = format!(
            "  0: juggler_app::switch at {home}/Dev/AIJuggler/crates/juggler-app/src/commands.rs:42\n  1: fetch https://figurehead.ai/api?token=abc\n  2: open {home}/x?query=1"
        );
        let scrubbed = [scrub(&message, home, user), scrub(&frames, home, user)].join("\n");
        for leaked in [home, user, secret, "https://", "token=abc", "query=1"] {
            assert!(!scrubbed.contains(leaked), "{leaked} survived: {scrubbed}");
        }
        assert!(scrubbed.contains("~/projects/acme"));
        assert!(scrubbed.contains("~/Dev/AIJuggler/crates/juggler-app/src/commands.rs:42"));
        assert!(scrubbed.contains("[url]"));
        let report = report_of(
            &message,
            "crates/juggler-app/src/commands.rs:42",
            &frames,
            0,
        );
        let json = serde_json::to_string(&report).unwrap();
        for absent in ["server_name", "\"user\"", "environment", "PATH="] {
            assert!(!json.contains(absent), "{absent} in {json}");
        }
        assert!(report.frames.len() <= MAX_FRAMES);
    }

    #[test]
    fn the_limits_hold_one_per_signature_and_five_a_day() {
        let dir = tempfile::tempdir().unwrap();
        let at = |n: i64| 1_791_979_200_000 + n;
        let mut report = report_of("boom", "a.rs:1", "", at(0));
        assert!(keep(dir.path(), &report, "ask").unwrap());
        report.id = "aa".into();
        assert!(
            !keep(dir.path(), &report, "ask").unwrap(),
            "same signature, same day"
        );
        for n in 1..10 {
            let mut other = report_of("boom", &format!("b{n}.rs:1"), "", at(n));
            other.id = format!("{n:02x}");
            let _ = keep(dir.path(), &other, "always").unwrap();
        }
        assert_eq!(
            kept(dir.path(), "asking").len() + kept(dir.path(), "queued").len(),
            MAX_PER_DAY
        );
        // A new day starts over.
        let mut tomorrow = report_of("boom", "a.rs:1", "", at(86_400_000));
        tomorrow.id = "bb".into();
        assert!(keep(dir.path(), &tomorrow, "ask").unwrap());
        // Never keeps nothing.
        let mut never = report_of("boom", "z.rs:1", "", at(2 * 86_400_000));
        never.id = "cc".into();
        assert!(!keep(dir.path(), &never, "never").unwrap());
    }

    #[test]
    fn the_setting_and_answers_move_reports_and_never_deletes_them() {
        let root = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState::init(root.path().join("data")).unwrap());
        assert_eq!(crash_info_impl(&state).setting, "never");
        crash_set_impl(&state, "ask").unwrap();
        let report = report_of("boom", "a.rs:1", "", 0);
        assert!(keep(&dir(&state), &report, "ask").unwrap());
        let info = crash_info_impl(&state);
        assert_eq!(info.asking.len(), 1);
        let after = crash_answer_impl(&state, &report.id, "always_send").unwrap();
        assert_eq!(
            (after.setting.as_str(), after.queued, after.asking.len()),
            ("always", 1, 0)
        );
        assert!(crash_answer_impl(&state, "../etc", "send").is_err());
        crash_set_impl(&state, "never").unwrap();
        assert!(!dir(&state).exists(), "never deletes every kept report");
        assert!(crash_set_impl(&state, "sometimes").is_err());
    }

    #[test]
    fn this_build_cannot_send() {
        assert!(service_address().is_none() || !cfg!(debug_assertions));
    }
}
