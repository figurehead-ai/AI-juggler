//! The plan windows Hermes Agent reports for an account (AJ-E1607 Req.2):
//! `hermes usage --json`, Hermes' own command, run under the account's
//! isolation on the usage cadence or "Measure now" only.
//!
//! What it prints is documented and its keys are stable
//! (`website/docs/reference/cli-commands.md` § `hermes usage`;
//! `hermes_cli/subcommands/usage.py`, `usage_snapshot_document`, at
//! `NousResearch/hermes-agent@422ad186`): `provider`, `source`, `title`,
//! `plan`, `fetched_at` (ISO 8601), `windows` (`label`, `used_percent`,
//! `resets_at`, `detail`), `details`, `unavailable_reason`. It exits 1
//! with nothing on stdout when no credential is configured for the
//! account's provider, the provider has no usage endpoint, or the fetch
//! failed. It runs without a session or a gateway and, per its docs,
//! never adds or refreshes a credential it wouldn't use for chat.
//!
//! The windows it names come from the provider the account is set up for:
//! Codex (`Session` and `Weekly`, labelled by their length),
//! Anthropic (`Current session`, `Current week`, and per-model weeks) and
//! `OpenRouter` (credits). A Nous Portal account has no `hermes usage`
//! reader in that source (`agent/account_usage.py`, `_USAGE_FETCHERS`):
//! Hermes shows its credits in a session's `/usage` only, so such an
//! account has no windows here and says so. Only the 5-hour and the week
//! of the whole account are kept; a model's own week, or a window AI
//! Juggler can't name, isn't guessed at. Each is a reading as of
//! `fetched_at`, labelled as Hermes' report (the capability
//! `LIMIT_WINDOWS_REPORTED`), never as the provider's own feed for a Codex
//! or Claude account AI Juggler manages.

use std::path::Path;
use std::time::{Duration, Instant};

use juggler_adapter::EnvMap;
use juggler_adapter::plan_usage::PlanReading;
use juggler_core::model::Confidence;
use juggler_core::quota::{QuotaSample, WindowKind};
use serde_json::Value;

/// A whole check stops past this.
const DEADLINE: Duration = Duration::from_secs(20);
/// The most of its output read.
const MAX_OUTPUT: u64 = 64 * 1024;

/// The window a label names: the whole account's 5-hour session or its
/// week, as Hermes labels them for Codex and Anthropic.
fn window_of(label: &str) -> Option<WindowKind> {
    match label {
        "Session" | "Current session" => Some(WindowKind::FiveHour),
        "Weekly" | "Current week" => Some(WindowKind::Weekly { anchor_ms: None }),
        _ => None,
    }
}

/// `hermes usage --json`'s document as readings.
///
/// # Errors
///
/// A plain message (never a secret) when Hermes says the reading is
/// unavailable, or the document isn't in the shape read.
pub fn reading_of(document: &Value) -> Result<PlanReading, String> {
    if let Some(why) = document
        .get("unavailable_reason")
        .and_then(Value::as_str)
        .filter(|why| !why.is_empty())
    {
        return Err(format!("Hermes Agent has no usage for this account: {why}"));
    }
    let changed = || "Hermes Agent's usage isn't in the shape AI Juggler reads".to_owned();
    let fetched = document
        .get("fetched_at")
        .and_then(Value::as_str)
        .and_then(juggler_core::time::timestamp_ms)
        .ok_or_else(changed)?;
    let windows = document
        .get("windows")
        .and_then(Value::as_array)
        .ok_or_else(changed)?;
    let mut reading = PlanReading {
        plan: document
            .get("plan")
            .and_then(Value::as_str)
            .filter(|plan| !plan.is_empty())
            .map(str::to_owned),
        ..PlanReading::default()
    };
    for window in windows {
        let Some(kind) = window
            .get("label")
            .and_then(Value::as_str)
            .and_then(window_of)
        else {
            continue;
        };
        // `used_percent` is null when the provider didn't report it: no
        // reading, never a zero.
        let Some(used) = window
            .get("used_percent")
            .and_then(Value::as_f64)
            .filter(|used| *used >= 0.0)
        else {
            continue;
        };
        reading.windows.push((
            kind,
            QuotaSample {
                ts: fetched,
                used_pct: used,
                resets_at: window
                    .get("resets_at")
                    .and_then(Value::as_str)
                    .and_then(juggler_core::time::timestamp_ms),
                confidence: Confidence::Measured,
                held_until: None,
            },
        ));
    }
    Ok(reading)
}

