//! AJ-E611: the quota alerts: each account's five-hour and weekly alert
//! machines, stepped on every tick, and the digest and mutes they keep.

pub mod commands;
pub mod dto;

use crate::features::alerts::dto::AlertEventInfo;
use crate::features::quota::alert_words::SteppedAlert;
use crate::features::quota::{alert_words, quota_display_inner};

/// The per-(account, window) persisted FSM states (`alert_fsms` KV).
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct AlertFsmState {
    #[serde(default)]
    five_hour: juggler_core::alerts::AlertFsm,
    #[serde(default)]
    weekly: juggler_core::alerts::AlertFsm,
}

const ALERT_FSMS_KEY: &str = "alert_fsms";

const ALERT_DIGEST_KEY: &str = "alert_digest";

pub(crate) const ALERT_MUTES_KEY: &str = "alert_mutes";

/// A window's reading's confidence, from its badge (no badge: no
/// percentage, so nothing measured).
pub(crate) fn badge_confidence(badge: Option<&str>) -> juggler_core::model::Confidence {
    match badge {
        Some("measured") => juggler_core::model::Confidence::Measured,
        Some("computed") => juggler_core::model::Confidence::Computed,
        _ => juggler_core::model::Confidence::Estimated,
    }
}

/// AJ-E611: one account's two window FSMs + its renewal-approaching
/// digest line (Req.5 — the subscription clock NEVER steps a window
/// machine). Pure with respect to Provider surfaces; mutates only the
/// persisted machine entry and the digest buffer.
fn step_account_alerts(
    storage: &juggler_core::storage::Storage,
    account: &juggler_core::model::Account,
    now: i64,
    entry: &mut AlertFsmState,
    digest: &mut juggler_core::alerts::Digest,
) -> Vec<SteppedAlert> {
    use juggler_core::alerts::{AlertEmission, WindowObservation};

    let mut out = Vec::new();
    let id = account.id.as_str().to_owned();
    // D-62: one renewal line per Account, however many folders are on it.
    let owner = |key: &str| storage.terms().owner(key).ok().flatten();
    let mine = owner(&id);
    let display = quota_display_inner(storage, account, now);
    // One buffered line per distinct copy — the label changes as the
    // date nears, which is the natural dedupe key.
    if display.renewal.state == "approaching"
        && !digest.entries.iter().any(|line| {
            line.text == display.renewal.label
                && (line.key == id || (mine.is_some() && owner(&line.key) == mine))
        })
    {
        *digest = juggler_core::alerts::digest_add(digest, &id, &display.renewal.label);
        crate::usage_stats::count("renewal.warning_shown", &[]);
    }
    for (slot, window) in [
        ("five_hour", &display.five_hour),
        ("weekly", &display.weekly),
    ] {
        let Some(window) = window else { continue };
        // AJ-E2904: the window's own reading: the user's levels fire on a
        // measured or computed one, and an estimate's words say "about".
        let confidence = badge_confidence(window.badge.as_deref());
        let observation = WindowObservation {
            // AJ-E2904: the user's first and last warnings.
            levels: crate::settings_policy::quota_levels(storage),
            tte_min: display.forecast.tte_min,
            used_pct: window.percentage,
            confidence,
            resets_at_ms: window.resets_at_ms,
        };
        let machine = if slot == "five_hour" {
            &entry.five_hour
        } else {
            &entry.weekly
        };
        let (next, emission) = juggler_core::alerts::step(machine, observation, now);
        if slot == "five_hour" {
            entry.five_hour = next;
        } else {
            entry.weekly = next;
        }
        let kind = match emission {
            AlertEmission::TrayAmber { .. } => Some("tray-amber"),
            AlertEmission::Actionable { .. } => Some("actionable"),
            AlertEmission::ExhaustedNotification => Some("exhausted"),
            AlertEmission::None | AlertEmission::ResetReached => None,
        };
        if let Some(kind) = kind {
            crate::usage_stats::quota_alert(account.harness.as_str(), slot, kind);
            out.push(SteppedAlert::Quota {
                kind,
                folder: id.clone(),
                quota: alert_words::AlertQuotaInfo {
                    account_label: account.label.clone(),
                    window: slot.to_owned(),
                    left_pct: window.percentage.map(|used| (100.0 - used).max(0.0)),
                    estimate: window.percentage.is_some()
                        && confidence == juggler_core::model::Confidence::Estimated,
                    minutes_left: display.forecast.tte_min,
                    resets_at_ms: window.resets_at_ms,
                },
                credits: display.credits.clone(),
            });
        }
    }
    out
}

