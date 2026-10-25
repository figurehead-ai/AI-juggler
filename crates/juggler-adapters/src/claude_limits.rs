//! Claude's limits from its own records, for the sessions AI Juggler
//! didn't start (the maintainer's third report: every Claude account read
//! "Not measured yet", because the status line only exists for AI
//! Juggler's own launches). Read-only: no permission, no setting changed,
//! nothing sent.
//!
//! - **The Claude app's meter.** Each instance of the Claude app (one per
//!   `--user-data-dir`) keeps `plan-usage-history.json`:
//!   `{samples: [{org, t, u: {fh, sd}}]}`, the 5-hour (`fh`) and weekly
//!   (`sd`) percent used, about every 15 minutes while it runs, for the
//!   organisation it is signed in to. It has no reset times. Only `org`,
//!   `t`, `fh` and `sd` are read. A sample is an account's only when the
//!   instance's own Code-session folders
//!   (`claude-code-sessions/<account>/<org>/`, names only) give exactly one
//!   account in that organisation.
//! - **Claude Code's last `/usage`.** The profile's state file keeps
//!   `cachedUsageUtilization {fetchedAtMs, accountUuid, utilization:
//!   {five_hour, seven_day: {utilization, resets_at}, limits[]}}` once
//!   `/usage` ran (or an app asked Claude Code for its usage). Only those
//!   fields and `oauthAccount`'s two ids are read, through a shape that
//!   skips everything else unkept; a reading for another account is not
//!   this profile's.
//! - **Limit hits.** When a limit is hit, Claude Code writes an assistant
//!   line with `error: "rate_limit"` and `quotaLimits {status: "rejected",
//!   rateLimitType, resetsAt}` into the session's transcript. Only the
//!   window, the reset and the line's time are kept.
//!
//! All three formats are undocumented (Claude Code's docs: the transcript
//! "entry format is internal to Claude Code and changes between
//! versions"). A file or line of another shape gives nothing, never a
//! guess.

use std::io::Read;
use std::path::{Path, PathBuf};

use juggler_core::model::Confidence;
use juggler_core::quota::{QuotaSample, WindowKind};

/// The Claude app's meter history, in each of its instance folders.
pub const APP_USAGE_FILE: &str = "plan-usage-history.json";
/// The largest meter history read (they are tens of KB).
const APP_USAGE_MAX: u64 = 4 * 1024 * 1024;
/// The most meter samples taken from one history (its newest).
const APP_SAMPLES_MAX: usize = 2_000;
/// The largest Claude Code state file read (most are under 1 MB).
const STATE_FILE_MAX: u64 = 32 * 1024 * 1024;
/// The app's Code-session folders, by account then organisation.
const APP_SESSIONS_DIR: &str = "claude-code-sessions";

/// A Claude sign-in: the account and the organisation it is in. Limits
/// belong to the pair (one person can be in a personal and a team
/// organisation, each with its own limits).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignIn {
    /// `oauthAccount.accountUuid`.
    pub account: String,
    /// `oauthAccount.organizationUuid`.
    pub org: String,
}

/// Claude Code's last `/usage` reading in a profile, when it is the
/// profile's own.
#[derive(Clone, Debug, PartialEq)]
pub struct UsageCacheReading {
    /// When Claude Code fetched it, epoch ms.
    pub fetched_at: i64,
    /// Its windows, as Measured samples taken at `fetched_at`.
    pub samples: Vec<(WindowKind, QuotaSample)>,
}

/// What a profile's state file says about limits: its sign-in, and its
/// last `/usage` reading.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProfileLimits {
    /// The profile's sign-in, when it has one.
    pub sign_in: Option<SignIn>,
    /// Claude Code's last `/usage` reading, when it is this sign-in's.
    pub cache: Option<UsageCacheReading>,
}

/// The state file's shape: two ids and the usage cache, nothing else
/// (every other key, the account's email and any key field included, is
/// skipped while parsing and never kept).
#[derive(serde::Deserialize)]
struct StateShape {
    #[serde(rename = "oauthAccount", default)]
    oauth: Option<OauthIds>,
    #[serde(rename = "cachedUsageUtilization", default)]
    cache: Option<CacheShape>,
}

