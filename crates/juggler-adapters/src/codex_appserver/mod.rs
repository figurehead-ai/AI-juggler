//! AJ-E2303 (D-40, ADR-0022 decision 3): Codex driven through the user's
//! own `codex app-server`, the default runtime for the Codex sessions AI
//! Juggler starts.
//!
//! - **One server per session**, `codex app-server --listen unix://<socket>`
//!   with the profile's `CODEX_HOME` and `CODEX_SQLITE_HOME` (the terminal
//!   launch's own environment), on a socket in AI Juggler's runtime folder.
//!   The app spawns it under its supervisor (a child in the spawn ledger)
//!   and stops it with the session; AI Juggler never attaches to Codex's
//!   own daemon.
//! - **Honest client**: `clientInfo { name: "ai_juggler", title: "AI
//!   Juggler" }`; a server whose `codexHome` isn't the profile's is refused.
//!   Stable methods only ([`surface`]).
//! - **Pinned protocol**: from [`surface::DRIVEN_FROM`]; a newer Codex is
//!   driven only while its own generated schema has every method AI
//!   Juggler uses. Anything else starts in the terminal, with the reason.
//! - **The real TUI on the same thread**: `codex --remote unix://<socket>
//!   resume <thread>` ([`DrivenRuntime::terminal_view`]).
//! - **Answers are the user's**: approvals and questions become asks with
//!   Codex's own options ([`asks`]); nothing here answers one by itself.

pub mod asks;
pub mod codec;
pub mod rpc;
pub mod session;
pub mod surface;

use std::path::{Path, PathBuf};

use juggler_adapter::runtime::{
    Connection, DrivenLaunch, DrivenRuntime, DrivenSession, DrivenSpec, Maturity, Runtime,
    TerminalView, Transport, ViewOf,
};
use juggler_adapter::{AdapterError, HarnessAdapter, InstallStatus, LaunchMode};
use juggler_core::model::{Account, Profile};

use crate::codex::CodexAdapter;

/// `clientInfo.name`: how AI Juggler names itself to Codex (and to the
/// vendor's compliance logs), never another client's name.
pub const CLIENT_NAME: &str = "ai_juggler";
/// `clientInfo.title`.
pub const CLIENT_TITLE: &str = "AI Juggler";

/// The app-server's address for a socket path.
#[must_use]
pub fn socket_url(socket: &Path) -> String {
    format!("unix://{}", socket.to_string_lossy())
}

/// The variables a schema check removes (an API key never reaches it).
const SCHEMA_UNSET: &[&str] = &["CODEX_API_KEY", "OPENAI_API_KEY"];

/// Generates the installed Codex's protocol schema offline, with a scratch
/// `CODEX_HOME` (no sign-in, no network), and says what AI Juggler uses
/// that it lacks.
fn schema_gap(bin: &Path) -> Result<Vec<&'static str>, String> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let scratch = std::env::temp_dir().join(format!(
        "ai-juggler-codex-schema-{}-{nanos}",
        std::process::id()
    ));
    let home = scratch.join("codex-home");
    let out = scratch.join("schema");
    std::fs::create_dir_all(&home).map_err(|e| e.to_string())?;
    let mut command = juggler_adapter::user_path::command(bin);
    command
        .args(["app-server", "generate-json-schema", "--out"])
        .arg(&out)
        .env("CODEX_HOME", &home)
        .env_remove("CODEX_SQLITE_HOME")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    for key in SCHEMA_UNSET {
        command.env_remove(key);
    }
    let status = command.status().map_err(|e| e.to_string());
    let gap = surface::missing_in(&out);
    let _ = std::fs::remove_dir_all(&scratch);
    match status {
        Ok(status) if status.success() => Ok(gap),
        Ok(status) => Err(format!("its schema generator exited with {status}")),
        Err(error) => Err(error),
    }
}

