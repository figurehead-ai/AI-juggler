//! 0.3.3 (maintainer, 2026-10-31: "Both for 0.3.3"): the desktop apps and
//! profiles the user runs, remembered, so that
//!
//! - **Open** starts one that isn't running (Home's tiles, the rail's
//!   place, Settings › Desktop apps), and **Open in {app}** reopens an
//!   ended session's app (Codex at the thread itself, by its link);
//! - **Keep alive**, turned on per app or profile, starts it again when
//!   it stops ([`keep`]; SPEC §1's third amendment).
//! - **Restart** (0.3.8) quits and starts again one that runs on but lost
//!   its window when the system restarted the display ([`restart`]): Open and
//!   Go to it ask first, keep alive does it on its own.
//!
//! A place is started exactly as it last ran: its bundle and the
//! instance folder it was given ([`start`]), so an account never changes.
//! What is remembered is in [`book`]; how it's found, in the live view's
//! scan (`conversations::places::running_apps`), and, once the user says
//! yes, beside a profile that ran ([`profiles`], AJ-E3203).

pub mod book;
pub mod commands;
pub mod handoff;
pub mod keep;
pub mod open_session;
pub mod profiles;
pub mod start;
mod store;

use crate::events::{AppEvent, EmitEvent as _};
use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

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
pub const EVENT_CHANGED: &str = AppEvent::PlacesChanged.name();

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
    /// 0.3.8: the places being restarted now (one restart at a time
    /// each: two would start two processes on one folder).
    restarting: Mutex<HashSet<String>>,
    /// What each added profile's own records call its conversations
    /// (`<folder>` [`IN_FOLDER`] `<tool>/<id>` → the app's title), read
    /// with its records: a row names a closed one as the app does.
    titles: Mutex<std::collections::HashMap<String, String>>,
    /// The unlinked profiles already looked for a folder for, this run
    /// (the look scans for folders: once each, not every minute).
    looked: Mutex<HashSet<String>>,
    /// ADR-0031: the profiles a move (a switch by move) holds closed while
    /// it changes their files: keep alive, Open, Restart and the automation
    /// watch leave them alone until the move lets go ([`MoveHold`]).
    moving: Mutex<HashSet<String>>,
}

/// ADR-0031: holds `ids` closed for a move while it lives: nothing in AI
/// Juggler starts them or writes their records meanwhile.
pub struct MoveHold {
    state: Arc<AppState>,
    ids: Vec<String>,
}

impl MoveHold {
    /// Holds `ids`, or none when another move holds one of them.
    #[must_use]
    pub fn take(state: &Arc<AppState>, ids: &[String]) -> Option<Self> {
        let mut now = state.places.moving.lock().ok()?;
        if ids.iter().any(|id| now.contains(id)) {
            return None;
        }
        now.extend(ids.iter().cloned());
        Some(Self {
            state: Arc::clone(state),
            ids: ids.to_vec(),
        })
    }
}

impl Drop for MoveHold {
    fn drop(&mut self) {
        if let Ok(mut now) = self.state.places.moving.lock() {
            for id in &self.ids {
                now.remove(id);
            }
        }
    }
}

/// Whether a move holds `id` closed now.
#[must_use]
pub fn held_for_move(state: &Arc<AppState>, id: &str) -> bool {
    state.places.moving.lock().is_ok_and(|now| now.contains(id))
}

/// Whether any move is running now.
#[must_use]
pub fn any_move(state: &Arc<AppState>) -> bool {
    state.places.moving.lock().is_ok_and(|now| !now.is_empty())
}

/// The words for a place a move holds.
const HELD: &str = "a switch is moving conversations into or out of this profile; it starts again by itself when the move is done";

impl std::fmt::Debug for Places {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Places").finish_non_exhaustive()
    }
}

fn changed(state: &Arc<AppState>) {
    // AJ-E3102: which App a session is in may have moved.
    crate::features::relations::changed(state, crate::features::relations::Change::PlacesChanged);
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
    let tool_of = |host: &str| tool_of(state, host);
    if slot.is_none() {
        *slot = Some(store::load(
            &*crate::commands::lock_storage_arc(state)?,
            &tool_of,
        )?);
    }
    let Some(book) = slot.as_mut() else {
        return Err(AppError::StatePoisoned);
    };
    let before = book.clone();
    let answer = body(book);
    if *book != before {
        store::save(
            &*crate::commands::lock_storage_arc(state)?,
            &before,
            book,
            &tool_of,
        )?;
    }
    Ok(answer)
}

/// The tool whose desktop app `host` is (the host's id when no adapter
/// names it).
fn tool_of(state: &Arc<AppState>, host: &str) -> String {
    state
        .registry
        .ids()
        .into_iter()
        .find(|id| {
            state
                .registry
                .get(id)
                .is_some_and(|adapter| adapter.host_apps().iter().any(|app| app.id == host))
        })
        .map_or_else(|| host.to_owned(), |id| id.to_string())
}

