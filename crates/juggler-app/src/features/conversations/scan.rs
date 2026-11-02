//! AJ-E2101: the live view's refresher. Which sessions are open, where,
//! and in what state, from the tools' own per-session files joined with
//! the process table (never an environment, D-37).
//!
//! Cheap by construction, off the main thread (the app's live thread):
//! - every [`TICK`] the small per-session files are listed, and only a
//!   file whose size or time changed is read again;
//! - a session's pid counts only while its recorded start time matches
//!   the process's own (pids are reused); known pids are checked every
//!   [`ALIVE_EVERY`], and a new pid's parent chain is read pid by pid;
//! - the whole process table is read at most every [`FULL_EVERY`];
//! - the files an app's session server holds open are asked for every
//!   [`HELD_EVERY`].
//!
//! A file of another shape gives nothing: the session is then what the
//! pid files alone say, never an older value.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use juggler_adapter::SharedAdapter;
use juggler_adapter::live::{AppSession, LiveSession, LiveStatus, TurnOutcome, same_start};

use super::places::{Host, HostRegistry, RunningApp, host_of, running_apps, running_instances};
use super::procs::{Proc, Table};

/// How often the per-session files are looked at.
pub const TICK: Duration = Duration::from_secs(2);
/// How often known pids are checked.
pub const ALIVE_EVERY: Duration = Duration::from_secs(5);
/// How often the whole process table is read, at most.
pub const FULL_EVERY: Duration = Duration::from_secs(60);
/// How often a session server's open files are asked for.
pub const HELD_EVERY: Duration = Duration::from_secs(15);
/// The most processes whose open files one pass asks for.
const HELD_MAX: usize = 8;

/// What the scan needs from the OS (a seam: the tests script it).
pub trait Os {
    /// Every process.
    fn table(&self) -> Table;
    /// Only these processes (a gone pid is absent).
    fn some(&self, pids: &[u32]) -> Table;
    /// The files a process holds open, when they can be listed.
    fn open_files(&self, pid: u32) -> Option<Vec<PathBuf>>;
}

/// The real OS: sysinfo for processes (no environments), `lsof` for a
/// process's open files.
#[derive(Clone, Copy, Debug, Default)]
pub struct RealOs;

impl Os for RealOs {
    fn table(&self) -> Table {
        super::procs::table()
    }

    fn some(&self, pids: &[u32]) -> Table {
        super::procs::some(pids)
    }

    fn open_files(&self, pid: u32) -> Option<Vec<PathBuf>> {
        open_files(pid)
    }
}

/// The paths `pid` holds open, from `/usr/sbin/lsof` (names only), within
/// two seconds; `None` when it can't say.
fn open_files(pid: u32) -> Option<Vec<PathBuf>> {
    let mut child = std::process::Command::new("/usr/sbin/lsof")
        .args(["-w", "-n", "-P", "-Fn", "-p", &pid.to_string()])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                // poll: a child's exit, bounded by the 2 s deadline above.
                std::thread::sleep(Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let out = child.wait_with_output().ok()?;
    Some(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| line.strip_prefix('n'))
            .filter(|name| name.starts_with('/'))
            .map(PathBuf::from)
            .collect(),
    )
}

/// A folder whose per-session files are read: one tool's profile, and the
/// account it is, when it is one of the user's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// The tool (its adapter's id).
    pub tool: String,
    /// The folder, as the OS resolves it.
    pub dir: PathBuf,
    /// The account it is.
    pub account_id: Option<String>,
}

/// One place a session is open: the process running it and its app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    /// The process running it.
    pub pid: u32,
    /// Its start time.
    pub start: Option<i64>,
    /// The app it runs in, when its parent chain names one.
    pub host: Option<Host>,
    /// The profile folder it uses.
    pub profile: Option<PathBuf>,
}

/// A session's state, in E21's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// It waits on the user (a question, a permission).
    AsksYou,
    /// A turn is running.
    Working,
    /// Waiting for the next thing.
    Idle,
    /// It finished something to look at.
    DoneReview,
}

impl State {
    /// The word the UI reads.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::AsksYou => "asks_you",
            Self::Working => "working",
            Self::Idle => "idle",
            Self::DoneReview => "done_review",
        }
    }
}

