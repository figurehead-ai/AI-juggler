//! AJ-E3013 (D-116, ADR-0020's D-116 addendum): a keyless call, attested.
//!
//! A desktop app's session never holds a key: AI Juggler didn't start it.
//! When its bridge (AI Juggler's connector, added with the user's consent)
//! calls with an empty key, the app works out who it is from the system,
//! never from anything the caller says, on **every** call:
//!
//! 1. the peer's pid and the time of the accept (the socket);
//! 2. its parent chain, from the kernel's own records
//!    ([`juggler_proc_list::list`]): the peer listed, not a zombie,
//!    readable as this user's, started no later than the accept; its
//!    parent and grandparent listed, not zombies, each started no later
//!    than its child (`chain_of`);
//! 3. every desktop App the user added, with a config folder, whose tool's
//!    adapter can attest ([`juggler_adapter::peer::DesktopPeer`]): exactly
//!    one must say this chain is one of its sessions.
//!
//! Then its facts (App, Account, Tool, Mac, conversation, project) are
//! kept under a minted `desktop:<conversation>` id in
//! [`juggler_core::storage::attested`], which this file alone writes, and
//! the call carries on exactly as a keyed one. Anything short of all that
//! is a refusal with the reason, never a fallback.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use juggler_adapter::SharedAdapter;
use juggler_adapter::peer::{DesktopPlace, MAX_HOPS, PeerVerdict, ProcStep};
use juggler_core::model::{AccountId, HarnessId, ProjectId, SessionId};
use juggler_core::pools::SessionFacts;
use juggler_core::storage::MemberRef;
use juggler_core::storage::attested::desktop_id;
use juggler_platform::local_ipc::Peer;
use juggler_proc_list::Listed;

use crate::state::AppState;

/// An attested desktop caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopCaller {
    /// Its minted session id (`desktop:<conversation>`).
    pub session: SessionId,
    /// The desktop App (`apps.id`).
    pub app_id: String,
    /// The App's config folder row.
    pub folder: String,
    /// The tool's conversation id.
    pub conversation: String,
    /// The tool's process the caller descends from.
    pub tool_pid: u32,
    /// Whether its App's sessions can be woken (D-119).
    pub can_wake: bool,
    /// L3: the facts this call was attested with, carried through it (the
    /// shared map is for the pool lookups, not re-read by the call).
    pub facts: SessionFacts,
}

impl DesktopCaller {
    /// The member ref others address it by (its Watched member's).
    #[must_use]
    pub fn member_ref(&self) -> String {
        MemberRef::Conversation {
            folder: self.folder.clone(),
            id: self.conversation.clone(),
        }
        .to_wire()
    }
}

/// A desktop App that may hold the caller.
pub(crate) struct Candidate {
    /// `apps.id`.
    pub app_id: String,
    /// Its config folder row.
    pub folder: String,
    /// That folder on disk.
    pub config_dir: PathBuf,
    /// The app instance's own folder.
    pub instance_dir: PathBuf,
    /// Its tool's adapter.
    pub adapter: SharedAdapter,
    /// Its host app's registry id (`claude-app`).
    pub host_app_id: Option<String>,
}

/// Epoch microseconds of `at` (0 before 1970).
fn micros(at: SystemTime) -> u128 {
    at.duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_micros())
}

/// When `process` started, epoch microseconds.
fn started(process: &Listed) -> u128 {
    u128::from(process.start_secs) * 1_000_000 + u128::from(process.start_micros)
}

