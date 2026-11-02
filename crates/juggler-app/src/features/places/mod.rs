//! 0.3.3 (maintainer, 2026-10-31: "Both for 0.3.3"): the desktop apps and
//! profiles the user runs, remembered, so that
//!
//! - **Open** starts one that isn't running (Home's tiles, the rail's
//!   place, Settings › Desktop apps), and **Open in {app}** reopens an
//!   ended session's app (Codex at the thread itself, by its link);
//! - **Keep alive**, turned on per app or profile, starts it again when
//!   it stops ([`keep`]; SPEC §1's third amendment).
//!
//! A place is started exactly as it last ran: its bundle and the
//! instance folder it was given ([`start`]), so an account never changes.
//! What is remembered is in [`book`]; how it's found, in the live view's
//! scan (`conversations::places::running_apps`).

pub mod book;
pub mod commands;
pub mod keep;
pub mod start;

use std::sync::{Arc, Mutex, OnceLock};

use book::{Book, KnownPlace};
use keep::{Act, Kept, Watch};

use crate::error::{AppError, AppResult};
use crate::features::conversations::dto::GoToOutcomeInfo;
use crate::features::conversations::goto::{self, Desktop};
use crate::features::conversations::places::{Host, HostRegistry, RunningApp, running_apps};
use crate::features::conversations::procs;
use crate::features::conversations::scan::LiveEntry;
use crate::state::AppState;

/// The event the window hears when a place changed (started, stopped,
/// kept alive, paused, forgotten).
pub const EVENT_CHANGED: &str = "places-changed";

/// How often the whole process list is read to find a kept-alive place
/// whose process isn't known (only while one is down).
const SEARCH_EVERY_MS: i64 = 10_000;

/// The places remembered, and the keep-alive watch.
#[derive(Default)]
pub struct Places {
    book: Mutex<Option<Book>>,
    /// The keep-alive watch (public for the live walk's seam).
    #[doc(hidden)]
    pub watch: Mutex<Watch>,
    /// Tells the window (set once the app is up).
    pub on_change: OnceLock<Box<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for Places {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Places").finish_non_exhaustive()
    }
}

fn changed(state: &Arc<AppState>) {
    if let Some(tell) = state.places.on_change.get() {
        tell();
    }
}

/// Runs `body` on the book (read from storage the first time), and
/// stores it when `body` changed it. Returns `body`'s answer.
///
/// # Errors
///
/// A poisoned lock; storage errors.
pub fn with_book<T>(state: &Arc<AppState>, body: impl FnOnce(&mut Book) -> T) -> AppResult<T> {
    let mut slot = state
        .places
        .book
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    if slot.is_none() {
        let stored = crate::commands::lock_storage_arc(state)?
            .app_settings()
            .get(book::KEY)?;
        *slot = Some(Book::parse(stored.as_deref()));
    }
    let Some(book) = slot.as_mut() else {
        return Err(AppError::StatePoisoned);
    };
    let before = book.clone();
    let answer = body(book);
    if *book != before {
        crate::commands::lock_storage_arc(state)?
            .app_settings()
            .set(book::KEY, &book.to_json())?;
    }
    Ok(answer)
}

/// The live view's full scan found `running` with `entries` open: they
/// are remembered, and each running place's process noted.
pub fn observe(state: &Arc<AppState>, running: &[RunningApp], entries: &[LiveEntry]) {
    let hosts = HostRegistry::of(&state.registry);
    let home = dirs::home_dir().unwrap_or_default();
    let now = crate::commands::now_millis();
    let Ok((changed_book, watched)) = with_book(state, |book| {
        let changed = book.observe(running, entries, &hosts, &home, now);
        let watched: Vec<String> = book
            .places
            .iter()
            .filter(|(_, place)| place.watched())
            .map(|(id, _)| id.clone())
            .collect();
        (changed, watched)
    }) else {
        return;
    };
    let moved = state.places.watch.lock().is_ok_and(|mut watch| {
        let before = watch.known();
        for app in running {
            let id = book::place_id(&app.app_id, app.instance.as_deref());
            watch.found(&id, app.pid, app.start);
        }
        // A place not running now and not watched: no process (a watched
        // one's end is the watch's to see, two misses in a row).
        for (id, _, _) in &before {
            let still = running
                .iter()
                .any(|app| book::place_id(&app.app_id, app.instance.as_deref()) == *id);
            if !still && !watched.contains(id) {
                watch.forget_pid(id);
            }
        }
        let mut after = watch.known();
        let mut was = before;
        after.sort();
        was.sort();
        after != was
    });
    if changed_book || moved {
        changed(state);
    }
}

