//! AJ-E508: the recovery summary after a crash.

pub mod commands;
pub mod dto;

use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::recovery::dto::RecoverySummaryInfo;
use crate::state::AppState;

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on read failure.
pub fn recovery_summary_get_arc(state: &Arc<AppState>) -> AppResult<Option<RecoverySummaryInfo>> {
    let storage = lock_storage_arc(state)?;
    let Some(summary) = storage
        .app_settings()
        .get("recovery_summary")
        .map_err(AppError::from)?
        .filter(|text| !text.is_empty())
    else {
        return Ok(None);
    };
    let consumed = storage
        .app_settings()
        .get("recovery_summary_consumed")
        .map_err(AppError::from)?
        .is_some_and(|value| value == "1");
    if consumed {
        return Ok(None);
    }
    // Counts ride the text's shape (written by the same template).
    let count = |part: Option<&str>| {
        part.and_then(|text| text.split_whitespace().next())
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or_default()
    };
    let mut fields = summary.split(',');
    let adopted = count(fields.next());
    let left_behind = count(fields.next());
    let unverifiable = count(fields.next());
    Ok(Some(RecoverySummaryInfo {
        summary,
        adopted,
        left_behind,
        unverifiable,
    }))
}
