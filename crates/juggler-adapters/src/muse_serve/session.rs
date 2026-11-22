//! One driven Muse session: the codec over a connection to the session's
//! own `muse serve`. Lines in, lines out; the codec decides what they mean.

use std::path::Path;
use std::time::Duration;

use juggler_adapter::{
    AdapterError,
    runtime::{Connection, session::LineSession},
};

use super::codec::{self, Codec};

/// How long Muse has to answer `initialize` (it loads its settings,
/// plugins and model catalog first).
pub const INITIALIZE_WAIT: Duration = Duration::from_secs(30);
/// How long a session may take to start or resume (a long log folds).
pub const OPEN_WAIT: Duration = Duration::from_secs(60);

/// A driven Muse session: `muse serve`'s connection with Muse's codec.
pub type MuseSession = LineSession<Codec>;

/// How a session opens.
#[derive(Clone, Debug)]
pub struct Open<'a> {
    /// Muse's data folder this account must run in (`museHome`).
    pub muse_home: &'a Path,
    /// The folder a new session works in.
    pub cwd: &'a str,
    /// A stored session to resume, by id.
    pub resume: Option<&'a str>,
    /// The model a new session starts with.
    pub model: Option<&'a str>,
}

/// The handshake: `initialize` (refusing a Muse that keeps its sessions
/// anywhere but this account's data folder: the isolation didn't take),
/// `initialized`, then `session/start` or `session/resume`, which
/// subscribe this connection to the session's events.
///
/// # Errors
///
/// When Muse doesn't answer in time, runs on another home, or refuses the
/// session.
pub fn connect(io: Connection, open: &Open<'_>) -> Result<MuseSession, AdapterError> {
    let mut session = LineSession::new(io, Codec::default(), "Muse's session server");
    session.codec.model = open.model.map(str::to_owned);
    let initialize = session.codec.initialize();
    session.send(&initialize)?;
    session.wait_for(INITIALIZE_WAIT, "answer", |codec| {
        codec.handshake.muse_home.is_some()
    })?;
    let served = session
        .codec
        .handshake
        .muse_home
        .clone()
        .unwrap_or_default();
    if !crate::common::rpc_codec::same_folder(Path::new(&served), open.muse_home) {
        return Err(AdapterError::InvalidLaunch(format!(
            "Muse keeps this run's sessions in {served}, not this account's folder: the account isn't used"
        )));
    }
    session.send(&codec::initialized())?;
    let first = match open.resume {
        Some(id) => session.codec.resume(id),
        None => session.codec.start(open.cwd),
    };
    session.send(&first)?;
    session.wait_for(OPEN_WAIT, "open the session", |codec| {
        codec.handshake.session.is_some()
    })?;
    Ok(session)
}
