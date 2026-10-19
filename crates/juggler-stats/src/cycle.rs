//! The weekly cycle (spec §5), run at launch and then hourly:
//!
//! 1. **Close.** Every closed ISO week (UTC) still in the store is
//!    assembled once into `pending/` (with an upload time 0–144 h after
//!    it closed) or `skipped/` with its reason: `local`,
//!    `partial_consent` (sharing didn't cover the whole week), `invalid`
//!    or `expired`.
//! 2. **Upload.** At most one due report per tick, oldest first, only in
//!    `share` and only where the build may upload. The config is read
//!    first (cached a day, reused up to 7); it can switch collection off,
//!    narrow versions or schemas, sample, or ask for a puzzle. The upload
//!    URL is compiled in and never read from the config.
//!
//! The crate does no networking itself: the app passes a [`Transport`],
//! and tests pass a recording one.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::config::{self, Config};
use crate::mode::{EffectiveMode, Environment, Mode};
use crate::report::{self, Envelope, Report};
use crate::store::{Store, StoreError};
use crate::{StatGauge, week};

/// Where reports go (compiled in).
pub const REPORTS_URL: &str = "https://stats.figurehead.ai/v1/reports";
/// The config file.
pub const CONFIG_URL: &str = "https://stats.figurehead.ai/v1/config.json";

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 24 * HOUR_MS;
/// Uploads spread over the 6 days after a week closes.
pub const UPLOAD_SPREAD_MS: i64 = 144 * HOUR_MS;
/// A report expires 13 days after its week ended.
pub const EXPIRY_MS: i64 = 13 * DAY_MS;
/// `Retry-After` is honoured up to 72 h.
pub const RETRY_AFTER_CAP_MS: i64 = 72 * HOUR_MS;
/// Give up after this many failed attempts.
pub const MAX_ATTEMPTS: u32 = 6;
/// Backoff after a network error or an unexpected status (±20 %).
pub const BACKOFF_HOURS: [i64; 5] = [1, 3, 9, 24, 48];
/// Sent and skipped reports are kept 12 weeks.
pub const KEEP_MS: i64 = 12 * 7 * DAY_MS;
/// `upload.log` is rotated past this size.
pub const LOG_ROTATE_BYTES: u64 = 256 * 1024;

/// Where this build may send reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoints {
    /// `POST` reports here.
    pub reports: String,
    /// `GET` the config here.
    pub config: String,
}

impl Endpoints {
    /// Release builds: the compiled-in service (the endpoint variable is
    /// ignored). Debug builds: only `AI_JUGGLER_STATS_ENDPOINT`, or none.
    #[must_use]
    pub fn for_build(env: &Environment) -> Option<Self> {
        if !env.debug_build {
            return Some(Self {
                reports: REPORTS_URL.to_owned(),
                config: CONFIG_URL.to_owned(),
            });
        }
        let reports = env.endpoint.as_deref()?.trim().trim_end_matches('/');
        if !(reports.starts_with("http://") || reports.starts_with("https://")) {
            return None;
        }
        let base = reports.strip_suffix("/v1/reports").unwrap_or(reports);
        Some(Self {
            reports: format!("{base}/v1/reports"),
            config: format!("{base}/v1/config.json"),
        })
    }
}

/// One HTTP response, as far as the cycle needs it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Response {
    /// The status code.
    pub status: u16,
    /// The `Retry-After` header.
    pub retry_after: Option<String>,
    /// The body.
    pub body: Vec<u8>,
}

/// Sends the two stats requests. Implementations MUST send
/// `User-Agent: AIJuggler/<version>`, no cookies, and time out after 10 s.
pub trait Transport {
    /// `GET url`.
    ///
    /// # Errors
    ///
    /// A network failure (no response).
    fn get(&self, url: &str) -> Result<Response, String>;

    /// `POST url` with a gzip-encoded JSON body
    /// (`Content-Type: application/json`, `Content-Encoding: gzip`).
    ///
    /// # Errors
    ///
    /// A network failure (no response).
    fn post_gzip_json(&self, url: &str, body: &[u8]) -> Result<Response, String>;
}

