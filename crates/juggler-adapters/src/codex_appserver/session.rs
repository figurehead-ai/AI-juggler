//! One driven Codex session: the codec over a connection to the session's
//! own app-server. Lines in, lines out; the codec decides what they mean.

use std::path::Path;
use std::time::Duration;

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::Connection;
use juggler_adapter::runtime::session::LineSession;

use super::codec::Codec;
use super::rpc;

/// How long the server has to answer `initialize`.
pub const INITIALIZE_WAIT: Duration = Duration::from_secs(10);
/// How long a thread may take to start or resume (a long thread loads).
pub const THREAD_WAIT: Duration = Duration::from_secs(30);

/// A driven Codex session: app-server's connection with Codex's codec.
pub type CodexSession = LineSession<Codec>;

fn same_folder(a: &Path, b: &Path) -> bool {
    let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    canonical(a) == canonical(b)
}

/// The handshake: `initialize` (refusing a server on another
/// `CODEX_HOME`), then the thread, then a first limits read.
///
/// # Errors
///
/// When the server doesn't answer in time, runs on another home, or
/// refuses the thread.
pub fn connect(
    io: Connection,
    home: &Path,
    cwd: &Path,
    resume: Option<&str>,
    model: Option<&str>,
) -> Result<CodexSession, AdapterError> {
    let mut session = LineSession::new(io, Codec::default(), "Codex's app-server");
    let initialize = session.codec.initialize();
    session.send(&initialize)?;
    session.wait_for(INITIALIZE_WAIT, "answer", |codec| {
        codec.handshake.codex_home.is_some()
    })?;
    let served = session
        .codec
        .handshake
        .codex_home
        .clone()
        .unwrap_or_default();
    if !same_folder(Path::new(&served), home) {
        return Err(AdapterError::InvalidLaunch(format!(
            "Codex's app-server runs on {served}, not this profile's folder"
        )));
    }
    session.send(&rpc::notification("initialized"))?;
    let thread = session.codec.thread(&cwd.to_string_lossy(), resume, model);
    session.send(&thread)?;
    session.wait_for(THREAD_WAIT, "start the conversation", |codec| {
        codec.handshake.thread.is_some()
    })?;
    let limits = session.codec.rate_limits();
    session.send(&limits)?;
    Ok(session)
}
