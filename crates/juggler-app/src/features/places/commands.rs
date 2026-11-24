//! The places' IPC (0.3.3): the apps and profiles remembered, Open, Keep
//! alive, Forget, and the place an ended session was last open in.

// Tauri command signatures take State by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::collections::BTreeMap;
use std::sync::Arc;

use super::book::KnownPlace;
use crate::error::AppResult;
use crate::features::conversations::dto::GoToOutcomeInfo;
use crate::features::conversations::places::HostRegistry;
use crate::features::presence::AppIdentity;
use crate::paging::{Page, PageRequest, SortKey};
use crate::state::AppState;

/// One app or profile AI Juggler remembers.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct KnownPlaceInfo {
    /// Its id (what the other commands take).
    pub id: String,
    /// The host registry's id for the app (its icon).
    pub app_id: String,
    /// The app's name (`Claude`).
    pub app: String,
    /// The instance's name when it isn't the app's default one.
    pub instance: Option<String>,
    /// The profile in words, as the rail and Home name it: its account's
    /// name, else the instance's.
    pub profile: Option<String>,
    /// `Claude · Work Max`: the app, then the profile.
    pub label: String,
    /// Its main process, while it runs.
    pub app_pid: Option<u32>,
    /// 0.3.8: it runs on but lost its window when macOS restarted its
    /// display (Open asks to restart it).
    pub lost: bool,
    /// Keep alive is on.
    pub keep_alive: bool,
    /// Why keep alive is paused; `null` while it watches (or is off).
    pub paused: Option<String>,
    /// The account its sessions ran on, when one did.
    pub account_id: Option<String>,
    /// When it was last seen running, epoch ms (to the hour).
    #[ts(type = "number")]
    pub seen_ms: i64,
    /// Why it can't be started from here, when it can't.
    pub cannot_start: Option<String>,
    /// 0.3.4 (D-64): the sessions seen open in it (`<tool>/<id>`, the
    /// book's newest 100), the most recently first seen first: the rail
    /// puts an app's sessions that aren't open under it.
    pub recent: Vec<String>,
    /// AJ-E3203: found by its folder's name beside a profile that ran,
    /// and not seen running since.
    pub found: bool,
    /// AJ-E3206: its colour as an App, given once and stored (an index
    /// into the UI's `MARK_COLOURS`).
    #[ts(optional = nullable)]
    pub colour_slot: Option<u8>,
    /// AJ-E1610: what its app allows from AI Juggler (its own links, how
    /// sure each is, its track-only parts and why); `null` for an app that
    /// declares nothing.
    #[ts(optional = nullable)]
    pub reach: Option<super::reach::AppReachInfo>,
    /// AJ-E2327: the tool folder its sessions are read from, what told,
    /// or why none is linked; `null` before it was looked for (or for an
    /// app with no tool folder).
    #[ts(optional = nullable)]
    pub folder: Option<PlaceFolderInfo>,
}

/// AJ-E2327: an App's tool folder, as its row and Add's confirmation say
/// it.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PlaceFolderInfo {
    /// The tool whose folder it is (`Claude Code`), for the words.
    pub tool: Option<String>,
    /// The folder's own name (`work`), when one is linked.
    pub name: Option<String>,
    /// Where it is, `~` for the home folder, when one is linked.
    pub path: Option<String>,
    /// What told which folder it is: `running` (its open sessions write
    /// there), `records` (its app's own list of its sessions is there),
    /// `recorded` (the folder AI Juggler starts it with, or the one you
    /// chose), `declared` (its app's one folder), or `named` (a folder
    /// named like the profile: a guess, checked when it runs).
    pub by: Option<String>,
    /// Why no folder is linked, in plain words.
    pub why: Option<String>,
}

