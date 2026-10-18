//! AJ-E512 fixture harness (SPEC §10, pass2 §14): scriptable fake
//! helpers whose every received signal lands in a per-fixture trap
//! log. Shared by the supervisor-level collateral suite (this crate)
//! and the app-level guardrails suite (`juggler-app/tests/guardrails.rs`
//! includes this module by `#[path]`).
//!
//! The seven fixture kinds and where each is PROTECTED (zero signals
//! asserted on the trap log + aliveness):
//!
//! | kind | shape | protected in |
//! |---|---|---|
//! | `vite` | port-holding DevServer (the INTENDED target) | — (it takes the TERM) |
//! | `term-ignorer` | logs TERM, survives until KILL | every path where it is not the target |
//! | `escapee` | `setsid` — own group, ppid chain intact | group-scoped TERM paths |
//! | `mcp` | `mcp-server` argv (McpServer role) | all automated tiers + pid scope |
//! | `shell` | `sh` child (TerminalShell role) | all automated tiers + pid scope |
//! | `unmanaged` | user server, NO ledger row | EVERY path (never ours to signal) |
//! | `stranger` | forged identity row (PID reuse) | EVERY path (Unverifiable, listed) |

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The fixture script, embedded so every including crate is
/// self-contained (no cross-crate runtime paths).
const HELPER_SRC: &str = include_str!("trap_helper.py");

/// Per-fixture file paths (trap log + readiness handshake).
pub struct Fixture {
    /// Fixture kind name (`vite`, `mcp`, …).
    pub kind: &'static str,
    /// Every received signal, one per line.
    pub trap_log: PathBuf,
    /// Touched once handlers are installed (+ port bound).
    pub ready: PathBuf,
}

impl Fixture {
    /// The signals this fixture received (trap log lines).
    pub fn signals(&self) -> Vec<String> {
        let mut text = String::new();
        if let Ok(mut file) = std::fs::File::open(&self.trap_log) {
            let _ = file.read_to_string(&mut text);
        }
        text.lines().map(str::to_string).collect()
    }

    /// True when the fixture received no signal at all.
    pub fn untouched(&self) -> bool {
        self.signals().is_empty()
    }
}

/// Writes the embedded helper script into the test dir; every spawn
/// references this copy.
pub fn write_helper(dir: &Path) -> PathBuf {
    let helper = dir.join("trap_helper.py");
    std::fs::write(&helper, HELPER_SRC).unwrap();
    helper
}

fn fixture(dir: &Path, kind: &'static str) -> Fixture {
    Fixture {
        kind,
        trap_log: dir.join(format!("{kind}.traps")),
        ready: dir.join(format!("{kind}.ready")),
    }
}

/// Declares the fixture set used by the suites. `port` is bound by the
/// `vite` fixture (and the unmanaged user server).
pub struct FixtureSet {
    /// The helper script path (from [`write_helper`]).
    pub helper: PathBuf,
    fixtures: Vec<Fixture>,
}

/// The fixture kinds spawned INSIDE a supervised root (background
/// lines for the root script, joined by the root's `wait`).
pub const IN_TREE_KINDS: &[&str] = &["vite", "term-ignorer", "escapee", "mcp", "shell"];

impl FixtureSet {
    /// Builds the fixture set for `dir` (one instance per kind). The
    /// vite fixture binds an OS-assigned port; read it with
    /// [`FixtureSet::vite_port`] once ready.
    pub fn new(dir: &Path) -> Self {
        let helper = write_helper(dir);
        let fixtures = IN_TREE_KINDS
            .iter()
            .map(|kind| fixture(dir, kind))
            .collect();
        Self { helper, fixtures }
    }

    /// The port the vite fixture actually bound (ready-file
    /// handshake; call after [`FixtureSet::wait_ready`]). Port 0 in
    /// the script = OS-assigned, killing the probe-then-bind race.
    pub fn vite_port(&self) -> u16 {
        let text = std::fs::read_to_string(&self.get("vite").ready).unwrap_or_default();
        text.trim().parse::<u16>().unwrap_or_default()
    }

