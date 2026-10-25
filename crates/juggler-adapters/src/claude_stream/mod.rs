//! AJ-E2308 (D-40, ADR-0022 decision 4): Claude Code without a terminal,
//! a per-session **Beta**: AI Juggler speaks Claude Code's stream-json
//! protocol from Rust to the user's own, unmodified `claude`, and its
//! questions and approvals are answered in AI Juggler.
//!
//! - **The launch**: `claude -p --input-format stream-json --output-format
//!   stream-json --verbose --permission-prompt-tool stdio` with
//!   `--session-id <new id>` or `--resume <id>`, the profile's
//!   `CLAUDE_CONFIG_DIR` and removed credentials (the terminal launch's own
//!   environment). Never `--bare`, `bypassPermissions`, a permission bypass
//!   or a spoofed entrypoint; no product text added to the conversation.
//! - **Only in a trusted folder**: headless Claude Code treats any folder
//!   as trusted and runs its hooks, so a folder whose trust question Claude
//!   Code hasn't had answered starts in the terminal, where it asks.
//! - **One writer**: Open in terminal lets go at a turn boundary and the
//!   terminal resumes with `claude --resume <id>`.
//! - **A Beta**: never a tool's or a profile's default, only picked per
//!   session with Settings › Sessions › Run without a terminal (Beta) on.
//!   It becomes a default only after ADR-0022 decision 4's gates.

pub mod codec;
pub mod session;

use std::collections::BTreeMap;
use std::path::Path;

use juggler_adapter::runtime::{
    Connection, DrivenLaunch, DrivenRuntime, DrivenSession, DrivenSpec, Maturity, Runtime,
    TerminalView, Transport, ViewOf,
};
use juggler_adapter::{AdapterError, HarnessAdapter, InstallStatus, LaunchMode};
use juggler_core::model::{Account, Profile};

use crate::claude_code::ClaudeCodeAdapter;

/// The oldest Claude Code driven headless (verified live 2026-10-25).
pub const DRIVEN_FROM: &str = "2.1.302";

/// The headless launch's own options (before the session's).
pub const HEADLESS_ARGS: [&str; 8] = [
    "-p",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--verbose",
    "--permission-prompt-tool",
    "stdio",
];

/// The one part of Claude Code's state file the trust check keeps: each
/// project's "trust question answered" flag (every other value is skipped
/// while parsing, never kept).
#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct TrustShape {
    projects: BTreeMap<String, TrustEntry>,
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct TrustEntry {
    #[serde(rename = "hasTrustDialogAccepted")]
    trusted: bool,
}

/// Whether Claude Code's own trust question was answered for `cwd` in the
/// account at `state_dir`.
#[must_use]
pub fn trusted(state_dir: &Path, cwd: &Path) -> bool {
    let Ok(file) = std::fs::File::open(crate::claude_code::claude_state_file(state_dir)) else {
        return false;
    };
    let Ok(shape) = serde_json::from_reader::<_, TrustShape>(std::io::BufReader::new(file)) else {
        return false;
    };
    let wanted = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    shape.projects.iter().any(|(path, entry)| {
        entry.trusted && Path::new(path).canonicalize().is_ok_and(|p| p == wanted)
    })
}

fn numbers(version: &str) -> Option<Vec<u64>> {
    version
        .split_whitespace()
        .next()?
        .split('.')
        .map(|part| part.parse().ok())
        .collect()
}

impl DrivenRuntime for ClaudeCodeAdapter {
    fn label(&self) -> &'static str {
        "Claude Code without a terminal"
    }

    fn default_runtime(&self) -> Runtime {
        Runtime::Terminal
    }

    fn maturity(&self) -> Maturity {
        Maturity::Beta
    }

    /// Anthropic's legal page and Help Center (research §2.1, checked
    /// 2026-10-25).
    fn terms_note(&self) -> Option<&'static str> {
        Some(
            "Its sessions run in their terminal. Run without a terminal (Beta) runs your own Claude Code with your own sign-in, which Anthropic's terms allow; Anthropic may count it as Agent SDK use.",
        )
    }

    fn unavailable(&self) -> Option<String> {
        let report = self.discover();
        let InstallStatus::Installed { version } = report.install else {
            return Some("Claude Code isn't installed here".into());
        };
        match (numbers(&version), numbers(DRIVEN_FROM)) {
            (Some(have), Some(need)) if have >= need => None,
            _ => Some(format!(
                "Claude Code {version} is older than {DRIVEN_FROM}, the first run without a terminal"
            )),
        }
    }

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
        crate::codex::refuse_option_like(&launch.fresh_id)?;
        let mode = launch
            .resume
            .as_ref()
            .map_or(LaunchMode::Interactive, |id| LaunchMode::Resume {
                session_id: id.clone(),
                delta_prompt: None,
            });
        let mut spec = self.launch_spec(account, profile, cwd, &mode)?;
        if !trusted(&account.state_dir, &spec.cwd) {
            return Err(AdapterError::InvalidLaunch(
                "Claude Code hasn't been trusted in this folder yet: start it once in its terminal, where it asks"
                    .into(),
            ));
        }
        let id = launch
            .resume
            .clone()
            .unwrap_or_else(|| launch.fresh_id.clone());
        let mut args: Vec<String> = HEADLESS_ARGS.iter().map(|a| (*a).to_owned()).collect();
        if launch.resume.is_some() {
            args.extend(["--resume".to_owned(), id.clone()]);
        } else {
            args.extend(["--session-id".to_owned(), id.clone()]);
        }
        if let Some(model) = &profile.settings_overlay.model {
            args.extend(["--model".to_owned(), model.clone()]);
        }
        spec.args = args;
        spec.needs_pty = false;
        Ok(DrivenSpec {
            launch: spec,
            transport: Transport::Stdio,
            native_id: Some(id),
            model: profile.settings_overlay.model.clone(),
        })
    }

    fn connect(
        &self,
        io: Connection,
        spec: &DrivenSpec,
        account: &Account,
    ) -> Result<Box<dyn DrivenSession>, AdapterError> {
        let id = spec
            .native_id
            .clone()
            .ok_or_else(|| AdapterError::InvalidLaunch("no session id".into()))?;
        let session = session::ClaudeSession::connect(io, &id, &account.state_dir)?;
        Ok(Box::new(session))
    }

    /// `claude --resume <id>` in a terminal, after the driver let go.
    fn terminal_view(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        of: &ViewOf,
    ) -> Result<TerminalView, AdapterError> {
        let mode = LaunchMode::Resume {
            session_id: of.native_id.clone(),
            delta_prompt: None,
        };
        self.launch_spec(account, profile, cwd, &mode)
            .map(TerminalView::AfterRelease)
    }
}