/// The state from what the tool recorded: a session the tool says waits
/// on the user Asks you (AJ-E2107); a running turn is Working; otherwise
/// the app's last-turn word decides (blocked: Asks you; review ready:
/// Done, review it; completed: Idle); idle alone is Idle; nothing known is
/// nothing.
#[must_use]
pub fn state_of(status: LiveStatus, outcome: Option<TurnOutcome>) -> Option<State> {
    match (status, outcome) {
        (LiveStatus::Busy, _) => Some(State::Working),
        (LiveStatus::Waiting, _) | (_, Some(TurnOutcome::Blocked)) => Some(State::AsksYou),
        (_, Some(TurnOutcome::ReviewReady)) => Some(State::DoneReview),
        (_, Some(TurnOutcome::Completed)) | (LiveStatus::Idle, None) => Some(State::Idle),
        (LiveStatus::Unknown, None) => None,
    }
}

/// One open conversation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveEntry {
    /// Its tool.
    pub tool: String,
    /// The account whose profile it runs in, when known.
    pub account_id: Option<String>,
    /// The tool's id for it.
    pub session_id: String,
    /// Where it is open (two places: open in two apps).
    pub places: Vec<Place>,
    /// Its state, when known.
    pub state: Option<State>,
    /// AJ-E2107: the state was worked out from the transcript's tail, not
    /// recorded by the tool ("probably").
    pub probably: bool,
    /// When the user last had it in front in its app, epoch ms.
    pub viewed_ms: Option<i64>,
    /// Its title in its app's sidebar.
    pub app_title: Option<String>,
    /// A headless run, not an interactive session.
    pub background: bool,
}

/// What is open now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LiveModel {
    /// Every open conversation, by tool, account and id.
    pub entries: Vec<LiveEntry>,
    /// Sessions in a running app's sidebar (not archived) that no process
    /// runs now: `(tool, id)`.
    pub inactive: Vec<(String, String)>,
    /// Each verified session pid's profile folder.
    pub pid_dirs: BTreeMap<u32, PathBuf>,
    /// 0.3.4: the sessions each session started, as a running app records
    /// them (the Claude app's `spawnedFrom`; archived ones left out, as
    /// its sidebar does).
    pub started: Vec<StartedLink>,
}

/// One session another started, from its app's own record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartedLink {
    /// Its tool.
    pub tool: String,
    /// The conversation it was started from.
    pub parent: String,
    /// Its own conversation.
    pub session_id: String,
    /// Its title in the app's sidebar.
    pub title: Option<String>,
    /// Its last activity, epoch ms.
    pub last_activity_ms: Option<i64>,
}

/// Each app session started from another (both named by their app ids),
/// as conversation ids: the parent's and its own.
fn started_links(app_sessions: &[AppEntry]) -> Vec<StartedLink> {
    let by_host: HashMap<(&str, &str), &str> = app_sessions
        .iter()
        .filter_map(|(tool, _, app)| {
            Some((
                (tool.as_str(), app.host_session_id.as_str()),
                app.session_id.as_deref()?,
            ))
        })
        .collect();
    let mut links: Vec<StartedLink> = app_sessions
        .iter()
        .filter(|(_, _, app)| !app.archived)
        .filter_map(|(tool, _, app)| {
            let parent = by_host.get(&(tool.as_str(), app.spawned_from.as_deref()?))?;
            Some(StartedLink {
                tool: tool.clone(),
                parent: (*parent).to_owned(),
                session_id: app.session_id.clone()?,
                title: app.title.clone(),
                last_activity_ms: app.last_activity_ms,
            })
        })
        .collect();
    links.sort_by(|a, b| {
        b.last_activity_ms
            .cmp(&a.last_activity_ms)
            .then_with(|| a.session_id.cmp(&b.session_id))
    });
    links.dedup_by(|a, b| a.tool == b.tool && a.session_id == b.session_id);
    links
}

/// A file as last read.
struct Cached<T> {
    len: u64,
    modified: Option<SystemTime>,
    value: Option<T>,
}

/// Reads `path` through `read` unless its size and time are unchanged.
fn cached<T: Clone>(
    cache: &mut HashMap<PathBuf, Cached<T>>,
    path: &Path,
    read: impl FnOnce(&Path) -> Option<T>,
) -> Option<T> {
    let meta = std::fs::metadata(path).ok()?;
    let (len, modified) = (meta.len(), meta.modified().ok());
    if let Some(entry) = cache.get(path)
        && entry.len == len
        && entry.modified == modified
    {
        return entry.value.clone();
    }
    let value = read(path);
    cache.insert(
        path.to_path_buf(),
        Cached {
            len,
            modified,
            value: value.clone(),
        },
    );
    value
}

