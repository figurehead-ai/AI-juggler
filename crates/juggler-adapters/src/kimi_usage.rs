//! AJ-E1603 R3 (D-106): a Kimi Code account's plan usage — its 5-hour,
//! weekly and monthly windows — from Kimi Code's own local server, `kimi
//! web`. Labelled experimental where it is shown.
//!
//! The research pass of 2026-11-13 (`docs/research/kimi.md` § 12, the MIT
//! `MoonshotAI/kimi-code` source at `30e3ce44`, 2.6.1) found no keyless
//! source of the windows: the plan's quota lives with Moonshot, and the
//! only local way to it is `kimi web`'s documented `GET
//! /api/v1/oauth/usage` (`reference/server-api-service.md`), which asks
//! Moonshot's account service with the user's Kimi sign-in (and may
//! refresh that sign-in). So, only for an account signed in with its Kimi
//! membership (an account on a Kimi Platform API key gets no usage call):
//!
//! 1. **Attach** to a `kimi web` the user already runs for that home: a
//!    live `<home>/server/instances/<id>.json` (`{server_id, pid, host,
//!    port, started_at, heartbeat_at, host_version?}`,
//!    `kap-server/src/instanceRegistry.ts`: its pid alive, a heartbeat in
//!    the last minute, bound to loopback or every interface, 2.x), the
//!    oldest first as Kimi Code itself picks. `GET /api/v1/healthz` first
//!    (no credential), then the usage with `Authorization: Bearer
//!    <server.token>` (the home's `server.token`, trimmed), to
//!    `127.0.0.1:<port>` and nowhere else. The token is read for that one
//!    request: never stored, logged, copied, or put in an error.
//! 2. **Self-started**, when none is live: `kimi web --no-open --port 0`
//!    with `KIMI_CODE_HOME=<home>` and `KIMI_CODE_PASSWORD=<a random
//!    secret made for this run>` (the server accepts it as a credential,
//!    `services/auth/password.ts`), through the helper seam
//!    ([`juggler_adapter::process::helper`]: a cleared environment), its
//!    output discarded unread (its banner prints `server.token`). Its port
//!    is in the instance file whose `pid` is the child's; the same two
//!    requests carry `Bearer <the secret>`. Then it is stopped at once:
//!    within a hard deadline, asked to end (SIGTERM), then killed; never
//!    restarted; at most one across every account at a time
//!    ([`self_started_live`] is back to 0). The secret is set on that
//!    child only and never kept. Each start also lets Kimi Code ask
//!    Moonshot for its model list and send its own telemetry, as any start
//!    of `kimi web` does.
//!
//! This is not an agent run (no session, no model turn), so the
//! interactive-only rule of Kimi Code's plans (R4) doesn't apply to it.
//! It runs only when the account's usage is wanted: the usage cadence or
//! the user's "Measure now", never as a standing process.
//!
//! The answer is checked before anything is believed (D-06: unknown, never
//! a guess or a zero): the envelope `{code: 0, data}`; `data.kind` `ok`
//! with `quota.usages` holding only `limit5h`, `limit7d`, `monthTotal` and
//! `monthCode` (`oauth/src/managed-usage.ts`: the server's own schema, so
//! another key means the format changed), each `{usedRatio, resetAt?}`
//! with a ratio in 0–1.05 (shown at most full) and an RFC 3339 reset;
//! `extraUsage` null or the six fields of its schema; other keys are
//! tolerated. `monthCode` is the coding plan's share of the month, a part
//! of it, not a window. A window left out isn't there (a new member has no
//! week yet).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use juggler_adapter::plan_usage::{PlanPart, PlanReading};
use juggler_core::model::Confidence;
use juggler_core::quota::{QuotaSample, WindowKind};
use serde_json::Value;
use zeroize::Zeroizing;

use crate::kimi_code::ENV_HOME;

/// The variable that gives the self-started server its per-run secret.
const ENV_PASSWORD: &str = "KIMI_CODE_PASSWORD";
/// The server's folder in a home, and its instance records in it.
const INSTANCES: [&str; 2] = ["server", "instances"];
/// The server's persistent bearer token in a home.
pub const TOKEN_FILE: &str = "server.token";
/// The routes asked.
const HEALTH: &str = "/api/v1/healthz";
const USAGE: &str = "/api/v1/oauth/usage";

/// A live server wrote its heartbeat (every 15 s) this recently.
const HEARTBEAT_FRESH_MS: i64 = 60_000;
/// The largest instance record and token read.
const MAX_INSTANCE: u64 = 16 * 1024;
const MAX_TOKEN: u64 = 4 * 1024;
/// One request's limit (the server's own call to Moonshot gives up at 8 s).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(12);
/// The whole self-started run's limit, from start to answer.
const RUN_DEADLINE: Duration = Duration::from_secs(30);
/// How long a self-started server may take to end after SIGTERM.
const STOP_GRACE: Duration = Duration::from_secs(3);

