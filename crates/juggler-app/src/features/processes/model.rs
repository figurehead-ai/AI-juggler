//! AJ-E2401: the process graph, built purely from what the 2 s tick
//! already holds (the ledger's rows, the collector's table and trees, the
//! live sessions, the sightings): app → session → process → subprocess,
//! each process in one vocabulary of states, with its facts and each
//! session's and app's totals. No OS call and no storage read happen here;
//! [`super::observe`] gathers the inputs.

use std::collections::{BTreeSet, HashMap, HashSet};

use juggler_core::storage::{LedgerRow, MarkKind, Sighting};

use super::roles::{Role, role_of, short_name};

/// A process's state, in the one vocabulary every surface uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProcState {
    /// It didn't stop when asked (a failed stop, or a session marked
    /// ended whose own process still runs).
    DidntStop,
    /// Left behind: still running after its session closed.
    LeftBehind,
    /// Left behind while AI Juggler was closed.
    LeftBehindClosed,
    /// Finished, waiting for its parent to collect it (a zombie).
    Zombie,
    /// AI Juggler can't confirm it is the process it started: never touched.
    WontTouch,
    /// Running, picked up again after AI Juggler restarted.
    PickedUp,
    /// Running and busy.
    Working,
    /// Running and quiet.
    Idle,
    /// Ended (kept 7 days).
    Ended,
}

impl ProcState {
    /// The stable word the UI reads.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::DidntStop => "didnt_stop",
            Self::LeftBehind => "left_behind",
            Self::LeftBehindClosed => "left_behind_closed",
            Self::Zombie => "zombie",
            Self::WontTouch => "wont_touch",
            Self::PickedUp => "picked_up",
            Self::Working => "working",
            Self::Idle => "idle",
            Self::Ended => "ended",
        }
    }

    /// The state from its word.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        [
            Self::DidntStop,
            Self::LeftBehind,
            Self::LeftBehindClosed,
            Self::Zombie,
            Self::WontTouch,
            Self::PickedUp,
            Self::Working,
            Self::Idle,
            Self::Ended,
        ]
        .into_iter()
        .find(|state| state.word() == word)
    }

    /// Something to clean up: left behind, or didn't stop (what the top
    /// bar counts).
    #[must_use]
    pub const fn to_clean(self) -> bool {
        matches!(
            self,
            Self::DidntStop | Self::LeftBehind | Self::LeftBehindClosed
        )
    }

    /// Alive (everything but Ended).
    #[must_use]
    pub const fn alive(self) -> bool {
        !matches!(self, Self::Ended)
    }
}

/// Whose a session or process is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Started by AI Juggler (a ledger root and its children).
    Ours,
    /// In a session another app runs (read-only, D-41).
    Theirs,
}

/// How a process may be stopped, if at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StopPath {
    /// The ledger's verified stop (the kill sheet).
    Ours,
    /// D-41b: the user's verified stop of another app's leftover.
    Observed,
    /// Not offered, and why (in words).
    Never(&'static str),
}

/// Why a process isn't offered for stopping, in words.
pub mod never {
    /// A zombie.
    pub const ZOMBIE: &str = "Only its parent can clear it";
    /// The ledger can't confirm it.
    pub const UNVERIFIED: &str = "AI Juggler can't confirm it's the process it started";
    /// AI Juggler's own.
    pub const OWN: &str = "Part of AI Juggler";
    /// Another app's session still runs.
    pub const SESSION_RUNS: &str = "Its session is still running in its app";
    /// A session's own process in another app.
    pub const THEIR_TOOL: &str = "The session itself: stop it in its app";
    /// Ended.
    pub const ENDED: &str = "It has ended";
}

/// One process's live facts, from the collector's table or a tree's row.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Live {
    /// Its pid.
    pub pid: u32,
    /// Its kernel start time, epoch seconds.
    pub start_time: i64,
    /// Its parent, when known.
    pub parent_pid: Option<u32>,
    /// Its name.
    pub name: String,
    /// CPU now, percent of one core, when measured.
    pub cpu_pct: Option<f64>,
    /// Memory now, bytes.
    pub rss_bytes: u64,
    /// CPU time used, ms.
    pub cpu_time_ms: u64,
    /// A zombie.
    pub zombie: bool,
}

