//! AJ-E413: a Provider's desktop app started under an account and tracked as
//! a session, and bringing its window forward.

pub mod commands;
pub mod dto;

use std::sync::Arc;

use juggler_core::model::{AccountId, SessionId};

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::desktop::dto::{DesktopLaunchInfo, FocusOutcomeInfo};
use crate::features::sessions::read_model::is_terminal;
use crate::focus;
use crate::ids::unique_suffix;
use crate::state::AppState;

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the provider ships no
/// desktop app, the account lacks launch settings (the owning recipe is
/// the storage anchor), or the process cannot start.
pub fn desktop_launch_arc(state: &Arc<AppState>, account_id: &str) -> AppResult<DesktopLaunchInfo> {
    let (account, recipe_id) = {
        let storage = lock_storage_arc(state)?;
        let account = storage
            .accounts()
            .get(&AccountId::new(account_id.to_string()))?;
        // Desktop instances anchor to the account's launch settings row
        // (the storage FK); the overlay itself is not applied — env
        // isolation is what the DESKTOP adapter declares, honestly.
        let recipe = storage
            .profiles()
            .list_by_account(&account.id)?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::NotFound {
                entity: "launch settings",
                id: account.id.to_string(),
            })?;
        (account, recipe.id)
    };
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    if !adapter
        .capabilities()
        .contains(juggler_adapter::Capabilities::DESKTOP_APP)
    {
        return Err(AppError::Validation {
            subject: "desktop launch",
            message: format!("provider {} ships no desktop app", account.harness.as_str()),
        });
    }
    // Idempotent account-dir preparation (the same contract terminal
    // launches honour) — the launch cwd must exist before spawn. Never
    // for a folder used where it is (ADR-0018 §2): a guard for desktop
    // adapters to come — no real adapter declares DESKTOP_APP today, and
    // a preparation that writes AI Juggler's marker (as Claude's does)
    // must never reach the user's own folder.
    if account.in_place {
        if !account.state_dir.is_dir() {
            return Err(AppError::Validation {
                subject: "desktop launch",
                message: "this account's folder isn't there any more".into(),
            });
        }
    } else {
        adapter.prepare_account(&account.state_dir)?;
    }
    let launch = adapter.desktop_launch(&account)?;
    let session_id = SessionId::new(format!("sess-{}", unique_suffix()));
    let project = {
        let storage = lock_storage_arc(state)?;
        let project = storage.projects().touch(&launch.cwd, None)?;
        if project.forgotten_at.is_some() {
            storage.projects().set_forgotten(&project.id, false)?;
        }
        project.id
    };
    // Intent before spawn (ADR-0010 §1): the row first, then the process,
    // then its window.
    let spawned = crate::features::sessions::spawn::start_desktop(
        state,
        &crate::features::sessions::spawn::NewSession {
            id: &session_id,
            account: &account,
            profile: &recipe_id,
            project: Some(project),
            cwd: launch.cwd.clone(),
            conversation: None,
            transcript_ref: None,
            origin: "new",
        },
        &launch,
    )?;
    Ok(DesktopLaunchInfo {
        session_id: session_id.to_string(),
        pid: spawned.pid,
        bundle_id: launch.bundle_id,
        isolation_label: launch.isolation_label.to_string(),
    })
}

/// The window identity for a desktop Instance: recorded at launch, or
/// synthesized for pre-v10 rows (bundle id from the adapter spec; pid 0
/// forces re-discovery). Shared by the focus tiers and the preview.
pub(crate) fn desktop_window_identity(
    state: &Arc<AppState>,
    record: &juggler_core::model::InstanceRecord,
) -> AppResult<juggler_core::model::WindowIdentity> {
    if let Some(identity) = record.window.clone() {
        return Ok(identity);
    }
    let bundle_id = {
        let storage = lock_storage_arc(state)?;
        let account = storage
            .accounts()
            .get(&storage.profiles().get(&record.profile)?.account)?;
        state
            .registry
            .get(&account.harness)
            .and_then(|adapter| adapter.desktop_launch(&account).ok())
            .and_then(|launch| launch.bundle_id)
    };
    Ok(juggler_core::model::WindowIdentity {
        pid: 0,
        bundle_id,
        title: None,
        cgwindow_id: None,
        start_time: None,
    })
}

/// The testable core (tests inject a scripted runner). `exact_windows`:
/// the user turned window focus on (tiers b and c may run).
///
/// # Errors
///
/// Same as [`desktop_focus`](crate::features::desktop::commands::desktop_focus).
pub fn desktop_focus_with(
    state: &Arc<AppState>,
    session_id: &str,
    runner: std::sync::Arc<dyn focus::ScriptRunner>,
    exact_windows: bool,
) -> AppResult<FocusOutcomeInfo> {
    let mut record = {
        let storage = lock_storage_arc(state)?;
        storage
            .sessions()
            .get(&SessionId::new(session_id.to_string()))?
    };
    if record.kind != juggler_core::model::InstanceKind::Desktop {
        return Err(AppError::Validation {
            subject: "desktop focus",
            message: "window focus applies to desktop sessions".into(),
        });
    }
    let identity = desktop_window_identity(state, &record)?;
    // A reused pid is not the window's process (AJ-E2101).
    let reused = identity.start_time.is_some_and(|recorded| {
        crate::features::conversations::procs::some(&[identity.pid])
            .get(&identity.pid)
            .and_then(|process| process.start)
            .is_none_or(|now| (now - recorded).abs() > 1)
    });
    if is_terminal(record.status) || reused {
        return Ok(FocusOutcomeInfo {
            outcome: focus::OUTCOME_NOT_RUNNING.to_string(),
            permission_missing: false,
            window_title: record.window.as_ref().and_then(|w| w.title.clone()),
        });
    }
    // The tier engine is platform-independent over the runner seam
    // (CI drives it everywhere); the platform gate lives in the
    // command wrapper above.
    let attempt = focus::FocusService::new(runner).attempt(&identity, exact_windows);
    // Persist the (possibly enriched) identity — first-sight titles and
    // re-discovered PIDs survive the attempt (Req.2).
    if attempt.identity != record.window {
        record.window.clone_from(&attempt.identity);
        let storage = lock_storage_arc(state)?;
        storage.sessions().update(&record)?;
    }
    Ok(FocusOutcomeInfo {
        outcome: attempt.outcome.to_string(),
        permission_missing: attempt.permission_missing,
        window_title: attempt
            .identity
            .as_ref()
            .and_then(|identity| identity.title.clone()),
    })
}
