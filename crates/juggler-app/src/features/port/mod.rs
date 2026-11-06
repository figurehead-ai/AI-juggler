//! AJ-E505: the port occupancy map: who listens where, and which of them are
//! ours.

pub mod commands;
pub mod dto;

use std::sync::Arc;

use juggler_core::model::{MachineId, SessionId};

use crate::commands::{local_machine_id, now_millis};
use crate::features::cleanup::sweep::{setting_f64, setting_u64};
use crate::features::port::dto::{HogFactInfo, PortInventoryInfo, PortRowInfo, PortWarningInfo};
use crate::state::AppState;

/// Humanized age for the copy-exact warning: `45s` / `12m` / `3h` /
/// `2d`.
fn human_age(ms: i64) -> String {
    let secs = ms.div_euclid(1000).max(0);
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3_600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3_600)
    } else {
        format!("{}d", secs / 86_400)
    }
}

/// The real probe for THIS platform (query-time freshness; the tick
/// join stores the durable view).
pub(crate) fn os_listeners() -> Result<Vec<juggler_supervisor::Listener>, String> {
    use juggler_supervisor::PortProbe as _;
    #[cfg(unix)]
    let probe = juggler_supervisor::NetstatProbe;
    #[cfg(not(unix))]
    let probe = juggler_supervisor::UnsupportedProbe;
    probe.listeners().map_err(|error| error.to_string())
}

/// Every ledger row of the local machine's sessions (all states —
/// dead roots matter for classification).
/// EVERY ledger row, all machines (AJ-E511: the "All machines"
/// inventory scope — foreign rows are display-only in the UI). Walks
/// the composite keys, then reads each key — the same session on two
/// machines stays two groups.
pub(crate) fn all_ledger_rows(state: &Arc<AppState>) -> Vec<juggler_core::storage::LedgerRow> {
    use juggler_supervisor::tree::LedgerStore as _;
    let mut rows = Vec::new();
    for (session, machine) in state.ledger.live_session_machines().unwrap_or_default() {
        if let Ok(mut key_rows) = state
            .ledger
            .rows_for_session_on_machine(&SessionId::new(session), &MachineId::new(machine))
        {
            rows.append(&mut key_rows);
        }
    }
    rows
}

pub(crate) fn local_ledger_rows(state: &Arc<AppState>) -> Vec<juggler_core::storage::LedgerRow> {
    use juggler_supervisor::tree::LedgerStore as _;
    let machine = local_machine_id(state);
    let mut rows = Vec::new();
    let mut sessions = std::collections::HashSet::new();
    if let Ok(candidates) = state.ledger.candidates(&machine) {
        for row in candidates {
            sessions.insert(row.session_id.clone());
        }
    }
    for session in sessions {
        // COMPOSITE key (AJ-E511): a session may exist on two
        // machines — session-only reads would leak foreign rows into
        // this node's scope (the fleet-scale false-kill trap).
        if let Ok(mut session_rows) = state
            .ledger
            .rows_for_session_on_machine(&SessionId::new(session), &machine)
        {
            rows.append(&mut session_rows);
        }
    }
    rows
}

/// `port_inventory` sans-Tauri (tests + evidence runs).
pub fn port_inventory_inner(state: &Arc<AppState>) -> PortInventoryInfo {
    let now = now_millis();
    let listeners = match os_listeners() {
        Ok(listeners) => listeners,
        Err(note) => {
            return PortInventoryInfo {
                rows: Vec::new(),
                note: Some(note),
                hogs: Vec::new(),
            };
        }
    };
    let rows = local_ledger_rows(state);
    let adopted: std::collections::HashSet<String> = state
        .supervisor
        .alive_adopted_sessions()
        .into_iter()
        .map(|session| session.as_str().to_string())
        .collect();
    let inventory = juggler_supervisor::classify_listeners(&listeners, &rows, &adopted, now);
    PortInventoryInfo {
        rows: inventory
            .into_iter()
            .map(|row| PortRowInfo {
                port: row.port,
                pid: row.pid,
                class: row.class.as_str().to_string(),
                session: row.session,
                provider: row.provider,
                account: row.account,
                project: row.project,
                listener_age_ms: row.listener_age_ms,
                name: row.name,
                hint: row.hint.map(str::to_string),
                killable: row.class != juggler_supervisor::ListenerClass::NotManaged,
            })
            .collect(),
        note: state
            .resources
            .lock()
            .ok()
            .and_then(|collector| collector.ports_note()),
        hogs: hog_facts(state, now),
    }
}

