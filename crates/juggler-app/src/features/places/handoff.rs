//! 0.3.5: a switch continues where the user picks, in the desktop app or
//! in a terminal: "Desktop" or "CLI".
//!
//! Desktop hands the session to the target account's profile of the app,
//! the way Claude Code's own `--desktop` and the user's own launcher do:
//! the app takes a session of its tool by a link
//! ([`HostApp::handoff_link`](juggler_adapter::live::HostApp::handoff_link)),
//! reading it from the profile's tool folder, where the switch put it. The
//! link goes to that profile's own process, never to the app at large
//! (macOS hands an app's links to whichever instance it picks, and with
//! one instance per profile that can be another account's). A profile that
//! isn't running is started first, on its tool folder. The switch says
//! "switched" only once the profile recorded the session
//! ([`HostApp::handoff_record`](juggler_adapter::live::HostApp::handoff_record)).
//!
//! A profile that's running is never started a second time: a second
//! process on one profile folder doesn't hand anything over (the walk of
//! 2026-11-02 found the running one only came forward) and two processes
//! on one folder put its data at risk. 0.3.8: nor is one that runs on but
//! lost its window when the system restarted the display; the switch says so,
//! and Open offers the restart.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::book::KnownPlace;
use super::{named, running_now, start, tool_folder, with_book};
use crate::features::conversations::goto;
use crate::features::conversations::places::HostRegistry;
use crate::state::AppState;

/// Where a switch can continue (the confirm step's two choices).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DestinationsInfo {
    /// `desktop` or `cli`: where the source ran, so the usual choice is
    /// to stay there.
    pub suggested: String,
    /// The target account's profile of a desktop app that takes its
    /// sessions, when AI Juggler knows one.
    pub desktop: Option<DesktopDestinationInfo>,
    /// Why Desktop can't be picked, in words, when there's none.
    pub desktop_why_not: Option<String>,
}

/// The desktop profile a switch can continue in.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DesktopDestinationInfo {
    /// The remembered place's id.
    pub place_id: String,
    /// Its name on screen ("Claude app · Work Max").
    pub name: String,
    /// The app's registry id (its icon).
    pub app_id: String,
    /// Running now; when not, the switch starts it.
    pub running: bool,
}

/// The two choices for a switch to `account` (its id and label) of a
/// session that `ran_in_desktop`.
#[must_use]
pub fn destinations(
    state: &Arc<AppState>,
    account: &str,
    label: &str,
    ran_in_desktop: bool,
) -> DestinationsInfo {
    let desktop = profile_for(state, account).map(|(id, place)| DesktopDestinationInfo {
        running: running_now(state, &id).is_some(),
        name: named(state, &place),
        app_id: place.app_id.clone(),
        place_id: id,
    });
    let desktop_why_not = desktop
        .is_none()
        .then(|| why_no_desktop(state, account, label));
    DestinationsInfo {
        suggested: if ran_in_desktop && desktop.is_some() {
            "desktop"
        } else {
            "cli"
        }
        .to_owned(),
        desktop,
        desktop_why_not,
    }
}

/// Why `account` has no Desktop: its app can't take a session (`ChatGPT`
/// today), or AI Juggler hasn't seen a profile of an app that can.
fn why_no_desktop(state: &Arc<AppState>, account: &str, label: &str) -> String {
    let hosts = HostRegistry::of(&state.registry);
    let app = with_book(state, |book| {
        book.places
            .values()
            .filter(|place| place.account_id.as_deref() == Some(account))
            .find_map(|place| hosts.by_id(&place.app_id))
            .map(|host| host.shown_name().to_owned())
    })
    .ok()
    .flatten();
    app.map_or_else(
        || format!("No desktop app profile on {label} yet. Open a session on it in the app once, and Desktop is offered here."),
        |app| format!("{app} can't pick up a switched session yet."),
    )
}

/// The remembered profile whose sessions run on `account`, of an app that
/// takes sessions: the running one first, else the one seen last.
fn profile_for(state: &Arc<AppState>, account: &str) -> Option<(String, KnownPlace)> {
    let hosts = HostRegistry::of(&state.registry);
    let places = with_book(state, |book| {
        book.places
            .iter()
            .filter(|(_, place)| place.account_id.as_deref() == Some(account))
            .filter(|(_, place)| {
                hosts.by_id(&place.app_id).is_some_and(|host| {
                    host.handoff_link.is_some() && host.handoff_record.is_some()
                })
            })
            .map(|(id, place)| (id.clone(), place.clone()))
            .collect::<Vec<_>>()
    })
    .ok()?;
    let running = places
        .iter()
        .find(|(id, _)| running_now(state, id).is_some())
        .cloned();
    running.or_else(|| places.into_iter().max_by_key(|(_, place)| place.seen_ms))
}

/// A profile's main process, as found now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Running {
    /// Its pid.
    pub pid: u32,
    /// 0.3.8: it runs on but lost its window when macOS restarted its
    /// display: it takes no link, and a second process beside it is
    /// never started.
    pub lost: bool,
}