/// What one held process names: `(tool, profile, conversation id)`.
type HeldSession = (String, Option<PathBuf>, String);
/// What each held process names, by `(pid, start)`.
type Held = HashMap<(u32, Option<i64>), Vec<HeldSession>>;
/// An app's session, with its tool and instance folder.
type AppEntry = (String, PathBuf, AppSession);

/// The refresher's memory between ticks.
#[derive(Default)]
pub struct Scanner {
    hosts: Option<HostRegistry>,
    table: Table,
    full_at: Option<Instant>,
    alive_at: Option<Instant>,
    held_at: Option<Instant>,
    pid_files: HashMap<PathBuf, Cached<LiveSession>>,
    app_files: HashMap<PathBuf, Cached<AppSession>>,
    /// What each session server or unexplained session process held:
    /// `(pid, start)` → `(tool, profile, id)`.
    held: Held,
    /// 0.3.3: the desktop apps the last full scan found, until taken.
    apps: Option<Vec<RunningApp>>,
}

impl std::fmt::Debug for Scanner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scanner")
            .field("processes", &self.table.len())
            .field("files", &(self.pid_files.len() + self.app_files.len()))
            .finish_non_exhaustive()
    }
}

/// Each due time: whether `at` is unset or `every` ago.
fn due(at: Option<Instant>, every: Duration, now: Instant) -> bool {
    at.is_none_or(|at| now.saturating_duration_since(at) >= every)
}

/// A verified pid-file session, before merging.
struct Found {
    profile: Profile,
    session: LiveSession,
    pid: u32,
    start: Option<i64>,
}

impl Scanner {
    /// 0.3.3: the desktop apps the last full scan found, once (the places
    /// remember them); `None` between full scans.
    pub fn take_apps(&mut self) -> Option<Vec<RunningApp>> {
        self.apps.take()
    }

    /// One refresh: what is open now.
    pub fn tick(
        &mut self,
        os: &dyn Os,
        adapters: &[(String, SharedAdapter)],
        profiles: &[Profile],
        home: &Path,
        now: Instant,
    ) -> LiveModel {
        let hosts = self
            .hosts
            .get_or_insert_with(|| {
                HostRegistry::new(
                    adapters
                        .iter()
                        .flat_map(|(_, adapter)| adapter.host_apps())
                        .collect(),
                )
            })
            .clone();
        let full = due(self.full_at, FULL_EVERY, now);
        if full {
            self.table = os.table();
            self.full_at = Some(now);
            self.alive_at = Some(now);
            self.apps = Some(running_apps(&self.table, &hosts, home));
        }
        let sessions = self.session_files(adapters, profiles);
        self.refresh_pids(os, &sessions, now);
        // A pid counts only while its start time matches the file's.
        let found: Vec<Found> = sessions
            .into_iter()
            .filter_map(|(profile, session)| {
                let pid = session.pid?;
                let process = self.table.get(&pid)?;
                same_start(session.started_secs, process.start).then_some(Found {
                    start: process.start,
                    profile,
                    session,
                    pid,
                })
            })
            .collect();
        let explained: HashSet<u32> = found.iter().map(|found| found.pid).collect();
        if full || due(self.held_at, HELD_EVERY, now) {
            self.held_at = Some(now);
            self.held = held(os, &self.table, adapters, profiles, &explained);
        }
        // A held process that's gone (or reused) holds nothing.
        let table = &self.table;
        self.held.retain(|(pid, start), _| {
            table
                .get(pid)
                .is_some_and(|process| same_start(*start, process.start))
        });
        let app_sessions = self.app_sessions(adapters, &hosts, home);
        build(&self.table, &hosts, home, found, &self.held, &app_sessions)
    }

    /// The per-session files: listed every tick, read when changed.
    fn session_files(
        &mut self,
        adapters: &[(String, SharedAdapter)],
        profiles: &[Profile],
    ) -> Vec<(Profile, LiveSession)> {
        let mut sessions = Vec::new();
        let mut listed: HashSet<PathBuf> = HashSet::new();
        for profile in profiles {
            let Some((_, adapter)) = adapters.iter().find(|(id, _)| *id == profile.tool) else {
                continue;
            };
            for path in adapter.live_session_files(&profile.dir) {
                if let Some(session) =
                    cached(&mut self.pid_files, &path, |p| adapter.read_live_session(p))
                {
                    sessions.push((profile.clone(), session));
                }
                listed.insert(path);
            }
        }
        self.pid_files.retain(|path, _| listed.contains(path));
        sessions
    }

