//! AJ-E3511, AJ-E3512 (D-78): what Settings › The Floor and the Floor list
//! do — the settings (each a `floor.*` key, an open Floor reading them
//! again), Re-create the office, Reset the view, an App left off the Floor
//! or put back, each App's floor for the list, and Show on the Floor.

// Tauri command signatures take State by value.
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use serde::Serialize;
use ts_rs::TS;

use super::render::Msg;
use super::store::{
    self, ARRANGE_KEY, AT_START_KEY, Arrange, DECAY_KEY, DETAIL_KEY, LABELS_KEY, LIGHT_KEY, Labels,
    Light, MOTION_KEY, ON_TOP_KEY, PER_FLOOR_KEY, SHOW_KEY, Settings, Show, Word,
};
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The Floor's settings, as Settings › The Floor shows them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FloorSettingsInfo {
    /// Open the Floor when AI Juggler starts.
    pub open_at_start: bool,
    /// Keep it on top.
    pub on_top: bool,
    /// Apps a floor holds (4–16).
    pub apps_per_floor: u32,
    /// `system` | `full` | `calm` | `off`.
    pub motion: String,
    /// D-80: `pixel` (Pixel art) | `full` (Full resolution).
    pub detail: String,
    /// Show unused Apps' decay.
    pub decay: bool,
    /// The Apps left off the Floor, by id.
    pub hidden: Vec<String>,
}

/// Where one App's main area is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FloorPlaceInfo {
    /// The App.
    pub app_id: String,
    /// Its floor, 0 the first.
    pub floor: u32,
    /// Its order on the Floor.
    pub order: u32,
}

fn settings_info(storage: &juggler_core::storage::Storage) -> AppResult<FloorSettingsInfo> {
    let settings = Settings::read(storage)?;
    let motion = storage
        .app_settings()
        .get(MOTION_KEY)?
        .filter(|v| ["full", "calm", "off"].contains(&v.as_str()));
    let hidden = storage
        .floor()
        .areas()?
        .into_iter()
        .filter(|a| a.hidden)
        .map(|a| a.app_id)
        .collect();
    Ok(FloorSettingsInfo {
        open_at_start: settings.at_start,
        on_top: settings.on_top,
        apps_per_floor: u32::try_from(settings.per_floor).unwrap_or(10),
        motion: motion.unwrap_or_else(|| "system".to_owned()),
        detail: if settings.detail.full() {
            "full"
        } else {
            "pixel"
        }
        .to_owned(),
        decay: settings.decay,
        hidden,
    })
}

/// The stored word for `key` = `value`, checked.
fn checked(key: &str, value: &str) -> AppResult<String> {
    let bad = || AppError::Refused(format!("The Floor has no setting {key} = {value}."));
    match key {
        AT_START_KEY | ON_TOP_KEY | DECAY_KEY => match value {
            "on" | "off" => Ok(value.to_owned()),
            _ => Err(bad()),
        },
        PER_FLOOR_KEY => value
            .parse::<u32>()
            .ok()
            .filter(|n| (4..=16).contains(n))
            .map(|n| n.to_string())
            .ok_or_else(bad),
        MOTION_KEY => match value {
            "system" | "full" | "calm" | "off" => Ok(value.to_owned()),
            _ => Err(bad()),
        },
        DETAIL_KEY => match value {
            "pixel" | "full" => Ok(value.to_owned()),
            _ => Err(bad()),
        },
        LIGHT_KEY if Light::WORDS.iter().any(|(_, w)| *w == value) => Ok(value.to_owned()),
        LABELS_KEY if Labels::WORDS.iter().any(|(_, w)| *w == value) => Ok(value.to_owned()),
        SHOW_KEY if Show::WORDS.iter().any(|(_, w)| *w == value) => Ok(value.to_owned()),
        ARRANGE_KEY if Arrange::WORDS.iter().any(|(_, w)| *w == value) => Ok(value.to_owned()),
        _ => Err(bad()),
    }
}

