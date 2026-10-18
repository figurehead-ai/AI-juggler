//! Ledger types + the storage-agnostic seam (AJ-E501, pass2 §12): the
//! supervisor speaks to [`LedgerStore`]; the SQL lives behind
//! [`juggler_core::storage::ProcessLedgerRepo`] and is bridged by
//! [`CoreLedgerStore`]. The JSONL journal stays a crash log — this is
//! the query store that survives root exit, app exit, and reboot.

use std::sync::Arc;

use juggler_core::model::{MachineId, SessionId};
use juggler_core::storage::{LedgerRow, ProcessLedgerRepo};

/// Why the spawn context exists: ownership (provider/account/project/
/// machine) is known by the CALLER, not the supervisor — the ledger
/// records it intent-first so orphans stay attributable after exit.
#[derive(Debug, Clone)]
pub struct SpawnContext {
    /// Provider id.
    pub provider: String,
    /// Owning Account id, when one exists (login terminals may not).
    pub account_id: Option<String>,
    /// Owning project id.
    pub project_id: Option<String>,
    /// Machine identity (D-13; the stable local id for local rows —
    /// never a hard-coded literal at call sites).
    pub machine: MachineId,
    /// `root` | `login` (helpers are upserted by the tick, role set by
    /// the store).
    pub role: &'static str,
}

/// Ledger failures are HEALTH, not spawn failures (Req.3): every
/// method returns `Result` and the supervisor degrades — spawns never
/// fail because the ledger did.
#[derive(Debug)]
pub enum LedgerError {
    /// The underlying store failed (SQL/I/O); text for the log.
    Store(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(message) => write!(f, "ledger store: {message}"),
        }
    }
}

/// The supervisor-side ledger contract (pass2 §12): `intent` before
/// spawn, `spawned` to patch the pid, batched `upsert_helpers` per
/// reconciler tick, `mark_exited` from wait threads, `candidates` for
/// orphan detection (AJ-E504), `prune` for retention.
pub trait LedgerStore: Send + Sync {
    /// This machine's boot identity (OS boot time, epoch seconds) —
    /// every row and exit patch carries it.
    fn boot_id(&self) -> String;

    /// Writes the Intent row (state `Starting`, no PID).
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] when the store fails — never panics,
    /// never fails the spawn.
    fn intent(&self, row: &LedgerRow) -> Result<(), LedgerError>;

    /// Patches the session's newest Intent row with pid/pgid/OS start
    /// time; state → `running`.
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] on store failure.
    #[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
    fn spawned(
        &self,
        session: &SessionId,
        pid: u32,
        pgid: Option<u32>,
        started_at: Option<i64>,
    ) -> Result<(), LedgerError>;

    /// Batched helper upsert (one transaction per tick).
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] on store failure.
    fn upsert_helpers(&self, session: &SessionId, rows: &[LedgerRow]) -> Result<(), LedgerError>;

    /// Patches one row's exit classification.
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] on store failure.
    fn mark_exited(
        &self,
        pid: u32,
        boot_id: &str,
        exit_kind: &str,
        reaped_by_us: bool,
    ) -> Result<(), LedgerError>;

    /// Running rows on a machine — the orphan-candidate input.
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] on store failure.
    fn candidates(&self, machine: &MachineId) -> Result<Vec<LedgerRow>, LedgerError>;

    /// Flips a running row to the permanent `unverifiable` state
    /// (AJ-E503 Req.5): identity verification failed, so the row
    /// leaves every kill path and the auto-tier candidate set
    /// forever — `candidates` selects `running` only, and the prune
    /// touches `exited` only.
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] on store failure.
    fn mark_unverifiable(&self, pid: u32, boot_id: &str) -> Result<(), LedgerError>;

    /// Flips a running helper row to the `orphaned` state (AJ-E504
    /// Req.3): root exited, grace elapsed, two consecutive ticks
    /// agreed. The row stays a candidate (re-verified each tick).
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] on store failure.
    fn mark_orphaned(&self, pid: u32, boot_id: &str) -> Result<(), LedgerError>;

    /// Flips a running helper row to `orphaned` AND sets the durable
    /// `down_period` marker (AJ-E510): the helper survived, but its
    /// root died while Juggler itself was closed. Only recovery
    /// writes this — the tick ladder uses [`LedgerStore::mark_orphaned`].
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] on store failure.
    fn mark_orphaned_down(&self, pid: u32, boot_id: &str) -> Result<(), LedgerError>;

    /// Every row of one session, all states (the reconciler's
    /// root-exit facts + tests/evidence dumps).
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] on store failure.
    fn session_rows(&self, session: &SessionId) -> Result<Vec<LedgerRow>, LedgerError>;

    /// The FLEET composite-key read (AJ-E511): one session's rows on
    /// ONE machine — the join key for every cross-machine query.
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] on store failure.
    fn rows_for_session_on_machine(
        &self,
        session: &SessionId,
        machine: &MachineId,
    ) -> Result<Vec<LedgerRow>, LedgerError>;

    /// Every LIVE (session, machine) composite key (AJ-E511) — the
    /// all-machines inventory's walk.
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] on store failure.
    fn live_session_machines(&self) -> Result<Vec<(String, String)>, LedgerError>;

    /// Batched listener join (AJ-E505): sets `listens_on` per row in
    /// ONE transaction per tick.
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] on store failure.
    fn apply_ports(&self, entries: &[(u32, &str, &str)]) -> Result<(), LedgerError>;

    /// Retention prune (exited rows only).
    ///
    /// # Errors
    ///
    /// [`LedgerError::Store`] on store failure.
    fn prune(&self, exited_before_ms: i64) -> Result<usize, LedgerError>;
}

