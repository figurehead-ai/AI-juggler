//! AJ-E2401..E2406 (D-41, ADR-0019): the Processes view's one model of
//! every app, session, process and subprocess on this Mac that AI Juggler
//! started or sees in a session another app runs.
//!
//! - **One source of truth.** The graph is built on the existing 2 s tick
//!   from what the tick already holds ([`observe`]): the spawn ledger, the
//!   collector's table, trees and rings, the live view's sessions and the
//!   sightings. No scan loop of its own. Cleanup, a session's Processes tab
//!   and This Mac read it, so they agree.
//! - **One vocabulary** of states ([`model::ProcState`]): Working, Idle,
//!   Left behind (also while AI Juggler was closed), Picked up after
//!   restart, Finished and waiting for its parent (a zombie), Didn't stop,
//!   Won't be touched, Ended (7 days).
//! - **Live.** A change is published as [`EVENT_CHANGED`] with the new
//!   version and counts; the page reads the rows it shows again
//!   ([`query`], paged by cursor).
//! - **Read-only for other apps.** Their sessions are observed, never in
//!   the ledger; a leftover of an ended one may be stopped only by the
//!   user's own click through the identity check against its sighting
//!   ([`stop`], D-41b).

pub mod commands;
pub mod dto;
pub mod ended;
pub mod model;
pub mod observe;
pub mod query;
pub mod roles;
pub mod rules;
pub mod sightings;
pub mod stop;

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use juggler_core::model::{MachineId, SessionId};
use juggler_core::storage::MarkKind;

use model::{Graph, Inputs};
use observe::OursMeta;

use crate::state::AppState;

/// Emitted when the graph changed: `{version, counts}`. The page reads the
/// rows it shows again.
pub const EVENT_CHANGED: &str = "processes://changed";

/// Samples kept per process for its sparkline (1 minute at 2 s).
const SPARK_SAMPLES: usize = 30;
/// Stored titles and statuses, hog facts and failed stops are read again
/// every this many ticks (10 s).
const SLOW_EVERY: u64 = 5;
/// A label from the tool's events is kept this long.
const LABEL_KEEP_MS: i64 = 3_600_000;
/// Labels kept per session.
const LABELS_PER_SESSION: usize = 40;
/// Leftovers a session left are told in one notification after this long
/// (the reconciler confirms them one by one).
const NOTIFY_AFTER_MS: i64 = 10_000;

/// What the graph remembers between ticks.
#[derive(Debug, Default)]
struct Inner {
    ticks: u64,
    machine: Option<MachineId>,
    boot_id: String,
    salt: Vec<u8>,
    sparks: HashMap<(u32, i64), VecDeque<f32>>,
    session_sparks: HashMap<String, VecDeque<f32>>,
    peaks: HashMap<(u32, i64), u64>,
    zombie_since: HashMap<(u32, i64), i64>,
    labels: HashMap<String, VecDeque<(i64, String)>>,
    sightings: sightings::Memory,
    marks: HashMap<(u32, i64), (MarkKind, Option<i64>)>,
    marks_loaded: bool,
    failed: HashMap<u32, String>,
    heavy: HashSet<u32>,
    meta: HashMap<String, OursMeta>,
    titles: HashMap<(String, String), String>,
    folders: HashMap<(String, String), std::path::PathBuf>,
    fingerprint: u64,
    /// Helpers the reconciler found left behind, by session, with when.
    orphaned: HashMap<String, (i64, u32)>,
}

/// The Processes view's state, held by [`AppState`].
#[derive(Debug, Default)]
pub struct Processes {
    inner: Mutex<Inner>,
    graph: RwLock<Arc<Graph>>,
    version: AtomicU64,
}

/// What changed, for the page (the [`EVENT_CHANGED`] payload).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProcessGraphChangedInfo {
    /// The graph's version now.
    #[ts(type = "number")]
    pub version: u64,
    /// The counts now.
    pub counts: dto::ProcessCountsInfo,
}

impl Processes {
    /// The graph as last built (empty before the first tick).
    #[must_use]
    pub fn graph(&self) -> Arc<Graph> {
        self.graph
            .read()
            .map(|graph| Arc::clone(&graph))
            .unwrap_or_default()
    }

