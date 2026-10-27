//! A second launch asks the running AI Juggler to come forward, and waits
//! for its answer (0.0.11 walk, Q8).
//!
//! The single-instance plugin's hand-over is one-way: a frozen app's
//! socket still accepts the message, so a second launch quit silently and
//! nothing came forward. Here the running app answers only once its main
//! thread has brought the window forward. A second launch that gets no
//! answer within [`ANSWER_WAIT`] says the open one isn't answering.
//!
//! Local IPC rules (SECURITY-GUIDELINES C7), kept by
//! [`juggler_platform::local_ipc`]:
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
//! plugin's hand-over, as before. So does every launch on a system with
//! no local connections yet (Windows): there [`Handover::for_data_dir`]
//! is `None`.

use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use juggler_platform::local_ipc::{self, BindError};
use juggler_platform::{file_trust, user::UserId};

/// How long a second launch waits for the running app's answer.
pub const ANSWER_WAIT: Duration = Duration::from_secs(3);
/// How long the running app gives its main thread to bring the window
/// forward; a stuck main thread never answers.
pub const RAISE_WAIT: Duration = Duration::from_millis(2_500);

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

impl From<BindError> for HandoverError {
    fn from(error: BindError) -> Self {
        match error {
            BindError::NotOurs(path) => Self::NotOurs(path),
            BindError::Io(path, source) => Self::Io { path, source },
        }
    }
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
    local_ipc::fits(&path).then_some(path)
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
    me: UserId,
}

impl Handover {
    /// For this edition's data folder. The data folder is ours (it was
    /// just created, or found, in our home), so its owner is the user
    /// both ends must be. `None` when the path won't fit, the folder
    /// can't be read, or this system has no local connections yet; the
    /// plugin's hand-over then works as before.
    #[must_use]
    pub fn for_data_dir(identifier: &str, data_dir: &Path) -> Option<Self> {
        if !local_ipc::SUPPORTED {
            return None;
        }
        Some(Self::at(
            socket_path(identifier, data_dir)?,
            file_trust::owner_of(data_dir)?,
        ))
    }

    /// At an explicit socket, for user `me` (tests).
    #[must_use]
    pub fn at(socket: PathBuf, me: UserId) -> Self {
        Self { socket, me }
    }

    /// The socket's path.
    #[must_use]
    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// A second launch: asks the running app to come forward and waits up
    /// to `wait` for its answer. Only a socket of ours, in a directory of
    /// ours, is asked.
    #[must_use]
    pub fn ask(&self, wait: Duration) -> Answer {
        let Some(dir) = self.socket.parent() else {
            return Answer::Untrusted;
        };
        match std::fs::symlink_metadata(&self.socket) {
            Err(error) if error.kind() == ErrorKind::NotFound => return Answer::NoneListening,
            Err(_) => return Answer::Untrusted,
            Ok(_)
                if !file_trust::dir_is_ours(dir, self.me)
                    || !local_ipc::endpoint_is_ours(&self.socket, self.me) =>
            {
                return Answer::Untrusted;
            }
            Ok(_) => {}
        }
        let mut connection = match local_ipc::connect(&self.socket) {
            Ok(connection) => connection,
            // Our file, nobody listening: a crashed run's.
            Err(error) if error.kind() == ErrorKind::ConnectionRefused => {
                return Answer::NoneListening;
            }
            Err(_) => return Answer::NotAnswering,
        };
        if connection.set_read_timeout(Some(wait)).is_err() || connection.write_all(ASK).is_err() {
            return Answer::NotAnswering;
        }
        let mut reply = [0_u8; 3];
        match connection.read_exact(&mut reply) {
            Ok(()) if &reply == ANSWER => Answer::Answered,
            _ => Answer::NotAnswering,
        }
    }

    /// The running app: answers hand-overs until it quits. `raise` brings
    /// the window forward and says whether it did (within [`RAISE_WAIT`]);
    /// only then is the answer sent. The socket's directory is made `0700`
    /// if missing; if present it must already be ours and closed (never
    /// chmod'ed into shape). A stale socket of ours (a crashed run) goes;
    /// anything else at the path is left alone and refused.
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
        let bound = local_ipc::bind(&self.socket, self.me)?;
        let raise = Arc::new(raise);
        tauri::async_runtime::spawn(async move {
            let Ok(listener) = bound.into_async() else {
                return;
            };
            loop {
                // Anyone but this user is dropped unanswered (`None`).
                let Ok(Some(stream)) = listener.accept().await else {
                    continue;
                };
                tauri::async_runtime::spawn(answer(stream, Arc::clone(&raise)));
            }
        });
        Ok(())
    }

    /// The running app is quitting: its socket goes with it (only ours).
    pub fn stop(&self) {
        local_ipc::remove_if_ours(&self.socket, self.me);
    }
}

/// One connection (already known to be this user's): only the one ask,
/// and the answer only once the window came forward.
async fn answer(
    mut stream: local_ipc::Stream,
    raise: Arc<impl Fn() -> bool + Send + Sync + 'static>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
