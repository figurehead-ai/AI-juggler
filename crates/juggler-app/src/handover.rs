//! A second launch asks the running AI Juggler to come forward, and waits
//! for its answer (0.0.11 walk, Q8).
//!
//! The single-instance plugin's hand-over is one-way: a frozen app's
//! socket still accepts the message, so a second launch quit silently and
//! nothing came forward. Here the running app answers only once its main
//! thread has brought the window forward; a second launch that gets no
//! answer within [`ANSWER_WAIT`] says the open one isn't answering.
//!
//! The socket is per data folder (a hash of its path), so builds of one
//! edition with different home folders never hand over to each other. An
//! older AI Juggler has no such socket: a second launch then falls back to
//! the plugin's hand-over, as before.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long a second launch waits for the running app's answer.
pub const ANSWER_WAIT: Duration = Duration::from_secs(3);
/// How long the running app gives its main thread to bring the window
/// forward; a stuck main thread never answers.
pub const RAISE_WAIT: Duration = Duration::from_millis(2_500);

const ASK: &[u8; 6] = b"raise\n";
const ANSWER: &[u8; 3] = b"ok\n";

/// What the running app said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// It came forward.
    Answered,
    /// Something holds the socket but didn't answer in time.
    NotAnswering,
    /// No hand-over socket here (an older build, or one still starting).
    NoneListening,
}

/// Pure: the hand-over socket for this edition and data folder. Short (a
/// Unix socket path must stay under ~100 bytes), in `/tmp` like the
/// plugin's, and different for every data folder.
#[must_use]
pub fn socket_path(identifier: &str, data_dir: &Path) -> PathBuf {
    // FNV-1a: stable across runs and builds (std's hasher is seeded).
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data_dir.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    let id: String = identifier
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(40)
        .collect();
    PathBuf::from(format!("/tmp/{id}_{hash:016x}_ho.sock"))
}

/// A second launch: asks the app holding `socket` to come forward and
/// waits up to `wait` for its answer.
#[must_use]
pub fn ask(socket: &Path, wait: Duration) -> Answer {
    imp::ask(socket, wait)
}

/// The running app: answers hand-overs on `socket` until it quits.
/// `raise` brings the window forward and says whether it did (within
/// [`RAISE_WAIT`]); only then is the answer sent.
pub fn serve(socket: &Path, raise: impl Fn() -> bool + Send + 'static) {
    imp::serve(socket, raise);
}

/// The running app is quitting: its socket goes with it.
pub fn stop(socket: &Path) {
    let _ = std::fs::remove_file(socket);
}

#[cfg(unix)]
mod imp {
    use super::{ANSWER, ASK, Answer};
    use std::io::{ErrorKind, Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::Path;
    use std::time::Duration;

    pub fn ask(socket: &Path, wait: Duration) -> Answer {
        let mut stream = match UnixStream::connect(socket) {
            Ok(stream) => stream,
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::NotFound | ErrorKind::ConnectionRefused
                ) =>
            {
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

    pub fn serve(socket: &Path, raise: impl Fn() -> bool + Send + 'static) {
        // Only the holder of the instance lock serves, so a file left here
        // is a crashed run's.
        let _ = std::fs::remove_file(socket);
        let listener = match UnixListener::bind(socket) {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!(
                    "ai-juggler: hand-over socket {} not bound: {error}",
                    socket.display()
                );
                return;
            }
        };
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                // A caller that connects and says nothing can't hold the
                // line for others.
                let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                let mut ask = [0_u8; 6];
                if stream.read_exact(&mut ask).is_ok() && &ask == ASK && raise() {
                    let _ = stream.write_all(ANSWER);
                }
            }
        });
    }
}

#[cfg(not(unix))]
mod imp {
    use super::Answer;
    use std::path::Path;
    use std::time::Duration;

    // Elsewhere the plugin's hand-over stays (a named pipe on Windows).
    pub fn ask(_socket: &Path, _wait: Duration) -> Answer {
        Answer::NoneListening
    }

    pub fn serve(_socket: &Path, _raise: impl Fn() -> bool + Send + 'static) {}
}
