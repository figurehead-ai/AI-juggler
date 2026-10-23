//! E14 machine health: how the Mac is doing, for Home's This Mac panel,
//! the launch advisor and the overload alerts (PROPOSAL §6.5, SPEC §5.3).
//!
//! AJ-E1401: the machine sampler runs on the 2 s processes tick; this
//! module turns its snapshot into the read model the UI shows, with the
//! one shared summary ("Plenty of room" / "Getting tight" / "Under
//! pressure") and its reasons in words.
//!
//! AJ-E1402: the AI tools AI Juggler did not start (the Claude app,
//! `claude` in a terminal) are found every 30 s and measured read-only
//! over their whole tree; they count in the totals, labelled as not ours,
//! and have no stop or cleanup path of any kind (D-08).
//!
//! Passive only: nothing here stops, pauses or launches anything.

// Tauri command signatures take State/values by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

pub use crate::machine_recommendations::recommendations;
pub use crate::machine_tools::{ToolGroupInfo, ToolTotalsInfo};
use crate::machine_tools::{by_tool, totals};
use std::path::PathBuf;
use std::sync::Arc;

use juggler_supervisor::machine::{
    MachineSampler, MachineSnapshot, MemoryBasis, PowerState, PressureLevel, RoomLevel,
    RoomThresholds, ThermalState, bytes_words, summarize, swap_growth_mb_per_min,
};
use juggler_supervisor::{LiveProcessStats, TreeSample};

use crate::running_setups::RunningTool;

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

/// The machine watch behind `AppState::machine`: the sampler plus the AI
/// tools AI Juggler did not start (AJ-E1402), re-read every 30 s.
pub struct MachineWatch {
    /// The machine sampler (AJ-E1401).
    pub sampler: MachineSampler,
    ticks: u32,
    tools: Vec<RunningTool>,
    /// Running peaks per session / observed tree (AJ-E1406), written as
    /// footprints every 10 min and when the tree ends.
    peaks: std::collections::HashMap<String, TreePeaks>,
    /// Overload alerts (AJ-E1408).
    alerts: crate::machine_alerts::AlertEngine,
    /// Whether a running-tools scan has read a process table yet.
    tools_scanned: bool,
    /// Bumped whenever footprints are written or pruned.
    pub(crate) footprints_written: u64,
    /// What This Mac's recommendations read from storage (AJ-E1409),
    /// re-read only when footprints changed or after a minute (names).
    pub(crate) advice_inputs: Option<crate::machine_recommendations::AdviceInputs>,
}

impl MachineWatch {
    /// How many 2 s ticks have started (read-only: lets a test know a tick
    /// is past its locked part without timing guesses).
    #[doc(hidden)]
    #[must_use]
    pub fn ticks(&self) -> u32 {
        self.ticks
    }
}

impl Default for MachineWatch {
    fn default() -> Self {
        Self {
            sampler: MachineSampler::new(),
            ticks: 0,
            tools: Vec::new(),
            peaks: std::collections::HashMap::new(),
            alerts: crate::machine_alerts::AlertEngine::default(),
            tools_scanned: false,
            footprints_written: 0,
            advice_inputs: None,
        }
    }
}

impl std::fmt::Debug for MachineWatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MachineWatch")
            .field("ticks", &self.ticks)
            .field("tools", &self.tools.len())
            .finish_non_exhaustive()
    }
}

/// The running tools are re-read every 15th tick (30 s): the scan reads the
/// collector's process table and, for the few roots it finds, one
/// environment variable (`CLAUDE_CONFIG_DIR`) — cheap, but no need to run
/// it every tick.
const TOOLS_EVERY: u32 = 15;

/// The 2 s tick's machine half (AJ-E1401 Req.1, AJ-E1402 Req.1–2). The
/// watch's lock is held only for the cheap part (microseconds); the disk
/// half — the folders in use (a storage read) and, every 10 minutes,
/// listing the disks (50–150 ms) — and the running-tools scan run outside
/// it, so a reader of the lock never waits on either.
pub fn tick(state: &Arc<AppState>, now_ms: i64) {
    let (work, ticks, scan) = {
        let Ok(mut watch) = state.machine.lock() else {
            return;
        };
        watch.ticks = watch.ticks.wrapping_add(1);
        let work = watch.sampler.tick(now_ms);
        // Until a scan has found a process table to read, every tick tries
        // (the collector may not have its first table yet).
        let scan = !watch.tools_scanned || watch.ticks.is_multiple_of(TOOLS_EVERY);
        (work, watch.ticks, scan)
    };
    if let Some(mut work) = work {
        let volumes = work.volumes(&paths_in_use(state));
        if let Ok(mut watch) = state.machine.lock() {
            watch.sampler.finish_disks(work, volumes);
        }
    }
    track_footprints(
        state,
        now_ms,
        ticks.is_multiple_of(FOOTPRINT_EVERY),
        ticks == 1 || ticks.is_multiple_of(FOOTPRINT_EVERY),
    );
    if !scan {
        return;
    }
    // Classified from the collector's own table (no second full refresh);
    // only the roots' `CLAUDE_CONFIG_DIR` is read.
    let briefs = state
        .resources
        .lock()
        .map(|collector| collector.process_briefs())
        .unwrap_or_default();
    if briefs.is_empty() {
        return; // no table yet: try again next tick
    }
    // A `claude` AI Juggler started is ours even after its session ended
    // and launchd adopted it: a pid with a ledger row is never "a tool AI
    // Juggler didn't start" (it's a leftover, in the leftovers count).
    let ours: std::collections::HashSet<u32> = crate::commands::local_ledger_rows(state)
        .into_iter()
        .filter_map(|row| row.pid)
        .collect();
    let tools = not_ours(
        dirs::home_dir().map_or_else(Vec::new, |home| {
            crate::running_setups::tools_from(&briefs, &home)
        }),
        &ours,
    );
    let roots: Vec<juggler_supervisor::ObservedRoot> = tools
        .iter()
        .map(|tool| juggler_supervisor::ObservedRoot {
            pid: tool.pid,
            start_time: tool.start_time,
        })
        .collect();
    if let Ok(mut watch) = state.machine.lock() {
        watch.tools = tools;
        watch.tools_scanned = true;
    }
    if let Ok(mut collector) = state.resources.lock() {
        collector.set_observed_roots(roots);
    }
}

// --- AJ-E1407: the machine launch advisor --------------------------------

/// Before 3 footprints exist, a launch is assumed to take this much.
const UNMEASURED_PROJECTION: u64 = 1_000_000_000;
/// Projections need this many footprints of the launch setting.
pub(crate) const MIN_FOOTPRINTS: u32 = 3;

/// "Can this Mac take another session?" for one launch setting (AJ-E1407).
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MachineAdviceInfo {
    /// The launch setting.
    pub recipe_id: String,
    /// `ok` | `tight` | `no`.
    pub verdict: String,
    /// The verdict in words ("This Mac can take another session").
    pub words: String,
    /// Why, in words; empty for a plain OK.
    pub reasons: Vec<String>,
    /// What a session of this setting is expected to take, bytes.
    #[ts(type = "number")]
    pub projected_bytes: u64,
    /// How many of this setting's sessions the projection learned from;
    /// 0 = not measured yet (the default is used).
    pub based_on: u32,
    /// The heaviest running session, when closing it would help.
    pub relief_session_id: Option<String>,
    /// Memory that closing it would free, bytes.
    #[ts(type = "number | null")]
    pub relief_bytes: Option<u64>,
}

