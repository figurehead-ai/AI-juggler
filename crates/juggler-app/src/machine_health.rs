//! E14 machine health: how the Mac is doing, for Home's This Mac panel,
//! the launch advisor and the overload alerts (PROPOSAL §6.5, SPEC §5.3).
//!
//! AJ-E1401: the machine sampler runs on the 2 s processes tick; this
//! module turns its snapshot into the read model the UI shows, with the
//! one shared summary ("Plenty of room" / "Getting tight" / "Under
//! pressure") and its reasons in words.
//!
//! Passive only: nothing here stops, pauses or launches anything.

// Tauri command signatures take State/values by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;
use std::sync::Arc;

use juggler_supervisor::machine::{
    MachineSnapshot, MemoryBasis, PowerState, PressureLevel, RoomLevel, RoomThresholds,
    ThermalState, summarize, swap_growth_mb_per_min,
};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// One reason the Mac is not "Plenty of room".
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RoomReasonInfo {
    /// `pressure` | `memory` | `swap` | `cpu` | `thermal` | `disk`.
    pub kind: String,
    /// `plenty` | `tight` | `pressure` — what this reason alone means.
    pub level: String,
    /// The reason in plain words.
    pub words: String,
}

/// The machine summary every surface shows.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RoomSummaryInfo {
    /// `plenty` | `tight` | `pressure`.
    pub level: String,
    /// "Plenty of room" | "Getting tight" | "Under pressure".
    pub words: String,
    /// Why, worst first; empty when there is plenty of room.
    pub reasons: Vec<RoomReasonInfo>,
}

/// One point of the 5-minute series.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MachinePointInfo {
    /// Sample time, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// Whole-Mac CPU, percent of all cores; `null` when unmeasured.
    pub cpu_pct: Option<f64>,
    /// Memory in use, bytes, on `MachineHealthInfo::memory_basis`; `null`
    /// when it couldn't be read.
    #[ts(type = "number | null")]
    pub memory_used: Option<u64>,
}

/// A volume in use and its free space.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct VolumeInfo {
    /// The volume's name.
    pub name: String,
    /// Total bytes.
    #[ts(type = "number")]
    pub total_bytes: u64,
    /// Bytes free.
    #[ts(type = "number")]
    pub available_bytes: u64,
}

/// Power as the OS reports it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PowerInfo {
    /// On battery (not on a power adapter).
    pub on_battery: bool,
    /// The OS estimate of minutes left, when it has one.
    pub minutes_left: Option<u32>,
    /// The OS low-battery warning is up.
    pub battery_low: bool,
    /// Low Power Mode is on.
    pub low_power_mode: bool,
}

/// How the Mac is doing (AJ-E1401 Req.4).
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MachineHealthInfo {
    /// False until the first sample (the UI hides what it can't show).
    pub sampled: bool,
    /// Logical cores.
    pub cores: u32,
    /// Whole-Mac CPU now, percent of all cores; `null` until measured.
    pub cpu_pct: Option<f64>,
    /// Physical memory, bytes.
    #[ts(type = "number")]
    pub memory_total: u64,
    /// Memory in use, bytes, measured as `memory_basis` says; `null` when
    /// the reading failed (nothing is shown then — never another measure
    /// under the same name).
    #[ts(type = "number | null")]
    pub memory_used: Option<u64>,
    /// What `memory_used` measures: `app_wired_compressed` (macOS: app
    /// memory + wired + the compressor's data — close to Activity Monitor's
    /// "Memory Used", a few percent lower) | `total_minus_available`
    /// (other systems); `null` when it couldn't be read.
    pub memory_basis: Option<String>,
    /// Memory free on the same basis (total − used, so used + free =
    /// total); the OS's own "available" only when used couldn't be read.
    #[ts(type = "number")]
    pub memory_available: u64,
    /// `normal` | `warning` | `critical`; `null` where the OS has none.
    pub pressure: Option<String>,
    /// Swap size, bytes.
    #[ts(type = "number")]
    pub swap_total: u64,
    /// Swap in use, bytes.
    #[ts(type = "number")]
    pub swap_used: u64,
    /// Swap growth over the last minute, MB a minute; `null` with too
    /// little history.
    pub swap_growth_mb_per_min: Option<f64>,
    /// Volumes in use.
    pub volumes: Vec<VolumeInfo>,
    /// Power; `null` where the OS does not report it.
    pub power: Option<PowerInfo>,
    /// `nominal` | `fair` | `serious` | `critical`; `null` where unknown.
    pub thermal: Option<String>,
    /// The summary in words.
    pub summary: RoomSummaryInfo,
    /// The last 5 minutes, oldest first.
    pub series: Vec<MachinePointInfo>,
}

