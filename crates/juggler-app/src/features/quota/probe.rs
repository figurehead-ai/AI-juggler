//! D-39 (needs the maintainer's ruling; off until then): asking the tool
//! itself for a profile's limits, when its own records are old. For Claude
//! that is Claude Code's own `/usage`, run headless in the profile
//! (`HarnessAdapter::limits_probe_spec`): a local command, so no model turn
//! and no quota; no transcript kept, no MCP server started, no hook run.
//! Claude Code asks Anthropic with its own sign-in (AI Juggler never sees
//! it, and no macOS prompt is shown: the item is Claude Code's own) and
//! leaves the answer in its state file, which `records` reads.
//!
//! - Nothing here runs, and the UI offers nothing, until [`ALLOWED_KEY`]
//!   is `on` (the ruling). Then each profile is off until its owner turns
//!   it on, and off again from the same place.
//! - While on, it runs at most every [`EVERY_MS`] per profile, only while
//!   the profile is in use (usage in the last hour) and its newest reading
//!   is older than that; or once when asked ("Measure now").
//! - A run's output is never kept or logged: only whether it ran.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use juggler_core::model::AccountId;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// `on` once D-39 is ruled for this option; until then nothing runs.
pub const ALLOWED_KEY: &str = "limits_probe_allowed";
/// Per profile: `on` when its owner turned it on.
const ACCOUNT_KEY_PREFIX: &str = "limits_probe:";
/// The least time between two runs for one profile, and the age a
/// reading must reach before a background run.
pub const EVERY_MS: i64 = 15 * 60_000;
/// "In use": usage seen this recently.
const IN_USE_MS: i64 = 60 * 60_000;
/// A run past this is stopped.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Where a profile's "ask the tool" stands.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProbeStatusInfo {
    /// Offered at all: ruled on, and the tool has a usage command.
    pub offered: bool,
    /// `off` or `on`.
    pub state: String,
    /// When it last ran here, epoch ms (this run of AI Juggler).
    #[ts(type = "number | null")]
    pub last_run_ms: Option<i64>,
    /// How that run ended: `ran`, `failed`, `timed_out`, `not_started`.
    pub last_outcome: Option<String>,
}

fn runs() -> &'static Mutex<HashMap<String, (i64, &'static str)>> {
    static RUNS: OnceLock<Mutex<HashMap<String, (i64, &'static str)>>> = OnceLock::new();
    RUNS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn account_key(account_id: &str) -> String {
    format!("{ACCOUNT_KEY_PREFIX}{account_id}")
}

fn setting_on(state: &Arc<AppState>, key: &str) -> AppResult<bool> {
    Ok(lock_storage_arc(state)?
        .app_settings()
        .get(key)?
        .is_some_and(|value| value == "on"))
}

/// The probe for an account, when it is ruled on and its tool has one.
fn spec_for(
    state: &Arc<AppState>,
    account_id: &str,
) -> AppResult<Option<juggler_adapter::LaunchSpec>> {
    if !setting_on(state, ALLOWED_KEY)? {
        return Ok(None);
    }
    let account = lock_storage_arc(state)?
        .accounts()
        .get(&AccountId::new(account_id))?;
    Ok(state
        .registry
        .get(&account.harness)
        .and_then(|adapter| adapter.limits_probe_spec(&account.state_dir)))
}

/// Where the account's probe stands (read-only).
///
/// # Errors
///
/// Storage failures; an unknown account.
pub fn status_impl(state: &Arc<AppState>, account_id: &str) -> AppResult<ProbeStatusInfo> {
    let offered = spec_for(state, account_id)?.is_some();
    let on = offered && setting_on(state, &account_key(account_id))?;
    let last = runs()
        .lock()
        .ok()
        .and_then(|runs| runs.get(account_id).copied());
    Ok(ProbeStatusInfo {
        offered,
        state: if on { "on" } else { "off" }.into(),
        last_run_ms: last.map(|(at, _)| at),
        last_outcome: last.map(|(_, outcome)| outcome.to_owned()),
    })
}

/// Turns it on (refused until it is ruled on), and asks once now.
///
/// # Errors
///
/// Refused when not offered; storage failures.
pub fn turn_on_impl(state: &Arc<AppState>, account_id: &str) -> AppResult<ProbeStatusInfo> {
    let Some(spec) = spec_for(state, account_id)? else {
        return Err(AppError::Refused(
            "asking the tool for its limits isn't available".into(),
        ));
    };
    lock_storage_arc(state)?
        .app_settings()
        .set(&account_key(account_id), "on")?;
    record(account_id, run(&spec, TIMEOUT));
    status_impl(state, account_id)
}

/// Turns it off: nothing more runs for this profile.
///
/// # Errors
///
/// Storage failures.
pub fn turn_off_impl(state: &Arc<AppState>, account_id: &str) -> AppResult<ProbeStatusInfo> {
    lock_storage_arc(state)?
        .app_settings()
        .set(&account_key(account_id), "off")?;
    status_impl(state, account_id)
}

/// Asks now, when it is on for this profile.
///
/// # Errors
///
/// Refused when it isn't on; storage failures.
pub fn run_now_impl(state: &Arc<AppState>, account_id: &str) -> AppResult<ProbeStatusInfo> {
    let spec = spec_for(state, account_id)?;
    let Some(spec) = spec.filter(|_| setting_on(state, &account_key(account_id)).unwrap_or(false))
    else {
        return Err(AppError::Refused(
            "it isn't turned on for this profile".into(),
        ));
    };
    record(account_id, run(&spec, TIMEOUT));
    status_impl(state, account_id)
}

fn record(account_id: &str, outcome: &'static str) {
    if let Ok(mut runs) = runs().lock() {
        runs.insert(
            account_id.to_owned(),
            (juggler_core::model::Timestamp::now().millis(), outcome),
        );
    }
}

/// Runs one probe and waits at most `timeout`: `ran` (it exited 0),
/// `failed`, `timed_out` (stopped) or `not_started`. Its output goes
/// nowhere: never kept, never logged.
#[must_use]
pub fn run(spec: &juggler_adapter::LaunchSpec, timeout: Duration) -> &'static str {
    let mut command = std::process::Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    juggler_adapter::launch::apply_env(&mut command, &spec.env);
    let Ok(mut child) = command.spawn() else {
        return "not_started";
    };
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return if status.success() { "ran" } else { "failed" },
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(100));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return "timed_out";
            }
        }
    }
}

