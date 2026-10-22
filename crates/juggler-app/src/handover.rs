//! A second launch asks the running AI Juggler to come forward, and waits
//! for its answer (0.0.11 walk, Q8).
//!
//! The single-instance plugin's hand-over is one-way: a frozen app's
//! socket still accepts the message, so a second launch quit silently and
//! nothing came forward. Here the running app answers only once its main
//! thread has brought the window forward. A second launch that gets no
//! answer within [`ANSWER_WAIT`] says the open one isn't answering.
//!
//! Local IPC rules (SECURITY-GUIDELINES C7):
//! - the socket sits in a `0700` directory of ours under the per-user
//!   temporary folder (`$TMPDIR`, `/var/folders/…` on macOS);
//! - it is created `0600`;
//! - every connection's peer uid is checked (`getpeereid`, through tokio's
//!   `peer_cred`) and anything that isn't this user is dropped unanswered;
//! - a stale socket is removed only when `lstat` shows it is a socket that
//!   we own, so a symlink is never followed;
//! - a second launch trusts an answer only from a socket file, in a
//!   directory, that we own.
//!
//! The path is per edition and data folder (a hash of both), so builds
//! with different home folders never hand over to each other. An older
//! AI Juggler has no such socket: a second launch then falls back to the
//! plugin's hand-over, as before.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long a second launch waits for the running app's answer.
pub const ANSWER_WAIT: Duration = Duration::from_secs(3);
/// How long the running app gives its main thread to bring the window
/// forward; a stuck main thread never answers.
pub const RAISE_WAIT: Duration = Duration::from_millis(2_500);
/// A Unix socket path must fit `sun_path` (104 bytes on macOS, with its
/// terminating NUL); stay well inside it.
pub const MAX_SOCKET_PATH: usize = 100;

const ASK: &[u8; 6] = b"raise\n";
const ANSWER: &[u8; 3] = b"ok\n";
const SOCKET_NAME: &str = "ho.sock";

/// What the running app said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// It came forward.
    Answered,
    /// Our socket is there but didn't answer in time.
    NotAnswering,
    /// No hand-over socket (an older build, or one still starting).
    NoneListening,
    /// Something is at the path that isn't ours (another user's socket,
    /// a symlink, a directory anyone can write to): never trusted.
    Untrusted,
}

/// Why the running app couldn't serve hand-overs. It then runs without
/// them, and a second launch falls back to the plugin's hand-over.
#[derive(Debug, thiserror::Error)]
pub enum HandoverError {
    /// The socket's directory or path is someone else's, or not a socket.
    #[error("{0} isn't ours")]
    NotOurs(PathBuf),
    /// The socket couldn't be created.
    #[error("{path}: {source}")]
    Io {
        /// Where.
        path: PathBuf,
        /// What happened.
        #[source]
        source: std::io::Error,
    },
}

/// Pure: the hand-over socket for this edition and data folder under
/// `temp` (the per-user temporary folder), or `None` when the path would
/// not fit a socket address.
#[must_use]
pub fn socket_path_in(temp: &Path, identifier: &str, data_dir: &Path) -> Option<PathBuf> {
    // FNV-1a over both: stable across runs and builds (std's hasher is
    // seeded), and short.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let bytes = identifier
        .as_bytes()
        .iter()
        .chain(b"\0")
        .chain(data_dir.as_os_str().as_encoded_bytes());
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    let path = temp.join(format!("aij-{hash:016x}")).join(SOCKET_NAME);
    (path.as_os_str().len() < MAX_SOCKET_PATH).then_some(path)
}

/// The hand-over socket for this edition and data folder, in the per-user
/// temporary folder.
#[must_use]
pub fn socket_path(identifier: &str, data_dir: &Path) -> Option<PathBuf> {
    socket_path_in(&std::env::temp_dir(), identifier, data_dir)
}