/// The `usages` keys of the server's schema.
const KNOWN_USAGES: &[&str] = &["limit5h", "limit7d", "monthTotal", "monthCode"];
/// The highest ratio believed (a little over full happens at the limit).
const RATIO_MAX: f64 = 1.05;

/// A self-started server is running: at most one at a time.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
/// Self-started servers not yet reaped.
static LIVE: AtomicUsize = AtomicUsize::new(0);

/// How many self-started servers are running now (0 between runs).
#[must_use]
pub fn self_started_live() -> usize {
    LIVE.load(Ordering::Acquire)
}

/// The limits of one run (tests make them short).
#[derive(Clone, Copy, Debug)]
pub struct RunLimits {
    /// The self-started run's whole deadline.
    pub run: Duration,
    /// How long it may take to end after being asked.
    pub stop_grace: Duration,
}

impl Default for RunLimits {
    fn default() -> Self {
        Self {
            run: RUN_DEADLINE,
            stop_grace: STOP_GRACE,
        }
    }
}

/// One `kimi web` as its instance record says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instance {
    /// Its process.
    pub pid: u32,
    /// Its port (0 until it is listening).
    pub port: u16,
    /// When it started, epoch ms.
    pub started_at: i64,
    host: String,
    heartbeat_at: i64,
    host_version: Option<String>,
}

#[derive(serde::Deserialize)]
struct InstanceDisk {
    #[serde(rename = "server_id")]
    _server_id: String,
    pid: u32,
    host: String,
    port: u16,
    started_at: i64,
    heartbeat_at: i64,
    #[serde(default)]
    host_version: Option<String>,
}

/// Every instance record in `home` that reads (a malformed one is skipped).
#[must_use]
pub fn instances(home: &Path) -> Vec<Instance> {
    let Ok(entries) = std::fs::read_dir(instances_dir(home)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| {
            let text = read_small(&path, MAX_INSTANCE)?;
            let disk: InstanceDisk = serde_json::from_str(&text).ok()?;
            Some(Instance {
                pid: disk.pid,
                port: disk.port,
                started_at: disk.started_at,
                host: disk.host,
                heartbeat_at: disk.heartbeat_at,
                host_version: disk.host_version,
            })
        })
        .collect()
}

/// A host 127.0.0.1 reaches: loopback, or every interface.
fn reachable_host(host: &str) -> bool {
    host == "localhost" || host == "0.0.0.0" || host == "::" || host.starts_with("127.")
}

/// Kimi Code 2 (a record without its version is taken as one).
fn known_version(version: Option<&str>) -> bool {
    version.is_none_or(|version| version.split('.').next() == Some("2"))
}

impl Instance {
    /// Whether it is live now and reachable at 127.0.0.1 (`alive` says
    /// whether a pid runs).
    fn usable(&self, now_ms: i64, alive: &dyn Fn(u32) -> bool) -> bool {
        self.port != 0
            && reachable_host(&self.host)
            && known_version(self.host_version.as_deref())
            && (now_ms - self.heartbeat_at).abs() <= HEARTBEAT_FRESH_MS
            && alive(self.pid)
    }
}

/// The `kimi web` the user runs for `home`, as Kimi Code picks one: the
/// oldest live and reachable.
#[must_use]
pub fn live_instance(home: &Path, now_ms: i64, alive: &dyn Fn(u32) -> bool) -> Option<Instance> {
    instances(home)
        .into_iter()
        .filter(|instance| instance.usable(now_ms, alive))
        .min_by_key(|instance| instance.started_at)
}

