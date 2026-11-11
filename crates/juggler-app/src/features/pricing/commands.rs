//! The pricing commands. Each runs off the main thread.

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::pricing::pricing_catalog;

/// AJ-E609 Req.1: set one user pricing override (the editor shows
/// `source_url` + effective date before save — AJ-E610's surface).
///
/// # Errors
///
/// Returns the typed [`AppError`] on validation or storage failure.
#[tauri::command]
pub async fn pricing_override_set(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    row: juggler_core::cost_ledger::LedgerPriceRow,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "pricing override set", move |state| {
        if row.model.trim().is_empty() || row.source_url.trim().is_empty() {
            return Err(AppError::Validation {
                subject: "pricing override",
                message: "model and source_url are required (auditable pricing law)".into(),
            });
        }
        let storage = lock_storage_arc(state)?;
        storage
            .usage()
            .pricing_override_upsert(&row)
            .map_err(AppError::from)?;
        Ok(())
    })
    .await
}

/// AJ-E609: clear one override (history reprices back to bundled).
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
#[tauri::command]
pub async fn pricing_override_clear(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    model: String,
    effective_from_ms: i64,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "pricing override clear", move |state| {
        let storage = lock_storage_arc(state)?;
        storage
            .usage()
            .pricing_override_delete(&model, effective_from_ms)
            .map_err(AppError::from)?;
        Ok(())
    })
    .await
}

/// AJ-E609: catalog metadata for the pricing editor (staleness, last
/// synced, the bundled + override rows).
#[tauri::command]
///
/// # Errors
///
/// The work could not finish off the main thread.
pub async fn pricing_catalog_info(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> AppResult<juggler_core::cost_ledger::PricingCatalog> {
    crate::ipc::blocking(state.inner(), "pricing catalog info", move |state| {
        Ok(match state.storage.lock() {
            Ok(storage) => pricing_catalog(&storage),
            Err(_) => juggler_core::cost_ledger::PricingCatalog::default(),
        })
    })
    .await
}