/// A session AI Juggler started, as the tick holds it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OursInput {
    /// The supervisor's session id.
    pub session_id: String,
    /// Its tool (adapter id).
    pub tool: String,
    /// Its account.
    pub account_id: Option<String>,
    /// Its project.
    pub project_id: Option<String>,
    /// Its title, as best known (the conversation's, else its folder).
    pub title: String,
    /// Its own process has ended (the supervisor saw it exit).
    pub finished: bool,
    /// Picked up again after AI Juggler restarted.
    pub adopted: bool,
    /// The session's stored status says it ended (stopped, finished, …).
    pub marked_ended: bool,
    /// Busy now (a turn running), when the session says.
    pub busy: Option<bool>,
    /// The tree's CPU samples, oldest first (the session's ring).
    pub ring: Vec<f32>,
    /// Its rows in the ledger that are alive (running, left behind,
    /// unverifiable).
    pub rows: Vec<LedgerRow>,
    /// Its live tree from the collector (processes the ledger may not
    /// hold yet).
    pub tree: Vec<Live>,
}

/// The app a session runs in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppRef {
    /// The registry's id for it, when known.
    pub app_id: Option<String>,
    /// Its name.
    pub name: String,
    /// Its main process.
    pub pid: Option<u32>,
    /// `desktop`, `terminal` or `editor`, when known (its icon's kind).
    pub kind: Option<String>,
}

/// A session another app runs, as the live view and the collector hold it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TheirsInput {
    /// Its tool (adapter id).
    pub tool: String,
    /// The tool's id for it.
    pub session_id: String,
    /// The user's account it runs on, when known.
    pub account_id: Option<String>,
    /// Its title, as best known.
    pub title: String,
    /// Where it runs.
    pub app: AppRef,
    /// Its own process.
    pub root_pid: u32,
    /// That process's start time.
    pub root_start: i64,
    /// Busy now, when its files say.
    pub busy: Option<bool>,
    /// The tree's CPU samples, oldest first.
    pub ring: Vec<f32>,
    /// Its processes (root first), measured.
    pub tree: Vec<Live>,
}

/// A process of another app's session that outlived the session.
#[derive(Clone, Debug, PartialEq)]
pub struct LeftoverInput {
    /// What was seen of it.
    pub sighting: Sighting,
    /// How it is now.
    pub live: Live,
}

/// Everything the graph is built from.
#[derive(Clone, Debug, Default)]
pub struct Inputs {
    /// Now, epoch ms.
    pub now_ms: i64,
    /// Every process's live facts (the collector's table), by pid.
    pub live: HashMap<u32, Live>,
    /// The sessions AI Juggler started.
    pub ours: Vec<OursInput>,
    /// The sessions other apps run.
    pub theirs: Vec<TheirsInput>,
    /// Other apps' sessions' leftovers.
    pub leftovers: Vec<LeftoverInput>,
    /// Listening ports, by pid.
    pub ports: HashMap<u32, Vec<u16>>,
    /// Program and command line by pid (role words only; never kept).
    pub commands: HashMap<u32, (String, String)>,
    /// The user's marks, by `(pid, start)`.
    pub marks: HashMap<(u32, i64), (MarkKind, Option<i64>)>,
    /// What started a process, from the tool's own events, by session:
    /// `(when, words)` newest last ("Bash: npm run dev").
    pub labels: HashMap<String, Vec<(i64, String)>>,
    /// Pids whose last stop failed, with the outcome.
    pub failed_stops: HashMap<u32, String>,
    /// Heavy pids (the hog rules).
    pub heavy: HashSet<u32>,
    /// Each process's recent CPU, oldest first, by `(pid, start)`.
    pub sparks: HashMap<(u32, i64), Vec<f32>>,
    /// Each process's peak memory seen, by `(pid, start)`.
    pub peaks: HashMap<(u32, i64), u64>,
    /// When each zombie was first seen, by `(pid, start)`.
    pub zombie_since: HashMap<(u32, i64), i64>,
    /// AI Juggler's own pid (never shown).
    pub own_pid: u32,
    /// Other apps' sessions' titles, by `(tool, session id)`, for the
    /// ones that ended (their leftovers' sessions).
    pub titles: HashMap<(String, String), String>,
}