/// The caller's chain (itself, then up to [`MAX_HOPS`] ancestors), from
/// the kernel's listing `table`; `ours` says whether a pid is this user's
/// to read.
///
/// # Errors
///
/// Why the chain can't be trusted, in words.
pub(crate) fn chain_of(
    peer: Peer,
    table: &HashMap<u32, Listed>,
    ours: &dyn Fn(u32) -> bool,
    version_of: &dyn Fn(u32) -> Option<u32>,
) -> Result<Vec<ProcStep>, String> {
    let accepted = peer.accepted;
    let pid = peer
        .pid
        .ok_or("the system didn't name the caller's process")?;
    // L4: the socket's audit token names the pid and its version; the live
    // process at that pid must be the very one that connected (a pid reused
    // since has another version).
    match (peer.pid_version, version_of(pid)) {
        (Some(token), Some(live)) if token == live => {}
        _ => {
            return Err("the caller's process can't be told from one that took its pid".to_owned());
        }
    }
    let peer = pid;
    let step = |process: &Listed| ProcStep {
        pid: process.pid,
        start_secs: i64::try_from(process.start_secs).unwrap_or(i64::MAX),
    };
    let caller = table
        .get(&peer)
        .filter(|process| !process.zombie)
        .ok_or("the caller's process is gone")?;
    if !ours(peer) {
        return Err("the caller's process isn't this user's".to_owned());
    }
    if started(caller) > micros(accepted) {
        // A pid reused after it connected.
        return Err("the caller's process started after it connected".to_owned());
    }
    let mut chain = vec![step(caller)];
    let mut child = caller;
    for _ in 0..MAX_HOPS {
        if child.ppid <= 1 {
            break; // the system's own: no session above it
        }
        let Some(parent) = table.get(&child.ppid).filter(|process| !process.zombie) else {
            return Err("a process above the caller is gone".to_owned());
        };
        if started(parent) > started(child) {
            // A parent that started after its child: the pid was reused.
            return Err("a process above the caller isn't its parent any more".to_owned());
        }
        chain.push(step(parent));
        child = parent;
    }
    Ok(chain)
}

/// The desktop Apps the user added that have a config folder and a tool
/// whose adapter can attest.
pub(crate) fn candidates(state: &Arc<AppState>) -> Vec<Candidate> {
    let Ok(storage) = state.storage.lock() else {
        return Vec::new();
    };
    let Ok(apps) = storage.apps().list() else {
        return Vec::new();
    };
    let home = juggler_platform::user::home();
    apps.into_iter()
        .filter(|app| app.kind == "desktop" && !app.hidden && app.is_users())
        .filter_map(|app| {
            let folder = app.folder_id.clone()?;
            let adapter = state.registry.get(&HarnessId::new(app.tool.as_str()))?;
            adapter.desktop_peer()?;
            let config_dir = storage
                .accounts()
                .get(&AccountId::new(folder.clone()))
                .ok()?
                .state_dir;
            let instance_dir = if let Some(dir) = app.instance_dir.as_deref() {
                PathBuf::from(dir)
            } else {
                let host = app.host_app_id.as_deref()?;
                let found = adapter.host_apps().into_iter().find(|h| h.id == host)?;
                home.as_ref()?.join(found.default_instance?)
            };
            Some(Candidate {
                app_id: app.id,
                folder,
                config_dir,
                instance_dir,
                adapter,
                host_app_id: app.host_app_id,
            })
        })
        .collect()
}

/// The candidate that attested the caller (its index) and the session,
/// or why none did.
pub(crate) type Found = Result<(usize, juggler_adapter::peer::AttestedSession), String>;

/// Which of `candidates` holds the caller whose chain is `chain`:
/// exactly one must attest it. The conversations found stale on the way
/// come back too (their kept facts are dropped).
pub(crate) fn decide(
    candidates: &[Candidate],
    chain: &[ProcStep],
) -> (Found, Vec<(usize, String)>) {
    let mut found = Vec::new();
    let mut stale: Vec<(usize, String)> = Vec::new();
    let mut why = None;
    for (at, candidate) in candidates.iter().enumerate() {
        let Some(peer) = candidate.adapter.desktop_peer() else {
            continue;
        };
        let place = DesktopPlace {
            config_dir: &candidate.config_dir,
            instance_dir: &candidate.instance_dir,
        };
        match peer.attest(&place, chain) {
            PeerVerdict::Attested(session) => found.push((at, session)),
            PeerVerdict::Unattested {
                reason,
                stale: gone,
            } => {
                stale.extend(gone.map(|conversation| (at, conversation)));
                why.get_or_insert(reason);
            }
        }
    }
    let verdict = match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err(why.unwrap_or_else(|| {
            "no desktop app AI Juggler knows has added its connector".to_owned()
        })),
        _ => Err("more than one desktop app claims this session".to_owned()),
    };
    (verdict, stale)
}

/// M1 (security review, round 2): what the lineage check reads of the
/// system: the real one in [`verify_tool`], a made-up one in the tests.
pub(crate) struct System<'a> {
    /// Whether the code **running** as a pid now meets a code-signing
    /// requirement (the pid's version the same before and after the
    /// check: [`juggler_platform::code_sign::running_satisfies`]).
    pub satisfies: &'a dyn Fn(u32, &str) -> bool,
    /// A pid's parent, from the kernel's listing.
    pub parent_of: &'a dyn Fn(u32) -> Option<u32>,
}