impl PlaceFolderInfo {
    /// What [`super::Linked`] says of App `id`'s folder, for the window.
    pub(crate) fn of(state: &Arc<AppState>, id: &str, linked: &super::Linked) -> Self {
        let home = juggler_platform::user::home();
        Self {
            tool: tool_word(state, id),
            name: linked
                .folder
                .as_deref()
                .and_then(std::path::Path::file_name)
                .map(|name| name.to_string_lossy().into_owned()),
            path: linked.folder.as_deref().map(|folder| {
                home.as_deref()
                    .and_then(|home| folder.strip_prefix(home).ok())
                    .map_or_else(
                        || folder.display().to_string(),
                        |rest| format!("~/{}", rest.display()),
                    )
            }),
            by: linked.by.map(|by| by.word().to_owned()),
            why: linked.why.clone(),
        }
    }

    /// An App's row: its last look's note.
    fn of_note(state: &Arc<AppState>, id: &str, note: super::LinkNote) -> Self {
        Self::of(
            state,
            id,
            &match note {
                super::LinkNote::Linked { folder, by } => super::Linked {
                    folder: Some(folder),
                    by: Some(by),
                    why: None,
                },
                super::LinkNote::Unlinked(why) => super::Linked {
                    why: Some(why),
                    ..super::Linked::default()
                },
            },
        )
    }
}

/// The name of the tool whose folders App `id`'s sessions are in
/// (`Claude Code` for the Claude app).
fn tool_word(state: &Arc<AppState>, id: &str) -> Option<String> {
    let app = super::with_book(state, |book| {
        book.places.get(id).map(|place| place.app_id.clone())
    })
    .ok()
    .flatten()?;
    state
        .registry
        .ids()
        .into_iter()
        .filter_map(|harness| state.registry.get(&harness))
        .find(|adapter| adapter.host_apps().iter().any(|host| host.id == app))
        .map(|adapter| adapter.spec().display_name.clone())
}

/// AJ-E2327: what Add did with the App's tool folder, for its
/// confirmation.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PlaceAddedInfo {
    /// The App's id.
    pub id: String,
    /// The folder it uses (nothing when Not now was the answer).
    pub folder: PlaceFolderInfo,
}

/// AJ-E2327: a desktop profile found running or beside one, paired with
/// the tool folder evidence says it uses (first run's "Your apps": one
/// row each). Nothing is brought in or linked by asking.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PlacePairingInfo {
    /// The place's id (an App's, or an offer's: what Add takes).
    pub id: String,
    /// `Claude · team-a`.
    pub label: String,
    /// Whether it's one of the user's Apps already.
    pub added: bool,
    /// The folder paired with it, what told, or why none.
    pub folder: PlaceFolderInfo,
    /// Whether the pairing is proof (its running sessions, its records or
    /// its app's declared folder) rather than a recalled or guessed one.
    pub certain: bool,
    /// 0.5.1 F35: the id of the tool its app runs (`claude-code`), so
    /// "Your apps" puts its row in that tool's group, paired or not.
    pub tool: Option<String>,
    /// 0.5.1 F35: the paired folder's full path, as the scan lists it
    /// (`folder.path` is for the words, `~` for the home folder), so its
    /// row is that folder's row exactly, never matched by name.
    pub path: Option<String>,
    /// It runs now (the process table says).
    pub running: bool,
}

/// AJ-E3203: finding a desktop app's profiles that aren't running.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FindProfilesInfo {
    /// `ask` until the user answers, then `on` or `off`.
    pub answer: String,
    /// The folders whose names it reads (or would): one per folder of
    /// profiles beside a profile that ran.
    pub folders: Vec<ProfilesFolderInfo>,
}

/// One folder of profiles.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProfilesFolderInfo {
    /// The host registry's id for the app whose profiles it holds.
    pub app_id: String,
    /// The folder, `~` for the home folder.
    pub folder: String,
}

/// A place's sessions, the most recently first seen first.
fn recent(conversations: &[String]) -> Vec<String> {
    conversations.iter().rev().cloned().collect()
}

