//! AJ-E1605 R3 (D-40, ADR-0022): `ZCode` driven through its own engine's
//! app-server, the only way its app runs the engine.
//!
//! - **One server per session, over stdio**, started as `ZCode`'s desktop
//!   app starts it: the app's own runtime in Node.js mode with the engine
//!   and `app-server --stdio` (`ELECTRON_RUN_AS_NODE=1`), with the App's
//!   tree variables and the app's built-in provider settings
//!   (the adapter's `engine_spec`). The app spawns it
//!   under its supervisor; it ends when its stdin closes.
//! - **The protocol** ([`codec`]): `session/create` or `session/resume`,
//!   `session/subscribe` for the session's events, `session/send` for a
//!   turn, `session/stop` to interrupt. Read in the engine's own code
//!   (0.16.9); research `docs/research/zcode.md` § E.
//! - **Answers are the user's**: the engine's permission and question
//!   requests become asks with its own options; nothing here answers one.
//!   Its request for a model call's headers is answered with none (no
//!   credential, ADR-0003); whether the engine then uses the App's own
//!   sign-in is **UNCONFIRMED** until the lab check (AJ-E1612).
//! - **A pack comes in by `ZCode`'s own import**: a new session with
//!   `importedHistory` (no model call), the pack's turns as its messages
//!   ([`juggler_adapter::runtime::DrivenLaunch::history`]).
//! - **No terminal**: the app ships no terminal UI, so there is none to
//!   fall back to or to view the session in
//!   ([`DrivenRuntime::terminal_ui`]).
//! - **Per-session MCP servers** (`mcpServers` on `session/create`) aren't
//!   passed: AI Juggler keeps no MCP set of an App's own for `ZCode` (the
//!   engine reads one set from the home folder for every App,
//!   [`crate::zcode_inventory`]).

pub mod codec;
pub mod session;

use std::path::Path;

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::{
    Connection, DrivenLaunch, DrivenRuntime, DrivenSession, DrivenSpec, Maturity, Runtime,
    TerminalView, Transport, ViewOf,
};
use juggler_core::model::{Account, Profile};

use crate::zcode::{NO_TERMINAL, ZCodeAdapter};
use session::{Open, ZCodeSession};

/// The engine's arguments for its server.
pub const SERVER_ARGS: &[&str] = &["app-server", "--stdio"];

/// The id `ZCode` takes for an imported session: `sess_<uuid>`.
#[must_use]
pub fn import_id(fresh: &str) -> String {
    format!("sess_{fresh}")
}

impl DrivenRuntime for ZCodeAdapter {
    fn label(&self) -> &'static str {
        "ZCode's app server"
    }

    fn default_runtime(&self) -> Runtime {
        Runtime::Driven
    }

    fn maturity(&self) -> Maturity {
        Maturity::Stable
    }

    fn terms_note(&self) -> Option<&'static str> {
        Some(
            "Its sessions run on ZCode's own engine, started the way ZCode's desktop app starts it; AI Juggler hands it no credential.",
        )
    }

    fn terminal_ui(&self) -> bool {
        false
    }

    fn imports_history(&self) -> bool {
        true
    }

    fn unavailable(&self) -> Option<String> {
        self.require_install().err().map(|error| error.to_string())
    }

    /// The engine's server on the app's runtime, in the App's tree: a new
    /// session starts in the profile's folder, a resumed one in `cwd`.
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
        let cwd = match &launch.resume {
            Some(_) => cwd.to_path_buf(),
            None => profile
                .default_cwd
                .clone()
                .unwrap_or_else(|| cwd.to_path_buf()),
        };
        let spec = self.engine_spec(
            &account.state_dir,
            &profile.settings_overlay.env,
            &cwd,
            SERVER_ARGS,
        )?;
        let history = launch.history.clone().filter(|_| launch.resume.is_none());
        let native_id = launch
            .resume
            .clone()
            .or_else(|| history.as_ref().map(|_| import_id(&launch.fresh_id)));
        Ok(DrivenSpec {
            launch: spec,
            transport: Transport::Stdio,
            native_id,
            model: None,
            history,
        })
    }

    fn connect(
        &self,
        io: Connection,
        spec: &DrivenSpec,
        _account: &Account,
    ) -> Result<Box<dyn DrivenSession>, AdapterError> {
        let cwd = spec.launch.cwd.to_string_lossy();
        let open = match (&spec.history, spec.native_id.as_deref()) {
            (Some(history), Some(id)) => Open::Import { id, history },
            (None, Some(id)) => Open::Resume(id),
            _ => Open::New,
        };
        Ok(Box::new(ZCodeSession::connect(io, &cwd, open)?))
    }

    /// None: the app ships no terminal UI.
    fn terminal_view(
        &self,
        _account: &Account,
        _profile: &Profile,
        _cwd: &Path,
        _of: &ViewOf,
    ) -> Result<TerminalView, AdapterError> {
        Err(AdapterError::Unsupported(NO_TERMINAL.into()))
    }
}
