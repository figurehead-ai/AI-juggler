//! Machine signals (AJ-E1401, SPEC §5.3): how the Mac itself is doing —
//! CPU, memory and the kernel's memory-pressure level, swap, free space on
//! the volumes in use, power and thermal state — sampled on the existing
//! 2 s tick with a 5-minute ring, and summarised in plain words by ONE
//! pure function every surface shares.
//!
//! Cadences (staggered, like the per-tree collector): CPU, memory,
//! pressure and swap every tick; free space on the disks in use every
//! 15th tick (30 s; the disk list itself every 10 min); power and
//! thermal every 30th tick (60 s). One `sysinfo::System` for the
//! sampler's lifetime — never one per tick.
//!
//! Honest values: what the OS does not report stays `None` (shown as
//! "—" with a reason), never a fabricated zero. Whole-machine CPU needs
//! two refreshes, so the first sample's CPU is `None`.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Ring capacity: 150 samples × 2 s = 5 minutes (the AJ-E410 budget).
pub const MACHINE_RING_CAPACITY: usize = 150;
/// Disks are re-read every 15th tick (30 s at the 2 s cadence).
const DISKS_EVERY: u32 = 15;
/// Power and thermal are re-read every 30th tick (60 s).
const POWER_EVERY: u32 = 30;
/// The disk LIST is rebuilt every 300th tick (10 min): listing costs
/// 50–150 ms on macOS, while refreshing one listed disk's free space
/// costs about a microsecond.
const DISK_LIST_EVERY: u32 = 300;

/// The kernel's own judgement of memory (macOS
/// `kern.memorystatus_vm_pressure_level`). Trust it over "used memory",
/// which on macOS is always high because of the file cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureLevel {
    /// Normal.
    Normal,
    /// The kernel is compressing and swapping to keep up.
    Warning,
    /// The kernel is out of room.
    Critical,
}

/// The OS thermal state (`ProcessInfo.thermalState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThermalState {
    /// No throttling.
    Nominal,
    /// Slightly warm; no visible effect.
    Fair,
    /// The OS is slowing things down to cool off.
    Serious,
    /// Heavily throttled.
    Critical,
}

/// Power as the OS reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct PowerState {
    /// True when running on battery (not on a power adapter).
    pub on_battery: bool,
    /// The OS estimate of minutes left on battery, when it has one.
    pub minutes_left: Option<u32>,
    /// The OS low-battery warning is up.
    pub battery_low: bool,
    /// Low Power Mode is on.
    pub low_power_mode: bool,
}

/// Free space on one volume in use.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct VolumeSample {
    /// The volume's name ("Macintosh HD").
    pub name: String,
    /// Its mount point.
    pub mount_point: String,
    /// Total bytes.
    pub total_bytes: u64,
    /// Bytes available to the user.
    pub available_bytes: u64,
}

/// What "memory in use" measures, so every surface can say so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryBasis {
    /// macOS: app memory (anonymous pages not purgeable) + wired + the
    /// compressor's compressed data. Close to Activity Monitor's "Memory
    /// Used", a few percent lower: Activity Monitor counts the pages the
    /// compressor occupies, which only `host_statistics64` (unsafe FFI)
    /// reports.
    AppWiredCompressed,
    /// Elsewhere: total − what the OS lists as available.
    TotalMinusAvailable,
    /// The reading failed; nothing is shown.
    #[default]
    Unknown,
}

/// macOS memory in use, by part (bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryParts {
    /// Anonymous pages that aren't purgeable.
    pub app: u64,
    /// Wired pages.
    pub wired: u64,
    /// The compressor's compressed data.
    pub compressed: u64,
}

impl MemoryParts {
    /// The parts together.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.app
            .saturating_add(self.wired)
            .saturating_add(self.compressed)
    }
}

/// One 2 s machine sample (the ring's element).
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize)]
pub struct MachineSample {
    /// Sample time, epoch ms.
    pub at_ms: i64,
    /// Whole-machine CPU, percent of ALL cores (0–100); `None` on the
    /// first sample (a delta needs two refreshes).
    pub cpu_pct: Option<f64>,
    /// Physical memory, bytes.
    pub memory_total: u64,
    /// Memory the OS can hand out without swapping, bytes.
    pub memory_available: u64,
    /// Memory in use, bytes, measured as `memory_basis` says; `None` when
    /// the OS reading failed (then nothing is shown — never a different
    /// measure under the same name).
    pub memory_used: Option<u64>,
    /// What `memory_used` measures.
    pub memory_basis: MemoryBasis,
    /// The kernel's pressure level; `None` where the OS has none.
    pub pressure: Option<PressureLevel>,
    /// Swap size, bytes.
    pub swap_total: u64,
    /// Swap in use, bytes.
    pub swap_used: u64,
}

impl MachineSample {
    /// Memory free, on the same basis as `memory_used` (total − used), so
    /// used + free = total; the OS's available figure only when the
    /// reading failed.
    #[must_use]
    pub fn memory_free(&self) -> u64 {
        self.memory_used.map_or(self.memory_available, |used| {
            self.memory_total.saturating_sub(used)
        })
    }
}

