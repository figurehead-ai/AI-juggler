//! AJ-E307: importing an existing setup as an account: the source check, the
//! import, and keeping the original folder in place (ADR-0018).

use std::sync::Arc;

use juggler_core::model::{Account, AccountId, Profile, ProfileId, SessionStatus, SettingsOverlay};

use crate::commands::{find_account_arc, lock_storage_arc};
use crate::error::{AppError, AppResult};
use crate::features::account::dto::{AccountInfo, FolderSizeInfo, ImportResult, SourceInspectInfo};
use crate::features::account::{account_info_of, login_probe_name};
use crate::ids::unique_suffix;
use crate::state::AppState;

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] on failure.
pub fn source_inspect_impl(
    state: &Arc<AppState>,
    harness: &str,
    source_path: &str,
) -> AppResult<SourceInspectInfo> {
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new(harness))
        .ok_or_else(|| AppError::UnknownHarness(harness.to_string()))?;
    let inspect = adapter.inspect_source(std::path::Path::new(source_path))?;
    // Keychain-bound: signed in with NO credentials file (the login
    // cannot move — E303's caveat, surfaced at Confirm-source).
    let keychain_bound = inspect.login == juggler_adapter::environment::LoginProbe::LoggedIn
        && juggler_adapters::sign_in::metadata(harness, std::path::Path::new(source_path))
            .is_none();
    Ok(SourceInspectInfo {
        total_bytes: inspect.total_bytes,
        file_count: inspect.file_count,
        folders: inspect
            .folders
            .into_iter()
            .map(|(name, bytes)| FolderSizeInfo { name, bytes })
            .collect(),
        truncated: inspect.truncated,
        ceiling_bytes: inspect.ceiling_bytes,
        is_config: inspect.is_config,
        already_managed: inspect.already_managed,
        login: login_probe_name(inspect.login),
        keychain_bound,
    })
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// [`AppError::Validation`] when there is no original folder, a session
/// is running on the account, or another account already uses it; the
/// adapter's refusal when the folder is gone or isn't a setup.
pub fn account_use_original_impl(
    state: &Arc<AppState>,
    account_id: &str,
) -> AppResult<AccountInfo> {
    let account = find_account_arc(state, account_id)?;
    if account.in_place {
        return Ok(account_info_of(state, &account));
    }
    let Some(source) = account.import_source_path.clone() else {
        return Err(AppError::Validation {
            subject: "account",
            message: "this account was made in AI Juggler — it has no original folder".into(),
        });
    };
    let canonical =
        |path: &std::path::Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    {
        let storage = lock_storage_arc(state)?;
        let profiles: Vec<ProfileId> = storage
            .profiles()
            .list_by_account(&account.id)?
            .into_iter()
            .map(|profile| profile.id)
            .collect();
        // Anything that may still run or resume in the copy (stuck,
        // waiting for its limit to reset) would resume against the
        // original, which doesn't have its history.
        let open = storage.sessions().list(None)?.into_iter().any(|record| {
            profiles.contains(&record.profile)
                && matches!(
                    record.status,
                    SessionStatus::Starting
                        | SessionStatus::Running
                        | SessionStatus::WaitingUser
                        | SessionStatus::Stuck
                        | SessionStatus::QuotaBlocked
                )
        });
        if open {
            return Err(AppError::Validation {
                subject: "account",
                message: "a session on this account is still open (running, stuck or waiting for its limit) — finish or stop it first, then switch".into(),
            });
        }
        let target = canonical(std::path::Path::new(&source));
        if let Some(other) = storage.accounts().list()?.into_iter().find(|other| {
            other.id != account.id && other.in_place && canonical(&other.state_dir) == target
        }) {
            return Err(AppError::Validation {
                subject: "account",
                message: format!("«{}» already uses that folder", other.label),
            });
        }
    }
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    // The exact path, not the resolved one the provenance keeps: the
    // default folder is `~/.claude` (`~/.codex`) itself even when it's a
    // link (it runs with the variable unset). The adapter's own home, so
    // this and its launch rule agree.
    let folder = adapter
        .vendor_default_dir()
        .filter(|default| canonical(default) == canonical(std::path::Path::new(&source)))
        .unwrap_or_else(|| std::path::PathBuf::from(&source));
    adapter.adopt_account(&folder)?;
    let mut updated = account;
    updated.state_dir = folder;
    updated.in_place = true;
    lock_storage_arc(state)?.accounts().update(&updated)?;
    // Its usage is read from a new place now: start the offsets fresh
    // (the store dedupes anything read twice).
    crate::usage_pipeline::forget_account_offsets(state, updated.id.as_str());
    Ok(account_info_of(state, &updated))
}

/// How a setup comes in (ADR-0018).
#[derive(Clone, Debug, Default)]
pub struct ImportRequest {
    /// Vendor plan tier, if known.
    pub plan: Option<String>,
    /// The discovery origin.
    pub origin: Option<String>,
    /// AJ-E307 Req.4: settings only, history skipped (copies).
    pub degraded: bool,
    /// Use the folder where it is instead of copying it.
    pub in_place: bool,
    /// AJ-E3202 req. 3: the user saw `label` and brought the folder in
    /// under it (first run's found step, or a name they typed), so it's
    /// noted as accepted at import. A name worked out without them
    /// (a desktop profile's linked folder, a seeded walk) isn't.
    pub accepted_name: bool,
}

