//! A session's output: what AI Juggler keeps of it and its live stream
//! (the terminal's view reads both, SK-23).

use juggler_core::model::SessionId;

use crate::error::SupervisorError;
use crate::supervisor::Supervisor;

/// What a new listener gets: the kept output (when asked for), the count of
/// bytes written before its first chunk, and its stream.
type Listening = (Vec<u8>, u64, std::sync::mpsc::Receiver<Vec<u8>>);

impl Supervisor {
    /// Snapshot of the retained output tail for a session.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::UnknownSession`] when the id is not
    /// supervised.
    pub fn output(&self, session: &SessionId) -> Result<Vec<u8>, SupervisorError> {
        self.output_at(session).map(|(bytes, _)| bytes)
    }

    /// The retained output tail and where it ends: the count of bytes the
    /// session has written in all. The tail is the bytes just before that
    /// place, so a reader can join it to a [`Supervisor::tap_at`] stream
    /// without a gap or a byte twice (SK-23).
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::UnknownSession`] when the id is not
    /// supervised.
    pub fn output_at(&self, session: &SessionId) -> Result<(Vec<u8>, u64), SupervisorError> {
        let handle = self.child(session)?;
        handle
            .ring
            .lock()
            .map(|ring| (ring.snapshot(), ring.total_written()))
            .map_err(|_| SupervisorError::UnknownSession(session.clone()))
    }

    /// The last `max_bytes` of captured output (for prompt and
    /// rate-limit signature detection; never the whole ring).
    #[must_use]
    pub fn output_tail(&self, session: &SessionId, max_bytes: usize) -> Option<String> {
        let output = self.output(session).ok()?;
        let start = output.len().saturating_sub(max_bytes);
        Some(String::from_utf8_lossy(&output[start..]).into_owned())
    }

    /// Subscribes to this session's live output stream (each chunk as
    /// written). Snapshot-then-tap is race-free: the ring forwards every
    /// byte to listeners, so a snapshot followed by a tap yields the
    /// stream exactly once from the snapshot point onward.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::UnknownSession`] when the id is not
    /// supervised.
    pub fn tap(
        &self,
        session: &SessionId,
    ) -> Result<std::sync::mpsc::Receiver<Vec<u8>>, SupervisorError> {
        self.tap_at(session).map(|(_, rx)| rx)
    }

    /// [`Supervisor::tap`], with where in the session's output it starts:
    /// the count of bytes written before its first chunk. Each chunk then
    /// follows the one before, so a chunk's place is known by adding up
    /// (SK-23: the terminal's view joins it to [`Supervisor::output_at`]).
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::UnknownSession`] when the id is not
    /// supervised.
    pub fn tap_at(
        &self,
        session: &SessionId,
    ) -> Result<(u64, std::sync::mpsc::Receiver<Vec<u8>>), SupervisorError> {
        self.listen(session, false).map(|(_, at, rx)| (at, rx))
    }

    /// The retained output AND a live tap, taken under one ring lock so
    /// no chunk falls between them or arrives twice. Use this when the
    /// reader needs everything: a fast process can write (and even
    /// finish) before a plain [`Supervisor::tap`] is attached.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::UnknownSession`] when the id is not
    /// supervised.
    pub fn tap_with_backlog(
        &self,
        session: &SessionId,
    ) -> Result<(Vec<u8>, std::sync::mpsc::Receiver<Vec<u8>>), SupervisorError> {
        self.listen(session, true)
            .map(|(backlog, _, rx)| (backlog, rx))
    }

    /// A new listener on the session's output, under one ring lock: the
    /// kept output first when `backlog` asks for it (else empty), and the
    /// count of bytes written before the listener's first chunk.
    fn listen(&self, session: &SessionId, backlog: bool) -> Result<Listening, SupervisorError> {
        let handle = self.child(session)?;
        let mut ring = handle
            .ring
            .lock()
            .map_err(|_| SupervisorError::UnknownSession(session.clone()))?;
        let kept = if backlog { ring.snapshot() } else { Vec::new() };
        let (tx, rx) = std::sync::mpsc::channel();
        ring.add_listener(tx);
        Ok((kept, ring.total_written(), rx))
    }
}