/// Sustained-breach hog facts (Req.5): a helper >2 GiB, a tree >90%
/// of one core, or a tree above its Provider's learned p95 — each
/// sustained over the 60 s window of the collector's rings. The p95
/// rule needs ≥3 Provider trees to compare against (an honest floor;
/// smaller populations never fire it).
#[allow(clippy::similar_names, reason = "state/stats read naturally here")]
pub(crate) fn hog_facts(state: &Arc<AppState>, now_ms: i64) -> Vec<HogFactInfo> {
    const HOG_WINDOW_MS: i64 = 60_000;
    // AJ-E512: the thresholds are settings-backed (defaults per SPEC
    // §10) so tests can lower them without faking gigabytes; the
    // production values never change here.
    let helper_rss_limit = setting_u64(state, "hog_helper_rss_limit_mb", 2_048) * 1024 * 1024;
    let cpu_pct_limit = setting_f64(state, "hog_cpu_pct", 90.0);
    let Some(collector) = state.resources.lock().ok() else {
        return Vec::new();
    };
    let stats = collector.stats_snapshot();
    let contexts: Vec<(SessionId, String)> = state
        .supervisor
        .spawn_contexts_snapshot()
        .into_iter()
        .map(|(session, ctx)| (session, ctx.provider.clone()))
        .collect();
    let mut facts = Vec::new();
    for (session, provider) in &contexts {
        let Some(tree_stats) = stats.get(session) else {
            continue;
        };
        // Sustained = every ring sample in the window breaches. The
        // window is the last minute of samples the tree HAS: a finished
        // tree's ring stops at root death (its rows freeze), and
        // anchoring at the wall clock would age a hog out after a
        // minute and let the sweep signal it (AJ-E512: never). A live
        // tree's newest sample is ~now, so nothing changes for it.
        let ring = collector.ring(session);
        let anchor = ring
            .last()
            .map_or(now_ms, |sample| sample.at_ms.min(now_ms));
        let in_window: Vec<_> = ring
            .iter()
            .filter(|sample| anchor - sample.at_ms <= HOG_WINDOW_MS)
            .collect();
        if in_window.len() < 2 {
            continue; // not enough samples to call anything sustained
        }
        let cpu_sustained = in_window
            .iter()
            .all(|sample| sample.cpu_pct.is_some_and(|cpu| cpu > cpu_pct_limit));
        if cpu_sustained {
            facts.push(HogFactInfo {
                pid: 0, // tree-level: the Cleanup Centre resolves the
                // Instance's live root pid at render time
                session: session.as_str().to_string(),
                provider: provider.clone(),
                rss_bytes: tree_stats.rss_bytes,
                cpu_pct: tree_stats.cpu_pct,
                rule: "tree-cpu>90%".to_string(),
            });
        }
        // The per-Provider p95 population: the OTHER trees of this
        // Provider (self-excluded), needing at least three peers.
        let peers: Vec<u64> = contexts
            .iter()
            .filter(|(peer, peer_provider)| peer_provider == provider && peer != session)
            .filter_map(|(peer, _)| stats.get(peer).map(|s| s.rss_bytes))
            .collect();
        if peers.len() >= 3 {
            let mut sorted = peers.clone();
            sorted.sort_unstable();
            let p95 = sorted[sorted.len() * 95 / 100];
            let rss_sustained = in_window.iter().all(|sample| sample.rss_bytes > p95);
            if rss_sustained && tree_stats.rss_bytes > p95 {
                facts.push(HogFactInfo {
                    pid: 0,
                    session: session.as_str().to_string(),
                    provider: provider.clone(),
                    rss_bytes: tree_stats.rss_bytes,
                    cpu_pct: tree_stats.cpu_pct,
                    rule: "tree-rss>p95(provider)".to_string(),
                });
            }
        }
        // Single-helper RSS: the live rows of the tree (root excluded).
        for row in collector.rows(session).into_iter().flatten().skip(1) {
            if row.rss_bytes > helper_rss_limit {
                facts.push(HogFactInfo {
                    pid: row.pid,
                    session: session.as_str().to_string(),
                    provider: provider.clone(),
                    rss_bytes: row.rss_bytes,
                    cpu_pct: row.cpu_pct,
                    rule: "helper-rss>2GiB".to_string(),
                });
            }
        }
    }
    facts
}