/// Bridges the core SQLite repo into [`LedgerStore`] — the ONLY SQL
/// implementation; the supervisor itself stays storage-agnostic.
#[derive(Debug)]
pub struct CoreLedgerStore {
    repo: ProcessLedgerRepo,
    /// Boot identity for this process's lifetime (OS boot time, epoch
    /// seconds — makes `(pid, started_at)` unique across reboots).
    boot_id: String,
}

impl CoreLedgerStore {
    /// Wraps an open repo; the boot id is the OS boot time (epoch
    /// seconds), probed once per process — it cannot change under a
    /// running process.
    #[must_use]
    pub fn new(repo: ProcessLedgerRepo) -> Self {
        Self {
            repo,
            boot_id: sysinfo::System::boot_time().to_string(),
        }
    }

    /// This store's boot id (tests assert it is stable per process).
    #[must_use]
    pub fn boot_id(&self) -> &str {
        &self.boot_id
    }

    /// Rows for one session (tests, the Processes view, evidence
    /// dumps).
    ///
    /// # Errors
    ///
    /// [`juggler_core::error::CoreError`] on store failure.
    pub fn rows_for_session(
        &self,
        session: &SessionId,
    ) -> Result<Vec<LedgerRow>, juggler_core::error::CoreError> {
        self.repo.rows_for_session(session.as_str())
    }
}

use std::time::{Duration, SystemTime};

impl LedgerStore for CoreLedgerStore {
    fn boot_id(&self) -> String {
        self.boot_id.clone()
    }

    fn intent(&self, row: &LedgerRow) -> Result<(), LedgerError> {
        self.repo
            .intent(row)
            .map_err(|e| LedgerError::Store(e.to_string()))
    }

    #[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
    fn spawned(
        &self,
        session: &SessionId,
        pid: u32,
        pgid: Option<u32>,
        started_at: Option<i64>,
    ) -> Result<(), LedgerError> {
        self.repo
            .spawned(session.as_str(), pid, pgid, started_at, now_ms())
            .map_err(|e| LedgerError::Store(e.to_string()))
    }

    fn upsert_helpers(&self, session: &SessionId, rows: &[LedgerRow]) -> Result<(), LedgerError> {
        self.repo
            .upsert_helpers(session.as_str(), rows)
            .map_err(|e| LedgerError::Store(e.to_string()))
    }

    fn mark_exited(
        &self,
        pid: u32,
        boot_id: &str,
        exit_kind: &str,
        reaped_by_us: bool,
    ) -> Result<(), LedgerError> {
        self.repo
            .mark_exited(pid, boot_id, exit_kind, reaped_by_us, now_ms())
            .map_err(|e| LedgerError::Store(e.to_string()))
    }

    fn candidates(&self, machine: &MachineId) -> Result<Vec<LedgerRow>, LedgerError> {
        self.repo
            .candidates(machine.as_str())
            .map_err(|e| LedgerError::Store(e.to_string()))
    }

    fn mark_unverifiable(&self, pid: u32, boot_id: &str) -> Result<(), LedgerError> {
        self.repo
            .mark_unverifiable(pid, boot_id, now_ms())
            .map_err(|e| LedgerError::Store(e.to_string()))
    }