/// A background run is under way (one at a time).
static RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The background ask (the usage pipeline's records cadence): for each
/// profile it is on for, in use, whose newest reading is older than
/// [`EVERY_MS`] and that wasn't asked in that long. One run at a time, off
/// the pipeline's thread.
pub(crate) fn tick(state: &Arc<AppState>, accounts: &[juggler_core::model::Account]) {
    let now_ms = juggler_core::model::Timestamp::now().millis();
    if !setting_on(state, ALLOWED_KEY).unwrap_or(false)
        || RUNNING.load(std::sync::atomic::Ordering::Acquire)
    {
        return;
    }
    let due = accounts.iter().find_map(|account| {
        let id = account.id.as_str();
        if !setting_on(state, &account_key(id)).unwrap_or(false) {
            return None;
        }
        let asked = runs().lock().ok()?.get(id).map(|(at, _)| *at);
        if asked.is_some_and(|at| now_ms.saturating_sub(at) < EVERY_MS) {
            return None;
        }
        let storage = state.storage.lock().ok()?;
        let usage = storage.usage();
        let in_use = !usage
            .events_for_account(id, now_ms - IN_USE_MS)
            .ok()?
            .is_empty();
        let newest = [
            juggler_core::quota::WindowKind::FiveHour,
            juggler_core::quota::WindowKind::Weekly { anchor_ms: None },
        ]
        .into_iter()
        .filter_map(|kind| usage.samples_for_account(id, kind).ok())
        .flatten()
        .map(|sample| sample.ts)
        .max();
        let old = newest.is_none_or(|ts| now_ms.saturating_sub(ts) >= EVERY_MS);
        drop(storage);
        (in_use && old).then(|| id.to_owned())
    });
    let Some(id) = due else {
        return;
    };
    let Ok(Some(spec)) = spec_for(state, &id) else {
        return;
    };
    RUNNING.store(true, std::sync::atomic::Ordering::Release);
    std::thread::spawn(move || {
        record(&id, run(&spec, TIMEOUT));
        RUNNING.store(false, std::sync::atomic::Ordering::Release);
    });
}

/// Where the account's "ask the tool" stands (read-only).
///
/// # Errors
///
/// As [`status_impl`].
#[tauri::command]
pub async fn quota_probe_status(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<ProbeStatusInfo> {
    crate::ipc::blocking(state.inner(), "read the limits probe", move |state| {
        status_impl(state, &account_id)
    })
    .await
}

/// Turns it on for a profile, and asks once.
///
/// # Errors
///
/// As [`turn_on_impl`].
#[tauri::command]
pub async fn quota_probe_turn_on(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<ProbeStatusInfo> {
    crate::ipc::blocking(state.inner(), "turn on the limits probe", move |state| {
        turn_on_impl(state, &account_id)
    })
    .await
}

/// Turns it off for a profile.
///
/// # Errors
///
/// As [`turn_off_impl`].
#[tauri::command]
pub async fn quota_probe_turn_off(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<ProbeStatusInfo> {
    crate::ipc::blocking(state.inner(), "turn off the limits probe", move |state| {
        turn_off_impl(state, &account_id)
    })
    .await
}

/// Asks now ("Measure now").
///
/// # Errors
///
/// As [`run_now_impl`].
#[tauri::command]
pub async fn quota_probe_run(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<ProbeStatusInfo> {
    crate::ipc::blocking(state.inner(), "ask the tool for its limits", move |state| {
        run_now_impl(state, &account_id)
    })
    .await
}
