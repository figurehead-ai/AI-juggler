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

/// Every process on this Mac: its executable, arguments, parent and start
/// time (presence's look every 5 s and on a launch or quit, the live
/// view's full scan, the places' reads).
///
/// AJ-E805 (0.4.5, idle CPU): the table is kept between reads. Each read
/// lists every process cheaply (`juggler_proc_list`: pid, parent, start
/// time, name; one system call) and reads the executable and arguments
/// only of the processes it hasn't seen before: a pid with the same start
/// time and name is the same process, and a process's arguments don't
/// change while it runs. Before, each read asked `sysinfo` for every
/// process's arguments and three more records of each (about 850
/// processes): presence's 5 s look alone was a sixth of the app's idle
/// CPU. Every [`WHOLE_EVERY`] everything is read again, and wherever the
/// cheap listing isn't there, every read is whole, as before.
#[must_use]
pub fn table() -> Table {
    let mut table = KEPT.lock().map_or_else(
        |_| whole(),
        |mut kept| kept.read(juggler_proc_list::list(), std::time::Instant::now(), &some),
    );
    // ADR-0027: a lab's write mode sees only the lab's processes.
    crate::lab_scope::keep(&mut table);
    table
}

/// Every process read whole (`sysinfo`, a fresh table).
fn whole() -> Table {
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind());
    system
        .processes()
        .iter()
        .map(|(pid, process)| (pid.as_u32(), proc_of(*pid, process)))
        .collect()
}

/// How often the kept table is read whole anyway: a safety net for what
/// the listing can't tell apart (a program that replaces itself with one
/// of the same name keeps its pid, start time and name).
pub const WHOLE_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

/// What tells one process from another in the listing: a pid reused, or
/// a process replaced by another program, reads as a new process.
type Identity = (u64, u64, String);

/// The table kept between reads (one per app: every reader shares it).
#[derive(Default)]
struct Kept {
    /// The processes read so far, with what told each apart.
    table: HashMap<u32, (Identity, Proc)>,
    /// When everything was last read whole.
    whole_at: Option<std::time::Instant>,
    /// Processes read whole since the start (the tests' measure).
    reads: u64,
}

static KEPT: std::sync::LazyLock<std::sync::Mutex<Kept>> =
    std::sync::LazyLock::new(std::sync::Mutex::default);

impl Kept {
    /// The table now, from a fresh `listing` (`None`: read it whole), with
    /// `read` reading the processes it names.
    fn read(
        &mut self,
        listing: Option<Vec<juggler_proc_list::Listed>>,
        now: std::time::Instant,
        read: &dyn Fn(&[u32]) -> Table,
    ) -> Table {
        let Some(listing) = listing else {
            self.table.clear();
            let table = whole();
            self.reads += table.len() as u64;
            return table;
        };
        if self
            .whole_at
            .is_none_or(|at| now.saturating_duration_since(at) >= WHOLE_EVERY)
        {
            self.table.clear();
            self.whole_at = Some(now);
        }
        let listing: Vec<(Identity, juggler_proc_list::Listed)> = listing
            .into_iter()
            .filter(|listed| !listed.zombie)
            .map(|listed| {
                (
                    (listed.start_secs, listed.start_micros, listed.name.clone()),
                    listed,
                )
            })
            .collect();
        let new: Vec<u32> = listing
            .iter()
            .filter(|(identity, listed)| {
                self.table
                    .get(&listed.pid)
                    .is_none_or(|(known, _)| known != identity)
            })
            .map(|(_, listed)| listed.pid)
            .collect();
        let mut fresh = if new.is_empty() {
            Table::new()
        } else {
            read(&new)
        };
        self.reads += new.len() as u64;
        let mut kept = HashMap::with_capacity(listing.len());
        for (identity, listed) in listing {
            let mut process = match fresh.remove(&listed.pid) {
                // Read just now: the same process the listing saw (a pid
                // reused in between is read again next time).
                Some(process) if process.start == i64::try_from(listed.start_secs).ok() => process,
                Some(_) => continue,
                None => match self.table.remove(&listed.pid) {
                    Some((known, process)) if known == identity => process,
                    // A process `sysinfo` couldn't read (gone since, or
                    // not ours to read) isn't in the table, as before.
                    _ => continue,
                },
            };
            // Its parent as it is now (a parent that ended hands its
            // children to launchd), as `sysinfo` reads it.
            process.ppid = (listed.ppid != 0).then_some(listed.ppid);
            kept.insert(listed.pid, (identity, process));
        }
        self.table = kept;
        self.table
            .iter()
            .map(|(pid, (_, process))| (*pid, process.clone()))
            .collect()
    }
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