fn read_small(path: &Path, most: u64) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > most {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// The home's `server.token`, trimmed, for one request only.
fn read_token(home: &Path) -> Option<Zeroizing<String>> {
    let raw = Zeroizing::new(read_small(&home.join(TOKEN_FILE), MAX_TOKEN)?);
    let token = raw.trim();
    (!token.is_empty()).then(|| Zeroizing::new(token.to_owned()))
}

/// Epoch ms now.
fn now_ms() -> i64 {
    juggler_core::model::Timestamp::now().millis()
}

/// The account's plan usage: from the user's own `kimi web` for `home`
/// when one is live, else from one started for this request (`bin`, the
/// `kimi` to start).
///
/// # Errors
///
/// Why it is unknown this time, in plain words, never a secret.
pub fn usage(bin: Option<&Path>, home: &Path, limits: RunLimits) -> Result<PlanReading, String> {
    if let Some(instance) = live_instance(home, now_ms(), &juggler_adapter::process::alive) {
        let token = read_token(home).ok_or_else(|| {
            "Kimi Code's server is running, but its token can't be read".to_owned()
        })?;
        return ask(instance.port, &token, Instant::now() + REQUEST_TIMEOUT * 2);
    }
    let bin = bin.ok_or_else(|| "Kimi Code isn't installed".to_owned())?;
    self_started(bin, home, limits)
}

/// healthz, then the usage, with `secret` as the credential, before
/// `deadline`.
fn ask(port: u16, secret: &str, deadline: Instant) -> Result<PlanReading, String> {
    let left = || {
        deadline
            .saturating_duration_since(Instant::now())
            .min(REQUEST_TIMEOUT)
    };
    let health = crate::local_http::get(port, HEALTH, None, left())?;
    if health.status != 200 {
        return Err(format!(
            "Kimi Code's server isn't ready (HTTP {})",
            health.status
        ));
    }
    if left().is_zero() {
        return Err("Kimi Code's server didn't answer in time".into());
    }
    let answer = crate::local_http::get(port, USAGE, Some(secret), left())?;
    parse_answer(answer.status, &answer.body, now_ms())
}

/// A random secret for one self-started server.
fn per_run_secret() -> Result<Zeroizing<String>, String> {
    let mut bytes = Zeroizing::new([0_u8; 32]);
    getrandom::fill(bytes.as_mut()).map_err(|_| "no random source".to_owned())?;
    let mut hex = Zeroizing::new(String::with_capacity(64));
    for byte in bytes.iter() {
        hex.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
        hex.push(char::from(b"0123456789abcdef"[usize::from(byte & 15)]));
    }
    Ok(hex)
}

/// A self-started server, stopped and reaped when dropped.
struct Started {
    child: std::process::Child,
    grace: Duration,
}

impl Started {
    /// Whether it has ended by itself.
    fn ended(&mut self) -> bool {
        !matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for Started {
    fn drop(&mut self) {
        if !self.ended() {
            let _ = juggler_adapter::process::terminate(&self.child);
            let until = Instant::now() + self.grace;
            while !self.ended() && Instant::now() < until {
                // poll: the server ending after SIGTERM (a child has no
                // exit event without a thread); bounded by the grace
                // (3 s), once per usage check.
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        if !self.ended() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        LIVE.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Starts `kimi web` for `home`, asks it, and stops it.
fn self_started(bin: &Path, home: &Path, limits: RunLimits) -> Result<PlanReading, String> {
    let _one = match ONE_AT_A_TIME.try_lock() {
        Ok(one) => one,
        // A run that panicked still reaped its server (its guard's drop).
        Err(std::sync::TryLockError::Poisoned(one)) => one.into_inner(),
        Err(std::sync::TryLockError::WouldBlock) => {
            return Err("another Kimi Code usage check is running".into());
        }
    };
    let deadline = Instant::now() + limits.run;
    let secret = per_run_secret()?;
    let mut command = juggler_adapter::process::helper(bin);
    command
        .args(["web", "--no-open", "--port", "0"])
        .env("PATH", child_path(bin))
        .env(ENV_HOME, home)
        .env(ENV_PASSWORD, secret.as_str())
        .current_dir(home)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let spawned = command.spawn();
    // The command held the secret: gone before anything else happens.
    drop(command);
    let child = spawned.map_err(|_| "Kimi Code's server couldn't be started".to_owned())?;
    LIVE.fetch_add(1, Ordering::AcqRel);
    let mut started = Started {
        child,
        grace: limits.stop_grace,
    };
    let port = port_of(&mut started, home, deadline)?;
    ask(port, &secret, deadline)
}

/// The child's `PATH`: the folder `kimi` is in (an npm install's `node`
/// is beside it), then the system's.
fn child_path(bin: &Path) -> String {
    let system = juggler_adapter::process::SYSTEM_PATH;
    bin.parent()
        .and_then(Path::to_str)
        .filter(|dir| !dir.is_empty() && !dir.contains(':'))
        .map_or_else(|| system.to_owned(), |dir| format!("{dir}:{system}"))
}

/// The port the self-started server listens on, from the instance record
/// with its pid, before `deadline`.
fn port_of(started: &mut Started, home: &Path, deadline: Instant) -> Result<u16, String> {
    let pid = started.child.id();
    loop {
        if let Some(port) = instances(home)
            .into_iter()
            .find(|instance| instance.pid == pid && instance.port != 0)
            .filter(|instance| reachable_host(&instance.host))
            .map(|instance| instance.port)
        {
            return Ok(port);
        }
        if started.ended() {
            return Err("Kimi Code's server stopped before it was ready".into());
        }
        if Instant::now() >= deadline {
            return Err("Kimi Code's server didn't start in time".into());
        }
        // poll: the self-started server writing its port into its instance
        // record (no event to wait on); bounded by the run's deadline
        // (30 s), only while the account's usage is being read.
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// What a usage answer says, after its format check.
///
/// # Errors
///
/// A refusal, Kimi Code's own error, or an answer of another form, in
/// plain words (the server's message text is never kept).
pub fn parse_answer(status: u16, body: &[u8], now_ms: i64) -> Result<PlanReading, String> {
    let changed = || "Kimi Code's usage came in a form AI Juggler doesn't read yet".to_owned();
    match status {
        200 => {}
        401 | 403 => {
            return Err(format!(
                "Kimi Code's server didn't accept the request (HTTP {status})"
            ));
        }
        other => return Err(format!("Kimi Code's server answered HTTP {other}")),
    }
    let envelope: Value = serde_json::from_slice(body).map_err(|_| changed())?;
    if envelope.get("code").and_then(Value::as_i64) != Some(0) {
        return Err("Kimi Code's server answered with an error".into());
    }
    let data = envelope
        .get("data")
        .filter(|data| data.is_object())
        .ok_or_else(changed)?;
    match data.get("kind").and_then(Value::as_str) {
        Some("ok") => {}
        Some("error") => {
            return Err(data.get("status").and_then(Value::as_u64).map_or_else(
                || "Kimi Code couldn't get the usage from Moonshot".to_owned(),
                |status| format!("Kimi Code couldn't get the usage from Moonshot (HTTP {status})"),
            ));
        }
        _ => return Err(changed()),
    }
    let quota = data
        .get("quota")
        .filter(|quota| quota.is_object())
        .ok_or_else(changed)?;
    let usages = quota
        .get("usages")
        .and_then(Value::as_object)
        .ok_or_else(changed)?;
    if usages
        .keys()
        .any(|key| !KNOWN_USAGES.contains(&key.as_str()))
    {
        return Err(changed());
    }
    if !extra_usage_reads(quota.get("extraUsage")) {
        return Err(changed());
    }
    let mut reading = PlanReading::default();
    for (key, entry) in usages {
        let (used_pct, resets_at) = entry_of(entry).ok_or_else(changed)?;
        let sample = QuotaSample {
            ts: now_ms,
            used_pct,
            resets_at,
            confidence: Confidence::Measured,
            held_until: None,
        };
        match key.as_str() {
            "limit5h" => reading.windows.push((WindowKind::FiveHour, sample)),
            "limit7d" => reading
                .windows
                .push((WindowKind::Weekly { anchor_ms: None }, sample)),
            "monthTotal" => reading.windows.push((WindowKind::Monthly, sample)),
            _ => reading.parts.push(PlanPart {
                window: WindowKind::Monthly,
                part: "code".to_owned(),
                used_pct,
            }),
        }
    }
    Ok(reading)
}

/// One `{usedRatio, resetAt?}`: percent used (at most 100) and the reset.
fn entry_of(entry: &Value) -> Option<(f64, Option<i64>)> {
    let ratio = entry.get("usedRatio")?.as_f64()?;
    if !ratio.is_finite() || !(0.0..=RATIO_MAX).contains(&ratio) {
        return None;
    }
    let resets_at = match entry.get("resetAt") {
        None | Some(Value::Null) => None,
        Some(reset) => Some(juggler_core::time::timestamp_ms(reset.as_str()?)?),
    };
    Some((ratio.min(1.0) * 100.0, resets_at))
}

/// `extraUsage` reads: absent, null, or the six fields of its schema.
fn extra_usage_reads(extra: Option<&Value>) -> bool {
    let Some(extra) = extra.filter(|extra| !extra.is_null()) else {
        return true;
    };
    let int = |key: &str| extra.get(key).is_some_and(|v| v.is_i64() || v.is_u64());
    [
        "balanceCents",
        "totalCents",
        "monthlyChargeLimitCents",
        "monthlyUsedCents",
    ]
    .into_iter()
    .all(int)
        && extra
            .get("monthlyChargeLimitEnabled")
            .is_some_and(Value::is_boolean)
        && extra.get("currency").is_some_and(Value::is_string)
}

/// The folder of a home's server instance records (`server/instances`).
#[must_use]
pub fn instances_dir(home: &Path) -> PathBuf {
    INSTANCES
        .iter()
        .fold(home.to_path_buf(), |dir, part| dir.join(part))
}

#[cfg(test)]
mod tests;
