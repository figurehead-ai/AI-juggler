//! AJ-E1547: the system's notifications, behind the gate. The first
//! notification an app shows makes the system ask whether it may notify
//! ("“AI Juggler” Notifications … Allow"), so:
//!
//! - background delivery (a session waits, a watch fires; watches are on
//!   from the start) shows nothing until the `notifications` entry is
//!   primed: the bell and Needs you still hold every item;
//! - the user's own turning on (Settings, Send test) goes through the
//!   sheet first, then shows a first notification so the system asks
//!   while they're there.
//!
//! The only use of the notification plugin (`scripts/os_prompt_lint.py`).

use std::sync::{Arc, OnceLock};

use juggler_platform::os_prompt::{self, Gate, Gated, Presence};

use super::registry::NOTIFICATIONS;
use crate::features::notifications::Outgoing;
use crate::state::AppState;

type Show = Box<dyn Fn(&Outgoing) + Send + Sync>;

/// The system's own notification, ungated: reached only through
/// [`deliver`] (the notifier's sink) and [`show_first`].
static SHOW: OnceLock<Show> = OnceLock::new();

/// The id the first notification carries (a click brings AI Juggler
/// forward, as an unknown id does).
const FIRST_ID: i32 = 1_547;

/// Delivers `outgoing` from background work: only once the entry is
/// primed (or `gate` says so), so the system never asks with nobody told.
/// Whether it was shown.
pub fn deliver(gate: Gate<'_>, outgoing: &Outgoing, show: &dyn Fn(&Outgoing)) -> bool {
    matches!(
        gate.through(NOTIFICATIONS, Presence::Background, &|| None, || show(
            outgoing
        )),
        Gated::Called { .. }
    )
}

/// Whether kinds may be turned on now, at the user's click: the entry is
/// primed, or the user said Continue on the sheet (then the first
/// notification shows, so the system asks while they're there).
#[must_use]
pub fn may_turn_on() -> bool {
    os_prompt::gate().primed(NOTIFICATIONS.id)
        || matches!(
            os_prompt::gate().through(NOTIFICATIONS, os_prompt::user(), &|| None, show_first),
            Gated::Called { .. }
        )
}

/// Whether Send test may show its notification now, at the user's click:
/// the entry is primed, or the user said Continue on the sheet (the
/// entry is then primed, and the test, through the notifier's own sink,
/// is the first notification, so the system asks while they're there).
#[must_use]
pub fn may_send_test() -> bool {
    matches!(
        os_prompt::gate().through(NOTIFICATIONS, os_prompt::user(), &|| None, || ()),
        Gated::Called { .. }
    )
}

/// The first notification, shown right after the user said go, so the
/// system asks while they're there.
pub fn show_first() {
    if let Some(show) = SHOW.get() {
        show(&Outgoing {
            id: FIRST_ID,
            title: crate::edition::current().app_name().to_owned(),
            body: "Notifications are on. You can choose which in Settings.".to_owned(),
        });
    }
}

/// The system's own notifications (the Tauri plugin), behind the gate,
/// clicks routed back to [`crate::features::notifications::clicked`].
pub fn connect(app: &tauri::AppHandle, state: &Arc<AppState>) {
    use tauri_plugin_notification::NotificationExt;
    let shower = app.clone();
    let _ = SHOW.set(Box::new(move |outgoing: &Outgoing| {
        let _ = shower
            .notification()
            .builder()
            .id(outgoing.id)
            .title(&outgoing.title)
            .body(&outgoing.body)
            .show();
    }));
    state.notifier.set_sink(Box::new(|outgoing: &Outgoing| {
        if let Some(show) = SHOW.get() {
            deliver(os_prompt::gate(), outgoing, show.as_ref());
        }
    }));
    let clicks = app.clone();
    let click_state = Arc::clone(state);
    let _ = app.notification().on_action(move |performed| {
        if performed.action_id() != "tap" {
            return;
        }
        if let Some(id) = performed
            .notification()
            .map(tauri_plugin_notification::ActiveNotification::id)
        {
            crate::features::notifications::clicked(&clicks, &click_state, id);
        }
    });
}