    /// What to clean up, and how many of those hold a port, once the graph
    /// has been built (This Mac and the top bar show the same count).
    #[must_use]
    pub fn leftovers(&self) -> Option<(u32, u32)> {
        let graph = self.graph();
        if graph.built_ms == 0 {
            return None;
        }
        let now = graph.built_ms;
        let ports = graph
            .apps
            .iter()
            .flat_map(|app| app.sessions.iter())
            .flat_map(|session| session.processes.iter())
            .filter(|node| {
                node.state.to_clean()
                    && node.unmarked(now)
                    && !node.role.ours_internal()
                    && !node.ports.is_empty()
            })
            .count();
        Some((
            graph.counts.to_clean,
            u32::try_from(ports).unwrap_or(u32::MAX),
        ))
    }

    /// The graph's version (moves on with every change).
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// The boot the graph's identities are of.
    #[must_use]
    pub fn boot_id(&self) -> String {
        self.inner
            .lock()
            .map(|inner| inner.boot_id.clone())
            .unwrap_or_default()
    }

    /// The sighting of one process of another app's session, when held.
    #[must_use]
    pub fn sighting(&self, pid: u32, start: i64) -> Option<juggler_core::storage::Sighting> {
        self.inner
            .lock()
            .ok()
            .and_then(|inner| inner.sightings.get(pid, start).cloned())
    }

    /// A conversation's title, when one was read (by `(tool, id)`).
    #[must_use]
    pub fn title_of(&self, tool: &str, id: &str) -> Option<String> {
        self.inner.lock().ok().and_then(|inner| {
            inner
                .titles
                .get(&(tool.to_owned(), id.to_owned()))
                .filter(|title| !title.trim().is_empty())
                .cloned()
        })
    }

    /// The install's command-line hash key.
    #[must_use]
    pub fn salt(&self) -> Vec<u8> {
        self.inner
            .lock()
            .map(|inner| inner.salt.clone())
            .unwrap_or_default()
    }

    /// The user's mark on a process, as held (Keep, Remind me later).
    pub fn set_mark(&self, pid: u32, start: i64, mark: Option<(MarkKind, Option<i64>)>) {
        if let Ok(mut inner) = self.inner.lock() {
            match mark {
                Some(mark) => {
                    inner.marks.insert((pid, start), mark);
                }
                None => {
                    inner.marks.remove(&(pid, start));
                }
            }
            inner.fingerprint = 0; // shows at the next tick
        }
    }

    /// Whether the user said Keep for this process.
    #[must_use]
    pub fn is_kept(&self, pid: u32, start: i64) -> bool {
        self.inner
            .lock()
            .is_ok_and(|inner| matches!(inner.marks.get(&(pid, start)), Some((MarkKind::Keep, _))))
    }

    /// AJ-E2401: what started a process, from the tool's own events (a
    /// Claude hook's command, a Codex command item): "Bash: npm run dev".
    /// Held in memory for an hour; never stored.
    pub fn note_label(&self, session: &str, words: &str, now_ms: i64) {
        let words: String = words.trim().chars().take(120).collect();
        if words.is_empty() {
            return;
        }
        if let Ok(mut inner) = self.inner.lock() {
            let labels = inner.labels.entry(session.to_owned()).or_default();
            labels.push_back((now_ms, words));
            while labels.len() > LABELS_PER_SESSION
                || labels
                    .front()
                    .is_some_and(|(at, _)| now_ms - at > LABEL_KEEP_MS)
            {
                labels.pop_front();
            }
        }
    }

    /// AJ-E2405: the reconciler found a helper of `session` left behind
    /// (`HelperOrphaned`, no longer dropped): the graph shows it at once
    /// and the user hears of it once per session.
    pub fn helper_orphaned(&self, session: &SessionId, pid: u32, now_ms: i64) {
        if let Ok(mut inner) = self.inner.lock() {
            let entry = inner
                .orphaned
                .entry(session.as_str().to_owned())
                .or_insert((now_ms, 0));
            entry.1 = entry.1.saturating_add(1);
            let _ = pid;
            inner.fingerprint = 0;
        }
    }
}

/// Keeps the last [`SPARK_SAMPLES`] of a series.
fn push_sample(series: &mut VecDeque<f32>, value: f32) {
    series.push_back(value);
    while series.len() > SPARK_SAMPLES {
        series.pop_front();
    }
}

