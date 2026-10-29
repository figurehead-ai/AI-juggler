//! AJ-E1515: the paired Macs as one place to work. New session can start
//! a session on another Mac — from its own launch settings and projects,
//! over the link's command path (the node re-checks everything; nothing
//! but ids crosses) — and Usage totals what each machine used in the
//! last day. A machine's usage travels as totals only, with the catalog
//! it already sends (never a session's text, a path or a prompt).

use std::collections::BTreeSet;
use std::sync::Arc;

use juggler_core::storage::Storage;
use juggler_link::proto;

use crate::error::AppResult;
use crate::remote::RemoteCommandOutcomeInfo;
use crate::state::AppState;

/// The span the machine totals cover.
pub const LAST_DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// What this Mac's sessions used in the last day: every counted unit,
/// the list-price estimate (as Usage prices it), and the sessions.
#[must_use]
pub fn last_day(storage: &Storage, now_ms: i64) -> proto::UsageTotals {
    let since = now_ms - LAST_DAY_MS;
    let usage = storage.usage();
    let events: Vec<juggler_core::quota::UsageEvent> = usage
        .usage_accounts()
        .unwrap_or_default()
        .iter()
        .flat_map(|account| {
            usage
                .events_between(account, since, now_ms)
                .unwrap_or_default()
        })
        .collect();
    let units = |event: &juggler_core::quota::UsageEvent| {
        let t = &event.tokens;
        t.input + t.output + t.cache_read + t.cache_write + t.reasoning
    };
    let sessions: BTreeSet<&str> = events
        .iter()
        .filter(|event| units(event) > 0)
        .filter_map(|event| event.session.as_deref())
        .collect();
    let usd_est = juggler_core::cost_ledger::rollup(
        &events,
        &crate::commands::pricing_catalog(storage),
        juggler_core::cost_ledger::RollupDimension::All,
    )
    .first()
    .and_then(|row| row.usd.usd);
    proto::UsageTotals {
        used_units: events.iter().map(units).sum(),
        usd_est,
        sessions: u32::try_from(sessions.len()).unwrap_or(u32::MAX),
        since_ms: since,
    }
}

/// One launch setting a paired Mac offers.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteRecipeOption {
    /// Its id there.
    pub id: String,
    /// Its name.
    pub name: String,
    /// The tool's name.
    pub tool: String,
    /// The account's name there.
    pub account: String,
}

/// One project a paired Mac knows (its name only — never a path).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteProjectChoice {
    /// Its id there.
    pub id: String,
    /// The folder's name.
    pub name: String,
}

/// A paired Mac as New session offers it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LaunchMachineInfo {
    /// Its machine id.
    pub machine_id: String,
    /// Its name here.
    pub label: String,
    /// Why a session can't start there now, in words; `None` when it can.
    pub why_not: Option<String>,
    /// Its launch settings.
    pub recipes: Vec<RemoteRecipeOption>,
    /// Its projects.
    pub projects: Vec<RemoteProjectChoice>,
}

/// One machine's usage in the last day.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MachineUsageInfo {
    /// Its machine id (`None` for this Mac).
    pub machine_id: Option<String>,
    /// Its name ("This Mac" for this one).
    pub label: String,
    /// Every model unit (token) counted.
    #[ts(type = "number")]
    pub used_units: u64,
    /// The list-price estimate, when any of it was priced.
    pub usd_est: Option<f64>,
    /// Sessions that used anything.
    pub sessions: u32,
    /// When it last reported (`None`: this Mac, or nothing heard yet).
    #[ts(type = "number | null")]
    pub as_of_ms: Option<i64>,
    /// Whether it has reported a total at all.
    pub reported: bool,
}

/// Core: every paired Mac, with what New session can start there.
///
/// # Errors
///
/// The typed fleet error.
pub fn launch_options_impl(state: &Arc<AppState>) -> AppResult<Vec<LaunchMachineInfo>> {
    let views = crate::remote::target_views(&state.link.remote);
    Ok(crate::fleet::fleet_peers_impl(state)?
        .into_iter()
        .map(|peer| {
            let view = views
                .iter()
                .find(|(id, _)| *id == peer.machine_id)
                .map(|(_, view)| view.clone())
                .unwrap_or_default();
            let catalog = view.catalog.unwrap_or_default();
            let why_not = if view.peer.is_none() {
                Some(format!("{} isn't connected right now", peer.label))
            } else if view.permission.as_deref() != Some("control") {
                Some(format!(
                    "View only: on {}, let this computer control it in Fleet",
                    peer.label
                ))
            } else if catalog.recipes.is_empty() {
                Some(format!(
                    "{} hasn't shared its launch settings yet",
                    peer.label
                ))
            } else {
                None
            };
            LaunchMachineInfo {
                machine_id: peer.machine_id,
                label: peer.label,
                why_not,
                recipes: catalog
                    .recipes
                    .into_iter()
                    .map(|recipe| RemoteRecipeOption {
                        tool: crate::features::quota::provider_name(&recipe.provider_id),
                        id: recipe.recipe_id,
                        name: recipe.name,
                        account: recipe.account_label,
                    })
                    .collect(),
                projects: catalog
                    .projects
                    .into_iter()
                    .map(|project| RemoteProjectChoice {
                        id: project.project_id,
                        name: project.name,
                    })
                    .collect(),
            }
        })
        .collect())
}