    fn mark_orphaned(&self, pid: u32, boot_id: &str) -> Result<(), LedgerError> {
        self.repo
            .mark_orphaned(pid, boot_id, now_ms())
            .map_err(|e| LedgerError::Store(e.to_string()))
    }

    fn mark_orphaned_down(&self, pid: u32, boot_id: &str) -> Result<(), LedgerError> {
        self.repo
            .mark_orphaned_down(pid, boot_id, now_ms())
            .map_err(|e| LedgerError::Store(e.to_string()))
    }

    fn session_rows(&self, session: &SessionId) -> Result<Vec<LedgerRow>, LedgerError> {
        self.repo
            .rows_for_session(session.as_str())
            .map_err(|e| LedgerError::Store(e.to_string()))
    }

    fn rows_for_session_on_machine(
        &self,
        session: &SessionId,
        machine: &MachineId,
    ) -> Result<Vec<LedgerRow>, LedgerError> {
        self.repo
            .rows_for_session_on_machine(session.as_str(), machine.as_str())
            .map_err(|e| LedgerError::Store(e.to_string()))
    }

    fn live_session_machines(&self) -> Result<Vec<(String, String)>, LedgerError> {
        self.repo
            .live_session_machines()
            .map_err(|e| LedgerError::Store(e.to_string()))
    }

    fn apply_ports(&self, entries: &[(u32, &str, &str)]) -> Result<(), LedgerError> {
        self.repo
            .apply_ports(entries)
            .map_err(|e| LedgerError::Store(e.to_string()))
    }

    fn prune(&self, exited_before_ms: i64) -> Result<usize, LedgerError> {
        self.repo
            .prune(exited_before_ms)
            .map_err(|e| LedgerError::Store(e.to_string()))
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Computes the display/hash fields of a command line (identity key —
/// `DefaultHasher`, not cryptography).
#[must_use]
pub fn cmdline_fields(program: &std::path::Path, args: &[String]) -> (String, String) {
    use std::hash::{Hash, Hasher};
    let mut line = program.to_string_lossy().into_owned();
    for arg in args {
        line.push(' ');
        line.push_str(arg);
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    line.hash(&mut hasher);
    let trunc: String = line.chars().take(200).collect();
    (format!("{:016x}", hasher.finish()), trunc)
}

/// Convenience alias for the shared store handle.
pub type SharedLedger = Arc<dyn LedgerStore>;

/// The identity of a process we own (AJ-E503, pass2 §12 / §6.2): a
/// PID is a NAME, not an identity — this tuple is. `start_time` is
/// the OS process start (epoch s, fork-time — exec does not change
/// it), `exe` the kernel-reported executable (resolved; may legitimately
/// change across an `exec`, so both the pre- and post-settle records
/// count as ours), `boot_id` the machine's boot identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessIdentity {
    /// OS process id (the name, kept for correlation).
    pub pid: u32,
    /// Process-group id (`None` where the platform lacks groups).
    pub pgid: Option<u32>,
    /// OS process start time, epoch seconds — the reuse discriminator.
    pub start_time: i64,
    /// Kernel-reported executable path at record time.
    pub exe: std::path::PathBuf,
    /// OS boot time, epoch seconds as text — same-machine-same-boot.
    pub boot_id: String,
}

/// Fail-closed verification outcome (§6.2): `Unverifiable` covers
/// every mismatch AND every probe error — the caller MUST refuse to
/// touch the process; `Verified` is only ever earned by full
/// agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verification {
    /// All recorded facts still name our process.
    Verified,
    /// Mismatch or probe error — the PID may belong to a stranger.
    Unverifiable,
}

/// What the OS says about a pid right now (the verify probe's facts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbedProcess {
    /// Live OS start time, epoch seconds.
    pub start_time: i64,
    /// Live kernel-reported executable path.
    pub exe: std::path::PathBuf,
}

/// The live-facts seam behind [`verify`]: production reads sysinfo
/// and the boot clock; tests forge results (the quality-gate
/// single-field mismatch tests).
pub trait IdentityProbe {
    /// Probes one pid; `None` is a PROBE ERROR (gone, unreadable exe,
    /// anything) — never a verdict of innocence.
    fn probe(&self, pid: u32) -> Option<ProbedProcess>;
    /// The machine's current boot identity.
    fn boot_id(&self) -> String;
}

/// The real probe: one targeted sysinfo refresh (start time + exe —
/// the §6.2 `stat`/`proc_pidinfo` cost) and the boot clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsIdentityProbe;

impl IdentityProbe for OsIdentityProbe {
    fn probe(&self, pid: u32) -> Option<ProbedProcess> {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
        let mut system = crate::new_system();
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]),
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
        );
        let process = system.process(sysinfo::Pid::from_u32(pid))?;
        // A missing exe is a probe error (fail closed), not a pass.
        let exe = process.exe()?.to_path_buf();
        Some(ProbedProcess {
            start_time: i64::try_from(process.start_time()).ok()?,
            exe,
        })
    }

    fn boot_id(&self) -> String {
        sysinfo::System::boot_time().to_string()
    }
}