/// Everything a tick needs from the app.
pub struct Context<'a> {
    /// What applies after the overrides.
    pub effective: EffectiveMode,
    /// The user answered the current consent version (otherwise nothing
    /// is sent until they do).
    pub consent_current: bool,
    /// The consent version they answered.
    pub consent_version: u32,
    /// App and OS.
    pub envelope: &'a Envelope,
    /// Where this build may send (`None`: nowhere).
    pub endpoints: Option<&'a Endpoints>,
    /// The network.
    pub transport: &'a dyn Transport,
    /// Now, epoch ms.
    pub now_ms: i64,
    /// The environment gauges, snapshotted when a week closes.
    pub gauges: &'a dyn Fn() -> Vec<StatGauge>,
    /// CPU time allowed for a puzzle.
    pub pow_budget: Duration,
}

/// Where a closed week went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Waiting for its upload time.
    Pending {
        /// When it may be uploaded, epoch ms.
        upload_at_ms: i64,
    },
    /// Never sent; visible locally.
    Skipped(String),
}

/// What happened to the report this tick looked at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UploadOutcome {
    /// Accepted.
    Sent,
    /// Moved to `skipped/` with this reason.
    Skipped(String),
    /// Try again at this time (epoch ms); the status, if one came back.
    Retry {
        /// When.
        at_ms: i64,
        /// The HTTP status, or `None` for a network error.
        status: Option<u16>,
    },
}

/// One tick's work.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TickOutcome {
    /// Weeks assembled, oldest first.
    pub assembled: Vec<(String, Placement)>,
    /// Reports moved or attempted, with the outcome (at most one upload).
    pub uploads: Vec<(String, UploadOutcome)>,
}

/// A report file and its state (Settings ▸ Privacy).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ReportFile {
    /// The ISO week.
    pub week: String,
    /// `pending` | `sent` | `skipped`.
    pub status: &'static str,
    /// Why it was skipped.
    pub reason: Option<String>,
    /// When a pending report may be uploaded.
    pub upload_at_ms: Option<i64>,
    /// When a failed upload is retried.
    pub next_try_ms: Option<i64>,
    /// Failed attempts so far.
    pub attempts: u32,
    /// Its size, uncompressed.
    pub bytes: u64,
}

fn random_u64() -> u64 {
    let mut b = [0u8; 8];
    let _ = getrandom::fill(&mut b);
    u64::from_le_bytes(b)
}

/// A random duration in `0..=max_ms`.
fn random_ms(max_ms: i64) -> i64 {
    let span = u64::try_from(max_ms.max(0)).unwrap_or(0).saturating_add(1);
    i64::try_from(random_u64() % span).unwrap_or(0)
}

/// `ms` ±20 %.
fn jitter(ms: i64) -> i64 {
    let fifth = ms / 5;
    ms - fifth + random_ms(2 * fifth)
}

/// The sampling decision (spec §5.3): stable within a week, unlinkable
/// across weeks, and the salt never leaves the Mac.
#[must_use]
pub fn sampled(salt: &[u8], week: &str, rate: f64) -> bool {
    if rate >= 1.0 {
        return true;
    }
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(week.as_bytes());
    let digest = hasher.finalize();
    let mut first = [0u8; 8];
    first.copy_from_slice(&digest[..8]);
    #[allow(clippy::cast_precision_loss)] // a fraction in [0, 1)
    let u = u64::from_be_bytes(first) as f64 / 18_446_744_073_709_551_616.0;
    u < rate
}

fn day_label(at_ms: i64) -> String {
    let (y, m, d) = week::civil(at_ms.div_euclid(DAY_MS));
    format!("{y:04}-{m:02}-{d:02}")
}

fn meta_i64(store: &Store, key: &str) -> Result<Option<i64>, StoreError> {
    Ok(store.meta(key)?.and_then(|v| v.parse().ok()))
}

