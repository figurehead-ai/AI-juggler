//! AJ-E506: the Cleanup Centre's inventory (read-only, Tier 0) and its audit
//! export.

use std::sync::Arc;

use juggler_core::model::{AccountId, SessionId};

use crate::commands::{local_machine_id, lock_storage_arc, now_millis};
use crate::features::cleanup::dto::{
    CleanupGroup, CleanupHistoryInfo, CleanupInventoryInfo, CleanupRow, EvidenceChain,
    VerificationBadge,
};
use crate::features::kill::{guess_role, role_words};
use crate::features::port::dto::HogFactInfo;
use crate::features::port::{all_ledger_rows, hog_facts, local_ledger_rows};
use crate::features::sessions::read_model::label_of;
use crate::state::AppState;

/// What a Cleanup group's title is made of, read from storage once per
/// inventory (names, not ids).
#[derive(Default)]
struct GroupNames {
    account_label: Option<String>,
    project_name: Option<String>,
    label: Option<String>,
    status: Option<String>,
    started_at_ms: Option<i64>,
}

/// The names behind each session's group (one storage lock for all).
fn group_names(
    state: &Arc<AppState>,
    keys: &[(String, Option<String>, Option<String>)],
) -> std::collections::HashMap<String, GroupNames> {
    let mut out = std::collections::HashMap::new();
    let Ok(storage) = lock_storage_arc(state) else {
        return out;
    };
    let marks = storage.sessions().marks().unwrap_or_default();
    for (session, account, project) in keys {
        let record = storage
            .sessions()
            .get(&SessionId::new(session.clone()))
            .ok();
        let project = project.clone().or_else(|| {
            record
                .as_ref()
                .and_then(|r| r.project.as_ref().map(ToString::to_string))
        });
        out.insert(
            session.clone(),
            GroupNames {
                account_label: account.as_ref().and_then(|id| {
                    storage
                        .accounts()
                        .get(&AccountId::new(id.clone()))
                        .ok()
                        .map(|account| account.label)
                }),
                project_name: project.and_then(|id| {
                    storage
                        .projects()
                        .get(&juggler_core::model::ProjectId::new(id))
                        .ok()
                        .map(|project| project.name)
                }),
                label: label_of(&marks, session),
                status: record.as_ref().map(|r| r.status.as_str().to_string()),
                started_at_ms: record.as_ref().map(|r| r.started_at.millis()),
            },
        );
    }
    out
}

/// `pass2` §2 state labels, VERBATIM (Req.2).
#[must_use]
pub fn cleanup_state_label(state: &str) -> &'static str {
    match state {
        "orphaned" => "Still running after its session closed",
        "re-adopted" | "adopted" => "Running — picked up after restart",
        "unverifiable" => "Unverifiable — will not be touched",
        "running" => "Running",
        "exited" => "Exited",
        "starting" => "Starting",
        _ => "Unknown",
    }
}