/// One process in the graph.
#[derive(Clone, Debug, PartialEq)]
pub struct ProcNode {
    /// `p:<pid>:<start>`.
    pub key: String,
    /// Its pid.
    pub pid: u32,
    /// Its start time, epoch seconds.
    pub start_time: i64,
    /// Its parent pid.
    pub parent_pid: Option<u32>,
    /// Indent under its session (0: directly under it).
    pub depth: u8,
    /// Its short name (`node (vite)`).
    pub name: String,
    /// Its role.
    pub role: Role,
    /// What started it, from the tool's own events, when known.
    pub label: Option<String>,
    /// Its state.
    pub state: ProcState,
    /// CPU now, percent of one core.
    pub cpu_pct: Option<f64>,
    /// Recent CPU, oldest first.
    pub spark: Vec<f32>,
    /// Memory now, bytes.
    pub rss_bytes: u64,
    /// The most memory seen, bytes.
    pub peak_rss: u64,
    /// Listening ports.
    pub ports: Vec<u16>,
    /// Age, ms.
    pub age_ms: i64,
    /// CPU time used, ms.
    pub cpu_time_ms: u64,
    /// Whose.
    pub origin: Origin,
    /// For a zombie: its parent's pid and name (only it can clear it).
    pub waits_for: Option<(u32, String)>,
    /// How it may be stopped.
    pub stop: StopPath,
    /// The user's mark: Keep, or Remind me later until a time.
    pub mark: Option<(MarkKind, Option<i64>)>,
    /// Heavy (the hog rules).
    pub heavy: bool,
    /// Why its last stop failed, when it did.
    pub failed: Option<String>,
    /// It needs the user (set when the graph is built).
    pub attention: bool,
}

impl ProcNode {
    /// Neither kept nor waiting on a reminder.
    #[must_use]
    pub fn unmarked(&self, now_ms: i64) -> bool {
        match self.mark {
            Some((MarkKind::Keep, _)) => false,
            Some((MarkKind::Snooze, until)) => until.is_some_and(|until| until <= now_ms),
            None => true,
        }
    }

    /// Needs the user: something to clean up, a lingering zombie, or heavy
    /// (not when marked Keep, or while a reminder is pending).
    #[must_use]
    pub fn needs_attention(&self, now_ms: i64, zombie_since: Option<i64>) -> bool {
        if self.role.ours_internal() || !self.unmarked(now_ms) {
            return false;
        }
        self.state.to_clean()
            || self.heavy
            || (self.state == ProcState::Zombie
                && zombie_since.is_some_and(|since| now_ms - since >= ZOMBIE_LINGER_MS))
    }
}

/// A zombie needs the user once it has waited this long.
pub const ZOMBIE_LINGER_MS: i64 = 60_000;

/// A process counts as busy at or above this much CPU, percent of a core.
pub const BUSY_CPU_PCT: f64 = 2.0;

/// Totals over a session's or an app's processes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rollup {
    /// Processes (AI Juggler's own hidden ones excluded).
    pub processes: u32,
    /// Memory, bytes.
    pub rss_bytes: u64,
    /// CPU, percent of one core.
    pub cpu_pct: f64,
    /// Listening ports, each once.
    pub ports: BTreeSet<u16>,
    /// Processes that need the user.
    pub attention: u32,
    /// Processes to clean up.
    pub to_clean: u32,
    /// The most urgent state among them.
    pub worst: Option<ProcState>,
}

impl Rollup {
    fn add(&mut self, node: &ProcNode, attention: bool, now_ms: i64) {
        if node.role.ours_internal() {
            return;
        }
        self.processes += 1;
        self.rss_bytes += node.rss_bytes;
        self.cpu_pct += node.cpu_pct.unwrap_or(0.0);
        self.ports.extend(node.ports.iter().copied());
        if attention {
            self.attention += 1;
        }
        if node.state.to_clean() && node.unmarked(now_ms) {
            self.to_clean += 1;
        }
        self.worst = Some(self.worst.map_or(node.state, |worst| worst.min(node.state)));
    }

    fn merge(&mut self, other: &Self) {
        self.processes += other.processes;
        self.rss_bytes += other.rss_bytes;
        self.cpu_pct += other.cpu_pct;
        self.ports.extend(other.ports.iter().copied());
        self.attention += other.attention;
        self.to_clean += other.to_clean;
        self.worst = match (self.worst, other.worst) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
    }
}

/// A session in the graph.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionNode {
    /// `s:<origin>:<id>`.
    pub key: String,
    /// Whose.
    pub origin: Origin,
    /// The session's id (the supervisor's, or the tool's for theirs).
    pub session_id: String,
    /// Its tool.
    pub tool: String,
    /// Its account.
    pub account_id: Option<String>,
    /// Its project.
    pub project_id: Option<String>,
    /// Its title.
    pub title: String,
    /// Its own process still runs.
    pub alive: bool,
    /// `working`, `idle` or `ended`.
    pub state: &'static str,
    /// Its processes in tree order.
    pub processes: Vec<ProcNode>,
    /// Its totals.
    pub rollup: Rollup,
    /// The tree's recent CPU, oldest first.
    pub spark: Vec<f32>,
    /// Its own process's pid, while it runs.
    pub root_pid: Option<u32>,
}