/// Pure (AJ-E1407 Req.1–2): the verdict for one setting on this Mac.
#[must_use]
#[allow(clippy::cast_precision_loss)] // shares for display
pub fn advise(
    machine: &MachineHealthInfo,
    recipe_id: &str,
    footprint: Option<&juggler_core::storage::FootprintStats>,
    heaviest: Option<(&str, u64)>,
) -> MachineAdviceInfo {
    let measured = footprint.filter(|stats| stats.count >= MIN_FOOTPRINTS);
    let projected = measured.map_or(UNMEASURED_PROJECTION, |stats| stats.median_peak_rss);
    let based_on = measured.map_or(0, |stats| stats.count);
    let total = machine.memory_total.max(1) as f64;
    let after = machine.memory_available.saturating_sub(projected);
    let after_share = after as f64 / total;
    let mut no = Vec::new();
    let mut tight = Vec::new();
    match machine.pressure.as_deref() {
        Some("critical") => no.push("macOS reports critical memory pressure".to_string()),
        Some("warning") => tight.push("macOS reports memory pressure".to_string()),
        _ => {}
    }
    if machine.sampled && machine.memory_total > 0 {
        let left = format!("it would leave about {} of memory free", bytes_words(after));
        if after_share < 0.05 {
            no.push(left);
        } else if after_share < 0.15 {
            tight.push(left);
        }
    }
    for reason in &machine.summary.reasons {
        if matches!(reason.kind.as_str(), "cpu" | "thermal" | "swap") {
            if reason.level == "pressure" {
                no.push(reason.words.clone());
            } else {
                tight.push(reason.words.clone());
            }
        }
    }
    let (verdict, words, reasons) = if !no.is_empty() {
        ("no", "Not now — this Mac is short of room", no)
    } else if !tight.is_empty() {
        ("tight", "Tight — this Mac can take it, just", tight)
    } else {
        ("ok", "This Mac can take another session", Vec::new())
    };
    let relief = (verdict != "ok")
        .then_some(heaviest)
        .flatten()
        .filter(|(_, bytes)| *bytes >= 500_000_000);
    MachineAdviceInfo {
        recipe_id: recipe_id.to_string(),
        verdict: verdict.to_string(),
        words: words.to_string(),
        reasons,
        projected_bytes: projected,
        based_on,
        relief_session_id: relief.map(|(id, _)| id.to_string()),
        relief_bytes: relief.map(|(_, bytes)| bytes),
    }
}

/// The advice for every launch setting (AJ-E1407).
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when a lock is poisoned; storage errors.
pub fn machine_advice_impl(state: &Arc<AppState>) -> AppResult<Vec<MachineAdviceInfo>> {
    let mac = this_mac_impl(state, crate::commands::now_millis())?;
    let (recipes, footprints) = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        let mut recipes = Vec::new();
        for account in storage.accounts().list()? {
            recipes.extend(storage.profiles().list_by_account(&account.id)?);
        }
        (recipes, storage.footprints().stats()?)
    };
    let heaviest = mac
        .top
        .iter()
        .find_map(|tree| tree.session_id.as_deref().map(|id| (id, tree.rss_bytes)));
    Ok(recipes
        .iter()
        .map(|recipe| {
            let key = format!("recipe:{}", recipe.id);
            advise(
                &mac.machine,
                recipe.id.as_str(),
                footprints.iter().find(|stats| stats.key == key),
                heaviest,
            )
        })
        .collect())
}

/// "Can this Mac take another session?" per launch setting (AJ-E1407).
/// Warns; never blocks a launch.
///
/// # Errors
///
/// As [`machine_advice_impl`].
#[tauri::command]
pub async fn machine_advice(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<MachineAdviceInfo>> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || machine_advice_impl(&state))
        .await
        .map_err(|error| AppError::Io {
            context: "read the launch advice",
            message: error.to_string(),
        })?
}

// --- AJ-E1406: footprints -------------------------------------------------

/// Footprints are written every 300th tick (10 min) while a tree runs, so
/// a crash still leaves one, and once more when it ends.
const FOOTPRINT_EVERY: u32 = 300;
/// CPU samples kept per tree for its p95: the last hour of 2 s samples.
const CPU_SAMPLES: usize = 1_800;

/// One tree's running peaks.
#[derive(Debug, Clone)]
struct TreePeaks {
    key: String,
    provider: String,
    account_id: Option<String>,
    first_ms: i64,
    last_ms: i64,
    max_rss: u64,
    max_helpers: u32,
    cpu: std::collections::VecDeque<f64>,
}

impl TreePeaks {
    fn new(key: String, provider: String, account_id: Option<String>, now_ms: i64) -> Self {
        Self {
            key,
            provider,
            account_id,
            first_ms: now_ms,
            last_ms: now_ms,
            max_rss: 0,
            max_helpers: 0,
            cpu: std::collections::VecDeque::new(),
        }
    }

    fn observe(&mut self, stats: &LiveProcessStats, now_ms: i64) {
        self.last_ms = now_ms;
        self.max_rss = self.max_rss.max(stats.rss_bytes);
        self.max_helpers = self.max_helpers.max(stats.child_count);
        if let Some(cpu) = stats.cpu_pct {
            self.cpu.push_back(cpu);
            while self.cpu.len() > CPU_SAMPLES {
                self.cpu.pop_front();
            }
        }
    }

    fn footprint(&self, id: &str) -> juggler_core::storage::Footprint {
        juggler_core::storage::Footprint {
            key: self.key.clone(),
            session_id: id.to_string(),
            provider: self.provider.clone(),
            account_id: self.account_id.clone(),
            peak_rss: self.max_rss,
            p95_cpu: p95(&self.cpu),
            peak_helpers: self.max_helpers,
            duration_ms: self.last_ms - self.first_ms,
            recorded_at: self.last_ms,
        }
    }
}

/// Pure: the 95th percentile (nearest rank) of a set of samples.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn p95(samples: &std::collections::VecDeque<f64>) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted: Vec<f64> = samples.iter().copied().collect();
    sorted.sort_by(f64::total_cmp);
    let rank = ((sorted.len() as f64) * 0.95).ceil() as usize;
    sorted.get(rank.saturating_sub(1)).copied()
}

/// Peaks for sessions seen for the first time, keyed by their launch
/// setting (one storage read each, once per session).
fn peaks_for_new_sessions(
    state: &Arc<AppState>,
    live: &std::collections::HashMap<String, LiveProcessStats>,
    now_ms: i64,
) -> Vec<(String, TreePeaks)> {
    let unknown: Vec<String> = state.machine.lock().map_or_else(
        |_| Vec::new(),
        |watch| {
            live.keys()
                .filter(|id| !watch.peaks.contains_key(*id))
                .cloned()
                .collect()
        },
    );
    if unknown.is_empty() {
        return Vec::new();
    }
    let contexts: std::collections::HashMap<String, juggler_supervisor::tree::SpawnContext> = state
        .supervisor
        .spawn_contexts_snapshot()
        .into_iter()
        .map(|(session, ctx)| (session.as_str().to_string(), ctx))
        .collect();
    let Ok(storage) = state.storage.lock() else {
        return Vec::new();
    };
    unknown
        .into_iter()
        .map(|id| {
            let recipe = storage
                .sessions()
                .get(&juggler_core::model::SessionId::new(id.as_str()))
                .map_or_else(
                    |_| "unknown".to_string(),
                    |record| record.profile.to_string(),
                );
            let ctx = contexts.get(&id);
            let peaks = TreePeaks::new(
                format!("recipe:{recipe}"),
                ctx.map_or_else(|| "unknown".to_string(), |ctx| ctx.provider.clone()),
                ctx.and_then(|ctx| ctx.account_id.clone()),
                now_ms,
            );
            (id, peaks)
        })
        .collect()
}

