//! ADR-0033 §4 (AJ-E3802): the one client for AI Juggler's model helpers.
//! A helper speaks newline-delimited JSON over stdin and stdout, one
//! request at a time; its first line is its `hello`. The client starts
//! it with only what it needs (`juggler_platform::process::helper`),
//! bounds every line, answers within a deadline or stops it, and stops it
//! when dropped. A helper exits by itself when its stdin closes, so AI
//! Juggler's death never leaves one behind.

use std::io::{BufRead, BufReader, Read as _, Write as _};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use crate::features::ai::engine::{EngineError, Stop};

/// The largest request line (ADR-0033 §4: 32 KiB), the same bound for
/// every helper; the local helper's protocol is its source.
pub const MAX_REQUEST_BYTES: usize = juggler_local_ai_protocol::MAX_REQUEST_BYTES;

/// The largest answer line a helper may write.
pub const MAX_LINE_BYTES: u64 = juggler_local_ai_protocol::MAX_ANSWER_BYTES as u64;

/// How often a wait looks at the job's stop.
const STOP_POLL: Duration = Duration::from_millis(50);

/// A running helper.
#[derive(Debug)]
pub struct Helper {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<Option<String>>,
    /// Its first line.
    pub hello: serde_json::Value,
}

impl Helper {
    /// Starts `program` and reads its `hello` within `wait`.
    ///
    /// # Errors
    ///
    /// It can't start, says nothing in time, or its hello isn't JSON:
    /// [`EngineError::Crashed`] (or [`EngineError::TimedOut`]).
    pub fn start(program: &Path, wait: Duration) -> Result<Self, EngineError> {
        Self::launch(juggler_platform::process::helper(program), wait)
    }

    /// Starts `command` (one of AI Juggler's own helpers, made with
    /// [`juggler_platform::process::helper`]: no shell, only the allowed
    /// environment) and reads its `hello` within `wait`.
    ///
    /// # Errors
    ///
    /// As [`Helper::start`].
    pub fn launch(mut command: Command, wait: Duration) -> Result<Self, EngineError> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| EngineError::Crashed)?;
        let stdin = child.stdin.take();
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(EngineError::Crashed);
        };
        let (send, lines) = channel();
        let reader = std::thread::Builder::new()
            .name("ai-helper-out".to_owned())
            .spawn(move || {
                let mut out = BufReader::new(stdout);
                loop {
                    let line = bounded_line(&mut out);
                    let end = line.is_none();
                    if send.send(line).is_err() || end {
                        break;
                    }
                }
            });
        let mut helper = Self {
            child,
            stdin,
            lines,
            hello: serde_json::Value::Null,
        };
        if reader.is_err() {
            return Err(EngineError::Crashed);
        }
        let first = helper.next_line(&Stop::at(Instant::now() + wait))?;
        helper.hello = serde_json::from_str(&first).map_err(|_| EngineError::Crashed)?;
        Ok(helper)
    }

    /// Whether it is still running.
    pub fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Sends `request` (one JSON line) and waits for its answer line.
    ///
    /// # Errors
    ///
    /// [`EngineError::TooLong`] for a request over [`MAX_REQUEST_BYTES`];
    /// [`EngineError::Stopped`] or [`EngineError::TimedOut`] by `stop`
    /// (the caller then drops the helper); [`EngineError::Crashed`] when it
    /// died or its answer isn't JSON.
    pub fn ask(
        &mut self,
        request: &serde_json::Value,
        stop: &Stop,
    ) -> Result<serde_json::Value, EngineError> {
        let mut line = serde_json::to_vec(request).map_err(|_| EngineError::Crashed)?;
        line.push(b'\n');
        let answer = self.exchange(&line, stop)?;
        serde_json::from_str(&answer).map_err(|_| EngineError::Crashed)
    }

    /// Sends one request `line` (already encoded, its newline included)
    /// and waits for the answer line, given back as it came for the
    /// caller's own strict reading.
    ///
    /// # Errors
    ///
    /// As [`Helper::ask`].
    pub fn exchange(&mut self, line: &[u8], stop: &Stop) -> Result<String, EngineError> {
        self.exchange_while(line, stop, &|| true)
    }

    /// As [`Helper::exchange`], and stopped as by the job's stop when
    /// `keep` says no (memory pressure turning critical, say).
    ///
    /// # Errors
    ///
    /// As [`Helper::ask`]; [`EngineError::Stopped`] when `keep` said no.
    pub fn exchange_while(
        &mut self,
        line: &[u8],
        stop: &Stop,
        keep: &dyn Fn() -> bool,
    ) -> Result<String, EngineError> {
        self.send(line)?;
        self.next_line_while(stop, keep)
    }

    /// Sends one request `line` (its newline included) without waiting.
    ///
    /// # Errors
    ///
    /// [`EngineError::TooLong`] over [`MAX_REQUEST_BYTES`] or without its
    /// newline; [`EngineError::Crashed`] when the helper can't take it.
    pub fn send(&mut self, line: &[u8]) -> Result<(), EngineError> {
        if line.len() > MAX_REQUEST_BYTES + 1 || line.last() != Some(&b'\n') {
            return Err(EngineError::TooLong);
        }
        let stdin = self.stdin.as_mut().ok_or(EngineError::Crashed)?;
        stdin
            .write_all(line)
            .and_then(|()| stdin.flush())
            .map_err(|_| EngineError::Crashed)
    }

    /// The next answer line within `wait`, ignoring any job's stop (the
    /// answer to a cancel, ADR-0033 §1's cancellation).
    ///
    /// # Errors
    ///
    /// [`EngineError::TimedOut`] when none came; [`EngineError::Crashed`]
    /// when the helper died.
    pub fn answer_within(&self, wait: Duration) -> Result<String, EngineError> {
        self.next_line(&Stop::at(Instant::now() + wait))
    }

    fn next_line(&self, stop: &Stop) -> Result<String, EngineError> {
        self.next_line_while(stop, &|| true)
    }

    fn next_line_while(&self, stop: &Stop, keep: &dyn Fn() -> bool) -> Result<String, EngineError> {
        loop {
            if stop.stopped() || !keep() {
                return Err(EngineError::Stopped);
            }
            let left = stop.deadline().saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(EngineError::TimedOut);
            }
            match self.lines.recv_timeout(left.min(STOP_POLL)) {
                Ok(Some(line)) => return Ok(line),
                Ok(None) | Err(RecvTimeoutError::Disconnected) => return Err(EngineError::Crashed),
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }
}

impl Drop for Helper {
    /// Closes its stdin (it exits by itself), then makes sure.
    fn drop(&mut self) {
        drop(self.stdin.take());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// One line of at most [`MAX_LINE_BYTES`] without its newline; `None` at
/// the end, on a read error, or for a line over the bound (the helper is
/// then treated as broken).
fn bounded_line(out: &mut impl BufRead) -> Option<String> {
    let mut bytes = Vec::new();
    let read = out
        .by_ref()
        .take(MAX_LINE_BYTES + 1)
        .read_until(b'\n', &mut bytes)
        .ok()?;
    if read == 0 || (bytes.last() != Some(&b'\n') && read as u64 > MAX_LINE_BYTES) {
        return None;
    }
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_over_the_bound_ends_the_stream() {
        let long = vec![b'a'; usize::try_from(MAX_LINE_BYTES).unwrap() + 10];
        assert_eq!(bounded_line(&mut long.as_slice()), None);
        assert_eq!(
            bounded_line(&mut b"{}\nrest".as_slice()),
            Some("{}".to_owned())
        );
        assert_eq!(bounded_line(&mut b"".as_slice()), None);
    }
}