/// An app in the graph.
#[derive(Clone, Debug, PartialEq)]
pub struct AppNode {
    /// `a:<id or name>`.
    pub key: String,
    /// The registry's id, when known.
    pub app_id: Option<String>,
    /// Its name.
    pub name: String,
    /// Its main process, when known.
    pub pid: Option<u32>,
    /// AI Juggler itself (its sessions are ours).
    pub ours: bool,
    /// `own`, `desktop`, `terminal` or `editor`, when known.
    pub kind: Option<String>,
    /// Its sessions.
    pub sessions: Vec<SessionNode>,
    /// Its totals.
    pub rollup: Rollup,
}

/// Counts over the whole graph.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Processes shown (AI Juggler's own hidden ones excluded).
    pub processes: u32,
    /// Sessions.
    pub sessions: u32,
    /// Processes needing the user.
    pub attention: u32,
    /// Processes to clean up (the top bar's count).
    pub to_clean: u32,
    /// Left behind (ours and theirs).
    pub left_behind: u32,
    /// Zombies.
    pub zombies: u32,
    /// Didn't stop.
    pub didnt_stop: u32,
    /// Won't be touched.
    pub wont_touch: u32,
}

/// The whole graph.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graph {
    /// Apps, AI Juggler first, then by name.
    pub apps: Vec<AppNode>,
    /// Counts.
    pub counts: Counts,
    /// Built at, epoch ms.
    pub built_ms: i64,
}

/// A CPU figure as a sparkline sample (percent of a core: far inside
/// `f32`'s range; the precision lost is far below a pixel).
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn f32_of(value: f64) -> f32 {
    value.clamp(0.0, 1.0e6) as f32
}

/// The name AI Juggler's own app goes by.
pub const OUR_APP: &str = "AI Juggler";

fn key_of(pid: u32, start: i64) -> String {
    format!("p:{pid}:{start}")
}

/// The words of the newest label whose command this process runs.
fn label_for(
    labels: Option<&Vec<(i64, String)>>,
    cmdline: &str,
    started_ms: i64,
) -> Option<String> {
    let labels = labels?;
    let line = cmdline.trim();
    if line.is_empty() {
        return None;
    }
    labels
        .iter()
        .rev()
        .filter(|(at, _)| *at <= started_ms + 2_000)
        .find(|(_, words)| {
            words
                .split_once(": ")
                .map(|(_, command)| command.trim())
                .is_some_and(|command| {
                    let head: String = command.chars().take(60).collect();
                    !head.is_empty() && line.contains(head.as_str())
                })
        })
        .map(|(_, words)| format!("Started by {words}"))
}

struct Ctx<'a> {
    inputs: &'a Inputs,
}

impl Ctx<'_> {
    fn spark(&self, pid: u32, start: i64) -> Vec<f32> {
        self.inputs
            .sparks
            .get(&(pid, start))
            .cloned()
            .unwrap_or_default()
    }

    fn peak(&self, pid: u32, start: i64, now: u64) -> u64 {
        self.inputs
            .peaks
            .get(&(pid, start))
            .copied()
            .unwrap_or(0)
            .max(now)
    }

    fn ports(&self, pid: u32) -> Vec<u16> {
        let mut ports = self.inputs.ports.get(&pid).cloned().unwrap_or_default();
        ports.sort_unstable();
        ports.dedup();
        ports
    }

    fn waits_for(&self, live: &Live) -> Option<(u32, String)> {
        if !live.zombie {
            return None;
        }
        let parent = live.parent_pid?;
        let name = self.inputs.live.get(&parent).map_or_else(
            || format!("process {parent}"),
            |p| short_name(&p.name, None),
        );
        Some((parent, name))
    }

    fn age(&self, start: i64) -> i64 {
        if start <= 0 {
            return 0;
        }
        (self.inputs.now_ms - start.saturating_mul(1000)).max(0)
    }
}

/// Live state from CPU alone.
fn busy_or_quiet(cpu: Option<f64>) -> ProcState {
    if cpu.is_some_and(|cpu| cpu >= BUSY_CPU_PCT) {
        ProcState::Working
    } else {
        ProcState::Idle
    }
}