/// The live view's full scan found `running` with `entries` open: they
/// are remembered, and each running place's process noted.
pub fn observe(state: &Arc<AppState>, running: &[RunningApp], entries: &[LiveEntry]) {
    let hosts = HostRegistry::of(&state.registry);
    let home = juggler_platform::user::home().unwrap_or_default();
    let now = crate::commands::now_millis();
    let Ok((changed_book, watched, new_offers)) = with_book(state, |book| {
        let before: std::collections::BTreeSet<String> = book.places.keys().cloned().collect();
        let changed = book.observe(running, entries, &hosts, &home, now);
        // D-84: an app seen for the first time that isn't the user's is
        // offered once (a quiet notice), never added.
        let new_offers: Vec<(String, String)> = book
            .places
            .iter()
            .filter(|(id, place)| {
                !before.contains(*id) && !place.users && place.account_id.is_none()
            })
            .map(|(id, place)| (id.clone(), place.label()))
            .collect();
        let watched: Vec<String> = book
            .places
            .iter()
            .filter(|(_, place)| place.watched())
            .map(|(id, _)| id.clone())
            .collect();
        (changed, watched, new_offers)
    }) else {
        return;
    };
    if !new_offers.is_empty()
        && let Ok(storage) = crate::commands::lock_storage_arc(state)
    {
        for (id, label) in &new_offers {
            let _ = crate::features::notifications::centre::record(
                &storage,
                &crate::features::notifications::sources::found_notice(id, label),
                now,
            );
        }
    }
    let found = profiles::after_scan(state, &hosts, &home, now);
    let moved = state.places.watch.lock().is_ok_and(|mut watch| {
        let before = watch.known();
        for app in running {
            let id = book::place_id(&app.app_id, app.instance.as_deref());
            watch.found(&id, app.pid, app.start);
            watch.note_exe(&id, &app.exe);
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
    let recorded = read_records(state);
    if changed_book || found || moved || recorded {
        changed(state);
    }
}

/// The go sitting's rerun (2026-11-07): an added profile listed none of
/// its sessions unless one was seen open in it. Each added profile's own
/// records (its tool's adapter reads them, running or not) name its
/// sessions; the book takes them, and the app's title for each is kept
/// for the rows. A profile added before its folder was linked (the
/// rerun's lasting "needs its sign-in") is linked here to its folder when
/// an Account holds it already; bringing a new folder in waits for Add's
/// yes. Returns whether a place changed.
fn read_records(state: &Arc<AppState>) -> bool {
    let Ok(places) = with_book(state, |book| {
        book.places
            .iter()
            .filter(|(_, place)| place.users)
            .filter_map(|(id, place)| {
                Some((
                    id.clone(),
                    place.app_id.clone(),
                    place.instance_dir.clone()?,
                    place.account_id.is_none(),
                ))
            })
            .collect::<Vec<_>>()
    }) else {
        return false;
    };
    let mut changed = false;
    let mut titles = std::collections::HashMap::new();
    for (id, app_id, dir, unlinked) in places {
        let first_look = unlinked
            && state
                .places
                .looked
                .lock()
                .is_ok_and(|mut looked| looked.insert(id.clone()));
        if first_look {
            changed |= link_its_folder(state, &id, false);
        }
        let folder = with_book(state, |book| {
            book.places
                .get(&id)
                .and_then(|place| place.account_id.clone())
        })
        .ok()
        .flatten();
        for harness in state.registry.ids() {
            let Some(adapter) = state.registry.get(&harness) else {
                continue;
            };
            let Some(family) = adapter.host_family() else {
                continue;
            };
            if !adapter.host_apps().iter().any(|host| host.id == app_id) {
                continue;
            }
            let records = family.conversations(&dir);
            let keys: Vec<String> = records
                .iter()
                .map(|record| book::conversation_key(harness.as_str(), &record.id))
                .collect();
            if let Some(folder) = &folder {
                for (key, record) in keys.iter().zip(&records) {
                    if let Some(title) = &record.title {
                        titles.insert(in_folder(folder, key), title.clone());
                    }
                }
            }
            if !keys.is_empty() {
                changed |= with_book(state, |book| book.take_records(&id, &keys)).unwrap_or(false);
            }
            break;
        }
    }
    if let Ok(mut kept) = state.places.titles.lock() {
        changed |= *kept != titles;
        *kept = titles;
    }
    changed
}

/// What the added profiles' records call the conversation `key`
/// (`<tool>/<id>`) whose files are in `folder`: the app's own title.
#[must_use]
pub fn record_title(state: &Arc<AppState>, folder: &str, key: &str) -> Option<String> {
    state
        .places
        .titles
        .lock()
        .ok()?
        .get(&in_folder(folder, key))
        .cloned()
}

/// D-62 (AJ-E3204): the desktop App each session was last seen open in
/// (`<tool>/<id>` → App id), the most recently seen App winning, as the
/// rail puts a session under its app. A place whose folder is known
/// claims only the copy in that folder (`<folder>` [`IN_FOLDER`]
/// `<tool>/<id>`): a switch copies a conversation into another profile's
/// folder under the same id, and the go sitting's rerun listed every such
/// session twice on the profile's page. Read it with [`app_of`]. Empty
/// when the book can't be read.
pub fn desktop_app_by_session(state: &Arc<AppState>) -> std::collections::BTreeMap<String, String> {
    with_book(state, |book| {
        let mut places: Vec<(&String, &KnownPlace)> = book.places.iter().collect();
        places.sort_by_key(|(_, place)| std::cmp::Reverse(place.seen_ms));
        let mut out = std::collections::BTreeMap::new();
        for (id, place) in places {
            for key in &place.conversations {
                let key = place
                    .account_id
                    .as_deref()
                    .map_or_else(|| key.clone(), |folder| in_folder(folder, key));
                out.entry(key).or_insert_with(|| id.clone());
            }
        }
        out
    })
    .unwrap_or_default()
}

/// Between a folder and a `<tool>/<id>` key in [`desktop_app_by_session`].
pub const IN_FOLDER: char = '\u{1f}';

fn in_folder(folder: &str, key: &str) -> String {
    format!("{folder}{IN_FOLDER}{key}")
}

/// rc22 (A3): each folder exactly one of the user's desktop Apps runs
/// its sessions on (`folder` → App), as [`sole_holders`] reads the book.
/// Read it with [`app_of`].
pub fn desktop_app_by_folder(state: &Arc<AppState>) -> std::collections::BTreeMap<String, String> {
    with_book(state, |book| {
        sole_holders(
            book.places
                .iter()
                .filter(|(_, place)| place.users)
                .map(|(id, place)| (id.as_str(), place.account_id.as_deref())),
        )
    })
    .unwrap_or_default()
}

/// rc22 (A3): of `(App, its folder)` pairs, each folder exactly one App
/// holds → that App. A desktop profile's Claude Code folder (brought in
/// when it was added) is where its Code sessions run: they are that
/// profile's, not a terminal's (the rail's `|<folder>` rule,
/// `lib/rail-apps.ts`'s `appByFolder`). A folder two profiles share
/// names neither.
#[must_use]
pub fn sole_holders<'a>(
    apps: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
) -> std::collections::BTreeMap<String, String> {
    let mut held: std::collections::BTreeMap<String, Option<String>> =
        std::collections::BTreeMap::new();
    for (app, folder) in apps {
        let Some(folder) = folder else { continue };
        held.entry(folder.to_owned())
            .and_modify(|one| {
                if one.as_deref() != Some(app) {
                    *one = None;
                }
            })
            .or_insert_with(|| Some(app.to_owned()));
    }
    held.into_iter()
        .filter_map(|(folder, app)| Some((folder, app?)))
        .collect()
}

/// Where the desktop Apps put sessions: by the conversations each was
/// last seen open in ([`desktop_app_by_session`]), and by the folder one
/// runs its sessions on ([`desktop_app_by_folder`]).
#[derive(Clone, Debug, Default)]
pub struct DesktopApps {
    /// `<tool>/<id>` (or folder-qualified) → App.
    pub by_session: std::collections::BTreeMap<String, String>,
    /// A folder one desktop App holds → that App.
    pub by_folder: std::collections::BTreeMap<String, String>,
}

/// [`DesktopApps`] as the book stands.
pub fn desktop_apps(state: &Arc<AppState>) -> DesktopApps {
    DesktopApps {
        by_session: desktop_app_by_session(state),
        by_folder: desktop_app_by_folder(state),
    }
}

/// D-62 (AJ-E3204): the App a session is in, as the rail groups it: the
/// desktop App it was last seen open in (`key`: `<tool>/<id>` of one
/// another app runs, `folder` the folder its files are in), else, unless
/// it runs in a terminal (`in_terminal`), the desktop App whose folder it
/// is on (rc22, A3: a Claude app profile's session is that profile's,
/// never a second App's), else its folder's CLI App. One rule for every
/// list and count (AJ-E3102).
#[must_use]
pub fn app_of(
    desktop: &DesktopApps,
    folder: Option<&str>,
    key: Option<&str>,
    in_terminal: bool,
) -> String {
    key.and_then(|key| {
        folder
            .and_then(|folder| desktop.by_session.get(&in_folder(folder, key)))
            .or_else(|| desktop.by_session.get(key))
            .cloned()
    })
    .or_else(|| {
        folder
            .filter(|_| !in_terminal)
            .and_then(|folder| desktop.by_folder.get(folder).cloned())
    })
    .unwrap_or_else(|| juggler_core::storage::cli_app_id(folder.unwrap_or_default()))
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

/// 0.3.5: the tool folder `place`'s sessions use: its account's folder,
/// started with it so the profile stays on that account.
/// ADR-0028: the instance folder of the app profile `id`, when known.
pub(crate) fn instance_dir_of(state: &Arc<AppState>, id: &str) -> Option<std::path::PathBuf> {
    with_book(state, |book| {
        book.places
            .get(id)
            .and_then(|place| place.instance_dir.clone())
    })
    .ok()
    .flatten()
}

pub(crate) fn tool_folder(state: &Arc<AppState>, place: &KnownPlace) -> Option<std::path::PathBuf> {
    let id = place.account_id.as_deref()?;
    state
        .storage
        .lock()
        .ok()?
        .accounts()
        .get(&juggler_core::model::AccountId::new(id.to_owned()))
        .ok()
        .map(|account| account.state_dir)
}

/// `place` in words, its profile named by its account (as the screens
/// name it).
pub(crate) fn named(state: &Arc<AppState>, place: &KnownPlace) -> String {
    let word = app_word(state, place);
    KnownPlace {
        name: word,
        ..place.clone()
    }
    .named(account_label(state, place).as_deref())
}

/// The app's word as every screen says it (`lib/app-name`'s
/// `desktopAppWord`, the 0.4 review): "Claude app" where its tool is named
/// after it (the Claude app runs Claude Code), else its own name
/// (`ChatGPT`), so its outcomes read "Restarting Claude app · Work Max".
pub(super) fn app_word(state: &Arc<AppState>, place: &KnownPlace) -> String {
    let registry = &state.registry;
    let tool = registry
        .ids()
        .iter()
        .filter_map(|id| registry.get(id))
        .find(|adapter| {
            adapter.host_apps().iter().any(|app| {
                app.id == place.app_id && app.kind == juggler_adapter::live::HostKind::Desktop
            })
        });
    match tool {
        Some(adapter) if adapter.spec().display_name.starts_with(&place.name) => {
            format!("{} app", place.name)
        }
        _ => place.name.clone(),
    }
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
        app.emit_event(AppEvent::PlacesChanged, ());
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
            .filter(|(id, _)| !held_for_move(state, id))
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
        let home = juggler_platform::user::home().unwrap_or_default();
        running_apps(&procs::table(), &hosts, &home)
    } else {
        Vec::new()
    };
    let mut places = Vec::new();
    let mut lost: HashSet<String> = HashSet::new();
    for (id, place) in &kept {
        let running = match watch.pid_of(id) {
            Some((pid, start)) => {
                let same = alive
                    .get(&pid)
                    .is_some_and(|process| juggler_adapter::live::same_start(start, process.start));
                let running = watch.seen(id, same);
                // 0.3.8: running on with no window counts as down, unless
                // a session in it is still working (a restart would end
                // it); then it waits for the next tick.
                if same && starter.detached(pid, start) && working_in(state, pid) == 0 {
                    lost.insert(id.clone());
                    false
                } else {
                    running
                }
            }
            None => {
                match found
                    .iter()
                    .find(|app| book::place_id(&app.app_id, app.instance.as_deref()) == *id)
                {
                    Some(app) => {
                        watch.found(id, app.pid, app.start);
                        watch.note_exe(id, &app.exe);
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
    act(state, acts, &kept, &lost, starter, now);
    changed(state);
}

/// Does what keep alive's step said: starts a place that's down (a lost
/// one is restarted: quit, then started), or pauses one, and says so.
fn act(
    state: &Arc<AppState>,
    acts: Vec<Act>,
    kept: &[(String, KnownPlace)],
    lost: &HashSet<String>,
    starter: &dyn start::Starter,
    now: i64,
) {
    let hosts = HostRegistry::of(&state.registry);
    for act in acts {
        match act {
            Act::Start(id, attempt) => {
                let Some((_, place)) = kept.iter().find(|(known, _)| *known == id) else {
                    continue;
                };
                let was_lost = lost.contains(&id);
                let up = if was_lost {
                    restart(
                        state,
                        &id,
                        starter,
                        true,
                        RestartOf::LostWindow,
                        QuitWait::default(),
                        now,
                    )
                    .map(|_| ())
                } else {
                    start_place(state, place, &hosts, starter, true)
                };
                match up {
                    Ok(()) => crate::features::notifications::sources::app_started_again(
                        state,
                        &id,
                        &named(state, place),
                        attempt,
                        was_lost,
                    ),
                    Err(why) => pause(state, &id, &why),
                }
            }
            Act::Pause(id, why) => pause(state, &id, &why),
        }
    }
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
        let home = juggler_platform::user::home().unwrap_or_default();
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
            .filter(|place| place.users)
            .ok_or_else(|| AppError::Refused(NOT_ADDED.to_owned()))?;
        if on {
            start::plan(place, &hosts, None, true).map_err(AppError::Refused)?;
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

/// D-84: what Open, Restart and Keep alive say about an app that isn't
/// one of the user's (an offer, or nothing known).
pub const NOT_ADDED: &str = "That app isn't in AI Juggler: add it first (Settings › Desktop apps)";

/// D-84: what AI Juggler found running and the user hasn't added or
/// declined: each offer's id and name. Offers whose folder is gone are
/// forgotten here.
///
/// # Errors
///
/// A poisoned lock; storage errors.
pub fn offers(state: &Arc<AppState>) -> AppResult<Vec<(String, String)>> {
    let gone: Vec<String> = with_book(state, |book| {
        book.places
            .iter()
            .filter(|(_, place)| {
                !place.users && place.instance_dir.as_ref().is_some_and(|dir| !dir.exists())
            })
            .map(|(id, _)| id.clone())
            .collect()
    })?;
    for id in &gone {
        forget(state, id)?;
    }
    with_book(state, |book| {
        book.places
            .iter()
            .filter(|(_, place)| !place.users && !place.declined)
            .map(|(id, place)| (id.clone(), place.label()))
            .collect()
    })
}

/// D-84: the user adds offer `id` (or answers "Not now": `add` false).
///
/// # Errors
///
/// [`AppError::Refused`] for an id that isn't an offer; storage errors.
pub fn answer_offer(state: &Arc<AppState>, id: &str, add: bool) -> AppResult<()> {
    {
        let storage = crate::commands::lock_storage_arc(state)?;
        if add {
            if !storage.apps().add(id)? {
                return Err(AppError::Refused(
                    "AI Juggler didn't find that app".to_owned(),
                ));
            }
        } else {
            storage.apps().decline(id)?;
        }
    }
    if add {
        link_its_folder(state, id, true);
    }
    // The book reads its rows again: the place is the user's now (or
    // declined).
    if let Ok(mut slot) = state.places.book.lock() {
        *slot = None;
    }
    if add {
        read_records(state);
    }
    changed(state);
    Ok(())
}

/// The folder discovery found under a profile's own `name` (the profile
/// launcher's convention).
fn folder_for_profile<'a>(
    name: &str,
    found: &'a [juggler_adapter::environment::DiscoveredInstall],
) -> Option<&'a juggler_adapter::environment::DiscoveredInstall> {
    found
        .iter()
        .find(|one| one.path.file_name().and_then(|n| n.to_str()) == Some(name))
}

/// The go sitting (2026-11-07, finding 1): a profile added after first
/// run listed none of its sessions, because the folder its tool writes
/// them to (the profile launcher's `ClaudeCodeProfiles/<name>` beside the
/// app's `ClaudeProfiles/<name>`) was never brought in. With the same
/// yes, the folder discovery finds under the profile's own name is
/// brought in where it is, as first run does (in place, never copied),
/// unless an Account holds it already; either way it's the profile's
/// folder from then on. That link is what tells the folder's tool sign-in
/// apart (the profile signs in inside its app, never in a terminal: the
/// rerun's "needs its sign-in" came from a folder brought in unlinked) and
/// gives the App page its Account. Only one an Account holds already
/// unless `may_bring_in` (Add's yes). Nothing when none matches; a
/// failure never fails the add. Returns whether the link is new.
fn link_its_folder(state: &Arc<AppState>, id: &str, may_bring_in: bool) -> bool {
    let Some((app_id, name)) = with_book(state, |book| {
        book.places.get(id).and_then(|place| {
            let name = place
                .instance_dir
                .as_deref()?
                .file_name()?
                .to_str()?
                .to_owned();
            Some((place.app_id.clone(), name))
        })
    })
    .ok()
    .flatten() else {
        return false;
    };
    let canonical =
        |path: &std::path::Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let accounts = crate::commands::lock_storage_arc(state)
        .ok()
        .and_then(|storage| storage.accounts().list().ok())
        .unwrap_or_default();
    for harness in state.registry.ids() {
        let Some(adapter) = state.registry.get(&harness) else {
            continue;
        };
        if !adapter.host_apps().iter().any(|host| host.id == app_id) {
            continue;
        }
        let installs = adapter.scan_environment();
        let Some(found) = folder_for_profile(&name, &installs) else {
            continue;
        };
        let held = accounts
            .iter()
            .find(|account| canonical(&account.state_dir) == canonical(&found.path))
            .map(|account| account.id.as_str().to_owned());
        let account = held.or_else(|| {
            if !may_bring_in {
                return None;
            }
            crate::features::account::import::account_import_mode_impl(
                state,
                harness.as_str(),
                &found.path.to_string_lossy(),
                &name,
                crate::features::account::import::ImportRequest {
                    origin: Some(found.origin.as_str().to_owned()),
                    in_place: true,
                    ..crate::features::account::import::ImportRequest::default()
                },
            )
            .map_err(|why| {
                tracing::warn!(
                    place = id,
                    "the added profile's folder wasn't brought in: {why}"
                );
            })
            .ok()
            .map(|done| done.account.id.as_str().to_owned())
        });
        return account.is_some_and(|account| {
            with_book(state, |book| {
                book.places.get_mut(id).is_some_and(|place| {
                    let new = place.account_id.as_deref() != Some(account.as_str());
                    place.account_id = Some(account);
                    new
                })
            })
            .unwrap_or(false)
        });
    }
    false
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
fn host_of(
    place: &KnownPlace,
    shown: String,
    profile: Option<String>,
    pid: u32,
    start: Option<i64>,
) -> Host {
    Host {
        app_id: Some(place.app_id.clone()),
        name: place.name.clone(),
        shown,
        kind: Some(juggler_adapter::live::HostKind::Desktop),
        pid,
        start,
        instance_dir: place.instance_dir.clone(),
        instance: profile,
    }
}

/// Whether `process` is still the app a place last ran as: the same
/// start time (to the second) and the same program. A pid alone isn't
/// enough: the system reuses them, and a closed profile read as running
/// on 0.4.0 (2026-11-06).
#[must_use]
pub fn is_its_process(
    exe: &std::path::Path,
    recorded_start: Option<i64>,
    process: &procs::Proc,
) -> bool {
    let started =
        matches!((recorded_start, process.start), (Some(was), Some(is)) if (was - is).abs() <= 1);
    let program = exe.as_os_str().is_empty() || process.exe.as_deref() == Some(exe);
    started && program
}

/// The process `id` runs as now: the watch's, when it's still that one,
/// else looked for in the whole list.
pub(crate) fn running_now(state: &Arc<AppState>, id: &str) -> Option<(u32, Option<i64>)> {
    let (known, exe) = state
        .places
        .watch
        .lock()
        .ok()
        .map(|watch| {
            (
                watch.pid_of(id),
                watch.exe_of(id).map(std::path::Path::to_path_buf),
            )
        })
        .unwrap_or_default();
    let exe = exe.unwrap_or_default();
    if let Some((pid, start)) = known
        && procs::some(&[pid])
            .get(&pid)
            .is_some_and(|process| is_its_process(&exe, start, process))
    {
        return Some((pid, start));
    }
    let hosts = HostRegistry::of(&state.registry);
    let home = juggler_platform::user::home().unwrap_or_default();
    let app = running_apps(&procs::table(), &hosts, &home)
        .into_iter()
        .find(|app| book::place_id(&app.app_id, app.instance.as_deref()) == id)?;
    if let Ok(mut watch) = state.places.watch.lock() {
        watch.found(id, app.pid, app.start);
        watch.note_exe(id, &app.exe);
    }
    Some((app.pid, app.start))
}

/// How long a lost app is given to quit, and how often it's looked at.
#[derive(Clone, Copy, Debug)]
pub struct QuitWait {
    /// The most it's waited for.
    pub limit: Duration,
    /// Between looks.
    pub every: Duration,
}

impl Default for QuitWait {
    fn default() -> Self {
        Self {
            limit: Duration::from_secs(10),
            every: Duration::from_millis(250),
        }
    }
}

/// How many sessions are working (a turn running) in the app whose main
/// process is `pid`, as the live view saw them last (read only for an
/// app that lost its window).
pub(crate) fn working_in(state: &Arc<AppState>, pid: u32) -> usize {
    use crate::features::conversations::scan::State;
    state
        .conversations
        .live_model()
        .entries
        .iter()
        .filter(|entry| entry.state == Some(State::Working))
        .filter(|entry| {
            entry
                .places
                .iter()
                .any(|place| place.host.as_ref().is_some_and(|host| host.pid == pid))
        })
        .count()
}

/// Which restart: of an app that lost its window (keep alive, the lost
/// window's offer), or of a running one the user confirmed (D-71).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartOf {
    /// Only one that runs on with no window; one with its window is left.
    LostWindow,
    /// Any running one: the user confirmed it, knowing what it stops.
    Confirmed,
}

/// 0.3.8: restarts `id`, an app or profile that runs on but lost its
/// window when the system restarted the display (the watchdog's restart of
/// 2026-11-04 left five Claude profiles running with no window): asks
/// its process to quit (SIGTERM), waits for it to end, then starts it
/// again exactly as it ran ([`start::start`], on its tool folder, behind
/// the app in front when `background`). It never quits an app that has
/// its window (Open brings that forward), never one it couldn't start
/// again, and never starts a second process beside one that won't quit.
/// A restart by hand ends a pause of keep alive, as Open does.
///
/// D-71 (2026-11-04): `RestartOf::Confirmed` restarts one that has its
/// window too, once the user has confirmed it (the restart stops what's
/// working in it); keep alive and Open only ever restart a lost one.
///
/// Returns what it did, in words.
///
/// # Errors
///
/// In words: it isn't remembered or can't be started, it has its window
/// (unless confirmed), it's already being restarted, it didn't quit, or it
/// didn't start.
pub fn restart(
    state: &Arc<AppState>,
    id: &str,
    starter: &dyn start::Starter,
    background: bool,
    of: RestartOf,
    wait: QuitWait,
    now: i64,
) -> Result<String, String> {
    let place = with_book(state, |book| {
        book.places
            .get_mut(id)
            .filter(|place| place.users)
            .map(|place| {
                if !background {
                    place.paused = None;
                }
                place.clone()
            })
    })
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "AI Juggler doesn't remember that app any more".to_owned())?;
    if held_for_move(state, id) {
        return Err(HELD.to_owned());
    }
    let label = named(state, &place);
    let hosts = HostRegistry::of(&state.registry);
    // Checked before anything is quit: it starts again as it ran.
    start::plan(&place, &hosts, None, background)?;
    let Some(_guard) = Restarting::begin(state, id) else {
        return Err(format!("{label} is restarting already"));
    };
    let coming_up = state
        .places
        .watch
        .lock()
        .is_ok_and(|watch| watch.coming_up(id, now));
    match running_now(state, id) {
        Some((pid, start)) => {
            if of == RestartOf::LostWindow && !starter.detached(pid, start) {
                return Err(format!(
                    "{label} has its window, so it wasn't restarted. Open brings it forward"
                ));
            }
            mark_restarting(state, id, now);
            starter
                .quit(pid)
                .map_err(|why| format!("Couldn't quit {label}: {why}"))?;
            if !wait_until(wait, || !starter.alive(pid, start)) {
                return Err(format!(
                    "{label} didn't quit within {} seconds, so it wasn't started again. Quit it in Activity Monitor, then start it here",
                    wait.limit.as_secs()
                ));
            }
        }
        // Already started (by keep alive, or a restart a moment ago): never
        // a second one.
        None if coming_up => return Ok(format!("{label} is starting")),
        None => {}
    }
    mark_restarting(state, id, now);
    // ADR-0028: the app is down: a switched family waiting for it nests
    // now, before it reads its records again.
    let nested = place
        .instance_dir
        .as_deref()
        .and_then(|dir| crate::features::handover::automation::apply_for_place(state, id, dir));
    let launched = start_place(state, &place, &hosts, starter, background);
    changed(state);
    launched.map(|()| {
        nested.map_or_else(
            || format!("Restarting {label}"),
            |said| format!("Restarting {label}: {said}"),
        )
    })
}

/// One restart of a place at a time: held while it runs.
struct Restarting<'a> {
    state: &'a Arc<AppState>,
    id: String,
}

impl<'a> Restarting<'a> {
    fn begin(state: &'a Arc<AppState>, id: &str) -> Option<Self> {
        let mut now = state.places.restarting.lock().ok()?;
        now.insert(id.to_owned()).then(|| Self {
            state,
            id: id.to_owned(),
        })
    }
}

impl Drop for Restarting<'_> {
    fn drop(&mut self) {
        if let Ok(mut now) = self.state.places.restarting.lock() {
            now.remove(&self.id);
        }
    }
}

/// Keep alive starts nothing of its own for `id` while it restarts.
fn mark_restarting(state: &Arc<AppState>, id: &str, now: i64) {
    if let Ok(mut watch) = state.places.watch.lock() {
        watch.restarting(id, now);
    }
}

fn wait_until(wait: QuitWait, mut done: impl FnMut() -> bool) -> bool {
    let until = Instant::now() + wait.limit;
    loop {
        if done() {
            return true;
        }
        if Instant::now() >= until {
            return false;
        }
        std::thread::sleep(wait.every);
    }
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
        book.places
            .get_mut(id)
            .filter(|place| place.users)
            .map(|place| {
                place.paused = None;
                place.clone()
            })
    })?
    .ok_or_else(|| AppError::Refused(NOT_ADDED.to_owned()))?;
    if held_for_move(state, id) {
        return Err(AppError::Refused(HELD.to_owned()));
    }
    if let Ok(mut watch) = state.places.watch.lock() {
        watch.drop_place(id);
    }
    changed(state);
    let hosts = HostRegistry::of(&state.registry);
    let account = account_label(state, &place);
    let label = named(state, &place);
    let profile = place.profile(account.as_deref());
    let link = conversation.and_then(|conversation| {
        hosts
            .by_id(&place.app_id)
            .and_then(|host| host.thread_link_for(conversation))
    });
    // 0.4.1: an app that opens its own sessions by its own link (the
    // Claude app): the session itself, in this profile, started first
    // when it isn't running.
    let own = conversation.and_then(|conversation| {
        let host = hosts.by_id(&place.app_id)?;
        let own = open_session::app_session_in(state, &place, conversation)?;
        host.session_link_for(&own)
    });
    if let Some(own) = own {
        let (state, id, still) = (Arc::clone(state), id.to_owned(), label.clone());
        return Ok(goto::within(goto::LIMIT, move || {
            open_session::open_in(
                &label,
                &id,
                &place,
                &own,
                &state,
                &handoff::Real,
                desktop.as_ref(),
                false,
                handoff::Waits::default(),
            )
        })
        .unwrap_or_else(|| {
            outcome(
                format!("Starting {still}… the session opens once it's up"),
                "pending",
            )
        }));
    }
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
            let mut said = goto::bring_forward(
                &host_of(&place, app_word(&state, &place), profile, pid, start),
                desktop.as_ref(),
            );
            if said.reached == "app" {
                said.outcome.push_str(pick);
            }
            // 0.3.8: lost its window: the toast offers to restart this
            // place, never to start a second one beside it.
            if said.reached == "lost" {
                said.place_id = Some(id.clone());
            }
            return said;
        }
        // ADR-0028: a switched family waiting for this profile's start
        // nests now, before the app reads its records.
        if let Some(dir) = place.instance_dir.as_deref() {
            crate::features::handover::automation::apply_for_place(&state, &id, dir);
        }
        match start_place(&state, &place, &hosts, starter.as_ref(), false) {
            // The go sitting (finding 3): the window follows a start to its
            // end by this id ("… is running", or why not).
            Ok(()) => GoToOutcomeInfo {
                place_id: Some(id.clone()),
                ..outcome(format!("Starting {label}{pick}"), "started")
            },
            Err(why) => outcome(why, "none"),
        }
    })
    .unwrap_or_else(|| outcome(format!("Still opening {still}…"), "pending")))
}