/// Everything the sampler knows right now.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize)]
pub struct MachineSnapshot {
    /// Logical cores.
    pub cores: u32,
    /// The newest 2 s sample; `None` before the first tick.
    pub latest: Option<MachineSample>,
    /// The last 5 minutes, oldest first.
    pub ring: Vec<MachineSample>,
    /// Volumes in use (the data folder's, running sessions' folders').
    pub volumes: Vec<VolumeSample>,
    /// Power; `None` where the OS does not report it.
    pub power: Option<PowerState>,
    /// Thermal state; `None` where the OS does not report it.
    pub thermal: Option<ThermalState>,
}

/// How the Mac is doing, in three words-worth of levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomLevel {
    /// "Plenty of room".
    Plenty,
    /// "Getting tight".
    Tight,
    /// "Under pressure".
    Pressure,
}

impl RoomLevel {
    /// The level in words.
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::Plenty => "Plenty of room",
            Self::Tight => "Getting tight",
            Self::Pressure => "Under pressure",
        }
    }
}

/// One reason the Mac is not "Plenty of room".
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RoomReason {
    /// Stable key (`pressure`, `memory`, `swap`, `cpu`, `thermal`, `disk`).
    pub kind: &'static str,
    /// The level this reason alone puts the Mac at.
    pub level: RoomLevel,
    /// The reason in plain words ("swap is growing — 240 MB a minute").
    pub words: String,
}

/// The summary every surface shows.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RoomSummary {
    /// The worst level any reason reached.
    pub level: RoomLevel,
    /// The level in words.
    pub words: &'static str,
    /// Why, worst first; empty when there is plenty of room.
    pub reasons: Vec<RoomReason>,
}

/// The thresholds behind [`summarize`] (SPEC §5.3 defaults).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoomThresholds {
    /// Available memory under this share of RAM is tight.
    pub memory_tight_share: f64,
    /// … and under this share is pressure.
    pub memory_pressure_share: f64,
    /// Swap growing faster than this (MB a minute) is tight.
    pub swap_tight_mb_per_min: f64,
    /// … and faster than this is pressure.
    pub swap_pressure_mb_per_min: f64,
    /// Whole-Mac CPU above this percent …
    pub cpu_busy_pct: f64,
    /// … for this long (every sample in the window) is tight.
    pub cpu_sustained_ms: i64,
    /// A volume with less than this many bytes free is tight (absolute,
    /// not a share: 5 % of a 1 TB disk is still 50 GB of room) …
    pub disk_tight_bytes: u64,
    /// … and under this many bytes free is pressure.
    pub disk_pressure_bytes: u64,
}

impl Default for RoomThresholds {
    fn default() -> Self {
        Self {
            memory_tight_share: 0.10,
            memory_pressure_share: 0.05,
            swap_tight_mb_per_min: 100.0,
            swap_pressure_mb_per_min: 500.0,
            cpu_busy_pct: 90.0,
            cpu_sustained_ms: 60_000,
            disk_tight_bytes: 20 * GB,
            disk_pressure_bytes: 5 * GB,
        }
    }
}

const GB: u64 = 1_000_000_000;
const MB: f64 = 1_000_000.0;

/// Bytes in plain words ("3.2 GB", "640 MB", "under 1 MB").
#[must_use]
#[allow(clippy::cast_precision_loss)] // display rounding only
pub fn bytes_words(bytes: u64) -> String {
    let value = bytes as f64;
    if value < 1e6 {
        return "under 1 MB".to_string();
    }
    if value >= 1e9 {
        let gb = value / 1e9;
        if gb >= 10.0 {
            format!("{gb:.0} GB")
        } else {
            format!("{gb:.1} GB")
        }
    } else {
        format!("{:.0} MB", value / 1e6)
    }
}

/// Swap growth over the ring's last minute (MB a minute); `None` with
/// fewer than 30 s of history. Shrinking swap is 0.
#[must_use]
#[allow(clippy::cast_precision_loss)] // MB/min is a display-grade rate
pub fn swap_growth_mb_per_min(ring: &[MachineSample]) -> Option<f64> {
    let newest = ring.last()?;
    let oldest = ring
        .iter()
        .find(|sample| newest.at_ms - sample.at_ms <= 60_000)?;
    let span_ms = newest.at_ms - oldest.at_ms;
    if span_ms < 30_000 {
        return None;
    }
    let grown = newest.swap_used.saturating_sub(oldest.swap_used) as f64 / MB;
    Some(grown * 60_000.0 / span_ms as f64)
}