fn info(
    id: &str,
    place: &KnownPlace,
    // Its one name ([`identities`]).
    named: Option<&AppIdentity>,
    // Its process, when it runs, and whether it lost its window (0.3.8).
    (pid, lost): (Option<u32>, bool),
    hosts: &HostRegistry,
    slots: &std::collections::BTreeMap<String, u8>,
) -> KnownPlaceInfo {
    KnownPlaceInfo {
        id: id.to_owned(),
        app_id: place.app_id.clone(),
        app: place.name.clone(),
        instance: place.instance.clone(),
        label: named.map_or_else(|| place.name.clone(), |n| n.label.clone()),
        profile: named.and_then(|n| n.profile.clone()),
        app_pid: pid,
        lost,
        keep_alive: place.keep_alive,
        paused: place.paused.clone(),
        account_id: place.account_id.clone(),
        seen_ms: place.seen_ms,
        cannot_start: super::start::plan(place, hosts, None, false).err(),
        recent: recent(&place.conversations),
        found: place.found,
        colour_slot: slots.get(id).copied(),
        reach: hosts.by_id(&place.app_id).and_then(super::reach::info),
        folder: None,
    }
}

/// ADR-0032: each of the user's desktop Apps' one identity
/// (`presence::identity`), by App id, as the rail names it: the Floor's
/// signs read the same, without a look at the process table.
pub(crate) fn identities(state: &Arc<AppState>) -> BTreeMap<String, AppIdentity> {
    let places: BTreeMap<String, KnownPlace> = super::with_book(state, |book| book.places.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, place)| place.users)
        .collect();
    identities_of(state, &places)
}

fn identities_of(
    state: &Arc<AppState>,
    places: &BTreeMap<String, KnownPlace>,
) -> BTreeMap<String, AppIdentity> {
    let accounts: std::collections::HashMap<String, String> = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.accounts().list().ok())
        .unwrap_or_default()
        .into_iter()
        .map(|account| (account.id.to_string(), account.label))
        .collect();
    // A folder two desktop Apps share (a test profile linked to the
    // default one's `~/.claude`) names neither: each goes by its own
    // instance, so two never read as one ("Claude app · Default" twice).
    let mut sharing: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for place in places.values() {
        if let Some(folder) = place.account_id.as_deref() {
            *sharing.entry(folder).or_default() += 1;
        }
    }
    places
        .iter()
        .map(|(id, place)| {
            let shared = place
                .account_id
                .as_deref()
                .is_some_and(|f| sharing.get(f).is_some_and(|n| *n >= 2));
            let account = place
                .account_id
                .as_ref()
                .filter(|_| !shared)
                .and_then(|f| accounts.get(f))
                .map(String::as_str);
            let word = super::app_word(state, place);
            let named = crate::features::presence::identity(crate::features::presence::Naming {
                id,
                desktop: true,
                app: &word,
                instance_dir: place.instance_dir.as_deref(),
                default_instance: place.instance.is_none(),
                folder_label: account,
                folder_shared: place.account_id.is_some() && account.is_none(),
                named: None,
            });
            (id.clone(), named)
        })
        .collect()
}

/// ADR-0032: the desktop Apps running now, from a fresh look at the
/// process table.
fn what_runs(
    state: &AppState,
    hosts: &HostRegistry,
) -> BTreeMap<String, crate::features::conversations::places::RunningApp> {
    crate::features::presence::running(&state.places.table(), hosts, &state.places.home())
}

