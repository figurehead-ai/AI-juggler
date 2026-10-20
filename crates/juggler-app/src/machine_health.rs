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

use std::path::PathBuf;
use std::sync::Arc;

use juggler_supervisor::machine::{
    MachineSampler, MachineSnapshot, PowerState, PressureLevel, RoomLevel, RoomThresholds,
    ThermalState, bytes_words, summarize, swap_growth_mb_per_min,
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
    /// Memory in use (total − available), bytes.
    #[ts(type = "number")]
    pub memory_used: u64,
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
    /// Memory in use (total − available), bytes.
    #[ts(type = "number")]
    pub memory_used: u64,
    /// Memory the OS can hand out without swapping, bytes.
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
        memory_used: latest.memory_total.saturating_sub(latest.memory_available),
        memory_available: latest.memory_available,
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
                memory_used: sample.memory_total.saturating_sub(sample.memory_available),
            })
            .collect(),
    }
}

/// The folders whose volumes matter: the data folder and every active
/// session's working folder.
fn paths_in_use(state: &Arc<AppState>) -> Vec<PathBuf> {
    let mut paths = vec![state.data_dir.clone()];
    if let Ok(storage) = state.storage.lock()
        && let Ok(sessions) = storage.sessions().list(None)
    {
        paths.extend(
            sessions
                .into_iter()
                .filter(|record| crate::commands::is_active(record.status))
                .map(|record| record.cwd),
        );
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
}

impl Default for MachineWatch {
    fn default() -> Self {
        Self {
            sampler: MachineSampler::new(),
            ticks: 0,
            tools: Vec::new(),
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

/// Which running tools are re-read every 15th tick (30 s): the read
/// covers every process's environment (only `CLAUDE_CONFIG_DIR` is kept),
/// so it never rides every tick.
const TOOLS_EVERY: u32 = 15;

/// The 2 s tick's machine half (AJ-E1401 Req.1, AJ-E1402 Req.1–2). Takes
/// only the watch's lock; the storage lock is taken (briefly, inside) on
/// the 30 s disk cadence alone, and the collector's lock after the watch's
/// is released.
pub fn tick(state: &Arc<AppState>, now_ms: i64) {
    let refresh_tools = {
        let Ok(mut watch) = state.machine.lock() else {
            return;
        };
        watch.ticks = watch.ticks.wrapping_add(1);
        watch.sampler.tick(now_ms, || paths_in_use(state));
        watch.ticks == 1 || watch.ticks.is_multiple_of(TOOLS_EVERY)
    };
    if !refresh_tools {
        return;
    }
    // Classified from the collector's own table (no second full refresh);
    // only the roots' `CLAUDE_CONFIG_DIR` is read.
    let briefs = state
        .resources
        .lock()
        .map(|collector| collector.process_briefs())
        .unwrap_or_default();
    let tools = dirs::home_dir().map_or_else(Vec::new, |home| {
        crate::running_setups::tools_from(&briefs, &home)
    });
    let roots: Vec<u32> = tools.iter().map(|tool| tool.pid).collect();
    if let Ok(mut watch) = state.machine.lock() {
        watch.tools = tools;
    }
    if let Ok(mut collector) = state.resources.lock() {
        collector.set_observed_roots(roots);
    }
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

/// How the Mac is doing (AJ-E1401 Req.4).
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when the watch's lock is poisoned.
#[tauri::command]
pub fn machine_health(state: tauri::State<'_, Arc<AppState>>) -> AppResult<MachineHealthInfo> {
    machine_health_impl(state.inner())
}

// --- AJ-E1403: the This Mac read model ------------------------------------

/// A session is heavy above this (percent of one core, the AJ-E512 rule)…
const HEAVY_CPU_PCT: f64 = 90.0;
/// … or above this much memory …
const HEAVY_RSS_BYTES: u64 = 2_000_000_000;
/// … for every sample of the last minute.
const HEAVY_WINDOW_MS: i64 = 60_000;

/// One process tree on This Mac: a session AI Juggler started, or an AI
/// tool it did not (read-only; no actions).
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ToolTreeInfo {
    /// The session id for a session AI Juggler started; `null` for a tool
    /// it did not start.
    pub session_id: Option<String>,
    /// For a tool AI Juggler did not start: `desktop` (the Claude app) or
    /// `terminal` (`claude` in a terminal).
    pub place: Option<String>,
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
}

/// AI tools' share of the Mac.
#[derive(Clone, PartialEq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ToolTotalsInfo {
    /// How many trees.
    pub count: u32,
    /// Share of the whole Mac's CPU (all cores), percent; `null` until
    /// measured.
    pub cpu_pct: Option<f64>,
    /// Memory, bytes.
    #[ts(type = "number")]
    pub rss_bytes: u64,
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
    /// The heaviest trees by memory, at most three.
    pub top: Vec<ToolTreeInfo>,
    /// Every heavy tree (the rail's notes, Home's Needs you).
    pub heavy: Vec<ToolTreeInfo>,
    /// Processes left running after their session closed.
    pub leftover_processes: u32,
    /// Ports those leftovers hold.
    pub leftover_ports: u32,
}

/// One tree, as the pure assembly sees it.
#[derive(Clone, Debug, Default)]
pub struct TreeInput {
    /// Session id (ours) or `None` (observed).
    pub session_id: Option<String>,
    /// `desktop` | `terminal` for observed trees.
    pub place: Option<&'static str>,
    /// The matched Account's label.
    pub account_label: Option<String>,
    /// Tree sums.
    pub stats: LiveProcessStats,
    /// The tree's 2 s ring.
    pub ring: Vec<TreeSample>,
    /// AJ-E512 hog rules that fired for it.
    pub hog_rules: Vec<String>,
}

/// Sustained = every sample of the last minute passes, and the ring
/// really spans (nearly) a minute.
fn sustained(ring: &[TreeSample], passes: impl Fn(&TreeSample) -> bool) -> bool {
    let Some(newest) = ring.last() else {
        return false;
    };
    let window: Vec<&TreeSample> = ring
        .iter()
        .filter(|sample| newest.at_ms - sample.at_ms <= HEAVY_WINDOW_MS)
        .collect();
    let span = window.first().map_or(0, |first| newest.at_ms - first.at_ms);
    span >= HEAVY_WINDOW_MS * 9 / 10 && window.iter().all(|sample| passes(sample))
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
        || sustained(&tree.ring, |sample| {
            sample.cpu_pct.is_some_and(|cpu| cpu > HEAVY_CPU_PCT)
        });
    let rss_heavy = rss_hog || sustained(&tree.ring, |sample| sample.rss_bytes > HEAVY_RSS_BYTES);
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
        session_id: tree.session_id.clone(),
        place: tree.place.map(str::to_string),
        account_label: tree.account_label.clone(),
        cpu_pct: tree.stats.cpu_pct,
        rss_bytes: tree.stats.rss_bytes,
        helpers: tree.stats.child_count,
        heavy_note: heavy_note(tree),
    }
}

#[allow(clippy::cast_precision_loss)] // shares for display
fn totals(trees: &[&TreeInput], cores: u32) -> ToolTotalsInfo {
    let measured: Vec<f64> = trees.iter().filter_map(|tree| tree.stats.cpu_pct).collect();
    ToolTotalsInfo {
        count: u32::try_from(trees.len()).unwrap_or(u32::MAX),
        cpu_pct: (!trees.is_empty() && measured.len() == trees.len())
            .then(|| (measured.iter().sum::<f64>() / f64::from(cores.max(1))).min(100.0))
            .or_else(|| trees.is_empty().then_some(0.0)),
        rss_bytes: trees.iter().map(|tree| tree.stats.rss_bytes).sum(),
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
    let all: Vec<&TreeInput> = trees.iter().collect();
    let ours: Vec<&TreeInput> = trees.iter().filter(|t| t.session_id.is_some()).collect();
    let theirs: Vec<&TreeInput> = trees.iter().filter(|t| t.session_id.is_none()).collect();
    let mut by_memory = all.clone();
    by_memory.sort_by_key(|tree| std::cmp::Reverse(tree.stats.rss_bytes));
    ThisMacInfo {
        tools: totals(&all, cores),
        ours: totals(&ours, cores),
        theirs: totals(&theirs, cores),
        top: by_memory
            .iter()
            .take(3)
            .map(|tree| tree_info(tree))
            .collect(),
        heavy: by_memory
            .iter()
            .map(|tree| tree_info(tree))
            .filter(|info| info.heavy_note.is_some())
            .collect(),
        leftover_processes: leftovers.0,
        leftover_ports: leftovers.1,
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
    let mut hogs: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for hog in crate::commands::hog_facts(state, now_ms) {
        hogs.entry(hog.session).or_default().push(hog.rule);
    }
    let contexts: std::collections::HashMap<String, Option<String>> = state
        .supervisor
        .spawn_contexts_snapshot()
        .into_iter()
        .map(|(session, ctx)| (session.as_str().to_string(), ctx.account_id))
        .collect();
    let home = dirs::home_dir().unwrap_or_default();
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
                account_label: contexts
                    .get(&id)
                    .and_then(Option::as_ref)
                    .and_then(|account| labels.get(account).cloned()),
                ring: collector.ring(&session),
                hog_rules: hogs.remove(&id).unwrap_or_default(),
                session_id: Some(id),
                place: None,
                stats,
            });
        }
        let observed = collector.observed_snapshot();
        for tool in &tools {
            let Some(tree) = observed.get(&tool.pid) else {
                continue; // not measured yet, or it is one of ours
            };
            trees.push(TreeInput {
                session_id: None,
                place: Some(tool.place),
                account_label: account_for(tool, &accounts, &home),
                stats: tree.stats,
                ring: tree.ring.iter().copied().collect(),
                hog_rules: Vec::new(),
            });
        }
    }
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
    Ok(assemble(machine, &trees, leftovers))
}

/// This Mac (AJ-E1403): the machine, AI tools' share, the heaviest
/// trees, leftovers.
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when a lock is poisoned; storage errors.
#[tauri::command]
pub fn this_mac(state: tauri::State<'_, Arc<AppState>>) -> AppResult<ThisMacInfo> {
    this_mac_impl(state.inner(), crate::commands::now_millis())
}

/// The summary level last emitted (an idle app emits only on a change).
static LAST_EMITTED_LEVEL: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Emits This Mac on the 5 s resource cadence (`machine://this-mac`), so
/// Home, the rail and the session header refresh without a UI timer.
/// Like the resource tick, an idle app costs the UI nothing: with no AI
/// tool running, it emits only when the machine's level changes.
///
/// # Errors
///
/// As [`this_mac_impl`].
pub fn emit_this_mac(app: &tauri::AppHandle, state: &Arc<AppState>) -> AppResult<()> {
    use tauri::Emitter;
    let payload = this_mac_impl(state, crate::commands::now_millis())?;
    let level = payload.machine.summary.level.clone();
    let changed = LAST_EMITTED_LEVEL.lock().map_or(true, |mut last| {
        last.replace(level.clone()).as_ref() != Some(&level)
    });
    if payload.tools.count == 0 && !changed {
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
                memory_available: 2 * G,
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
        assert_eq!(info.memory_used, 30 * G);
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
            session_id: session.map(str::to_string),
            place: session.is_none().then_some("desktop"),
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
        collector.set_observed_roots(tools.iter().map(|tool| tool.pid).collect());
        for tick in 1..4 {
            collector.tick(&supervisor, tick * 2_000);
            std::thread::sleep(std::time::Duration::from_millis(700));
        }
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
}