/// The hand-over between one launch and the running app, for one socket
/// and the user both sides must be.
#[derive(Clone, Debug)]
pub struct Handover {
    socket: PathBuf,
    me: u32,
}

impl Handover {
    /// For this edition's data folder. The data folder is ours (it was
    /// just created, or found, in our home), so its owner is the user
    /// both ends must be. `None` when the path won't fit or the folder
    /// can't be read; the plugin's hand-over then works as before.
    #[must_use]
    pub fn for_data_dir(identifier: &str, data_dir: &Path) -> Option<Self> {
        Some(Self::at(
            socket_path(identifier, data_dir)?,
            imp::owner_of(data_dir)?,
        ))
    }

    /// At an explicit socket, for user `me` (tests).
    #[must_use]
    pub fn at(socket: PathBuf, me: u32) -> Self {
        Self { socket, me }
    }

    /// The socket's path.
    #[must_use]
    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// A second launch: asks the running app to come forward and waits up
    /// to `wait` for its answer. Only a socket of ours is asked.
    #[must_use]
    pub fn ask(&self, wait: Duration) -> Answer {
        imp::ask(&self.socket, self.me, wait)
    }

    /// The running app: answers hand-overs until it quits. `raise` brings
    /// the window forward and says whether it did (within [`RAISE_WAIT`]);
    /// only then is the answer sent.
    ///
    /// # Errors
    ///
    /// [`HandoverError::NotOurs`] when the directory or a file at the
    /// socket's path belongs to someone else or isn't a socket (nothing is
    /// removed); [`HandoverError::Io`] when the socket can't be made.
    pub fn serve(
        &self,
        raise: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Result<(), HandoverError> {
        imp::serve(&self.socket, self.me, raise)
    }

    /// The running app is quitting: its socket goes with it (only ours).
    pub fn stop(&self) {
        imp::remove_if_ours(&self.socket, self.me);
    }
}

/// Pure: whether a connection from `peer` may be answered.
#[must_use]
pub fn peer_allowed(peer: Option<u32>, me: u32) -> bool {
    peer == Some(me)
}

#[cfg(unix)]
mod imp {
    use super::{ANSWER, ASK, Answer, HandoverError, RAISE_WAIT, peer_allowed};
    use std::io::{ErrorKind, Read, Write};
    use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
    use std::os::unix::net::UnixStream;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;

    // The ownership checks are shared (juggler_adapter::file_trust).
    use juggler_adapter::file_trust::dir_is_ours;
    pub use juggler_adapter::file_trust::owner_of;

    /// The path is a socket we own (`lstat`: a link is never followed).
    fn socket_is_ours(socket: &Path, me: u32) -> bool {
        std::fs::symlink_metadata(socket)
            .is_ok_and(|meta| meta.file_type().is_socket() && meta.uid() == me)
    }

    pub fn remove_if_ours(socket: &Path, me: u32) {
        if socket_is_ours(socket, me) {
            let _ = std::fs::remove_file(socket);
        }
    }

    pub fn ask(socket: &Path, me: u32, wait: Duration) -> Answer {
        let Some(dir) = socket.parent() else {
            return Answer::Untrusted;
        };
        match std::fs::symlink_metadata(socket) {
            Err(error) if error.kind() == ErrorKind::NotFound => return Answer::NoneListening,
            Err(_) => return Answer::Untrusted,
            Ok(_) if !dir_is_ours(dir, me) || !socket_is_ours(socket, me) => {
                return Answer::Untrusted;
            }
            Ok(_) => {}
        }
        let mut stream = match UnixStream::connect(socket) {
            Ok(stream) => stream,
            // Our file, nobody listening: a crashed run's.
            Err(error) if error.kind() == ErrorKind::ConnectionRefused => {
                return Answer::NoneListening;
            }
            Err(_) => return Answer::NotAnswering,
        };
        if stream.set_read_timeout(Some(wait)).is_err() || stream.write_all(ASK).is_err() {
            return Answer::NotAnswering;
        }
        let mut reply = [0_u8; 3];
        match stream.read_exact(&mut reply) {
            Ok(()) if &reply == ANSWER => Answer::Answered,
            _ => Answer::NotAnswering,
        }
    }