/// The parts of the graph a change is about: states, processes and ports,
/// CPU to the whole percent and memory to 16 MB.
fn fingerprint(graph: &Graph) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for app in &graph.apps {
        app.key.hash(&mut hasher);
        for session in &app.sessions {
            session.key.hash(&mut hasher);
            session.state.hash(&mut hasher);
            session.title.hash(&mut hasher);
            for node in &session.processes {
                node.key.hash(&mut hasher);
                node.state.hash(&mut hasher);
                node.ports.hash(&mut hasher);
                node.mark
                    .map(|(kind, until)| (kind.as_str(), until))
                    .hash(&mut hasher);
                node.cpu_pct
                    .map(|cpu| (cpu * 10.0).round().to_bits())
                    .hash(&mut hasher);
                (node.rss_bytes >> 24).hash(&mut hasher);
            }
        }
    }
    for program in &graph.others {
        program.name.hash(&mut hasher);
        program.processes.hash(&mut hasher);
        (program.rss_bytes >> 26).hash(&mut hasher);
    }
    hasher.finish().max(1)
}

/// The failed stops of the last day still worth showing: the audit's
/// outcomes that weren't a success, by pid.
fn failed_stops(state: &Arc<AppState>, now_ms: i64) -> HashMap<u32, String> {
    let since = now_ms - 24 * 3_600_000;
    let mut out = HashMap::new();
    let Ok(rows) = state.audit.history(since) else {
        return out;
    };
    // Newest first: a later success clears an earlier failure.
    let mut decided: HashSet<u32> = HashSet::new();
    for row in rows {
        if !decided.insert(row.pid) {
            continue;
        }
        if row.outcome.contains("refused") {
            out.insert(row.pid, row.outcome.clone());
        }
    }
    out
}