/// Test seam: the hog facts as of `now_ms` (AJ-E512 regression: a
/// finished tree stays a hog however long after it froze).
#[doc(hidden)]
#[must_use]
pub fn hog_sessions_at_for_test(state: &Arc<AppState>, now_ms: i64) -> Vec<String> {
    hog_facts(state, now_ms)
        .into_iter()
        .map(|hog| hog.session)
        .collect()
}

/// `launch_port_warning` sans-Tauri: the learned expected ports of
/// `project_id`'s instances (their observed `listens_on` history,
/// any row state) crossed with CURRENT orphan-held listeners.
#[allow(
    clippy::needless_pass_by_value,
    reason = "its signature as it was in commands.rs; AJ-E2017 moved it unchanged"
)]
pub fn launch_port_warning_inner(
    state: &Arc<AppState>,
    project_id: Option<String>,
) -> Option<PortWarningInfo> {
    let now = now_millis();
    let expected: std::collections::HashSet<u16> = local_ledger_rows(state)
        .into_iter()
        .filter(|row| {
            project_id
                .as_deref()
                .is_none_or(|project| row.project_id.as_deref() == Some(project))
        })
        .filter_map(|row| row.listens_on)
        .flat_map(|ports| {
            ports
                .split(',')
                .filter_map(|port| port.parse::<u16>().ok())
                .collect::<Vec<_>>()
        })
        .collect();
    if expected.is_empty() {
        return None;
    }
    let listeners = os_listeners().ok()?;
    let rows = local_ledger_rows(state);
    let adopted: std::collections::HashSet<String> = state
        .supervisor
        .alive_adopted_sessions()
        .into_iter()
        .map(|session| session.as_str().to_string())
        .collect();
    let inventory = juggler_supervisor::classify_listeners(&listeners, &rows, &adopted, now);
    inventory
        .into_iter()
        .find(|row| {
            expected.contains(&row.port)
                && row.class == juggler_supervisor::ListenerClass::ManagedOrphan
        })
        .map(|row| {
            let row_port = row.port;
            let instance = format!(
                "{} ({})",
                row.provider.as_deref().unwrap_or("instance"),
                row.session
                    .as_deref()
                    .map(str::chars)
                    .map(|mut chars| chars.by_ref().take(8).collect::<String>())
                    .unwrap_or_default()
            );
            let age_ms = row.listener_age_ms.unwrap_or_default();
            let message = format!(
                ":{} is already held by a process left running from {instance} ({}).",
                row_port,
                human_age(age_ms)
            );
            PortWarningInfo {
                port: row_port,
                pid: row.pid,
                instance,
                age_ms,
                message,
            }
        })
}

/// The ports a pid holds RIGHT NOW (the audit's before/after facts).
pub(crate) fn ports_held_by(pid: u32) -> Option<String> {
    let listeners = os_listeners().ok()?;
    let ports: Vec<String> = listeners
        .iter()
        .filter(|listener| listener.pid == pid)
        .map(|listener| listener.port.to_string())
        .collect();
    (!ports.is_empty()).then(|| ports.join(","))
}
