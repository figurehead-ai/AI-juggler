//! One call to the running app: connect to its socket, send the request
//! line, read the answer line. Nothing is kept between calls.
//!
//! The key goes only to a socket that looks like AI Juggler's: a socket
//! (not a link) in a folder no one else can write to, both owned by the
//! same user. The app checks the other side: the caller's user, then the
//! key.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::wire::{Call, MAX_RESPONSE, Refusal, Request, Response, SOCKET_ENV, TOKEN_ENV, VERSION};

/// How long a call may take before the bridge gives up (the app answers
/// in well under 50 ms; this is for an app that's stuck).
pub const CALL_WAIT: Duration = Duration::from_secs(5);

/// Why a call has no result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallError {
    /// The session wasn't started by AI Juggler with its agent tools.
    NoKey,
    /// AI Juggler isn't running, or its socket isn't trustworthy.
    Unreachable(String),
    /// The app refused, with why.
    Refused(Refusal, String),
    /// The answer couldn't be read.
    Garbled(String),
}

impl CallError {
    /// The words an agent is shown.
    #[must_use]
    pub fn words(&self) -> String {
        match self {
            Self::NoKey => "This session wasn't started by AI Juggler with its agent tools, so it has no key. Continue it in AI Juggler to use them.".to_owned(),
            Self::Unreachable(why) => format!("AI Juggler isn't answering: {why}. Is it running?"),
            Self::Refused(_, why) => why.clone(),
            Self::Garbled(why) => format!("AI Juggler's answer couldn't be read: {why}"),
        }
    }
}

/// Where to call and with which key: from this program's own
/// environment (AI Juggler put both there when it started the session).
#[derive(Clone, Debug)]
pub struct Target {
    /// The app's socket.
    pub socket: PathBuf,
    /// The session's key.
    pub token: String,
}

impl Target {
    /// From this process's environment.
    ///
    /// # Errors
    ///
    /// [`CallError::NoKey`] when either is missing or empty.
    pub fn from_env() -> Result<Self, CallError> {
        let token = std::env::var(TOKEN_ENV).unwrap_or_default();
        let socket = std::env::var_os(SOCKET_ENV).map(PathBuf::from);
        match socket {
            Some(socket) if !token.is_empty() && !socket.as_os_str().is_empty() => {
                Ok(Self { socket, token })
            }
            _ => Err(CallError::NoKey),
        }
    }
}

/// Whether `socket` may be given the key: a socket (`lstat`, a link is
/// never followed) in a folder only its owner can write, both owned by
/// the same user.
#[cfg(unix)]
#[must_use]
pub fn trusted_socket(socket: &Path) -> bool {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let (Ok(file), Some(Ok(dir))) = (
        std::fs::symlink_metadata(socket),
        socket.parent().map(std::fs::symlink_metadata),
    ) else {
        return false;
    };
    file.file_type().is_socket()
        && dir.is_dir()
        && dir.mode() & 0o022 == 0
        && dir.uid() == file.uid()
}

/// Elsewhere there's no socket to trust.
#[cfg(not(unix))]
#[must_use]
pub fn trusted_socket(_socket: &Path) -> bool {
    false
}

/// Makes one call.
///
/// # Errors
///
/// [`CallError`] when the app can't be reached, refuses, or answers
/// something unreadable.
pub fn call(
    target: &Target,
    call: Call,
    text: Option<String>,
) -> Result<serde_json::Value, CallError> {
    let request = Request {
        v: VERSION,
        token: target.token.clone(),
        call,
        text,
    };
    let line =
        serde_json::to_string(&request).map_err(|error| CallError::Garbled(error.to_string()))?;
    let answer = exchange(&target.socket, &line, CALL_WAIT)?;
    let response: Response = serde_json::from_str(answer.trim_end())
        .map_err(|error| CallError::Garbled(error.to_string()))?;
    match (response.result, response.refused) {
        (Some(result), None) => Ok(result),
        (_, Some(refusal)) => Err(CallError::Refused(
            refusal,
            response.reason.unwrap_or_else(|| "refused".to_owned()),
        )),
        (None, None) => Err(CallError::Garbled("an empty answer".to_owned())),
    }
}

/// Sends one line to the app and reads its answer, waiting at most `wait`
/// for each (a tool's hook must never hold its tool up for long).
///
/// # Errors
///
/// As [`call`].
pub fn send_line(socket: &Path, line: &str, wait: Duration) -> Result<String, CallError> {
    exchange(socket, line, wait)
}

#[cfg(unix)]
fn exchange(socket: &Path, line: &str, wait: Duration) -> Result<String, CallError> {
    use std::os::unix::net::UnixStream;
    if !trusted_socket(socket) {
        return Err(CallError::Unreachable(format!(
            "{} isn't its socket",
            socket.display()
        )));
    }
    let unreachable = |error: std::io::Error| CallError::Unreachable(error.to_string());
    let mut stream = UnixStream::connect(socket).map_err(unreachable)?;
    stream.set_read_timeout(Some(wait)).map_err(unreachable)?;
    stream.set_write_timeout(Some(wait)).map_err(unreachable)?;
    stream.write_all(line.as_bytes()).map_err(unreachable)?;
    stream.write_all(b"\n").map_err(unreachable)?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(unreachable)?;
    let mut answer = Vec::new();
    stream
        .take(u64::try_from(MAX_RESPONSE).unwrap_or(u64::MAX) + 1)
        .read_to_end(&mut answer)
        .map_err(unreachable)?;
    if answer.len() > MAX_RESPONSE {
        return Err(CallError::Garbled("the answer is too long".to_owned()));
    }
    String::from_utf8(answer).map_err(|error| CallError::Garbled(error.to_string()))
}

#[cfg(not(unix))]
fn exchange(_socket: &Path, _line: &str, _wait: Duration) -> Result<String, CallError> {
    Err(CallError::Unreachable(
        "the agent tools need macOS".to_owned(),
    ))
}

#[cfg(all(test, unix))]
mod tests {
    use super::trusted_socket;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn the_key_goes_only_to_a_socket_in_a_closed_folder() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("d");
        std::fs::create_dir(&dir).unwrap();
        let socket = dir.join("s.sock");
        let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(trusted_socket(&socket));
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o777)).unwrap();
        assert!(!trusted_socket(&socket), "a folder anyone can write");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let file = dir.join("plain");
        std::fs::write(&file, "x").unwrap();
        assert!(!trusted_socket(&file), "not a socket");
        let link = dir.join("link.sock");
        std::os::unix::fs::symlink(&socket, &link).unwrap();
        assert!(!trusted_socket(&link), "a link is never followed");
        assert!(!trusted_socket(&dir.join("missing.sock")));
    }
}
