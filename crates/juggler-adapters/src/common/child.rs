//! A tool's process AI Juggler starts for itself — a server for one
//! request, a command it reads, a daemon it owns — ended and reaped
//! however it went: asked to end, given a grace, then killed, never left
//! behind (AJ-E1607: Hermes' gateway forks its own workers, so it is ended
//! by its process group).

use std::io::Read as _;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// How long an ended process may take before it is killed.
pub(crate) const STOP_GRACE: Duration = Duration::from_secs(3);

/// A started process, ended and reaped when dropped. With `group`, it
/// leads a process group of its own and the whole group is ended.
pub(crate) struct Reaped {
    child: Child,
    group: bool,
    /// Ended and reaped already (its pid may be another's now).
    done: bool,
}

impl Reaped {
    /// `child`, ended alone.
    pub(crate) fn new(child: Child) -> Self {
        Self {
            child,
            group: false,
            done: false,
        }
    }

    /// `command` started as the leader of a process group of its own, so
    /// everything it forks ends with it.
    ///
    /// # Errors
    ///
    /// The spawn's own error.
    pub(crate) fn spawn_group(command: &mut Command) -> std::io::Result<Self> {
        juggler_adapter::process::own_group(command);
        command.spawn().map(|child| Self {
            child,
            group: true,
            done: false,
        })
    }

    /// The process.
    pub(crate) fn child(&mut self) -> &mut Child {
        &mut self.child
    }

    /// Its process id (its group's, when it leads one).
    pub(crate) fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Whether it still runs.
    pub(crate) fn running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Ends it: asked, given [`STOP_GRACE`], then killed; then reaped.
    /// With a group, the group is asked and killed too, so a worker that
    /// outlived its leader goes with it.
    pub(crate) fn end(&mut self) {
        if self.done {
            return;
        }
        let leader = self.pid();
        if self.running() {
            let _ = if self.group {
                juggler_adapter::process::end_group(leader, false)
            } else {
                juggler_adapter::process::terminate(&self.child)
            };
            let until = Instant::now() + STOP_GRACE;
            while self.running() && Instant::now() < until {
                // poll: the process ending after SIGTERM, bounded by the
                // grace (3 s), once per process AI Juggler started.
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        if self.running() {
            let _ = self.child.kill();
        }
        if self.group {
            // Whatever of the group is still there (a worker the leader
            // left), killed; an empty group answers with an error, ignored.
            let _ = juggler_adapter::process::end_group(leader, true);
        }
        let _ = self.child.wait();
        self.done = true;
    }
}

impl Drop for Reaped {
    fn drop(&mut self) {
        self.end();
    }
}

/// How a bounded command ended.
pub(crate) struct Output {
    /// Its exit, `None` when it ran past the deadline (it was ended).
    pub(crate) status: Option<ExitStatus>,
    /// At most `max_bytes` of what it printed.
    pub(crate) stdout: String,
}

/// Runs `command` (its stdin closed, stderr dropped) until it ends or
/// `deadline` passes, reading at most `max_bytes` of its output; it is
/// ended and reaped either way, with its process group, so nothing it
/// forked keeps its output open.
///
/// # Errors
///
/// When it can't be started, or has no output to read.
pub(crate) fn output_within(
    command: &mut Command,
    deadline: Instant,
    max_bytes: u64,
) -> std::io::Result<Output> {
    let mut started = Reaped::spawn_group(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    )?;
    let mut stdout = started
        .child()
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("no output"))?;
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = (&mut stdout).take(max_bytes).read_to_string(&mut text);
        text
    });
    while started.running() && Instant::now() < deadline {
        // poll: the command ending, bounded by its deadline, once per run.
        std::thread::sleep(Duration::from_millis(50));
    }
    let status = started.child().try_wait().ok().flatten();
    drop(started);
    Ok(Output {
        status,
        stdout: reader.join().unwrap_or_default(),
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn a_command_past_its_deadline_is_ended() {
        let started = Instant::now();
        let out = output_within(
            Command::new("/bin/sh").args(["-c", "echo partial; sleep 30"]),
            Instant::now() + Duration::from_millis(300),
            1024,
        )
        .unwrap();
        assert!(out.status.is_none(), "ran past its deadline");
        assert_eq!(out.stdout.trim(), "partial");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn a_command_that_ends_gives_its_exit_and_output() {
        let out = output_within(
            Command::new("/bin/sh").args(["-c", "echo done"]),
            Instant::now() + Duration::from_secs(10),
            1024,
        )
        .unwrap();
        assert!(out.status.is_some_and(|status| status.success()));
        assert_eq!(out.stdout.trim(), "done");
    }

    #[test]
    fn a_group_is_ended_with_the_worker_its_leader_forked() {
        let dir = tempfile::tempdir().unwrap();
        let pid_file = dir.path().join("worker.pid");
        // The leader forks a worker that ignores SIGTERM, then waits.
        let script = format!(
            "sh -c 'trap \"\" TERM; echo $$ > {}; sleep 60' & wait",
            pid_file.display()
        );
        let mut leader =
            Reaped::spawn_group(Command::new("/bin/sh").args(["-c", &script])).unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while !pid_file.exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(20));
        }
        let worker: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!(juggler_adapter::process::alive(worker));
        leader.end();
        let until = Instant::now() + Duration::from_secs(5);
        while juggler_adapter::process::alive(worker) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            !juggler_adapter::process::alive(worker),
            "the worker went with its group"
        );
    }
}