/// Every tick: fold the live trees into their peaks; write footprints for
/// trees that ended (and, every 10 min, for every running one). At start
/// and every 10 min, a deleted launch setting's or Account's footprints
/// are forgotten.
fn track_footprints(state: &Arc<AppState>, now_ms: i64, periodic: bool, prune: bool) {
    let Ok((sessions, observed)) = state
        .resources
        .lock()
        .map(|collector| (collector.stats_snapshot(), collector.observed_snapshot()))
    else {
        return;
    };
    let live: std::collections::HashMap<String, LiveProcessStats> = sessions
        .into_iter()
        .filter(|(_, tree)| tree.rss_bytes > 0)
        .map(|(session, tree)| (session.as_str().to_string(), tree))
        .collect();
    let fresh = peaks_for_new_sessions(state, &live, now_ms);
    let mut writes = Vec::new();
    {
        let Ok(mut watch) = state.machine.lock() else {
            return;
        };
        for (id, peak) in fresh {
            watch.peaks.insert(id, peak);
        }
        let places: std::collections::HashMap<u32, &'static str> = watch
            .tools
            .iter()
            .map(|tool| (tool.pid, tool.place))
            .collect();
        for (root, tree) in &observed {
            let id = format!("observed:{root}");
            let place = places.get(root).copied().unwrap_or("desktop");
            watch
                .peaks
                .entry(id)
                .or_insert_with(|| {
                    TreePeaks::new(
                        format!("observed:{place}"),
                        OBSERVED_TOOL.into(),
                        None,
                        now_ms,
                    )
                })
                .observe(&tree.stats, now_ms);
        }
        for (id, tree) in &live {
            if let Some(peaks) = watch.peaks.get_mut(id) {
                peaks.observe(tree, now_ms);
            }
        }
        // Ended: written once more, then forgotten.
        let ended: Vec<String> = watch
            .peaks
            .keys()
            .filter(|id| match id.strip_prefix("observed:") {
                Some(root) => root
                    .parse::<u32>()
                    .map_or(true, |root| !observed.contains_key(&root)),
                None => !live.contains_key(*id),
            })
            .cloned()
            .collect();
        for id in ended {
            if let Some(peak) = watch.peaks.remove(&id) {
                writes.push(footprint_of(&id, &peak));
            }
        }
        if periodic {
            writes.extend(watch.peaks.iter().map(|(id, peak)| footprint_of(id, peak)));
        }
    }
    if writes.is_empty() && !prune {
        return;
    }
    if let Ok(storage) = state.storage.lock() {
        for footprint in writes.iter().filter(|footprint| footprint.peak_rss > 0) {
            let _ = storage.footprints().record(footprint);
        }
        if prune {
            let _ = storage.footprints().forget_deleted();
        }
    }
    if let Ok(mut watch) = state.machine.lock() {
        watch.footprints_written = watch.footprints_written.wrapping_add(1);
    }
}

/// An observed tree's footprint id carries when it was first seen, so a
/// reused pid never merges two different trees.
fn footprint_of(id: &str, peak: &TreePeaks) -> juggler_core::storage::Footprint {
    if id.starts_with("observed:") {
        peak.footprint(&format!("{id}@{}", peak.first_ms))
    } else {
        peak.footprint(id)
    }
}

/// Pure: the running tools minus any pid AI Juggler has a ledger row for.
#[must_use]
pub fn not_ours<S: std::hash::BuildHasher>(
    tools: Vec<RunningTool>,
    ours: &std::collections::HashSet<u32, S>,
) -> Vec<RunningTool> {
    tools
        .into_iter()
        .filter(|tool| !ours.contains(&tool.pid))
        .collect()
}

/// The read model now.
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when the watch's lock is poisoned.
pub fn machine_health_impl(state: &Arc<AppState>) -> AppResult<MachineHealthInfo> {
    let watch = state.machine.lock().map_err(|_| AppError::StatePoisoned)?;
    Ok(health_of(
        watch.sampler.snapshot(),
        &RoomThresholds::default(),
    ))
}

/// How the Mac is doing (AJ-E1401 Req.4). Off the main thread: it takes a
/// lock the sampling tick also takes.
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when the watch's lock is poisoned.
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

// --- AJ-E1403: the This Mac read model ------------------------------------

/// A session is heavy above this (percent of one core, the AJ-E512 rule)…
const HEAVY_CPU_PCT: f64 = 90.0;
/// … or above this much memory …
const HEAVY_RSS_BYTES: u64 = 2_000_000_000;
/// … for every sample of the last minute.
const HEAVY_WINDOW_MS: i64 = 60_000;
/// A heavy tree turns calm only below this share of the level that made it
/// heavy …
const HEAVY_EXIT_RATIO: f64 = 0.8;
/// … held this long, so a session hovering near the threshold doesn't
/// leave Needs you, the rail and the header and come back a minute later.
const HEAVY_EXIT_MS: i64 = 45_000;

/// The tool the observed trees are: `running_setups` finds the Claude app
/// and `claude` outside it, nothing else yet.
pub(crate) const OBSERVED_TOOL: &str = "claude-code";

/// One process tree on This Mac: a session AI Juggler started, or an AI
/// tool it did not (read-only; no actions).
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ToolTreeInfo {
    /// The tree's root process: what identifies the row while it lives.
    /// `null` for a group of trees.
    pub root_pid: Option<u32>,
    /// The session id for a session AI Juggler started; `null` for a tool
    /// it did not start.
    pub session_id: Option<String>,
    /// For a tool AI Juggler did not start: `desktop` (the Claude app) or
    /// `terminal` (`claude` in a terminal).
    pub place: Option<String>,
    /// The tool's id, for its mark (AJ-E1526); `null` when it isn't known.
    pub tool: Option<String>,
    /// The `by_tool` group it is listed under (that group's `tool`, with
    /// `ours` as `session_id` says): the This Mac page lists each group's
    /// trees by it (AJ-E1528).
    pub group: String,
    /// The Account it uses, when one of yours matches.
    pub account_label: Option<String>,
    /// CPU summed over the tree, percent of ONE core (can pass 100);
    /// `null` until measured.
    pub cpu_pct: Option<f64>,
    /// Memory summed over the tree, bytes.
    #[ts(type = "number")]
    pub rss_bytes: u64,
    /// Helper processes (the tree minus its root).
    pub helpers: u32,
    /// "CPU 95%", "3.2 GB" — only when the tree is heavy.
    pub heavy_note: Option<String>,
    /// The memory is an estimate: helper RSS summed, so memory those
    /// helpers share is counted once per helper (a tool AI Juggler didn't
    /// start, whose exact footprint needs an unsafe OS call). Show it as
    /// "about".
    pub memory_estimate: bool,
}

