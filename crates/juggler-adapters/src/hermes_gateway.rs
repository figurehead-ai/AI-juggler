//! Hermes Agent's gateway, per account (AJ-E1607 Req.2).
//!
//! The gateway is the one writer of a Hermes home's sessions: the CLI, the
//! TUI, Desktop, ACP and cron are its clients over `/api/ws` (research
//! `hermes.md` §4). What the source says of its life
//! (`NousResearch/hermes-agent@422ad186`):
//!
//! - It records itself in the home: `gateway.pid`, a JSON record
//!   `{"pid", "kind": "hermes-gateway", "argv", "start_time"}` (an older
//!   one a bare pid), with `gateway.lock` and `gateway_state.json` beside it
//!   (`gateway/status.py`, `_get_pid_path`, `_build_pid_record`).
//! - One gateway per OS user multiplexes every profile of one root: its
//!   host lock and rendezvous record (`host-gateway.json`, a 0600 token)
//!   live in `$HERMES_GATEWAY_LOCK_DIR`, else
//!   `$XDG_STATE_HOME/hermes/gateway-locks` (`gateway/status.py`,
//!   `_get_lock_dir`; `gateway/host_rendezvous.py`). An account AI Juggler
//!   makes is pinned to a lock folder of its own, so its gateway never
//!   attaches to the user's or another account's.
//! - A client with no gateway starts one itself: on macOS a one-off
//!   launchd job labelled `com.nousresearch.hermes.runtime.<id>` that ends
//!   itself when idle (`--idle-exit`;
//!   `hermes_cli/gateway_runtime_start.py`, `spawn_unmanaged_gateway`;
//!   `hermes_cli/gateway_launchd_runtime.py`). That one is Hermes' own,
//!   in the account's home and lock folder; Hermes' own
//!   `hermes gateway stop` ends it ([`stop`]).
//! - `hermes gateway run --external-supervisor` runs one in the foreground
//!   owned by the process that started it: an in-chat restart or update
//!   exits back to that owner instead of starting a detached replacement
//!   (`hermes_cli/subcommands/gateway.py`). That is [`OwnedGateway`]: AI
//!   Juggler's, started in a process group of its own and ended with
//!   everything it forked.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use juggler_adapter::EnvMap;

use crate::common::child::Reaped;

/// `hermes gateway run`, in the foreground, owned by AI Juggler.
pub const RUN_ARGS: &[&str] = &["gateway", "run", "--quiet", "--external-supervisor"];
/// Hermes' own stop of the gateway serving a home.
pub const STOP_ARGS: &[&str] = &["gateway", "stop"];
/// The gateway's record in its home.
pub const PID_FILE: &str = "gateway.pid";
/// The kind its record names.
pub const KIND: &str = "hermes-gateway";
/// How long Hermes' own stop may take.
const STOP_DEADLINE: Duration = Duration::from_secs(30);

/// A gateway's record in a Hermes home.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayRecord {
    /// Its process.
    pub pid: u32,
    /// How it was started, as it recorded it (empty for an old record).
    pub argv: Vec<String>,
}

/// The gateway record of the Hermes home `home`, when there is one in a
/// shape read: a JSON record of kind [`KIND`], or an older bare pid.
#[must_use]
pub fn recorded(home: &Path) -> Option<GatewayRecord> {
    let path = home.join(PID_FILE);
    let small = std::fs::metadata(&path).is_ok_and(|meta| meta.is_file() && meta.len() < 64 * 1024);
    let text = small
        .then(|| std::fs::read_to_string(&path).ok())
        .flatten()?;
    let text = text.trim();
    let pid = |value: i64| u32::try_from(value).ok().filter(|pid| *pid > 1);
    if let Ok(bare) = text.parse::<i64>() {
        return pid(bare).map(|pid| GatewayRecord {
            pid,
            argv: Vec::new(),
        });
    }
    let record: serde_json::Value = serde_json::from_str(text).ok()?;
    if record.get("kind").and_then(serde_json::Value::as_str) != Some(KIND) {
        return None;
    }
    Some(GatewayRecord {
        pid: pid(record.get("pid")?.as_i64()?)?,
        argv: record
            .get("argv")
            .and_then(serde_json::Value::as_array)
            .map(|argv| {
                argv.iter()
                    .filter_map(|arg| arg.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// The process of the gateway serving the home `home` now: its record's,
/// when that process is there.
#[must_use]
pub fn running(home: &Path) -> Option<u32> {
    recorded(home)
        .map(|record| record.pid)
        .filter(|pid| juggler_adapter::process::alive(*pid))
}

/// A Hermes command for the account: the user's own binary with the
/// user's PATH and the account's environment `env` (a variable that runs
/// unset is left out), in its home.
fn hermes(bin: &Path, home: &Path, env: &EnvMap, args: &[&str]) -> std::process::Command {
    let mut command = juggler_adapter::user_path::command(bin);
    command.args(args).current_dir(home);
    for (key, value) in env {
        if value != juggler_adapter::launch::ENV_UNSET {
            command.env(key, value);
        }
    }
    command
}

/// Hermes' own stop of the gateway serving the account's home (one Hermes
/// started for a client, or any other): Hermes proves its identity from
/// its record before signalling it. Bounded in time.
///
/// # Errors
///
/// A plain message when it can't run, fails, or runs too long.
pub fn stop(bin: &Path, home: &Path, env: &EnvMap) -> Result<(), String> {
    let out = crate::common::child::output_within(
        &mut hermes(bin, home, env, STOP_ARGS),
        Instant::now() + STOP_DEADLINE,
        16 * 1024,
    )
    .map_err(|_| "`hermes gateway stop` couldn't be started".to_owned())?;
    match out.status {
        Some(status) if status.success() => Ok(()),
        Some(_) => Err("`hermes gateway stop` failed".into()),
        None => Err("`hermes gateway stop` didn't answer in time".into()),
    }
}

/// A gateway AI Juggler started for an account and owns: it leads a
/// process group of its own, and ending it (or dropping it) ends the
/// group, workers included, and reaps it.
pub struct OwnedGateway {
    process: Reaped,
    home: PathBuf,
    pid: u32,
}

impl OwnedGateway {
    /// Starts `hermes gateway run` for the account's home `home` with its
    /// environment `env` (its isolation and guard), output dropped.
    ///
    /// # Errors
    ///
    /// A plain message when it can't be started, or another gateway
    /// already serves the home (its clients would talk to that one).
    pub fn start(bin: &Path, home: &Path, env: &EnvMap) -> Result<Self, String> {
        if let Some(pid) = running(home) {
            return Err(format!(
                "a Hermes gateway already serves this account (process {pid}); stop it first"
            ));
        }
        let mut command = hermes(bin, home, env, RUN_ARGS);
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let process = Reaped::spawn_group(&mut command)
            .map_err(|_| "Hermes Agent's gateway couldn't be started".to_owned())?;
        let pid = process.pid();
        Ok(Self {
            process,
            home: home.to_path_buf(),
            pid,
        })
    }

    /// Its process (its group's leader).
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// The home it serves.
    #[must_use]
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// Whether it still runs.
    pub fn running(&mut self) -> bool {
        self.process.running()
    }

    /// Ends it and everything it forked, and reaps it.
    pub fn stop(mut self) {
        self.process.end();
    }
}