/// The name of the account `place`'s sessions ran on.
fn account_label(state: &Arc<AppState>, place: &KnownPlace) -> Option<String> {
    let id = place.account_id.as_deref()?;
    state
        .storage
        .lock()
        .ok()?
        .accounts()
        .get(&juggler_core::model::AccountId::new(id.to_owned()))
        .ok()
        .map(|account| account.label)
}

/// `place` in words, its profile named by its account (as the screens
/// name it).
fn named(state: &Arc<AppState>, place: &KnownPlace) -> String {
    place.named(account_label(state, place).as_deref())
}

/// Why `place`'s account may only be used by a person at the keyboard.
fn only_by_you(state: &Arc<AppState>, place: &KnownPlace) -> Option<String> {
    let account_id = place.account_id.as_deref()?;
    let account = state
        .storage
        .lock()
        .ok()?
        .accounts()
        .get(&juggler_core::model::AccountId::new(account_id.to_owned()))
        .ok()?;
    let adapter = state.registry.get(&account.harness)?;
    adapter
        .interactive_only(&account.state_dir)
        .or_else(|| crate::features::service_accounts::interactive_only(state, &account))
}

/// Connects the window (it hears [`EVENT_CHANGED`]) and starts the
/// keep-alive watch, every 2 s.
pub fn spawn(app: tauri::AppHandle, state: Arc<AppState>) {
    let _ = state.places.on_change.set(Box::new(move || {
        use tauri::Emitter;
        let _ = app.emit(EVENT_CHANGED, ());
    }));
    crate::ticks::every("keep-alive", std::time::Duration::from_secs(2), move || {
        tick(&state);
    });
}

/// The keep-alive watch, every 2 s: checks each kept-alive place's
/// process, then starts or pauses as [`Watch::step`] says.
pub fn tick(state: &Arc<AppState>) {
    tick_with(
        state,
        crate::commands::now_millis(),
        &start::System,
        // A lock state that can't be read counts as locked: never a start
        // on a guess.
        &|| crate::focus::screen_locked().unwrap_or(true),
    );
}

/// [`tick`] at `now` with its seams (tests script the system and the
/// lock).
pub fn tick_with(
    state: &Arc<AppState>,
    now: i64,
    starter: &dyn start::Starter,
    locked: &dyn Fn() -> bool,
) {
    let Ok(kept) = with_book(state, |book| {
        book.places
            .iter()
            .filter(|(_, place)| place.watched())
            .map(|(id, place)| (id.clone(), place.clone()))
            .collect::<Vec<_>>()
    }) else {
        return;
    };
    let Ok(mut watch) = state.places.watch.lock() else {
        return;
    };
    watch.clock(now);
    if kept.is_empty() {
        return;
    }
    // Known processes: still the same ones?
    let pids: Vec<u32> = kept
        .iter()
        .filter_map(|(id, _)| watch.pid_of(id).map(|(pid, _)| pid))
        .collect();
    let alive = procs::some(&pids);
    // A place with no known process: looked for in the whole list, at
    // most every 10 s.
    let lost = kept.iter().any(|(id, _)| watch.pid_of(id).is_none());
    let search = lost
        && watch
            .searched_ms
            .is_none_or(|at| now.saturating_sub(at) >= SEARCH_EVERY_MS);
    let found = if search {
        watch.searched_ms = Some(now);
        let hosts = HostRegistry::of(&state.registry);
        let home = dirs::home_dir().unwrap_or_default();
        running_apps(&procs::table(), &hosts, &home)
    } else {
        Vec::new()
    };
    let mut places = Vec::new();
    for (id, place) in &kept {
        let running = match watch.pid_of(id) {
            Some((pid, start)) => {
                let same = alive
                    .get(&pid)
                    .is_some_and(|process| juggler_adapter::live::same_start(start, process.start));
                watch.seen(id, same)
            }
            None => {
                match found
                    .iter()
                    .find(|app| book::place_id(&app.app_id, app.instance.as_deref()) == *id)
                {
                    Some(app) => {
                        watch.found(id, app.pid, app.start);
                        true
                    }
                    None => false,
                }
            }
        };
        places.push(Kept {
            id: id.clone(),
            running,
            only_by_you: if running {
                None
            } else {
                only_by_you(state, place)
            },
        });
    }
    let acts = watch.step(now, &places, locked);
    drop(watch);
    if acts.is_empty() {
        return;
    }
    let hosts = HostRegistry::of(&state.registry);
    for act in acts {
        match act {
            Act::Start(id, attempt) => {
                let Some((_, place)) = kept.iter().find(|(known, _)| *known == id) else {
                    continue;
                };
                match start::start(place, &hosts, starter, true) {
                    Ok(()) => crate::features::notifications::sources::app_started_again(
                        state,
                        &id,
                        &named(state, place),
                        attempt,
                    ),
                    Err(why) => pause(state, &id, &why),
                }
            }
            Act::Pause(id, why) => pause(state, &id, &why),
        }
    }
    changed(state);
}

