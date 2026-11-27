//! The schedules' IPC (ADR-0039 §6): the user's list (reminders and, later,
//! timed watches and a scheduled digest) with edit, pause, remove and
//! Pause all; Settings › Advanced's read-only list of the app's own; and
//! Remind me, from the feed's picker or Bot's confirmed draft. Only the
//! user's click calls these: nothing here takes agent text.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::reminder::{self, ReminderPayload};
use super::service::context;
use super::store::{self, CreatedBy, NewSchedule, Schedule, ScheduleError};
use super::trigger::{MissedPolicy, Modifiers, Spec, Trigger};
use crate::error::{AppError, AppResult};
use crate::features::watches::model::WatchTarget;
use crate::paging::{Page, PageRequest, SortKey, page_of_fetched};
use crate::state::AppState;

/// One schedule, as a list shows it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ScheduleInfo {
    /// Its id (actions target it by this).
    pub id: String,
    /// Plain words.
    pub title: String,
    /// What runs it: `reminder`, `cloud_backup`, …
    pub handler: String,
    /// Whose: `ai_juggler` (later, another tool's id: shown read-only).
    pub source: String,
    /// When.
    pub trigger: Trigger,
    /// Window, jitter, lifetime.
    pub modifiers: Modifiers,
    /// When it is next due.
    #[ts(type = "number | null")]
    pub next_due_ms: Option<i64>,
    /// When it last ran.
    #[ts(type = "number | null")]
    pub last_run_ms: Option<i64>,
    /// How that went: `ok`, `failed`, `timeout`, `missed`, `no_handler`.
    pub last_outcome: Option<String>,
    /// Paused.
    pub paused: bool,
    /// When it ended (its lifetime over).
    #[ts(type = "number | null")]
    pub ended_ms: Option<i64>,
    /// When it was made.
    #[ts(type = "number")]
    pub created_ms: i64,
    /// `user`, `app` or `bot`.
    pub created_by: String,
    /// What a reminder is about.
    #[ts(optional = nullable)]
    pub about: Option<WatchTarget>,
    /// 0.5.2 G42: the session it is about, named as the rail names it
    /// (the one rule), for a list the window doesn't hold that session
    /// in; none for another kind or one storage doesn't hold.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub about_name: Option<String>,
}

impl ScheduleInfo {
    fn of(found: &Schedule) -> Self {
        let about = (found.row.handler == reminder::HANDLER)
            .then(|| serde_json::from_value::<ReminderPayload>(found.payload.clone()).ok())
            .flatten()
            .and_then(|p| p.about);
        Self {
            id: found.row.id.clone(),
            title: found.row.title.clone(),
            handler: found.row.handler.clone(),
            source: found.row.source.clone(),
            trigger: found.spec.trigger.clone(),
            modifiers: found.spec.modifiers,
            next_due_ms: found.row.next_due_ms,
            last_run_ms: found.row.last_run_ms,
            last_outcome: found.row.last_outcome.clone(),
            paused: found.row.paused,
            ended_ms: found.row.ended_ms,
            created_ms: found.row.created_ms,
            created_by: found.row.created_by.clone(),
            about,
            about_name: None,
        }
    }
}

/// 0.5.2 G42: each reminder about a session named by the one rule
/// ([`crate::features::sessions::names::names_of`]), one read for the
/// list; the caller holds no lock.
fn named(state: &Arc<AppState>, infos: &mut [ScheduleInfo]) {
    let session = |info: &ScheduleInfo| match &info.about {
        Some(WatchTarget::Session(id)) => Some(id.clone()),
        _ => None,
    };
    let ids: Vec<String> = infos.iter().filter_map(session).collect();
    let names =
        crate::features::sessions::names::names_of(state, &ids, crate::commands::now_millis());
    for info in infos {
        info.about_name = session(info).and_then(|id| names.get(&id).cloned());
    }
}

/// [`named`] for one.
fn named_one(state: &Arc<AppState>, mut info: ScheduleInfo) -> ScheduleInfo {
    named(state, std::slice::from_mut(&mut info));
    info
}

/// Which list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum ScheduleListWhich {
    /// The user's own (reminders, …).
    #[default]
    Yours,
    /// The app's own (Settings › Advanced, read-only).
    App,
}

/// The user's list at a glance (its header and Pause all).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SchedulesSummary {
    /// Live and not paused.
    pub active: u32,
    /// Live and paused.
    pub paused: u32,
}

/// A reminder to set (the feed's Remind me, Bot's confirmed draft).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemindRequest {
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// The words it shows (what the user saw when they set it).
    pub title: String,
    /// What it is about.
    #[ts(optional = nullable)]
    pub about: Option<WatchTarget>,
    /// The Bot turn whose draft the user confirmed.
    #[ts(optional = nullable)]
    pub chat_turn: Option<String>,
}