const fn level_key(level: RoomLevel) -> &'static str {
    match level {
        RoomLevel::Plenty => "plenty",
        RoomLevel::Tight => "tight",
        RoomLevel::Pressure => "pressure",
    }
}

const fn pressure_key(level: PressureLevel) -> &'static str {
    match level {
        PressureLevel::Normal => "normal",
        PressureLevel::Warning => "warning",
        PressureLevel::Critical => "critical",
    }
}

const fn thermal_key(state: ThermalState) -> &'static str {
    match state {
        ThermalState::Nominal => "nominal",
        ThermalState::Fair => "fair",
        ThermalState::Serious => "serious",
        ThermalState::Critical => "critical",
    }
}

fn power_info(power: PowerState) -> PowerInfo {
    PowerInfo {
        on_battery: power.on_battery,
        minutes_left: power.minutes_left,
        battery_low: power.battery_low,
        low_power_mode: power.low_power_mode,
    }
}

const fn basis_key(basis: MemoryBasis) -> Option<&'static str> {
    match basis {
        MemoryBasis::AppWiredCompressed => Some("app_wired_compressed"),
        MemoryBasis::TotalMinusAvailable => Some("total_minus_available"),
        MemoryBasis::Unknown => None,
    }
}

/// Pure: the read model from a sampler snapshot.
#[must_use]
pub fn health_of(snapshot: &MachineSnapshot, thresholds: &RoomThresholds) -> MachineHealthInfo {
    let summary = summarize(snapshot, thresholds);
    let latest = snapshot.latest.unwrap_or_default();
    MachineHealthInfo {
        sampled: snapshot.latest.is_some(),
        cores: snapshot.cores,
        cpu_pct: latest.cpu_pct,
        memory_total: latest.memory_total,
        memory_used: latest.memory_used,
        memory_basis: latest
            .memory_used
            .and(basis_key(latest.memory_basis))
            .map(str::to_string),
        memory_available: latest.memory_free(),
        pressure: latest.pressure.map(|level| pressure_key(level).to_string()),
        swap_total: latest.swap_total,
        swap_used: latest.swap_used,
        swap_growth_mb_per_min: swap_growth_mb_per_min(&snapshot.ring),
        volumes: snapshot
            .volumes
            .iter()
            .map(|volume| VolumeInfo {
                name: volume.name.clone(),
                total_bytes: volume.total_bytes,
                available_bytes: volume.available_bytes,
            })
            .collect(),
        power: snapshot.power.map(power_info),
        thermal: snapshot.thermal.map(|state| thermal_key(state).to_string()),
        summary: RoomSummaryInfo {
            level: level_key(summary.level).to_string(),
            words: summary.words.to_string(),
            reasons: summary
                .reasons
                .into_iter()
                .map(|reason| RoomReasonInfo {
                    kind: reason.kind.to_string(),
                    level: level_key(reason.level).to_string(),
                    words: reason.words,
                })
                .collect(),
        },
        series: snapshot
            .ring
            .iter()
            .map(|sample| MachinePointInfo {
                at_ms: sample.at_ms,
                cpu_pct: sample.cpu_pct,
                memory_used: sample.memory_used,
            })
            .collect(),
    }
}

/// The folders whose volumes matter: the data folder and every active
/// session's working folder (one query for active sessions only).
fn paths_in_use(state: &Arc<AppState>) -> Vec<PathBuf> {
    let mut paths = vec![state.data_dir.clone()];
    // Resolved before the storage lock below (it takes that lock itself).
    let machine = crate::commands::local_machine_id(state);
    if let Ok(storage) = state.storage.lock()
        && let Ok(cwds) = storage.sessions().active_cwds(&machine)
    {
        paths.extend(cwds);
    }
    paths
}

/// The 2 s tick's machine half (AJ-E1401 Req.1). The sampler's lock is
/// held only for the cheap part (microseconds); the disk half — the folders
/// in use (a storage read) and, every 10 minutes, listing the disks (50–150
/// ms) — runs outside it, so a reader of the lock never waits on either.
pub fn tick(state: &Arc<AppState>, now_ms: i64) {
    let work = {
        let Ok(mut sampler) = state.machine.lock() else {
            return;
        };
        sampler.tick(now_ms)
    };
    let Some(mut work) = work else {
        return;
    };
    let volumes = work.volumes(&paths_in_use(state));
    if let Ok(mut sampler) = state.machine.lock() {
        sampler.finish_disks(work, volumes);
    }
}

