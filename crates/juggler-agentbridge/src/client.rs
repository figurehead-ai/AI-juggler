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

use crate::wire::{Args, Call, MAX_RESPONSE, Refusal, Request, Response, SOCKET_ENV, TOKEN_ENV};

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
            Self::NoKey => "This session wasn't started by AI Juggler with its agent tools, so it has no key. Continue it in AI Juggler to use them, or add AI Juggler's connector to its app.".to_owned(),
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

    /// AJ-E3013 (ADR-0020's D-116 addendum): as the connector in a
    /// desktop app's session runs it. With a key (a session AI Juggler
    /// started), as [`Self::from_env`]; with none but the socket AI
    /// Juggler's connector named, a keyless target, which the app attests
    /// from the system on every call or refuses.
    ///
    /// # Errors
    ///
    /// [`CallError::NoKey`] when there's no socket either.
    pub fn for_connector() -> Result<Self, CallError> {
        if let Ok(keyed) = Self::from_env() {
            return Ok(keyed);
        }
        connector_socket()
            .map(|socket| Self {
                socket,
                token: String::new(),
            })
            .ok_or(CallError::NoKey)
    }
}

/// AJ-E3013: the socket AI Juggler's connector calls: the one it named
/// ([`SOCKET_ENV`]), else the one in the edition's data folder the
/// plugin's launcher found its program in ([`DATA_ENV`]: `<data>/agent/`,
/// as every edition keeps it). The key never comes this way: only the
/// app's attestation.
#[must_use]
pub fn connector_socket() -> Option<PathBuf> {
    let named = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    if let Some(socket) = named(SOCKET_ENV) {
        return Some(PathBuf::from(socket));
    }
    named(DATA_ENV).map(|data| PathBuf::from(data).join("agent").join("bridge.sock"))
}

/// The edition's data folder, as the published plugin's launcher names it
/// for the program it found (a path, not a secret).
pub const DATA_ENV: &str = "JUGGLER_AGENT_DATA";

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
pub fn call(target: &Target, call: Call, args: Args) -> Result<serde_json::Value, CallError> {
    let request = Request::new(target.token.clone(), call, args);
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
    let mut text = line.to_owned();
    text.push('\n');
    let answer = talk(socket, &text, wait, true)?;
    String::from_utf8(answer).map_err(|error| CallError::Garbled(error.to_string()))
}

#[cfg(not(unix))]
fn exchange(socket: &Path, line: &str, wait: Duration) -> Result<String, CallError> {
    talk(socket, line, wait, true).map(|_| String::new())
}

/// D-119: writes `text` to `socket` (a socket of this user's, never a
/// link) and reads nothing back: a post to a session's own inbox.
///
/// # Errors
///
/// When the socket isn't trustworthy or the write fails.
pub fn post_lines(socket: &Path, text: &str, wait: Duration) -> Result<(), CallError> {
    talk(socket, text, wait, false).map(|_| ())
}

/// The one connection: to a trusted socket only, `text` written, the
/// writing side closed, and the answer read when `answer` says so.
#[cfg(unix)]
fn talk(socket: &Path, text: &str, wait: Duration, answer: bool) -> Result<Vec<u8>, CallError> {
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
    stream.write_all(text.as_bytes()).map_err(unreachable)?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(unreachable)?;
    let mut read = Vec::new();
    if answer {
        stream
            .take(u64::try_from(MAX_RESPONSE).unwrap_or(u64::MAX) + 1)
            .read_to_end(&mut read)
            .map_err(unreachable)?;
        if read.len() > MAX_RESPONSE {
            return Err(CallError::Garbled("the answer is too long".to_owned()));
        }
    }
    Ok(read)
}

#[cfg(not(unix))]
fn talk(_socket: &Path, _text: &str, _wait: Duration, _answer: bool) -> Result<Vec<u8>, CallError> {
    Err(CallError::Unreachable(
        "the agent tools aren't available on this system yet".to_owned(),
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