/// Verifies `pid` against the recorded [`ProcessIdentity`] using the
/// real OS probe (§6.2, fail closed).
#[must_use]
pub fn verify(pid: u32, expect: &ProcessIdentity) -> Verification {
    verify_with(&OsIdentityProbe, pid, expect)
}

/// The injectable core: ALL of (a) live start-time, (b) live exe,
/// (c) current boot id must equal the record. The boot gate runs
/// FIRST (FMEA C8): a reboot invalidates start-time comparisons as a
/// class, not field by field. Any mismatch — or any probe error —
/// is `Unverifiable`; errors are NEVER `Verified`.
pub fn verify_with(probe: &dyn IdentityProbe, pid: u32, expect: &ProcessIdentity) -> Verification {
    // (c) first: a different boot means the record predates the
    // machine's current life — every other comparison is meaningless.
    if probe.boot_id() != expect.boot_id {
        return Verification::Unverifiable;
    }
    // A probe error is a refusal, not an acquittal.
    let Some(probed) = probe.probe(pid) else {
        return Verification::Unverifiable;
    };
    if probed.start_time != expect.start_time {
        return Verification::Unverifiable;
    }
    if probed.exe != expect.exe {
        return Verification::Unverifiable;
    }
    Verification::Verified
}

/// Boot + start-time verification WITHOUT the exe comparison — for
/// settled root identities whose exe legitimately moved across
/// `exec` (the E503/E504 aliasing rule: a `/bin/sh` launch records
/// `/bin/sh`, the kernel later reads `/bin/bash`). Sound for live
/// handles we own: an un-reaped child's pid cannot be reused, so
/// `(boot, start_time)` alone names the process.
#[must_use]
pub fn verify_start_time(pid: u32, expect: &ProcessIdentity) -> Verification {
    let probe = OsIdentityProbe;
    if probe.boot_id() != expect.boot_id {
        return Verification::Unverifiable;
    }
    let Some(probed) = probe.probe(pid) else {
        return Verification::Unverifiable;
    };
    if probed.start_time == expect.start_time {
        Verification::Verified
    } else {
        Verification::Unverifiable
    }
}

/// Adoption replay helper: a pid passes when ANY recorded identity
/// fully verifies (the spawn-time record AND the post-`exec` settled
/// record both name our process — a stranger can match neither,
/// because start-time must agree in the matched record).
#[must_use]
pub fn verify_any(
    probe: &dyn IdentityProbe,
    pid: u32,
    expects: &[ProcessIdentity],
) -> Verification {
    if expects.is_empty() {
        return Verification::Unverifiable;
    }
    let mut outcome = Verification::Unverifiable;
    for expect in expects {
        if verify_with(probe, pid, expect) == Verification::Verified {
            outcome = Verification::Verified;
            break;
        }
    }
    outcome
}

// --- AJ-E507: the verified TERM→grace→KILL ladder (pass2 §12) ------

/// What the ladder targets: one verified process, or its whole
/// process group (the deterministic-kill scope created at spawn,
/// AJ-E502).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LadderScope {
    /// One verified PID.
    PidOnly(u32),
    /// The verified PGID (every member enumerated and reported).
    Pgid(u32),
}

/// One member's ladder outcome (Req.6's vocabulary — `AlreadyGone` is
/// SUCCESS, never an error).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessOutcome {
    /// Exited after SIGTERM.
    ExitedOnTerm,
    /// Survived TERM; needed SIGKILL.
    NeededKill,
    /// Gone before/at TERM — success.
    AlreadyGone,
    /// Alive even after SIGKILL (unreapable) — the honest failure.
    Refused,
    /// TERM delivered, grace cancelled before the outcome was known
    /// (the caller aborted escalation; the audit row stays pending —
    /// the same "TERM'd, outcome unknown" language E509 renders).
    TermSent,
}

impl ProcessOutcome {
    /// The stable API/UI label (one language with the audit history).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExitedOnTerm => "exited on TERM",
            Self::NeededKill => "needed KILL",
            Self::AlreadyGone => "already gone",
            Self::Refused => "refused",
            Self::TermSent => "TERM'd — outcome unknown",
        }
    }
}

