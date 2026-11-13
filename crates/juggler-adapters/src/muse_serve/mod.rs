//! AJ-E1608 R2 (D-40, ADR-0022): Muse Code driven over its own session
//! server, `muse serve` (the Muse Session Protocol, MSP, schema version 1).
//!
//! - **One server per session, over stdio**, started as the terminal
//!   launch is: the same binary, the account's isolation and sign-in, and
//!   `serve` instead of the TUI. It ends when its stdin closes.
//! - **The protocol** ([`codec`]): `initialize` (stable API only), then
//!   `session/start` or `session/resume`, `turn/start`, `turn/steer`,
//!   `turn/interrupt`, `session/setModel`, `model/list`, and `usage/read`
//!   for the plan's windows. The session's events come on the same
//!   connection: the start or resume subscribes it.
//! - **The isolation is checked**: Muse names the folder it keeps sessions
//!   in (`museHome`), and a run that isn't this account's is refused
//!   ([`session::connect`]).
//! - **Answers are the user's**: Muse's approvals and questions become
//!   asks with its own choices; nothing here answers one.
//! - **Per-session MCP servers** (`session/start`'s `config.mcpServers`)
//!   are carried by the codec; AI Juggler hands none yet (it keeps no MCP
//!   set of its own for a Muse session).
//! - **The terminal view** is Muse's own `muse resume <id>` once the driver
//!   lets go between turns: a stdio host and a terminal never drive one
//!   session at once (Muse refuses a resume while it is in use).

pub mod codec;
pub mod session;

use std::path::Path;

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::{
    Connection, DrivenLaunch, DrivenRuntime, DrivenSession, DrivenSpec, Maturity, Runtime,
    TerminalView, ViewOf,
};
use juggler_core::model::{Account, Profile};

use crate::muse::MuseAdapter;

/// The server's arguments.
pub const SERVER_ARGS: &[&str] = &["serve"];

impl DrivenRuntime for MuseAdapter {
    fn label(&self) -> &'static str {
        "Muse's session server"
    }

    fn default_runtime(&self) -> Runtime {
        Runtime::Driven
    }

    fn maturity(&self) -> Maturity {
        Maturity::Stable
    }

    /// `turn/steer`.
    fn steers(&self) -> bool {
        true
    }

    fn terms_note(&self) -> Option<&'static str> {
        Some(
            "Its sessions run through your own Muse Code's session server (muse serve), with that account's own sign-in: Meta's SDK is made for that, and AI Juggler never takes Muse's credential.",
        )
    }

    fn unavailable(&self) -> Option<String> {
        self.require_bin().err().map(|error| error.to_string())
    }

    /// The terminal launch's binary, environment and folder, serving MSP
    /// on its stdin and stdout instead of the TUI; a resume continues its
    /// id over the protocol, the model is sent with the start.
    fn driven_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        launch: &DrivenLaunch,
    ) -> Result<DrivenSpec, AdapterError> {
        crate::common::driven::serving(self, account, profile, cwd, launch, SERVER_ARGS)
    }

    fn connect(
        &self,
        io: Connection,
        spec: &DrivenSpec,
        account: &Account,
    ) -> Result<Box<dyn DrivenSession>, AdapterError> {
        let muse_home = crate::muse::data_of(&account.state_dir);
        let cwd = spec.launch.cwd.to_string_lossy();
        let open = session::Open {
            muse_home: &muse_home,
            cwd: &cwd,
            resume: spec.native_id.as_deref(),
            model: spec.model.as_deref(),
        };
        Ok(Box::new(session::connect(io, &open)?))
    }

    /// Muse's own `muse resume <id>`, after the driver lets go.
    fn terminal_view(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        view: &ViewOf,
    ) -> Result<TerminalView, AdapterError> {
        crate::common::driven::terminal_resume(self, account, profile, cwd, view)
    }
}