impl CodexAdapter {
    /// Why driven sessions can't run, worked out once per binary and
    /// version (see [`DrivenRuntime::unavailable`]).
    fn driven_check(&self) -> Option<String> {
        let report = self.discover();
        let (Some(bin), InstallStatus::Installed { version }) = (report.binary, report.install)
        else {
            return Some("Codex isn't installed here".into());
        };
        let key = (bin.clone(), version.clone());
        if let Ok(cache) = self.driven_cache().lock()
            && let Some((known, why)) = cache.as_ref()
            && *known == key
        {
            return why.clone();
        }
        let why = if surface::new_enough(&version) {
            match schema_gap(&bin) {
                Ok(gap) if gap.is_empty() => None,
                Ok(gap) => Some(format!(
                    "Codex {version}'s app-server lacks what AI Juggler uses: {}",
                    gap.join(", ")
                )),
                Err(why) => Some(format!(
                    "Codex {version}'s protocol couldn't be checked: {why}"
                )),
            }
        } else {
            Some(format!(
                "Codex {version} is older than {}, the first AI Juggler drives",
                surface::DRIVEN_FROM
            ))
        };
        if let Ok(mut cache) = self.driven_cache().lock() {
            *cache = Some((key, why.clone()));
        }
        why
    }
}

impl DrivenRuntime for CodexAdapter {
    fn default_runtime(&self) -> Runtime {
        Runtime::Driven
    }

    fn maturity(&self) -> Maturity {
        Maturity::Stable
    }

    fn unavailable(&self) -> Option<String> {
        self.driven_check()
    }

    /// The terminal launch's binary, environment and folder, serving
    /// app-server on `launch.socket` instead of the TUI.
    fn driven_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        launch: &DrivenLaunch,
    ) -> Result<DrivenSpec, AdapterError> {
        if let Some(id) = &launch.resume {
            crate::codex::refuse_option_like(id)?;
        }
        let mode = launch
            .resume
            .as_ref()
            .map_or(LaunchMode::Interactive, |id| LaunchMode::Resume {
                session_id: id.clone(),
                delta_prompt: None,
            });
        let mut spec = self.launch_spec(account, profile, cwd, &mode)?;
        spec.args = vec![
            "app-server".to_owned(),
            "--listen".to_owned(),
            socket_url(&launch.socket),
        ];
        spec.needs_pty = false;
        Ok(DrivenSpec {
            launch: spec,
            transport: Transport::Socket(launch.socket.clone()),
            native_id: launch.resume.clone(),
            model: profile.settings_overlay.model.clone(),
        })
    }

    fn connect(
        &self,
        io: Connection,
        spec: &DrivenSpec,
        account: &Account,
    ) -> Result<Box<dyn DrivenSession>, AdapterError> {
        let session = session::CodexSession::connect(
            io,
            &account.state_dir,
            &spec.launch.cwd,
            spec.native_id.as_deref(),
            spec.model.as_deref(),
        )?;
        Ok(Box::new(session))
    }

    /// Attached live with `--remote` while the server runs; after it, the
    /// terminal resume.
    fn terminal_view(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        of: &ViewOf,
    ) -> Result<TerminalView, AdapterError> {
        crate::codex::refuse_option_like(&of.native_id)?;
        let mode = LaunchMode::Resume {
            session_id: of.native_id.clone(),
            delta_prompt: None,
        };
        let mut spec = self.launch_spec(account, profile, cwd, &mode)?;
        let Some(socket) = &of.socket else {
            return Ok(TerminalView::AfterRelease(spec));
        };
        spec.args = vec![
            "--remote".to_owned(),
            socket_url(socket),
            "resume".to_owned(),
            of.native_id.clone(),
        ];
        Ok(TerminalView::Attach(spec))
    }
}

/// The cache [`CodexAdapter::driven_check`] keeps.
pub(crate) type DrivenCache =
    std::sync::Arc<std::sync::Mutex<Option<((PathBuf, String), Option<String>)>>>;
