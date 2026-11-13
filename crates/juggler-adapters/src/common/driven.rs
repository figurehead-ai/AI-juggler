//! What the driven runtimes share around their codecs: the terminal launch
//! a driven launch starts from, and the terminal resume after the driver
//! lets go.

use std::path::Path;

use juggler_adapter::runtime::{DrivenLaunch, DrivenSpec, TerminalView, Transport, ViewOf};
use juggler_adapter::{AdapterError, LaunchMode, LaunchSpec, ToolLaunch};
use juggler_core::model::{Account, Profile};

use crate::codex::refuse_option_like;

/// The terminal launch's mode a driven launch starts from: a resume of
/// [`DrivenLaunch::resume`], else a new interactive session.
///
/// # Errors
///
/// A resume id that reads as an option (`refuse_option_like`).
pub fn launch_mode(launch: &DrivenLaunch) -> Result<LaunchMode, AdapterError> {
    match &launch.resume {
        Some(id) => {
            refuse_option_like(id)?;
            Ok(resume(id))
        }
        None => Ok(LaunchMode::Interactive),
    }
}

/// A plain resume of `id` (no first turn).
#[must_use]
pub fn resume(id: &str) -> LaunchMode {
    LaunchMode::Resume {
        session_id: id.to_owned(),
        delta_prompt: None,
    }
}

/// A driven launch over the tool's stdin and stdout: the terminal launch
/// `spec` with `args` instead of its own and no PTY, continuing
/// `native_id` (or naming a new one) with `model`.
#[must_use]
pub fn over_stdio(
    mut spec: LaunchSpec,
    args: Vec<String>,
    native_id: Option<String>,
    model: Option<String>,
) -> DrivenSpec {
    spec.args = args;
    spec.needs_pty = false;
    DrivenSpec {
        launch: spec,
        transport: Transport::Stdio,
        native_id,
        model,
        history: None,
    }
}

/// The tool's own terminal resume of `of`'s conversation, after the driver
/// lets go at a turn boundary: one writer at a time.
///
/// # Errors
///
/// An id that reads as an option; the launch's own errors.
pub fn terminal_resume(
    tool: &dyn ToolLaunch,
    account: &Account,
    profile: &Profile,
    cwd: &Path,
    of: &ViewOf,
) -> Result<TerminalView, AdapterError> {
    refuse_option_like(&of.native_id)?;
    tool.launch_spec(account, profile, cwd, &resume(&of.native_id))
        .map(TerminalView::AfterRelease)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn launch(resume: Option<&str>) -> DrivenLaunch {
        DrivenLaunch {
            socket: PathBuf::new(),
            resume: resume.map(str::to_owned),
            fresh_id: "fresh".into(),
            history: None,
        }
    }

    #[test]
    fn a_driven_launch_resumes_its_id_or_starts_new() {
        assert_eq!(
            launch_mode(&launch(None)).expect("new"),
            LaunchMode::Interactive
        );
        assert_eq!(
            launch_mode(&launch(Some("abc"))).expect("resume"),
            resume("abc")
        );
        assert!(launch_mode(&launch(Some("--help"))).is_err());
        assert!(launch_mode(&launch(Some(""))).is_err());
    }
}