/// The places remembered with their processes, each checked alive now.
pub(crate) fn known(state: &Arc<AppState>) -> AppResult<Vec<KnownPlaceInfo>> {
    // D-84: the user's Apps only; an offer is listed by `places_offers`.
    let places: std::collections::BTreeMap<String, super::book::KnownPlace> =
        super::with_book(state, |book| book.places.clone())?
            .into_iter()
            .filter(|(_, place)| place.users)
            .collect();
    let hosts = HostRegistry::of(&state.registry);
    let running = what_runs(state, &hosts);
    // Keep alive's watch follows what runs (its own misses and restarts
    // stay its own).
    if let Ok(mut watch) = state.places.watch.lock() {
        for (id, app) in &running {
            if places.contains_key(id) {
                watch.found(id, app.pid, app.start);
                watch.note_exe(id, &app.exe);
            }
        }
    }
    let slots = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.apps().colour_slots().ok())
        .unwrap_or_default();
    let named = identities_of(state, &places);
    Ok(places
        .iter()
        .map(|(id, place)| {
            // ADR-0032: running is what the process table says now.
            let running = running.get(id).map(|app| (app.pid, app.start));
            let lost = running.is_some_and(|(pid, start)| {
                super::start::Starter::detached(&super::start::System, pid, start)
            });
            info(
                id,
                place,
                named.get(id),
                (running.map(|(pid, _)| pid), lost),
                &hosts,
                &slots,
            )
        })
        .map(|mut row| {
            row.folder = super::link_note(state, &row.id)
                .map(|note| PlaceFolderInfo::of_note(state, &row.id, note));
            row
        })
        .collect())
}

/// Where a remembered app sits in Desktop apps: running first, then the
/// closed ones by when they were last seen, then the ones only found by
/// their folder (design review #62: kept-alive first, then by name, buried
/// the one you used last).
fn place_order(row: &KnownPlaceInfo) -> SortKey {
    let group = if row.app_pid.is_some() {
        0
    } else if row.found {
        2
    } else {
        1
    };
    SortKey::new(
        vec![group, -row.seen_ms],
        format!("{}\u{0}{}", row.label, row.id),
    )
}

/// The places remembered, in `place_order` (at most `book::MAX`, so one
/// page).
///
/// # Errors
///
/// A poisoned lock; storage errors; a cursor this app didn't make.
#[tauri::command]
pub async fn places_list(
    state: tauri::State<'_, Arc<AppState>>,
    page: Option<PageRequest>,
) -> AppResult<Page<KnownPlaceInfo>> {
    crate::ipc::blocking(state.inner(), "the apps remembered", move |state| {
        let rows = known(state)?;
        crate::paging::page(rows, place_order, &page.unwrap_or_default())
    })
    .await
}

/// The place an ended session of `tool` was last open in, when one is
/// remembered.
///
/// # Errors
///
/// A poisoned lock; storage errors.
#[tauri::command]
pub async fn places_find(
    state: tauri::State<'_, Arc<AppState>>,
    tool: String,
    conversation_id: String,
    account: Option<String>,
    app: Option<String>,
) -> AppResult<Option<KnownPlaceInfo>> {
    crate::ipc::blocking(state.inner(), "the session's app", move |state| {
        let id = super::with_book(state, |book| {
            book.place_of(&tool, &conversation_id)
                .filter(|(_, place)| place.users)
                .map(|(id, _)| id.clone())
                // 0.4.1: a session started in a desktop app (`app`, from
                // its tool's files) that was never seen open is that app's
                // profile on its folder (`account`) when the user has
                // exactly one (a closed Claude app profile's Code sessions).
                .or_else(|| {
                    book.users_place_on(app.as_deref()?, account.as_deref()?)
                        .cloned()
                })
        })?;
        Ok(id.and_then(|id| known(state).ok()?.into_iter().find(|row| row.id == id)))
    })
    .await
}

/// One app or profile AI Juggler found running that the user hasn't
/// added (D-84).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PlaceOfferInfo {
    /// Its App id, if added.
    pub id: String,
    /// `Claude app · aij-test`.
    pub label: String,
    /// ADR-0032 (D-100): it runs now (the process table says): the rail's
    /// "Running, not added", with Add.
    pub running: bool,
}