/// Whole-Mac CPU when EVERY sample in the last `window_ms` is above
/// `limit` (and the window really spans it); the window's mean.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn sustained_cpu(ring: &[MachineSample], limit: f64, window_ms: i64) -> Option<f64> {
    let newest = ring.last()?;
    let window: Vec<&MachineSample> = ring
        .iter()
        .filter(|sample| newest.at_ms - sample.at_ms <= window_ms)
        .collect();
    let span = newest.at_ms - window.first()?.at_ms;
    if span < window_ms * 9 / 10 {
        return None; // not enough history to call anything sustained
    }
    let values: Vec<f64> = window.iter().filter_map(|sample| sample.cpu_pct).collect();
    if values.len() != window.len() || values.iter().any(|cpu| *cpu <= limit) {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

/// THE machine summary (AJ-E1401 Req.3): one pure function, used by
/// Home, the launch advisor and the alerts alike.
#[must_use]
#[allow(clippy::cast_precision_loss)] // shares and rates for display
pub fn summarize(snapshot: &MachineSnapshot, thresholds: &RoomThresholds) -> RoomSummary {
    let mut reasons = Vec::new();
    if let Some(latest) = snapshot.latest {
        match latest.pressure {
            Some(PressureLevel::Critical) => reasons.push(RoomReason {
                kind: "pressure",
                level: RoomLevel::Pressure,
                words: "macOS reports critical memory pressure".to_string(),
            }),
            Some(PressureLevel::Warning) => reasons.push(RoomReason {
                kind: "pressure",
                level: RoomLevel::Tight,
                words: "macOS reports memory pressure".to_string(),
            }),
            _ => {}
        }
        if latest.memory_total > 0 {
            let free = latest.memory_free();
            let share = free as f64 / latest.memory_total as f64;
            let level = if share < thresholds.memory_pressure_share {
                Some(RoomLevel::Pressure)
            } else if share < thresholds.memory_tight_share {
                Some(RoomLevel::Tight)
            } else {
                None
            };
            if let Some(level) = level {
                reasons.push(RoomReason {
                    kind: "memory",
                    level,
                    words: format!("only {} of memory free", bytes_words(free)),
                });
            }
        }
    }
    if let Some(rate) = swap_growth_mb_per_min(&snapshot.ring) {
        let level = if rate > thresholds.swap_pressure_mb_per_min {
            Some(RoomLevel::Pressure)
        } else if rate > thresholds.swap_tight_mb_per_min {
            Some(RoomLevel::Tight)
        } else {
            None
        };
        if let Some(level) = level {
            reasons.push(RoomReason {
                kind: "swap",
                level,
                words: format!("swap is growing — {rate:.0} MB a minute"),
            });
        }
    }
    if let Some(mean) = sustained_cpu(
        &snapshot.ring,
        thresholds.cpu_busy_pct,
        thresholds.cpu_sustained_ms,
    ) {
        reasons.push(RoomReason {
            kind: "cpu",
            level: RoomLevel::Tight,
            words: format!(
                "the processor has been {mean:.0}% busy for over {} s",
                thresholds.cpu_sustained_ms / 1000
            ),
        });
    }
    match snapshot.thermal {
        Some(ThermalState::Critical) => reasons.push(RoomReason {
            kind: "thermal",
            level: RoomLevel::Pressure,
            words: "the Mac is hot and heavily slowed down".to_string(),
        }),
        Some(ThermalState::Serious) => reasons.push(RoomReason {
            kind: "thermal",
            level: RoomLevel::Tight,
            words: "the Mac is warm and slowing down to cool off".to_string(),
        }),
        _ => {}
    }
    reasons.extend(disk_reasons(&snapshot.volumes, thresholds));
    reasons.sort_by_key(|reason| std::cmp::Reverse(reason.level));
    let level = reasons
        .first()
        .map_or(RoomLevel::Plenty, |reason| reason.level);
    RoomSummary {
        level,
        words: level.words(),
        reasons,
    }
}

/// The disk reasons: a volume in use running out of space.
fn disk_reasons(volumes: &[VolumeSample], thresholds: &RoomThresholds) -> Vec<RoomReason> {
    let mut reasons = Vec::new();
    for volume in volumes {
        if volume.total_bytes == 0 {
            continue;
        }
        let level = if volume.available_bytes < thresholds.disk_pressure_bytes {
            Some(RoomLevel::Pressure)
        } else if volume.available_bytes < thresholds.disk_tight_bytes {
            Some(RoomLevel::Tight)
        } else {
            None
        };
        if let Some(level) = level {
            reasons.push(RoomReason {
                kind: "disk",
                level,
                words: format!(
                    "{} has only {} free",
                    volume.name,
                    bytes_words(volume.available_bytes)
                ),
            });
        }
    }
    reasons
}

/// The OS readings the sampler can't get from `sysinfo` — a seam so tests
/// run without the real OS calls.
pub trait OsReadings: Send {
    /// The kernel's memory-pressure level.
    fn pressure(&self) -> Option<PressureLevel>;
    /// Power state.
    fn power(&self) -> Option<PowerState>;
    /// Thermal state.
    fn thermal(&self) -> Option<ThermalState>;
    /// Memory in use and what it measures, given `sysinfo`'s total and
    /// available. By default (non-macOS): total − available.
    fn memory_in_use(&self, total: u64, available: u64) -> (Option<u64>, MemoryBasis) {
        (
            Some(total.saturating_sub(available)),
            MemoryBasis::TotalMinusAvailable,
        )
    }
}

/// The sysctls behind [`memory_parts`], in its order (all required).
#[cfg(target_os = "macos")]
const MEMORY_SYSCTLS: [&str; 4] = [
    "hw.pagesize",
    "vm.page_pageable_internal_count",
    "vm.page_purgeable_count",
    "vm.page_wired_count",
];

/// macOS: app memory, wired and compressed, as Activity Monitor and
/// `vm_stat` count them: app and wired through sysctl, compressed as the
/// pages the compressor occupies (D-27's `host_statistics64`; the
/// compressed data inside them, `vm.compressor_bytes_used`, read 3 % of
/// physical memory low). `None` if any part can't be read (never a
/// partial sum).
#[cfg(target_os = "macos")]
#[must_use]
pub fn memory_parts() -> Option<MemoryParts> {
    let [page, internal, purgeable, wired] = MEMORY_SYSCTLS.map(sysctl_u64);
    let page = page?;
    Some(MemoryParts {
        app: internal?.saturating_sub(purgeable?).saturating_mul(page),
        wired: wired?.saturating_mul(page),
        compressed: juggler_vm_stats::compressor_pages()?.saturating_mul(page),
    })
}

/// Reads one numeric sysctl (macOS).
#[cfg(target_os = "macos")]
fn sysctl_u64(name: &str) -> Option<u64> {
    use sysctl::Sysctl as _;
    match sysctl::Ctl::new(name).ok()?.value().ok()? {
        sysctl::CtlValue::Int(value) => u64::try_from(value).ok(),
        sysctl::CtlValue::Uint(value) => Some(u64::from(value)),
        sysctl::CtlValue::S64(value) | sysctl::CtlValue::Long(value) => u64::try_from(value).ok(),
        sysctl::CtlValue::U64(value) | sysctl::CtlValue::Ulong(value) => Some(value),
        _ => None,
    }
}

/// The real readings on this OS.
#[derive(Debug, Default, Clone, Copy)]
pub struct HostReadings;

#[cfg(target_os = "macos")]
impl OsReadings for HostReadings {
    fn pressure(&self) -> Option<PressureLevel> {
        use sysctl::Sysctl as _;
        let ctl = sysctl::Ctl::new("kern.memorystatus_vm_pressure_level").ok()?;
        let level = match ctl.value().ok()? {
            sysctl::CtlValue::Int(level) => i64::from(level),
            sysctl::CtlValue::Uint(level) => i64::from(level),
            sysctl::CtlValue::S64(level) => level,
            sysctl::CtlValue::U64(level) => i64::try_from(level).ok()?,
            _ => return None,
        };
        // xnu: 1 normal, 2 warning, 4 critical.
        Some(match level {
            4.. => PressureLevel::Critical,
            2 | 3 => PressureLevel::Warning,
            _ => PressureLevel::Normal,
        })
    }

    fn power(&self) -> Option<PowerState> {
        use objc2_io_kit::{
            IOPSGetBatteryWarningLevel, IOPSGetTimeRemainingEstimate, IOPSLowBatteryWarningLevel,
            kIOPSTimeRemainingUnknown, kIOPSTimeRemainingUnlimited,
        };
        let remaining = IOPSGetTimeRemainingEstimate();
        // Unlimited = drawing from an adapter (or a Mac without a battery).
        #[allow(clippy::float_cmp)] // documented sentinel values
        let on_battery = remaining != kIOPSTimeRemainingUnlimited;
        #[allow(
            clippy::float_cmp,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let minutes_left =
            (on_battery && remaining != kIOPSTimeRemainingUnknown && remaining > 0.0)
                .then(|| (remaining / 60.0).round() as u32);
        let warning = IOPSGetBatteryWarningLevel();
        let low_power_mode = objc2_foundation::NSProcessInfo::processInfo().isLowPowerModeEnabled();
        Some(PowerState {
            on_battery,
            minutes_left,
            battery_low: warning != IOPSLowBatteryWarningLevel::None,
            low_power_mode,
        })
    }

    /// App memory + wired + compressed ([`memory_parts`]). `sysinfo`'s
    /// total − available is far lower on macOS (inactive app pages count
    /// as available). If the reading fails, nothing — not the other
    /// measure under the same name.
    fn memory_in_use(&self, total: u64, _available: u64) -> (Option<u64>, MemoryBasis) {
        memory_parts().map_or((None, MemoryBasis::Unknown), |parts| {
            (
                Some(parts.total().min(total)),
                MemoryBasis::AppWiredCompressed,
            )
        })
    }

    fn thermal(&self) -> Option<ThermalState> {
        use objc2_foundation::{NSProcessInfo, NSProcessInfoThermalState};
        let state = NSProcessInfo::processInfo().thermalState();
        Some(if state == NSProcessInfoThermalState::Critical {
            ThermalState::Critical
        } else if state == NSProcessInfoThermalState::Serious {
            ThermalState::Serious
        } else if state == NSProcessInfoThermalState::Fair {
            ThermalState::Fair
        } else {
            ThermalState::Nominal
        })
    }
}

#[cfg(not(target_os = "macos"))]
impl OsReadings for HostReadings {
    fn pressure(&self) -> Option<PressureLevel> {
        None
    }
    fn power(&self) -> Option<PowerState> {
        None
    }
    fn thermal(&self) -> Option<ThermalState> {
        None
    }
}

/// Samples the machine on the 2 s tick (AJ-E1401 Req.1–2).
pub struct MachineSampler {
    system: sysinfo::System,
    disks: Option<sysinfo::Disks>,
    os: Box<dyn OsReadings>,
    ticks: u32,
    /// The tick the disk list was last rebuilt (an unknown `/Volumes/…`
    /// folder may ask for an early rebuild, at most every 5 minutes).
    last_relist: Option<u32>,
    /// Hands out disk work: a [`DiskWork`] from an older hand-out is
    /// dropped at [`MachineSampler::finish_disks`], so two in flight can
    /// never put stale volumes back (today one tick thread does all of
    /// it; this makes that safe rather than assumed).
    disk_generation: u64,
    /// How many times the disk list was built (tests).
    relists: u32,
    cpu_primed: bool,
    snapshot: MachineSnapshot,
    ring: VecDeque<MachineSample>,
}

impl std::fmt::Debug for MachineSampler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MachineSampler")
            .field("ticks", &self.ticks)
            .field("snapshot", &self.snapshot)
            .finish_non_exhaustive()
    }
}