/// Orders `nodes` as a tree (parents before children, depth set), each
/// node's children by pid; nodes whose parent isn't among them start at
/// depth 0 (or under `root`, when one is given and alive).
fn tree_order(mut nodes: Vec<ProcNode>, root: Option<u32>) -> Vec<ProcNode> {
    let pids: HashSet<u32> = nodes.iter().map(|node| node.pid).collect();
    let mut children: HashMap<Option<u32>, Vec<usize>> = HashMap::new();
    nodes.sort_by_key(|node| (node.start_time, node.pid));
    for (index, node) in nodes.iter().enumerate() {
        let parent = node
            .parent_pid
            .filter(|parent| pids.contains(parent) && Some(node.pid) != root)
            .or_else(|| root.filter(|root| *root != node.pid && pids.contains(root)));
        children.entry(parent).or_default().push(index);
    }
    let mut order: Vec<(usize, u8)> = Vec::with_capacity(nodes.len());
    let mut seen: HashSet<usize> = HashSet::new();
    let mut stack: Vec<(usize, u8)> = children
        .get(&None)
        .map(|tops| tops.iter().rev().map(|index| (*index, 0)).collect())
        .unwrap_or_default();
    while let Some((index, depth)) = stack.pop() {
        if !seen.insert(index) {
            continue;
        }
        order.push((index, depth));
        if let Some(kids) = children.get(&Some(nodes[index].pid)) {
            for kid in kids.iter().rev() {
                stack.push((*kid, depth.saturating_add(1)));
            }
        }
    }
    // A cycle in stale parent links: whatever wasn't reached goes last.
    for index in 0..nodes.len() {
        if !seen.contains(&index) {
            order.push((index, 0));
        }
    }
    let mut slots: Vec<Option<ProcNode>> = nodes.into_iter().map(Some).collect();
    order
        .into_iter()
        .filter_map(|(index, depth)| {
            slots[index].take().map(|mut node| {
                node.depth = depth;
                node
            })
        })
        .collect()
}