/// D-84: what's offered ("Found on this computer … Add / Not now").
///
/// # Errors
///
/// A poisoned lock; storage errors.
#[tauri::command]
pub async fn places_offers(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<PlaceOfferInfo>> {
    crate::ipc::blocking(state.inner(), "the apps found", move |state| {
        let running = what_runs(state, &HostRegistry::of(&state.registry));
        Ok(super::offers(state)?
            .into_iter()
            .map(|(id, label)| PlaceOfferInfo {
                running: running.contains_key(&id),
                id,
                label,
            })
            .collect())
    })
    .await
}

/// D-84: Add (`add`) or Not now to offer `id`.
///
/// # Errors
///
/// As [`super::answer_offer`].
#[tauri::command]
pub async fn places_offer_answer(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    add: bool,
) -> AppResult<PlaceAddedInfo> {
    crate::ipc::blocking(state.inner(), "add the app", move |state| {
        let linked = super::answer_offer(state, &id, add)?;
        let folder = PlaceFolderInfo::of(state, &id, &linked);
        Ok(PlaceAddedInfo { id, folder })
    })
    .await
}

/// AJ-E2327, "Choose its folder…": App `id` uses `folder`, a folder the
/// user picked in the system's picker (vouched for: a path the page made
/// up is refused).
///
/// # Errors
///
/// A folder that wasn't picked; why it can't be the App's folder.
#[tauri::command]
pub async fn places_choose_folder(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    folder: String,
) -> AppResult<PlaceAddedInfo> {
    // Picked or dropped by the user, or a folder AI Juggler's own scan
    // listed: never a path the page made up.
    let folder = crate::picked::vouch(&folder, crate::picked::WORK_FOLDER, |_| false)?;
    crate::ipc::blocking(state.inner(), "choose the app's folder", move |state| {
        let linked = super::link::choose_folder(state, &id, &folder)
            .map_err(crate::error::AppError::Refused)?;
        let folder = PlaceFolderInfo::of(state, &id, &linked);
        Ok(PlaceAddedInfo { id, folder })
    })
    .await
}

/// AJ-E2327: every desktop profile AI Juggler knows (the user's Apps and
/// the ones offered), each paired with the tool folder evidence says it
/// uses, for first run's "Your apps". Reads only: nothing is added,
/// brought in or linked.
///
/// # Errors
///
/// A poisoned lock; storage errors.
#[tauri::command]
pub async fn places_pairings(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<PlacePairingInfo>> {
    crate::ipc::blocking(state.inner(), "pair the apps with their folders", pairings).await
}

/// [`places_pairings`]' body.
///
/// # Errors
///
/// A poisoned lock; storage errors.
pub fn pairings(state: &Arc<AppState>) -> AppResult<Vec<PlacePairingInfo>> {
    let places: Vec<(String, KnownPlace)> = super::with_book(state, |book| {
        book.places
            .iter()
            // An offer whose profile folder is gone is no row (as
            // `offers` forgets it).
            .filter(|(_, place)| {
                !place.declined
                    && (place.users || place.instance_dir.as_ref().is_none_or(|dir| dir.exists()))
            })
            .map(|(id, place)| (id.clone(), place.clone()))
            .collect()
    })?;
    let running = what_runs(state, &HostRegistry::of(&state.registry));
    Ok(places
        .into_iter()
        .map(|(id, place)| {
            let paired = super::link::pairing(state, &id);
            let tool = state
                .registry
                .ids()
                .into_iter()
                .find(|harness| {
                    state.registry.get(harness).is_some_and(|adapter| {
                        adapter.host_apps().iter().any(|host| host.id == place.app_id)
                    })
                })
                .map(|harness| harness.as_str().to_owned());
            let folder = PlaceFolderInfo::of(state, &id, &super::Linked {
                folder: paired.as_ref().map(|(folder, _, _)| folder.clone()),
                by: paired.as_ref().map(|(_, by, _)| *by),
                why: paired.is_none().then(|| {
                    "AI Juggler can't tell which folder it uses yet: none of its sessions is open and its app lists none it can find.".to_owned()
                }),
            });
            PlacePairingInfo {
                label: place.label(),
                added: place.users,
                certain: paired.as_ref().is_some_and(|(_, _, certain)| *certain),
                folder,
                tool,
                path: paired.map(|(folder, _, _)| folder.to_string_lossy().into_owned()),
                running: running.contains_key(&id),
                id,
            }
        })
        .collect())
}

/// Open: brings the place forward when it runs, else starts it; with a
/// session, at that session where its app has a link to it. AJ-E1610,
/// "Open here": with `action` (`new_chat`, `new_in_folder` with `folder`,
/// a folder the user picked or a project's), the app's own documented
/// link for it, sent to the running place's process alone.
///
/// # Errors
///
/// As [`super::open`], or [`super::reach::open_route`] with an action; a
/// folder AI Juggler doesn't know.
#[tauri::command]
pub async fn places_open(
    state: tauri::State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    id: String,
    conversation_id: Option<String>,
    action: Option<String>,
    folder: Option<String>,
) -> AppResult<GoToOutcomeInfo> {
    crate::ipc::blocking(state.inner(), "opening the app", move |state| {
        let Some(action) = action else {
            return super::open(
                state,
                &id,
                conversation_id.as_deref(),
                Arc::new(crate::features::conversations::goto::MacDesktop),
                Arc::new(super::start::System),
                Some(crate::features::conversations::goto::tell_window(app)),
            );
        };
        // AJ-E2003 R6: a folder the user picked, or one AI Juggler already
        // works in (a project's), never one the page made up.
        let folder = folder
            .map(|folder| crate::picked::work_folder(state, &folder))
            .transpose()?;
        super::reach::open_route(
            state,
            &id,
            &action,
            folder.as_deref().and_then(std::path::Path::to_str),
            &crate::features::conversations::goto::MacDesktop,
        )
    })
    .await
}

/// 0.3.8: Restart, the answer to "lost its window … Restart it?": quits
/// the place's process that lost its window and starts it again, in front.
/// D-71: with `confirmed`, a running one with its window too (the user
/// confirmed what it stops).
/// Waits for it to quit (at most 10 s), so it runs off the window's
/// thread.
///
/// # Errors
///
/// None of its own: what went wrong is the outcome's words (`none`).
#[tauri::command]
pub async fn places_restart(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    confirmed: Option<bool>,
) -> AppResult<GoToOutcomeInfo> {
    // D-71: a running one only once the user confirmed it.
    let of = if confirmed == Some(true) {
        super::RestartOf::Confirmed
    } else {
        super::RestartOf::LostWindow
    };
    crate::ipc::blocking(state.inner(), "restarting the app", move |state| {
        let said = super::restart(
            state,
            &id,
            &super::start::System,
            false,
            of,
            super::QuitWait::default(),
            crate::commands::now_millis(),
        );
        Ok(match said {
            // Followed by the window until it runs (the go sitting, finding 3).
            Ok(words) => GoToOutcomeInfo {
                outcome: words,
                reached: "started".to_owned(),
                place_id: Some(id.clone()),
                ticket: None,
            },
            Err(why) => GoToOutcomeInfo {
                outcome: why,
                reached: "none".to_owned(),
                place_id: None,
                ticket: None,
            },
        })
    })
    .await
}

/// 0.5.1 F98: Quit…, after its confirm: quits a running app or profile
/// and leaves it closed (keep alive pauses for it until its next Open),
/// through the one quit Restart and Move use. Waits for it to quit (at
/// most 10 s), so it runs off the window's thread.
///
/// # Errors
///
/// None of its own: what went wrong is the outcome's words (`none`).
#[tauri::command]
pub async fn places_quit(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<GoToOutcomeInfo> {
    crate::ipc::blocking(state.inner(), "quitting the app", move |state| {
        let said = super::quit::quit(
            state,
            &id,
            &super::start::System,
            super::QuitWait::default(),
        );
        Ok(match said {
            Ok(words) => GoToOutcomeInfo {
                outcome: words,
                reached: "quit".to_owned(),
                place_id: Some(id.clone()),
                ticket: None,
            },
            Err(why) => GoToOutcomeInfo {
                outcome: why,
                reached: "none".to_owned(),
                place_id: None,
                ticket: None,
            },
        })
    })
    .await
}

/// Keep alive on or off for a place.
///
/// # Errors
///
/// As [`super::set_keep_alive`].
#[tauri::command]
pub async fn places_keep_alive(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    on: bool,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "keep alive", move |state| {
        super::set_keep_alive(state, &id, on)
    })
    .await
}

/// Where finding the profiles that aren't running stands, and the
/// folders whose names it reads.
fn profiles_info(state: &Arc<AppState>) -> AppResult<FindProfilesInfo> {
    let answer = super::profiles::answer(state)?;
    let hosts = HostRegistry::of(&state.registry);
    let home = juggler_platform::user::home().unwrap_or_default();
    let folders = super::with_book(state, |book| super::profiles::folders(book, &hosts, &home))?;
    Ok(FindProfilesInfo {
        answer: answer.word().to_owned(),
        folders: folders
            .into_iter()
            .map(|folder| ProfilesFolderInfo {
                app_id: folder.app_id,
                folder: super::profiles::shown(&folder.folder, &home),
            })
            .collect(),
    })
}

/// AJ-E3203: whether AI Juggler finds the profiles that aren't running
/// (asked once, on an App's page), and where it looks.
///
/// # Errors
///
/// A poisoned lock; storage errors.
#[tauri::command]
pub async fn places_profiles(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<FindProfilesInfo> {
    crate::ipc::blocking(state.inner(), "finding profiles", profiles_info).await
}

/// AJ-E3203: the user's answer. Yes reads the folder names now; no (or
/// off, from Settings) forgets the profiles found that way.
///
/// # Errors
///
/// A poisoned lock; storage errors.
#[tauri::command]
pub async fn places_profiles_set(
    state: tauri::State<'_, Arc<AppState>>,
    on: bool,
) -> AppResult<FindProfilesInfo> {
    crate::ipc::blocking(state.inner(), "finding profiles", move |state| {
        let home = juggler_platform::user::home().unwrap_or_default();
        super::profiles::set(state, on, &home, crate::commands::now_millis())?;
        profiles_info(state)
    })
    .await
}

/// Forgets a place.
///
/// # Errors
///
/// As [`super::forget`].
#[tauri::command]
pub async fn places_forget(state: tauri::State<'_, Arc<AppState>>, id: String) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "forgetting the app", move |state| {
        super::forget(state, &id)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::{KnownPlaceInfo, place_order};

    #[test]
    fn desktop_apps_list_running_then_last_seen_then_found() {
        let row = |label: &str, pid: Option<u32>, seen_ms: i64, found: bool| KnownPlaceInfo {
            label: label.into(),
            id: label.into(),
            app_pid: pid,
            seen_ms,
            found,
            ..KnownPlaceInfo::default()
        };
        let mut rows = [
            row("Found", None, 0, true),
            row("Old", None, 10, false),
            row("Running", Some(7), 1, false),
            row("Recent", None, 99, false),
        ];
        rows.sort_by_key(place_order);
        let order: Vec<&str> = rows.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(order, ["Running", "Recent", "Old", "Found"]);
    }

    /// D-64: the window gets an app's sessions newest first, all the book
    /// keeps (the rail picks its last 3 itself).
    #[test]
    fn an_apps_sessions_go_newest_first() {
        let book: Vec<String> = (0..100).map(|n| format!("claude-code/s-{n}")).collect();
        let recent = super::recent(&book);
        assert_eq!(recent.len(), 100);
        assert_eq!(recent[0], "claude-code/s-99");
        assert_eq!(recent[99], "claude-code/s-0");
    }
}