    /// New pids are read with their parent chains, pid by pid; known ones
    /// every few seconds (gone ones leave, reused ones change).
    fn refresh_pids(&mut self, os: &dyn Os, sessions: &[(Profile, LiveSession)], now: Instant) {
        let missing: Vec<u32> = sessions
            .iter()
            .filter_map(|(_, session)| session.pid)
            .filter(|pid| !self.table.contains_key(pid))
            .collect();
        for pid in missing {
            fill_chain(&mut self.table, pid, os);
        }
        if !due(self.alive_at, ALIVE_EVERY, now) {
            return;
        }
        self.alive_at = Some(now);
        let mut pids: Vec<u32> = sessions
            .iter()
            .filter_map(|(_, session)| session.pid)
            .collect();
        pids.extend(self.held.keys().map(|(pid, _)| *pid));
        pids.sort_unstable();
        pids.dedup();
        let fresh = os.some(&pids);
        for pid in pids {
            match fresh.get(&pid) {
                Some(process) => {
                    self.table.insert(pid, process.clone());
                }
                None => {
                    self.table.remove(&pid);
                }
            }
        }
    }

    /// The apps' own session state, for each running instance.
    fn app_sessions(
        &mut self,
        adapters: &[(String, SharedAdapter)],
        hosts: &HostRegistry,
        home: &Path,
    ) -> Vec<AppEntry> {
        let mut out = Vec::new();
        let mut listed: HashSet<PathBuf> = HashSet::new();
        for (app_id, instance) in running_instances(&self.table, hosts, home) {
            for (tool, adapter) in adapters {
                if !adapter.host_apps().iter().any(|app| app.id == app_id) {
                    continue;
                }
                for path in adapter.app_session_files(&instance) {
                    if let Some(session) =
                        cached(&mut self.app_files, &path, |p| adapter.read_app_session(p))
                    {
                        out.push((tool.clone(), instance.clone(), session));
                    }
                    listed.insert(path);
                }
            }
        }
        self.app_files.retain(|path, _| listed.contains(path));
        out
    }
}

/// Adds `pid` and its missing ancestors to `table`, one read each.
fn fill_chain(table: &mut Table, pid: u32, os: &dyn Os) {
    let mut next = Some(pid);
    for _ in 0..64 {
        let Some(at) = next else { break };
        if at <= 1 {
            break;
        }
        if let std::collections::hash_map::Entry::Vacant(slot) = table.entry(at) {
            let Some(found) = os.some(&[at]).remove(&at) else {
                break;
            };
            slot.insert(found);
        }
        next = table.get(&at).and_then(|process| process.ppid);
    }
}

/// What session servers (and session processes no pid file explains)
/// hold open, and which conversation an unexplained one resumes.
fn held(
    os: &dyn Os,
    table: &Table,
    adapters: &[(String, SharedAdapter)],
    profiles: &[Profile],
    explained: &HashSet<u32>,
) -> Held {
    let mut out = Held::new();
    let mut asked = 0;
    let mut candidates: Vec<&Proc> = table.values().collect();
    candidates.sort_by_key(|process| process.pid);
    for process in candidates {
        if explained.contains(&process.pid) {
            continue;
        }
        let exe = process.exe.as_deref();
        for (tool, adapter) in adapters {
            let server = adapter.is_session_server(exe, &process.args);
            let session = if server {
                None
            } else {
                match adapter.session_process(exe, &process.args) {
                    Some(session) => Some(session),
                    None => continue,
                }
            };
            // A session started by another of its tool's processes is that
            // one's (a launcher, a shim): only the outermost counts.
            if session.is_some()
                && process
                    .ppid
                    .and_then(|ppid| table.get(&ppid))
                    .is_some_and(|parent| {
                        adapter
                            .session_process(parent.exe.as_deref(), &parent.args)
                            .is_some()
                    })
            {
                break;
            }
            let mut ids: Vec<HeldSession> = Vec::new();
            if asked < HELD_MAX
                && let Some(open) = os.open_files(process.pid)
            {
                asked += 1;
                for profile in profiles.iter().filter(|profile| profile.tool == *tool) {
                    for id in adapter.held_sessions(&profile.dir, &open) {
                        ids.push((tool.clone(), Some(profile.dir.clone()), id));
                    }
                }
            }
            if ids.is_empty()
                && let Some(id) = session.and_then(|session| session.resumes)
            {
                ids.push((tool.clone(), None, id));
            }
            if !ids.is_empty() {
                out.entry((process.pid, process.start))
                    .or_default()
                    .extend(ids);
            }
            break;
        }
    }
    out
}