#[derive(serde::Deserialize)]
struct OauthIds {
    #[serde(rename = "accountUuid", default)]
    account: Option<String>,
    #[serde(rename = "organizationUuid", default)]
    org: Option<String>,
}

#[derive(serde::Deserialize)]
struct CacheShape {
    #[serde(rename = "fetchedAtMs", default)]
    fetched_at_ms: Option<serde_json::Number>,
    #[serde(rename = "accountUuid", default)]
    account: Option<String>,
    #[serde(default)]
    utilization: Option<UtilizationShape>,
}

#[derive(serde::Deserialize)]
struct UtilizationShape {
    #[serde(default)]
    five_hour: Option<WindowShape>,
    #[serde(default)]
    seven_day: Option<WindowShape>,
    #[serde(default)]
    limits: Option<Vec<LimitShape>>,
}

#[derive(serde::Deserialize)]
struct WindowShape {
    #[serde(default)]
    utilization: Option<serde_json::Value>,
    #[serde(default)]
    resets_at: Option<serde_json::Value>,
}

/// One row of the newer `limits[]` list: its kind (`session`,
/// `weekly_all`, `weekly_scoped`), percent and reset.
#[derive(serde::Deserialize)]
struct LimitShape {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    percent: Option<serde_json::Value>,
    #[serde(default)]
    resets_at: Option<serde_json::Value>,
}

/// The app's meter history: its samples, each read field by field.
#[derive(serde::Deserialize)]
struct History {
    #[serde(default)]
    samples: Vec<serde_json::Value>,
}

/// The sign-in and last `/usage` reading in a Claude Code state file's
/// bytes. `None` when it isn't a state file of that shape.
#[must_use]
pub fn profile_limits_from(bytes: &[u8]) -> Option<ProfileLimits> {
    let shape: StateShape = serde_json::from_slice(bytes).ok()?;
    let sign_in = shape.oauth.and_then(|ids| {
        Some(SignIn {
            account: ids.account.filter(|id| !id.is_empty())?,
            org: ids.org.filter(|id| !id.is_empty())?,
        })
    });
    let cache = shape.cache.and_then(|cache| {
        // Claude Code itself drops a reading made for another account.
        let own = sign_in.as_ref()?.account.as_str();
        if cache.account.as_deref() != Some(own) {
            return None;
        }
        let fetched_at = cache
            .fetched_at_ms
            .and_then(|n| n.as_f64())
            .filter(|ms| *ms > 0.0)?;
        #[allow(clippy::cast_possible_truncation)]
        let fetched_at = fetched_at as i64;
        let windows = cache.utilization?;
        let mut samples = Vec::new();
        let rows = windows.limits.unwrap_or_default();
        let row = |kind: &str| {
            rows.iter()
                .find(|row| row.kind.as_deref() == Some(kind))
                .map(|row| (row.percent.as_ref(), row.resets_at.as_ref()))
        };
        for (kind, window, fallback) in [
            (
                WindowKind::FiveHour,
                windows.five_hour.as_ref(),
                row("session"),
            ),
            (
                WindowKind::Weekly { anchor_ms: None },
                windows.seven_day.as_ref(),
                row("weekly_all"),
            ),
        ] {
            let (used, reset) = match window {
                Some(window) => (window.utilization.as_ref(), window.resets_at.as_ref()),
                None => fallback.unwrap_or((None, None)),
            };
            let Some(used) = used
                .and_then(serde_json::Value::as_f64)
                .filter(|v| v.is_finite())
            else {
                continue;
            };
            samples.push((
                kind,
                QuotaSample {
                    ts: fetched_at,
                    used_pct: used.clamp(0.0, 100.0),
                    resets_at: reset.and_then(crate::quota_ingest::reset_ms),
                    confidence: Confidence::Measured,
                },
            ));
        }
        Some(UsageCacheReading {
            fetched_at,
            samples,
        })
    });
    Some(ProfileLimits { sign_in, cache })
}

