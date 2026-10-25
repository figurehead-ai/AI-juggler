//! AJ-E2302 (D-40, ADR-0022): how a session AI Juggler starts is run.
//!
//! - [`Runtime::Terminal`] is the tool's own terminal UI under a PTY, as
//!   before E23: always available, the fallback for every tool, and the
//!   only runtime of a tool without a protocol.
//! - [`Runtime::Driven`] is the tool's own protocol, spoken by AI Juggler
//!   from Rust to the user's own, unmodified binary, over pipes or a socket
//!   AI Juggler owns. An adapter offers it by declaring
//!   [`crate::Capabilities::PROTOCOL_SERVER`] and returning a
//!   [`DrivenRuntime`] from [`crate::HarnessAdapter::driven`] (the
//!   conformance kit holds both).
//!
//! The codecs live in the adapters; core, the supervisor and the UI see
//! only the neutral types here ([`events`]). No vendor SDK is bundled.
//!
//! **Answers are the user's (D-28 as amended by D-40).**
//! [`DrivenSession::answer`] carries the user's own click to the tool; it
//! is called only from a command the user's click runs, never from a
//! timer, a rule, an agent or a default (a guardrail test in the app
//! scans for any other caller). "Don't ask again" is an option the tool
//! itself offers ([`events::OptionEffect::AllowRule`]).

pub mod book;
pub mod events;
pub mod lines;

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::AdapterError;
use crate::launch::LaunchSpec;
use crate::model::{Account, Profile};

pub use events::{
    Approval, ApprovalKind, AskAnswer, AskId, AskOption, Choice, ErrorCategory, ItemKind,
    ItemSummary, LimitWindow, LimitsSnapshot, LiveStatusWord, ModelChoice, OptionEffect,
    PartAnswer, Question, QuestionPart, RuntimeEvent, TurnEnd, TurnInput, TurnOrigin, TurnUsage,
};

/// How one session runs. Chosen per launch from what its adapter
/// declares; the user's choice is remembered per profile and can be
/// changed per session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Runtime {
    /// The tool's terminal UI under a PTY.
    #[default]
    Terminal,
    /// The tool's protocol, driven by AI Juggler.
    Driven,
}

impl Runtime {
    /// The wire word (`terminal`, `driven`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Driven => "driven",
        }
    }

    /// The runtime a wire word names.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "terminal" => Some(Self::Terminal),
            "driven" => Some(Self::Driven),
            _ => None,
        }
    }
}

/// How settled an adapter's driven runtime is: a Beta one is off by
/// default, labelled, and chosen per session (ADR-0022 decision 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Maturity {
    /// Offered as the tool's default where the adapter says so.
    Stable,
    /// Only on the user's choice, per session, labelled Beta.
    Beta,
}

/// How AI Juggler reaches a driven session's process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transport {
    /// The child's stdin and stdout, one message per line.
    Stdio,
    /// A unix socket the child serves at this path, in AI Juggler's
    /// runtime folder (`0700`); the socket is made `0600` once it exists.
    Socket(PathBuf),
}

/// What the app hands an adapter for one driven launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrivenLaunch {
    /// The socket this launch may serve, in AI Juggler's runtime folder
    /// (already short enough for a socket address). A stdio runtime
    /// ignores it.
    pub socket: PathBuf,
    /// The tool's own conversation id to continue, when resuming.
    pub resume: Option<String>,
    /// A fresh id AI Juggler picked (a UUID), for a tool that lets the
    /// caller name a new conversation; a tool that names its own ignores it.
    pub fresh_id: String,
}

/// Everything needed to start one driven session. Produced by adapters;
/// consumed by the supervisor (the only layer that spawns).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrivenSpec {
    /// The process: the user's own binary, unmodified, piped (never a PTY).
    pub launch: LaunchSpec,
    /// How to reach it.
    pub transport: Transport,
    /// The conversation's id when it is known before the tool starts (or
    /// the one to resume, for a tool that resumes over its protocol).
    pub native_id: Option<String>,
    /// The model to start with, for a tool that takes it over its protocol
    /// rather than as a flag (the profile's launch settings).
    pub model: Option<String>,
}