/// One tick of the graph (on the 2 s tick, after the collector's): reads
/// what the tick holds, notes sightings, rebuilds the graph and returns
/// what changed, if anything.
#[allow(clippy::too_many_lines)] // one pass in the tick's order; each step named
pub fn tick(state: &Arc<AppState>) -> Option<ProcessGraphChangedInfo> {
    let now = crate::commands::now_millis();
    let (ticks, machine, first) = {
        let mut inner = state.processes.inner.lock().ok()?;
        inner.ticks = inner.ticks.wrapping_add(1);
        (inner.ticks, inner.machine.clone(), inner.ticks == 1)
    };
    let machine = machine.unwrap_or_else(|| crate::commands::local_machine_id(state));
    let slow = first || ticks.is_multiple_of(SLOW_EVERY);
    let boot_id = state.ledger.boot_id().to_owned();
    // The ledger's live rows and the supervisor's roots.
    let rows = state.ledger.live_rows(&machine).unwrap_or_default();
    let roots = state.supervisor.live_roots();
    let adopted: HashSet<String> = state
        .supervisor
        .alive_adopted_sessions()
        .into_iter()
        .map(|session| session.as_str().to_owned())
        .collect();
    let mut sessions: Vec<SessionId> = roots
        .iter()
        .map(|(session, _, _)| session.clone())
        .collect();
    sessions.sort();
    sessions.dedup();
    let ledger_pids: HashSet<u32> = rows.iter().filter_map(|row| row.pid).collect();
    let model_now = state.conversations.live_model();
    // What the stored rows, the hog rules and the audit say (every 10 s).
    let slow_reads = slow.then(|| {
        let mut ids: Vec<String> = rows.iter().map(|row| row.session_id.clone()).collect();
        ids.extend(sessions.iter().map(|session| session.as_str().to_owned()));
        ids.sort();
        ids.dedup();
        let meta = observe::ours_meta(state, &ids);
        let heavy: HashSet<u32> = crate::commands::hog_facts(state, now)
            .into_iter()
            .filter(|hog| hog.pid != 0)
            .map(|hog| hog.pid)
            .collect();
        (meta, heavy, failed_stops(state, now))
    });
    // Leftover pids of other apps' sessions are read from the table too.
    let leftover_pids: Vec<(u32, i64)> = state
        .processes
        .inner
        .lock()
        .map(|inner| inner.sightings_pids())
        .unwrap_or_default();
    let mut wanted: HashSet<u32> = ledger_pids.clone();
    wanted.extend(leftover_pids.iter().map(|(pid, _)| *pid));
    let view = observe::collector_view(state, &sessions, &wanted);
    let mut ours_pids: HashSet<u32> = ledger_pids;
    for (tree, _) in view.trees.values() {
        ours_pids.extend(tree.iter().map(|live| live.pid));
    }
    let table = state.supervisor.snapshot_table();
    let alive = |pid: u32, start: i64| -> Option<model::Live> {
        let facts = table.as_ref()?.facts.get(&pid)?;
        (facts.start_time == start).then(|| {
            view.live.get(&pid).cloned().unwrap_or_else(|| model::Live {
                pid,
                start_time: facts.start_time,
                parent_pid: facts.ppid,
                name: facts.name.clone(),
                cpu_pct: None,
                rss_bytes: facts.rss_bytes,
                cpu_time_ms: facts.cpu_time_ms,
                zombie: facts.zombie,
            })
        })
    };
    // First tick: the stored sightings and marks of this boot, the key.
    let stored = first.then(|| {
        state.storage.lock().ok().map(|storage| {
            let since = now - juggler_core::storage::SIGHTING_KEEP_MS;
            (
                storage
                    .sightings()
                    .seen_since(&boot_id, since)
                    .unwrap_or_default(),
                storage.sightings().marks(&boot_id, now).unwrap_or_default(),
                sightings::salt(&storage),
            )
        })
    });
    let mut titles_wanted: HashSet<(String, String)> = model_now
        .entries
        .iter()
        .filter(|entry| {
            entry
                .app_title
                .as_deref()
                .is_none_or(|title| title.trim().is_empty())
        })
        .map(|entry| (entry.tool.clone(), entry.session_id.clone()))
        .collect();
    if let Some((meta, _, _)) = slow_reads.as_ref() {
        for row in &rows {
            if let Some(conversation) = meta
                .get(&row.session_id)
                .and_then(|meta| meta.conversation.clone())
            {
                titles_wanted.insert((row.provider.clone(), conversation));
            }
        }
    }
    if let Ok(inner) = state.processes.inner.lock() {
        titles_wanted.extend(inner.sightings.sessions());
    }
    let fresh_titles = slow.then(|| {
        state
            .conversations
            .titles_of(&titles_wanted)
            .into_iter()
            .map(|(key, (title, folder))| {
                let words = title.filter(|t| !t.trim().is_empty()).or_else(|| {
                    folder
                        .as_ref()
                        .and_then(|f| f.file_name().map(|n| n.to_string_lossy().into_owned()))
                });
                (key, (words.unwrap_or_default(), folder))
            })
            .collect::<HashMap<_, _>>()
    });
    let (graph, writes, extra, prune, notify) = {
        let mut inner = state.processes.inner.lock().ok()?;
        inner.machine = Some(machine);
        inner.boot_id.clone_from(&boot_id);
        if let Some(Some((sighted, marks, salt))) = stored {
            inner.sightings.load(sighted, now);
            for mark in marks {
                inner
                    .marks
                    .insert((mark.pid, mark.start_time), (mark.kind, mark.until_ms));
            }
            inner.marks_loaded = true;
            inner.salt = salt;
        }
        if let Some((meta, heavy, failed)) = slow_reads {
            inner.meta = meta;
            inner.heavy = heavy;
            inner.failed = failed;
        }
        if let Some(titles) = fresh_titles {
            for (key, (title, folder)) in titles {
                if let Some(folder) = folder {
                    inner.folders.insert(key.clone(), folder);
                }
                inner.titles.insert(key, title);
            }
        }
        let (theirs, mut extra) =
            observe::theirs_sessions(&model_now, &ours_pids, &view, &inner.titles, &inner.folders);
        let live_keys: HashSet<(String, String)> = model_now
            .entries
            .iter()
            .map(|entry| (entry.tool.clone(), entry.session_id.clone()))
            .collect();
        let salt = inner.salt.clone();
        let noted = inner.sightings.note(
            &theirs,
            &view.commands,
            &alive,
            &live_keys,
            &salt,
            &boot_id,
            now,
        );
        // Leftovers are measured from the listing: their top processes.
        let leftover_set: HashSet<u32> = noted.leftovers.iter().map(|l| l.live.pid).collect();
        for leftover in &noted.leftovers {
            let top = leftover
                .live
                .parent_pid
                .is_none_or(|parent| !leftover_set.contains(&parent));
            if top {
                extra.push(juggler_supervisor::ObservedRoot {
                    pid: leftover.live.pid,
                    start_time: Some(leftover.live.start_time),
                });
            }
        }
        // Leftovers' CPU and children from their measured trees.
        let mut leftovers = noted.leftovers;
        for leftover in &mut leftovers {
            if let Some(row) = view
                .extra
                .values()
                .flat_map(|tree| tree.rows.iter())
                .find(|row| {
                    row.pid == leftover.live.pid && row.start_time == leftover.live.start_time
                })
            {
                leftover.live = observe::live_of(row);
            }
        }
        let ours =
            observe::ours_sessions(rows, &roots, &adopted, &inner.meta, &inner.titles, &view);
        let labels: HashMap<String, Vec<(i64, String)>> = inner
            .labels
            .iter()
            .map(|(session, labels)| (session.clone(), labels.iter().cloned().collect()))
            .collect();
        let mut live = view.live.clone();
        for leftover in &leftovers {
            live.entry(leftover.live.pid)
                .or_insert_with(|| leftover.live.clone());
        }
        let commands: HashMap<u32, (String, String)> = view
            .commands
            .iter()
            .map(|(pid, (exe, cmd, _))| (*pid, (exe.clone(), cmd.clone())))
            .collect();
        let mut inputs = Inputs {
            now_ms: now,
            live,
            ours,
            theirs,
            leftovers,
            ports: view.ports.clone(),
            commands,
            marks: inner.marks.clone(),
            labels,
            failed_stops: inner.failed.clone(),
            heavy: inner.heavy.clone(),
            sparks: HashMap::new(),
            peaks: inner.peaks.clone(),
            zombie_since: inner.zombie_since.clone(),
            own_pid: std::process::id(),
            titles: inner.titles.clone(),
            folders: inner.folders.clone(),
            others: view.others.clone(),
            subagents: HashMap::new(),
        };
        // AJ-E1520 (0.3): what each session's subagents ran, for "started
        // by subagent" (only sessions whose readers are kept: no read).
        let mut subagents = HashMap::new();
        for session in &inputs.ours {
            let conversation = inner
                .meta
                .get(&session.session_id)
                .and_then(|meta| meta.conversation.as_deref());
            if let (Some(account), Some(conversation)) =
                (session.account_id.as_deref(), conversation)
            {
                let commands = state.subagents.commands_of(account, conversation);
                if !commands.is_empty() {
                    subagents.insert(session.session_id.clone(), commands);
                }
            }
        }
        for session in &inputs.theirs {
            if let Some(account) = session.account_id.as_deref() {
                let commands = state.subagents.commands_of(account, &session.session_id);
                if !commands.is_empty() {
                    subagents.insert(session.session_id.clone(), commands);
                }
            }
        }
        inputs.subagents = subagents;
        // Each process's CPU now joins its sparkline before the build.
        let mut seen: HashSet<(u32, i64)> = HashSet::new();
        let mut note = |pid: u32, start: i64, cpu: Option<f64>, rss: u64, zombie: bool| {
            seen.insert((pid, start));
            push_sample(
                inner.sparks.entry((pid, start)).or_default(),
                model::f32_of(cpu.unwrap_or(0.0)),
            );
            let peak = inner.peaks.entry((pid, start)).or_insert(0);
            *peak = (*peak).max(rss);
            if zombie {
                inner.zombie_since.entry((pid, start)).or_insert(now);
            }
        };
        for session in &inputs.ours {
            for live in &session.tree {
                note(
                    live.pid,
                    live.start_time,
                    live.cpu_pct,
                    live.rss_bytes,
                    live.zombie,
                );
            }
            for row in &session.rows {
                if let Some(pid) = row.pid
                    && let Some(live) = inputs.live.get(&pid)
                    && !session.tree.iter().any(|l| l.pid == pid)
                {
                    note(
                        pid,
                        live.start_time,
                        live.cpu_pct,
                        live.rss_bytes,
                        live.zombie,
                    );
                }
            }
        }
        for session in &inputs.theirs {
            for live in &session.tree {
                note(
                    live.pid,
                    live.start_time,
                    live.cpu_pct,
                    live.rss_bytes,
                    live.zombie,
                );
            }
        }
        for leftover in &inputs.leftovers {
            let live = &leftover.live;
            note(
                live.pid,
                live.start_time,
                live.cpu_pct,
                live.rss_bytes,
                live.zombie,
            );
        }
        inner.sparks.retain(|identity, _| seen.contains(identity));
        inner.peaks.retain(|identity, _| seen.contains(identity));
        inner
            .zombie_since
            .retain(|identity, _| seen.contains(identity));
        inputs.sparks = inner
            .sparks
            .iter()
            .map(|(identity, series)| (*identity, series.iter().copied().collect()))
            .collect();
        inputs.peaks.clone_from(&inner.peaks);
        inputs.zombie_since.clone_from(&inner.zombie_since);
        let mut graph = model::build(&inputs);
        // A session's sparkline: its tree's ring, else the graph's own.
        let mut keep_sessions: HashSet<String> = HashSet::new();
        for app in &mut graph.apps {
            for session in &mut app.sessions {
                keep_sessions.insert(session.key.clone());
                let series = inner.session_sparks.entry(session.key.clone()).or_default();
                push_sample(series, model::f32_of(session.rollup.cpu_pct));
                if session.spark.is_empty() {
                    session.spark = series.iter().copied().collect();
                }
            }
        }
        inner
            .session_sparks
            .retain(|key, _| keep_sessions.contains(key));
        let prune = now - inner.sightings.pruned_ms >= sightings::PRUNE_EVERY_MS;
        if prune {
            inner.sightings.pruned_ms = now;
        }
        // Leftovers told once per session, when the reconciler is done.
        let due: Vec<(String, u32)> = inner
            .orphaned
            .iter()
            .filter(|(_, (at, _))| now - at >= NOTIFY_AFTER_MS)
            .map(|(session, (_, count))| (session.clone(), *count))
            .collect();
        for (session, _) in &due {
            inner.orphaned.remove(session);
        }
        (graph, noted.writes, extra, prune, due)
    };
    if (!writes.is_empty() || prune)
        && let Ok(storage) = state.storage.lock()
    {
        let repo = storage.sightings();
        let _ = repo.record(&writes);
        if prune {
            let _ = repo.prune(now - juggler_core::storage::SIGHTING_KEEP_MS, now);
        }
    }
    if let Ok(mut collector) = state.resources.lock() {
        collector.set_observed_extra(extra);
    }
    for (session, _) in &notify {
        let left = graph
            .apps
            .iter()
            .flat_map(|app| app.sessions.iter())
            .find(|node| node.session_id == *session)
            .map_or(0, |node| node.rollup.to_clean);
        if left > 0 {
            crate::features::notifications::left_behind(state, session, left);
        }
    }
    let print = fingerprint(&graph);
    let counts = dto::ProcessCountsInfo::of(&graph.counts);
    let changed = {
        let mut inner = state.processes.inner.lock().ok()?;
        let changed = inner.fingerprint != print;
        inner.fingerprint = print;
        changed
    };
    if let Ok(mut slot) = state.processes.graph.write() {
        *slot = Arc::new(graph);
    }
    if !changed {
        return None;
    }
    let version = state.processes.version.fetch_add(1, Ordering::AcqRel) + 1;
    Some(ProcessGraphChangedInfo { version, counts })
}

/// AJ-E2405/E2407: the reconciler found `pid` of `session` left behind
/// (`HelperOrphaned`): counted for usage statistics (its category only,
/// when the user shares them), shown at once, told once per session.
pub fn helper_orphaned(state: &Arc<AppState>, session: &SessionId, pid: u32) {
    use juggler_supervisor::tree::LedgerStore as _;
    let row = state
        .ledger
        .session_rows(session)
        .ok()
        .and_then(|rows| rows.into_iter().find(|row| row.pid == Some(pid)));
    let (exe, cmdline) = row.map_or((String::new(), String::new()), |row| {
        (row.exe_path, row.cmdline_trunc)
    });
    crate::usage_stats::count(
        "leftover.detected",
        &[(
            "category",
            crate::usage_stats::leftover_category(&exe, &cmdline),
        )],
    );
    state
        .processes
        .helper_orphaned(session, pid, crate::commands::now_millis());
}

impl Inner {
    /// The identities of the held sightings whose process may still run.
    fn sightings_pids(&self) -> Vec<(u32, i64)> {
        self.sightings.identities()
    }
}