impl Default for MachineSampler {
    fn default() -> Self {
        Self::with_readings(Box::new(HostReadings))
    }
}

impl MachineSampler {
    /// A sampler on this OS's real readings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A sampler on the given OS readings (tests).
    #[must_use]
    pub fn with_readings(os: Box<dyn OsReadings>) -> Self {
        // The first call into the power and thermal APIs loads their
        // frameworks — hundreds of milliseconds in a large app binary.
        // Pay it here, at construction (app start, outside any lock), so
        // no tick ever holds the sampler's lock over it.
        let _ = os.power();
        let _ = os.thermal();
        let _ = os.pressure();
        let system = crate::new_system();
        let cores = u32::try_from(
            std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        )
        .unwrap_or(1);
        Self {
            system,
            disks: None,
            os,
            ticks: 0,
            last_relist: None,
            disk_generation: 0,
            relists: 0,
            cpu_primed: false,
            snapshot: MachineSnapshot {
                cores,
                ..MachineSnapshot::default()
            },
            ring: VecDeque::with_capacity(MACHINE_RING_CAPACITY),
        }
    }

    /// One tick — the cheap part only (CPU, memory, pressure, swap; power
    /// and thermal on their cadence): microseconds, safe under a lock the
    /// UI also reads. When the disk cadence is due it hands back a
    /// [`DiskWork`]: do it OUTSIDE the lock (listing disks costs 50–150 ms,
    /// and finding the folders in use reads storage), then give the result
    /// to [`MachineSampler::finish_disks`].
    #[must_use]
    pub fn tick(&mut self, now_ms: i64) -> Option<DiskWork> {
        self.ticks = self.ticks.wrapping_add(1);
        let first = self.ticks == 1;
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        let cpu_pct = if self.cpu_primed {
            Some(f64::from(self.system.global_cpu_usage()).clamp(0.0, 100.0))
        } else {
            self.cpu_primed = true;
            None
        };
        let total = self.system.total_memory();
        let (memory_used, memory_basis) =
            self.os.memory_in_use(total, self.system.available_memory());
        let sample = MachineSample {
            at_ms: now_ms,
            cpu_pct,
            memory_total: self.system.total_memory(),
            memory_available: self.system.available_memory(),
            memory_used,
            memory_basis,
            pressure: self.os.pressure(),
            swap_total: self.system.total_swap(),
            swap_used: self.system.used_swap(),
        };
        push_machine_sample(&mut self.ring, sample);
        self.snapshot.latest = Some(sample);
        self.snapshot.ring = self.ring.iter().copied().collect();
        if first || self.ticks.is_multiple_of(POWER_EVERY) {
            self.snapshot.power = self.os.power();
            self.snapshot.thermal = self.os.thermal();
        }
        (first || self.ticks.is_multiple_of(DISKS_EVERY)).then(|| {
            self.disk_generation = self.disk_generation.wrapping_add(1);
            let due = self.disks.is_none() || self.ticks.is_multiple_of(DISK_LIST_EVERY);
            let early_allowed = self
                .last_relist
                .is_none_or(|at| self.ticks.wrapping_sub(at) >= EARLY_RELIST_EVERY);
            DiskWork {
                generation: self.disk_generation,
                disks: self.disks.take(),
                relist: due,
                early_relist_allowed: early_allowed,
                relisted: false,
            }
        })
    }