fn refused(error: ScheduleError) -> AppError {
    match error {
        ScheduleError::NotFound => AppError::NotFound {
            entity: "schedule",
            id: String::new(),
        },
        ScheduleError::Storage(message) => AppError::Validation {
            subject: "schedule",
            message,
        },
        ScheduleError::Full => AppError::Validation {
            subject: "schedule",
            message: format!(
                "You have {} reminders already; remove one first.",
                store::USER_MAX
            ),
        },
        ScheduleError::Title => AppError::Validation {
            subject: "schedule",
            message: "A reminder needs a few words.".to_owned(),
        },
        ScheduleError::Invalid("at") => AppError::Validation {
            subject: "schedule",
            message: "That time can't be used.".to_owned(),
        },
        other => AppError::Validation {
            subject: "schedule",
            message: other.to_string(),
        },
    }
}

fn changed(state: &Arc<AppState>) {
    state.schedules.changed();
}

/// Core: sets a reminder. A time already past is due at once.
///
/// # Errors
///
/// Out of bounds (the time, the words, what it is about), the list full,
/// the scheduler not started, storage.
pub fn schedule_remind_impl(
    state: &Arc<AppState>,
    request: &RemindRequest,
) -> AppResult<ScheduleInfo> {
    let registry = state
        .schedules
        .registry()
        .cloned()
        .ok_or(AppError::Validation {
            subject: "schedule",
            message: "Reminders start a moment after AI Juggler opens; try again.".to_owned(),
        })?;
    let payload = serde_json::to_value(ReminderPayload {
        about: request.about.clone(),
    })
    .map_err(|e| AppError::Validation {
        subject: "schedule",
        message: e.to_string(),
    })?;
    let new = NewSchedule {
        owner: reminder::OWNER,
        handler: reminder::HANDLER,
        payload,
        spec: Spec {
            trigger: Trigger::At {
                at_ms: request.at_ms,
            },
            modifiers: Modifiers::once(),
            missed: MissedPolicy::RunOnce,
        },
        user_visible: true,
        created_by: if request.chat_turn.is_some() {
            CreatedBy::Bot
        } else {
            CreatedBy::User
        },
        title: request.title.clone(),
    };
    let now = crate::commands::now_millis();
    let made = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        let ctx = context(&storage);
        store::create(&storage, &registry, new, &ctx, now).map_err(refused)?
    };
    changed(state);
    let info = named_one(state, ScheduleInfo::of(&made));
    // 0.5.2 G30: Bot's turn keeps which reminder came of its draft.
    if let Some(turn) = &request.chat_turn {
        crate::features::bot::commands::reminder_set(state, turn, &info.id);
    }
    Ok(info)
}

/// Core: a page of one list, newest first.
///
/// # Errors
///
/// A cursor from another list; storage.
pub fn schedule_list_impl(
    state: &Arc<AppState>,
    request: &PageRequest,
    which: ScheduleListWhich,
) -> AppResult<Page<ScheduleInfo>> {
    let after = request
        .cursor
        .as_deref()
        .map(SortKey::parse)
        .transpose()?
        .map(|key| (key.ranks.first().copied().unwrap_or(i64::MAX), key.id));
    let limit = request.limit();
    let rows = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage
            .schedules()
            .page(
                which == ScheduleListWhich::Yours,
                after.as_ref(),
                u32::try_from(limit + 1).unwrap_or(u32::MAX),
            )
            .map_err(AppError::from)?
    };
    let fetched = rows
        .into_iter()
        .map(|row| (SortKey::new([row.created_ms], row.id.clone()), row))
        .filter_map(|(key, row)| Schedule::of(row).map(|found| (key, ScheduleInfo::of(&found))))
        .collect();
    let mut page = page_of_fetched(fetched, limit, None);
    named(state, &mut page.items);
    Ok(page)
}

/// Core: the user's list at a glance.
///
/// # Errors
///
/// Storage.
pub fn schedule_summary_impl(state: &Arc<AppState>) -> AppResult<SchedulesSummary> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let (active, paused) = storage.schedules().user_counts().map_err(AppError::from)?;
    Ok(SchedulesSummary { active, paused })
}

