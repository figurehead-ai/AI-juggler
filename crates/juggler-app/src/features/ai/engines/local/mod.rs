//! ADR-0033 §4.2, ADR-0035 §11 (AJ-E3802, D-131): the local engine for
//! downloaded GGUF files, through AI Juggler's helper `juggler-local-ai`
//! (llama.cpp by llama-cpp-2 0.1.146, `crates/juggler-local-ai`, its own
//! workspace). **Nothing runs a model in AI Juggler's process:** the
//! helper does, sandboxed with no network, and its memory goes when it
//! exits.
//!
//! **What it serves.** One card per installed `gguf_local` model (the
//! store's, AJ-E3803). A card is ready when this is a Mac with Apple
//! silicon, the Mac has the model's `min_memory_gb`, the helper is in the
//! app (found and signature-checked by the one resolver,
//! [`juggler_platform::code_trust::own_helper`]) and said hello on this
//! protocol, and the engine isn't unhealthy (three failures within an
//! hour). The router's Automatic choice then picks among the cards by the
//! section's verdicts and the recommendation (AJ-E3811), as for any engine.
//!
//! **One job's path** ([`AiEngine::respond`]):
//! 1. the installed model's file, `<sha256>.gguf` in the models' folder,
//!    hashed in full before its first load and stamped ([`verify`]);
//! 2. the memory check against `min_memory_gb`, never under critical
//!    memory pressure; then the helper, started lazily
//!    ([`Local::command`]): under its own deny-by-default sandbox profile
//!    (`sandbox-exec`), a memory cap ([`cap_mib`]) and utility quality of
//!    service (`taskpolicy`), in the models' folder with the folder as its
//!    only argument, the allowed environment only, and llama.cpp's crash
//!    backtrace off;
//! 3. `load` by the file's name only, with the device and inode that
//!    were hashed (the helper opens it without following a link and loads
//!    only that file), when another model (or none) is loaded, within
//!    [`LOAD_WAIT`];
//! 4. `generate` with the job's frame, its shape as a JSON schema
//!    ([`shape`]), the model's tested sampling within the protocol's
//!    compiled bounds, and the job's deadline as the timeout. A stop or
//!    the deadline sends `cancel` and waits [`CANCEL_GRACE`] for the
//!    helper to say so; one that doesn't is stopped (its own child).
//!    Critical memory pressure stops it the same way and lets the helper
//!    go.
//!
//! **Its life.** The helper exits by itself after
//! [`juggler_local_ai_protocol::IDLE_EXIT_SECS`] idle and when stdin
//! closes; a probe that finds it idle longer drops it ([`Local::probe`]),
//! and [`AiEngine::release`] drops it at once. A crash, a broken answer
//! or a timeout that didn't stop drops it; the next job starts it again.
//!
//! Windows and Linux compile it and serve nothing yet (D-52: the same
//! contract with CPU or Vulkan, later).

pub mod shape;
pub mod verify;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use juggler_local_ai_protocol::{
    self as protocol, Answer, Fault, Finish as WireFinish, Guide, MODELS_DIR_ARG, Sampling, bounds,
    model_file_name,
};

use super::helper::{Health, Helper};
use crate::features::ai::engine::{
    AiEngine, Card, Draft, EngineError, Finish, Readiness, Reason, Request, Stop, Tier,
};
use crate::features::ai::models::store::Installed;
use crate::features::ai::models::{Engine as ModelEngine, ModelEntry};

/// How long a starting helper may take to say hello.
const HELLO_WAIT: Duration = Duration::from_secs(10);

/// How long loading a model may take (a few seconds a gigabyte from a
/// cold disk, then the GPU's copy).
pub const LOAD_WAIT: Duration = Duration::from_secs(90);

/// How long a stopped generation's helper has to say it stopped before it
/// is stopped itself.
pub const CANCEL_GRACE: Duration = Duration::from_secs(2);

/// The context a model runs with when its entry names none (ADR-0033
/// §4.2: 4 K).
const DEFAULT_CONTEXT: u32 = 4_096;

/// The helper's quality of service and memory cap: the system's own tool
/// clamps it to utility and gives it a memory limit (review L3).
const TASKPOLICY: &str = juggler_adapter::system_tools::TASKPOLICY;