    /// Puts the disk work back: the (maybe rebuilt) disk list and the
    /// volumes in use.
    /// Work from an older hand-out is dropped (its disk list is kept only
    /// if the sampler has none).
    pub fn finish_disks(&mut self, work: DiskWork, volumes: Vec<VolumeSample>) {
        if work.relisted {
            self.relists = self.relists.saturating_add(1);
        }
        if work.generation != self.disk_generation {
            if self.disks.is_none() {
                self.disks = work.disks;
            }
            return;
        }
        if work.relisted {
            self.last_relist = Some(self.ticks);
        }
        self.disks = work.disks;
        self.snapshot.volumes = volumes;
    }

    /// How many times the disk list has been built (tests).
    #[doc(hidden)]
    #[must_use]
    pub fn relists(&self) -> u32 {
        self.relists
    }

    /// A tick with its disk work done inline (tests, and callers without a
    /// lock to keep short). `paths_in_use` lists the folders whose volumes
    /// matter; it is called only on the disk cadence.
    pub fn tick_with(&mut self, now_ms: i64, paths_in_use: impl FnOnce() -> Vec<PathBuf>) {
        if let Some(mut work) = self.tick(now_ms) {
            let volumes = work.volumes(&paths_in_use());
            self.finish_disks(work, volumes);
        }
    }

    /// What the sampler knows now.
    #[must_use]
    pub fn snapshot(&self) -> &MachineSnapshot {
        &self.snapshot
    }
}

/// An unknown `/Volumes/…` folder may rebuild the disk list early, at most
/// every 150th tick (5 minutes).
const EARLY_RELIST_EVERY: u32 = 150;

/// The disk half of a tick, done outside the sampler's lock (AJ-E1401).
#[derive(Debug)]
pub struct DiskWork {
    generation: u64,
    disks: Option<sysinfo::Disks>,
    relist: bool,
    early_relist_allowed: bool,
    relisted: bool,
}

