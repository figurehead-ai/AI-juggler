//! The alerts commands. Each runs off the main thread.

use crate::commands::now_millis;
use crate::error::AppResult;
use crate::features::alerts::alerts_tick_inner;
use crate::features::alerts::dto::AlertEventInfo;

/// AJ-E611: the alert evaluation pass (UI polls this on its own
/// cadence; emissions are at-most-once by the machines). The UI passes
/// its wall-clock local hour — the quiet-hours gate for the digest
/// flush.
#[tauri::command]
///
/// # Errors
///
/// The work could not finish off the main thread.
pub async fn alerts_tick(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    local_hour: Option<u32>,
) -> AppResult<Vec<AlertEventInfo>> {
    crate::ipc::blocking(state.inner(), "alerts tick", move |state| {
        state.notifier.local_hour_seen(local_hour, now_millis()); // AJ-E1510: quiet hours
        let events = alerts_tick_inner(state, local_hour);
        // AJ-E709: paired consoles hear of them (no-op with Link off); AJ-E2501: the model too.
        crate::link::publish_alerts(state, &events);
        crate::features::notifications::sources::quota_pass(state, &events);
        Ok(events)
    })
    .await
}