fn file(dir: &Path, status: &str, week: &str) -> PathBuf {
    dir.join(status).join(format!("{week}.json"))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Keeps `share_since` in step with the effective mode: set when sharing
/// starts, cleared when it stops. A week counts as consented only if
/// sharing started before it did.
///
/// # Errors
///
/// Returns [`StoreError`] on SQLite failures.
pub fn observe_mode(
    store: &Store,
    effective: EffectiveMode,
    now_ms: i64,
) -> Result<(), StoreError> {
    if effective.mode == Mode::Share {
        if store.meta("share_since")?.is_none() {
            store.set_meta("share_since", &now_ms.to_string())?;
        }
    } else {
        store.delete_meta("share_since")?;
    }
    Ok(())
}

/// Runs one tick: assemble every closed week, then attempt at most one
/// upload. In `off` (or with no `stats/` folder) it does nothing at all.
///
/// # Errors
///
/// Returns [`StoreError`] on storage failures (network failures are
/// outcomes, not errors).
pub fn tick(dir: &Path, ctx: &Context<'_>) -> Result<TickOutcome, StoreError> {
    let mut out = TickOutcome::default();
    if ctx.effective.mode == Mode::Off || !dir.exists() {
        return Ok(out);
    }
    let store = Store::open(dir)?;
    observe_mode(&store, ctx.effective, ctx.now_ms)?;
    let current = week::label(ctx.now_ms);
    if store.meta("first_week")?.is_none() {
        store.set_meta("first_week", &current)?;
    }
    store.mark_active(&day_label(ctx.now_ms))?;
    for closed in store.weeks()? {
        if week::end_ms(&closed).is_some_and(|end| end <= ctx.now_ms) {
            let placement = close_week(&store, dir, &closed, &current, ctx)?;
            out.assembled.push((closed, placement));
        }
    }
    upload_due(&store, dir, &current, ctx, &mut out)?;
    prune(&store, dir, ctx.now_ms)?;
    Ok(out)
}

fn close_week(
    store: &Store,
    dir: &Path,
    closed: &str,
    current: &str,
    ctx: &Context<'_>,
) -> Result<Placement, StoreError> {
    for gauge in (ctx.gauges)() {
        store.set_gauge(
            closed,
            gauge.metric(),
            &crate::dims_key(&gauge.dims()),
            &gauge.value(),
        )?;
    }
    let (report, adjustments) = report::assemble(store, closed, ctx.envelope, ctx.consent_version)?;
    let start = week::start_ms(closed).unwrap_or(0);
    let end = week::end_ms(closed).unwrap_or(0);
    let share_since = meta_i64(store, "share_since")?;
    let placement = if report::validate(&report).is_err() {
        store.add(current, "stats.invalid_report", "{}", 1)?;
        Placement::Skipped("invalid".into())
    } else if ctx.effective.mode != Mode::Share {
        Placement::Skipped("local".into())
    } else if share_since.is_none_or(|since| since > start) {
        Placement::Skipped("partial_consent".into())
    } else if ctx.now_ms > end + EXPIRY_MS {
        Placement::Skipped("expired".into())
    } else {
        Placement::Pending {
            upload_at_ms: end + random_ms(UPLOAD_SPREAD_MS),
        }
    };
    let bytes = report.to_bytes();
    match &placement {
        Placement::Pending { upload_at_ms } => {
            write_file(&file(dir, "pending", closed), &bytes)?;
            store.set_meta(&format!("upload_at:{closed}"), &upload_at_ms.to_string())?;
            store.set_meta(&format!("week_state:{closed}"), "pending")?;
        }
        Placement::Skipped(reason) => {
            write_file(&file(dir, "skipped", closed), &bytes)?;
            store.set_meta(
                &format!("week_state:{closed}"),
                &format!("skipped:{reason}"),
            )?;
        }
    }
    store.transaction(|store| {
        if adjustments.truncated > 0 {
            store.add(current, "stats.truncated", "{}", adjustments.truncated)?;
        }
        store.forget_week(closed, &day_label(end))
    })?;
    Ok(placement)
}

fn skip(store: &Store, dir: &Path, week: &str, reason: &str) -> Result<UploadOutcome, StoreError> {
    let from = file(dir, "pending", week);
    if from.exists() {
        std::fs::rename(&from, file(dir, "skipped", week))?;
    }
    store.set_meta(&format!("week_state:{week}"), &format!("skipped:{reason}"))?;
    for key in ["upload_at", "next_try", "attempts"] {
        store.delete_meta(&format!("{key}:{week}"))?;
    }
    Ok(UploadOutcome::Skipped(reason.to_owned()))
}

fn pending_weeks(dir: &Path) -> Vec<String> {
    let mut weeks: Vec<String> = std::fs::read_dir(dir.join("pending"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|e| {
                    e.file_name()
                        .to_str()?
                        .strip_suffix(".json")
                        .map(str::to_owned)
                })
                .filter(|w| week::start_ms(w).is_some())
                .collect()
        })
        .unwrap_or_default();
    weeks.sort();
    weeks
}

