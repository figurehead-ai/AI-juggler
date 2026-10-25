//! AJ-E2101 (D-37): the process table the live view reads: each
//! process's executable, arguments, parent and start time, and **never
//! its environment**. The Claude app's sessions carry a sign-in token in
//! their environment; which profile a session runs in comes from where its
//! own per-session file sits instead, and its app instance from its parent
//! chain and arguments.

use std::collections::HashMap;
use std::path::PathBuf;

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// One process, as the live view needs it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Proc {
    /// Its pid.
    pub pid: u32,
    /// Its parent's pid, when known.
    pub ppid: Option<u32>,
    /// Its executable, when the OS reports it.
    pub exe: Option<PathBuf>,
    /// Its arguments, the first being the program.
    pub args: Vec<String>,
    /// Its kernel start time, epoch seconds.
    pub start: Option<i64>,
}

/// The process table, by pid.
pub type Table = HashMap<u32, Proc>;

/// What every process read asks the OS for: the executable and the
/// arguments (the parent and the start time come with every read).
/// Never the environment: the guardrail test checks this value.
#[must_use]
pub fn refresh_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing()
        .without_tasks()
        .with_exe(UpdateKind::Always)
        .with_cmd(UpdateKind::Always)
}

fn proc_of(pid: Pid, process: &sysinfo::Process) -> Proc {
    Proc {
        pid: pid.as_u32(),
        ppid: process.parent().map(Pid::as_u32),
        exe: process.exe().map(std::path::Path::to_path_buf),
        args: process
            .cmd()
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect(),
        start: i64::try_from(process.start_time())
            .ok()
            .filter(|at| *at > 0),
    }
}

/// Every process on this Mac (the full scan: at most once a minute).
#[must_use]
pub fn table() -> Table {
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind());
    system
        .processes()
        .iter()
        .map(|(pid, process)| (pid.as_u32(), proc_of(*pid, process)))
        .collect()
}

/// Only these processes, as they are now (liveness and start times; a
/// gone pid is absent).
#[must_use]
pub fn some(pids: &[u32]) -> Table {
    if pids.is_empty() {
        return Table::new();
    }
    let targets: Vec<Pid> = pids.iter().map(|pid| Pid::from_u32(*pid)).collect();
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&targets), false, refresh_kind());
    system
        .processes()
        .iter()
        .map(|(pid, process)| (pid.as_u32(), proc_of(*pid, process)))
        .collect()
}

/// `pid`'s ancestors in `table`, nearest first (at most 64: a cycle in a
/// stale table must not loop).
#[must_use]
pub fn ancestors(table: &Table, pid: u32) -> Vec<&Proc> {
    let mut out = Vec::new();
    let mut next = table.get(&pid).and_then(|process| process.ppid);
    while let Some(parent) = next {
        if out.len() >= 64 || parent == 0 {
            break;
        }
        let Some(process) = table.get(&parent) else {
            break;
        };
        out.push(process);
        next = process.ppid.filter(|ppid| *ppid != parent);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The guardrail (D-37): the process read never asks for
    /// environments.
    #[test]
    fn the_process_read_never_asks_for_environments() {
        let kind = refresh_kind();
        assert_eq!(kind.environ(), UpdateKind::Never);
        assert_eq!(kind.exe(), UpdateKind::Always);
        assert_eq!(kind.cmd(), UpdateKind::Always);
    }

    #[test]
    fn ancestors_stop_at_a_cycle_or_a_gap() {
        let proc = |pid, ppid| Proc {
            pid,
            ppid: Some(ppid),
            ..Proc::default()
        };
        let table: Table = [(3, proc(3, 2)), (2, proc(2, 1)), (1, proc(1, 0))]
            .into_iter()
            .collect();
        let chain: Vec<u32> = ancestors(&table, 3).iter().map(|p| p.pid).collect();
        assert_eq!(chain, [2, 1]);
        let cycle: Table = [(5, proc(5, 6)), (6, proc(6, 5))].into_iter().collect();
        assert!(ancestors(&cycle, 5).len() <= 64);
    }
}