/// The ladder result: one outcome per member pid, plus whether the
/// caller cancelled mid-grace (in which case no KILL was sent and the
/// audit intent must stay pending — outcome unknown).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderOutcome {
    /// `(pid, outcome)` per enumerated member (scope pid-only ⇒ one).
    pub members: Vec<(u32, ProcessOutcome)>,
    /// True when `cancelled` fired during the grace window: escalation
    /// stopped, TERM survivors carry [`ProcessOutcome::TermSent`].
    pub cancelled: bool,
}

impl LadderOutcome {
    /// True when every member reached a success outcome
    /// (`exited on TERM`, `needed KILL`, or `already gone`).
    #[must_use]
    pub fn all_succeeded(&self) -> bool {
        self.members
            .iter()
            .all(|(_, outcome)| *outcome != ProcessOutcome::Refused)
    }
}

/// The platform's ladder shape (the Windows sheet's verbatim wording
/// source; `TerminateProcess`-only until the M7-5 bring-up).
#[must_use]
pub fn ladder_platform() -> &'static str {
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        "TERM → grace → KILL"
    } else {
        "TerminateProcess only"
    }
}

/// The verified ladder primitive (pass2 §12): TERM the scope, poll
/// through `grace` (checking `cancelled` each tick — the sheet's
/// grace window is cancellable), KILL only survivors. `members_of`
/// enumerates the scope (tests inject fixtures; production walks the
/// live table for PGID membership or returns the single pid).
///
/// True when the pid is GONE — or a ZOMBIE: the image already died
/// and only the parent's unreaped slot keeps `kill(pid, 0)`
/// succeeding. Without this, the ladder would escalate on a corpse
/// (SIGKILL a zombie, wait 2 s, report `refused` for a process that
/// died on TERM). One targeted refresh.
fn member_gone(pid: u32) -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
    if !signal_pid(pid, Signal::Probe) {
        return true;
    }
    let mut system = crate::new_system();
    let target = sysinfo::Pid::from_u32(pid);
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[target]),
        true,
        ProcessRefreshKind::nothing(),
    );
    system
        .process(target)
        .is_none_or(|process| process.status() == sysinfo::ProcessStatus::Zombie)
}

