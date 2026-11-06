//! AJ-E512 Req.3: the Cleanup Centre's passive alerts (hogs and idle
//! orphans).

use std::sync::Arc;

use juggler_core::model::SessionId;

use crate::commands::{local_machine_id, now_millis};
use crate::features::cleanup::dto::{CleanupAlertInfo, CleanupAlertsInfo};
use crate::features::cleanup::sweep::setting_u64;
use crate::features::port::{hog_facts, local_ledger_rows};
use crate::state::AppState;

/// Formats bytes the way the alert copy needs (GiB one decimal, MiB
/// whole).
#[must_use]
pub fn rss_text(bytes: u64) -> String {
    #[allow(clippy::cast_precision_loss, reason = "display rounding, not math")]
    let gib = bytes as f64 / 1_073_741_824.0;
    if bytes >= 1024 * 1024 * 1024 {
        format!("{gib:.1} GiB")
    } else {
        format!("{} MiB", bytes / (1024 * 1024))
    }
}

/// Formats an age the way the nudge copy needs.
fn age_text(ms: i64) -> String {
    let mins = ms / 60_000;
    if mins < 60 {
        format!("{mins}m")
    } else {
        format!("{}h", mins / 60)
    }
}

/// Computes the passive alerts (AJ-E512 Req.3): for every hog-flagged
/// tree, ONE alert naming its TOP-RSS helper (NN/g: name the helper,
/// not just the Instance), and — once per orphan — an idle nudge
/// (CPU≈0 + age only). Purely passive: no alert carries a signal
/// path; the hog alert's pid routes to the STAGED E507 sheet
/// (app prepares, user executes).
pub fn cleanup_alerts_inner(state: &Arc<AppState>) -> CleanupAlertsInfo {
    let now = now_millis();
    let machine = local_machine_id(state);
    let mut alerts = Vec::new();

    // Hog alerts: top-RSS live helper per hog-flagged session.
    let facts = hog_facts(state, now);
    let mut hog_sessions: Vec<String> = facts.iter().map(|f| f.session.clone()).collect();
    hog_sessions.sort();
    hog_sessions.dedup();
    if !hog_sessions.is_empty() {
        let Some(collector) = state.resources.lock().ok() else {
            return CleanupAlertsInfo { alerts };
        };
        for session in hog_sessions {
            let top = collector
                .rows(&SessionId::new(session.clone()))
                .into_iter()
                .flatten()
                .skip(1) // the root
                .max_by_key(|row| row.rss_bytes);
            let Some(top) = top else { continue };
            let helper = top.name.rsplit('/').next().unwrap_or(&top.name).to_string();
            let instance = instance_label_for(state, &session);
            crate::usage_stats::resource_alert_shown("memory_hog", &session, top.pid);
            alerts.push(CleanupAlertInfo {
                kind: "hog".into(),
                text: format!(
                    "{helper} under {instance} holds {} — review it in Processes.",
                    rss_text(top.rss_bytes)
                ),
                pid: top.pid,
                session: session.clone(),
            });
        }
    }

    // Idle nudges: orphaned helpers, CPU≈0, aged past the threshold —
    // fired at most ONCE per orphan (persisted flag).
    let min_age = setting_u64(state, "idle_nudge_min_age_ms", 3_600_000).cast_signed();
    for row in local_ledger_rows(state) {
        if row.state != "orphaned" || row.machine_id != machine.as_str() || row.role != "helper" {
            continue;
        }
        let Some(pid) = row.pid else { continue };
        // Its CPU now (the session's rows froze when its root exited); not
        // known yet, or gone, is never idle.
        let cpu_ok = state
            .resources
            .lock()
            .is_ok_and(|collector| collector.cpu_now(pid).is_some_and(|cpu| cpu < 1.0));
        if !cpu_ok {
            continue;
        }
        let age = now - row.started_at.map(|s| s * 1000).unwrap_or_default();
        if age < min_age {
            continue;
        }
        let flag = format!("idle_nudge:{}:{pid}", row.session_id);
        let already = state
            .storage
            .lock()
            .ok()
            .and_then(|storage| storage.app_settings().get(&flag).ok().flatten())
            .is_some();
        if already {
            continue;
        }
        if let Ok(storage) = state.storage.lock() {
            let _ = storage.app_settings().set(&flag, "1");
        }
        let helper = row
            .exe_path
            .rsplit('/')
            .next()
            .unwrap_or(&row.exe_path)
            .to_string();
        crate::usage_stats::resource_alert_shown("idle", &row.session_id, pid);
        alerts.push(CleanupAlertInfo {
            kind: "idle".into(),
            text: format!(
                "{helper} has been idle for {} — review it in Processes.",
                age_text(age)
            ),
            pid,
            session: row.session_id.clone(),
        });
    }
    CleanupAlertsInfo { alerts }
}

/// The Instance's display label (account label → provider → session).
fn instance_label_for(state: &Arc<AppState>, session: &str) -> String {
    let lookup = (|| {
        let storage = state.storage.lock().ok()?;
        let record = storage
            .sessions()
            .get(&SessionId::new(session.to_string()))
            .ok()?;
        let recipe = storage.profiles().get(&record.profile).ok()?;
        let account = storage.accounts().get(&recipe.account).ok()?;
        // The App whole, as every screen names it ("Claude Code · Work
        // Max"; the 0.4 review): a bare folder name is two Apps' name
        // when an app profile shares it.
        Some(state.registry.get(&account.harness).map_or_else(
            || account.label.clone(),
            |adapter| format!("{} · {}", adapter.spec().display_name, account.label),
        ))
    })();
    lookup.unwrap_or_else(|| "its session".to_string())
}