#[allow(clippy::too_many_lines)] // one session's rows, the ledger's then the tree's
fn ours_session(ctx: &Ctx<'_>, session: &OursInput) -> SessionNode {
    let inputs = ctx.inputs;
    let labels = inputs.labels.get(&session.session_id);
    let root_row = session
        .rows
        .iter()
        .find(|row| row.role == juggler_core::storage::ROLE_ROOT);
    let root_pid = root_row.and_then(|row| row.pid);
    let mut nodes: Vec<ProcNode> = Vec::new();
    let mut listed: HashSet<u32> = HashSet::new();
    for row in &session.rows {
        let Some(pid) = row.pid else { continue };
        if pid == inputs.own_pid || pid == 0 {
            continue;
        }
        let start = row.started_at.unwrap_or_default();
        let live = inputs
            .live
            .get(&pid)
            .filter(|live| start == 0 || live.start_time == start);
        let is_root = row.role == juggler_core::storage::ROLE_ROOT;
        let state = match row.state.as_str() {
            "unverifiable" => ProcState::WontTouch,
            "orphaned" if row.down_period => ProcState::LeftBehindClosed,
            "orphaned" => ProcState::LeftBehind,
            _ => {
                // Running: a process the table no longer has has ended.
                let Some(live) = live else { continue };
                if live.zombie {
                    ProcState::Zombie
                } else if inputs.failed_stops.contains_key(&pid)
                    || (is_root && session.marked_ended)
                {
                    ProcState::DidntStop
                } else if session.adopted {
                    ProcState::PickedUp
                } else {
                    busy_or_quiet(live.cpu_pct)
                }
            }
        };
        let role = role_of(&row.exe_path, &row.cmdline_trunc, is_root);
        let stop = if role.ours_internal() {
            StopPath::Never(never::OWN)
        } else {
            match state {
                ProcState::Zombie => StopPath::Never(never::ZOMBIE),
                ProcState::WontTouch => StopPath::Never(never::UNVERIFIED),
                _ => StopPath::Ours,
            }
        };
        let cpu = live.and_then(|live| live.cpu_pct);
        let rss = live.map_or_else(
            || row.peak_rss.map_or(0, i64::unsigned_abs),
            |live| live.rss_bytes,
        );
        let name = live.map_or_else(
            || super::roles::program_name(&row.exe_path).to_owned(),
            |live| live.name.clone(),
        );
        listed.insert(pid);
        nodes.push(ProcNode {
            key: key_of(pid, start),
            pid,
            start_time: start,
            parent_pid: row
                .parent_pid
                .or_else(|| live.and_then(|live| live.parent_pid)),
            depth: 0,
            name: short_name(&name, Some(&row.cmdline_trunc)),
            role,
            label: label_for(labels, &row.cmdline_trunc, start.saturating_mul(1000)),
            state,
            cpu_pct: cpu,
            spark: ctx.spark(pid, start),
            rss_bytes: rss,
            peak_rss: ctx
                .peak(pid, start, rss)
                .max(row.peak_rss.map_or(0, i64::unsigned_abs)),
            ports: {
                let mut ports = ctx.ports(pid);
                if ports.is_empty() {
                    ports = row
                        .listens_on
                        .as_deref()
                        .map(|text| {
                            text.split(',')
                                .filter_map(|p| p.trim().parse().ok())
                                .collect()
                        })
                        .unwrap_or_default();
                }
                ports
            },
            age_ms: ctx.age(start),
            cpu_time_ms: live.map_or(0, |live| live.cpu_time_ms),
            origin: Origin::Ours,
            waits_for: live.and_then(|live| ctx.waits_for(live)),
            stop,
            mark: inputs.marks.get(&(pid, start)).copied(),
            heavy: inputs.heavy.contains(&pid),
            failed: inputs.failed_stops.get(&pid).cloned(),
            attention: false,
        });
    }
    // The collector's live tree: processes the ledger doesn't hold yet
    // (its upsert runs every 2 s), shown as the session's.
    if !session.finished {
        for live in &session.tree {
            if listed.contains(&live.pid) || live.pid == inputs.own_pid {
                continue;
            }
            let (exe, cmd) = inputs
                .commands
                .get(&live.pid)
                .cloned()
                .unwrap_or_else(|| (live.name.clone(), String::new()));
            let is_root = Some(live.pid) == root_pid;
            let role = role_of(&exe, &cmd, is_root);
            let state = if live.zombie {
                ProcState::Zombie
            } else if session.adopted {
                ProcState::PickedUp
            } else {
                busy_or_quiet(live.cpu_pct)
            };
            nodes.push(ProcNode {
                key: key_of(live.pid, live.start_time),
                pid: live.pid,
                start_time: live.start_time,
                parent_pid: live.parent_pid,
                depth: 0,
                name: short_name(&live.name, Some(&cmd)),
                role,
                label: label_for(labels, &cmd, live.start_time.saturating_mul(1000)),
                state,
                cpu_pct: live.cpu_pct,
                spark: ctx.spark(live.pid, live.start_time),
                rss_bytes: live.rss_bytes,
                peak_rss: ctx.peak(live.pid, live.start_time, live.rss_bytes),
                ports: ctx.ports(live.pid),
                age_ms: ctx.age(live.start_time),
                cpu_time_ms: live.cpu_time_ms,
                origin: Origin::Ours,
                waits_for: ctx.waits_for(live),
                stop: if role.ours_internal() {
                    StopPath::Never(never::OWN)
                } else if live.zombie {
                    StopPath::Never(never::ZOMBIE)
                } else {
                    StopPath::Ours
                },
                mark: inputs.marks.get(&(live.pid, live.start_time)).copied(),
                heavy: inputs.heavy.contains(&live.pid),
                failed: None,
                attention: false,
            });
        }
    }
    let root_alive = root_pid.is_some_and(|root| {
        nodes
            .iter()
            .any(|node| node.pid == root && node.state.alive() && node.state != ProcState::Zombie)
    }) && !session.finished;
    let processes = tree_order(nodes, root_pid.filter(|_| root_alive));
    let state = if !root_alive {
        "ended"
    } else if session.busy.unwrap_or_else(|| {
        processes
            .iter()
            .any(|node| node.state == ProcState::Working)
    }) {
        "working"
    } else {
        "idle"
    };
    finish_session(
        SessionNode {
            key: format!("s:ours:{}", session.session_id),
            origin: Origin::Ours,
            session_id: session.session_id.clone(),
            tool: session.tool.clone(),
            account_id: session.account_id.clone(),
            project_id: session.project_id.clone(),
            title: session.title.clone(),
            alive: root_alive,
            state,
            processes,
            rollup: Rollup::default(),
            spark: session.ring.clone(),
            root_pid: root_pid.filter(|_| root_alive),
        },
        inputs,
    )
}