/// M1: the requirements an App's adapter pins
/// ([`juggler_adapter::peer::DesktopPeer::tool_requirement`] and the rest).
#[derive(Clone, Copy)]
pub(crate) struct Requirements<'a> {
    /// The tool's running program.
    pub tool: &'a str,
    /// The App's own main process.
    pub host: &'a str,
    /// A helper of the App between it and the tool.
    pub helper: &'a str,
}

/// M1: the App's main process that runs the tool at `tool`, known by its
/// verified running code only (never a path, a bundle or a folder name):
/// the tool's running code meets the tool's requirement, and its parent's
/// the App's, or its parent's an App helper's and its grandparent's the
/// App's.
///
/// # Errors
///
/// Why not, in words.
pub(crate) fn host_of_tool(
    tool: u32,
    requirements: Requirements<'_>,
    system: &System<'_>,
) -> Result<u32, String> {
    if !(system.satisfies)(tool, requirements.tool) {
        return Err("the session's running program isn't its maker's Claude Code".to_owned());
    }
    let above = |pid: u32| (system.parent_of)(pid).filter(|parent| *parent > 1);
    let parent = above(tool).ok_or("the session's Claude Code has no app above it")?;
    if (system.satisfies)(parent, requirements.host) {
        return Ok(parent);
    }
    if !(system.satisfies)(parent, requirements.helper) {
        return Err("the session's Claude Code wasn't started by its app".to_owned());
    }
    let grandparent = above(parent).ok_or("the session's Claude Code has no app above it")?;
    if (system.satisfies)(grandparent, requirements.host) {
        Ok(grandparent)
    } else {
        Err("the session's Claude Code isn't running under its app".to_owned())
    }
}

/// M1: whether each of `pids` is still the process `before` listed (the
/// same start time and parent) in the kernel's listing `after`, read once
/// the checks were done: none ended, and none's pid was taken since.
pub(crate) fn still_the_same(
    pids: &[u32],
    before: &HashMap<u32, Listed>,
    after: &HashMap<u32, Listed>,
) -> bool {
    pids.iter()
        .all(|pid| match (before.get(pid), after.get(pid)) {
            (Some(was), Some(now)) => {
                !now.zombie && started(was) == started(now) && was.ppid == now.ppid
            }
            _ => false,
        })
}

/// M1: whether the App's main process, whose arguments are `args`, runs
/// the instance at `instance_dir` (its `--user-data-dir`, else the App's
/// default folder under `home`), compared as real paths.
pub(crate) fn runs_instance(
    args: &[String],
    host: &juggler_adapter::live::HostApp,
    home: &std::path::Path,
    instance_dir: &std::path::Path,
) -> bool {
    let running = host
        .instance_arg
        .as_deref()
        .and_then(|flag| crate::features::conversations::places::arg_value(args, flag))
        .map(PathBuf::from)
        .or_else(|| host.default_instance.as_deref().map(|dir| home.join(dir)));
    match (
        running.map(std::fs::canonicalize),
        std::fs::canonicalize(instance_dir),
    ) {
        (Some(Ok(running)), Ok(ours)) => running == ours,
        _ => false,
    }
}