/// How much a recorded status says, to merge two places' statuses.
fn status_rank(status: LiveStatus) -> u8 {
    match status {
        LiveStatus::Waiting => 3,
        LiveStatus::Busy => 2,
        LiveStatus::Idle => 1,
        LiveStatus::Unknown => 0,
    }
}

/// The model from what was found.
fn build(
    table: &Table,
    hosts: &HostRegistry,
    home: &Path,
    found: Vec<Found>,
    held: &Held,
    app_sessions: &[AppEntry],
) -> LiveModel {
    type Key = (String, Option<String>, String);
    let mut entries: BTreeMap<Key, LiveEntry> = BTreeMap::new();
    let mut pid_dirs = BTreeMap::new();
    let mut status_of: HashMap<Key, LiveStatus> = HashMap::new();
    for found in found {
        pid_dirs.insert(found.pid, found.profile.dir.clone());
        let host = host_of(found.pid, table, hosts, home);
        // The app's own record of this session: in the instance the
        // process runs in, by the app's id for it.
        let app = found.session.host_session_id.as_ref().and_then(|host_id| {
            app_sessions.iter().find(|(tool, instance, app)| {
                *tool == found.profile.tool
                    && app.host_session_id == *host_id
                    && host
                        .as_ref()
                        .and_then(|host| host.instance_dir.as_ref())
                        .is_none_or(|dir| dir == instance)
            })
        });
        let key = (
            found.profile.tool.clone(),
            found.profile.account_id.clone(),
            found.session.session_id.clone(),
        );
        // Open in two places: the one that says most wins (waiting on the
        // user, then busy, then idle).
        let status = status_of.entry(key.clone()).or_default();
        if status_rank(found.session.status) > status_rank(*status) {
            *status = found.session.status;
        }
        let entry = entries.entry(key).or_insert_with(|| LiveEntry {
            tool: found.profile.tool.clone(),
            account_id: found.profile.account_id.clone(),
            session_id: found.session.session_id.clone(),
            places: Vec::new(),
            state: None,
            probably: false,
            viewed_ms: None,
            app_title: None,
            background: found.session.background,
        });
        entry.background &= found.session.background;
        if let Some((_, _, app)) = app {
            entry.viewed_ms = entry.viewed_ms.max(app.last_focused_ms);
            entry.app_title = entry.app_title.take().or_else(|| app.title.clone());
            entry.state = state_of(*status, app.outcome);
        } else if entry.state.is_none() || status_rank(*status) >= status_rank(LiveStatus::Busy) {
            entry.state = state_of(*status, None);
        }
        entry.places.push(Place {
            pid: found.pid,
            start: found.start,
            host,
            profile: Some(found.profile.dir),
        });
    }
    for ((pid, start), ids) in held {
        for (tool, profile, id) in ids {
            let key = (tool.clone(), None, id.clone());
            let entry = entries.entry(key).or_insert_with(|| LiveEntry {
                tool: tool.clone(),
                account_id: None,
                session_id: id.clone(),
                places: Vec::new(),
                state: None,
                probably: false,
                viewed_ms: None,
                app_title: None,
                background: false,
            });
            if !entry.places.iter().any(|place| place.pid == *pid) {
                entry.places.push(Place {
                    pid: *pid,
                    start: *start,
                    host: host_of(*pid, table, hosts, home),
                    profile: profile.clone(),
                });
            }
        }
    }
    let mut entries: Vec<LiveEntry> = entries.into_values().collect();
    for entry in &mut entries {
        entry.places.sort_by_key(|place| place.pid);
    }
    let open: HashSet<(&str, &str)> = entries
        .iter()
        .map(|entry| (entry.tool.as_str(), entry.session_id.as_str()))
        .collect();
    let mut inactive: Vec<(String, String)> = app_sessions
        .iter()
        .filter(|(_, _, app)| !app.archived)
        .filter_map(|(tool, _, app)| {
            let id = app.session_id.as_deref()?;
            (!open.contains(&(tool.as_str(), id))).then(|| (tool.clone(), id.to_owned()))
        })
        .collect();
    inactive.sort();
    inactive.dedup();
    LiveModel {
        entries,
        inactive,
        pid_dirs,
        started: started_links(app_sessions),
    }
}

#[cfg(test)]
mod tests;
