//! AJ-E410: a session's process tree for the Processes tab.

use std::sync::Arc;

use juggler_core::CoreError;
use juggler_core::model::SessionId;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::processes::dto::{ProcessRowInfo, ProcessesInfo, TreeSampleInfo};
use crate::state::AppState;

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the Instance is unknown.
pub fn instance_processes_arc(state: &Arc<AppState>, session_id: &str) -> AppResult<ProcessesInfo> {
    let (record_exists, supervised) = {
        let storage = lock_storage_arc(state)?;
        let exists = storage
            .sessions()
            .get(&SessionId::new(session_id.to_string()))
            .is_ok();
        (
            exists,
            state
                .supervisor
                .is_supervised(&SessionId::new(session_id.to_string())),
        )
    };
    if !record_exists {
        return Err(AppError::from(CoreError::NotFound {
            entity: "instance",
            id: session_id.to_string(),
        }));
    }
    let session = SessionId::new(session_id.to_string());
    let (rows, spark, unavailable) = {
        let Ok(collector) = state.resources.lock() else {
            return Ok(unavailable_processes("resources unavailable"));
        };
        let rows = collector.rows(&session).unwrap_or_default();
        let spark = collector.ring(&session);
        // Adopted/desktop-kind sessions this app never supervised have
        // no tree to read: honest unavailability, never flat-zero rows.
        let unavailable =
            (!supervised && rows.is_empty()).then(|| "resources unavailable".to_string());
        (rows, spark, unavailable)
    };
    Ok(ProcessesInfo {
        rows: rows
            .into_iter()
            .map(|row| ProcessRowInfo {
                pid: row.pid,
                parent_pid: row.parent_pid,
                name: row.name,
                cpu_pct: row.cpu_pct,
                rss_bytes: row.rss_bytes,
                age_ms: row.age_ms,
                child_count: row.child_count,
                depth: row.depth,
            })
            .collect(),
        spark: spark
            .into_iter()
            .map(|sample| TreeSampleInfo {
                at_ms: sample.at_ms,
                cpu_pct: sample.cpu_pct,
                rss_bytes: sample.rss_bytes,
            })
            .collect(),
        unavailable_reason: unavailable,
    })
}

/// An honest `resources unavailable` payload (Req.7 — never flat zeros).
fn unavailable_processes(reason: &str) -> ProcessesInfo {
    ProcessesInfo {
        rows: Vec::new(),
        spark: Vec::new(),
        unavailable_reason: Some(reason.to_string()),
    }
}
