//! Launch requests and specifications (spec §4).

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Environment map injected into a launched harness process. Keys shadow the
/// inherited environment; per-profile isolation variables live here
/// (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `HOME`, …).
pub type EnvMap = BTreeMap<String, String>;

/// How the caller wants the harness to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchMode {
    /// Full interactive session (needs a PTY).
    Interactive,
    /// Single-shot run with an initial prompt (spec §5.1 driven runs).
    Headless {
        /// The prompt handed to the harness.
        prompt: String,
    },
    /// Resume a specific native session by id (the juggling handover's
    /// target: same cwd, session transplanted from another account).
    Resume {
        /// The harness's own session id (transcript file stem).
        session_id: String,
        /// An optional first turn delivered on resume — the E207
        /// "While you were away" delta (one summarised turn with the
        /// pack attached; NEVER a copy-back over the transcript).
        delta_prompt: Option<String>,
    },
}

/// Everything needed to spawn one harness session. Produced by adapters;
/// consumed by the supervisor — the only layer allowed to spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    /// Program to execute (the real, unmodified harness binary).
    pub program: PathBuf,
    /// Argument vector.
    pub args: Vec<String>,
    /// Environment overlay (merged over the supervisor's base environment).
    pub env: EnvMap,
    /// Working directory for the session.
    pub cwd: PathBuf,
    /// Whether the run needs a PTY (interactive modes).
    pub needs_pty: bool,
}

/// Convenience constructor used by adapters.
#[must_use]
pub fn headless_spec(program: PathBuf, args: Vec<String>, env: EnvMap, cwd: PathBuf) -> LaunchSpec {
    LaunchSpec {
        program,
        args,
        env,
        cwd,
        needs_pty: false,
    }
}

/// Convenience constructor used by adapters.
#[must_use]
pub fn interactive_spec(
    program: PathBuf,
    args: Vec<String>,
    env: EnvMap,
    cwd: PathBuf,
) -> LaunchSpec {
    LaunchSpec {
        program,
        args,
        env,
        cwd,
        needs_pty: true,
    }
}

/// A desktop-class harness launch (AJ-E413): a windowed application
/// tracked as an Instance in the same spawn ledger as terminal
/// instances — **no PTY, no output capture** (the window is the
/// surface; the terminal card must not render for this kind).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopLaunch {
    /// Executable to spawn (the real, unmodified app binary).
    pub program: PathBuf,
    /// Bundle identifier when the platform has one (macOS).
    pub bundle_id: Option<String>,
    /// Argument vector.
    pub args: Vec<String>,
    /// Environment overlay (isolation vars where the vendor supports
    /// them — see `isolation_label` for the honest limits statement).
    pub env: EnvMap,
    /// Working directory.
    pub cwd: PathBuf,
    /// The per-Provider isolation claim, shown verbatim on the Instance
    /// (Req.1): e.g. `Single-account app — Juggler tracks, cannot
    /// isolate` for apps Juggler cannot partition.
    pub isolation_label: &'static str,
}

#[cfg(test)]
mod tests {
    use super::{EnvMap, headless_spec, interactive_spec};
    use std::path::PathBuf;

    #[test]
    fn constructors_set_pty_flag_and_carry_fields() {
        let env = EnvMap::from([("K".to_string(), "V".to_string())]);
        let spec = headless_spec(
            PathBuf::from("/bin/true"),
            vec!["a".into()],
            env.clone(),
            PathBuf::from("/tmp"),
        );
        assert!(!spec.needs_pty);
        assert_eq!(spec.program, PathBuf::from("/bin/true"));
        assert_eq!(spec.args, vec!["a".to_string()]);
        assert_eq!(spec.env, env);
        assert_eq!(spec.cwd, PathBuf::from("/tmp"));

        let spec = interactive_spec(
            PathBuf::from("/bin/true"),
            Vec::new(),
            EnvMap::new(),
            PathBuf::from("/tmp"),
        );
        assert!(spec.needs_pty);
    }
}
