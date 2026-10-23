//! AJ-E1509 Req.2, 3 and 5 — the Mac kept working for this Mac's
//! sessions, and This Mac saying so. Every 5 s (the This Mac cadence):
//! how many sessions here are working (running or starting; not waiting
//! for the user, stuck or at a limit) and supervised, the power source,
//! and Settings › Sessions decide what the supervisor's `AwakeKeeper`
//! holds. Settings changes apply on the next tick.

use std::sync::{Arc, Mutex};

use juggler_core::model::SessionStatus;
use juggler_supervisor::awake::{AwakeKeeper, AwakePlan};

use crate::settings_policy::SessionPrefsInfo;
use crate::state::AppState;

/// This Mac's row: what keeps the Mac awake now, and why (or why not).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AwakeInfo {
    /// Sessions working on this Mac now.
    pub working: u32,
    /// AI Juggler is keeping the Mac from idle sleep now.
    pub kept_awake: bool,
    /// On battery (the battery setting applies).
    pub on_battery: bool,
}

/// The keeper and what it last did.
#[derive(Debug, Default)]
pub struct Awake {
    keeper: AwakeKeeper,
    info: AwakeInfo,
}

/// Pure: what to hold, from the settings, the power source and the
/// sessions here.
#[must_use]
pub fn plan(
    prefs: &SessionPrefsInfo,
    on_battery: bool,
    working: u32,
    supervised: u32,
) -> AwakePlan {
    AwakePlan {
        working,
        supervised,
        // "On battery too": the battery switch adds to the main one.
        keep_awake: prefs.keep_awake_on_power && (!on_battery || prefs.keep_awake_on_battery),
    }
}

/// Sessions on this Mac: (working, supervised).
fn counts(state: &Arc<AppState>) -> (u32, u32) {
    let here = crate::commands::local_machine_id(state);
    let Ok(storage) = state.storage.lock() else {
        return (0, 0);
    };
    let sessions = storage.sessions().list(None).unwrap_or_default();
    let mine = sessions.iter().filter(|record| record.machine == here);
    let (mut working, mut supervised) = (0_u32, 0_u32);
    for record in mine {
        if crate::commands::is_active(record.status) {
            supervised = supervised.saturating_add(1);
        }
        if matches!(
            record.status,
            SessionStatus::Running | SessionStatus::Starting
        ) {
            working = working.saturating_add(1);
        }
    }
    (working, supervised)
}

/// One tick: the keeper made to match the sessions, the power source
/// and the settings.
pub fn tick(state: &Arc<AppState>) {
    let (working, supervised) = counts(state);
    let on_battery = state
        .machine
        .lock()
        .ok()
        .and_then(|watch| watch.sampler.snapshot().power.map(|power| power.on_battery))
        .unwrap_or(false);
    let prefs = state
        .storage
        .lock()
        .map(|storage| crate::settings_policy::always_on(&storage))
        .unwrap_or_default();
    let wanted = plan(&prefs, on_battery, working, supervised);
    if let Ok(mut awake) = state.awake.lock() {
        awake.keeper.apply(wanted);
        awake.info = AwakeInfo {
            working,
            kept_awake: awake.keeper.awake_for().is_some(),
            on_battery,
        };
    }
}

/// What This Mac says now.
#[must_use]
pub fn current(awake: &Mutex<Awake>) -> AwakeInfo {
    awake.lock().map(|awake| awake.info).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_power_by_default_on_battery_only_when_asked() {
        let defaults = SessionPrefsInfo::default();
        assert!(plan(&defaults, false, 2, 2).keep_awake);
        assert!(!plan(&defaults, true, 2, 2).keep_awake);
        let battery_too = SessionPrefsInfo {
            keep_awake_on_battery: true,
            ..defaults.clone()
        };
        assert!(plan(&battery_too, true, 2, 2).keep_awake);
        let never = SessionPrefsInfo {
            keep_awake_on_power: false,
            keep_awake_on_battery: true,
            ..defaults
        };
        assert!(!plan(&never, false, 2, 2).keep_awake);
        assert!(
            !plan(&never, true, 2, 2).keep_awake,
            "battery too needs the main switch"
        );
    }
}