/// [`profile_limits_from`] for a profile folder's state file (the vendor's
/// default folder's is `~/.claude.json`). Streamed through the narrow
/// shape, size-capped; `None` when it can't be read.
#[must_use]
pub fn profile_limits(state_dir: &Path) -> Option<ProfileLimits> {
    let path = crate::claude_code::claude_state_file(state_dir);
    let meta = std::fs::symlink_metadata(&path).ok()?;
    if !meta.is_file() || meta.len() > STATE_FILE_MAX {
        return None;
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .ok()?
        .take(STATE_FILE_MAX)
        .read_to_end(&mut bytes)
        .ok()?;
    profile_limits_from(&bytes)
}

/// The profile's state file, for the caller's "has it changed" check.
#[must_use]
pub fn profile_state_file(state_dir: &Path) -> PathBuf {
    crate::claude_code::claude_state_file(state_dir)
}

/// The sign-ins an app instance's Code sessions were made under: the
/// names of its `claude-code-sessions/<account>/<org>/` folders (nothing
/// inside them is opened).
#[must_use]
pub fn instance_sign_ins(instance_dir: &Path) -> Vec<SignIn> {
    let dirs = |dir: &Path| -> Vec<String> {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                    .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    let root = instance_dir.join(APP_SESSIONS_DIR);
    let mut out: Vec<SignIn> = dirs(&root)
        .into_iter()
        .flat_map(|account| {
            dirs(&root.join(&account))
                .into_iter()
                .map(move |org| SignIn {
                    account: account.clone(),
                    org,
                })
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The meter samples in an app instance's history that are `sign_in`'s,
/// newer than `after_ms`, oldest first (at most [`APP_SAMPLES_MAX`] of
/// the newest), from the history's bytes and the instance's sign-ins.
///
/// They are `sign_in`'s only when the instance's Code sessions name
/// exactly one account in the sample's organisation, and it is
/// `sign_in.account`: an instance two accounts of one organisation used
/// can't say whose a sample is, so none counts.
#[must_use]
pub fn app_usage_samples_from(
    bytes: &[u8],
    instance_sign_ins: &[SignIn],
    sign_in: &SignIn,
    after_ms: i64,
) -> Vec<(WindowKind, QuotaSample)> {
    let accounts_in_org: Vec<&str> = instance_sign_ins
        .iter()
        .filter(|seen| seen.org == sign_in.org)
        .map(|seen| seen.account.as_str())
        .collect();
    if accounts_in_org != [sign_in.account.as_str()] {
        return Vec::new();
    }
    let Ok(history) = serde_json::from_slice::<History>(bytes) else {
        return Vec::new();
    };
    let skip = history.samples.len().saturating_sub(APP_SAMPLES_MAX);
    let mut out = Vec::new();
    for sample in history.samples.iter().skip(skip) {
        if sample.get("org").and_then(serde_json::Value::as_str) != Some(sign_in.org.as_str()) {
            continue;
        }
        #[allow(clippy::cast_possible_truncation)]
        let Some(ts) = sample
            .get("t")
            .and_then(serde_json::Value::as_f64)
            .filter(|t| t.is_finite() && *t > 0.0)
            .map(|t| t as i64)
        else {
            continue;
        };
        if ts <= after_ms {
            continue;
        }
        let used = sample.get("u");
        for (key, kind) in [
            ("fh", WindowKind::FiveHour),
            ("sd", WindowKind::Weekly { anchor_ms: None }),
        ] {
            let Some(pct) = used
                .and_then(|u| u.get(key))
                .and_then(serde_json::Value::as_f64)
                .filter(|pct| pct.is_finite())
            else {
                continue;
            };
            out.push((
                kind,
                QuotaSample {
                    ts,
                    used_pct: pct.clamp(0.0, 100.0),
                    resets_at: None,
                    confidence: Confidence::Measured,
                },
            ));
        }
    }
    out.sort_by_key(|(_, sample)| sample.ts);
    out
}

/// [`app_usage_samples_from`] for an instance folder: its history file,
/// size-capped, and its sign-ins. Nothing when either is missing.
#[must_use]
pub fn app_usage_samples(
    instance_dir: &Path,
    sign_in: &SignIn,
    after_ms: i64,
) -> Vec<(WindowKind, QuotaSample)> {
    let path = instance_dir.join(APP_USAGE_FILE);
    let Some(text) = juggler_adapter::live::read_small(&path, APP_USAGE_MAX) else {
        return Vec::new();
    };
    app_usage_samples_from(
        text.as_bytes(),
        &instance_sign_ins(instance_dir),
        sign_in,
        after_ms,
    )
}

/// A limit hit, from one transcript line: the window that ran out, used
/// up from the line's time until the vendor's reset. Only `error`,
/// `timestamp` and `quotaLimits`' (or `apiErrorParams.rate_limit_info`'s)
/// `status`, `rateLimitType` and `resetsAt` are read. A per-model weekly
/// limit (Opus, Sonnet) says nothing about the whole week, so it gives
/// nothing.
#[must_use]
pub fn limit_hit_sample(line: &serde_json::Value) -> Option<(WindowKind, QuotaSample)> {
    if line.get("error").and_then(serde_json::Value::as_str) != Some("rate_limit") {
        return None;
    }
    let info = line
        .get("quotaLimits")
        .filter(|v| v.is_object())
        .or_else(|| {
            line.get("apiErrorParams")
                .and_then(|params| params.get("rate_limit_info"))
                .filter(|v| v.is_object())
        })?;
    if info.get("status").and_then(serde_json::Value::as_str) != Some("rejected") {
        return None;
    }
    let kind = match info
        .get("rateLimitType")
        .and_then(serde_json::Value::as_str)?
    {
        "five_hour" => WindowKind::FiveHour,
        "seven_day" => WindowKind::Weekly { anchor_ms: None },
        _ => return None,
    };
    let resets_at = info
        .get("resetsAt")
        .and_then(crate::quota_ingest::reset_ms)?;
    let ts = line
        .get("timestamp")
        .and_then(serde_json::Value::as_str)
        .and_then(juggler_core::time::timestamp_ms)?;
    Some((
        kind,
        QuotaSample {
            ts,
            used_pct: 100.0,
            resets_at: Some(resets_at),
            confidence: Confidence::Measured,
        },
    ))
}

/// The limit hits in one transcript line's bytes: parsed only when the
/// line names the error.
fn limit_hit_in(line: &[u8]) -> Option<(WindowKind, QuotaSample)> {
    const NEEDLE: &[u8] = b"\"rate_limit\"";
    if !line.windows(NEEDLE.len()).any(|window| window == NEEDLE) {
        return None;
    }
    let value = serde_json::from_slice::<serde_json::Value>(line).ok()?;
    limit_hit_sample(&value)
}

/// The limit hits in a whole transcript, one line in memory at a time
/// (the one-time look back for hits written before they were kept).
#[must_use]
pub fn limit_hits_in(path: &Path) -> Vec<(WindowKind, QuotaSample)> {
    let mut out = Vec::new();
    let _ = crate::transcript::for_each_line(path, |_, line| {
        out.extend(limit_hit_in(line));
        true
    });
    out
}

/// D-39 (needs the maintainer's ruling; off until then): the arguments
/// that run Claude Code's own `/usage` headless. It is a local command (no
/// model call), keeps no transcript, starts no MCP server and runs no
/// hook. Claude Code asks Anthropic with its own sign-in (AI Juggler never
/// sees it) and leaves the answer in the profile's state file, where
/// [`profile_limits`] reads it.
#[must_use]
pub fn usage_probe_args() -> Vec<String> {
    [
        "-p",
        "/usage",
        "--no-session-persistence",
        "--strict-mcp-config",
        "--settings",
        r#"{"disableAllHooks":true}"#,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[cfg(test)]
mod tests;
