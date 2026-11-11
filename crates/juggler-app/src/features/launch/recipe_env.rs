//! AJ-E309 Req.5: a launch setting's environment overrides.

use std::sync::Arc;

use crate::commands::{find_profile_with_account_arc, lock_storage_arc};
use crate::env_overlay::validate_env_overlay;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`launch_recipe_env_set`](crate::features::launch::commands::launch_recipe_env_set).
pub fn launch_recipe_env_set_impl(
    state: &Arc<AppState>,
    recipe_id: &str,
    env: std::collections::BTreeMap<String, String>,
) -> AppResult<std::collections::BTreeMap<String, String>> {
    let (mut recipe, account) = find_profile_with_account_arc(state, recipe_id)?;
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    validate_env_overlay(adapter.isolation_env_keys(), &env)?;
    recipe.settings_overlay.env = env;
    lock_storage_arc(state)?.profiles().update(&recipe)?;
    Ok(recipe.settings_overlay.env)
}