/// Pauses keep alive for `id`, and says why.
fn pause(state: &Arc<AppState>, id: &str, why: &str) {
    let paused = with_book(state, |book| {
        book.places.get_mut(id).map(|place| {
            place.paused = Some(why.to_owned());
            place.clone()
        })
    })
    .ok()
    .flatten();
    let label = paused.map(|place| named(state, &place));
    if let Some(label) = label {
        crate::features::notifications::sources::keep_alive_paused(state, id, &label, why);
    }
}

/// The user stopped process `pid` from AI Juggler: a kept-alive place
/// running as it pauses until its next Open (a stop is never undone by
/// keep alive).
pub fn stopped_here(state: &Arc<AppState>, pid: u32) {
    let known = state.places.watch.lock().ok().and_then(|watch| {
        watch
            .known()
            .into_iter()
            .find(|(_, known, _)| *known == pid)
            .map(|(id, _, _)| id)
    });
    // A place keep alive just started isn't in the watch until its next
    // look (up to 10 s, the live walk of 0.3.3): the process list says.
    let id = known.or_else(|| {
        let hosts = HostRegistry::of(&state.registry);
        let home = dirs::home_dir().unwrap_or_default();
        running_apps(&procs::table(), &hosts, &home)
            .into_iter()
            .find(|app| app.pid == pid)
            .map(|app| book::place_id(&app.app_id, app.instance.as_deref()))
    });
    let Some(id) = id else { return };
    let watched = with_book(state, |book| {
        book.places.get_mut(&id).is_some_and(|place| {
            if place.watched() {
                place.paused = Some("you stopped it in AI Juggler".to_owned());
                true
            } else {
                false
            }
        })
    })
    .unwrap_or(false);
    if watched {
        changed(state);
    }
}

/// Turns keep alive on or off for `id`. On, a place that isn't running is
/// started within seconds (after AI Juggler's first minute).
///
/// # Errors
///
/// [`AppError::Refused`] for a place that isn't remembered (or can't be
/// started); a poisoned lock; storage errors.
pub fn set_keep_alive(state: &Arc<AppState>, id: &str, on: bool) -> AppResult<()> {
    let hosts = HostRegistry::of(&state.registry);
    with_book(state, |book| {
        let place = book
            .places
            .get_mut(id)
            .ok_or_else(|| AppError::Refused("AI Juggler doesn't remember that app".to_owned()))?;
        if on {
            start::plan(place, &hosts, true).map_err(AppError::Refused)?;
        }
        place.keep_alive = on;
        place.paused = None;
        Ok::<(), AppError>(())
    })??;
    if let Ok(mut watch) = state.places.watch.lock() {
        watch.drop_place(id);
    }
    changed(state);
    Ok(())
}

/// Forgets `id` (it's remembered again the next time it runs).
///
/// # Errors
///
/// A poisoned lock; storage errors.
pub fn forget(state: &Arc<AppState>, id: &str) -> AppResult<()> {
    with_book(state, |book| book.places.remove(id))?;
    if let Ok(mut watch) = state.places.watch.lock() {
        watch.drop_place(id);
        watch.forget_pid(id);
    }
    changed(state);
    Ok(())
}

