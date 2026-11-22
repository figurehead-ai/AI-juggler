//! One driven `ZCode` session: the codec over a connection to the
//! session's own app-server. Lines in, lines out; the codec decides what
//! they mean.

use std::time::Duration;

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::session::LineSession;
use juggler_adapter::runtime::{Connection, ImportedHistory};

use super::codec::Codec;

/// How long the engine may take to open (or import) a session: it starts
/// its store and its provider settings first.
pub const OPEN_WAIT: Duration = Duration::from_secs(60);

/// How a session opens.
#[derive(Clone, Copy, Debug)]
pub enum Open<'a> {
    /// A new session.
    New,
    /// A stored one, by id.
    Resume(&'a str),
    /// A new one, `id`, with a conversation imported into it.
    Import {
        /// Its id (`sess_<uuid>`).
        id: &'a str,
        /// The conversation.
        history: &'a ImportedHistory,
    },
}

/// A driven `ZCode` session: the engine's connection with `ZCode`'s codec.
pub type ZCodeSession = LineSession<Codec>;

/// Opens the session (`session/create` or `session/resume`) in `cwd`,
/// then subscribes to its events. No `initialize`: the engine has none.
///
/// # Errors
///
/// When the engine doesn't answer in time or refuses.
pub fn connect(io: Connection, cwd: &str, open: Open<'_>) -> Result<ZCodeSession, AdapterError> {
    let mut session = LineSession::new(io, Codec::default(), "ZCode's app server");
    let first = match open {
        Open::New => session.codec.create(cwd, None, None),
        Open::Resume(id) => session.codec.resume(id, cwd),
        Open::Import { id, history } => session.codec.create(cwd, Some(id), Some(history)),
    };
    session.send(&first)?;
    session.wait_for(OPEN_WAIT, "open the session", |codec| {
        codec.handshake.session.is_some()
    })?;
    let subscribe = session.codec.subscribe();
    session.send(&subscribe)?;
    session.wait_for(OPEN_WAIT, "send the session's events", |codec| {
        codec.handshake.subscribed
    })?;
    Ok(session)
}
