//! AJ-E1508: a new account starts with the user's usual settings, copied
//! from an account of the same tool they pick: its MCP servers, its
//! permissions, its status line, and its plugins (those need the tool's
//! own install, and say how).
//!
//! The source and target adapters read what each account has
//! (`HarnessAdapter::inventory`), and the target's adapter says how each
//! item can be set up; the switch set-up's check, record and undo do the
//! rest (AJ-E1542). Only settings move: an account's sign-in, its history
//! and its caches are never among the items, so they never reach the new
//! account. Nothing is written until the user applies, and every set-up
//! can be undone from Settings.

pub mod commands;

use std::sync::Arc;

use juggler_adapter::inventory::ItemKind;
use juggler_core::model::{Account, AccountId};

use crate::error::{AppError, AppResult};
use crate::features::switch_setup::{self, AppliedInfo, SetupCheckInfo, Sides};
use crate::state::AppState;

/// What a new account can start with.
fn usual_settings(kind: ItemKind) -> bool {
    matches!(
        kind,
        ItemKind::McpServer | ItemKind::Plugin | ItemKind::Permissions | ItemKind::StatusLine
    )
}

/// The two accounts, `source` copied into `target`: the same tool only,
/// since another tool's settings mean something else.
fn sides(state: &Arc<AppState>, target: &str, source: &str) -> AppResult<Sides> {
    let account = |id: &str| -> AppResult<Account> {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage
            .accounts()
            .get(&AccountId::new(id))
            .map_err(|_| AppError::Refused("that account isn't there any more".to_owned()))
    };
    let (target, source) = (account(target)?, account(source)?);
    if target.id == source.id {
        return Err(AppError::Refused(
            "pick another account to copy from".to_owned(),
        ));
    }
    if target.harness != source.harness {
        return Err(AppError::Refused(format!(
            "{} uses another tool: pick an account of the same tool",
            source.label
        )));
    }
    Ok(Sides {
        source,
        target,
        project: None,
    })
}

/// What `target` could start with from `source`, each with how it would be
/// set up. Read-only. With no `target` yet (Add account, before the
/// account is made), what a fresh account of `source`'s tool would get:
/// it has nothing, and the offers' ids are the same once it exists.
///
/// # Errors
///
/// [`AppError::Refused`] when an account is gone, or they're the same or of
/// different tools; the inventories' errors.
pub fn check_impl(
    state: &Arc<AppState>,
    target: Option<&str>,
    source: &str,
) -> AppResult<SetupCheckInfo> {
    if let Some(target) = target {
        return switch_setup::check_sides(state, &sides(state, target, source)?, &usual_settings);
    }
    let source = {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage
            .accounts()
            .get(&AccountId::new(source))
            .map_err(|_| AppError::Refused("that account isn't there any more".to_owned()))?
    };
    let adapter = state
        .registry
        .get(&source.harness)
        .ok_or_else(|| AppError::UnknownHarness(source.harness.to_string()))?;
    let items: Vec<_> = adapter
        .inventory(&source.state_dir, None)
        .into_iter()
        .filter(|item| usual_settings(item.kind))
        .collect();
    // The new account is of the same tool: its verdicts are this tool's.
    let (offers, _) = switch_setup::offers(&source, adapter.as_ref(), &items, &[]);
    Ok(SetupCheckInfo {
        from: source.label.clone(),
        to: "the new account".to_owned(),
        offers,
        already_there: 0,
    })
}

/// Sets up the `picked` offers from [`check_impl`], each recorded so it can
/// be undone from Settings.
///
/// # Errors
///
/// As [`check_impl`]; a failed record write.
pub fn apply_impl(
    state: &Arc<AppState>,
    target: &str,
    source: &str,
    picked: &[String],
) -> AppResult<Vec<AppliedInfo>> {
    switch_setup::apply_sides(
        state,
        &sides(state, target, source)?,
        picked,
        &usual_settings,
    )
}