    fn listed(pid: u32, ppid: u32, start: u64, name: &str) -> juggler_proc_list::Listed {
        juggler_proc_list::Listed {
            pid,
            ppid,
            start_secs: start,
            start_micros: 7,
            name: name.into(),
            zombie: false,
        }
    }

    /// AJ-E805: a read after the first reads only the processes that are
    /// new to it (a new pid, a reused one, another program in its place);
    /// the rest keep what was read, with their parent as listed now; every
    /// [`WHOLE_EVERY`] it reads them all again. It read every process,
    /// arguments and all, on every read (presence's 5 s look: a sixth of
    /// the app's idle CPU in the lab).
    #[test]
    fn a_read_after_the_first_reads_only_what_is_new() {
        let asked = std::cell::RefCell::new(Vec::<Vec<u32>>::new());
        let reader = |pids: &[u32]| -> Table {
            asked.borrow_mut().push(pids.to_vec());
            pids.iter()
                .map(|pid| {
                    let start = if *pid == 10 && asked.borrow().len() > 2 {
                        200
                    } else {
                        100
                    };
                    (
                        *pid,
                        Proc {
                            pid: *pid,
                            ppid: Some(1),
                            exe: Some(PathBuf::from(format!("/bin/p{pid}"))),
                            args: vec![format!("p{pid}"), "--x".into()],
                            start: Some(start),
                        },
                    )
                })
                .collect()
        };
        let t0 = std::time::Instant::now();
        let at = |secs| t0 + std::time::Duration::from_secs(secs);
        let mut kept = Kept::default();
        let first = vec![listed(10, 1, 100, "a"), listed(11, 10, 100, "b")];
        let table = kept.read(Some(first.clone()), at(0), &reader);
        assert_eq!(table.len(), 2);
        assert_eq!(table[&11].args, ["p11", "--x"]);
        // Nothing new: nothing read.
        let again = kept.read(Some(first), at(2), &reader);
        assert_eq!(again, table);
        // A new process is read; one whose parent ended is under launchd
        // now, without being read again; a zombie is never read or kept.
        let mut zombie = listed(13, 10, 100, "z");
        zombie.zombie = true;
        let next = vec![
            listed(10, 1, 100, "a"),
            listed(11, 1, 100, "b"),
            listed(12, 11, 100, "c"),
            zombie,
        ];
        let table = kept.read(Some(next), at(4), &reader);
        assert_eq!(table.len(), 3);
        assert_eq!(table[&11].ppid, Some(1));
        // A pid reused by another process is read again.
        let reused = vec![
            listed(10, 1, 200, "a"),
            listed(11, 1, 100, "b"),
            listed(12, 11, 100, "c"),
        ];
        let table = kept.read(Some(reused.clone()), at(6), &reader);
        assert_eq!(table[&10].start, Some(200));
        // A gone process is gone.
        let table = kept.read(Some(reused[..2].to_vec()), at(8), &reader);
        assert!(!table.contains_key(&12));
        // Past WHOLE_EVERY, everything is read again.
        kept.read(
            Some(reused[..2].to_vec()),
            at(8 + WHOLE_EVERY.as_secs()),
            &reader,
        );
        let mut asked = asked.into_inner();
        for pids in &mut asked {
            pids.sort_unstable();
        }
        assert_eq!(asked, vec![vec![10, 11], vec![12], vec![10], vec![10, 11]]);
        assert_eq!(kept.reads, 6);
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