#[cfg(test)]
thread_local! {
    /// Tests that drive a real attestation with this test process's own
    /// parent standing in for Claude Code skip M1's check of its program
    /// (which no test process can pass); the check itself is tested apart.
    pub(crate) static ACCEPT_ANY_TOOL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// M1 (security review, rounds 1 and 2): the tool process the registry
/// names must be this App's own Claude Code, its **running** code signed
/// by its maker, running under this App instance's own **running** main
/// process:
/// - its executable has no link on its path and sits where the App keeps
///   its tool ([`juggler_adapter::peer::DesktopPeer::tool_binary`]): a
///   filter, never the proof;
/// - the code running as the tool, and as the App's main process above it
///   (its parent, or its grandparent through one of the App's own signed
///   helpers), meets the requirements the adapter pins
///   ([`host_of_tool`]): the kernel's code-signing check of the running
///   code by pid ([`juggler_platform::code_sign::running_satisfies`]),
///   each pid's version read before and after it;
/// - every process checked is still the one first listed (start time and
///   parent) once the checks are done ([`still_the_same`]);
/// - that main process runs this App's instance ([`runs_instance`]).
///
/// # Errors
///
/// Why not, in words.
fn verify_tool(
    candidate: &Candidate,
    session: &juggler_adapter::peer::AttestedSession,
    kernel: &HashMap<u32, Listed>,
) -> Result<(), String> {
    #[cfg(test)]
    if ACCEPT_ANY_TOOL.with(std::cell::Cell::get) {
        return Ok(());
    }
    use crate::features::conversations::procs;
    let peer = candidate
        .adapter
        .desktop_peer()
        .ok_or("this app's tool can't be checked")?;
    let (Some(tool), Some(host), Some(helper)) = (
        peer.tool_requirement(),
        peer.host_requirement(),
        peer.helper_requirement(),
    ) else {
        return Err("this app's tool can't be checked".to_owned());
    };
    let exe = procs::some(&[session.tool_pid])
        .remove(&session.tool_pid)
        .and_then(|process| process.exe)
        .ok_or("the session's Claude Code process can't be read")?;
    if std::fs::canonicalize(&exe).ok().as_ref() != Some(&exe) {
        return Err("the session's program is reached through a link".to_owned());
    }
    let place = DesktopPlace {
        config_dir: &candidate.config_dir,
        instance_dir: &candidate.instance_dir,
    };
    if !peer.tool_binary(&place, &exe) {
        return Err("the session's process isn't this app's own Claude Code".to_owned());
    }
    let satisfies = |pid: u32, requirement: &str| {
        let Some(before) = juggler_proc_list::pid_version(pid) else {
            return false;
        };
        juggler_platform::code_sign::running_satisfies(pid, before, requirement)
            && juggler_proc_list::pid_version(pid) == Some(before)
    };
    let parent_of = |pid: u32| kernel.get(&pid).map(|process| process.ppid);
    let system = System {
        satisfies: &satisfies,
        parent_of: &parent_of,
    };
    let host_pid = host_of_tool(
        session.tool_pid,
        Requirements { tool, host, helper },
        &system,
    )?;
    let mut checked = vec![session.tool_pid, host_pid];
    if let Some(parent) = parent_of(session.tool_pid).filter(|parent| *parent != host_pid) {
        checked.push(parent);
    }
    let now: HashMap<u32, Listed> = juggler_proc_list::list()
        .ok_or("this system doesn't list its processes")?
        .into_iter()
        .map(|process| (process.pid, process))
        .collect();
    if !still_the_same(&checked, kernel, &now) {
        return Err("a process checked ended or changed while it was checked".to_owned());
    }
    let app = candidate
        .host_app_id
        .as_deref()
        .and_then(|id| {
            candidate
                .adapter
                .host_apps()
                .into_iter()
                .find(|app| app.id == id)
        })
        .ok_or("this app's own app isn't known")?;
    let args = procs::some(&[host_pid])
        .remove(&host_pid)
        .map(|process| process.args)
        .unwrap_or_default();
    let home = juggler_platform::user::home().unwrap_or_default();
    if !runs_instance(&args, &app, &home, &candidate.instance_dir) {
        return Err("the session runs under another profile of its app".to_owned());
    }
    Ok(())
}

/// H3: drops at once what is kept for each attested desktop session whose
/// tool's registry no longer names it (the entry gone, or its process's
/// start time changed), or whose kept facts ran out, besides the ten
/// minutes' expiry. Runs on every attested call (the refresh path).
pub(crate) fn sweep(state: &Arc<AppState>, candidates: &[Candidate]) {
    let kept: Vec<(String, (String, juggler_adapter::peer::AttestedSession))> =
        match state.agent_bridge.desktop.lock() {
            Ok(kept) => kept
                .iter()
                .map(|(id, at)| (id.clone(), at.clone()))
                .collect(),
            Err(_) => return,
        };
    let mut gone = Vec::new();
    for (id, (app_id, session)) in kept {
        let current = candidates
            .iter()
            .find(|candidate| candidate.app_id == app_id)
            .is_some_and(|candidate| {
                let place = DesktopPlace {
                    config_dir: &candidate.config_dir,
                    instance_dir: &candidate.instance_dir,
                };
                candidate
                    .adapter
                    .desktop_peer()
                    .is_some_and(|peer| peer.still_current(&place, &session))
            });
        let fresh = state.storage.lock().is_ok_and(|storage| {
            storage
                .attested()
                .seen(&juggler_core::model::SessionId::new(id.as_str()))
                .is_some()
        });
        if !current || !fresh {
            gone.push(id);
        }
    }
    if gone.is_empty() {
        return;
    }
    if let Ok(storage) = state.storage.lock() {
        for id in &gone {
            storage
                .attested()
                .forget(&juggler_core::model::SessionId::new(id.as_str()));
        }
    }
    if let Ok(mut kept) = state.agent_bridge.desktop.lock() {
        for id in &gone {
            kept.remove(id);
        }
    }
}

/// The project of `conversation` in `folder`, from the conversation index
/// (`None` when it has none): read once per conversation, then kept with
/// its facts.
fn project_of(state: &Arc<AppState>, folder: &str, conversation: &str) -> Option<ProjectId> {
    let found = state
        .conversations
        .all()
        .ok()?
        .into_iter()
        .find(|one| one.account_id == folder && one.summary.id == conversation)?;
    let cwd = found.folder?;
    let storage = state.storage.lock().ok()?;
    let homes = storage.projects().worktree_homes().ok()?;
    let root = juggler_core::storage::project_root_with(&cwd, &homes);
    crate::features::projects::by_root(&storage, |project| project.id)
        .ok()?
        .remove(&root)
}

/// Attests the caller at `peer`, and keeps its facts (the only writer of
/// [`juggler_core::storage::Storage::attested`]).
///
/// # Errors
///
/// Why it isn't attested, in words: the call is refused with them.
pub fn attest(state: &Arc<AppState>, peer: Peer) -> Result<DesktopCaller, String> {
    attest_inner(state, peer).inspect_err(|why| log_refusal(why))
}

/// H2: how often a refusal's reason is logged: once a minute a reason,
/// the next line saying how many were left out.
pub const LOG_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

/// The most reasons the log keeps count of (a flood of distinct ones
/// can't grow it).
const LOG_REASONS_MAX: usize = 64;

/// H2: the line to log for a refusal `why` now, if one is due: at most
/// one a reason in [`LOG_EVERY`], with how many were left out since.
pub(crate) fn refusal_line(
    seen: &mut HashMap<String, (std::time::Instant, u32)>,
    why: &str,
    now: std::time::Instant,
) -> Option<String> {
    let skipped = match seen.get_mut(why) {
        Some((since, skipped)) if now.duration_since(*since) < LOG_EVERY => {
            *skipped = skipped.saturating_add(1);
            return None;
        }
        Some(entry) => std::mem::replace(entry, (now, 0)).1,
        None => {
            if seen.len() >= LOG_REASONS_MAX {
                seen.clear();
            }
            seen.insert(why.to_owned(), (now, 0));
            0
        }
    };
    Some(if skipped == 0 {
        format!("agent bridge: a keyless call wasn't attested: {why}")
    } else {
        format!(
            "agent bridge: a keyless call wasn't attested: {why} ({skipped} more like it in the last minute)"
        )
    })
}

/// H2: logs a refusal by its reason only (scrubbed of the home folder,
/// the user name, paths and anything secret-shaped: the crash reports'
/// scrubber), never a session id or a record's contents, and at most once
/// a minute a reason.
fn log_refusal(why: &str) {
    static SEEN: std::sync::OnceLock<std::sync::Mutex<HashMap<String, (std::time::Instant, u32)>>> =
        std::sync::OnceLock::new();
    let home = juggler_platform::user::home()
        .map(|home| home.to_string_lossy().into_owned())
        .unwrap_or_default();
    let user = std::env::var("USER").unwrap_or_default();
    let reason = crate::crash::scrub(why, &home, &user);
    let line = SEEN
        .get_or_init(std::sync::Mutex::default)
        .lock()
        .ok()
        .and_then(|mut seen| refusal_line(&mut seen, &reason, std::time::Instant::now()));
    if let Some(line) = line {
        tracing::info!("{line}");
    }
}

fn attest_inner(state: &Arc<AppState>, peer: Peer) -> Result<DesktopCaller, String> {
    let candidates = candidates(state);
    // H3: whatever else this call turns out to be, every desktop session
    // kept now is checked against its tool's registry again.
    sweep(state, &candidates);
    let table: HashMap<u32, Listed> = juggler_proc_list::list()
        .ok_or("this system doesn't list its processes")?
        .into_iter()
        .map(|process| (process.pid, process))
        .collect();
    let ours = |pid: u32| juggler_proc_list::task(pid).is_some();
    let chain = chain_of(peer, &table, &ours, &juggler_proc_list::pid_version)?;
    let (verdict, gone) = decide(&candidates, &chain);
    if let Ok(storage) = state.storage.lock() {
        for (at, conversation) in &gone {
            storage
                .attested()
                .forget(&desktop_id(&candidates[*at].folder, conversation));
        }
    }
    let (at, session) = verdict?;
    let candidate = &candidates[at];
    // M1: the registry's word isn't enough: its process must be this App's
    // own Claude Code, its running code signed by its maker, under the
    // App's own running main process for this instance.
    verify_tool(candidate, &session, &table)?;
    let id = desktop_id(&candidate.folder, &session.conversation);
    let kept_project = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.attested().get(&id))
        .filter(|facts| {
            facts.conversation.as_ref()
                == Some(&(candidate.folder.clone(), session.conversation.clone()))
        })
        .map(|facts| facts.project);
    let project =
        kept_project.unwrap_or_else(|| project_of(state, &candidate.folder, &session.conversation));
    let machine = crate::commands::local_machine_id(state);
    let storage = state
        .storage
        .lock()
        .map_err(|_| "AI Juggler is busy".to_owned())?;
    let account = storage.subscriptions().folders().ok().and_then(|folders| {
        folders
            .into_iter()
            .find(|(folder, _)| *folder == candidate.folder)
            .and_then(|(_, account)| account)
    });
    let tool = storage
        .accounts()
        .get(&AccountId::new(candidate.folder.clone()))
        .ok()
        .map(|found| found.harness.as_str().to_owned());
    let facts = SessionFacts {
        session: id.clone(),
        work: None,
        project,
        app: Some(candidate.app_id.clone()),
        account,
        tool,
        mac: Some(machine.as_str().to_owned()),
        conversation: Some((candidate.folder.clone(), session.conversation.clone())),
    };
    storage.attested().put(facts.clone());
    drop(storage);
    if let Ok(mut kept) = state.agent_bridge.desktop.lock() {
        kept.insert(
            id.as_str().to_owned(),
            (candidate.app_id.clone(), session.clone()),
        );
    }
    Ok(DesktopCaller {
        session: id,
        app_id: candidate.app_id.clone(),
        folder: candidate.folder.clone(),
        conversation: session.conversation,
        tool_pid: session.tool_pid,
        can_wake: candidate
            .adapter
            .desktop_peer()
            .is_some_and(juggler_adapter::peer::DesktopPeer::can_wake),
        facts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn listed(pid: u32, ppid: u32, start_secs: u64) -> Listed {
        Listed {
            pid,
            ppid,
            start_secs,
            start_micros: 0,
            name: "x".into(),
            zombie: false,
        }
    }

    fn table(rows: &[Listed]) -> HashMap<u32, Listed> {
        rows.iter().map(|row| (row.pid, row.clone())).collect()
    }

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    /// A peer `pid` that connected at `secs`, its token's version 7.
    fn peer(pid: u32, secs: u64) -> Peer {
        Peer {
            pid: Some(pid),
            pid_version: Some(7),
            accepted: at(secs),
        }
    }

    fn seven(_: u32) -> Option<u32> {
        Some(7)
    }

    #[test]
    fn the_chain_comes_from_the_kernel_and_stops_at_two_steps() {
        let rows = table(&[
            listed(1, 0, 1),
            listed(100, 1, 1_000),
            listed(200, 100, 2_000),
            listed(300, 200, 3_000),
            listed(400, 300, 4_000),
        ]);
        let chain = chain_of(peer(400, 5_000), &rows, &|_| true, &seven).unwrap();
        let pids: Vec<u32> = chain.iter().map(|step| step.pid).collect();
        assert_eq!(pids, [400, 300, 200]);
        let short = chain_of(peer(100, 5_000), &rows, &|_| true, &seven).unwrap();
        assert_eq!(short.len(), 1, "nothing above a process launchd started");
    }

    #[test]
    fn a_reused_or_missing_process_is_refused() {
        let rows = table(&[listed(300, 200, 3_000), listed(400, 300, 4_000)]);
        assert!(
            chain_of(peer(400, 3_500), &rows, &|_| true, &seven).is_err(),
            "started after it connected: a reused pid"
        );
        assert!(
            chain_of(peer(400, 5_000), &rows, &|_| true, &seven).is_err(),
            "its grandparent is gone"
        );
        let reparented = table(&[
            listed(200, 1, 9_000),
            listed(300, 200, 3_000),
            listed(400, 300, 4_000),
        ]);
        assert!(
            chain_of(peer(400, 5_000), &reparented, &|_| true, &seven).is_err(),
            "a parent younger than its child: the pid was reused"
        );
        let mut zombie = listed(400, 300, 4_000);
        zombie.zombie = true;
        assert!(chain_of(peer(400, 5_000), &table(&[zombie]), &|_| true, &seven).is_err());
        assert!(
            chain_of(peer(400, 5_000), &rows, &|_| false, &seven).is_err(),
            "not this user's"
        );
        assert!(chain_of(peer(999, 5_000), &rows, &|_| true, &seven).is_err());
    }

    #[test]
    fn a_pid_taken_since_the_connection_is_refused() {
        // L4: the token's version must be the live process's.
        let rows = table(&[listed(300, 1, 3_000), listed(400, 300, 4_000)]);
        assert!(chain_of(peer(400, 5_000), &rows, &|_| true, &seven).is_ok());
        let other = |_: u32| Some(8);
        assert!(chain_of(peer(400, 5_000), &rows, &|_| true, &other).is_err());
        let gone = |_: u32| None;
        assert!(chain_of(peer(400, 5_000), &rows, &|_| true, &gone).is_err());
        let no_token = Peer {
            pid_version: None,
            ..peer(400, 5_000)
        };
        assert!(chain_of(no_token, &rows, &|_| true, &seven).is_err());
    }

    const TOOL: &str = "tool requirement";
    const HOST: &str = "host requirement";
    const HELPER: &str = "helper requirement";

    fn requirements() -> Requirements<'static> {
        Requirements {
            tool: TOOL,
            host: HOST,
            helper: HELPER,
        }
    }

    /// A made-up system: each pid's parent, and the requirements the code
    /// running as it meets.
    fn lineage(rows: &[(u32, u32, &[&str])]) -> Result<u32, String> {
        let meets: HashMap<u32, Vec<String>> = rows
            .iter()
            .map(|(pid, _, met)| (*pid, met.iter().map(|one| (*one).to_owned()).collect()))
            .collect();
        let parents: HashMap<u32, u32> = rows.iter().map(|(pid, ppid, _)| (*pid, *ppid)).collect();
        let satisfies = |pid: u32, requirement: &str| {
            meets
                .get(&pid)
                .is_some_and(|met| met.iter().any(|one| one == requirement))
        };
        let parent_of = |pid: u32| parents.get(&pid).copied();
        host_of_tool(
            400,
            requirements(),
            &System {
                satisfies: &satisfies,
                parent_of: &parent_of,
            },
        )
    }

    #[test]
    fn the_tool_runs_under_the_apps_running_code_directly_or_through_its_helper() {
        // M1 round 2.
        assert_eq!(
            lineage(&[(400, 300, &[TOOL]), (300, 1, &[HOST, HELPER])]),
            Ok(300)
        );
        assert_eq!(
            lineage(&[
                (400, 300, &[TOOL]),
                (300, 200, &[HELPER]),
                (200, 1, &[HOST, HELPER])
            ]),
            Ok(200),
            "the app's own helper (its disclaimer) between them"
        );
    }

    #[test]
    fn the_host_is_known_by_its_running_code_never_its_name() {
        // M1 round 2, the review's probe (a): a program at `~/x/Claude.app/
        // Contents/MacOS/Claude` whose running code isn't the Claude app's
        // (an impostor, signed ad hoc or by anyone else) hosts nothing,
        // whatever its path or folder says (the check never reads either).
        let impostor = lineage(&[(400, 300, &[TOOL]), (300, 1, &[])]);
        assert_eq!(
            impostor,
            Err("the session's Claude Code wasn't started by its app".to_owned())
        );
        assert!(
            lineage(&[
                (400, 300, &[TOOL]),
                (300, 200, &[HELPER]),
                (200, 1, &[HELPER])
            ])
            .is_err(),
            "a helper of the maker's under something that isn't the app"
        );
        assert!(
            lineage(&[(400, 300, &[TOOL]), (300, 200, &[]), (200, 1, &[HOST])]).is_err(),
            "the app above something of no one's (a shell it didn't sign)"
        );
        assert!(
            lineage(&[(400, 300, &[TOOL]), (300, 1, &[HELPER])]).is_err(),
            "a helper with nothing above it"
        );
        assert!(
            lineage(&[(400, 1, &[TOOL])]).is_err(),
            "a Claude Code launchd runs: no app at all"
        );
    }

    #[test]
    fn a_tool_whose_running_code_isnt_genuine_is_refused() {
        // M1 round 2, the review's probe (b): the genuine program renamed
        // over a running ad-hoc one changes nothing: what runs is checked
        // (`code_sign`'s own probe proves the check reads the running
        // code); here, a tool whose running code fails is refused even
        // under the genuine app.
        assert_eq!(
            lineage(&[(400, 300, &[]), (300, 1, &[HOST])]),
            Err("the session's running program isn't its maker's Claude Code".to_owned())
        );
    }

    #[test]
    fn a_process_that_changed_while_it_was_checked_is_refused() {
        // M1 round 2: the start time and parent of every pid checked, read
        // again once the checks are done.
        let before = table(&[listed(200, 1, 1_000), listed(400, 200, 2_000)]);
        assert!(still_the_same(&[200, 400], &before, &before));
        let reused = table(&[listed(200, 1, 3_000), listed(400, 200, 2_000)]);
        assert!(
            !still_the_same(&[200, 400], &before, &reused),
            "a pid taken since"
        );
        let moved = table(&[listed(200, 1, 1_000), listed(400, 1, 2_000)]);
        assert!(
            !still_the_same(&[200, 400], &before, &moved),
            "its parent ended"
        );
        let gone = table(&[listed(200, 1, 1_000)]);
        assert!(!still_the_same(&[200, 400], &before, &gone));
        let mut zombie = listed(400, 200, 2_000);
        zombie.zombie = true;
        let ended = table(&[listed(200, 1, 1_000), zombie]);
        assert!(!still_the_same(&[200, 400], &before, &ended));
    }

    #[test]
    fn the_apps_main_process_must_run_this_instance() {
        // M1: the instance comes from the verified main process's own
        // arguments, compared as real paths.
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let ours = root.path().join("profiles/work");
        let other = root.path().join("profiles/play");
        let default = home.join("Library/Application Support/Claude");
        for dir in [&ours, &other, &default] {
            std::fs::create_dir_all(dir).unwrap();
        }
        let app = juggler_adapters::claude_live::host_apps().remove(0);
        let given = |dir: &std::path::Path| {
            vec![
                "/Applications/Claude.app/Contents/MacOS/Claude".to_owned(),
                format!("--user-data-dir={}", dir.display()),
            ]
        };
        assert!(runs_instance(&given(&ours), &app, &home, &ours));
        assert!(!runs_instance(&given(&other), &app, &home, &ours));
        let plain = vec!["/Applications/Claude.app/Contents/MacOS/Claude".to_owned()];
        assert!(
            runs_instance(&plain, &app, &home, &default),
            "its default one"
        );
        assert!(!runs_instance(&plain, &app, &home, &ours));
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&ours, &link).unwrap();
        assert!(
            runs_instance(&given(&link), &app, &home, &ours),
            "as real paths"
        );
        assert!(!runs_instance(
            &given(&root.path().join("none")),
            &app,
            &home,
            &ours
        ));
    }

    #[test]
    fn a_refusal_is_logged_once_a_minute_a_reason_with_what_was_left_out() {
        // H2.
        let mut seen = HashMap::new();
        let start = std::time::Instant::now();
        let first = refusal_line(&mut seen, "the caller's process is gone", start).unwrap();
        assert_eq!(
            first,
            "agent bridge: a keyless call wasn't attested: the caller's process is gone"
        );
        for n in 1..=5 {
            let at = start + Duration::from_secs(n);
            assert_eq!(
                refusal_line(&mut seen, "the caller's process is gone", at),
                None
            );
        }
        assert!(
            refusal_line(&mut seen, "another reason", start + Duration::from_secs(2)).is_some(),
            "each reason its own"
        );
        let later =
            refusal_line(&mut seen, "the caller's process is gone", start + LOG_EVERY).unwrap();
        assert!(
            later.ends_with("(5 more like it in the last minute)"),
            "{later}"
        );
        // A flood of distinct reasons can't grow it without bound.
        for n in 0..1_000 {
            let _ = refusal_line(&mut seen, &format!("reason {n}"), start);
        }
        assert!(seen.len() <= LOG_REASONS_MAX);
    }

    #[test]
    fn a_logged_reason_carries_no_path_or_user() {
        // H2: the reason goes through the crash reports' scrubber first.
        let scrubbed = crate::crash::scrub(
            "no record at /Users/jdoe/Library/Application Support/x.json for jdoe",
            "/Users/jdoe",
            "jdoe",
        );
        assert!(!scrubbed.contains("jdoe"), "{scrubbed}");
    }
}
