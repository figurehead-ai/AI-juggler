//! A process's CPU %, worked out from its CPU time over at least a second.
//!
//! sysinfo's own figure fails twice on macOS. It is recomputed only when the
//! process used some CPU since the previous refresh, so one that used none
//! keeps its last figure: a helper asleep in one blocking call read as its
//! start-up burst for good (5.4 %, CI on #167), and the idle nudge never
//! came. And its divisor is the time between the table's refreshes (only
//! updated 200 ms apart), not the process's: a pid refreshed again a few
//! milliseconds later, or only every 10 s, reads far too low or too high.
//! The accumulated CPU time is exact on every platform, so each refresh
//! notes it with the time it was read, and the figure is its growth over a
//! window of at least [`MIN_WINDOW`]. A reading may come from `sysinfo`'s
//! table or straight from the kernel's task record
//! ([`CpuSince::note_reading`], AJ-E805: the listing's observed trees).

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// The shortest window a figure is worked out over: CPU time comes in whole
/// milliseconds (10 ms clock ticks on Linux), so a second gives 0.1 % steps
/// (1 % on Linux).
pub const MIN_WINDOW: Duration = Duration::from_secs(1);

/// What one table's refreshes have seen of each process's CPU time.
#[derive(Debug, Default)]
pub struct CpuSince {
    seen: HashMap<u32, Seen>,
}

/// One reading: the CPU time so far (ns) and when it was read.
#[derive(Debug, Clone, Copy)]
struct Reading {
    cpu_ns: u64,
    at: Instant,
}

#[derive(Debug, Clone, Copy)]
struct Seen {
    start_time: u64,
    /// The window's start: at least [`MIN_WINDOW`] before `next` was taken.
    from: Option<Reading>,
    /// The next window start, once the latest reading is far enough on.
    next: Reading,
    latest: Reading,
}

impl CpuSince {
    /// Notes the processes just refreshed in `system` (`pids`); `at` is
    /// taken just before that refresh.
    pub fn note(&mut self, system: &sysinfo::System, pids: &[sysinfo::Pid], at: Instant) {
        for pid in pids {
            if let Some(process) = system.process(*pid) {
                self.note_one(process, at);
            }
        }
    }

    /// Forgets the processes `system` no longer holds (after a targeted
    /// refresh dropped the ones gone).
    pub fn forget_gone(&mut self, system: &sysinfo::System) {
        let processes = system.processes();
        self.seen
            .retain(|pid, _| processes.contains_key(&sysinfo::Pid::from_u32(*pid)));
    }

    /// Notes every process after a full refresh (`at`: just before it),
    /// and forgets the ones gone.
    pub fn note_all(&mut self, system: &sysinfo::System, at: Instant) {
        let processes = system.processes();
        self.seen
            .retain(|pid, _| processes.contains_key(&sysinfo::Pid::from_u32(*pid)));
        for process in processes.values() {
            self.note_one(process, at);
        }
    }

    fn note_one(&mut self, process: &sysinfo::Process, at: Instant) {
        self.note_reading(
            process.pid().as_u32(),
            process.start_time(),
            process.accumulated_cpu_time().saturating_mul(1_000_000),
            at,
        );
    }

    /// Notes one process's CPU time so far (`cpu_ns`) read at `at`; the
    /// process is told apart by its start time (epoch seconds).
    pub fn note_reading(&mut self, pid: u32, start_time: u64, cpu_ns: u64, at: Instant) {
        let reading = Reading { cpu_ns, at };
        let seen = match self.seen.get(&pid) {
            // Another process under a reused pid starts over.
            Some(seen) if seen.start_time == start_time => {
                let mut seen = *seen;
                seen.latest = reading;
                if at.duration_since(seen.next.at) >= MIN_WINDOW {
                    seen.from = Some(seen.next);
                    seen.next = reading;
                }
                seen
            }
            _ => Seen {
                start_time,
                from: None,
                next: reading,
                latest: reading,
            },
        };
        self.seen.insert(pid, seen);
    }

    /// Forgets every process but those `keep` holds.
    pub fn keep_only(&mut self, keep: impl Fn(u32) -> bool) {
        self.seen.retain(|pid, _| keep(*pid));
    }

    /// The process's CPU % (100 = one core) from the window's start to its
    /// latest reading; `None` until a window exists (never a fake idle).
    #[must_use]
    pub fn pct(&self, process: &sysinfo::Process) -> Option<f64> {
        self.pct_of(process.pid().as_u32(), process.start_time())
    }

    /// As [`CpuSince::pct`], for the process `pid` that started at
    /// `start_time` (epoch seconds).
    #[must_use]
    pub fn pct_of(&self, pid: u32, start_time: u64) -> Option<f64> {
        let seen = self
            .seen
            .get(&pid)
            .filter(|seen| seen.start_time == start_time)?;
        let from = seen.from?;
        let wall = seen.latest.at.duration_since(from.at).as_secs_f64();
        if wall <= 0.0 {
            return None;
        }
        let cpu = seen.latest.cpu_ns.saturating_sub(from.cpu_ns);
        #[allow(clippy::cast_precision_loss)] // CPU ns never nears 2^52 within a window
        Some(cpu as f64 / 1e9 / wall * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A busy process reads busy, and once asleep in one blocking call it
    /// reads 0 % (sysinfo's own figure kept its last busy reading on macOS).
    #[test]
    fn a_process_reads_busy_then_zero_once_asleep() {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
        // Busy until the test says stop (bounded, no forks), then asleep.
        let dir = tempfile::tempdir().unwrap();
        let stop = dir.path().join("stop");
        let child = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(format!(
                "i=0; while [ ! -e '{}' ] && [ $i -lt 20000000 ]; do i=$((i+1)); done; exec sleep 30",
                stop.display()
            ))
            .spawn()
            .unwrap();
        let pid = sysinfo::Pid::from_u32(child.id());
        let mut system = sysinfo::System::new();
        let mut cpu = CpuSince::default();
        // Killed however the test ends: the spin never outlives a failure.
        struct Reap(std::process::Child);
        impl Drop for Reap {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let child = Reap(child);
        let mut read = || {
            let at = Instant::now();
            system.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[pid]),
                true,
                ProcessRefreshKind::nothing().with_cpu(),
            );
            cpu.note(&system, &[pid], at);
            let process = system.process(pid).unwrap();
            (cpu.pct(process), f64::from(process.cpu_usage()))
        };
        assert_eq!(read().0, None, "one reading is no window");
        let mut busy = 0.0_f64;
        for _ in 0..15 {
            std::thread::sleep(Duration::from_millis(100));
            busy = busy.max(read().0.unwrap_or(0.0));
        }
        // A second refresh a moment later (the Processes view re-reads the
        // pids a tick just read) keeps the window: still busy, never ~0.
        let again = read().0.unwrap_or(0.0);
        std::fs::write(&stop, "").unwrap();
        // Asleep: a window that holds only the sleep reads 0 (bounded wait).
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut last = read();
        while last.0 != Some(0.0) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(200));
            last = read();
        }
        drop(child);
        assert!(busy > 10.0, "the busy phase read {busy} %");
        assert!(
            again > 10.0,
            "re-read at once, the busy phase read {again} %"
        );
        assert_eq!(
            last.0,
            Some(0.0),
            "asleep but read {:?} % (sysinfo's own figure: {} %)",
            last.0,
            last.1
        );
    }
}
