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
//!    than its child ([`chain_of`]);
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
    peer: u32,
    accepted: SystemTime,
    table: &HashMap<u32, Listed>,
    ours: &dyn Fn(u32) -> bool,
) -> Result<Vec<ProcStep>, String> {
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
            })
        })
        .collect()
}

/// Which of `candidates` holds the caller whose chain is `chain`:
/// exactly one must attest it. The conversations found stale on the way
/// come back too (their kept facts are dropped).
pub(crate) fn decide(
    candidates: &[Candidate],
    chain: &[ProcStep],
) -> (
    Result<(usize, juggler_adapter::peer::AttestedSession), String>,
    Vec<String>,
) {
    let mut found = Vec::new();
    let mut stale = Vec::new();
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
                stale.extend(gone);
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
    let pid = peer
        .pid
        .ok_or("the system didn't name the caller's process")?;
    let table: HashMap<u32, Listed> = juggler_proc_list::list()
        .ok_or("this system doesn't list its processes")?
        .into_iter()
        .map(|process| (process.pid, process))
        .collect();
    let ours = |pid: u32| juggler_proc_list::task(pid).is_some();
    let chain = chain_of(pid, peer.accepted, &table, &ours)?;
    let candidates = candidates(state);
    let (verdict, gone) = decide(&candidates, &chain);
    if let Ok(storage) = state.storage.lock() {
        for conversation in &gone {
            storage.attested().forget(&desktop_id(conversation));
        }
    }
    let (at, session) = verdict?;
    let candidate = &candidates[at];
    let id = desktop_id(&session.conversation);
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
    storage.attested().put(SessionFacts {
        session: id.clone(),
        work: None,
        project,
        app: Some(candidate.app_id.clone()),
        account,
        tool,
        mac: Some(machine.as_str().to_owned()),
        conversation: Some((candidate.folder.clone(), session.conversation.clone())),
    });
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

    #[test]
    fn the_chain_comes_from_the_kernel_and_stops_at_two_steps() {
        let rows = table(&[
            listed(1, 0, 1),
            listed(100, 1, 1_000),
            listed(200, 100, 2_000),
            listed(300, 200, 3_000),
            listed(400, 300, 4_000),
        ]);
        let chain = chain_of(400, at(5_000), &rows, &|_| true).unwrap();
        let pids: Vec<u32> = chain.iter().map(|step| step.pid).collect();
        assert_eq!(pids, [400, 300, 200]);
        let short = chain_of(100, at(5_000), &rows, &|_| true).unwrap();
        assert_eq!(short.len(), 1, "nothing above a process launchd started");
    }

    #[test]
    fn a_reused_or_missing_process_is_refused() {
        let rows = table(&[listed(300, 200, 3_000), listed(400, 300, 4_000)]);
        assert!(
            chain_of(400, at(3_500), &rows, &|_| true).is_err(),
            "started after it connected: a reused pid"
        );
        assert!(
            chain_of(400, at(5_000), &rows, &|_| true).is_err(),
            "its grandparent is gone"
        );
        let reparented = table(&[
            listed(200, 1, 9_000),
            listed(300, 200, 3_000),
            listed(400, 300, 4_000),
        ]);
        assert!(
            chain_of(400, at(5_000), &reparented, &|_| true).is_err(),
            "a parent younger than its child: the pid was reused"
        );
        let mut zombie = listed(400, 300, 4_000);
        zombie.zombie = true;
        assert!(chain_of(400, at(5_000), &table(&[zombie]), &|_| true).is_err());
        assert!(
            chain_of(400, at(5_000), &rows, &|_| false).is_err(),
            "not this user's"
        );
        assert!(chain_of(999, at(5_000), &rows, &|_| true).is_err());
    }
}