    /// One background line per in-tree fixture, for the root script
    /// (`sh -c "<lines>; wait"`). The `shell` kind is a direct `sh`
    /// child (TerminalShell role); the rest ride the python helper.
    pub fn root_script(&self) -> String {
        let mut lines = Vec::new();
        for fixture in &self.fixtures {
            let path = |suffix: &str| fixture.trap_log.with_extension(suffix);
            let _ = path;
            let trap = fixture.trap_log.to_string_lossy();
            let ready = fixture.ready.to_string_lossy();
            let helper = self.helper.to_string_lossy();
            let line = match fixture.kind {
                "vite" => format!("python3 {helper} --trap-log {trap} --ready {ready} --port 0 &"),
                "term-ignorer" => {
                    format!("python3 {helper} --trap-log {trap} --ready {ready} --ignore-term &")
                }
                "escapee" => {
                    format!("python3 {helper} --trap-log {trap} --ready {ready} --setsid &")
                }
                "mcp" => format!("python3 {helper} --trap-log {trap} --ready {ready} mcp-server &"),
                // bash, NOT sh: on Ubuntu /bin/sh is a symlink to
                // dash, and sysinfo reports the exe path differently
                // across reads (symlink vs resolved) — the helper's
                // full-triple verify flaps and the row flips
                // `unverifiable`. bash is a stable real path on both
                // platforms and still guesses TerminalShell.
                //
                // The trap logs through a FILE DESCRIPTOR opened once
                // (no per-trap path re-parsing), marks itself
                // installed, and the body sleeps in 1 s beats — the
                // trap handler runs at the next beat even under
                // deferred-handler semantics, and a stray long
                // foreground child can never swallow it.
                "shell" => format!(
                    "bash -c 'exec 9>>{trap}; \
                     trap \"echo TERM >&9\" TERM; trap \"echo HUP >&9\" HUP; \
                     trap \"echo INT >&9\" INT; touch {ready}; \
                     while :; do sleep 1; done' &"
                ),
                _ => unreachable!("fixture kind"),
            };
            lines.push(line);
        }
        lines.join("\n")
    }

    /// Waits (≤10 s) until every in-tree fixture touched its ready
    /// file — the handover-note rule: fix synchronisation, never the
    /// assertion.
    pub fn wait_ready(&self) -> bool {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if self.fixtures.iter().all(|f| f.ready.exists()) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }

    /// The fixture of one kind.
    pub fn get(&self, kind: &str) -> &Fixture {
        self.fixtures
            .iter()
            .find(|f| f.kind == kind)
            .unwrap_or_else(|| panic!("unknown fixture kind {kind}"))
    }

    /// Asserts the named fixtures received ZERO signals (the
    /// collateral law — panics listing every violation).
    pub fn assert_untouched(&self, kinds: &[&str], context: &str) {
        let mut violations = Vec::new();
        for kind in kinds {
            let fixture = self.get(kind);
            let signals = fixture.signals();
            if !signals.is_empty() {
                violations.push(format!("{kind}: {:?}", signals));
            }
        }
        assert!(
            violations.is_empty(),
            "{context}: collateral fixtures were signalled — {violations:?}"
        );
    }
}

/// Spawns a long-running fixture THROUGH a wrapper that exits at
/// once (the clippy `zombie_processes` law: every spawned Child is
/// waited; the fixture itself is the wrapper's re-parented orphan).
/// Returns the fixture pid (read from `pidfile`).
pub fn spawn_detached(helper: &Path, args: &[&str], trap: &Path, ready: &Path) -> u32 {
    let pidfile = trap.with_extension("pid");
    let mut line = format!(
        "python3 {} --trap-log {} --ready {}",
        helper.to_string_lossy(),
        trap.to_string_lossy(),
        ready.to_string_lossy()
    );
    for arg in args {
        line.push(' ');
        line.push_str(arg);
    }
    line.push_str(" & echo $! > ");
    line.push_str(&pidfile.to_string_lossy());
    let mut outer = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(line)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let _ = outer.wait();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if ready.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(ready.exists(), "detached fixture never became ready");
    std::fs::read_to_string(&pidfile)
        .unwrap()
        .trim()
        .parse::<u32>()
        .unwrap()
}

/// Spawns the UNMANAGED user server (outside any supervised root, no
/// ledger row — never ours to signal). Returns its pid.
pub fn spawn_unmanaged(dir: &Path) -> u32 {
    let helper = write_helper(dir);
    let trap = dir.join("unmanaged.traps");
    let ready = dir.join("unmanaged.ready");
    spawn_detached(&helper, &["--port", "0"], &trap, &ready)
}

/// The port the unmanaged server actually bound.
#[allow(dead_code)] // used by the suites that assert on it
pub fn unmanaged_port(dir: &Path) -> u16 {
    let text = std::fs::read_to_string(dir.join("unmanaged.ready")).unwrap_or_default();
    text.trim().parse::<u16>().unwrap_or_default()
}

/// The unmanaged fixture's paths (for zero-signal assertions).
pub fn unmanaged_fixture(dir: &Path) -> Fixture {
    fixture(dir, "unmanaged")
}

/// Is the pid alive? (`kill -0`)
pub fn alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// SIGKILL, ignoring errors (cleanup).
pub fn kill9(pid: u32) {
    let _ = std::process::Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .status();
}

/// Polls `pred` until it turns true or `timeout_ms` elapses.
pub fn wait_for(timeout_ms: u64, mut pred: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    while Instant::now() < deadline {
        if pred() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// A free ephemeral port (bind :0, read it, drop the listener).
#[allow(dead_code)] // superseded by the ready-file port handshake
pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// True when 127.0.0.1:port can be bound (the honest port-freed probe).
pub fn port_free(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}