/// The system's sandbox tool: it applies the helper's profile
/// (`juggler_local_ai_protocol::SANDBOX_PROFILE`) and then runs it. Without
/// it the helper never runs (fails closed).
const SANDBOX_EXEC: &str = juggler_adapter::system_tools::SANDBOX_EXEC;

/// The cap a helper with no model yet starts with (a probe's hello).
pub const BARE_CAP_MIB: u64 = 1_024;

/// This computer, as far as the engine goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Host {
    /// It is macOS (the only system with a helper yet).
    pub macos: bool,
    /// Its chip is Apple silicon (the helper is arm64 only).
    pub apple_silicon: bool,
    /// Its memory in GB (10⁹ bytes, as the recommendation counts).
    pub memory_gb: u64,
}

impl Host {
    /// This computer, read once.
    #[must_use]
    pub fn this_one() -> Self {
        static HOST: std::sync::OnceLock<Host> = std::sync::OnceLock::new();
        *HOST.get_or_init(|| {
            let machine = juggler_platform::ai::machine();
            Self {
                macos: cfg!(target_os = "macos"),
                apple_silicon: matches!(machine.chip, juggler_platform::ai::Chip::AppleSilicon(_)),
                memory_gb: machine
                    .memory_bytes
                    .map_or(0, |bytes| bytes / 1_000_000_000),
            }
        })
    }
}

/// Where the engine finds its helper and its models: the app's own, or a
/// test's.
#[derive(Debug, Default)]
pub struct Setup {
    /// The helper; `None` is the app's own, through the resolver.
    pub program: Option<PathBuf>,
    /// The models' folder and what is installed in it; `None` is the
    /// app's store.
    pub models: Option<(PathBuf, Vec<Installed>)>,
    /// This computer; `None` reads it.
    pub host: Option<Host>,
    /// Whether memory pressure is critical now; `None` reads the kernel's
    /// level.
    pub pressure: Option<fn() -> bool>,
    /// Run the helper without its sandbox (tests with a fake helper
    /// script only: the sandbox lets the helper run nothing but itself).
    pub unsandboxed: bool,
    /// The sandbox tool; `None` is the system's (a test names a stand-in
    /// that refuses the profile, or ignores it).
    pub sandbox_tool: Option<PathBuf>,
}

/// The kernel's memory pressure is critical now.
fn critical_pressure() -> bool {
    use juggler_supervisor::machine::{HostReadings, OsReadings as _, PressureLevel};
    HostReadings.pressure() == Some(PressureLevel::Critical)
}

/// The memory cap for a helper running a model of `size_bytes` (review
/// L3): twice the file and 2 GiB for its context and the GPU's buffers,
/// never more than three quarters of this computer's memory, never less
/// than [`BARE_CAP_MIB`]. Past it the system ends the helper (the job
/// then falls back, as for any crash).
#[must_use]
pub fn cap_mib(size_bytes: u64, host: Host) -> u64 {
    let want = size_bytes
        .div_ceil(1 << 20)
        .saturating_mul(2)
        .saturating_add(2_048);
    let most = host.memory_gb.saturating_mul(1_000_000_000) / (1 << 20) * 3 / 4;
    want.min(most).max(BARE_CAP_MIB)
}

/// Metal's shader cache for the helper: the user's cache folder (the
/// sibling `C` of the per-user temporary folder `T` under
/// `/private/var/folders`) and the helper's identifier; a path that holds
/// nothing when it can't tell (the helper then compiles its kernels at
/// every start).
#[must_use]
pub fn metal_cache() -> PathBuf {
    let tmp = std::fs::canonicalize(std::env::temp_dir()).ok();
    tmp.filter(|t| t.file_name().is_some_and(|n| n == "T") && t.starts_with("/private/var/folders"))
        .and_then(|t| t.parent().map(|p| p.join("C").join(protocol::IDENTIFIER)))
        .unwrap_or_else(|| PathBuf::from("/nonexistent/ai-juggler-local-ai-cache"))
}

/// What the last probe saw of the helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seen {
    /// Not asked yet (no model installed, or no query).
    NotAsked,
    /// Not in the app, or not safe to run (owner, signature).
    Missing,
    /// It said hello on this protocol.
    Ready,
    /// It didn't say hello properly.
    Failed,
    /// The system's sandbox tool isn't there: the helper is never run
    /// without its sandbox (the re-review's N3, said loudly: logged, and
    /// its jobs hidden as with a missing helper).
    SandboxToolMissing,
    /// The sandbox tool refused the helper's profile (an operation this
    /// macOS doesn't know).
    SandboxRejected,
    /// The helper started but its own check found it unsandboxed: never
    /// used (fails closed, never open).
    NotSandboxed,
}