/// The config: a copy younger than a day; otherwise a fetch (at most once
/// a day while a usable copy exists); otherwise a copy up to 7 days old.
fn config_for(
    store: &Store,
    ctx: &Context<'_>,
    endpoints: &Endpoints,
) -> Result<Option<Config>, StoreError> {
    let now = ctx.now_ms;
    let cached = store
        .meta("config")?
        .and_then(|raw| Config::parse(raw.as_bytes()));
    let fetched_at = meta_i64(store, "config_fetched_at")?.unwrap_or(i64::MIN / 2);
    let tried_at = meta_i64(store, "config_tried_at")?.unwrap_or(i64::MIN / 2);
    let usable = cached.is_some() && now - fetched_at < config::USABLE_MS;
    if cached.is_some() && now - fetched_at < config::FRESH_MS {
        return Ok(cached);
    }
    if usable && now - tried_at < config::FRESH_MS {
        return Ok(cached);
    }
    store.set_meta("config_tried_at", &now.to_string())?;
    if let Ok(response) = ctx.transport.get(&endpoints.config)
        && response.status == 200
        && let Some(fresh) = Config::parse(&response.body)
    {
        store.set_meta("config", &String::from_utf8_lossy(&response.body))?;
        store.set_meta("config_fetched_at", &now.to_string())?;
        return Ok(Some(fresh));
    }
    Ok(if usable { cached } else { None })
}

fn log_attempt(
    dir: &Path,
    now_ms: i64,
    week: &str,
    attempt: u32,
    status: Option<u16>,
    bytes: usize,
) {
    let path = dir.join("upload.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > LOG_ROTATE_BYTES) {
        let _ = std::fs::rename(&path, dir.join("upload.log.1"));
    }
    let line = serde_json::json!({
        "ts": now_ms,
        "week": week,
        "attempt": attempt,
        "status": status.map_or_else(|| serde_json::Value::from("network"), serde_json::Value::from),
        "bytes": bytes,
    });
    if let Ok(mut log) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(log, "{line}");
    }
}

fn retry_after_ms(header: Option<&str>) -> Option<i64> {
    let seconds: i64 = header?.trim().parse().ok()?;
    Some((seconds.max(0) * 1000).min(RETRY_AFTER_CAP_MS))
}

