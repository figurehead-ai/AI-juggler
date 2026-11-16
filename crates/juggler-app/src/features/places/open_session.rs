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
//!
//! 0.4.6 (the rc2 walk): a profile just started can miss that link: it
//! restored the session it last had open (another "Echo lab", its own
//! `claude --resume`) and the link to the one asked for came before it
//! was ready. Once started, the link goes again, as a hand-off's does,
//! until the app's own record of that session (by its id) says it came
//! in front; only then does Go to it say the session opened.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

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
        SessionLink::of(&app_session, &link),
        state,
        system,
        desktop,
        links_may_ask,
        waits,
    ))
}

/// The session a link opens: the app's own id for it (its record's) and
/// the link that names it.
#[derive(Clone, Copy, Debug)]
pub struct SessionLink<'a> {
    /// The app's id for the session (the Claude app's `local_<uuid>`).
    pub record: &'a str,
    /// Its own link to it.
    pub link: &'a str,
}

impl<'a> SessionLink<'a> {
    /// Session `record` and its `link`.
    #[must_use]
    pub const fn of(record: &'a str, link: &'a str) -> Self {
        Self { record, link }
    }
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
    target: SessionLink<'_>,
    state: &Arc<AppState>,
    system: &dyn System,
    desktop: &dyn Desktop,
    links_may_ask: bool,
    waits: Waits,
) -> GoToOutcomeInfo {
    let link = target.link;
    let asked_ms = crate::commands::now_millis();
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
        // Just started: sent again until the app's record of the session
        // says it's in front (it may have restored another one instead).
        LinkSent::Sent
            if started
                && !came_in_front(state, place, target.record, asked_ms, waits, || {
                    desktop.send_link_to(pid, link, links_may_ask)
                }) =>
        {
            outcome(
                format!(
                    "Started {name}, but it opened another session: pick this one in its sidebar"
                ),
                "app",
            )
        }
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

/// The app's own record of its session `record` (its file is named by the
/// app's id for it) in profile `place`, and the adapter that reads it.
fn record_of(
    state: &Arc<AppState>,
    place: &KnownPlace,
    record: &str,
) -> Option<(juggler_adapter::SharedAdapter, PathBuf)> {
    let dir = place.instance_dir.as_deref()?;
    state.registry.ids().into_iter().find_map(|adapter_id| {
        let adapter = state.registry.get(&adapter_id)?;
        if !adapter.host_apps().iter().any(|app| app.id == place.app_id) {
            return None;
        }
        let path = adapter.app_session_files(dir).into_iter().find(|path| {
            path.file_stem()
                .is_some_and(|stem| stem.to_string_lossy() == record)
        })?;
        Some((adapter, path))
    })
}

/// When the app last had the session in front, as its record says.
fn in_front_ms(adapter: &juggler_adapter::SharedAdapter, path: &Path) -> Option<i64> {
    adapter.read_app_session(path)?.last_focused_ms
}

/// 0.4.6 (the rc2 walk): whether the app brought session `record` in front
/// since `since_ms` (before it was started), by its own record of it; the
/// link `again` every `waits.resend` until it does, for at most
/// `waits.arrive` (the first was sent already). Without a record to read,
/// it can't be told: not opened.
fn came_in_front(
    state: &Arc<AppState>,
    place: &KnownPlace,
    record: &str,
    since_ms: i64,
    waits: Waits,
    again: impl Fn() -> LinkSent,
) -> bool {
    let Some((adapter, path)) = record_of(state, place, record) else {
        return false;
    };
    let until = Instant::now() + waits.arrive;
    loop {
        let left = until.saturating_duration_since(Instant::now());
        if wait_for(waits.resend.min(left), waits.every, || {
            in_front_ms(&adapter, &path)
                .is_some_and(|at| at >= since_ms)
                .then_some(())
        })
        .is_some()
        {
            return true;
        }
        if Instant::now() >= until || again() != LinkSent::Sent {
            return false;
        }
    }
}