/// This function SIGNALS — the caller MUST have verified identity
/// this tick AND written the audit intent (AJ-E503/E509 gates).
pub fn terminate_group<F, C>(grace: Duration, members_of: F, cancelled: C) -> LadderOutcome
where
    F: FnOnce() -> Vec<u32>,
    C: Fn() -> bool,
{
    let members = members_of();
    let mut outcomes: Vec<(u32, ProcessOutcome)> = Vec::new();
    if members.is_empty() {
        return LadderOutcome {
            members: Vec::new(),
            cancelled: false,
        };
    }
    // Step 1: TERM every member; gone-before-TERM = AlreadyGone
    // (zombies included — their image already exited).
    let mut survivors = Vec::new();
    for pid in &members {
        if member_gone(*pid) {
            outcomes.push((*pid, ProcessOutcome::AlreadyGone));
        } else if signal_pid(*pid, Signal::Term) {
            survivors.push(*pid);
        } else {
            outcomes.push((*pid, ProcessOutcome::AlreadyGone));
        }
    }
    // Step 2: the visible grace window — cancellable at every tick.
    let mut aborted = false;
    let deadline = std::time::Instant::now() + grace;
    while !survivors.is_empty() && std::time::Instant::now() < deadline {
        if cancelled() {
            aborted = true;
            break;
        }
        let mut exited: Vec<u32> = Vec::new();
        survivors.retain(|pid| {
            let gone = member_gone(*pid);
            if gone {
                exited.push(*pid);
            }
            !gone
        });
        for pid in exited {
            outcomes.push((pid, ProcessOutcome::ExitedOnTerm));
        }
        if !survivors.is_empty() {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    if aborted {
        // The user stopped the ladder mid-grace: NO escalation, and
        // the audit intent stays pending ("TERM'd, outcome unknown").
        for pid in survivors.drain(..) {
            outcomes.push((pid, ProcessOutcome::TermSent));
        }
        outcomes.sort_unstable_by_key(|(pid, _)| *pid);
        return LadderOutcome {
            members: outcomes,
            cancelled: true,
        };
    }
    // Step 3: grace expired — the survivors ignored SIGTERM. KILL
    // them ("Ignored SIGTERM" in the sheet), then wait honestly.
    let mut refused = Vec::new();
    for pid in survivors.drain(..) {
        signal_pid(pid, Signal::Kill);
        refused.push(pid);
    }
    let kill_deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < kill_deadline && !refused.is_empty() {
        let mut died: Vec<u32> = Vec::new();
        refused.retain(|pid| {
            let gone = member_gone(*pid);
            if gone {
                died.push(*pid);
            }
            !gone
        });
        for pid in died {
            outcomes.push((pid, ProcessOutcome::NeededKill));
        }
        if refused.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    for pid in refused {
        outcomes.push((pid, ProcessOutcome::Refused));
    }
    outcomes.sort_unstable_by_key(|(pid, _)| *pid);
    LadderOutcome {
        members: outcomes,
        cancelled: false,
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Signal {
    Term,
    Kill,
    /// liveness probe (signal 0)
    Probe,
}

/// Sends one signal to one pid — the ONE platform seam (unix signals
/// vs the TerminateProcess-only ladder, M7-5).
#[cfg(unix)]
pub(crate) fn signal_pid(pid: u32, signal: Signal) -> bool {
    use nix::sys::signal::{self, Signal as NixSignal};
    use nix::unistd::Pid;
    let Ok(nix_pid) = i32::try_from(pid).map(Pid::from_raw) else {
        return false;
    };
    let result = match signal {
        Signal::Term => signal::kill(nix_pid, Some(NixSignal::SIGTERM)).map(|()| true),
        Signal::Kill => signal::kill(nix_pid, Some(NixSignal::SIGKILL)).map(|()| true),
        Signal::Probe => signal::kill(nix_pid, None).map(|()| true),
    };
    result.unwrap_or(false)
}

#[cfg(not(unix))]
pub(crate) fn signal_pid(_pid: u32, _signal: Signal) -> bool {
    // Non-unix: the ladder is TerminateProcess-only (M7-5); no
    // signal surface exists to fake here — the sheet says so.
    false
}

// --- AJ-E508: the tier evaluator (pass2 §10 / ADR-0010 amendment) --

/// The facts one candidate row contributes to the tier decision
/// (pure data: the sweep assembles, the evaluator decides, tests
/// forge — role stays a GUESS everywhere it appears, pass2 C4).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // the tier gates ARE booleans by law (ADR-0010)
pub struct TierCandidate {
    /// The candidate pid.
    pub pid: u32,
    /// pass2 §5 role guess (`DevServer|Watcher|McpServer|…`).
    pub role_guess: String,
    /// True for root rows — roots are NEVER auto-eligible.
    pub is_root: bool,
    /// Ledger state (`orphaned` is the only eligible one).
    pub state: String,
    /// True when the helper left the root's process group.
    pub escaped_group: bool,
    /// `Some(true)` = root exited code 0 (Tier 1's premise);
    /// `Some(false)` = crash exit (Tier 2's); `None` = unknown
    /// (fail closed — observe only).
    pub root_exit_clean: Option<bool>,
    /// False for another machine's row (D-13/E511).
    pub machine_local: bool,
    /// The Account's Tier-1 opt-in (clean exits).
    pub tier1_opt_in: bool,
    /// The Account's Tier-2 opt-in (crash exits; SEPARATE consent).
    pub tier2_opt_in: bool,
}

/// The evaluator's decision: signal through the AJ-E507 ladder at a
/// tier, or stay Tier 0 (list-only) with the honest reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TierVerdict {
    /// Eligible for the automatic ladder (tier 1 or 2). Timing
    /// (grace, 5 min, the Undo window) is the sweep's job.
    Signal(u8),
    /// Listed, never signalled. The reason is surfaced in tests and
    /// in the demote audit note — never silently dropped.
    ObserveOnly(String),
}

/// The tier decision, one pure function (ADR-0010's E508 amendment
/// verbatim): only `DevServer|Watcher` helpers of cleanly-exited
/// roots (Tier 1) or crash-exited roots with Tier 2's separate
/// opt-in are EVER auto-eligible; roots, `McpServer`,
/// `TerminalShell`, database-role guesses, re-adopted survivors,
/// unverifiable rows, escaped-group helpers, other machines' rows,
/// and unknown exit kinds are hard-excluded at every tier. Widening
/// the eligible role set requires a new decision-log entry.
#[must_use]
pub fn evaluate_tier(candidate: &TierCandidate) -> TierVerdict {
    if candidate.is_root {
        return TierVerdict::ObserveOnly("roots are never auto-cleaned".into());
    }
    if candidate.state != "orphaned" {
        return TierVerdict::ObserveOnly(format!(
            "state {} is not auto-eligible (listed only)",
            candidate.state
        ));
    }
    if !matches!(candidate.role_guess.as_str(), "DevServer" | "Watcher") {
        return TierVerdict::ObserveOnly(format!(
            "role guess {} is not auto-eligible (only DevServer|Watcher)",
            candidate.role_guess
        ));
    }
    if candidate.escaped_group {
        return TierVerdict::ObserveOnly("escaped its process group — containment void".into());
    }
    if !candidate.machine_local {
        return TierVerdict::ObserveOnly("another machine's process (D-13)".into());
    }
    match candidate.root_exit_clean {
        Some(true) => {
            if candidate.tier1_opt_in {
                TierVerdict::Signal(1)
            } else {
                TierVerdict::ObserveOnly("Account has not opted in to tidy (Tier 1)".into())
            }
        }
        Some(false) => {
            if candidate.tier2_opt_in {
                TierVerdict::Signal(2)
            } else {
                TierVerdict::ObserveOnly(
                    "Account has not opted in to tidy after crash (Tier 2)".into(),
                )
            }
        }
        None => {
            TierVerdict::ObserveOnly("root exit kind unknown — fail closed, listed only".into())
        }
    }
}

/// Live command line + cwd for one pid — the verify sheet's
/// re-read-NOW facts (pass2 §9.2 step 1). One targeted refresh
/// (never a full scan); `None` = gone or unreadable, and the caller
/// shows the recorded values with that caveat instead of guessing.
#[must_use]
pub fn live_details(pid: u32) -> Option<(String, String)> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
    let mut system = crate::new_system();
    let target = sysinfo::Pid::from_u32(pid);
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[target]),
        true,
        ProcessRefreshKind::nothing()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always),
    );
    let process = system.process(target)?;
    let cmdline = process
        .cmd()
        .iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    let cwd = process
        .cwd()
        .map_or_else(String::new, |cwd| cwd.to_string_lossy().to_string());
    Some((cmdline, cwd))
}