/// Home's This Mac panel (AJ-E1403).
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ThisMacInfo {
    /// How the Mac is doing.
    pub machine: MachineHealthInfo,
    /// Every AI tool together.
    pub tools: ToolTotalsInfo,
    /// The sessions AI Juggler started.
    pub ours: ToolTotalsInfo,
    /// The AI tools it did not start (the Claude app, `claude` in a
    /// terminal).
    pub theirs: ToolTotalsInfo,
    /// Each AI tool on its own (the menu bar's per-tool lines): our
    /// sessions by Provider, heaviest first, then the Claude app, then
    /// `claude` in a terminal. Only tools that are running.
    pub by_tool: Vec<ToolGroupInfo>,
    /// The heaviest trees by memory, at most three.
    pub top: Vec<ToolTreeInfo>,
    /// Every other tree, heaviest first: shown behind "and N more", never
    /// dropped (QA, P6: 3 of 6 Claude app windows, and no word of the rest).
    pub others: Vec<ToolTreeInfo>,
    /// Every heavy tree (the rail's notes, Home's Needs you).
    pub heavy: Vec<ToolTreeInfo>,
    /// Processes left running after their session closed.
    pub leftover_processes: u32,
    /// Ports those leftovers hold.
    pub leftover_ports: u32,
    /// Overload alerts active now (AJ-E1408; Home's Needs you).
    pub alerts: Vec<crate::machine_alerts::OverloadAlertInfo>,
    /// Advice from this Mac's own history (AJ-E1409); empty until there
    /// is enough of it.
    pub recommendations: Vec<String>,
    /// AJ-E1509: what keeps the Mac awake for the sessions, and why.
    pub awake: crate::features::app::awake::AwakeInfo,
    /// AJ-E1509: AI Juggler itself (with its watchdog; opens at login).
    pub own: crate::features::app::own::OwnInfo,
}

/// One tree, as the pure assembly sees it.
#[derive(Clone, Debug, Default)]
pub struct TreeInput {
    /// The tree's root process, when known.
    pub root_pid: Option<u32>,
    /// Session id (ours) or `None` (observed).
    pub session_id: Option<String>,
    /// `desktop` | `terminal` for observed trees.
    pub place: Option<&'static str>,
    /// The tool's id, when known.
    pub tool: Option<String>,
    /// The matched Account's label.
    pub account_label: Option<String>,
    /// Tree sums.
    pub stats: LiveProcessStats,
    /// The tree's 2 s ring.
    pub ring: Vec<TreeSample>,
    /// AJ-E512 hog rules that fired for it.
    pub hog_rules: Vec<String>,
}

/// Heavy with hysteresis, from the ring alone. A tree turns heavy once a
/// run of samples passes `over` for (nearly) a minute, and stays heavy
/// until the samples since have passed `calm` for (nearly) `HEAVY_EXIT_MS`
/// — or the ring (5 minutes) no longer holds the minute that made it heavy.
fn heavy_with_hysteresis(
    ring: &[TreeSample],
    over: impl Fn(&TreeSample) -> bool,
    calm: impl Fn(&TreeSample) -> bool,
) -> bool {
    // The newest sample that ends a minute-long run over the level.
    let mut run_from: Option<i64> = None;
    let mut entered: Option<usize> = None;
    for (index, sample) in ring.iter().enumerate() {
        if over(sample) {
            let from = *run_from.get_or_insert(sample.at_ms);
            if sample.at_ms - from >= HEAVY_WINDOW_MS * 9 / 10 {
                entered = Some(index);
            }
        } else {
            run_from = None;
        }
    }
    let Some(entered) = entered else {
        return false;
    };
    let since = &ring[entered + 1..];
    let Some(newest) = since.last() else {
        return true; // heavy right now
    };
    // The calm run at the end, after it turned heavy.
    let calm_from = since
        .iter()
        .rev()
        .take_while(|sample| calm(sample))
        .last()
        .map(|sample| sample.at_ms);
    calm_from.is_none_or(|from| newest.at_ms - from < HEAVY_EXIT_MS * 9 / 10)
}

