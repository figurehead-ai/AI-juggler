//! AJ-E309: the local onboarding funnel: events appended to this Mac's store,
//! and their export.

pub mod commands;

use std::sync::Arc;

use crate::commands::{core_to_app, lock_storage_arc};
use crate::error::AppResult;
use crate::state::AppState;

/// AJ-E309: the install moment the funnel measures from — written
/// once, on the first funnel event this data dir ever records.
const FUNNEL_INSTALL_AT_KEY: &str = "funnel_install_at_ms";

/// AJ-E309: set once the first Instance launch has been recorded
/// (`first_launch` is a once-per-install event).
const FUNNEL_FIRST_LAUNCH_KEY: &str = "funnel_first_launch_recorded";

/// Core implementation shared with integration tests (`now_ms` is
/// injected so the dry-run export is deterministic).
///
/// The first event ever recorded stamps the install moment;
/// `first_launch` is recorded once per install, with
/// `seconds_since_install` computed HERE (the UI never supplies it).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) per [`funnel_event`](crate::features::funnel::commands::funnel_event).
pub fn funnel_event_impl(
    state: &Arc<AppState>,
    kind: &str,
    fields: std::collections::BTreeMap<String, String>,
    now_ms: i64,
) -> AppResult<()> {
    let mut event = juggler_core::funnel_events::FunnelEvent {
        kind: kind.to_owned(),
        at_ms: now_ms,
        fields,
    };
    juggler_core::funnel_events::validate(&event).map_err(core_to_app)?;
    let first_launch = kind == "first_launch";
    {
        let storage = lock_storage_arc(state)?;
        let settings = storage.app_settings();
        let stored = settings
            .get(FUNNEL_INSTALL_AT_KEY)?
            .and_then(|raw| raw.parse::<i64>().ok());
        let install_at = if let Some(at) = stored {
            at
        } else {
            settings.set(FUNNEL_INSTALL_AT_KEY, &now_ms.to_string())?;
            now_ms
        };
        if first_launch {
            if settings.get(FUNNEL_FIRST_LAUNCH_KEY)?.is_some() {
                return Ok(());
            }
            event.fields.clear();
            event.fields.insert(
                "seconds_since_install".to_owned(),
                ((now_ms - install_at).max(0) / 1000).to_string(),
            );
        }
    }
    juggler_core::funnel_events::FunnelLog::open(&state.data_dir)
        .append(&event)
        .map_err(core_to_app)?;
    // AJ-E1103: the onboarding steps the funnel already records, counted
    // (its fields are schema-checked counts and words).
    crate::usage_stats::funnel_event(kind, &event.fields);
    if first_launch {
        lock_storage_arc(state)?
            .app_settings()
            .set(FUNNEL_FIRST_LAUNCH_KEY, "1")?;
    }
    Ok(())
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) on storage failure.
pub fn funnel_export_impl(state: &Arc<AppState>) -> AppResult<String> {
    let events = juggler_core::funnel_events::FunnelLog::open(&state.data_dir)
        .read()
        .map_err(core_to_app)?;
    Ok(juggler_core::funnel_events::report(&events))
}