/// Production PGID membership: every pid whose live PGID matches,
/// from one full-table scan (the kill path may scan; the tick's
/// construction gate does not apply to it).
#[must_use]
#[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
pub fn pgid_members(pgid: u32) -> Vec<u32> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
    let mut system = crate::new_system();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let mut members = Vec::new();
    for sys_pid in system.processes().keys() {
        let pid = sys_pid.as_u32();
        if crate::group::pgid_of(pid) == Ok(pgid) {
            members.push(pid);
        }
    }
    members
}

#[cfg(test)]
mod verify_tests {
    use super::{
        IdentityProbe, ProbedProcess, ProcessIdentity, Verification, verify_any, verify_with,
    };
    use std::path::PathBuf;

    /// Forges probe results; `probed = None` simulates a probe error.
    struct FakeProbe {
        probed: Option<ProbedProcess>,
        boot: String,
    }

    impl IdentityProbe for FakeProbe {
        fn probe(&self, _pid: u32) -> Option<ProbedProcess> {
            self.probed.clone()
        }

        fn boot_id(&self) -> String {
            self.boot.clone()
        }
    }

    fn facts() -> ProbedProcess {
        ProbedProcess {
            start_time: 1_792_000_100,
            exe: PathBuf::from("/bin/sleep"),
        }
    }

    fn recorded() -> ProcessIdentity {
        ProcessIdentity {
            pid: 4242,
            pgid: Some(4242),
            start_time: 1_792_000_100,
            exe: PathBuf::from("/bin/sleep"),
            boot_id: "1791999000".to_string(),
        }
    }

    fn probe_with(probed: Option<ProbedProcess>, boot: &str) -> FakeProbe {
        FakeProbe {
            probed,
            boot: boot.to_string(),
        }
    }

    #[test]
    fn all_three_facts_agree_verifies() {
        let probe = probe_with(Some(facts()), "1791999000");
        assert_eq!(
            verify_with(&probe, 4242, &recorded()),
            Verification::Verified
        );
    }

    #[test]
    fn start_time_mismatch_is_unverifiable() {
        let probe = probe_with(Some(facts()), "1791999000");
        let mut expect = recorded();
        expect.start_time += 1;
        assert_eq!(
            verify_with(&probe, 4242, &expect),
            Verification::Unverifiable
        );
    }