/// Pure: the rail's note for a tree, only when it's heavy.
#[must_use]
pub fn heavy_note(tree: &TreeInput) -> Option<String> {
    let cpu_hog = tree
        .hog_rules
        .iter()
        .any(|rule| rule.starts_with("tree-cpu"));
    let rss_hog = tree
        .hog_rules
        .iter()
        .any(|rule| !rule.starts_with("tree-cpu"));
    let cpu_heavy = cpu_hog
        || heavy_with_hysteresis(
            &tree.ring,
            |sample| sample.cpu_pct.is_some_and(|cpu| cpu > HEAVY_CPU_PCT),
            |sample| {
                sample
                    .cpu_pct
                    .is_none_or(|cpu| cpu < HEAVY_CPU_PCT * HEAVY_EXIT_RATIO)
            },
        );
    // A tool AI Juggler didn't start (the Claude app: Electron helpers that
    // share pages) is summed RSS, an overcount — so its heaviness is CPU
    // only, never a memory figure that flags every window.
    let rss_heavy = tree.place.is_none()
        && (rss_hog
            || heavy_with_hysteresis(
                &tree.ring,
                |sample| sample.rss_bytes > HEAVY_RSS_BYTES,
                #[allow(clippy::cast_precision_loss)]
                |sample| (sample.rss_bytes as f64) < HEAVY_RSS_BYTES as f64 * HEAVY_EXIT_RATIO,
            ));
    let mut parts = Vec::new();
    if cpu_heavy && let Some(cpu) = tree.stats.cpu_pct {
        parts.push(format!("CPU {cpu:.0}%"));
    }
    if rss_heavy {
        parts.push(bytes_words(tree.stats.rss_bytes));
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn tree_info(tree: &TreeInput) -> ToolTreeInfo {
    ToolTreeInfo {
        root_pid: tree.root_pid,
        session_id: tree.session_id.clone(),
        place: tree.place.map(str::to_string),
        tool: tree.tool.clone(),
        group: crate::machine_tools::group_of(tree),
        account_label: tree.account_label.clone(),
        cpu_pct: tree.stats.cpu_pct,
        rss_bytes: tree.stats.rss_bytes,
        helpers: tree.stats.child_count,
        heavy_note: heavy_note(tree),
        memory_estimate: tree.place.is_some(),
    }
}

/// Pure: This Mac from its parts (AJ-E1403 Req.1–2).
#[must_use]
pub fn assemble(
    machine: MachineHealthInfo,
    trees: &[TreeInput],
    leftovers: (u32, u32),
) -> ThisMacInfo {
    let cores = machine.cores;
    // A tree with nothing measured (its process has exited, or it has no
    // sample yet) isn't on This Mac — never listed as "under 1 MB".
    let all: Vec<&TreeInput> = trees
        .iter()
        .filter(|t| t.stats.measured().is_some())
        .collect();
    let ours: Vec<&TreeInput> = all
        .iter()
        .copied()
        .filter(|t| t.session_id.is_some())
        .collect();
    let theirs: Vec<&TreeInput> = all
        .iter()
        .copied()
        .filter(|t| t.session_id.is_none())
        .collect();
    let mut by_memory = all.clone();
    by_memory.sort_by_key(|tree| std::cmp::Reverse(tree.stats.rss_bytes));
    ThisMacInfo {
        tools: totals(&all, cores),
        ours: totals(&ours, cores),
        theirs: totals(&theirs, cores),
        by_tool: by_tool(&ours, &theirs, cores),
        top: by_memory
            .iter()
            .take(3)
            .map(|tree| tree_info(tree))
            .collect(),
        others: by_memory
            .iter()
            .skip(3)
            .map(|tree| tree_info(tree))
            .collect(),
        heavy: by_memory
            .iter()
            .map(|tree| tree_info(tree))
            .filter(|info| info.heavy_note.is_some())
            .collect(),
        leftover_processes: leftovers.0,
        leftover_ports: leftovers.1,
        alerts: Vec::new(),
        recommendations: Vec::new(),
        awake: crate::features::app::awake::AwakeInfo::default(),
        own: crate::features::app::own::OwnInfo::default(),
        machine,
    }
}

/// The Account whose folder a running tool uses (its managed folder or
/// the folder it was brought in from).
fn account_for(
    tool: &RunningTool,
    accounts: &[juggler_core::model::Account],
    home: &std::path::Path,
) -> Option<String> {
    accounts
        .iter()
        .find(|account| {
            crate::running_setups::uses_folder(tool, &account.state_dir.to_string_lossy(), home)
                || account
                    .import_source_path
                    .as_deref()
                    .is_some_and(|source| crate::running_setups::uses_folder(tool, source, home))
        })
        .map(|account| account.label.clone())
}

/// Every tree This Mac shows: our live sessions' and the observed tools'.
fn tree_inputs(
    state: &Arc<AppState>,
    now_ms: i64,
    tools: &[RunningTool],
    accounts: &[juggler_core::model::Account],
    live: &std::collections::HashSet<String>,
    labels: &std::collections::HashMap<String, String>,
) -> AppResult<Vec<TreeInput>> {
    let mut hogs: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for hog in crate::commands::hog_facts(state, now_ms) {
        hogs.entry(hog.session).or_default().push(hog.rule);
    }
    let contexts: std::collections::HashMap<String, juggler_supervisor::tree::SpawnContext> = state
        .supervisor
        .spawn_contexts_snapshot()
        .into_iter()
        .map(|(session, ctx)| (session.as_str().to_string(), ctx))
        .collect();
    let home = dirs::home_dir().unwrap_or_default();
    let roots: std::collections::HashMap<String, u32> = state
        .supervisor
        .live_roots()
        .into_iter()
        .map(|(session, pid, _)| (session.as_str().to_string(), pid))
        .collect();
    let mut trees = Vec::new();
    {
        let collector = state
            .resources
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        for (session, stats) in collector.stats_snapshot() {
            let id = session.as_str().to_string();
            if !live.contains(&id) {
                continue;
            }
            trees.push(TreeInput {
                root_pid: roots.get(&id).copied(),
                tool: contexts.get(&id).map(|ctx| ctx.provider.clone()),
                account_label: contexts
                    .get(&id)
                    .and_then(|ctx| ctx.account_id.as_ref())
                    .and_then(|account| labels.get(account).cloned()),
                ring: collector.ring(&session),
                hog_rules: hogs.remove(&id).unwrap_or_default(),
                session_id: Some(id),
                place: None,
                stats,
            });
        }
        let observed = collector.observed_snapshot();
        for tool in tools {
            let Some(tree) = observed.get(&tool.pid) else {
                continue; // not measured yet, or it is one of ours
            };
            trees.push(TreeInput {
                root_pid: Some(tool.pid),
                session_id: None,
                place: Some(tool.place),
                tool: Some(OBSERVED_TOOL.to_string()),
                account_label: account_for(tool, accounts, &home),
                stats: tree.stats,
                ring: tree.ring.iter().copied().collect(),
                hog_rules: Vec::new(),
            });
        }
    }
    Ok(trees)
}

/// This Mac now (AJ-E1403).
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when a lock is poisoned; storage errors.
pub fn this_mac_impl(state: &Arc<AppState>, now_ms: i64) -> AppResult<ThisMacInfo> {
    let (machine, tools) = {
        let watch = state.machine.lock().map_err(|_| AppError::StatePoisoned)?;
        (
            health_of(watch.sampler.snapshot(), &RoomThresholds::default()),
            watch.tools.clone(),
        )
    };
    let (accounts, live, labels) = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        let accounts = storage.accounts().list()?;
        let live: std::collections::HashSet<String> = storage
            .sessions()
            .list(None)?
            .into_iter()
            .filter(|record| crate::commands::is_active(record.status))
            .map(|record| record.id.to_string())
            .collect();
        let labels: std::collections::HashMap<String, String> = accounts
            .iter()
            .map(|account| (account.id.to_string(), account.label.clone()))
            .collect();
        (accounts, live, labels)
    };
    let trees = tree_inputs(state, now_ms, &tools, &accounts, &live, &labels)?;
    let rows = crate::commands::local_ledger_rows(state);
    let orphaned: Vec<_> = rows
        .iter()
        .filter(|row| row.state == "orphaned" && row.role == "helper")
        .collect();
    let leftovers = (
        u32::try_from(orphaned.len()).unwrap_or(u32::MAX),
        u32::try_from(
            orphaned
                .iter()
                .filter(|row| row.listens_on.is_some())
                .count(),
        )
        .unwrap_or(u32::MAX),
    );
    let mut info = assemble(machine, &trees, leftovers);
    info.recommendations =
        crate::machine_recommendations::recommendations_for(state, &info, now_ms);
    info.awake = crate::features::app::awake::current(&state.awake);
    info.own = crate::features::app::own::measure(state);
    if let Ok(mut watch) = state.machine.lock() {
        watch.alerts.evaluate(&info, now_ms);
        info.alerts = watch.alerts.active();
    }
    align_with_alerts(&mut info.machine.summary, &info.alerts);
    Ok(info)
}

/// The words This Mac uses while the memory-pressure alert is on.
pub const UNDER_MEMORY_PRESSURE: &str = "Under memory pressure";

/// One severity model (the 0.0.12 report: the alert said "under memory
/// pressure" while This Mac said "Getting tight"): while the memory
/// pressure alert is on, This Mac's state says the same. "Getting tight"
/// stays for the calmer state before an alert.
pub fn align_with_alerts(
    summary: &mut RoomSummaryInfo,
    alerts: &[crate::machine_alerts::OverloadAlertInfo],
) {
    if !alerts.iter().any(|alert| alert.kind == "memory_pressure") {
        return;
    }
    summary.level = "pressure".to_string();
    summary.words = UNDER_MEMORY_PRESSURE.to_string();
    for reason in &mut summary.reasons {
        if reason.kind == "pressure" {
            reason.level = "pressure".to_string();
        }
    }
}

/// The alerts to tell since the last call (AJ-E1408; the notification
/// region's poll drains them).
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when the watch's lock is poisoned.
pub fn machine_alerts_take_impl(
    state: &Arc<AppState>,
) -> AppResult<Vec<crate::machine_alerts::OverloadAlertInfo>> {
    let mut watch = state.machine.lock().map_err(|_| AppError::StatePoisoned)?;
    Ok(watch.alerts.take_fresh())
}

/// Overload alerts to tell now (AJ-E1408). Passive: a notification only.
///
/// # Errors
///
/// As [`machine_alerts_take_impl`].
#[tauri::command]
pub async fn machine_alerts_take(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<crate::machine_alerts::OverloadAlertInfo>> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || machine_alerts_take_impl(&state))
        .await
        .map_err(|error| AppError::Io {
            context: "read the overload alerts",
            message: error.to_string(),
        })?
}

/// This Mac (AJ-E1403): the machine, AI tools' share, the heaviest
/// trees, leftovers. Off the main thread: it reads storage and two locks
/// the sampling tick also takes.
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when a lock is poisoned; storage errors.
#[tauri::command]
pub async fn this_mac(state: tauri::State<'_, Arc<AppState>>) -> AppResult<ThisMacInfo> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        this_mac_impl(&state, crate::commands::now_millis())
    })
    .await
    .map_err(|error| AppError::Io {
        context: "read This Mac",
        message: error.to_string(),
    })?
}

/// What was last emitted: the machine's level and how many AI tools ran.
static LAST_EMITTED: std::sync::Mutex<Option<(String, u32)>> = std::sync::Mutex::new(None);

/// Pure: whether This Mac goes out this tick. Always while an AI tool
/// runs; otherwise only when something changed — the level, or the count
/// (so the step from one tool to none is told, and Home never keeps a
/// stale "1 running").
#[must_use]
pub fn should_emit(last: &mut Option<(String, u32)>, level: &str, tools: u32) -> bool {
    let now = (level.to_string(), tools);
    let changed = last.as_ref() != Some(&now);
    *last = Some(now);
    tools > 0 || changed
}