/// The read model now.
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when the sampler's lock is poisoned.
pub fn machine_health_impl(state: &Arc<AppState>) -> AppResult<MachineHealthInfo> {
    let sampler = state.machine.lock().map_err(|_| AppError::StatePoisoned)?;
    Ok(health_of(sampler.snapshot(), &RoomThresholds::default()))
}

/// How the Mac is doing (AJ-E1401 Req.4). Off the main thread: it takes a
/// lock the sampling tick also takes.
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when the sampler's lock is poisoned.
#[tauri::command]
pub async fn machine_health(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<MachineHealthInfo> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || machine_health_impl(&state))
        .await
        .map_err(|error| AppError::Io {
            context: "read how the Mac is doing",
            message: error.to_string(),
        })?
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use juggler_supervisor::machine::{MachineSample, VolumeSample};

    const G: u64 = 1_000_000_000;

    #[test]
    fn an_unsampled_mac_says_so() {
        let info = health_of(&MachineSnapshot::default(), &RoomThresholds::default());
        assert!(!info.sampled);
        assert_eq!(info.summary.level, "plenty");
        assert!(info.series.is_empty());
    }

    #[test]
    fn the_read_model_carries_words_series_and_volumes() {
        let ring: Vec<MachineSample> = (0..3)
            .map(|i| MachineSample {
                at_ms: i * 2_000,
                cpu_pct: (i > 0).then_some(25.0),
                memory_total: 32 * G,
                memory_available: 5 * G,
                memory_used: Some(30 * G),
                memory_basis: MemoryBasis::TotalMinusAvailable,
                pressure: Some(PressureLevel::Warning),
                swap_total: 4 * G,
                swap_used: G,
            })
            .collect();
        let snapshot = MachineSnapshot {
            cores: 10,
            latest: ring.last().copied(),
            ring,
            volumes: vec![VolumeSample {
                name: "Macintosh HD".into(),
                mount_point: "/".into(),
                total_bytes: 1_000 * G,
                available_bytes: 400 * G,
            }],
            power: None,
            thermal: Some(ThermalState::Nominal),
        };
        let info = health_of(&snapshot, &RoomThresholds::default());
        assert!(info.sampled);
        assert_eq!(info.memory_used, Some(30 * G));
        assert_eq!(info.memory_basis.as_deref(), Some("total_minus_available"));
        assert_eq!(
            info.memory_available,
            2 * G,
            "free on the same basis as used"
        );
        assert_eq!(info.pressure.as_deref(), Some("warning"));
        assert_eq!(info.thermal.as_deref(), Some("nominal"));
        assert_eq!(info.summary.words, "Getting tight");
        let kinds: Vec<&str> = info
            .summary
            .reasons
            .iter()
            .map(|r| r.kind.as_str())
            .collect();
        assert_eq!(kinds, ["pressure", "memory"]);
        assert_eq!(info.series.len(), 3);
        assert_eq!(info.series[0].cpu_pct, None);
        assert_eq!(info.volumes[0].name, "Macintosh HD");
        assert_eq!(
            info.swap_growth_mb_per_min, None,
            "4 s is too little history"
        );
    }

    #[test]
    fn memory_says_what_it_measures_and_free_adds_up() {
        let sample = MachineSample {
            memory_total: 32 * G,
            memory_available: 20 * G,
            memory_used: Some(26 * G),
            memory_basis: MemoryBasis::AppWiredCompressed,
            ..MachineSample::default()
        };
        let snapshot = MachineSnapshot {
            cores: 8,
            latest: Some(sample),
            ring: vec![sample],
            ..MachineSnapshot::default()
        };
        let info = health_of(&snapshot, &RoomThresholds::default());
        assert_eq!(
            info.memory_used,
            Some(26 * G),
            "not total − available (12 GB)"
        );
        assert_eq!(info.memory_basis.as_deref(), Some("app_wired_compressed"));
        assert_eq!(info.memory_available, 6 * G, "used + free = total");
        assert_eq!(info.series[0].memory_used, Some(26 * G));
    }

    #[test]
    fn a_failed_reading_shows_nothing_rather_than_another_measure() {
        let sample = MachineSample {
            memory_total: 32 * G,
            memory_available: 20 * G,
            memory_used: None,
            memory_basis: MemoryBasis::Unknown,
            ..MachineSample::default()
        };
        let snapshot = MachineSnapshot {
            cores: 8,
            latest: Some(sample),
            ring: vec![sample],
            ..MachineSnapshot::default()
        };
        let info = health_of(&snapshot, &RoomThresholds::default());
        assert_eq!(info.memory_used, None);
        assert_eq!(info.memory_basis, None);
        assert_eq!(info.series[0].memory_used, None);
    }
}
