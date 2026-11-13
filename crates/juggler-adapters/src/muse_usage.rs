//! The plan's windows, as Muse Code reports them (AJ-E1608 R3, D-106's
//! route-first order).
//!
//! Muse records the 5-hour window and the week its provider last reported
//! (`SubscriptionUsage`: each `usedPercent`, verbatim and possibly over
//! 100, and `resetsAtMs`; `observedAtMs`, when Muse received it). AI
//! Juggler asks Muse's own session server for them, under the account's
//! own isolation and sign-in, on the usage cadence or "Measure now" only
//! ([`juggler_adapter::ToolQuota::plan_usage`]):
//!
//! 1. `muse serve`, briefly: `initialize` with the experimental API, then
//!    `usage/refresh` (one fresh read: a key mint by Muse itself, at most
//!    once a minute per account here, [`REFRESH_EVERY`]) or, between
//!    those, `usage/read` (what Muse last saw, no network). The server is
//!    stopped before this returns.
//! 2. When that fails, `muse usage --json` (Muse's own command, no model
//!    turn); its output's shape isn't documented, so only the same
//!    `usage` object is read from it, and anything else is an error.
//!
//! Each window is a Measured reading as of `observedAtMs`, never as of
//! now; a window Muse didn't report isn't there (D-06); the plan's name is
//! Muse's own `tierName` (never its tier id). Nothing is kept but the
//! readings: no answer, key or log.

use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use juggler_adapter::EnvMap;
use juggler_adapter::plan_usage::PlanReading;
use juggler_adapter::runtime::Connection;
use juggler_adapter::runtime::pipe::Pipe;
use juggler_core::model::Confidence;
use juggler_core::quota::{QuotaSample, WindowKind};
use serde_json::{Value, json};

use crate::codex_appserver::rpc::{self, Incoming};
use crate::muse_serve::codec::{self, methods};

/// The least time between two fresh reads for one account.
pub const REFRESH_EVERY: Duration = Duration::from_secs(60);
/// A whole check, server or command, stops past this.
const DEADLINE: Duration = Duration::from_secs(20);
/// How long a stopped server may take to end before it is killed.
const STOP_GRACE: Duration = Duration::from_secs(3);
/// The 5-hour window's length.
const FIVE_HOURS_MINS: u64 = 300;