/// Emits This Mac on the 5 s resource cadence (`machine://this-mac`), so
/// Home, the rail and the session header refresh without a UI timer.
/// Like the resource tick, an idle app costs the UI nothing: with no AI
/// tool running, it emits only when something changed (see [`should_emit`]).
///
/// # Errors
///
/// As [`this_mac_impl`].
pub fn emit_this_mac(app: &tauri::AppHandle, state: &Arc<AppState>) -> AppResult<()> {
    use tauri::Emitter;
    let payload = this_mac_impl(state, crate::commands::now_millis())?;
    let emit = LAST_EMITTED.lock().map_or(true, |mut last| {
        should_emit(
            &mut last,
            &payload.machine.summary.level,
            payload.tools.count,
        )
    });
    if !emit {
        return Ok(());
    }
    let _ = app.emit(crate::EVENT_THIS_MAC, payload);
    Ok(())
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

    fn machine(available: u64, pressure: &str, reasons: Vec<(&str, &str)>) -> MachineHealthInfo {
        let mut info = health_of(&MachineSnapshot::default(), &RoomThresholds::default());
        info.sampled = true;
        info.memory_total = 32 * G;
        info.memory_available = available;
        info.memory_used = Some(32 * G - available);
        info.memory_basis = Some("app_wired_compressed".into());
        info.pressure = Some(pressure.to_string());
        info.summary.reasons = reasons
            .into_iter()
            .map(|(kind, level)| RoomReasonInfo {
                kind: kind.into(),
                level: level.into(),
                words: format!("{kind} words"),
            })
            .collect();
        info
    }

    fn learned(count: u32, median: u64) -> juggler_core::storage::FootprintStats {
        juggler_core::storage::FootprintStats {
            key: "recipe:r".into(),
            count,
            median_peak_rss: median,
            median_p95_cpu: Some(30.0),
        }
    }

    #[test]
    fn the_advisor_says_ok_tight_or_no_and_why() {
        let roomy = advise(
            &machine(12 * G, "normal", vec![]),
            "r",
            Some(&learned(6, 2 * G)),
            None,
        );
        assert_eq!(roomy.verdict, "ok");
        assert_eq!(roomy.words, "This Mac can take another session");
        assert_eq!((roomy.projected_bytes, roomy.based_on), (2 * G, 6));
        assert!(roomy.reasons.is_empty());
        // 6 GB free − 2 GB = 4 GB = 12.5 % of 32 GB: tight.
        let tight = advise(
            &machine(6 * G, "normal", vec![]),
            "r",
            Some(&learned(6, 2 * G)),
            Some(("s-heavy", 3 * G)),
        );
        assert_eq!(tight.verdict, "tight");
        assert_eq!(
            tight.reasons,
            ["it would leave about 4.0 GB of memory free"]
        );
        assert_eq!(tight.relief_session_id.as_deref(), Some("s-heavy"));
        assert_eq!(tight.relief_bytes, Some(3 * G));
        // Under 5 % left after the launch: no.
        let no = advise(
            &machine(3 * G, "normal", vec![]),
            "r",
            Some(&learned(6, 2 * G)),
            None,
        );
        assert_eq!(no.verdict, "no");
        assert_eq!(no.words, "Not now — this Mac is short of room");
        // The kernel's own pressure decides too.
        assert_eq!(
            advise(&machine(20 * G, "warning", vec![]), "r", None, None).verdict,
            "tight"
        );
        assert_eq!(
            advise(&machine(20 * G, "critical", vec![]), "r", None, None).verdict,
            "no"
        );
        // Busy CPU or a hot Mac is tight; a memory reason is already counted.
        let busy = advise(
            &machine(
                20 * G,
                "normal",
                vec![("cpu", "tight"), ("memory", "tight")],
            ),
            "r",
            None,
            None,
        );
        assert_eq!(busy.verdict, "tight");
        assert_eq!(busy.reasons, ["cpu words"]);
    }

    #[test]
    fn until_three_sessions_are_measured_the_projection_is_the_default() {
        let fresh = advise(
            &machine(12 * G, "normal", vec![]),
            "r",
            Some(&learned(2, 5 * G)),
            None,
        );
        assert_eq!(
            (fresh.projected_bytes, fresh.based_on),
            (UNMEASURED_PROJECTION, 0)
        );
        let none = advise(&machine(12 * G, "normal", vec![]), "r", None, None);
        assert_eq!(none.based_on, 0);
    }

    fn stats(key: &str, count: u32, median: u64) -> juggler_core::storage::FootprintStats {
        juggler_core::storage::FootprintStats {
            key: key.into(),
            count,
            median_peak_rss: median,
            median_p95_cpu: None,
        }
    }

    #[test]
    fn recommendations_come_from_enough_real_sessions() {
        // 32 GB Mac, 20 GB in use of which 6 GB are AI tools: 14 GB is
        // everything else; 80 % of 32 = 25.6 → 11.6 GB for sessions.
        let machine = machine(12 * G, "normal", vec![]);
        let names: std::collections::HashMap<String, String> = [
            ("deep".to_string(), "“Deep work” (Work Max)".to_string()),
            ("quick".to_string(), "“Quick” (Work Max)".to_string()),
        ]
        .into();
        assert!(
            recommendations(&machine, 6 * G, &[stats("recipe:deep", 2, 2 * G)], &names).is_empty(),
            "two sessions are not enough"
        );
        let one = recommendations(&machine, 6 * G, &[stats("recipe:deep", 4, 2 * G)], &names);
        assert_eq!(
            one,
            [
                "This Mac comfortably runs about 5 sessions at once (about 2.0 GB each, from your last 4)"
            ]
        );
        let two = recommendations(
            &machine,
            6 * G,
            &[
                stats("recipe:deep", 4, 3 * G),
                stats("recipe:quick", 6, G),
                stats("observed:desktop", 9, 9 * G),
            ],
            &names,
        );
        assert_eq!(two.len(), 2);
        assert_eq!(
            two[1],
            "“Deep work” (Work Max) is the heaviest launch setting — about 3.0 GB a session"
        );
        // A deleted launch setting (no name) doesn't count.
        let gone = recommendations(
            &machine,
            6 * G,
            &[
                stats("recipe:deep", 4, 2 * G),
                stats("recipe:gone", 9, 8 * G),
            ],
            &names,
        );
        assert_eq!(
            gone,
            [
                "This Mac comfortably runs about 5 sessions at once (about 2.0 GB each, from your last 4)"
            ]
        );
        // Close weights name no heaviest one.
        let close = recommendations(
            &machine,
            6 * G,
            &[
                stats("recipe:deep", 4, 2 * G),
                stats("recipe:quick", 4, 1_600_000_000),
            ],
            &names,
        );
        assert_eq!(close.len(), 1);
    }

    #[test]
    fn p95_is_the_nearest_rank() {
        let samples: std::collections::VecDeque<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(p95(&samples), Some(95.0));
        assert_eq!(p95(&std::collections::VecDeque::new()), None);
        assert_eq!(p95(&[7.0].into_iter().collect()), Some(7.0));
    }

    fn ring(cpu: f64, rss: u64, seconds: i64) -> Vec<TreeSample> {
        (0..=seconds / 2)
            .map(|i| TreeSample {
                at_ms: i * 2_000,
                cpu_pct: Some(cpu),
                rss_bytes: rss,
            })
            .collect()
    }

    fn tree(session: Option<&str>, cpu: f64, rss: u64, seconds: i64) -> TreeInput {
        TreeInput {
            root_pid: None,
            session_id: session.map(str::to_string),
            place: session.is_none().then_some("desktop"),
            tool: session.map(|_| OBSERVED_TOOL.to_string()),
            account_label: Some("Work Max".into()),
            stats: LiveProcessStats {
                child_count: 3,
                cpu_pct: Some(cpu),
                rss_bytes: rss,
            },
            ring: ring(cpu, rss, seconds),
            hog_rules: Vec::new(),
        }
    }

    #[test]
    fn heavy_means_sustained_for_a_minute() {
        assert_eq!(
            heavy_note(&tree(Some("a"), 95.0, G, 60)).as_deref(),
            Some("CPU 95%")
        );
        assert_eq!(
            heavy_note(&tree(Some("a"), 20.0, 3_200_000_000, 60)).as_deref(),
            Some("3.2 GB")
        );
        assert_eq!(
            heavy_note(&tree(Some("a"), 120.0, 3_200_000_000, 60)).as_deref(),
            Some("CPU 120% · 3.2 GB")
        );
        // Twenty seconds of it is a spike, not a heavy session.
        assert_eq!(heavy_note(&tree(Some("a"), 99.0, 5 * G, 20)), None);
        // A quiet session is never noted.
        assert_eq!(heavy_note(&tree(Some("a"), 12.0, G, 300)), None);
        // An AJ-E512 hog rule counts on its own.
        let mut hog = tree(Some("a"), 40.0, 2_500_000_000, 10);
        hog.hog_rules = vec!["tree-rss>p95(provider)".into()];
        assert_eq!(heavy_note(&hog).as_deref(), Some("2.5 GB"));
    }

    /// A ring from (seconds, cpu) legs, one sample every 2 s, capped as the
    /// supervisor caps it.
    fn legs(legs: &[(i64, f64)]) -> Vec<TreeSample> {
        let mut at = 0;
        let mut out = Vec::new();
        for &(seconds, cpu) in legs {
            for _ in 0..seconds / 2 {
                out.push(TreeSample {
                    at_ms: at,
                    cpu_pct: Some(cpu),
                    rss_bytes: G,
                });
                at += 2_000;
            }
        }
        // The supervisor's ring keeps 150 samples (5 minutes).
        let keep = out.len().saturating_sub(150);
        out.split_off(keep)
    }

    #[test]
    fn a_session_hovering_near_the_line_stays_heavy_until_it_calms() {
        let note = |ring: Vec<TreeSample>| {
            let mut hovering = tree(Some("a"), 0.0, G, 0);
            hovering.stats.cpu_pct = ring.last().and_then(|sample| sample.cpu_pct);
            hovering.ring = ring;
            heavy_note(&hovering)
        };
        // A minute over 90 %, then dips between 80 and 90 %: still heavy.
        assert!(note(legs(&[(62, 95.0), (4, 85.0)])).is_some());
        assert!(note(legs(&[(62, 95.0), (4, 85.0), (4, 95.0), (60, 86.0)])).is_some());
        // One sample under 80 % doesn't clear it either …
        assert!(note(legs(&[(62, 95.0), (2, 30.0), (10, 92.0)])).is_some());
        assert!(note(legs(&[(62, 95.0), (20, 30.0)])).is_some());
        // … 45 s under 80 % does.
        assert!(note(legs(&[(62, 95.0), (48, 30.0)])).is_none());
        // Never a minute over: never heavy, however long it hovers.
        assert!(note(legs(&[(40, 95.0), (2, 85.0), (40, 95.0)])).is_none());
        // Gone from the ring (5 minutes later) is calm too.
        assert!(note(legs(&[(62, 95.0), (260, 86.0)])).is_none());
    }

    #[test]
    fn a_tool_we_didnt_start_is_heavy_on_cpu_only_and_its_memory_is_an_estimate() {
        // A Claude app window: 16 GB summed over helpers that share pages.
        let mut app = tree(None, 20.0, 16 * G, 60);
        assert_eq!(heavy_note(&app), None, "summed RSS never flags it");
        app = tree(None, 140.0, 16 * G, 60);
        assert_eq!(heavy_note(&app).as_deref(), Some("CPU 140%"));
        let machine = health_of(
            &MachineSnapshot {
                cores: 10,
                ..MachineSnapshot::default()
            },
            &RoomThresholds::default(),
        );
        let info = assemble(machine, &[app, tree(Some("s1"), 10.0, G, 60)], (0, 0));
        let theirs = info.top.iter().find(|t| t.session_id.is_none()).unwrap();
        assert!(theirs.memory_estimate);
        assert!(
            !info
                .top
                .iter()
                .find(|t| t.session_id.is_some())
                .unwrap()
                .memory_estimate
        );
        assert!(info.tools.memory_estimate && info.theirs.memory_estimate);
        assert!(!info.ours.memory_estimate);
    }

    #[test]
    fn going_from_one_tool_to_none_is_told() {
        let mut last = None;
        assert!(should_emit(&mut last, "plenty", 1));
        assert!(
            should_emit(&mut last, "plenty", 1),
            "always while a tool runs"
        );
        assert!(
            should_emit(&mut last, "plenty", 0),
            "the step to zero is told"
        );
        assert!(
            !should_emit(&mut last, "plenty", 0),
            "then an idle app is quiet"
        );
        assert!(should_emit(&mut last, "tight", 0), "a level change is told");
        assert!(!should_emit(&mut last, "tight", 0));
    }

    #[test]
    fn a_claude_we_started_is_never_counted_as_theirs() {
        let tool = |pid| RunningTool {
            pid,
            place: "terminal",
            config_dir: std::path::PathBuf::from("/Users/you/.claude"),
            start_time: None,
        };
        // 42 is a leftover `claude` AI Juggler started (it has a ledger
        // row), now adopted by launchd; 7 is the user's own terminal.
        let ours: std::collections::HashSet<u32> = [42].into_iter().collect();
        let left = not_ours(vec![tool(42), tool(7)], &ours);
        assert_eq!(left.iter().map(|t| t.pid).collect::<Vec<_>>(), [7]);
    }

    #[test]
    fn this_mac_splits_ours_from_theirs_and_ranks_by_memory() {
        let machine = health_of(
            &MachineSnapshot {
                cores: 10,
                ..MachineSnapshot::default()
            },
            &RoomThresholds::default(),
        );
        let trees = vec![
            tree(Some("s1"), 50.0, G, 60),
            tree(Some("s2"), 95.0, 3 * G, 60),
            tree(None, 30.0, 2 * G - 1, 60),
            tree(Some("s3"), 5.0, 500_000_000, 60),
        ];
        let info = assemble(machine, &trees, (2, 1));
        assert_eq!(info.tools.count, 4);
        assert_eq!(info.ours.count, 3);
        assert_eq!(info.theirs.count, 1);
        assert_eq!(info.tools.rss_bytes, 6_500_000_000 - 1);
        // 180 % of one core over 10 cores = 18 % of the Mac.
        assert!((info.tools.cpu_pct.unwrap() - 18.0).abs() < 1e-9);
        assert!((info.theirs.cpu_pct.unwrap() - 3.0).abs() < 1e-9);
        let top: Vec<Option<&str>> = info.top.iter().map(|t| t.session_id.as_deref()).collect();
        assert_eq!(top, [Some("s2"), None, Some("s1")]);
        assert_eq!(info.top[1].place.as_deref(), Some("desktop"));
        assert_eq!(info.heavy.len(), 1);
        assert_eq!(
            info.heavy[0].heavy_note.as_deref(),
            Some("CPU 95% · 3.0 GB")
        );
        assert_eq!((info.leftover_processes, info.leftover_ports), (2, 1));
    }

    #[test]
    fn an_unmeasured_tree_leaves_the_cpu_total_unknown_never_zero() {
        let machine = health_of(
            &MachineSnapshot {
                cores: 8,
                ..MachineSnapshot::default()
            },
            &RoomThresholds::default(),
        );
        let mut fresh = tree(Some("s1"), 0.0, G, 0);
        fresh.stats.cpu_pct = None;
        let info = assemble(machine.clone(), &[fresh], (0, 0));
        assert_eq!(info.tools.cpu_pct, None);
        let empty = assemble(machine, &[], (0, 0));
        assert_eq!(
            empty.tools.cpu_pct,
            Some(0.0),
            "nothing running uses nothing"
        );
        assert!(empty.top.is_empty());
    }

    /// AJ-E1535: a session whose process has exited before it's marked
    /// ended reports zero sums. That is nothing measured, never a tree
    /// "under 1 MB": This Mac leaves it out, and its totals with it.
    #[test]
    fn a_tree_with_nothing_measured_is_not_on_this_mac() {
        let machine = health_of(
            &MachineSnapshot {
                cores: 8,
                ..MachineSnapshot::default()
            },
            &RoomThresholds::default(),
        );
        let mut gone = tree(Some("gone"), 0.0, 0, 0);
        gone.stats = LiveProcessStats::dead();
        let info = assemble(machine, &[gone, tree(Some("s1"), 10.0, G, 60)], (0, 0));
        assert_eq!(info.top.len(), 1);
        assert_eq!(info.top[0].session_id.as_deref(), Some("s1"));
        assert_eq!((info.ours.count, info.ours.rss_bytes), (1, G));
        assert!(LiveProcessStats::dead().measured().is_none());
    }

    /// P6 (QA): This Mac showed 3 of 6 Claude app windows and said nothing
    /// of the rest. The top three, then every other tree, heaviest first.
    #[test]
    fn no_tree_is_dropped_past_the_top_three() {
        let machine = health_of(
            &MachineSnapshot {
                cores: 8,
                ..MachineSnapshot::default()
            },
            &RoomThresholds::default(),
        );
        let trees: Vec<TreeInput> = [5, 1, 4, 2, 3]
            .into_iter()
            .map(|gigs| tree(None, 0.0, gigs * G, 0))
            .collect();
        let info = assemble(machine, &trees, (0, 0));
        let sizes =
            |list: &[ToolTreeInfo]| list.iter().map(|t| t.rss_bytes / G).collect::<Vec<_>>();
        assert_eq!(sizes(&info.top), [5, 4, 3]);
        assert_eq!(sizes(&info.others), [2, 1]);
    }
}