impl Seen {
    /// The sandbox failed, in one of the three ways that are never quiet.
    #[must_use]
    pub const fn sandbox_fault(self) -> bool {
        matches!(
            self,
            Self::SandboxToolMissing | Self::SandboxRejected | Self::NotSandboxed
        )
    }
}

/// The helper while it runs.
#[derive(Debug)]
struct Running {
    helper: Helper,
    /// The loaded model's hash.
    loaded: Option<String>,
    /// When it last answered.
    used: Instant,
    /// The memory cap it was started with, in MiB.
    cap_mib: u64,
}

/// The local engine.
#[derive(Debug)]
pub struct Local {
    setup: Setup,
    running: Mutex<Option<Running>>,
    verified: verify::Verified,
    health: Health<Seen>,
    next_id: AtomicU64,
}

impl Default for Local {
    fn default() -> Self {
        Self::with(Setup::default())
    }
}

/// The readiness of a model with `min_memory_gb` on `host`, the helper
/// `seen`, and the engine `unhealthy` (pure).
#[must_use]
pub const fn readiness(host: Host, min_memory_gb: u64, seen: Seen, unhealthy: bool) -> Readiness {
    if !host.macos {
        return Readiness::NotHere(Reason::NotThisSystem);
    }
    if !host.apple_silicon || host.memory_gb < min_memory_gb {
        return Readiness::NotHere(Reason::NotThisComputer);
    }
    match seen {
        // A sandbox that can't be applied hides the jobs as a missing helper
        // does; the exact reason is the engine's (`Local::seen`) and the log's.
        Seen::Missing | Seen::SandboxToolMissing | Seen::SandboxRejected | Seen::NotSandboxed => {
            Readiness::NotHere(Reason::HelperMissing)
        }
        Seen::NotAsked | Seen::Failed => Readiness::NotHere(Reason::Unknown),
        Seen::Ready if unhealthy => Readiness::NotHere(Reason::Unhealthy),
        Seen::Ready => Readiness::Ready,
    }
}

/// The context a model runs with: its entry's, within the protocol's
/// bounds, or the default.
fn context_of(entry: &ModelEntry) -> u32 {
    entry
        .sampling
        .context_tokens
        .filter(|n| bounds::CONTEXT_TOKENS.holds(*n))
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(DEFAULT_CONTEXT)
}

/// The model's tested sampling (ADR-0035 §11.1), each field kept only
/// within the protocol's compiled bounds (the section reads them with the
/// same bounds, so none is dropped in practice).
#[must_use]
pub fn sampling_of(entry: &ModelEntry) -> Sampling {
    let keep = |value: Option<u64>, range: bounds::Range| {
        value
            .filter(|n| range.holds(*n))
            .and_then(|n| u32::try_from(n).ok())
    };
    let s = &entry.sampling;
    Sampling {
        temperature_milli: keep(s.temperature_milli, bounds::TEMPERATURE_MILLI),
        top_p_milli: keep(s.top_p_milli, bounds::TOP_P_MILLI),
        top_k: keep(s.top_k, bounds::TOP_K),
        min_p_milli: keep(s.min_p_milli, bounds::MIN_P_MILLI),
        presence_milli: keep(s.presence_milli, bounds::PRESENCE_MILLI),
    }
}

/// `sandbox-exec`'s arguments for the helper's profile and its three
/// parameters.
fn sandbox_args(program: &Path, dir: &Path) -> Vec<std::ffi::OsString> {
    let param = |name: &str, path: &Path| {
        let mut arg = std::ffi::OsString::from(format!("{name}="));
        arg.push(path);
        arg
    };
    vec![
        "-p".into(),
        protocol::SANDBOX_PROFILE.into(),
        "-D".into(),
        param("HELPER", program),
        "-D".into(),
        param("MODELS", dir),
        "-D".into(),
        param("CACHE", &metal_cache()),
    ]
}

/// The exit status `sandbox-exec` gives a profile it can't read.
const PROFILE_REFUSED: i32 = 65;