fn theirs_session(ctx: &Ctx<'_>, session: &TheirsInput) -> SessionNode {
    let inputs = ctx.inputs;
    let labels = inputs.labels.get(&session.session_id);
    let mut nodes = Vec::with_capacity(session.tree.len());
    for live in &session.tree {
        if live.pid == inputs.own_pid {
            continue;
        }
        let (exe, cmd) = inputs
            .commands
            .get(&live.pid)
            .cloned()
            .unwrap_or_else(|| (live.name.clone(), String::new()));
        let is_root = live.pid == session.root_pid;
        let role = role_of(&exe, &cmd, is_root);
        let state = if live.zombie {
            ProcState::Zombie
        } else {
            busy_or_quiet(live.cpu_pct)
        };
        let stop = if role.ours_internal() {
            StopPath::Never(never::OWN)
        } else if live.zombie {
            StopPath::Never(never::ZOMBIE)
        } else if is_root {
            StopPath::Never(never::THEIR_TOOL)
        } else {
            StopPath::Never(never::SESSION_RUNS)
        };
        nodes.push(ProcNode {
            key: key_of(live.pid, live.start_time),
            pid: live.pid,
            start_time: live.start_time,
            parent_pid: live.parent_pid,
            depth: 0,
            name: short_name(&live.name, Some(&cmd)),
            role,
            label: label_for(labels, &cmd, live.start_time.saturating_mul(1000)),
            state,
            cpu_pct: live.cpu_pct,
            spark: ctx.spark(live.pid, live.start_time),
            rss_bytes: live.rss_bytes,
            peak_rss: ctx.peak(live.pid, live.start_time, live.rss_bytes),
            ports: ctx.ports(live.pid),
            age_ms: ctx.age(live.start_time),
            cpu_time_ms: live.cpu_time_ms,
            origin: Origin::Theirs,
            waits_for: ctx.waits_for(live),
            stop,
            mark: inputs.marks.get(&(live.pid, live.start_time)).copied(),
            heavy: inputs.heavy.contains(&live.pid),
            failed: None,
            attention: false,
        });
    }
    let processes = tree_order(nodes, Some(session.root_pid));
    let state = if session.busy.unwrap_or_else(|| {
        processes
            .iter()
            .any(|node| node.state == ProcState::Working)
    }) {
        "working"
    } else {
        "idle"
    };
    finish_session(
        SessionNode {
            key: format!("s:theirs:{}:{}", session.tool, session.session_id),
            origin: Origin::Theirs,
            session_id: session.session_id.clone(),
            tool: session.tool.clone(),
            account_id: session.account_id.clone(),
            project_id: None,
            title: session.title.clone(),
            alive: true,
            state,
            processes,
            rollup: Rollup::default(),
            spark: session.ring.clone(),
            root_pid: Some(session.root_pid),
        },
        inputs,
    )
}

/// Other apps' ended sessions, from their leftovers: one session node per
/// `(tool, session)`, with the app the sighting names.
fn leftover_sessions(ctx: &Ctx<'_>) -> Vec<(AppRef, SessionNode)> {
    let inputs = ctx.inputs;
    let mut by_session: HashMap<(String, String), (AppRef, Vec<ProcNode>)> = HashMap::new();
    for leftover in &inputs.leftovers {
        let sighting = &leftover.sighting;
        let live = &leftover.live;
        if live.pid == inputs.own_pid {
            continue;
        }
        let (exe, cmd) = inputs
            .commands
            .get(&live.pid)
            .cloned()
            .unwrap_or_else(|| (sighting.exe_name.clone(), String::new()));
        let role = role_of(&exe, &cmd, sighting.session_root);
        let state = if live.zombie {
            ProcState::Zombie
        } else if sighting.session_root {
            // Its own process still runs: the session hasn't ended.
            busy_or_quiet(live.cpu_pct)
        } else {
            ProcState::LeftBehind
        };
        let stop = if role.ours_internal() {
            StopPath::Never(never::OWN)
        } else if live.zombie {
            StopPath::Never(never::ZOMBIE)
        } else if sighting.session_root {
            StopPath::Never(never::THEIR_TOOL)
        } else {
            StopPath::Observed
        };
        let node = ProcNode {
            key: key_of(live.pid, live.start_time),
            pid: live.pid,
            start_time: live.start_time,
            parent_pid: live.parent_pid,
            depth: 0,
            name: short_name(&live.name, Some(&cmd)),
            role,
            label: None,
            state,
            cpu_pct: live.cpu_pct,
            spark: ctx.spark(live.pid, live.start_time),
            rss_bytes: live.rss_bytes,
            peak_rss: ctx.peak(live.pid, live.start_time, live.rss_bytes),
            ports: ctx.ports(live.pid),
            age_ms: ctx.age(live.start_time),
            cpu_time_ms: live.cpu_time_ms,
            origin: Origin::Theirs,
            waits_for: ctx.waits_for(live),
            stop,
            mark: inputs.marks.get(&(live.pid, live.start_time)).copied(),
            heavy: inputs.heavy.contains(&live.pid),
            failed: inputs.failed_stops.get(&live.pid).cloned(),
            attention: false,
        };
        let app = AppRef {
            app_id: None,
            name: sighting
                .app
                .clone()
                .unwrap_or_else(|| "Terminal".to_owned()),
            pid: None,
            kind: None,
        };
        by_session
            .entry((sighting.tool.clone(), sighting.session_id.clone()))
            .or_insert_with(|| (app, Vec::new()))
            .1
            .push(node);
    }
    let mut out = Vec::new();
    for ((tool, session_id), (app, nodes)) in by_session {
        let processes = tree_order(nodes, None);
        out.push((
            app,
            finish_session(
                SessionNode {
                    key: format!("s:theirs:{tool}:{session_id}"),
                    origin: Origin::Theirs,
                    session_id: session_id.clone(),
                    tool: tool.clone(),
                    account_id: None,
                    project_id: None,
                    title: inputs
                        .titles
                        .get(&(tool.clone(), session_id.clone()))
                        .cloned()
                        .unwrap_or_default(),
                    alive: false,
                    state: "ended",
                    processes,
                    rollup: Rollup::default(),
                    spark: Vec::new(),
                    root_pid: None,
                },
                inputs,
            ),
        ));
    }
    out
}