impl DiskWork {
    /// The volumes holding `paths`, with free space refreshed for those
    /// alone (about a microsecond each). The disk list is rebuilt on its
    /// 10-minute cadence, or early when a folder under `/Volumes/` matches
    /// no listed volume (a disk plugged in since); a folder still unknown
    /// after that is left out, never credited to the startup disk.
    pub fn volumes(&mut self, paths: &[PathBuf]) -> Vec<VolumeSample> {
        if self.relist || self.disks.is_none() {
            self.relist();
        }
        let (_, unknown) = volumes_in_use(self.listed(), paths);
        if unknown && self.early_relist_allowed && !self.relisted {
            self.relist();
        }
        let (in_use, _) = volumes_in_use(self.listed(), paths);
        let refresh = sysinfo::DiskRefreshKind::nothing().with_storage();
        if let Some(disks) = self.disks.as_mut() {
            for disk in disks.list_mut() {
                let mount = disk.mount_point().to_string_lossy();
                if in_use.iter().any(|volume| volume.mount_point == mount) {
                    disk.refresh_specifics(refresh);
                }
            }
        }
        volumes_in_use(self.listed(), paths).0
    }

    fn relist(&mut self) {
        self.disks = Some(sysinfo::Disks::new_with_refreshed_list());
        self.relisted = true;
    }

    fn listed(&self) -> Vec<VolumeSample> {
        self.disks
            .as_ref()
            .map(|disks| disks.list().iter().map(volume_of).collect())
            .unwrap_or_default()
    }
}

fn volume_of(disk: &sysinfo::Disk) -> VolumeSample {
    VolumeSample {
        name: disk.name().to_string_lossy().into_owned(),
        mount_point: disk.mount_point().to_string_lossy().into_owned(),
        total_bytes: disk.total_space(),
        available_bytes: disk.available_space(),
    }
}

/// Appends one sample, dropping the oldest past capacity.
#[doc(hidden)]
pub fn push_machine_sample(ring: &mut VecDeque<MachineSample>, sample: MachineSample) {
    ring.push_back(sample);
    while ring.len() > MACHINE_RING_CAPACITY {
        ring.pop_front();
    }
}

