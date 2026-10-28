//! What a paired machine asks this Mac to do, and what this Mac tells it
//! (AJ-E707, AJ-E711): clean one process it owns (re-verified here), the
//! cleanup picture it sends, a command in the prompt's plain words, and
//! focusing a session's window. Split from `link` (the size budget).

use std::collections::HashMap;
use std::sync::Arc;

use juggler_link::node::CommandRejection;
use juggler_link::proto;

use crate::error::AppError;
use crate::state::AppState;

/// AJ-E707: a console asked to clean one process. The node re-verifies
/// ownership NOW (pid + start time + exe + boot — the same check as a
/// local Clean), runs its own TERM → grace → KILL ladder, and audits it
/// as `fleet:<console>`. An unverifiable process is never signalled.
pub(super) fn clean_here(
    state: &Arc<AppState>,
    console: &str,
    instance_id: &str,
    pid: u32,
) -> Result<String, CommandRejection> {
    let reject = |code: &str, message: String| CommandRejection {
        reason_code: code.to_owned(),
        message,
    };
    let actor = format!(
        "fleet:{}",
        crate::fleet::peer_label(state, console).unwrap_or_else(|| console.to_owned())
    );
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let result = tokio::task::block_in_place(|| {
        crate::commands::kill_execute_as(
            state,
            state.audit.as_ref(),
            &actor,
            instance_id,
            pid,
            "pid",
            &cancel,
            None,
        )
    });
    let info = result.map_err(|error| match error {
        AppError::NotFound { .. } => reject(
            "unknown_record",
            "Not owned by that session on this computer — never touched.".to_owned(),
        ),
        other => reject("clean_failed", other.wire().user_message),
    })?;
    if let Some(refusal) = info.refusal {
        return Err(reject("refused", format!("Refused — {refusal}")));
    }
    Ok(info
        .members
        .iter()
        .map(|member| format!("{} (pid {})", member_words(&member.outcome), member.pid))
        .collect::<Vec<_>>()
        .join(" · "))
}

/// The ladder's outcome in the console's words (AJ-E707 Req.7).
fn member_words(outcome: &str) -> &str {
    match outcome {
        "exited on TERM" => "Exited",
        "needed KILL" => "Needed SIGKILL",
        "already gone" => "Already gone",
        "refused" => "Refused — identity changed",
        other => other,
    }
}

/// This Mac's managed processes as a `CleanupDelta` (never a command
/// line — the executable's file name only).
pub(super) fn cleanup_delta(state: &Arc<AppState>) -> proto::CleanupDelta {
    let inventory = crate::commands::cleanup_inventory_inner(state, "all-managed", false);
    let labels: HashMap<String, (String, String)> = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| crate::commands::session_infos_snapshot(&storage).ok())
        .map(|rows| {
            rows.into_iter()
                .map(|row| (row.id, (row.provider, row.account_label)))
                .collect()
        })
        .unwrap_or_default();
    let processes = inventory
        .groups
        .iter()
        .flat_map(|group| {
            let (provider, account_label) = labels
                .get(&group.session)
                .cloned()
                .unwrap_or_else(|| (group.provider.clone(), String::new()));
            group.rows.iter().map(move |row| proto::CleanupProcess {
                instance_id: group.session.clone(),
                pid: row.pid,
                role: row.role.clone(),
                state: row.state.clone(),
                state_label: row.state_label.clone(),
                verified: row.evidence.verification.verdict == "verified"
                    && row.role != "root"
                    && matches!(row.state.as_str(), "running" | "orphaned"),
                name: std::path::Path::new(&row.exe_path)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                age_ms: row.age_ms,
                rss_bytes: row.rss_bytes,
                cpu_percent: row.cpu_pct,
                ports: row.ports.clone().unwrap_or_default(),
                provider: provider.clone(),
                account_label: account_label.clone(),
            })
        })
        .collect();
    proto::CleanupDelta {
        processes,
        ledger_degraded: inventory.ledger_degraded,
    }
}

/// Plain words for a command, as the prompt shows it.
pub(super) fn describe_action(
    state: &Arc<AppState>,
    action: &proto::control_command::Action,
) -> String {
    match action {
        proto::control_command::Action::StopInstance(stop) => {
            let label = state
                .storage
                .lock()
                .ok()
                .and_then(|storage| crate::commands::session_infos_snapshot(&storage).ok())
                .and_then(|rows| rows.into_iter().find(|row| row.id == stop.instance_id))
                .map(|row| format!("{} ({})", row.account_label, row.provider));
            format!("stop {}", label.unwrap_or_else(|| "a session".to_owned()))
        }
        proto::control_command::Action::LaunchRecipe(_) => {
            "start a session from your Launch settings".to_owned()
        }
        proto::control_command::Action::CleanProcess(clean) => {
            format!("clean up process {} of one of your sessions", clean.pid)
        }
        proto::control_command::Action::TakeHandover(take) => format!(
            "take over work from {} — continue it here",
            if take.from_account_label.is_empty() {
                "another Account"
            } else {
                take.from_account_label.as_str()
            }
        ),
        proto::control_command::Action::SwitchBack(_) => {
            "bring work back to its original session here".to_owned()
        }
        proto::control_command::Action::FocusInstance(focus) => {
            let label = state
                .storage
                .lock()
                .ok()
                .and_then(|storage| crate::commands::session_infos_snapshot(&storage).ok())
                .and_then(|rows| rows.into_iter().find(|row| row.id == focus.instance_id))
                .map(|row| format!("{} ({})", row.account_label, row.provider));
            format!(
                "bring {}'s window to the front",
                label.unwrap_or_else(|| "a session".to_owned())
            )
        }
    }
}

/// AJ-E711: runs the SAME tiered focus as a local click (AJ-E414), on
/// this Mac, with this Mac's Accessibility permission — never on a
/// locked screen, and never anything but the honest outcome words.
pub(super) fn focus_here(
    state: &Arc<AppState>,
    instance_id: &str,
) -> Result<String, CommandRejection> {
    let reject = |code: &str, message: String| CommandRejection {
        reason_code: code.to_owned(),
        message,
    };
    let here = crate::fleet::fleet_self_impl(state)
        .map_or_else(|_| "this Mac".to_owned(), |info| info.label);
    let known = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| crate::commands::session_infos_snapshot(&storage).ok())
        .is_some_and(|rows| rows.iter().any(|row| row.id == instance_id));
    if !known {
        return Err(reject(
            "unknown_record",
            "No such session on this computer.".to_owned(),
        ));
    }
    match crate::focus::screen_locked() {
        Ok(false) => {}
        Ok(true) => {
            return Err(reject(
                "screen_locked",
                format!("{here} is locked — nothing was focused."),
            ));
        }
        Err(_) => {
            return Err(reject(
                "lock_unknown",
                format!("Couldn't tell whether {here} is locked — nothing was focused."),
            ));
        }
    }
    let outcome = crate::commands::desktop_focus_with(
        state,
        instance_id,
        crate::focus::link_runner(),
        crate::focus::link_exact_windows(state),
    )
    .map_err(|error| match error {
        AppError::Validation { .. } => reject(
            "not_desktop",
            "That session runs in a terminal — there's no window to focus.".to_owned(),
        ),
        other => reject("focus_failed", other.wire().user_message),
    })?;
    if outcome.permission_missing {
        return Ok(format!("Permission needed on {here}"));
    }
    Ok(outcome.outcome)
}