fn upload_due(
    store: &Store,
    dir: &Path,
    current: &str,
    ctx: &Context<'_>,
    out: &mut TickOutcome,
) -> Result<(), StoreError> {
    let now = ctx.now_ms;
    for week in pending_weeks(dir) {
        let end = week::end_ms(&week).unwrap_or(0);
        if now > end + EXPIRY_MS {
            out.uploads
                .push((week.clone(), skip(store, dir, &week, "gave_up")?));
            continue;
        }
        if ctx.effective.mode != Mode::Share {
            out.uploads
                .push((week.clone(), skip(store, dir, &week, "mode_changed")?));
            continue;
        }
        let upload_at = meta_i64(store, &format!("upload_at:{week}"))?.unwrap_or(end);
        let next_try = meta_i64(store, &format!("next_try:{week}"))?.unwrap_or(i64::MIN);
        if upload_at > now || next_try > now {
            continue;
        }
        let (true, true, Some(endpoints)) =
            (ctx.effective.can_upload, ctx.consent_current, ctx.endpoints)
        else {
            // Counting continues; this build, environment or consent state
            // never sends.
            return Ok(());
        };
        let Some(config) = config_for(store, ctx, endpoints)? else {
            return Ok(()); // no usable config: try again next tick
        };
        if let Some(reason) = refusal(store, ctx, &config, &week)? {
            out.uploads
                .push((week.clone(), skip(store, dir, &week, reason)?));
            continue;
        }
        let path = file(dir, "pending", &week);
        let Some(mut report) = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Report>(&bytes).ok())
        else {
            out.uploads
                .push((week.clone(), skip(store, dir, &week, "invalid")?));
            continue;
        };
        report.sample_rate = config.sample_rate;
        report.pow = None;
        if config.pow_bits > crate::pow::MAX_BITS {
            let at_ms = now + DAY_MS;
            store.set_meta(&format!("next_try:{week}"), &at_ms.to_string())?;
            out.uploads.push((
                week,
                UploadOutcome::Retry {
                    at_ms,
                    status: None,
                },
            ));
            return Ok(());
        }
        if config.pow_bits > 0 {
            let Some(nonce) =
                crate::pow::solve(&report.report_id, &week, config.pow_bits, ctx.pow_budget)
            else {
                return Ok(()); // out of budget: next tick
            };
            report.pow = Some(nonce);
        }
        let limit = config.max_report_bytes.min(report::MAX_REPORT_BYTES);
        let dropped = report::truncate(&mut report, limit);
        if dropped > 0 {
            store.add(current, "stats.truncated", "{}", dropped)?;
        }
        // The pending file holds exactly what is sent.
        let bytes = report.to_bytes();
        write_file(&path, &bytes)?;
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gzip.write_all(&bytes)?;
        let body = gzip.finish()?;
        let attempts = meta_i64(store, &format!("attempts:{week}"))?
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0);
        let response = ctx.transport.post_gzip_json(&endpoints.reports, &body);
        log_attempt(
            dir,
            now,
            &week,
            attempts + 1,
            response.as_ref().ok().map(|r| r.status),
            bytes.len(),
        );
        let outcome = settle(store, dir, &week, attempts, response, now)?;
        out.uploads.push((week, outcome));
        return Ok(()); // never more than one upload per tick
    }
    Ok(())
}

/// Why the config says not to send this week, if it does.
fn refusal(
    store: &Store,
    ctx: &Context<'_>,
    config: &Config,
    week: &str,
) -> Result<Option<&'static str>, StoreError> {
    Ok(if !config.enabled {
        Some("server_disabled")
    } else if !config.accepts_version(&ctx.envelope.app_version) {
        Some("version")
    } else if !config.schemas.contains(&crate::SCHEMA_VERSION) {
        Some("schema")
    } else if !sampled(&store.salt()?, week, config.sample_rate) {
        Some("not_sampled")
    } else {
        None
    })
}