/// Whether `tool` refuses the helper's profile: after a start that gave no
/// hello, the profile alone is tried on `/usr/bin/true` (which the profile
/// itself doesn't let run, so nothing runs either way).
fn profile_rejected(tool: &Path, program: &Path, dir: &Path) -> bool {
    juggler_platform::process::helper(tool)
        .args(sandbox_args(program, dir))
        .arg(juggler_adapter::system_tools::TRUE)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.code() == Some(PROFILE_REFUSED))
}

impl Local {
    /// The engine over `setup`, nothing started.
    #[must_use]
    pub fn with(setup: Setup) -> Self {
        Self {
            setup,
            running: Mutex::new(None),
            verified: verify::Verified::default(),
            health: Health::new(Seen::NotAsked),
            next_id: AtomicU64::new(0),
        }
    }

    /// What the last probe saw.
    #[must_use]
    pub fn seen(&self) -> Seen {
        self.health.seen(Seen::Failed)
    }

    fn host(&self) -> Host {
        self.setup.host.unwrap_or_else(Host::this_one)
    }

    /// Memory pressure is critical now.
    fn critical(&self) -> bool {
        self.setup
            .pressure
            .map_or_else(critical_pressure, |read| read())
    }

    /// The models' folder and the installed models.
    fn models(&self) -> Option<(PathBuf, Vec<Installed>)> {
        if let Some((dir, installed)) = &self.setup.models {
            return Some((dir.clone(), installed.clone()));
        }
        let store = crate::features::ai::models::store::shared_if_made()?;
        Some((store.dir().to_path_buf(), store.installed()))
    }

    fn installed(&self) -> Vec<Installed> {
        self.models()
            .map(|(_, installed)| installed)
            .unwrap_or_default()
            .into_iter()
            .filter(|m| m.entry.engine == ModelEngine::GgufLocal)
            .collect()
    }

    /// The helper to run: a test's, or the app's own through the one
    /// resolver (in the app's bundle, ours and writable by no one else,
    /// signed as [`protocol::IDENTIFIER`] by the app's own team).
    fn program(&self) -> Option<PathBuf> {
        if let Some(program) = &self.setup.program {
            return program.is_file().then(|| program.clone());
        }
        juggler_platform::code_trust::own_helper(protocol::PROGRAM, protocol::IDENTIFIER)
    }

    /// The helper's command (review M1, L3; re-review N1): `sandbox_tool`
    /// (`sandbox-exec`) applies the helper's profile and runs `taskpolicy
    /// -c utility -m <cap>`, which runs the helper with the folder as its
    /// only argument, in its folder, with the allowed environment only and
    /// llama.cpp's crash backtrace off. The cap is set last, after the
    /// sandbox's exec, so no exec drops it. Without `sandbox_tool` (a fake
    /// helper's tests) it starts at `taskpolicy`. `None` off macOS or
    /// without `taskpolicy`.
    #[must_use]
    pub fn command(
        program: &Path,
        dir: &Path,
        cap_mib: u64,
        sandbox_tool: Option<&Path>,
    ) -> Option<Command> {
        let taskpolicy = Path::new(TASKPOLICY);
        if !cfg!(target_os = "macos") || !taskpolicy.is_file() {
            return None;
        }
        let mut command = match sandbox_tool {
            Some(tool) => {
                let mut command = juggler_platform::process::helper(tool);
                command.args(sandbox_args(program, dir)).arg(taskpolicy);
                command
            }
            None => juggler_platform::process::helper(taskpolicy),
        };
        command
            .args(["-c", "utility", "-m"])
            .arg(cap_mib.to_string())
            .arg(program)
            .arg(MODELS_DIR_ARG)
            .arg(dir)
            .current_dir(dir)
            .env("GGML_NO_BACKTRACE", "1");
        Some(command)
    }

    /// The sandbox tool to use: a test's stand-in, or the system's.
    fn sandbox_tool(&self) -> Option<PathBuf> {
        if self.setup.unsandboxed {
            return None;
        }
        Some(
            self.setup
                .sandbox_tool
                .clone()
                .unwrap_or_else(|| PathBuf::from(SANDBOX_EXEC)),
        )
    }

    /// Records a sandbox fault, and logs it the first time it is seen.
    fn sandbox_fault(&self, seen: Seen) {
        if self.seen() != seen {
            let why = match seen {
                Seen::SandboxToolMissing => "the system's sandbox tool is missing",
                Seen::SandboxRejected => "the system's sandbox tool refused the helper's profile",
                _ => "the helper reported it isn't sandboxed",
            };
            tracing::error!(
                target: "ai",
                "local models are off: {why}; the helper is never run without its sandbox"
            );
        }
        self.health.see(seen);
    }