/// Core: this Mac's last day, then each paired Mac's as it last reported.
///
/// # Errors
///
/// The typed storage or fleet error.
pub fn usage_by_machine_impl(
    state: &Arc<AppState>,
    now_ms: i64,
) -> AppResult<Vec<MachineUsageInfo>> {
    let here = {
        let storage = crate::commands::lock_storage_arc(state)?;
        last_day(&storage, now_ms)
    };
    let mut out = vec![MachineUsageInfo {
        machine_id: None,
        label: "This Mac".to_owned(),
        used_units: here.used_units,
        usd_est: here.usd_est,
        sessions: here.sessions,
        as_of_ms: None,
        reported: true,
    }];
    let reported = crate::remote::reported_usage(&state.link.remote);
    for peer in crate::fleet::fleet_peers_impl(state)? {
        let last = reported.iter().find(|(id, _, _)| *id == peer.machine_id);
        out.push(match last {
            Some((_, totals, at)) => MachineUsageInfo {
                machine_id: Some(peer.machine_id),
                label: peer.label,
                used_units: totals.used_units,
                usd_est: totals.usd_est,
                sessions: totals.sessions,
                as_of_ms: Some(*at),
                reported: true,
            },
            None => MachineUsageInfo {
                machine_id: Some(peer.machine_id),
                label: peer.label,
                used_units: 0,
                usd_est: None,
                sessions: 0,
                as_of_ms: None,
                reported: false,
            },
        });
    }
    Ok(out)
}

/// What New session can start on each paired Mac.
///
/// # Errors
///
/// As [`launch_options_impl`].
#[tauri::command]
pub async fn fleet_launch_options(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<LaunchMachineInfo>> {
    crate::ipc::blocking(state.inner(), "paired Macs", launch_options_impl).await
}

/// Starts a session on `machine_id` from its launch setting `recipe_id`,
/// in its project `project_id` (its own default folder when absent). That
/// Mac re-checks everything and answers in its own words.
///
/// # Errors
///
/// [`crate::error::AppError::Validation`] when it isn't connected.
#[tauri::command]
pub async fn fleet_launch_start(
    state: tauri::State<'_, Arc<AppState>>,
    machine_id: String,
    recipe_id: String,
    project_id: Option<String>,
    app: tauri::AppHandle,
) -> AppResult<RemoteCommandOutcomeInfo> {
    use tauri::Emitter as _;
    let outcome = start_impl(
        state.inner().clone(),
        machine_id.clone(),
        recipe_id,
        project_id,
    )
    .await?;
    // Started: the app opens it, as it's seen from this Mac.
    if let Some(started) = &outcome.started_instance_id {
        let _ = app.emit(
            crate::features::notifications::EVENT_OPEN,
            crate::features::notifications::OpenRequest {
                session_id: Some(started.clone()),
                page: None,
                machine_id: Some(machine_id),
            },
        );
    }
    Ok(outcome)
}

/// Core of [`fleet_launch_start`].
///
/// # Errors
///
/// As [`fleet_launch_start`].
pub async fn start_impl(
    state: Arc<AppState>,
    machine_id: String,
    recipe_id: String,
    project_id: Option<String>,
) -> AppResult<RemoteCommandOutcomeInfo> {
    crate::remote::send_action(
        state,
        machine_id,
        proto::control_command::Action::LaunchRecipe(proto::LaunchRecipe {
            recipe_id,
            project_id: project_id.unwrap_or_default(),
        }),
        "launch",
    )
    .await
}

/// Each machine's usage in the last day: this Mac's, then its paired Macs'.
///
/// # Errors
///
/// As [`usage_by_machine_impl`].
#[tauri::command]
pub async fn fleet_usage_by_machine(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<MachineUsageInfo>> {
    crate::ipc::blocking(state.inner(), "usage by machine", |state| {
        usage_by_machine_impl(state, crate::commands::now_millis())
    })
    .await
}