/// A connected transport: what the codec reads and writes.
pub struct Connection {
    /// The tool's messages.
    pub reader: Box<dyn BufRead + Send>,
    /// AI Juggler's messages to the tool.
    pub writer: Box<dyn Write + Send>,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection").finish_non_exhaustive()
    }
}

/// Which driven session a terminal view is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewOf {
    /// The tool's conversation id.
    pub native_id: String,
    /// The socket it is served on, for a tool whose terminal attaches.
    pub socket: Option<PathBuf>,
}

/// The real terminal UI on a driven session's conversation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalView {
    /// Attached live to the same server, while the driver stays (Codex
    /// `--remote`).
    Attach(LaunchSpec),
    /// After the driver lets go at a turn boundary (Claude `--resume`):
    /// never two writers at once.
    AfterRelease(LaunchSpec),
}

/// How a driver lets go of a session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    /// When the running turn ends (at most this long), then close.
    AfterTurn(Duration),
    /// Now: interrupt a running turn, then close.
    Now,
}

/// What letting go gave.
#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseOutcome {
    /// The conversation's id (what a resume takes), when known.
    pub native_id: Option<String>,
    /// It was let go between turns (no turn was cut short).
    pub at_boundary: bool,
    /// Events that arrived while waiting for the turn's end.
    pub tail: Vec<RuntimeEvent>,
}

/// What an adapter adds for driven sessions (ADR-0022 decision 1).
pub trait DrivenRuntime: Send + Sync {
    /// The runtime this tool's sessions start in when the user hasn't
    /// chosen (data, not a branch: Codex says Driven, Claude Terminal).
    fn default_runtime(&self) -> Runtime;

    /// Whether driven is settled or a Beta.
    fn maturity(&self) -> Maturity;

    /// Why driven sessions can't run with the tool installed now (too old,
    /// a protocol AI Juggler hasn't pinned), in words; `None` when they
    /// can. May run the tool's own `--version` or schema generator, never
    /// against an account.
    fn unavailable(&self) -> Option<String>;

    /// The process to start for one driven session: the user's own binary,
    /// with the same isolation and removed credentials as its terminal
    /// launch, never an SDK, `--bare` or a permission bypass.
    ///
    /// # Errors
    ///
    /// [`AdapterError::InvalidLaunch`] for a missing binary or an id that
    /// reads as an option.
    fn driven_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        launch: &DrivenLaunch,
    ) -> Result<DrivenSpec, AdapterError>;

    /// Wraps the started process's transport in a session: the handshake
    /// (bounded in time), then events.
    ///
    /// # Errors
    ///
    /// When the handshake fails or the tool is not the one expected (the
    /// app then starts the terminal instead, with the reason).
    fn connect(
        &self,
        io: Connection,
        spec: &DrivenSpec,
        account: &Account,
    ) -> Result<Box<dyn DrivenSession>, AdapterError>;

    /// The real terminal UI on the same conversation.
    ///
    /// # Errors
    ///
    /// As [`crate::HarnessAdapter::launch_spec`].
    fn terminal_view(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        of: &ViewOf,
    ) -> Result<TerminalView, AdapterError>;
}

/// One driven session (research §6.1). Methods that ask the tool for
/// something only send; what comes back arrives as events through
/// [`DrivenSession::next_event`].
pub trait DrivenSession: Send {
    /// The conversation's id, once known.
    fn native_id(&self) -> Option<String>;

    /// The next event, waiting at most `timeout`; `None` when nothing came.
    ///
    /// # Errors
    ///
    /// When the tool's messages can't be read any more.
    fn next_event(&mut self, timeout: Duration) -> Result<Option<RuntimeEvent>, AdapterError>;

    /// Starts a turn with `input`. Only between turns (E18 delivers here,
    /// with the message's origin, never by typing into a PTY).
    ///
    /// # Errors
    ///
    /// [`AdapterError::InvalidLaunch`] while a turn runs; the write's error.
    fn send_turn(&mut self, input: TurnInput) -> Result<(), AdapterError>;