    /// Starts the helper and checks its hello: this protocol, and (when it
    /// was started sandboxed) its own check that the sandbox holds. A
    /// missing sandbox tool, a refused profile or an unsandboxed helper is
    /// a sandbox fault (logged, the jobs hidden), never a quiet failure.
    fn start(&self, program: &Path, dir: &Path, cap_mib: u64) -> Result<Helper, EngineError> {
        let missing = EngineError::NotHere(Reason::HelperMissing);
        let program = std::fs::canonicalize(program).map_err(|_| missing)?;
        let dir = std::fs::canonicalize(dir).map_err(|_| EngineError::NotHere(Reason::Unknown))?;
        let tool = self.sandbox_tool();
        if tool.as_ref().is_some_and(|tool| !tool.is_file()) {
            self.sandbox_fault(Seen::SandboxToolMissing);
            return Err(missing);
        }
        let command = Self::command(&program, &dir, cap_mib, tool.as_deref()).ok_or(missing)?;
        let helper = match Helper::launch(command, HELLO_WAIT) {
            Ok(helper) => helper,
            Err(error) => {
                if tool
                    .as_deref()
                    .is_some_and(|tool| profile_rejected(tool, &program, &dir))
                {
                    self.sandbox_fault(Seen::SandboxRejected);
                    return Err(missing);
                }
                self.health.see(Seen::Failed);
                return Err(error);
            }
        };
        let hello = serde_json::to_vec(&helper.hello).map_err(|_| EngineError::Crashed)?;
        let Ok(Answer::Hello {
            protocol: protocol::PROTOCOL,
            sandboxed,
            ..
        }) = protocol::parse_answer(&hello)
        else {
            self.health.see(Seen::Failed);
            return Err(EngineError::Crashed);
        };
        if tool.is_some() && !sandboxed {
            self.sandbox_fault(Seen::NotSandboxed);
            return Err(missing);
        }
        self.health.see(Seen::Ready);
        Ok(helper)
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// The model `model` is loaded in the running helper (for the card).
    /// Never waits: a job holding the helper means it is running.
    fn loaded(&self, sha256: &str) -> bool {
        match self.running.try_lock() {
            Ok(mut running) => running
                .as_mut()
                .is_some_and(|r| r.helper.alive() && r.loaded.as_deref() == Some(sha256)),
            Err(std::sync::TryLockError::WouldBlock) => true,
            Err(std::sync::TryLockError::Poisoned(_)) => false,
        }
    }

    /// Loads `file` in `running`'s helper, within [`LOAD_WAIT`] or the job's
    /// stop.
    fn load(
        &self,
        running: &mut Running,
        sha256: &str,
        size_bytes: u64,
        id_on_disk: juggler_platform::transfer::FileId,
        context_tokens: u32,
        stop: &Stop,
    ) -> Result<(), EngineError> {
        let file = model_file_name(sha256).ok_or(EngineError::NotHere(Reason::Unknown))?;
        let id = self.next_id();
        let line = protocol::Request::Load {
            id,
            file,
            size_bytes,
            device: id_on_disk.device,
            inode: id_on_disk.inode,
            context_tokens,
        }
        .encode()
        .map_err(|_| EngineError::Unsupported)?;
        running.loaded = None;
        let answer = running
            .helper
            .exchange(&line, &stop.until(Instant::now() + LOAD_WAIT))?;
        match protocol::parse_answer(answer.as_bytes()) {
            Ok(Answer::Loaded { id: got, .. }) if got == id => {
                running.loaded = Some(sha256.to_owned());
                Ok(())
            }
            Ok(Answer::Error {
                id: Some(got),
                error,
            }) if got == id => Err(match error {
                Fault::NoFile | Fault::WrongFile | Fault::LoadFailed | Fault::NoTemplate => {
                    EngineError::NotHere(Reason::Unknown)
                }
                _ => EngineError::Crashed,
            }),
            _ => Err(EngineError::Crashed),
        }
    }

    /// One generation in `running`'s helper; a stop or the deadline sends
    /// `cancel` and waits [`CANCEL_GRACE`]. `Ok(Err(..))` keeps the
    /// helper; `Err(..)` means it must go.
    fn generate(
        &self,
        running: &mut Running,
        entry: &ModelEntry,
        request: &Request,
        stop: &Stop,
    ) -> Result<Result<Draft, EngineError>, EngineError> {
        let guide = match &request.shape {
            None => None,
            Some(shape) => Some(Guide::JsonSchema(
                shape::json_schema(shape).ok_or(EngineError::Unsupported)?,
            )),
        };
        let left = stop.deadline().saturating_duration_since(Instant::now());
        let timeout_ms = u64::try_from(left.as_millis())
            .unwrap_or(u64::MAX)
            .clamp(bounds::TIMEOUT_MS.min, bounds::TIMEOUT_MS.max);
        let max_tokens =
            u64::from(request.max_tokens).clamp(bounds::MAX_TOKENS.min, bounds::MAX_TOKENS.max);
        let id = self.next_id();
        let line = protocol::Request::Generate {
            id,
            instructions: request.instructions.clone(),
            prompt: request.prompt.clone(),
            guide,
            sampling: sampling_of(entry),
            max_tokens: u32::try_from(max_tokens).unwrap_or(1),
            timeout_ms,
        }
        .encode();
        let line = match line {
            Ok(line) => line,
            Err(Fault::TooLarge) => return Ok(Err(EngineError::TooLong)),
            Err(_) => return Ok(Err(EngineError::Unsupported)),
        };
        // Review L3: critical memory pressure stops the generation (read
        // at most every quarter second while it runs).
        let checked = std::cell::Cell::new(Instant::now());
        let keep = || {
            if checked.get().elapsed() < Duration::from_millis(250) {
                return true;
            }
            checked.set(Instant::now());
            !self.critical()
        };
        let answer = match running.helper.exchange_while(&line, stop, &keep) {
            Ok(answer) => answer,
            Err(stopped @ (EngineError::Stopped | EngineError::TimedOut)) => {
                return Self::cancel(running, id, stopped);
            }
            Err(error) => return Err(error),
        };
        match protocol::parse_answer(answer.as_bytes()) {
            Ok(Answer::Text {
                id: got,
                text,
                finish,
                ..
            }) if got == id => Ok(Ok(Draft {
                text,
                finish: match finish {
                    WireFinish::Stop => Finish::Stop,
                    WireFinish::Length => Finish::Length,
                },
            })),
            Ok(Answer::Error {
                id: Some(got),
                error,
            }) if got == id => match error {
                Fault::TooLong => Ok(Err(EngineError::TooLong)),
                Fault::Guide => Ok(Err(EngineError::Unsupported)),
                Fault::Stopped => Ok(Err(EngineError::Stopped)),
                Fault::TimedOut => Ok(Err(EngineError::TimedOut)),
                _ => Err(EngineError::Crashed),
            },
            _ => Err(EngineError::Crashed),
        }
    }

    /// After a stop: `cancel`, then the generation's own answer within
    /// [`CANCEL_GRACE`]. The helper stays only when it answered for `id`.
    fn cancel(
        running: &mut Running,
        id: u64,
        why: EngineError,
    ) -> Result<Result<Draft, EngineError>, EngineError> {
        let line = protocol::Request::Cancel { id }.encode().map_err(|_| why)?;
        running.helper.send(&line).map_err(|_| why)?;
        let answer = running
            .helper
            .answer_within(CANCEL_GRACE)
            .map_err(|_| why)?;
        match protocol::parse_answer(answer.as_bytes()) {
            Ok(Answer::Error { id: Some(got), .. } | Answer::Text { id: got, .. }) if got == id => {
                Ok(Err(why))
            }
            _ => Err(why),
        }
    }
}

impl AiEngine for Local {
    fn cards(&self, engine: usize, models: &[ModelEntry]) -> Vec<Card> {
        let installed = self.installed();
        let host = self.host();
        let seen = self.seen();
        let unhealthy = self.health.unhealthy();
        models
            .iter()
            .filter(|m| m.engine == ModelEngine::GgufLocal)
            .filter_map(|m| {
                let mine = installed.iter().find(|i| i.entry.id == m.id)?;
                Some(Card {
                    engine,
                    model: m.id.clone(),
                    context_tokens: context_of(&mine.entry),
                    tools: true,
                    tier: Tier::Basic,
                    uses_download: true,
                    running: self.loaded(&mine.sha256),
                    ready: readiness(
                        host,
                        m.min_memory_gb.max(mine.entry.min_memory_gb),
                        seen,
                        unhealthy,
                    ),
                })
            })
            .collect()
    }