/// Starts `place` (its tool's folder found here), and on success has
/// the live view look for it every tick for a while: every door's Start,
/// Restart and Keep alive says it runs within seconds.
pub(crate) fn start_place(
    state: &Arc<AppState>,
    place: &KnownPlace,
    hosts: &HostRegistry,
    starter: &dyn start::Starter,
    background: bool,
) -> Result<(), String> {
    start::start(
        place,
        hosts,
        tool_folder(state, place).as_deref(),
        starter,
        background,
    )
    .inspect(|()| crate::features::conversations::look_for_started(state))
}

fn outcome(text: String, reached: &str) -> GoToOutcomeInfo {
    GoToOutcomeInfo {
        outcome: text,
        reached: reached.to_owned(),
        place_id: None,
    }
}

/// The remembered places whose sessions run on `account` and that `keep`
/// says yes to (with their app as the host registry knows it), by id.
/// `None` when the book can't be read.
pub(crate) fn places_on(
    state: &Arc<AppState>,
    account: &str,
    keep: impl Fn(&KnownPlace, &juggler_adapter::live::HostApp) -> bool,
) -> Option<Vec<(String, KnownPlace)>> {
    let hosts = HostRegistry::of(&state.registry);
    with_book(state, |book| {
        book.places
            .iter()
            .filter(|(_, place)| place.account_id.as_deref() == Some(account))
            .filter(|(_, place)| {
                hosts
                    .by_id(&place.app_id)
                    .is_some_and(|host| keep(place, host))
            })
            .map(|(id, place)| (id.clone(), place.clone()))
            .collect()
    })
    .ok()
}