/// The place as Go to it names it (`profile`, as the screens do),
/// running as `pid`.
fn host_of(place: &KnownPlace, profile: Option<String>, pid: u32, start: Option<i64>) -> Host {
    Host {
        app_id: Some(place.app_id.clone()),
        name: place.name.clone(),
        shown: place.name.clone(),
        kind: Some(juggler_adapter::live::HostKind::Desktop),
        pid,
        start,
        instance_dir: place.instance_dir.clone(),
        instance: profile,
    }
}

/// The process `id` runs as now: the watch's, when it's still that one,
/// else looked for in the whole list.
fn running_now(state: &Arc<AppState>, id: &str) -> Option<(u32, Option<i64>)> {
    let known = state
        .places
        .watch
        .lock()
        .ok()
        .and_then(|watch| watch.pid_of(id));
    if let Some((pid, start)) = known
        && procs::some(&[pid])
            .get(&pid)
            .is_some_and(|process| juggler_adapter::live::same_start(start, process.start))
    {
        return Some((pid, start));
    }
    let hosts = HostRegistry::of(&state.registry);
    let home = dirs::home_dir().unwrap_or_default();
    let app = running_apps(&procs::table(), &hosts, &home)
        .into_iter()
        .find(|app| book::place_id(&app.app_id, app.instance.as_deref()) == id)?;
    if let Ok(mut watch) = state.places.watch.lock() {
        watch.found(id, app.pid, app.start);
    }
    Some((app.pid, app.start))
}

/// Open: `id` brought forward when it runs, else started (in front).
/// With a session (`tool`, `conversation`), an app with a link to its
/// sessions (Codex) opens at it. Open also ends a pause of keep alive.
///
/// # Errors
///
/// [`AppError::Refused`] for a place that isn't remembered; a poisoned
/// lock; storage errors.
pub fn open(
    state: &Arc<AppState>,
    id: &str,
    conversation: Option<&str>,
    desktop: Arc<dyn Desktop>,
    starter: Arc<dyn start::Starter>,
) -> AppResult<GoToOutcomeInfo> {
    let place = with_book(state, |book| {
        book.places.get_mut(id).map(|place| {
            place.paused = None;
            place.clone()
        })
    })?
    .ok_or_else(|| AppError::Refused("AI Juggler doesn't remember that app".to_owned()))?;
    if let Ok(mut watch) = state.places.watch.lock() {
        watch.drop_place(id);
    }
    changed(state);
    let hosts = HostRegistry::of(&state.registry);
    let account = account_label(state, &place);
    let label = place.named(account.as_deref());
    let profile = place.profile(account.as_deref());
    let link = conversation.and_then(|conversation| {
        hosts
            .by_id(&place.app_id)
            .and_then(|host| host.thread_link_for(conversation))
    });
    if let Some(link) = link {
        let opened = desktop.open_link(&link);
        return Ok(outcome(
            if opened {
                format!("Opened it in {label}")
            } else {
                format!("Couldn't open it in {label}")
            },
            if opened { "thread" } else { "none" },
        ));
    }
    let pick = if conversation.is_some() {
        "; pick the session in its sidebar"
    } else {
        ""
    };
    let state = Arc::clone(state);
    let id = id.to_owned();
    let still = label.clone();
    Ok(goto::within(goto::LIMIT, move || {
        if let Some((pid, start)) = running_now(&state, &id) {
            let mut said =
                goto::bring_forward(&host_of(&place, profile, pid, start), desktop.as_ref());
            if said.reached == "app" {
                said.outcome.push_str(pick);
            }
            return said;
        }
        match start::start(&place, &hosts, starter.as_ref(), false) {
            Ok(()) => outcome(format!("Starting {label}{pick}"), "started"),
            Err(why) => outcome(why, "none"),
        }
    })
    .unwrap_or_else(|| outcome(format!("Still opening {still}…"), "pending")))
}

fn outcome(text: String, reached: &str) -> GoToOutcomeInfo {
    GoToOutcomeInfo {
        outcome: text,
        reached: reached.to_owned(),
    }
}

#[cfg(test)]
mod tests;