/// The account's plan windows from `hermes usage --json`, run with the
/// account's own environment `env` in its home `home`.
///
/// # Errors
///
/// A plain message when it can't run, fails (no sign-in for its provider,
/// or a provider without usage), runs too long, or prints something else.
pub fn usage(bin: &Path, home: &Path, env: &EnvMap) -> Result<PlanReading, String> {
    let mut command = juggler_adapter::process::helper(bin);
    command.args(["usage", "--json"]).current_dir(home);
    for (key, value) in env {
        if value != juggler_adapter::launch::ENV_UNSET {
            command.env(key, value);
        }
    }
    let out =
        crate::common::child::output_within(&mut command, Instant::now() + DEADLINE, MAX_OUTPUT)
            .map_err(|_| "`hermes usage` couldn't be started".to_owned())?;
    match out.status {
        Some(status) if status.success() => {}
        Some(_) => {
            return Err(
                "`hermes usage` has nothing for this account (no sign-in for its provider, \
                 or a provider Hermes can't read usage from, such as Nous Portal)"
                    .into(),
            );
        }
        None => return Err("`hermes usage` didn't answer in time".into()),
    }
    let document: Value = serde_json::from_str(out.stdout.trim()).map_err(|_| {
        "`hermes usage --json` printed something AI Juggler doesn't read".to_owned()
    })?;
    reading_of(&document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The documented example (cli-commands.md § `hermes usage`).
    fn documented() -> Value {
        json!({
            "provider": "openai-codex",
            "source": "usage_api",
            "title": "Account limits",
            "plan": "Plus",
            "fetched_at": "2026-09-19T07:58:55+00:00",
            "windows": [
                {"label": "Session", "used_percent": 37.0, "resets_at": "2026-09-19T21:00:00+00:00", "detail": null},
                {"label": "Weekly", "used_percent": 12.5, "resets_at": "2026-09-25T09:00:00+00:00", "detail": null}
            ],
            "details": ["You have 1 reset banked - use /usage reset to activate"],
            "unavailable_reason": null
        })
    }

    #[test]
    fn the_documented_document_reads_as_two_windows_as_of_its_fetch() {
        let reading = reading_of(&documented()).unwrap();
        assert_eq!(reading.plan.as_deref(), Some("Plus"));
        assert_eq!(reading.windows.len(), 2);
        let (kind, session) = reading.windows[0];
        assert_eq!(kind, WindowKind::FiveHour);
        assert_eq!(
            session.ts,
            juggler_core::time::timestamp_ms("2026-09-19T07:58:55Z").unwrap(),
            "as of when Hermes fetched it"
        );
        assert!((session.used_pct - 37.0).abs() < f64::EPSILON);
        assert_eq!(
            session.resets_at,
            juggler_core::time::timestamp_ms("2026-09-19T21:00:00Z")
        );
        assert_eq!(reading.windows[1].0, WindowKind::Weekly { anchor_ms: None });
    }

    #[test]
    fn a_models_own_week_an_unreported_window_and_an_unknown_one_are_left_out() {
        let document = json!({
            "provider": "anthropic", "plan": null,
            "fetched_at": "2026-11-13T10:00:00.123456+00:00",
            "windows": [
                {"label": "Current session", "used_percent": 5.0, "resets_at": null, "detail": null},
                {"label": "Opus week", "used_percent": 90.0, "resets_at": null, "detail": null},
                {"label": "Current week", "used_percent": null, "resets_at": null, "detail": null},
                {"label": "Monthly-ish", "used_percent": 1.0, "resets_at": null, "detail": null}
            ],
            "unavailable_reason": null
        });
        let reading = reading_of(&document).unwrap();
        assert_eq!(reading.plan, None);
        assert_eq!(reading.windows.len(), 1);
        assert_eq!(reading.windows[0].0, WindowKind::FiveHour);
        assert_eq!(reading.windows[0].1.resets_at, None);
    }

    #[test]
    fn unavailable_and_a_changed_shape_are_errors_never_zeros() {
        let unavailable = json!({
            "provider": "anthropic", "fetched_at": "2026-11-13T10:00:00+00:00", "windows": [],
            "unavailable_reason": "Anthropic account limits are only available for OAuth-backed Claude accounts."
        });
        assert!(
            reading_of(&unavailable)
                .unwrap_err()
                .contains("OAuth-backed")
        );
        assert!(
            reading_of(&json!({"windows": []})).is_err(),
            "no fetched_at"
        );
        assert!(
            reading_of(&json!({"fetched_at": "2026-11-13T10:00:00+00:00"})).is_err(),
            "no windows"
        );
    }
}