/// A by-hand probe on a real Mac (read-only, AJ-E1402 evidence): the scan's
/// cost and the AI tools AI Juggler did not start, by place, with their
/// tree totals. Prints no paths, names or environments.
/// `cargo test -p juggler-app --lib machine_health::live -- --ignored --nocapture`
#[cfg(test)]
mod live {
    #[test]
    #[ignore = "reads this Mac's real process table; run by hand"]
    fn print_observed_tools() {
        let home = dirs::home_dir().unwrap_or_default();
        let mut costs = Vec::new();
        let mut tools = Vec::new();
        let supervisor = juggler_supervisor::Supervisor::new();
        let mut collector = juggler_supervisor::LiveStatsCollector::new();
        collector.tick(&supervisor, 0); // a full scan fills the table
        for _ in 0..5 {
            let started = std::time::Instant::now();
            tools = crate::running_setups::tools_from(&collector.process_briefs(), &home);
            costs.push(started.elapsed().as_micros());
        }
        costs.sort_unstable();
        println!("scan: median {} µs, max {} µs (5 runs)", costs[2], costs[4]);
        collector.set_observed_roots(
            tools
                .iter()
                .map(|tool| juggler_supervisor::ObservedRoot {
                    pid: tool.pid,
                    start_time: tool.start_time,
                })
                .collect(),
        );
        for tick in 1..4 {
            collector.tick(&supervisor, tick * 2_000);
            std::thread::sleep(std::time::Duration::from_millis(700));
        }
        // Baseline: the same ticks on a collector observing nothing.
        let mut bare = juggler_supervisor::LiveStatsCollector::new();
        let mut base = Vec::new();
        for tick in 0..12 {
            let started = std::time::Instant::now();
            bare.tick(&supervisor, tick * 2_000);
            if tick >= 4 {
                base.push(started.elapsed().as_micros());
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        base.sort_unstable();
        println!(
            "tick observing nothing: median {} µs, max {} µs",
            base[4], base[7]
        );
        // The cost of the hot tick now that every helper of every observed
        // tree is refreshed (review of #68).
        let mut costs = Vec::new();
        for tick in 4..12 {
            let started = std::time::Instant::now();
            collector.tick(&supervisor, tick * 2_000);
            costs.push(started.elapsed().as_micros());
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        costs.sort_unstable();
        println!(
            "tick with observed trees: median {} µs, max {} µs (8 ticks, full scans included)",
            costs[4], costs[7]
        );
        let observed = collector.observed_snapshot();
        for place in ["desktop", "terminal"] {
            let trees: Vec<_> = tools
                .iter()
                .filter(|tool| tool.place == place)
                .filter_map(|tool| observed.get(&tool.pid))
                .collect();
            let rss: u64 = trees.iter().map(|tree| tree.stats.rss_bytes).sum();
            let cpu: f64 = trees.iter().filter_map(|tree| tree.stats.cpu_pct).sum();
            let helpers: u32 = trees.iter().map(|tree| tree.stats.child_count).sum();
            println!(
                "{place}: {} trees, {} helpers, CPU {cpu:.0}% of one core, {}",
                trees.len(),
                helpers,
                juggler_supervisor::machine::bytes_words(rss)
            );
        }
    }
    /// The 0.0.12 report: one state, one set of words. The alert on means
    /// This Mac says "Under memory pressure"; before it, "Getting tight".
    #[test]
    fn this_mac_says_what_the_pressure_alert_says() {
        let tight = || crate::machine_health::RoomSummaryInfo {
            level: "tight".into(),
            words: "Getting tight".into(),
            reasons: vec![crate::machine_health::RoomReasonInfo {
                kind: "pressure".into(),
                level: "tight".into(),
                words: "macOS reports memory pressure".into(),
            }],
        };
        let mut before = tight();
        crate::machine_health::align_with_alerts(&mut before, &[]);
        assert_eq!(before.words, "Getting tight", "calm before the alert");
        let alert = crate::machine_alerts::OverloadAlertInfo {
            key: "memory_pressure".into(),
            kind: "memory_pressure".into(),
            title: "Your Mac is under memory pressure".into(),
            detail: "macOS reports memory pressure".into(),
            session_id: None,
            place: Some("desktop".into()),
            subject_bytes: Some(16_000_000_000),
            subject_estimate: true,
            subject_cpu: None,
            subject_windows: Some(3),
            since_ms: 0,
        };
        let mut during = tight();
        crate::machine_health::align_with_alerts(&mut during, std::slice::from_ref(&alert));
        assert_eq!(during.level, "pressure");
        assert_eq!(during.words, crate::machine_health::UNDER_MEMORY_PRESSURE);
        assert!(
            alert
                .title
                .to_lowercase()
                .contains(&crate::machine_health::UNDER_MEMORY_PRESSURE.to_lowercase())
        );
        assert_eq!(during.reasons[0].level, "pressure");
    }
}