/// Files the upload's response (spec §5.2 step 6).
fn settle(
    store: &Store,
    dir: &Path,
    week: &str,
    attempts: u32,
    response: Result<Response, String>,
    now: i64,
) -> Result<UploadOutcome, StoreError> {
    let status = response.as_ref().ok().map(|r| r.status);
    match response {
        Ok(r) if matches!(r.status, 200 | 202) => {
            std::fs::rename(file(dir, "pending", week), file(dir, "sent", week))?;
            store.set_meta(&format!("week_state:{week}"), "sent")?;
            for key in ["upload_at", "next_try", "attempts"] {
                store.delete_meta(&format!("{key}:{week}"))?;
            }
            Ok(UploadOutcome::Sent)
        }
        Ok(r) if matches!(r.status, 400 | 404 | 413) => {
            skip(store, dir, week, &format!("rejected:{}", r.status))
        }
        failed => {
            let attempts = attempts + 1;
            if attempts >= MAX_ATTEMPTS {
                return skip(store, dir, week, "gave_up");
            }
            let delay = match &failed {
                Ok(r) if r.status == 403 && is_pow_challenge(&r.body) => {
                    // The service raised the puzzle: refetch the config and
                    // solve again next tick.
                    store.delete_meta("config")?;
                    store.delete_meta("config_fetched_at")?;
                    0
                }
                Ok(r) if matches!(r.status, 429 | 503) => retry_after_ms(r.retry_after.as_deref())
                    .unwrap_or_else(|| jitter(backoff_ms(attempts))),
                _ => jitter(backoff_ms(attempts)),
            };
            let at_ms = now + delay;
            store.set_meta(&format!("attempts:{week}"), &attempts.to_string())?;
            store.set_meta(&format!("next_try:{week}"), &at_ms.to_string())?;
            Ok(UploadOutcome::Retry { at_ms, status })
        }
    }
}

fn backoff_ms(attempts: u32) -> i64 {
    let index = usize::try_from(attempts.saturating_sub(1)).unwrap_or(0);
    BACKOFF_HOURS[index.min(BACKOFF_HOURS.len() - 1)] * HOUR_MS
}

fn is_pow_challenge(body: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(body)
        .is_ok_and(|v| v.get("error").and_then(serde_json::Value::as_str) == Some("pow"))
}

/// Deletes sent and skipped reports older than 12 weeks.
fn prune(store: &Store, dir: &Path, now_ms: i64) -> Result<(), StoreError> {
    for status in ["sent", "skipped"] {
        let Ok(entries) = std::fs::read_dir(dir.join(status)) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name();
            let Some(week) = name.to_str().and_then(|n| n.strip_suffix(".json")) else {
                continue;
            };
            if week::start_ms(week).is_some_and(|start| start < now_ms - KEEP_MS) {
                std::fs::remove_file(entry.path())?;
                store.delete_meta(&format!("week_state:{week}"))?;
            }
        }
    }
    Ok(())
}

/// Every report on this Mac, newest first. Creates nothing.
///
/// # Errors
///
/// Returns [`StoreError`] on storage failures.
pub fn reports(dir: &Path) -> Result<Vec<ReportFile>, StoreError> {
    if !dir.join("counters.sqlite").exists() {
        return Ok(Vec::new());
    }
    let store = Store::open(dir)?;
    let mut out = Vec::new();
    for status in ["pending", "sent", "skipped"] {
        let Ok(entries) = std::fs::read_dir(dir.join(status)) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name();
            let Some(week) = name.to_str().and_then(|n| n.strip_suffix(".json")) else {
                continue;
            };
            let state = store.meta(&format!("week_state:{week}"))?;
            out.push(ReportFile {
                week: week.to_owned(),
                status,
                reason: state
                    .as_deref()
                    .and_then(|s| s.strip_prefix("skipped:"))
                    .map(str::to_owned),
                upload_at_ms: meta_i64(&store, &format!("upload_at:{week}"))?,
                next_try_ms: meta_i64(&store, &format!("next_try:{week}"))?,
                attempts: meta_i64(&store, &format!("attempts:{week}"))?
                    .and_then(|n| u32::try_from(n).ok())
                    .unwrap_or(0),
                bytes: entry.metadata().map_or(0, |m| m.len()),
            });
        }
    }
    out.sort_by(|a, b| b.week.cmp(&a.week));
    Ok(out)
}

/// The exact JSON of one report (`status`: pending, sent or skipped).
///
/// # Errors
///
/// Returns the filesystem error (or `InvalidInput` for a bad name).
pub fn read_report(dir: &Path, status: &str, week: &str) -> std::io::Result<String> {
    if !matches!(status, "pending" | "sent" | "skipped") || week::start_ms(week).is_none() {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }
    std::fs::read_to_string(file(dir, status, week))
}

#[cfg(test)]
#[path = "cycle_tests.rs"]
mod tests;
