//! The places' IPC (0.3.3): the apps and profiles remembered, Open, Keep
//! alive, Forget, and the place an ended session was last open in.

// Tauri command signatures take State by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use super::book::KnownPlace;
use crate::error::AppResult;
use crate::features::conversations::dto::GoToOutcomeInfo;
use crate::features::conversations::places::HostRegistry;
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
    // Its app's word as every screen says it ("Claude app"), for its label.
    word: &str,
    // Its process, when it runs, and whether it lost its window (0.3.8).
    (pid, lost): (Option<u32>, bool),
    hosts: &HostRegistry,
    accounts: &std::collections::HashMap<String, String>,
    slots: &std::collections::BTreeMap<String, u8>,
) -> KnownPlaceInfo {
    let account = place
        .account_id
        .as_ref()
        .and_then(|id| accounts.get(id))
        .map(String::as_str);
    KnownPlaceInfo {
        id: id.to_owned(),
        app_id: place.app_id.clone(),
        app: place.name.clone(),
        instance: place.instance.clone(),
        label: place
            .profile(account)
            .map_or_else(|| word.to_owned(), |profile| format!("{word} · {profile}")),
        profile: place.profile(account),
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
    }
}

/// Each watched place's id, pid and start.
type Watched = Vec<(String, u32, Option<i64>)>;
/// Each watched place's program, as found running.
type Programs = std::collections::HashMap<String, std::path::PathBuf>;

/// The places remembered with their processes, each checked alive now.
pub(crate) fn known(state: &Arc<AppState>) -> AppResult<Vec<KnownPlaceInfo>> {
    // D-84: the user's Apps only; an offer is listed by `places_offers`.
    let places: std::collections::BTreeMap<String, super::book::KnownPlace> =
        super::with_book(state, |book| book.places.clone())?
            .into_iter()
            .filter(|(_, place)| place.users)
            .collect();
    let (pids, exes): (Watched, Programs) = state
        .places
        .watch
        .lock()
        .map(|watch| {
            let known = watch.known();
            let exes = known
                .iter()
                .filter_map(|(id, _, _)| Some((id.clone(), watch.exe_of(id)?.to_path_buf())))
                .collect();
            (known, exes)
        })
        .unwrap_or_default();
    let alive = crate::features::conversations::procs::some(
        &pids.iter().map(|(_, pid, _)| *pid).collect::<Vec<_>>(),
    );
    let hosts = HostRegistry::of(&state.registry);
    let (folders, slots) = state
        .storage
        .lock()
        .ok()
        .map_or_else(Default::default, |storage| {
            (
                storage.accounts().list().ok().unwrap_or_default(),
                storage.apps().colour_slots().ok().unwrap_or_default(),
            )
        });
    let accounts: std::collections::HashMap<String, String> = folders
        .into_iter()
        .map(|account| (account.id.to_string(), account.label))
        .collect();
    Ok(places
        .iter()
        .map(|(id, place)| {
            let running =
                pids.iter()
                    .find(|(known, _, _)| known == id)
                    .filter(|(_, pid, start)| {
                        alive.get(pid).is_some_and(|process| {
                            super::is_its_process(
                                exes.get(id).map_or(&place.exe, |exe| exe),
                                *start,
                                process,
                            )
                        })
                    });
            let lost = running.is_some_and(|(_, pid, start)| {
                super::start::Starter::detached(&super::start::System, *pid, *start)
            });
            info(
                id,
                place,
                &super::app_word(state, place),
                (running.map(|(_, pid, _)| *pid), lost),
                &hosts,
                &accounts,
                &slots,
            )
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
        Ok(super::offers(state)?
            .into_iter()
            .map(|(id, label)| PlaceOfferInfo { id, label })
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
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "add the app", move |state| {
        super::answer_offer(state, &id, add)
    })
    .await
}

/// Open: brings the place forward when it runs, else starts it; with a
/// session, at that session where its app has a link to it.
///
/// # Errors
///
/// As [`super::open`].
#[tauri::command]
pub async fn places_open(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    conversation_id: Option<String>,
) -> AppResult<GoToOutcomeInfo> {
    crate::ipc::blocking(state.inner(), "opening the app", move |state| {
        super::open(
            state,
            &id,
            conversation_id.as_deref(),
            Arc::new(crate::features::conversations::goto::MacDesktop),
            Arc::new(super::start::System),
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
            },
            Err(why) => GoToOutcomeInfo {
                outcome: why,
                reached: "none".to_owned(),
                place_id: None,
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
    let home = dirs::home_dir().unwrap_or_default();
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
        let home = dirs::home_dir().unwrap_or_default();
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
