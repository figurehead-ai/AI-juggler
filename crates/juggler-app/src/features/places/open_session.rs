//! 0.4.1 (the maintainer's finding): Go to it for a desktop app's session
//! that isn't open now. The profile that keeps it in its own records (the
//! Claude app's `local_<uuid>` under its instance folder) is started when
//! it isn't running, on its tool folder, as a hand-off starts it; then the
//! app's own link to that session goes to that profile's process alone
//! ([`HostApp::session_link`](juggler_adapter::live::HostApp::session_link)),
//! never to the app at large. A profile that runs on but lost its window
//! is never started beside itself: Go to it offers its restart. The link
//! is sent without letting macOS ask, unless the user asked for it
//! ("Open the session itself").

use std::sync::Arc;

use super::book::KnownPlace;
use super::handoff::{Running, System, Waits, wait_for};
use super::named;
use crate::features::conversations::dto::GoToOutcomeInfo;
use crate::features::conversations::goto::{self, Desktop, LinkSent, outcome};
use crate::features::conversations::places::HostRegistry;
use crate::state::AppState;

/// A remembered profile that keeps session `conversation_id` (of folder
/// `account_id`) in its own records: its id, the place, and the app's own
/// id for the session.
#[must_use]
pub fn keeping(
    state: &Arc<AppState>,
    account_id: &str,
    conversation_id: &str,
) -> Option<(String, KnownPlace, String)> {
    let places = super::places_on(state, account_id, |place, host| {
        place.instance_dir.is_some() && host.session_link.is_some()
    })?;
    places.into_iter().find_map(|(id, place)| {
        let own = app_session_in(state, &place, conversation_id)?;
        Some((id, place, own))
    })
}

/// The app's own id for session `conversation_id` when profile `place`
/// keeps it in its records.
#[must_use]
pub fn app_session_in(
    state: &Arc<AppState>,
    place: &KnownPlace,
    conversation_id: &str,
) -> Option<String> {
    let dir = place.instance_dir.as_deref()?;
    state.registry.ids().into_iter().find_map(|adapter_id| {
        let adapter = state.registry.get(&adapter_id)?;
        if !adapter.host_apps().iter().any(|app| app.id == place.app_id) {
            return None;
        }
        adapter.app_session_files(dir).into_iter().find_map(|path| {
            adapter
                .read_app_session(&path)
                .filter(|session| session.session_id.as_deref() == Some(conversation_id))
                .map(|session| session.host_session_id)
        })
    })
}

/// Opens session `conversation_id` in the profile that keeps it, starting
/// the profile first when it isn't running; `None` when no remembered
/// profile keeps it (Go to it says it isn't open).
pub fn open(
    state: &Arc<AppState>,
    account_id: &str,
    conversation_id: &str,
    system: &dyn System,
    desktop: &dyn Desktop,
    links_may_ask: bool,
    waits: Waits,
) -> Option<GoToOutcomeInfo> {
    let (place_id, place, app_session) = keeping(state, account_id, conversation_id)?;
    let name = named(state, &place);
    let link = HostRegistry::of(&state.registry)
        .by_id(&place.app_id)?
        .session_link_for(&app_session)?;
    Some(open_in(
        &name,
        &place_id,
        &place,
        &link,
        state,
        system,
        desktop,
        links_may_ask,
        waits,
    ))
}

/// [`open`] once the profile and its link are known.
#[allow(
    clippy::too_many_arguments,
    reason = "the profile, its link and the seams, each its own"
)]
pub(crate) fn open_in(
    name: &str,
    place_id: &str,
    place: &KnownPlace,
    link: &str,
    state: &Arc<AppState>,
    system: &dyn System,
    desktop: &dyn Desktop,
    links_may_ask: bool,
    waits: Waits,
) -> GoToOutcomeInfo {
    let (pid, started) = match system.running(state, place_id) {
        Some(Running { lost: true, .. }) => return goto::lost(name, Some(place_id.to_owned())),
        Some(Running { pid, .. }) => (pid, false),
        None => {
            // ADR-0028: a switched family waiting for this profile's
            // start nests now, before the app reads its records.
            if let Some(dir) = place.instance_dir.as_deref() {
                crate::features::handover::automation::apply_for_place(state, place_id, dir);
            }
            if let Err(why) = system.start(state, place) {
                return outcome(format!("Couldn't start {name}: {why}"), "none");
            }
            let Some(pid) = wait_for(waits.start, waits.every, || {
                system
                    .running(state, place_id)
                    .filter(|running| !running.lost)
                    .map(|running| running.pid)
            }) else {
                return outcome(format!("{name} didn't finish starting"), "none");
            };
            (pid, true)
        }
    };
    // The profile forward first, then its link to it alone.
    let _ = desktop.request(pid);
    let started_words = if started {
        format!("Started {name}")
    } else {
        format!("Brought {name} forward")
    };
    match desktop.send_link_to(pid, link, links_may_ask) {
        LinkSent::Sent => outcome(
            if started {
                format!("Started {name} and opened the session")
            } else {
                format!("Opened the session in {name}")
            },
            if started { "started" } else { "thread" },
        ),
        LinkSent::NeedsConsent => outcome(
            format!(
                "{started_words}. To open the session itself, AI Juggler needs to send {name} a link: the system asks you once"
            ),
            "consent",
        ),
        LinkSent::Denied => outcome(
            format!(
                "{started_words}; pick the session in its sidebar (to open it for you, allow AI Juggler in System Settings › Privacy & Security › Automation)"
            ),
            "app",
        ),
        LinkSent::Failed => outcome(
            format!("{started_words}; pick the session in its sidebar"),
            "app",
        ),
    }
}