/// `cleanup_inventory` sans-Tauri (tests + dogfood evidence).
#[allow(clippy::similar_names, reason = "row/root_row read naturally here")]
#[allow(
    clippy::too_many_lines,
    reason = "one pass per filter; splitting hides the grouping rules"
)]
pub fn cleanup_inventory_inner(
    state: &Arc<AppState>,
    filter: &str,
    all_machines: bool,
) -> CleanupInventoryInfo {
    let now = now_millis();
    let degraded = state.supervisor.ledger_degraded();
    // History (7 days) reads the AUDIT LOG (AJ-E509 Req.4) — the
    // outcome vocabulary is one language with AJ-E507's.
    if filter == "history" {
        let since = now - 7 * 24 * 3_600_000;
        let rows = state.audit.history(since).unwrap_or_default();
        let empty_note = rows
            .is_empty()
            .then(|| "No cleanups yet — automatic cleanup is off by default.".to_string());
        return CleanupInventoryInfo {
            groups: Vec::new(),
            local_machine: local_machine_id(state).as_str().to_string(),
            clean_instances: 0,
            ledger_degraded: degraded,
            history: Some(CleanupHistoryInfo { rows, empty_note }),
        };
    }
    let adopted: std::collections::HashSet<String> = state
        .supervisor
        .alive_adopted_sessions()
        .into_iter()
        .map(|session| session.as_str().to_string())
        .collect();
    let hogs = hog_facts(state, now);
    let live_rows: std::collections::HashMap<u32, juggler_supervisor::ProcessRow> = state
        .resources
        .lock()
        .ok()
        .map(|collector| {
            collector
                .stats_snapshot()
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .flat_map(|session| {
                    collector
                        .rows(&session)
                        .into_iter()
                        .flatten()
                        .map(|row| (row.pid, row))
                        .collect::<Vec<_>>()
                })
                .collect()
        })
        .unwrap_or_default();
    let table = state.supervisor.snapshot_table();

    let mut groups: Vec<CleanupGroup> = Vec::new();
    let mut clean = 0_usize;
    // Group rows per (session, machine) — the D-13 composite key: the
    // same session may exist on two machines and MUST render as two
    // groups (attribution stays joinable, never merged). The default
    // scope reads THIS machine only; "all" adds future-synced
    // foreign rows (display-only in the UI).
    let mut by_key: std::collections::HashMap<
        (String, String),
        Vec<juggler_core::storage::LedgerRow>,
    > = std::collections::HashMap::new();
    for row in if all_machines {
        all_ledger_rows(state)
    } else {
        local_ledger_rows(state)
    } {
        by_key
            .entry((row.session_id.clone(), row.machine_id.clone()))
            .or_default()
            .push(row);
    }
    let mut session_order: Vec<(String, String)> = by_key.keys().cloned().collect();
    session_order.sort();
    for (session, row_machine) in session_order {
        let rows_raw = &by_key[&(session.clone(), row_machine.clone())];
        let provider = rows_raw
            .first()
            .map(|row| row.provider.clone())
            .unwrap_or_default();
        let account = rows_raw.first().and_then(|row| row.account_id.clone());
        let project = rows_raw.first().and_then(|row| row.project_id.clone());
        let machine = rows_raw
            .first()
            .map(|row| row.machine_id.clone())
            .unwrap_or_default();
        let root_row = rows_raw.iter().find(|row| row.role == "root");
        let root_pid = root_row.and_then(|row| row.pid);
        let root_alive = root_row.is_some_and(|row| row.state == "running");
        let session_adopted = adopted.contains(&session);

        let mut rows: Vec<CleanupRow> = Vec::new();
        let mut any_attention = false;
        for row in rows_raw {
            let pid = row.pid.unwrap_or_default();
            if pid == 0 {
                continue;
            }
            // History filter: exited rows within 7 days only.
            if filter == "history" && row.state != "exited" {
                continue;
            }
            if filter != "history" && row.state == "exited" {
                continue;
            }
            // Query-time verification (E503): table facts vs record.
            let verification = if row.state == "unverifiable" {
                VerificationBadge {
                    verdict: "unverifiable".into(),
                    detail: "identity check failed — permanently untouched".into(),
                }
            } else if let Some(facts) = table.as_ref().and_then(|t| t.facts.get(&pid)) {
                let start_ok = row
                    .started_at
                    .is_none_or(|start| start == 0 || start == facts.start_time);
                let exe_ok = !row.exe_path.starts_with('/')
                    || facts.exe.as_deref().is_none_or(|exe| exe == row.exe_path);
                if start_ok && exe_ok {
                    VerificationBadge {
                        verdict: "verified".into(),
                        detail: format!(
                            "start-time and exe match the live process (start {}, exe {:?})",
                            facts.start_time,
                            facts.exe.as_deref().unwrap_or("?")
                        ),
                    }
                } else {
                    VerificationBadge {
                        verdict: "unverifiable".into(),
                        detail: "live start-time or exe differs from the record".into(),
                    }
                }
            } else if row.state == "exited" {
                VerificationBadge {
                    verdict: "unknown".into(),
                    detail: "process exited — no live check applies".into(),
                }
            } else {
                VerificationBadge {
                    verdict: "unknown".into(),
                    detail: "not in the current process table".into(),
                }
            };

            let row_hogs: Vec<&HogFactInfo> = hogs
                .iter()
                .filter(|hog| hog.pid == pid || (hog.pid == 0 && hog.session == session))
                .collect();
            let port_collision = row.state == "orphaned" && row.listens_on.is_some();
            let effective_state = if session_adopted && row.state == "running" {
                "adopted"
            } else {
                row.state.as_str()
            };
            let needs_attention = row.state == "orphaned" || !row_hogs.is_empty() || port_collision;
            if needs_attention {
                any_attention = true;
            }
            // Filter sets.
            let matches = match filter {
                "needs-attention" => needs_attention,
                "re-adopted" => session_adopted,
                _ => true, // all-managed + history (its row gate is above)
            };
            if !matches {
                continue;
            }
            let live = live_rows.get(&pid);
            // AJ-E510 Req.2: the durable down-period marker (set by
            // crash recovery) — distinct from live-session orphans.
            let down_period = effective_state == "orphaned" && row.down_period;
            let state_label = if down_period {
                "left behind while Juggler was closed".to_string()
            } else {
                cleanup_state_label(effective_state).to_string()
            };
            rows.push(CleanupRow {
                machine_id: row.machine_id.clone(),
                pid,
                pgid: row.pgid,
                role: row.role.clone(),
                state: effective_state.to_string(),
                state_label,
                down_period,
                role_guess: role_words(guess_role(&row.exe_path, &row.cmdline_trunc)).to_string(),
                exe_path: row.exe_path.clone(),
                cmdline: row.cmdline_trunc.clone(),
                age_ms: row
                    .started_at
                    .map(|start| (now - start.saturating_mul(1000)).max(0)),
                rss_bytes: live
                    .map(|l| l.rss_bytes)
                    .or(row.peak_rss.map(i64::unsigned_abs)),
                cpu_pct: live.and_then(|l| l.cpu_pct).or(row.peak_cpu),
                ports: row.listens_on.clone(),
                hog: !row_hogs.is_empty(),
                hog_rules: row_hogs.iter().map(|hog| hog.rule.clone()).collect(),
                evidence: EvidenceChain {
                    root_pid,
                    parent_pid: row.parent_pid,
                    pgid: row.pgid,
                    escaped_group: row.escaped_group,
                    verification,
                    first_seen: row.first_seen,
                    last_seen: row.last_seen,
                },
            });
        }
        if rows.is_empty() {
            continue;
        }
        let healthy = !any_attention && root_alive;
        if healthy
            && root_alive
            && rows_raw
                .iter()
                .all(|row| row.state == "running" || row.state == "exited")
        {
            clean += 1;
        }
        groups.push(CleanupGroup {
            session: session.clone(),
            provider,
            account,
            project,
            machine,
            healthy,
            account_label: None,
            project_name: None,
            label: None,
            status: None,
            started_at_ms: None,
            live: root_alive || session_adopted,
            ended_at_ms: (!root_alive && !session_adopted)
                .then(|| root_row.map(|row| row.last_seen))
                .flatten(),
            rows,
        });
    }
    // Names, not ids, for each group's title (storage only, after the
    // collector reads: the two locks are never held together).
    let keys: Vec<(String, Option<String>, Option<String>)> = groups
        .iter()
        .map(|group| {
            (
                group.session.clone(),
                group.account.clone(),
                group.project.clone(),
            )
        })
        .collect();
    let mut names = group_names(state, &keys);
    for group in &mut groups {
        if let Some(found) = names.remove(&group.session) {
            group.account_label = found.account_label;
            group.project_name = found.project_name;
            group.label = found.label;
            group.status = found.status;
            group.started_at_ms = found.started_at_ms;
        }
    }
    CleanupInventoryInfo {
        groups,
        local_machine: local_machine_id(state).as_str().to_string(),
        clean_instances: clean,
        ledger_degraded: degraded,
        history: None,
    }
}

/// `cleanup_audit_export` sans-Tauri (tests + evidence).
pub fn cleanup_audit_export_inner(state: &Arc<AppState>) -> String {
    let since = now_millis() - 7 * 24 * 3_600_000;
    let rows = state.audit.rows_since(since).unwrap_or_default();
    let mut lines = vec![r#"{"note":"cleanup audit export — command lines are hashed and truncated; full cmdlines are never stored"}"#.to_string()];
    for row in rows {
        if let Ok(line) = serde_json::to_string(&row) {
            lines.push(line);
        }
    }
    lines.join("\n")
}