/// AJ-E611 Req.4: one alert evaluation pass — steps every account's
/// two window FSMs from the engine's current readings, emits AT MOST
/// the §Q5 notifications, and persists the machines. Non-actionable
/// crossings (Req.5: renewal `approaching`) batch into the hourly
/// digest; quiet hours defer without dropping. Passive by
/// construction: nothing here launches, signals, or writes any
/// Provider surface (the guardrail test pins it). `local_hour` comes
/// from the polling UI (the webview knows wall-clock local time; the
/// backend carries no tz database) — `None` defers the flush.
///
/// D-62: the machines step per folder, but the words name the folder's
/// Account (its folders share its limits); the Accounts are read only
/// when there is an alert to word.
#[must_use]
pub fn alerts_tick_inner(
    state: &std::sync::Arc<crate::state::AppState>,
    local_hour: Option<u32>,
) -> Vec<AlertEventInfo> {
    let now = juggler_core::model::Timestamp::now().millis();
    alert_words::word_all(state, step_alerts(state, now, local_hour), now)
}

/// [`alerts_tick_inner`]'s steps, under the storage lock.
fn step_alerts(
    state: &std::sync::Arc<crate::state::AppState>,
    now: i64,
    local_hour: Option<u32>,
) -> Vec<SteppedAlert> {
    let mut out = Vec::new();
    let Ok(storage) = state.storage.lock() else {
        return out;
    };
    let accounts = storage.accounts().list().unwrap_or_default();
    let mut machines: std::collections::BTreeMap<String, AlertFsmState> = storage
        .app_settings()
        .get(ALERT_FSMS_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default();
    let mut digest: juggler_core::alerts::Digest = storage
        .app_settings()
        .get(ALERT_DIGEST_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default();

    for account in &accounts {
        let entry = machines.entry(account.id.as_str().to_owned()).or_default();
        out.extend(step_account_alerts(
            &storage,
            account,
            now,
            entry,
            &mut digest,
        ));
    }
    if let Ok(blob) = serde_json::to_string(&machines) {
        let _ = storage.app_settings().set(ALERT_FSMS_KEY, &blob);
    }

    out.extend(queue_and_digest_events(&storage, now, local_hour, digest));
    out
}

/// AJ-E611 Req.4/6 tail of the alert pass: reset-queue notifications
/// (due items notify ONCE — `notified`; broken lineage goes `stale`,
/// explained) plus the hourly digest flush (muted keys drop; quiet
/// hours — a missing local hour counts as quiet — defer without
/// dropping). The next digest state is persisted here; the caller
/// receives only the events.
#[allow(
    clippy::needless_pass_by_value,
    reason = "its signature as it was in commands.rs; AJ-E2017 moved it unchanged"
)]
fn queue_and_digest_events(
    storage: &juggler_core::storage::Storage,
    now: i64,
    local_hour: Option<u32>,
    digest: juggler_core::alerts::Digest,
) -> Vec<SteppedAlert> {
    let mut out = Vec::new();
    let session_ids: Vec<String> = storage
        .sessions()
        .list(None)
        .unwrap_or_default()
        .iter()
        .map(|record| record.id.as_str().to_owned())
        .collect();
    let usage = storage.usage();
    for (id, session, account, _resume_at) in usage.pending_resumes(now).unwrap_or_default() {
        // 0.3.2: another app's session (`conv:…`) has no lineage here.
        if !session.starts_with("conv:") && !session_ids.contains(&session) {
            let _ = usage.set_reset_state(&id, "stale");
            continue;
        }
        out.push(SteppedAlert::ResetReached {
            own: alert_words::folder_label(storage, &account),
            place: alert_words::session_place(storage, &session),
            folder: account,
        });
        let _ = usage.set_reset_state(&id, "notified");
    }

    let muted: Vec<String> = storage
        .app_settings()
        .get(ALERT_MUTES_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default();
    let quiet_hours = crate::settings_policy::quiet_hours(storage);
    let quiet_hour = local_hour.is_none_or(|hour| quiet_hours.contains(hour));
    let (next_digest, flushed) = if quiet_hour {
        (digest.clone(), Vec::new())
    } else {
        juggler_core::alerts::digest_flush_with(
            &digest,
            now,
            &muted,
            local_hour.unwrap_or(12),
            quiet_hours,
        )
    };
    for entry in flushed {
        out.push(SteppedAlert::Worded(AlertEventInfo {
            kind: "digest".to_owned(),
            account: entry.key.clone(),
            text: entry.text,
            at_ms: now,
            quota: None,
        }));
    }
    if let Ok(blob) = serde_json::to_string(&next_digest) {
        let _ = storage.app_settings().set(ALERT_DIGEST_KEY, &blob);
    }
    out
}