    #[test]
    fn exe_mismatch_is_unverifiable() {
        let probe = probe_with(Some(facts()), "1791999000");
        let mut expect = recorded();
        expect.exe = PathBuf::from("/usr/local/bin/stranger");
        assert_eq!(
            verify_with(&probe, 4242, &expect),
            Verification::Unverifiable
        );
    }

    #[test]
    fn boot_id_mismatch_is_unverifiable() {
        // Live facts agree on everything except the boot.
        let probe = probe_with(Some(facts()), "1791999999");
        assert_eq!(
            verify_with(&probe, 4242, &recorded()),
            Verification::Unverifiable
        );
    }

    #[test]
    fn probe_error_is_unverifiable_never_verified() {
        let probe = probe_with(None, "1791999000");
        assert_eq!(
            verify_with(&probe, 4242, &recorded()),
            Verification::Unverifiable
        );
    }

    #[test]
    fn empty_or_partial_identity_sets_do_not_verify() {
        let probe = probe_with(Some(facts()), "1791999000");
        assert_eq!(verify_any(&probe, 4242, &[]), Verification::Unverifiable);
        // One matching record among mismatched ones still verifies —
        // each record must FULLY agree on its own.
        let mut stranger = recorded();
        stranger.start_time += 9;
        assert_eq!(
            verify_any(&probe, 4242, &[stranger, recorded()]),
            Verification::Verified
        );
    }
}

#[cfg(test)]
mod tier_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{TierCandidate, TierVerdict, evaluate_tier};

    fn candidate() -> TierCandidate {
        TierCandidate {
            pid: 8123,
            role_guess: "DevServer".into(),
            is_root: false,
            state: "orphaned".into(),
            escaped_group: false,
            root_exit_clean: Some(true),
            machine_local: true,
            tier1_opt_in: true,
            tier2_opt_in: false,
        }
    }

    #[test]
    fn clean_exit_dev_server_with_tier1_is_eligible() {
        assert_eq!(evaluate_tier(&candidate()), TierVerdict::Signal(1));
    }

    #[test]
    fn crash_exit_needs_the_separate_tier2_consent() {
        let mut crash = candidate();
        crash.root_exit_clean = Some(false);
        assert_eq!(
            evaluate_tier(&crash),
            TierVerdict::ObserveOnly(
                "Account has not opted in to tidy after crash (Tier 2)".into()
            )
        );
        crash.tier2_opt_in = true;
        assert_eq!(evaluate_tier(&crash), TierVerdict::Signal(2));
    }

    #[test]
    fn hard_exclusions_apply_at_every_tier() {
        for (name, mutate) in [
            (
                "root",
                Box::new(|c: &mut TierCandidate| c.is_root = true)
                    as Box<dyn Fn(&mut TierCandidate)>,
            ),
            (
                "mcp role",
                Box::new(|c: &mut TierCandidate| c.role_guess = "McpServer".into()),
            ),
            (
                "shell role",
                Box::new(|c: &mut TierCandidate| c.role_guess = "TerminalShell".into()),
            ),
            (
                "database role",
                Box::new(|c: &mut TierCandidate| c.role_guess = "Database".into()),
            ),
            (
                "unknown role",
                Box::new(|c: &mut TierCandidate| c.role_guess = "Unknown".into()),
            ),
            (
                "re-adopted survivor",
                Box::new(|c: &mut TierCandidate| c.state = "adopted".into()),
            ),
            (
                "unverifiable",
                Box::new(|c: &mut TierCandidate| c.state = "unverifiable".into()),
            ),
            (
                "escaped group",
                Box::new(|c: &mut TierCandidate| c.escaped_group = true),
            ),
            (
                "remote machine",
                Box::new(|c: &mut TierCandidate| c.machine_local = false),
            ),
            (
                "unknown exit kind",
                Box::new(|c: &mut TierCandidate| c.root_exit_clean = None),
            ),
            (
                "tier1 off",
                Box::new(|c: &mut TierCandidate| c.tier1_opt_in = false),
            ),
        ] {
            let mut excluded = candidate();
            mutate(&mut excluded);
            // Exclusions hold even when BOTH tiers are on.
            excluded.tier2_opt_in = true;
            let verdict = evaluate_tier(&excluded);
            assert!(
                matches!(verdict, TierVerdict::ObserveOnly(_)),
                "{name}: expected observe-only, got {verdict:?}"
            );
        }
    }

    #[test]
    fn watcher_is_the_only_other_eligible_role() {
        let mut watcher = candidate();
        watcher.role_guess = "Watcher".into();
        assert_eq!(evaluate_tier(&watcher), TierVerdict::Signal(1));
    }
}