fn finish_session(mut session: SessionNode, inputs: &Inputs) -> SessionNode {
    let mut rollup = Rollup::default();
    for node in &mut session.processes {
        node.attention = node.needs_attention(
            inputs.now_ms,
            inputs
                .zombie_since
                .get(&(node.pid, node.start_time))
                .copied(),
        );
        rollup.add(node, node.attention, inputs.now_ms);
    }
    session.rollup = rollup;
    session
}

/// Attention first, then working, then by title.
fn session_rank(session: &SessionNode) -> (u8, u8, String) {
    (
        u8::from(session.rollup.attention == 0),
        match session.state {
            "working" => 0,
            "idle" => 1,
            _ => 2,
        },
        session.title.to_lowercase(),
    )
}

/// Builds the graph.
#[must_use]
#[allow(clippy::too_many_lines)] // apps, then sessions, then the counts, in order
pub fn build(inputs: &Inputs) -> Graph {
    let ctx = Ctx { inputs };
    let mut apps: Vec<AppNode> = Vec::new();
    let mut ours_app = AppNode {
        key: "a:ai-juggler".into(),
        app_id: None,
        name: OUR_APP.into(),
        pid: Some(inputs.own_pid),
        ours: true,
        kind: Some("own".into()),
        sessions: Vec::new(),
        rollup: Rollup::default(),
    };
    for session in &inputs.ours {
        let node = ours_session(&ctx, session);
        if node.processes.is_empty() {
            continue;
        }
        ours_app.sessions.push(node);
    }
    if !ours_app.sessions.is_empty() {
        apps.push(ours_app);
    }
    let mut theirs: HashMap<String, AppNode> = HashMap::new();
    let app_key = |app: &AppRef| {
        format!(
            "a:{}",
            app.app_id
                .clone()
                .unwrap_or_else(|| app.name.to_lowercase())
        )
    };
    let mut seen_sessions: HashSet<String> = HashSet::new();
    for session in &inputs.theirs {
        let node = theirs_session(&ctx, session);
        seen_sessions.insert(node.key.clone());
        let key = app_key(&session.app);
        theirs
            .entry(key.clone())
            .or_insert_with(|| AppNode {
                key,
                app_id: session.app.app_id.clone(),
                name: session.app.name.clone(),
                pid: session.app.pid,
                ours: false,
                kind: session.app.kind.clone(),
                sessions: Vec::new(),
                rollup: Rollup::default(),
            })
            .sessions
            .push(node);
    }
    for (app, node) in leftover_sessions(&ctx) {
        // A session still listed live holds its own processes already.
        if seen_sessions.contains(&node.key) {
            continue;
        }
        let key = theirs
            .iter()
            .find(|(_, known)| known.name == app.name)
            .map_or_else(|| app_key(&app), |(key, _)| key.clone());
        theirs
            .entry(key.clone())
            .or_insert_with(|| AppNode {
                key,
                app_id: None,
                name: app.name.clone(),
                pid: None,
                ours: false,
                kind: app.kind.clone(),
                sessions: Vec::new(),
                rollup: Rollup::default(),
            })
            .sessions
            .push(node);
    }
    let mut theirs: Vec<AppNode> = theirs.into_values().collect();
    theirs.sort_by_key(|app| app.name.to_lowercase());
    apps.extend(theirs);
    let mut counts = Counts::default();
    for app in &mut apps {
        app.sessions.sort_by_key(session_rank);
        let mut rollup = Rollup::default();
        for session in &app.sessions {
            rollup.merge(&session.rollup);
            counts.sessions += 1;
            for node in &session.processes {
                if node.role.ours_internal() {
                    continue;
                }
                counts.processes += 1;
                match node.state {
                    ProcState::LeftBehind | ProcState::LeftBehindClosed => counts.left_behind += 1,
                    ProcState::Zombie => counts.zombies += 1,
                    ProcState::DidntStop => counts.didnt_stop += 1,
                    ProcState::WontTouch => counts.wont_touch += 1,
                    _ => {}
                }
            }
        }
        counts.attention += rollup.attention;
        counts.to_clean += rollup.to_clean;
        app.rollup = rollup;
    }
    Graph {
        apps,
        counts,
        built_ms: inputs.now_ms,
    }
}
