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
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
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
}

fn info(
    id: &str,
    place: &KnownPlace,
    pid: Option<u32>,
    hosts: &HostRegistry,
    accounts: &std::collections::HashMap<String, String>,
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
        label: place.named(account),
        profile: place.profile(account),
        app_pid: pid,
        keep_alive: place.keep_alive,
        paused: place.paused.clone(),
        account_id: place.account_id.clone(),
        seen_ms: place.seen_ms,
        cannot_start: super::start::plan(place, hosts, false).err(),
    }
}

/// The places remembered with their processes, each checked alive now.
fn known(state: &Arc<AppState>) -> AppResult<Vec<KnownPlaceInfo>> {
    let places = super::with_book(state, |book| book.places.clone())?;
    let pids: Vec<(String, u32, Option<i64>)> = state
        .places
        .watch
        .lock()
        .map(|watch| watch.known())
        .unwrap_or_default();
    let alive = crate::features::conversations::procs::some(
        &pids.iter().map(|(_, pid, _)| *pid).collect::<Vec<_>>(),
    );
    let hosts = HostRegistry::of(&state.registry);
    let accounts: std::collections::HashMap<String, String> = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.accounts().list().ok())
        .unwrap_or_default()
        .into_iter()
        .map(|account| (account.id.to_string(), account.label))
        .collect();
    Ok(places
        .iter()
        .map(|(id, place)| {
            let pid = pids
                .iter()
                .find(|(known, _, _)| known == id)
                .filter(|(_, pid, start)| {
                    alive.get(pid).is_some_and(|process| {
                        juggler_adapter::live::same_start(*start, process.start)
                    })
                })
                .map(|(_, pid, _)| *pid);
            info(id, place, pid, &hosts, &accounts)
        })
        .collect())
}

/// The places remembered, kept-alive first, then by name (at most
/// `book::MAX`, so one page).
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
        crate::paging::page(
            rows,
            |row| {
                SortKey::new(
                    vec![i64::from(!row.keep_alive)],
                    format!("{}\u{0}{}", row.label, row.id),
                )
            },
            &page.unwrap_or_default(),
        )
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
) -> AppResult<Option<KnownPlaceInfo>> {
    crate::ipc::blocking(state.inner(), "the session's app", move |state| {
        let id = super::with_book(state, |book| {
            book.place_of(&tool, &conversation_id)
                .map(|(id, _)| id.clone())
        })?;
        Ok(id.and_then(|id| known(state).ok()?.into_iter().find(|row| row.id == id)))
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
