//! Starting sessions: a launch setting's session through the supervisor, its
//! environment overrides (AJ-E309), and the port warning before a launch
//! (AJ-E505).

pub mod commands;
pub mod dto;
pub mod in_flight;
pub mod recipe_env;
pub mod resume_gate;

use std::sync::Arc;

use juggler_core::model::{SessionId, Timestamp};

use crate::commands::{find_profile_with_account_arc, lock_storage_arc};
use crate::error::{AppError, AppResult};
use crate::features::launch::dto::LaunchResult;
use crate::ids::unique_suffix;
use crate::state::AppState;

/// Launches a session for a profile through the supervisor (one click on
/// the UI side; the only writer of harness state is the harness itself,
/// inside its managed dir).
/// The account's usage snapshot from the two-clock engine (AJ-E606):
/// the single producer of `UsageSnapshot.window` — every session
/// insert site calls this, so quota state has exactly one
/// representation. Until ingest (AJ-E607) feeds the store this is the
/// honest zero (`window: None`, default confidence) — never an
/// invented 0%. Errors degrade to the same honest zero (a broken
/// quota read must never block launching).
pub(crate) fn engine_usage_snapshot(
    storage: &juggler_core::storage::Storage,
    profile: &juggler_core::model::ProfileId,
) -> juggler_core::model::UsageSnapshot {
    use juggler_core::quota::{self, QuotaCap, SubscriptionClock, WindowKind};
    use juggler_core::storage::{anchor_settings_key, cap_settings_key};

    let account = match storage.profiles().get(profile) {
        Ok(recipe) => recipe.account,
        Err(_) => return juggler_core::model::UsageSnapshot::default(),
    };
    let account = account.as_str().to_string();
    let Ok(Some(_)) = storage.terms().own(&account) else {
        return juggler_core::model::UsageSnapshot::default();
    };
    let now_ms = Timestamp::now().millis();
    let usage_repo = storage.usage();
    let Ok(events) = usage_repo.events_for_account(&account, now_ms - quota::WEEK_MS) else {
        return juggler_core::model::UsageSnapshot::default();
    };
    let settings = storage.app_settings();
    let anchor = settings
        .get(&anchor_settings_key(&account))
        .ok()
        .flatten()
        .and_then(|raw| raw.parse::<i64>().ok());
    // Two weeks and the newest before: the evaluation reads the current
    // window and the newest reading, never the whole history.
    let since = now_ms - 2 * quota::WEEK_MS;
    let samples_five_hour = usage_repo
        .samples_since(&account, WindowKind::FiveHour, since)
        .unwrap_or_default();
    let samples_weekly = usage_repo
        .samples_since(&account, WindowKind::Weekly { anchor_ms: anchor }, since)
        .unwrap_or_default();
    let cap = |kind: WindowKind| -> QuotaCap {
        settings
            .get(&cap_settings_key(&account, kind))
            .ok()
            .flatten()
            .map(|raw| QuotaCap::decode(&raw))
            .unwrap_or_default()
    };
    let renewal_date = storage
        .terms()
        .effective(&account)
        .ok()
        .and_then(|t| t.renewal_date);
    let subscription = SubscriptionClock {
        entered_by_user: renewal_date.is_some(),
        renewal_date,
        from_vendor_feed: false,
    };
    let evaluation = quota::evaluate_account(quota::AccountEvaluation {
        events: &events,
        samples_five_hour: &samples_five_hour,
        samples_weekly: &samples_weekly,
        account: &account,
        cap_five_hour: cap(WindowKind::FiveHour),
        cap_weekly: cap(WindowKind::Weekly { anchor_ms: anchor }),
        weekly_anchor_ms: anchor,
        subscription: &subscription,
        now_ms,
        today: epoch_today(now_ms),
    });
    // The session snapshot carries the 5 h pool (the pool a session
    // draws down first); the weekly clock + renewal stay account-level
    // surfaces (AccountInfo / AJ-E610). Unknown/Learning carry no
    // percentage — the window stays absent rather than reading 0%.
    let five_hour = evaluation.five_hour;
    match five_hour.percentage {
        Some(used) => juggler_core::model::UsageSnapshot {
            tokens: juggler_core::model::TokenCounts::default(),
            window: Some(juggler_core::model::QuotaWindow {
                used_percentage: used,
                resets_at: five_hour
                    .resets_at
                    .map(juggler_core::model::Timestamp::from_millis),
            }),
            confidence: five_hour.confidence,
            at_end: None,
        },
        None => juggler_core::model::UsageSnapshot::default(),
    }
}

/// Today as a UTC calendar date (the engine's injected day; renewal
/// rendering stays a UI concern per AJ-E411).
pub(crate) fn epoch_today(now_ms: i64) -> juggler_core::renewal::CalendarDate {
    juggler_core::renewal::civil_from_days(now_ms.div_euclid(24 * 60 * 60 * 1000))
}