/// What a hand-off needs from the system (a seam: the tests script it).
pub trait System: Send + Sync {
    /// The profile's main process, when it runs.
    fn running(&self, state: &Arc<AppState>, place_id: &str) -> Option<Running>;
    /// Starts the profile on its tool folder.
    ///
    /// # Errors
    ///
    /// Why it didn't start, in words.
    fn start(&self, state: &Arc<AppState>, place: &KnownPlace) -> Result<(), String>;
    /// Sends `link` to process `pid` alone.
    ///
    /// # Errors
    ///
    /// Why it couldn't, in words.
    fn send(&self, pid: u32, link: &str) -> Result<(), String>;
}

/// The real system.
#[derive(Clone, Copy, Debug, Default)]
pub struct Real;

impl System for Real {
    fn running(&self, state: &Arc<AppState>, place_id: &str) -> Option<Running> {
        running_now(state, place_id).map(|(pid, start)| Running {
            pid,
            lost: juggler_platform::apps::detached(pid, start),
        })
    }

    fn start(&self, state: &Arc<AppState>, place: &KnownPlace) -> Result<(), String> {
        let hosts = HostRegistry::of(&state.registry);
        start::start(
            place,
            &hosts,
            tool_folder(state, place).as_deref(),
            &start::System,
            false,
        )
    }

    fn send(&self, pid: u32, link: &str) -> Result<(), String> {
        juggler_platform::apps::send_link(pid, link).map_err(|error| error.to_string())
    }
}

/// How long a hand-off waits: for a started profile to come up, and for
/// the profile to record the session.
#[derive(Clone, Copy, Debug)]
pub struct Waits {
    /// For the profile's process after starting it.
    pub start: Duration,
    /// For the profile's record of the session.
    pub arrive: Duration,
    /// Between looks.
    pub every: Duration,
}

impl Default for Waits {
    fn default() -> Self {
        Self {
            start: Duration::from_secs(20),
            arrive: Duration::from_secs(25),
            every: Duration::from_millis(250),
        }
    }
}

/// Hands session `session_id` (in the profile's tool folder already) to
/// profile `place_id`: its process, started first when it isn't running.
/// Returns that process once the profile recorded the session.
///
/// # Errors
///
/// In words: the profile isn't remembered or takes no sessions, it didn't
/// start, the link couldn't be sent, or the profile didn't record it.
pub fn hand_off(
    state: &Arc<AppState>,
    place_id: &str,
    session_id: &str,
    system: &dyn System,
    waits: Waits,
) -> Result<u32, String> {
    let place = with_book(state, |book| book.places.get(place_id).cloned())
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "AI Juggler doesn't remember that app profile any more".to_owned())?;
    let name = named(state, &place);
    let hosts = HostRegistry::of(&state.registry);
    let host = hosts
        .by_id(&place.app_id)
        .ok_or_else(|| format!("AI Juggler doesn't know {name} any more"))?;
    let link = host
        .handoff_link_for(session_id)
        .ok_or_else(|| format!("{name} can't take a session from AI Juggler"))?;
    let records = match (&place.instance_dir, &host.handoff_record) {
        (Some(dir), Some(record)) => dir.join(record),
        _ => return Err(format!("{name} can't take a session from AI Juggler")),
    };
    let pid = match system.running(state, place_id) {
        Some(Running { lost: true, .. }) => return Err(lost(&name)),
        Some(Running { pid, .. }) => pid,
        None => {
            system.start(state, &place)?;
            wait_for(waits.start, waits.every, || {
                system
                    .running(state, place_id)
                    .filter(|running| !running.lost)
                    .map(|running| running.pid)
            })
            .ok_or_else(|| format!("{name} didn't finish starting"))?
        }
    };
    system
        .send(pid, &link)
        .map_err(|why| format!("Couldn't hand it to {name}: {why}"))?;
    wait_for(waits.arrive, waits.every, || {
        taken(&records, session_id).then_some(())
    })
    .ok_or_else(|| {
        format!("{name} got the session but hasn't opened it yet: look at its window")
    })?;
    Ok(pid)
}

/// The switch's words for a profile that lost its window: what happened,
/// and where its restart is.
fn lost(name: &str) -> String {
    format!(
        "{}. Restart it from Home or Settings › Desktop apps, then switch again",
        goto::lost_happened(name)
    )
}

fn wait_for<T>(limit: Duration, every: Duration, mut look: impl FnMut() -> Option<T>) -> Option<T> {
    let until = Instant::now() + limit;
    loop {
        if let Some(found) = look() {
            return Some(found);
        }
        if Instant::now() >= until {
            return None;
        }
        std::thread::sleep(every);
    }
}

/// The most folders looked into, and how deep, for a profile's record.
const LOOK_MOST: usize = 2_000;
const LOOK_DEPTH: usize = 4;

/// Whether the profile recorded session `id`: a `local_{id}.json` below
/// `records`. Only names are read, links are never followed, and the
/// walk is bounded.
pub(crate) fn taken(records: &Path, id: &str) -> bool {
    let wanted = format!("local_{id}.json");
    let mut folders: Vec<(PathBuf, usize)> = vec![(records.to_path_buf(), 0)];
    let mut looked = 0_usize;
    while let Some((folder, depth)) = folders.pop() {
        looked += 1;
        if looked > LOOK_MOST {
            return false;
        }
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_file() && entry.file_name().to_str() == Some(wanted.as_str()) {
                return true;
            }
            if kind.is_dir() && depth < LOOK_DEPTH {
                folders.push((entry.path(), depth + 1));
            }
        }
    }
    false
}

#[cfg(test)]
#[path = "handoff_tests.rs"]
mod tests;