/// Core: the user moves one of their reminders to another time, or
/// renames it.
///
/// # Errors
///
/// Not found, not theirs, out of bounds, storage.
pub fn schedule_update_impl(
    state: &Arc<AppState>,
    id: &str,
    at_ms: i64,
    title: &str,
) -> AppResult<ScheduleInfo> {
    let registry = state
        .schedules
        .registry()
        .cloned()
        .ok_or(AppError::NotFound {
            entity: "schedule",
            id: id.to_owned(),
        })?;
    let now = crate::commands::now_millis();
    let edited = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        let ctx = context(&storage);
        store::edit_users(
            &storage,
            &registry,
            id,
            Trigger::At { at_ms },
            Modifiers::once(),
            title,
            &ctx,
            now,
        )
        .map_err(refused)?
    };
    changed(state);
    Ok(named_one(state, ScheduleInfo::of(&edited)))
}

/// Core: pauses or resumes one of the user's schedules.
///
/// # Errors
///
/// Not found, not theirs, storage.
pub fn schedule_pause_impl(
    state: &Arc<AppState>,
    id: &str,
    paused: bool,
) -> AppResult<ScheduleInfo> {
    let now = crate::commands::now_millis();
    let found = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        store::pause_users(&storage, id, paused, now).map_err(refused)?
    };
    changed(state);
    Ok(named_one(state, ScheduleInfo::of(&found)))
}

/// Core: Pause all (or Resume all) in the user's list.
///
/// # Errors
///
/// Storage.
pub fn schedule_pause_all_impl(state: &Arc<AppState>, paused: bool) -> AppResult<SchedulesSummary> {
    let now = crate::commands::now_millis();
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        store::pause_all_users(&storage, paused, now).map_err(refused)?;
    }
    changed(state);
    schedule_summary_impl(state)
}

/// Core: removes one of the user's schedules.
///
/// # Errors
///
/// Not found, not theirs, storage.
pub fn schedule_remove_impl(state: &Arc<AppState>, id: &str) -> AppResult<()> {
    let now = crate::commands::now_millis();
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        store::remove_users(&storage, id, now).map_err(refused)?;
    }
    changed(state);
    Ok(())
}

/// Sets a reminder (see [`schedule_remind_impl`]).
///
/// # Errors
///
/// As [`schedule_remind_impl`].
#[tauri::command]
pub async fn schedule_remind(
    state: tauri::State<'_, Arc<AppState>>,
    request: RemindRequest,
) -> AppResult<ScheduleInfo> {
    crate::ipc::blocking(state.inner(), "schedules", move |state| {
        schedule_remind_impl(state, &request)
    })
    .await
}

/// A page of one list (see [`schedule_list_impl`]).
///
/// # Errors
///
/// As [`schedule_list_impl`].
#[tauri::command]
pub async fn schedule_list(
    state: tauri::State<'_, Arc<AppState>>,
    request: PageRequest,
    which: Option<ScheduleListWhich>,
) -> AppResult<Page<ScheduleInfo>> {
    crate::ipc::blocking(state.inner(), "schedules", move |state| {
        schedule_list_impl(state, &request, which.unwrap_or_default())
    })
    .await
}

/// The user's list at a glance (see [`schedule_summary_impl`]).
///
/// # Errors
///
/// As [`schedule_summary_impl`].
#[tauri::command]
pub async fn schedule_summary(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<SchedulesSummary> {
    crate::ipc::blocking(state.inner(), "schedules", schedule_summary_impl).await
}

/// Moves or renames a reminder (see [`schedule_update_impl`]).
///
/// # Errors
///
/// As [`schedule_update_impl`].
#[tauri::command]
pub async fn schedule_update(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    at_ms: i64,
    title: String,
) -> AppResult<ScheduleInfo> {
    crate::ipc::blocking(state.inner(), "schedules", move |state| {
        schedule_update_impl(state, &id, at_ms, &title)
    })
    .await
}

/// Pauses or resumes one (see [`schedule_pause_impl`]).
///
/// # Errors
///
/// As [`schedule_pause_impl`].
#[tauri::command]
pub async fn schedule_pause(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    paused: bool,
) -> AppResult<ScheduleInfo> {
    crate::ipc::blocking(state.inner(), "schedules", move |state| {
        schedule_pause_impl(state, &id, paused)
    })
    .await
}

/// Pause all or Resume all (see [`schedule_pause_all_impl`]).
///
/// # Errors
///
/// As [`schedule_pause_all_impl`].
#[tauri::command]
pub async fn schedule_pause_all(
    state: tauri::State<'_, Arc<AppState>>,
    paused: bool,
) -> AppResult<SchedulesSummary> {
    crate::ipc::blocking(state.inner(), "schedules", move |state| {
        schedule_pause_all_impl(state, paused)
    })
    .await
}

/// Removes one (see [`schedule_remove_impl`]).
///
/// # Errors
///
/// As [`schedule_remove_impl`].
#[tauri::command]
pub async fn schedule_remove(state: tauri::State<'_, Arc<AppState>>, id: String) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "schedules", move |state| {
        schedule_remove_impl(state, &id)
    })
    .await
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