/// Core implementation shared with integration tests (the runtime the
/// profile or the tool picks).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn launch_profile(
    state: &Arc<AppState>,
    profile_id: &str,
    cwd: Option<String>,
) -> AppResult<LaunchResult> {
    launch_profile_as(state, profile_id, cwd, None)
}

/// [`launch_profile`] with this session's own runtime choice.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn launch_profile_as(
    state: &Arc<AppState>,
    profile_id: &str,
    cwd: Option<String>,
    runtime: Option<juggler_adapter::Runtime>,
) -> AppResult<LaunchResult> {
    launch_profile_primed(state, profile_id, cwd, runtime, None)
}

/// E19: [`launch_profile_as`] with the user's first prompt (a session
/// the user starts for a task): a terminal launch takes it as the tool's
/// own first prompt; a driven one gets it from its caller as the user's
/// first message.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn launch_profile_primed(
    state: &Arc<AppState>,
    profile_id: &str,
    cwd: Option<String>,
    runtime: Option<juggler_adapter::Runtime>,
    first: Option<String>,
) -> AppResult<LaunchResult> {
    use juggler_adapter::LaunchMode;

    // AJ-E2907 R3: counted as under way before the deleted check below,
    // until its session is recorded, so a delete can't slip between them.
    let _flight = state.launches.begin(profile_id);
    let (profile, account) = find_profile_with_account_arc(state, profile_id)?;
    // AJ-E1501: a removed account starts nothing (its folder may be in
    // the Trash, or no longer the user's choice).
    if lock_storage_arc(state)?
        .accounts()
        .is_removed(&account.id)?
    {
        return Err(AppError::Refused(format!(
            "«{}» was removed — add it again to start a session on it",
            account.label
        )));
    }
    // The real-accounts rule (ADR-0032 evidence): the Dev build is the
    // agents' lab, and it started the Claude CLI on two real profiles'
    // folders. In Dev, a session starts only on a lab's own folder.
    if crate::edition::current() == crate::edition::Edition::Dev {
        let home = juggler_platform::user::home().unwrap_or_default();
        if let Some(why) = crate::lab_scope::refuses_real_folder(&account.state_dir, &home) {
            return Err(AppError::Refused(format!(
                "AI Juggler Dev won't start a session on «{}»: {why}",
                account.label
            )));
        }
    }
    // D-60: nor do deleted launch settings.
    if lock_storage_arc(state)?
        .profiles()
        .is_removed(&profile.id)?
    {
        return Err(AppError::Refused(format!(
            "«{}» was deleted — pick other launch settings to start a session",
            profile.name
        )));
    }
    // The profile's model goes in as `claude --model` (the adapter's
    // launch spec); settings.json is never rewritten (ADR-0018 §3).
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    let effective_cwd = cwd
        .or_else(|| {
            profile
                .default_cwd
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
        })
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    // AJ-E1001: who this Account is signed in as — before anything runs.
    let assertion = crate::identity::assert_identity(state, &account)?;
    let mode = first.map_or(LaunchMode::Interactive, |prompt| LaunchMode::Primed {
        prompt,
        read_dirs: Vec::new(),
    });
    let spec = adapter.launch_spec(&account, &profile, &effective_cwd, &mode)?;
    let session_id = SessionId::new(format!("sess-{}", unique_suffix()));
    // The juggling anchor: this launch's cwd becomes (or refreshes) the
    // tracked project, and the session is linked to it (ADR-0007).
    // Resolved BEFORE spawn so the ledger Intent row is attributable.
    let canonical_cwd = spec.cwd.canonicalize().unwrap_or_else(|_| spec.cwd.clone());
    // A worktree's sessions belong to its repository's project.
    let project_root = juggler_core::storage::worktree_repo_root(&canonical_cwd)
        .unwrap_or_else(|| canonical_cwd.clone());
    let project = {
        let storage = lock_storage_arc(state)?;
        let project = storage.projects().touch(&project_root, None)?;
        // AJ-E1502: a session started here brings a forgotten project back.
        if project.forgotten_at.is_some() {
            storage.projects().set_forgotten(&project.id, false)?;
        }
        project.id
    };
    // The one path: the row first (ADR-0010 §1), then the runtime its plan
    // picks (AJ-E2302), with the bridge and the account's key in the launch
    // that runs.
    let started = crate::features::sessions::spawn::start_session(
        state,
        &crate::features::sessions::spawn::NewSession {
            id: &session_id,
            account: &account,
            profile: &profile.id,
            project: Some(project),
            cwd: spec.cwd.clone(),
            conversation: None,
            transcript_ref: None,
            origin: "new",
        },
        crate::features::sessions::spawn::Runs::Planned {
            profile: &profile,
            resume: None,
            requested: runtime,
            terminal: spec,
        },
    )?;
    crate::identity::record(state, &started.session, &account, &assertion, "launch");
    Ok(LaunchResult {
        session_id: started.session.to_string(),
        pid: started.spawned.pid,
    })
}
