//! ADR-0033 §4 (AJ-E3802): the one client for AI Juggler's model helpers.
//! A helper speaks newline-delimited JSON over stdin and stdout, one
//! request at a time; its first line is its `hello`. The client starts
//! it with only what it needs (`juggler_platform::process::helper`),
//! bounds every line, answers within a deadline or stops it, and stops it
//! when dropped. A helper exits by itself when its stdin closes, so AI
//! Juggler's death never leaves one behind.

use std::io::{BufRead, BufReader, Read as _, Write as _};
use std::path::Path;
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use crate::features::ai::engine::{EngineError, Stop};

/// The largest request line (ADR-0033 §4: 32 KiB).
pub const MAX_REQUEST_BYTES: usize = 32 * 1024;

/// The largest answer line a helper may write.
pub const MAX_LINE_BYTES: u64 = 64 * 1024;

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
        let mut child = juggler_platform::process::helper(program)
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
        if line.len() > MAX_REQUEST_BYTES {
            return Err(EngineError::TooLong);
        }
        line.push(b'\n');
        let stdin = self.stdin.as_mut().ok_or(EngineError::Crashed)?;
        stdin
            .write_all(&line)
            .and_then(|()| stdin.flush())
            .map_err(|_| EngineError::Crashed)?;
        let answer = self.next_line(stop)?;
        serde_json::from_str(&answer).map_err(|_| EngineError::Crashed)
    }

    fn next_line(&self, stop: &Stop) -> Result<String, EngineError> {
        loop {
            if stop.stopped() {
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