/// Settings › The Floor's values.
///
/// # Errors
///
/// Storage errors.
#[tauri::command]
pub async fn floor_settings(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<FloorSettingsInfo> {
    crate::ipc::blocking(state.inner(), "the Floor's settings", |state| {
        let storage = lock_storage_arc(state)?;
        settings_info(&storage)
    })
    .await
}

/// Changes one of the Floor's settings; an open Floor reads it at once.
///
/// # Errors
///
/// [`AppError::Refused`] for a key or value the Floor doesn't have;
/// storage errors.
#[tauri::command]
pub async fn floor_setting_set(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    key: String,
    value: String,
) -> AppResult<FloorSettingsInfo> {
    let info = crate::ipc::blocking(state.inner(), "the Floor's settings", move |state| {
        let word = checked(&key, &value)?;
        let storage = lock_storage_arc(state)?;
        storage.app_settings().set(&key, &word)?;
        settings_info(&storage)
    })
    .await?;
    super::window::reload(&app);
    Ok(info)
}

/// Re-create the office: a new look and layout for every area, the same
/// order; nothing outside the Floor changes. An open Floor redraws.
///
/// # Errors
///
/// Storage errors, and then nothing changes.
#[tauri::command]
pub async fn floor_recreate(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "re-creating the office", |state| {
        let storage = lock_storage_arc(state)?;
        let settings = Settings::read(&storage)?;
        let (mut office, _) = store::load(&storage, settings.per_floor, 0)?;
        let seed = office.seed.wrapping_mul(2_654_435_761).wrapping_add(
            u32::try_from(juggler_core::model::Timestamp::now().millis() & 0xffff_ffff)
                .unwrap_or(1),
        );
        office.recreate(seed);
        let looks: Vec<(String, String, u32)> = store::rows_of(&office)
            .into_iter()
            .map(|row| (row.app_id, row.layout, row.seed))
            .collect();
        storage.floor().recreate(seed, &looks)?;
        Ok(())
    })
    .await?;
    super::window::reload(&app);
    Ok(())
}

/// Leaves an App off the Floor, or puts it back.
///
/// # Errors
///
/// Storage errors.
#[tauri::command]
pub async fn floor_app_hidden_set(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    app_id: String,
    hidden: bool,
) -> AppResult<FloorSettingsInfo> {
    let info = crate::ipc::blocking(state.inner(), "the Floor's Apps", move |state| {
        let storage = lock_storage_arc(state)?;
        storage.floor().set_hidden(&app_id, hidden)?;
        settings_info(&storage)
    })
    .await?;
    super::window::reload(&app);
    Ok(info)
}

/// Each App's floor, in the Floor's order (the list groups by it).
///
/// # Errors
///
/// Storage errors.
#[tauri::command]
pub async fn floor_places(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<FloorPlaceInfo>> {
    crate::ipc::blocking(state.inner(), "the Floor's places", |state| {
        let storage = lock_storage_arc(state)?;
        let settings = Settings::read(&storage)?;
        let (office, _) = store::load(&storage, settings.per_floor, 0)?;
        let plan = juggler_floor::layout::pack(&office);
        let order = |id: &str| office.apps.get(id).map_or(u32::MAX, |app| app.order);
        Ok(plan
            .areas
            .iter()
            .filter(|area| area.annex == 0)
            .map(|area| FloorPlaceInfo {
                app_id: area.id.clone(),
                floor: u32::try_from(area.floor).unwrap_or(0),
                order: order(&area.id),
            })
            .collect())
    })
    .await
}

/// Reset the view: an open Floor's camera back to the first floor.
///
/// # Errors
///
/// Never; the result type matches the others.
#[tauri::command]
pub async fn floor_reset_view(app: tauri::AppHandle) -> AppResult<()> {
    super::window::send(&app, Msg::ShowFloor(0));
    Ok(())
}

/// Show on the Floor: opens the Floor (or brings it forward) on the floor
/// that holds `app_id`'s area.
///
/// # Errors
///
/// As [`super::commands::floor_open`].
#[tauri::command]
pub async fn floor_show_app(app: tauri::AppHandle, app_id: String) -> AppResult<()> {
    let opener = app.clone();
    tauri::async_runtime::spawn_blocking(move || super::window::open(&opener))
        .await
        .map_err(|e| AppError::Refused(format!("The Floor couldn't open: {e}.")))??;
    super::window::send(&app, Msg::ShowApp(app_id));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::checked;

    #[test]
    fn only_the_floors_own_settings_and_values_are_kept() {
        assert_eq!(checked("floor.on_top", "on").unwrap(), "on");
        assert_eq!(checked("floor.apps_per_floor", "6").unwrap(), "6");
        assert_eq!(checked("floor.motion", "system").unwrap(), "system");
        assert_eq!(checked("floor.detail", "pixel").unwrap(), "pixel");
        assert_eq!(checked("floor.detail", "full").unwrap(), "full");
        for (key, value) in [
            ("floor.apps_per_floor", "3"),
            ("floor.apps_per_floor", "17"),
            ("floor.apps_per_floor", "ten"),
            ("floor.motion", "fast"),
            ("floor.detail", "4k"),
            ("floor.decay", "yes"),
            ("floor.seed", "1"),
            ("ui.theme", "dark"),
        ] {
            assert!(checked(key, value).is_err(), "{key} = {value}");
        }
    }
}