    /// Asks again only when a model is installed: whether the helper is
    /// in the app and says hello (it exits at once after; nothing is
    /// loaded). A helper idle past its own exit time is let go.
    fn probe(&self) {
        // A job holding the helper isn't idle: never wait for it.
        if let Ok(mut running) = self.running.try_lock() {
            let idle = Duration::from_secs(protocol::IDLE_EXIT_SECS);
            if running
                .as_mut()
                .is_some_and(|r| !r.helper.alive() || r.used.elapsed() > idle)
            {
                *running = None;
            }
        }
        let host = self.host();
        if !host.macos || !host.apple_silicon || self.installed().is_empty() {
            return;
        }
        let Some(program) = self.program() else {
            self.health.see(Seen::Missing);
            return;
        };
        if self.seen() == Seen::Ready {
            return;
        }
        let Some((dir, _)) = self.models() else {
            return;
        };
        // `start` records what it found (Ready, Failed or a sandbox fault).
        if let Ok(helper) = self.start(&program, &dir, BARE_CAP_MIB) {
            self.health.healed();
            drop(helper);
        }
    }

    fn respond(&self, model: &str, request: &Request, stop: &Stop) -> Result<Draft, EngineError> {
        let (dir, installed) = self.models().ok_or(EngineError::Unsupported)?;
        let mine = installed
            .into_iter()
            .find(|m| m.entry.id == model && m.entry.engine == ModelEngine::GgufLocal)
            .ok_or(EngineError::Unsupported)?;
        if let Readiness::NotHere(why) | Readiness::Preparing(why) = readiness(
            self.host(),
            mine.entry.min_memory_gb,
            Seen::Ready,
            self.health.unhealthy(),
        ) {
            return Err(EngineError::NotHere(why));
        }
        let program = self
            .program()
            .ok_or(EngineError::NotHere(Reason::HelperMissing))?;
        let file = model_file_name(&mine.sha256)
            .map(|name| dir.join(name))
            .ok_or(EngineError::NotHere(Reason::Unknown))?;
        let id_on_disk = self
            .verified
            .check(&file, &mine.sha256, mine.size_bytes, stop)?;
        let mut slot = self.running.lock().map_err(|_| EngineError::Crashed)?;
        // Review L3: never start or load under critical memory pressure,
        // and let go of what is held.
        if self.critical() {
            *slot = None;
            return Err(EngineError::NotHere(Reason::Unknown));
        }
        let cap = cap_mib(mine.size_bytes, self.host());
        if !slot
            .as_mut()
            .is_some_and(|r| r.helper.alive() && r.cap_mib >= cap)
        {
            *slot = None;
            let helper = self
                .start(&program, &dir, cap)
                .inspect_err(|_| self.health.failed())?;
            *slot = Some(Running {
                helper,
                loaded: None,
                used: Instant::now(),
                cap_mib: cap,
            });
        }
        let Some(running) = slot.as_mut() else {
            return Err(EngineError::Crashed);
        };
        if running.loaded.as_deref() != Some(mine.sha256.as_str()) {
            match self.load(
                running,
                &mine.sha256,
                mine.size_bytes,
                id_on_disk,
                context_of(&mine.entry),
                stop,
            ) {
                Ok(()) => {}
                Err(EngineError::NotHere(why)) => {
                    running.used = Instant::now();
                    self.health.failed();
                    return Err(EngineError::NotHere(why));
                }
                Err(error) => {
                    // A load that didn't answer can't be cancelled: stop it.
                    *slot = None;
                    if error == EngineError::Crashed {
                        self.health.failed();
                    }
                    return Err(error);
                }
            }
        }
        let result = self.generate(running, &mine.entry, request, stop);
        running.used = Instant::now();
        if self.critical() {
            // Stopped by critical pressure: free the model's memory now.
            *slot = None;
        }
        match result {
            Ok(answer) => answer,
            Err(error) => {
                *slot = None;
                if error == EngineError::Crashed {
                    self.health.failed();
                }
                Err(error)
            }
        }
    }

    fn release(&self) {
        if let Ok(mut running) = self.running.lock() {
            *running = None;
        }
    }
}

#[cfg(test)]
mod tests;