/// Pure: the volumes holding `paths` (longest mount-point prefix wins),
/// each once, in first-use order — and whether some folder under
/// `/Volumes/` matched no volume of its own (only the startup disk's
/// "/"): such a folder is left out rather than credited to the wrong disk
/// (a disk plugged in since the list was built, or a network share
/// `sysinfo` does not list).
#[must_use]
pub fn volumes_in_use(
    volumes: impl IntoIterator<Item = VolumeSample>,
    paths: &[PathBuf],
) -> (Vec<VolumeSample>, bool) {
    let all: Vec<VolumeSample> = volumes.into_iter().collect();
    let mut out: Vec<VolumeSample> = Vec::new();
    let mut unknown = false;
    for path in paths {
        let best = all
            .iter()
            .filter(|volume| path.starts_with(Path::new(&volume.mount_point)))
            .max_by_key(|volume| volume.mount_point.len());
        let mis_credited =
            best.is_none_or(|volume| volume.mount_point == "/") && path.starts_with("/Volumes/");
        if mis_credited {
            unknown = true;
            continue;
        }
        if let Some(volume) = best
            && !out
                .iter()
                .any(|seen| seen.mount_point == volume.mount_point)
        {
            out.push(volume.clone());
        }
    }
    (out, unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    const G: u64 = 1_000_000_000;

    fn sample(at_ms: i64, cpu: Option<f64>, available: u64, swap_used: u64) -> MachineSample {
        MachineSample {
            at_ms,
            cpu_pct: cpu,
            memory_total: 32 * G,
            memory_available: available,
            memory_used: None,
            memory_basis: MemoryBasis::Unknown,
            pressure: Some(PressureLevel::Normal),
            swap_total: 8 * G,
            swap_used,
        }
    }

    fn snapshot_of(ring: Vec<MachineSample>) -> MachineSnapshot {
        MachineSnapshot {
            cores: 10,
            latest: ring.last().copied(),
            ring,
            ..MachineSnapshot::default()
        }
    }

    fn calm_ring() -> Vec<MachineSample> {
        (0..60)
            .map(|i| sample(i * 2_000, Some(20.0), 12 * G, 22 * G))
            .collect()
    }

    #[test]
    fn a_calm_mac_has_plenty_of_room_even_with_lots_of_old_swap() {
        let summary = summarize(&snapshot_of(calm_ring()), &RoomThresholds::default());
        assert_eq!(summary.level, RoomLevel::Plenty);
        assert_eq!(summary.words, "Plenty of room");
        assert!(summary.reasons.is_empty());
    }

    #[test]
    fn the_kernels_pressure_level_alone_decides_tight_or_pressure() {
        for (level, expect) in [
            (PressureLevel::Warning, RoomLevel::Tight),
            (PressureLevel::Critical, RoomLevel::Pressure),
        ] {
            let mut ring = calm_ring();
            ring.last_mut().unwrap().pressure = Some(level);
            let summary = summarize(&snapshot_of(ring), &RoomThresholds::default());
            assert_eq!(summary.level, expect);
            assert_eq!(summary.reasons[0].kind, "pressure");
        }
    }

    #[test]
    fn low_available_memory_is_tight_then_pressure() {
        let mut ring = calm_ring();
        ring.last_mut().unwrap().memory_available = 2 * G; // 6 %
        let summary = summarize(&snapshot_of(ring.clone()), &RoomThresholds::default());
        assert_eq!(summary.level, RoomLevel::Tight);
        assert_eq!(summary.reasons[0].words, "only 2.0 GB of memory free");
        ring.last_mut().unwrap().memory_available = G; // 3 %
        let summary = summarize(&snapshot_of(ring), &RoomThresholds::default());
        assert_eq!(summary.level, RoomLevel::Pressure);
    }

    #[test]
    fn growing_swap_is_the_thrash_signal_not_swap_used() {
        // 60 s of swap climbing 4 MB per 2 s sample = 120 MB a minute.
        let ring: Vec<MachineSample> = (0..31)
            .map(|i| {
                sample(
                    i * 2_000,
                    Some(30.0),
                    12 * G,
                    4 * G + (i as u64) * 4_000_000,
                )
            })
            .collect();
        let summary = summarize(&snapshot_of(ring), &RoomThresholds::default());
        assert_eq!(summary.level, RoomLevel::Tight);
        assert_eq!(summary.reasons[0].kind, "swap");
        assert_eq!(
            summary.reasons[0].words,
            "swap is growing — 120 MB a minute"
        );
    }

    #[test]
    fn swap_growth_needs_thirty_seconds_of_history() {
        let ring: Vec<MachineSample> = (0..10)
            .map(|i| sample(i * 2_000, Some(30.0), 12 * G, (i as u64) * 100_000_000))
            .collect();
        assert_eq!(swap_growth_mb_per_min(&ring), None);
    }

    #[test]
    fn busy_cpu_counts_only_when_sustained_for_the_whole_window() {
        let busy: Vec<MachineSample> = (0..31)
            .map(|i| sample(i * 2_000, Some(95.0), 12 * G, 0))
            .collect();
        let summary = summarize(&snapshot_of(busy.clone()), &RoomThresholds::default());
        assert_eq!(summary.level, RoomLevel::Tight);
        assert_eq!(summary.reasons[0].kind, "cpu");
        // One quiet sample inside the window: a spike, not sustained.
        let mut spiky = busy;
        spiky[20].cpu_pct = Some(40.0);
        assert_eq!(
            summarize(&snapshot_of(spiky), &RoomThresholds::default()).level,
            RoomLevel::Plenty
        );
        // Too little history to call it sustained.
        let short: Vec<MachineSample> = (0..5)
            .map(|i| sample(i * 2_000, Some(99.0), 12 * G, 0))
            .collect();
        assert_eq!(
            summarize(&snapshot_of(short), &RoomThresholds::default()).level,
            RoomLevel::Plenty
        );
    }

    #[test]
    fn an_unknown_first_cpu_sample_never_counts_as_busy() {
        let mut ring: Vec<MachineSample> = (0..31)
            .map(|i| sample(i * 2_000, Some(95.0), 12 * G, 0))
            .collect();
        ring[0].cpu_pct = None;
        assert_eq!(
            sustained_cpu(&ring, 90.0, 60_000),
            None,
            "an unmeasured sample is not a busy one"
        );
    }

    #[test]
    fn thermal_and_disk_reasons_and_worst_first_order() {
        let mut snapshot = snapshot_of(calm_ring());
        snapshot.thermal = Some(ThermalState::Serious);
        snapshot.volumes = vec![VolumeSample {
            name: "Macintosh HD".into(),
            mount_point: "/".into(),
            total_bytes: 1_000 * G,
            available_bytes: G,
        }];
        let summary = summarize(&snapshot, &RoomThresholds::default());
        assert_eq!(summary.level, RoomLevel::Pressure);
        assert_eq!(summary.reasons[0].kind, "disk");
        assert_eq!(
            summary.reasons[0].words,
            "Macintosh HD has only 1.0 GB free"
        );
        assert_eq!(summary.reasons[1].kind, "thermal");
        // Fair/Nominal are fine.
        snapshot.thermal = Some(ThermalState::Fair);
        snapshot.volumes.clear();
        assert_eq!(
            summarize(&snapshot, &RoomThresholds::default()).level,
            RoomLevel::Plenty
        );
    }

    #[test]
    fn disk_room_is_absolute_not_a_share() {
        let mut snapshot = snapshot_of(calm_ring());
        // 5 % of a 1 TB disk is still plenty of room.
        snapshot.volumes = vec![VolumeSample {
            name: "Macintosh HD".into(),
            mount_point: "/".into(),
            total_bytes: 1_000 * G,
            available_bytes: 50 * G,
        }];
        let summary = summarize(&snapshot, &RoomThresholds::default());
        assert_eq!(summary.level, RoomLevel::Plenty);
        snapshot.volumes[0].available_bytes = 15 * G;
        let summary = summarize(&snapshot, &RoomThresholds::default());
        assert_eq!(summary.level, RoomLevel::Tight);
        assert_eq!(summary.reasons[0].words, "Macintosh HD has only 15 GB free");
    }

    #[test]
    fn missing_values_are_never_reasons() {
        let snapshot = MachineSnapshot::default();
        let summary = summarize(&snapshot, &RoomThresholds::default());
        assert_eq!(summary.level, RoomLevel::Plenty);
    }

    #[test]
    fn the_ring_keeps_five_minutes() {
        let mut ring = VecDeque::new();
        for i in 0..400 {
            push_machine_sample(&mut ring, sample(i * 2_000, None, 0, 0));
        }
        assert_eq!(ring.len(), MACHINE_RING_CAPACITY);
        assert_eq!(ring.front().unwrap().at_ms, 250 * 2_000, "oldest dropped");
    }

    #[test]
    fn volumes_in_use_pick_the_longest_mount_and_dedupe() {
        let volumes = vec![
            VolumeSample {
                name: "Macintosh HD".into(),
                mount_point: "/".into(),
                total_bytes: 1,
                available_bytes: 1,
            },
            VolumeSample {
                name: "Work".into(),
                mount_point: "/Volumes/Work".into(),
                total_bytes: 1,
                available_bytes: 1,
            },
        ];
        let used = volumes_in_use(
            volumes,
            &[
                PathBuf::from("/Users/you/Library/Application Support/ai-juggler"),
                PathBuf::from("/Volumes/Work/repo"),
                PathBuf::from("/Users/you/code"),
            ],
        );
        let names: Vec<&str> = used.0.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["Macintosh HD", "Work"]);
        assert!(!used.1, "every folder found its volume");
    }

    #[test]
    fn an_unlisted_disk_is_never_credited_to_the_startup_disk() {
        // A disk plugged in (or a share mounted) since the list was built:
        // /Volumes/Fresh matches only "/" — it must not count as
        // Macintosh HD, and the caller is told to look again.
        let volumes = vec![VolumeSample {
            name: "Macintosh HD".into(),
            mount_point: "/".into(),
            total_bytes: 1_000 * G,
            available_bytes: 300 * G,
        }];
        let (used, unknown) = volumes_in_use(
            volumes,
            &[
                PathBuf::from("/Users/you/code"),
                PathBuf::from("/Volumes/Fresh/repo"),
            ],
        );
        assert_eq!(used.len(), 1);
        assert_eq!(used[0].name, "Macintosh HD");
        assert!(unknown);
    }

    #[test]
    fn free_memory_is_on_the_same_basis_as_used() {
        let mut ring = calm_ring();
        let last = ring.last_mut().unwrap();
        last.memory_available = 12 * G; // the OS's figure
        last.memory_used = Some(30 * G); // app + wired + compressed
        last.memory_basis = MemoryBasis::AppWiredCompressed;
        assert_eq!(last.memory_free(), 2 * G, "used + free = total");
        let summary = summarize(&snapshot_of(ring), &RoomThresholds::default());
        assert_eq!(summary.level, RoomLevel::Tight);
        assert_eq!(summary.reasons[0].words, "only 2.0 GB of memory free");
    }

    #[test]
    fn stale_disk_work_never_overwrites_newer_volumes() {
        let mut sampler = MachineSampler::with_readings(Box::new(HostReadings));
        let old = sampler.tick(0).expect("the first tick does disks");
        // A second hand-out before the first came back (15 ticks later).
        let mut newer = None;
        for i in 1..=15 {
            if let Some(work) = sampler.tick(i * 2_000) {
                newer = Some(work);
            }
        }
        let newer = newer.expect("the disk cadence handed out again");
        let fresh = vec![VolumeSample {
            name: "Fresh".into(),
            mount_point: "/".into(),
            total_bytes: 1,
            available_bytes: 1,
        }];
        sampler.finish_disks(newer, fresh.clone());
        sampler.finish_disks(old, Vec::new());
        assert_eq!(
            sampler.snapshot().volumes,
            fresh,
            "the stale result was dropped"
        );
    }

    #[test]
    fn words_for_bytes() {
        assert_eq!(bytes_words(3_200_000_000), "3.2 GB");
        assert_eq!(bytes_words(24_000_000_000), "24 GB");
        assert_eq!(bytes_words(640_000_000), "640 MB");
        assert_eq!(bytes_words(123_456), "under 1 MB");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn host_readings_answer_on_macos() {
        let host = HostReadings;
        assert!(
            host.pressure().is_some(),
            "kern.memorystatus_vm_pressure_level"
        );
        assert!(host.thermal().is_some());
        // The parts add up, and stay within the Mac's memory — or, where
        // this macOS doesn't report one of them (GitHub's macOS runner
        // didn't), nothing is shown: never a partial sum, never another
        // measure under the same name. The log names what's missing.
        let total = {
            let mut system = sysinfo::System::new();
            system.refresh_memory();
            system.total_memory()
        };
        let (used, basis) = host.memory_in_use(total, 0);
        if let Some(parts) = memory_parts() {
            assert!(parts.app > 0 && parts.wired > 0, "{parts:?}");
            assert!(parts.app <= total && parts.wired <= total && parts.compressed <= total);
            assert_eq!(basis, MemoryBasis::AppWiredCompressed);
            let used = used.expect("read");
            assert!(used <= total, "used {used} ≤ total {total}");
            // Read a moment apart, so within 2 % of the parts' sum.
            let sum = parts.total().min(total);
            assert!(
                used.abs_diff(sum) <= total / 50,
                "used {used} vs parts {sum}"
            );
        } else {
            let missing: Vec<&str> = MEMORY_SYSCTLS
                .iter()
                .copied()
                .filter(|name| sysctl_u64(name).is_none())
                .collect();
            eprintln!("memory parts unavailable on this macOS; missing: {missing:?}");
            assert!(!missing.is_empty(), "None only when a sysctl is missing");
            assert_eq!(used, None, "nothing shown rather than another measure");
            assert_eq!(basis, MemoryBasis::Unknown);
        }
        assert!(host.power().is_some());
    }
}