fn refreshed() -> &'static Mutex<HashMap<PathBuf, Instant>> {
    static REFRESHED: OnceLock<Mutex<HashMap<PathBuf, Instant>>> = OnceLock::new();
    REFRESHED.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Whether a fresh read is allowed for the account at `state_dir` now (and
/// counts it): at most one per [`REFRESH_EVERY`].
#[must_use]
pub fn may_refresh(state_dir: &Path, now: Instant) -> bool {
    let Ok(mut last) = refreshed().lock() else {
        return false;
    };
    match last.get(state_dir) {
        Some(at) if now.saturating_duration_since(*at) < REFRESH_EVERY => false,
        _ => {
            last.insert(state_dir.to_path_buf(), now);
            true
        }
    }
}

/// A `usage/read` or `usage/refresh` result (or `muse usage --json`'s
/// object) as readings. No `usage` member is no reading yet (Muse saw none);
/// a refresh that says the sign-in has no subscription is an error; a
/// window in a shape not read is an error, never a zero.
///
/// # Errors
///
/// A plain message (never a secret).
pub fn reading_of(result: &Value) -> Result<PlanReading, String> {
    if result.get("subscriptionActive").and_then(Value::as_bool) == Some(false) {
        return Err("Muse Code is signed in without a subscription: no plan windows".into());
    }
    let mut reading = PlanReading {
        plan: result
            .get("tierName")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .map(str::to_owned),
        ..PlanReading::default()
    };
    let Some(usage) = result.get("usage") else {
        return Ok(reading);
    };
    let changed = || "Muse Code's usage isn't in the shape AI Juggler reads (1.4.13)".to_owned();
    let observed = usage
        .get("observedAtMs")
        .and_then(Value::as_i64)
        .ok_or_else(changed)?;
    let sample = |block: &Value| -> Result<QuotaSample, String> {
        Ok(QuotaSample {
            ts: observed,
            used_pct: block
                .get("usedPercent")
                .and_then(Value::as_f64)
                .filter(|pct| *pct >= 0.0)
                .ok_or_else(changed)?,
            resets_at: block.get("resetsAtMs").and_then(Value::as_i64),
            confidence: Confidence::Measured,
            held_until: None,
        })
    };
    if let Some(window) = usage.get("window") {
        // Only the 5-hour window is one AI Juggler keeps; another length
        // would be a window it can't name, so it isn't guessed at.
        if window.get("windowDurationMins").and_then(Value::as_u64) == Some(FIVE_HOURS_MINS) {
            reading
                .windows
                .push((WindowKind::FiveHour, sample(window)?));
        }
    }
    if let Some(weekly) = usage.get("weekly") {
        reading
            .windows
            .push((WindowKind::Weekly { anchor_ms: None }, sample(weekly)?));
    }
    Ok(reading)
}

/// The account's plan windows: Muse's server first, its command second.
///
/// # Errors
///
/// A plain message when neither answered.
pub fn usage(bin: &Path, state_dir: &Path, env: &EnvMap) -> Result<PlanReading, String> {
    let refresh = may_refresh(state_dir, Instant::now());
    match from_server(bin, state_dir, env, refresh) {
        Ok(reading) => Ok(reading),
        Err(server) => {
            from_command(bin, state_dir, env).map_err(|command| format!("{server}; and {command}"))
        }
    }
}

/// A started Muse, stopped and reaped when dropped.
struct Started(Child);

impl Drop for Started {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = juggler_adapter::process::terminate(&self.0);
            let until = Instant::now() + STOP_GRACE;
            while matches!(self.0.try_wait(), Ok(None)) && Instant::now() < until {
                // poll: the server ending after SIGTERM, bounded by the
                // grace (3 s), once per usage check.
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

/// Muse run for the account: its own binary with the account's isolation
/// (a variable that runs unset is left out), output piped or dropped.
fn command(bin: &Path, state_dir: &Path, env: &EnvMap, args: &[&str]) -> std::process::Command {
    let mut command = juggler_adapter::process::helper(bin);
    command.args(args).current_dir(state_dir);
    for (key, value) in env {
        if value != juggler_adapter::launch::ENV_UNSET {
            command.env(key, value);
        }
    }
    command
}

fn from_server(
    bin: &Path,
    state_dir: &Path,
    env: &EnvMap,
    refresh: bool,
) -> Result<PlanReading, String> {
    let deadline = Instant::now() + DEADLINE;
    let mut child = command(bin, state_dir, env, crate::muse_serve::SERVER_ARGS)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Muse Code's session server couldn't be started".to_owned())?;
    let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        drop(Started(child));
        return Err("Muse Code's session server has no pipes".into());
    };
    let _started = Started(child);
    let mut pipe = Pipe::new(Connection {
        reader: Box::new(BufReader::new(stdout)),
        writer: Box::new(stdin),
    });
    pipe.send(&codec::initialize(1, true))
        .map_err(|_| "Muse Code's session server closed".to_owned())?;
    answer(&mut pipe, 1, deadline)?;
    pipe.send(&codec::initialized())
        .map_err(|_| "Muse Code's session server closed".to_owned())?;
    if refresh {
        pipe.send(&codec::request(2, methods::USAGE_REFRESH, json!({})))
            .map_err(|_| "Muse Code's session server closed".to_owned())?;
        if let Ok(result) = answer(&mut pipe, 2, deadline) {
            return reading_of(&result);
        }
    }
    pipe.send(&codec::request(3, methods::USAGE_READ, json!({})))
        .map_err(|_| "Muse Code's session server closed".to_owned())?;
    reading_of(&answer(&mut pipe, 3, deadline)?)
}

/// Waits for the answer to request `id` before `deadline`.
fn answer(pipe: &mut Pipe, id: i64, deadline: Instant) -> Result<Value, String> {
    let mut unknown = 0;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || pipe.exited() {
            return Err("Muse Code's session server didn't answer in time".into());
        }
        let Some(line) = pipe.read(left.min(Duration::from_millis(500))) else {
            continue;
        };
        if let Some(Incoming::Response { id: got, result }) = rpc::message(&line, &mut unknown)
            && got == id
        {
            return result.map_err(|message| format!("Muse Code refused: {message}"));
        }
    }
}

/// `muse usage --json`, bounded in time; its JSON object is read like the
/// server's answer.
fn from_command(bin: &Path, state_dir: &Path, env: &EnvMap) -> Result<PlanReading, String> {
    let deadline = Instant::now() + DEADLINE;
    let child = command(bin, state_dir, env, &["usage", "--json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "`muse usage` couldn't be started".to_owned())?;
    let mut started = Started(child);
    let mut stdout = started
        .0
        .stdout
        .take()
        .ok_or("`muse usage` has no output")?;
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(
            &mut std::io::Read::take(&mut stdout, 64 * 1024),
            &mut text,
        );
        text
    });
    while matches!(started.0.try_wait(), Ok(None)) && Instant::now() < deadline {
        // poll: the command ending, bounded by the deadline (20 s), once
        // per usage check.
        std::thread::sleep(Duration::from_millis(50));
    }
    let ended = started.0.try_wait().ok().flatten();
    drop(started);
    let text = reader.join().unwrap_or_default();
    match ended {
        Some(status) if status.success() => {}
        Some(_) => return Err("`muse usage` failed (not signed in, or no subscription)".into()),
        None => return Err("`muse usage` didn't answer in time".into()),
    }
    let value: Value = serde_json::from_str(text.trim())
        .map_err(|_| "`muse usage --json` printed something AI Juggler doesn't read".to_owned())?;
    if value.get("usage").is_some() || value.get("subscriptionActive").is_some() {
        reading_of(&value)
    } else if value.get("window").is_some() || value.get("weekly").is_some() {
        reading_of(&json!({"usage": value}))
    } else {
        Err("`muse usage --json` printed something AI Juggler doesn't read".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refresh_reads_as_measured_windows_as_of_their_observation() {
        let result = json!({
            "subscriptionActive": true,
            "tierName": "High",
            "usage": {
                "observedAtMs": 1_794_589_000_000_i64,
                "tier": "tier_high_internal",
                "window": {"usedPercent": 112, "resetsAtMs": 1_794_600_000_000_i64, "windowDurationMins": 300},
                "weekly": {"usedPercent": 40, "resetsAtMs": 1_795_000_000_000_i64}
            }
        });
        let reading = reading_of(&result).unwrap();
        assert_eq!(
            reading.plan.as_deref(),
            Some("High"),
            "tierName, never the tier id"
        );
        assert_eq!(reading.windows.len(), 2);
        let (kind, five) = reading.windows[0];
        assert_eq!(kind, WindowKind::FiveHour);
        assert_eq!(five.ts, 1_794_589_000_000, "as of when Muse saw it");
        assert!(
            (five.used_pct - 112.0).abs() < f64::EPSILON,
            "over 100 kept"
        );
        assert_eq!(five.confidence, Confidence::Measured);
        assert_eq!(reading.windows[1].0, WindowKind::Weekly { anchor_ms: None });
    }

    #[test]
    fn nothing_seen_no_subscription_and_a_changed_shape_are_told_apart() {
        assert_eq!(reading_of(&json!({})).unwrap(), PlanReading::default());
        assert!(reading_of(&json!({"subscriptionActive": false})).is_err());
        assert!(reading_of(&json!({"usage": {"window": {"usedPercent": 1}}})).is_err());
        let odd = json!({"usage": {"observedAtMs": 1, "window": {"usedPercent": 5, "resetsAtMs": 2, "windowDurationMins": 240}}});
        assert!(
            reading_of(&odd).unwrap().windows.is_empty(),
            "a window it can't name isn't guessed"
        );
    }

    #[test]
    fn a_fresh_read_is_allowed_once_a_minute_per_account() {
        let now = Instant::now();
        let a = Path::new("/kit/muse-usage-a");
        assert!(may_refresh(a, now));
        assert!(!may_refresh(a, now + Duration::from_secs(59)));
        assert!(may_refresh(Path::new("/kit/muse-usage-b"), now));
        assert!(may_refresh(a, now + Duration::from_secs(61)));
    }
}
