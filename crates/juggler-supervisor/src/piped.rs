//! The piped spawn (headless runs) and the driven spawn (AJ-E2302): a
//! child of `supervisor.rs`, which keeps the rest of the lifecycle.

#![allow(clippy::wildcard_imports)]

use super::*;

/// AJ-E2302: a driven session's stdin and stdout, for its runtime's codec
/// (the supervisor still owns the process, its group and its exit).
#[derive(Debug)]
pub struct DriverPipes {
    /// What AI Juggler writes to the tool.
    pub stdin: std::process::ChildStdin,
    /// What the tool writes to AI Juggler.
    pub stdout: std::process::ChildStdout,
}

impl Supervisor {
    /// AJ-E2302 (ADR-0022): spawns a driven session whose protocol runs on
    /// its stdin and stdout, and hands both to the caller (the runtime's
    /// codec). It is owned exactly as any other session: an intent row
    /// before the spawn, its own process group, the journal, the wait
    /// thread and the stop ladder; its stderr goes to the output ring.
    ///
    /// # Errors
    ///
    /// [`SupervisorError::Spawn`] when the process cannot start, or the
    /// spec asks for a PTY (a driven session never has one).
    pub fn spawn_driven(
        &self,
        session: &SessionId,
        spec: &LaunchSpec,
        ctx: &crate::tree::SpawnContext,
    ) -> Result<(SpawnedProcess, DriverPipes), SupervisorError> {
        if spec.needs_pty {
            return Err(SupervisorError::Spawn {
                program: spec.program.clone(),
                message: "a driven session runs without a terminal".into(),
            });
        }
        let (spawned, pipes) = self.spawn_owned(session, spec, ctx, true)?;
        let pipes = pipes.ok_or_else(|| SupervisorError::Spawn {
            program: spec.program.clone(),
            message: "the session's pipes couldn't be opened".into(),
        })?;
        Ok((spawned, pipes))
    }

    /// A piped child; with `driver`, its stdin and stdout go to the
    /// caller instead of null and the ring.
    #[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
    pub(super) fn spawn_piped(
        &self,
        session: &SessionId,
        spec: &LaunchSpec,
        driver: bool,
    ) -> Result<(SpawnedProcess, Option<DriverPipes>), SupervisorError> {
        let mut command = piped_command(session, spec, driver);
        let child = command.spawn().map_err(|e| SupervisorError::Spawn {
            program: spec.program.clone(),
            message: e.to_string(),
        })?;
        let pid = child.id();
        let pgid = group::pgid_of(pid).ok();
        let identity = Self::spawn_identity(pid, pgid);
        let ring = Arc::new(Mutex::new(OutputRing::new(DEFAULT_RING_CAPACITY)));
        let child = Arc::new(Mutex::new(child));
        let finished = Arc::new(AtomicBool::new(false));
        let kill_child = Arc::clone(&child);
        let force_kill: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = kill_child.lock().map(|mut c| c.kill());
        });
        // Register before readers start so output tracking exists from
        // the first chunk (AJ-E405).
        let exit_at = Arc::new(AtomicI64::new(0));
        let handle = self.register(
            session.clone(),
            pid,
            Arc::clone(&ring),
            Arc::clone(&finished),
            force_kill,
            ChildExtras {
                pty: None,
                last_output_ms: Arc::new(std::sync::atomic::AtomicI64::new(0)),
                pgid,
                identity: identity.clone(),
                exit_at_ms: Arc::clone(&exit_at),
            },
        );
        let last_output = Arc::clone(&handle.last_output_ms);
        let (out_atomic, err_atomic) = (Arc::clone(&last_output), last_output);
        let (out_tx, err_tx) = (
            std::sync::mpsc::SyncSender::clone(&self.activity_tx),
            std::sync::mpsc::SyncSender::clone(&self.activity_tx),
        );
        let (out_session, err_session) = (session.clone(), session.clone());
        let (drained_tx, drained) = std::sync::mpsc::channel();
        let pipes = if driver {
            let mut locked = child_lock(&child);
            locked
                .stdin
                .take()
                .zip(locked.stdout.take())
                .map(|(stdin, stdout)| DriverPipes { stdin, stdout })
        } else {
            spawn_reader(
                child_lock(&child).stdout.take(),
                Arc::clone(&ring),
                out_atomic,
                out_session,
                out_tx,
                drained_tx.clone(),
            );
            None
        };
        spawn_reader(
            child_lock(&child).stderr.take(),
            ring,
            err_atomic,
            err_session,
            err_tx,
            drained_tx,
        );

        self.watch_exit(session, pid, child, &finished, exit_at, drained);

        Ok((
            SpawnedProcess {
                session: session.clone(),
                pid,
                pgid,
                identity,
            },
            pipes,
        ))
    }

    /// The wait thread: the exit is reported once the output has drained.
    fn watch_exit(
        &self,
        session: &SessionId,
        pid: u32,
        child: Arc<Mutex<std::process::Child>>,
        finished: &Arc<AtomicBool>,
        exit_at: Arc<AtomicI64>,
        drained: std::sync::mpsc::Receiver<()>,
    ) {
        let session_for_thread = session.clone();
        let journal = self.journal.clone();
        let ledger = self.ledger.clone();
        let events = self.events_tx.clone();
        let done = Arc::clone(finished);
        std::thread::spawn(move || {
            let status = child.lock().ok().and_then(|mut c| c.wait().ok());
            if let Some(status) = status {
                emit_exit(
                    &session_for_thread,
                    pid,
                    exit_kind_std(status),
                    &done,
                    &exit_at,
                    journal.as_ref(),
                    ledger.as_ref(),
                    &events,
                    Some(&drained),
                );
            } else {
                wait_for_output(&drained);
                done.store(true, Ordering::Release);
            }
        });
    }
}

/// The piped child's command: the overlay's environment, its own process
/// group, stdin piped for a driver (else null), stdout and stderr piped.
fn piped_command(session: &SessionId, spec: &LaunchSpec, driver: bool) -> std::process::Command {
    use std::process::{Command, Stdio};

    let mut command = Command::new(&spec.program);
    command.env("PATH", launch_path(&spec.program));
    apply_env(&mut command, &spec.env);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .stdin(if driver {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // AJ-E502: the piped root becomes its OWN group leader at
    // spawn — every in-group helper is later reachable as one
    // owned group (deterministic kill), not a ppid-walk snapshot.
    // Where groups do not exist, spawn proceeds groupless (the
    // honest `None`, never a pretended group).
    if let Err(group::GroupError::UnsupportedPlatform) = group::make_group_leader(&mut command) {
        eprintln!(
            "ai-juggler: process groups unsupported; tracking pid-only for {}",
            session.as_str()
        );
    }
    command
}