/// For tests: a Claude app profile `name` under `root` (its bundle and
/// folder made), remembered on `account`. Returns its id, the place and
/// its folder.
#[cfg(test)]
pub(crate) fn remember_test_profile(
    state: &Arc<AppState>,
    root: &std::path::Path,
    name: &str,
    account: Option<&str>,
) -> (String, KnownPlace, std::path::PathBuf) {
    let bundle = root.join("Claude.app");
    std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
    let profile = root.join("Profiles").join(name);
    std::fs::create_dir_all(&profile).unwrap();
    let id = book::place_id("claude-app", Some(&profile));
    let place = KnownPlace {
        app_id: "claude-app".into(),
        name: "Claude".into(),
        instance: Some(name.into()),
        bundle: bundle.clone(),
        exe: bundle.join("Contents/MacOS/Claude"),
        given: Some(profile.clone()),
        instance_dir: Some(profile.clone()),
        account_id: account.map(str::to_owned),
        conversations: Vec::new(),
        seen_ms: 1,
        keep_alive: false,
        paused: None,
        found: false,
        users: true,
        declined: false,
    };
    with_book(state, |book| {
        book.places.insert(id.clone(), place.clone());
    })
    .unwrap();
    (id, place, profile)
}

#[cfg(test)]
mod d84_tests;
#[cfg(test)]
pub(crate) mod fakes;
#[cfg(test)]
mod tests;