    fn io(path: &Path) -> impl FnOnce(std::io::Error) -> HandoverError + '_ {
        move |source| HandoverError::Io {
            path: path.to_path_buf(),
            source,
        }
    }

    /// The socket's directory: made `0700` if missing; if present it must
    /// already be ours and closed (never chmod'ed into shape: a directory
    /// someone else made is refused).
    fn prepare_dir(dir: &Path, me: u32) -> Result<(), HandoverError> {
        match std::fs::DirBuilder::new().mode(0o700).create(dir) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => return Err(io(dir)(error)),
        }
        if dir_is_ours(dir, me) {
            Ok(())
        } else {
            Err(HandoverError::NotOurs(dir.to_path_buf()))
        }
    }

    pub fn serve(
        socket: &Path,
        me: u32,
        raise: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Result<(), HandoverError> {
        let dir = socket
            .parent()
            .ok_or_else(|| HandoverError::NotOurs(socket.to_path_buf()))?;
        prepare_dir(dir, me)?;
        // A stale socket of ours (a crashed run) goes; anything else at the
        // path — a link, a file, someone else's socket — is left alone and
        // refused.
        match std::fs::symlink_metadata(socket) {
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(io(socket)(error)),
            Ok(_) if socket_is_ours(socket, me) => {
                std::fs::remove_file(socket).map_err(io(socket))?;
            }
            Ok(_) => return Err(HandoverError::NotOurs(socket.to_path_buf())),
        }
        let listener = std::os::unix::net::UnixListener::bind(socket).map_err(io(socket))?;
        std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))
            .map_err(io(socket))?;
        listener.set_nonblocking(true).map_err(io(socket))?;
        let raise = Arc::new(raise);
        tauri::async_runtime::spawn(async move {
            let Ok(listener) = tokio::net::UnixListener::from_std(listener) else {
                return;
            };
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };
                let raise = Arc::clone(&raise);
                tauri::async_runtime::spawn(answer(stream, me, raise));
            }
        });
        Ok(())
    }

    /// One connection: only this user, only the one ask, and the answer
    /// only once the window came forward.
    async fn answer(
        mut stream: tokio::net::UnixStream,
        me: u32,
        raise: Arc<impl Fn() -> bool + Send + Sync + 'static>,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        if !peer_allowed(stream.peer_cred().ok().map(|cred| cred.uid()), me) {
            return;
        }
        let mut ask = [0_u8; 6];
        // A caller that connects and says nothing can't hold a task long.
        let read = tokio::time::timeout(Duration::from_secs(1), stream.read_exact(&mut ask)).await;
        if !matches!(read, Ok(Ok(_))) || &ask != ASK {
            return;
        }
        let raised = tokio::time::timeout(
            RAISE_WAIT,
            tauri::async_runtime::spawn_blocking(move || raise()),
        )
        .await;
        if matches!(raised, Ok(Ok(true))) {
            let _ = stream.write_all(ANSWER).await;
        }
    }
}

#[cfg(not(unix))]
mod imp {
    use super::{Answer, HandoverError};
    use std::path::Path;
    use std::time::Duration;

    // Elsewhere the plugin's hand-over stays (a named pipe on Windows).
    pub fn owner_of(_path: &Path) -> Option<u32> {
        None
    }

    pub fn ask(_socket: &Path, _me: u32, _wait: Duration) -> Answer {
        Answer::NoneListening
    }

    pub fn serve(
        _socket: &Path,
        _me: u32,
        _raise: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Result<(), HandoverError> {
        Ok(())
    }

    pub fn remove_if_ours(_socket: &Path, _me: u32) {}
}