    /// Adds `input` to the running turn.
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] where the tool has no steering.
    fn steer(&mut self, input: TurnInput) -> Result<(), AdapterError> {
        let _ = input;
        Err(AdapterError::Unsupported(
            "this tool can't add to a running turn".into(),
        ))
    }

    /// Interrupts the running turn.
    ///
    /// # Errors
    ///
    /// The write's error.
    fn interrupt(&mut self) -> Result<(), AdapterError>;

    /// Carries the user's own answer to an open ask.
    ///
    /// # Errors
    ///
    /// [`AdapterError::InvalidLaunch`] for an ask that isn't open or an
    /// option it didn't offer; the write's error.
    fn answer(&mut self, ask: &AskId, answer: AskAnswer) -> Result<(), AdapterError>;

    /// Changes the model for the next turns.
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] where the tool can't.
    fn set_model(&mut self, model: &str) -> Result<(), AdapterError>;

    /// Asks the tool for the models it offers (they arrive as
    /// [`RuntimeEvent::Models`]).
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] where the tool can't list them.
    fn list_models(&mut self) -> Result<(), AdapterError> {
        Err(AdapterError::Unsupported(
            "this tool doesn't list its models".into(),
        ))
    }

    /// Compacts the conversation.
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] where the tool can't.
    fn compact(&mut self) -> Result<(), AdapterError> {
        Err(AdapterError::Unsupported(
            "this tool can't compact from AI Juggler".into(),
        ))
    }

    /// What the tool last reported about its limits, and asks it again
    /// where it can (the answer arrives as [`RuntimeEvent::LimitsUpdated`]).
    ///
    /// # Errors
    ///
    /// The write's error.
    fn limits(&mut self) -> Result<Option<LimitsSnapshot>, AdapterError>;

    /// Lets go of the session.
    ///
    /// # Errors
    ///
    /// The write's error.
    fn release(self: Box<Self>, how: Release) -> Result<ReleaseOutcome, AdapterError>;
}

/// The options an approval offers, by id: an answer naming another is
/// refused (the user can only pick what the tool offered).
///
/// # Errors
///
/// [`AdapterError::InvalidLaunch`] when `option` isn't one of them.
pub fn offered<'a>(approval: &'a Approval, option: &str) -> Result<&'a AskOption, AdapterError> {
    approval
        .options
        .iter()
        .find(|offered| offered.id == option)
        .ok_or_else(|| AdapterError::InvalidLaunch("that option wasn't offered".into()))
}

/// Flags that would bypass a tool's own permission prompts or its sign-in
/// (research §2.1): no driven or terminal launch AI Juggler builds ever
/// passes one (the conformance kit checks every spec).
pub const FORBIDDEN_ARGS: &[&str] = &[
    "--bare",
    "--dangerously-skip-permissions",
    "bypassPermissions",
    "--dangerously-bypass-approvals-and-sandbox",
    "--yolo",
];

/// Whether `args` hold a [`FORBIDDEN_ARGS`] word.
#[must_use]
pub fn has_forbidden_arg(args: &[String]) -> bool {
    args.iter().any(|arg| {
        FORBIDDEN_ARGS
            .iter()
            .any(|bad| arg == bad || arg.contains(bad))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtimes_have_wire_words() {
        for runtime in [Runtime::Terminal, Runtime::Driven] {
            assert_eq!(Runtime::from_word(runtime.as_str()), Some(runtime));
        }
        assert_eq!(Runtime::from_word("pty"), None);
        assert_eq!(Runtime::default(), Runtime::Terminal);
    }

    #[test]
    fn only_offered_options_are_answers() {
        let approval = Approval {
            id: AskId("1".into()),
            kind: ApprovalKind::Command,
            summary: "ls".into(),
            reason: None,
            options: vec![AskOption {
                id: "accept".into(),
                label: "Allow once".into(),
                effect: OptionEffect::AllowOnce,
            }],
        };
        assert!(offered(&approval, "accept").is_ok());
        assert!(offered(&approval, "acceptForSession").is_err());
    }

    #[test]
    fn forbidden_flags_are_found() {
        let args = |words: &[&str]| words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
        assert!(has_forbidden_arg(&args(&["-p", "--bare"])));
        assert!(has_forbidden_arg(&args(&[
            "--permission-mode",
            "bypassPermissions"
        ])));
        assert!(!has_forbidden_arg(&args(&["-p", "--verbose"])));
    }
}
