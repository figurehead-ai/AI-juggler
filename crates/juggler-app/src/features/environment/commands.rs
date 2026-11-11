//! The environment commands. Each runs off the main thread.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::error::AppResult;
use crate::features::environment::dto::ScanReportInfo;
use crate::features::environment::environment_scan_impl;
use crate::features::environment::recommendation::recommendation_for_impl;
use crate::features::environment::scan::{SCAN_CANCEL, environment_scan_full_impl};
use crate::features::tools::scan_wire::DiscoveredInfo;
use crate::state::AppState;

/// Scans the machine for existing harness setups the onboarding wizard can
/// import (metadata-only; ADR-0006). Never fails — an empty list is a
/// valid "nothing found".
#[tauri::command]
///
/// # Errors
///
/// The work could not finish off the main thread.
pub async fn environment_scan(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<DiscoveredInfo>> {
    crate::ipc::blocking(state.inner(), "environment scan", move |state| {
        let found = environment_scan_impl(state);
        // AJ-E2003 R6: what the scan found may come back to be imported.
        crate::picked::record_offered(found.iter().map(|install| install.path.as_str()));
        Ok(found)
    })
    .await
}

/// AJ-E302: the full 10-signal scan. Probes run in PARALLEL (thread
/// per probe group — metadata walks and `--version` subprocesses
/// never block the UI thread); progress streams as `n of 10` events;
/// the cancel flag stops between probe groups and the report carries
/// `cancelled` (an empty-but-cancelled scan never reads as "nothing
/// found").
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) on registry failures (a denied or
/// partial probe is a ROW, never an error).
#[tauri::command]
pub async fn environment_scan_full(
    state: tauri::State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> AppResult<ScanReportInfo> {
    crate::ipc::blocking(state.inner(), "environment scan full", move |state| {
        // Progress streams to the webview as `scan-progress` events
        // ("Checking… n of 10" — H1 visibility, WCAG-safe text).
        let sink = move |done: u32, total: u32| {
            let _ = tauri::Emitter::emit(
                &app,
                "scan-progress",
                serde_json::json!({ "done": done, "total": total }),
            );
        };
        let report = environment_scan_full_impl(state, Some(&sink));
        if let Ok(report) = &report {
            crate::picked::record_offered(
                report.installs.iter().map(|install| install.path.as_str()),
            );
        }
        report
    })
    .await
}

/// AJ-E302 Req.5: cancels a running scan (the wizard calls it on
/// close/unmount; the report that returns carries `cancelled`).
#[tauri::command]
pub async fn scan_cancel() {
    SCAN_CANCEL.store(true, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
// AJ-E304: the recommendation read side — the scan inventory becomes
// one set-level proposal. STAGING ONLY: no account, login, or file
// copy happens here (the accept click executes, in the wizard).

/// AJ-E304: builds the recommendation from the current scan (the
/// engine itself is pure in `juggler_core::recommendation`).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) when the scan fails.
#[tauri::command]
pub async fn recommendation_for(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<juggler_core::recommendation::Recommendation> {
    crate::ipc::blocking(state.inner(), "recommendation for", move |state| {
        recommendation_for_impl(state)
    })
    .await
}
