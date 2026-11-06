//! AJ-E309 Req.5: the Expert setup panel: where a Provider's, account's or
//! launch setting's files are, from AI Juggler's own state.

pub mod commands;
pub mod dto;

use std::sync::Arc;

use crate::commands::{find_account_arc, find_profile_with_account_arc};
use crate::error::{AppError, AppResult};
use crate::features::expert::dto::ExpertSetupInfo;
use crate::state::AppState;

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`expert_setup_info`](crate::features::expert::commands::expert_setup_info).
pub fn expert_setup_info_impl(
    state: &Arc<AppState>,
    provider: &str,
    account_id: Option<&str>,
    recipe_id: Option<&str>,
) -> AppResult<ExpertSetupInfo> {
    let Some(account_id) = account_id else {
        let adapter = state
            .registry
            .get(&juggler_core::model::HarnessId::new(provider))
            .ok_or_else(|| AppError::UnknownHarness(provider.to_owned()))?;
        let dir = state.accounts_dir().join(provider);
        return Ok(ExpertSetupInfo {
            provider: provider.to_owned(),
            account_id: None,
            exists: dir.is_dir(),
            state_dir: dir.to_string_lossy().into_owned(),
            shell_snippet: None,
            protected_env_keys: adapter
                .isolation_env_keys()
                .iter()
                .map(|key| (*key).to_owned())
                .collect(),
            note: "Each new app gets its own isolated folder in here — the shell line \
                   appears once the app exists."
                .to_owned(),
        });
    };
    let account = find_account_arc(state, account_id)?;
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    let extra_env = match recipe_id {
        Some(recipe_id) => {
            let (recipe, owner) = find_profile_with_account_arc(state, recipe_id)?;
            if owner.id != account.id {
                return Err(AppError::Validation {
                    subject: "launch settings",
                    message: "those Launch settings belong to another account".to_owned(),
                });
            }
            recipe.settings_overlay.env
        }
        None => std::collections::BTreeMap::new(),
    };
    Ok(ExpertSetupInfo {
        provider: account.harness.to_string(),
        account_id: Some(account.id.to_string()),
        exists: account.state_dir.is_dir(),
        state_dir: account.state_dir.to_string_lossy().into_owned(),
        shell_snippet: juggler_adapter::adapter::shell_snippet(
            adapter.as_ref(),
            &account.state_dir,
            &extra_env,
        ),
        protected_env_keys: adapter
            .isolation_env_keys()
            .iter()
            .map(|key| (*key).to_owned())
            .collect(),
        note: format!(
            "“{}” keeps all of its state here — isolated from your other accounts and \
             from your own install.",
            account.label
        ),
    })
}

/// The folder [`expert_reveal`](crate::features::expert::commands::expert_reveal) would show (shared with tests): the
/// requested managed folder, or its nearest existing ancestor inside
/// the data dir.
///
/// # Errors
///
/// Returns the typed [`AppError`] for unknown ids.
pub fn expert_reveal_target(
    state: &Arc<AppState>,
    provider: &str,
    account_id: Option<&str>,
) -> AppResult<std::path::PathBuf> {
    let info = expert_setup_info_impl(state, provider, account_id, None)?;
    // ADR-0018: an in-place account's folder is the user's own — reveal it.
    if let Some(id) = account_id
        && let Ok(account) = find_account_arc(state, id)
        && account.in_place
        && account.state_dir.is_dir()
    {
        return Ok(account.state_dir);
    }
    let mut target = std::path::PathBuf::from(info.state_dir);
    while !target.is_dir() && target != state.data_dir {
        if !target.pop() {
            break;
        }
    }
    if !target.starts_with(&state.data_dir) {
        target.clone_from(&state.data_dir);
    }
    Ok(target)
}