/// Core implementation shared with integration tests (a copy).
///
/// # Errors
///
/// As [`account_import_mode_impl`].
pub fn account_import_impl(
    state: &Arc<AppState>,
    harness: &str,
    source_path: &str,
    label: &str,
    plan: Option<String>,
    origin: Option<String>,
    degraded: bool,
) -> AppResult<ImportResult> {
    account_import_mode_impl(
        state,
        harness,
        source_path,
        label,
        ImportRequest {
            plan,
            origin,
            degraded,
            in_place: false,
            accepted_name: false,
        },
    )
}

/// Brings a setup in, copied or where it is (ADR-0018).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn account_import_mode_impl(
    state: &Arc<AppState>,
    harness: &str,
    source_path: &str,
    label: &str,
    request: ImportRequest,
) -> AppResult<ImportResult> {
    // AJ-E1010: no "Restart to update" while a bring-in is under way.
    let _bringing_in = state.updates.bring_in();
    bring_in_setup(state, harness, source_path, label, request)
}

fn bring_in_setup(
    state: &Arc<AppState>,
    harness: &str,
    source_path: &str,
    label: &str,
    request: ImportRequest,
) -> AppResult<ImportResult> {
    let ImportRequest {
        plan,
        origin,
        degraded,
        in_place,
        accepted_name,
    } = request;
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new(harness))
        .ok_or_else(|| AppError::UnknownHarness(harness.to_string()))?;
    // AJ-E307 Req.2: duplicate protection — a provenance-matched
    // source is blocked with the existing account's label, before
    // anything is written.
    let canonical_source = match std::path::Path::new(source_path).canonicalize() {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(_) => source_path.to_owned(),
    };
    {
        let storage = lock_storage_arc(state)?;
        for existing in storage.accounts().list()? {
            if existing.import_source_path.as_deref() == Some(canonical_source.as_str()) {
                return Err(AppError::AlreadyImported {
                    label: if existing.in_place {
                        existing.label.clone()
                    } else {
                        format!("{}, a copy", existing.label)
                    },
                });
            }
        }
    }
    let id = AccountId::new(format!("{harness}-{}", unique_suffix()));
    let (state_dir, outcome) = if in_place {
        // The folder exactly as found (Claude's Keychain name hashes the
        // path string), without a trailing separator.
        let trimmed = source_path.trim_end_matches(['/', '\\']);
        let folder = std::path::PathBuf::from(if trimmed.is_empty() {
            source_path
        } else {
            trimmed
        });
        let outcome = adapter.adopt_account(&folder)?;
        (folder, outcome)
    } else {
        let state_dir = state.accounts_dir().join(harness).join(id.as_str());
        let outcome = if degraded {
            adapter.import_account_degraded(std::path::Path::new(source_path), &state_dir)?
        } else {
            adapter.import_account(std::path::Path::new(source_path), &state_dir)?
        };
        (state_dir, outcome)
    };
    let account = Account {
        // AJ-E307 Req.1: the import's provenance, written at import
        // time — canonical path + origin + completion stamp.
        import_source_path: Some(canonical_source),
        import_origin: origin,
        imported_at: Some(juggler_core::model::Timestamp::now().millis()),
        in_place,
        ..super::new_account(id, harness, label, plan, state_dir)
    };
    let mut inserted = false;
    let saved = (|| -> AppResult<()> {
        let storage = lock_storage_arc(state)?;
        storage.accounts().insert(&account)?;
        inserted = true;
        if accepted_name {
            storage
                .accounts()
                .set_import_name(&account.id, Some(&account.label))?;
        }
        // A default profile: the imported account launches in one click.
        storage.profiles().insert(&Profile {
            id: ProfileId::new(format!("prof-{}", unique_suffix())),
            name: "Main".to_string(),
            account: account.id.clone(),
            settings_overlay: SettingsOverlay::default(),
            default_cwd: None,
        })?;
        Ok(())
    })();
    if let Err(error) = saved {
        // ADR-0018 §5: a copy whose record can't be saved doesn't linger
        // (only ever our own marked copy — never an in-place folder).
        if !in_place && juggler_adapters::claude_code::is_managed_dir(&account.state_dir) {
            let _ = std::fs::remove_dir_all(&account.state_dir);
        }
        if inserted && let Ok(storage) = lock_storage_arc(state) {
            let _ = storage.accounts().delete(&account.id);
        }
        return Err(error);
    }
    let account = crate::import_report::settled(state, account, &outcome);
    Ok(ImportResult {
        account: account_info_of(state, &account),
        files_copied: outcome.files_copied,
        bytes_copied: outcome.bytes_copied,
        skipped: outcome.skipped,
        login: login_probe_name(outcome.login),
        login_note: outcome.login_note,
        folders: outcome
            .folders
            .into_iter()
            .map(|(name, bytes)| FolderSizeInfo { name, bytes })
            .collect(),
        degraded: outcome.degraded,
    })
}
