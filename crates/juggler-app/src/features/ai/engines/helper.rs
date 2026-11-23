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
use std::sync::Mutex;
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

/// How long a helper whose output ended may take to be seen exited.
const EXIT_WAIT: Duration = Duration::from_millis(500);

/// Why a helper didn't start (AJ-E3823): the engines say which in plain
/// words, and never read one as "this computer can't run it".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartFailure {
    /// The system refused to start it, or stopped it before it said
    /// hello (macOS: not allowed to run, or killed).
    Blocked,
    /// It started and said nothing in time (a system prompt about it
    /// may be waiting for the user).
    NoAnswer,
    /// It couldn't be started, ended early, or its hello wasn't JSON.
    Broken,
}

impl StartFailure {
    /// The engine error a caller that needs no reason sees.
    #[must_use]
    pub const fn error(self) -> EngineError {
        match self {
            Self::NoAnswer => EngineError::TimedOut,
            Self::Blocked | Self::Broken => EngineError::Crashed,
        }
    }
}

/// ADR-0033 §3's health rule: this many failures within [`HEALTH_WINDOW`]
/// mark an engine unhealthy until the next probe that finds it fine.
const HEALTH_FAILURES: usize = 3;
const HEALTH_WINDOW: Duration = Duration::from_secs(3_600);

/// What an engine knows of its helper: what it last saw, and its recent
/// failures, for the health rule.
#[derive(Debug)]
pub struct Health<S> {
    seen: Mutex<S>,
    failures: Mutex<Vec<Instant>>,
}

impl<S: Copy> Health<S> {
    /// Nothing seen yet (`seen`), and no failures.
    pub const fn new(seen: S) -> Self {
        Self {
            seen: Mutex::new(seen),
            failures: Mutex::new(Vec::new()),
        }
    }

    /// What it last saw; `poisoned` when its lock is poisoned.
    pub fn seen(&self, poisoned: S) -> S {
        self.seen.lock().map_or(poisoned, |seen| *seen)
    }

    /// What it sees now.
    pub fn see(&self, seen: S) {
        if let Ok(mut slot) = self.seen.lock() {
            *slot = seen;
        }
    }

    /// Too many failures within the window.
    pub fn unhealthy(&self) -> bool {
        self.failures.lock().is_ok_and(|failures| {
            failures
                .iter()
                .filter(|at| at.elapsed() < HEALTH_WINDOW)
                .count()
                >= HEALTH_FAILURES
        })
    }

    /// One more failure, now.
    pub fn failed(&self) {
        if let Ok(mut failures) = self.failures.lock() {
            failures.retain(|at| at.elapsed() < HEALTH_WINDOW);
            failures.push(Instant::now());
        }
    }

    /// A probe found the engine fine: its failures are forgotten.
    pub fn healed(&self) {
        if let Ok(mut failures) = self.failures.lock() {
            failures.clear();
        }
    }
}

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

    /// As [`Helper::start`], saying why it didn't start (AJ-E3823): the
    /// system refused or stopped it, it never answered, or it broke.
    ///
    /// # Errors
    ///
    /// The [`StartFailure`].
    pub fn start_checked(program: &Path, wait: Duration) -> Result<Self, StartFailure> {
        Self::launch_checked(juggler_platform::process::helper(program), wait)
    }

    /// Starts `command` (one of AI Juggler's own helpers, made with
    /// [`juggler_platform::process::helper`]: no shell, only the allowed
    /// environment) and reads its `hello` within `wait`.
    ///
    /// # Errors
    ///
    /// As [`Helper::start`].
    pub fn launch(command: Command, wait: Duration) -> Result<Self, EngineError> {
        Self::launch_checked(command, wait).map_err(StartFailure::error)
    }

    /// As [`Helper::launch`], saying why it didn't start.
    ///
    /// # Errors
    ///
    /// The [`StartFailure`].
    pub fn launch_checked(mut command: Command, wait: Duration) -> Result<Self, StartFailure> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                if juggler_platform::process::refused_to_start(&error) {
                    StartFailure::Blocked
                } else {
                    StartFailure::Broken
                }
            })?;
        let stdin = child.stdin.take();
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(StartFailure::Broken);
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
            return Err(StartFailure::Broken);
        }
        let first = match helper.next_line(&Stop::at(Instant::now() + wait)) {
            Ok(first) => first,
            Err(EngineError::TimedOut | EngineError::Stopped) => {
                return Err(StartFailure::NoAnswer);
            }
            Err(_) => return Err(helper.why_it_ended()),
        };
        helper.hello = serde_json::from_str(&first).map_err(|_| StartFailure::Broken)?;
        Ok(helper)
    }

    /// Why a helper whose output ended before its hello is gone: the
    /// system stopped it, or it broke. Its exit is waited for a moment
    /// (the output closes just before the exit is known).
    fn why_it_ended(&mut self) -> StartFailure {
        let until = Instant::now() + EXIT_WAIT;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) if juggler_platform::process::stopped_by_system(&status) => {
                    return StartFailure::Blocked;
                }
                Ok(Some(_)) | Err(_) => return StartFailure::Broken,
                Ok(None) if Instant::now() >= until => return StartFailure::Broken,
                Ok(None) => std::thread::sleep(STOP_POLL),
            }
        }
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

    /// AJ-E3823: why a helper didn't start, from fake helpers: one the
    /// system kills before its hello reads as blocked, one that never
    /// answers as no answer, one that just fails as broken.
    #[cfg(unix)]
    #[test]
    fn a_helper_that_doesnt_start_says_why() {
        let dir = tempfile::tempdir().unwrap();
        let fake = |name: &str, body: &str| {
            let path = dir.path().join(name);
            crate::test_support::script(&path, &format!("#!/bin/sh\n{body}\n"));
            path
        };
        let wait = Duration::from_millis(400);
        let killed = fake("killed", "kill -9 $$");
        assert_eq!(
            Helper::start_checked(&killed, wait).err(),
            Some(StartFailure::Blocked)
        );
        let silent = fake("silent", "sleep 5");
        assert_eq!(
            Helper::start_checked(&silent, wait).err(),
            Some(StartFailure::NoAnswer)
        );
        let broken = fake("broken", "exit 3");
        assert_eq!(
            Helper::start_checked(&broken, wait).err(),
            Some(StartFailure::Broken)
        );
        let garbled = fake("garbled", "echo 'not json'; cat > /dev/null");
        assert_eq!(
            Helper::start_checked(&garbled, wait).err(),
            Some(StartFailure::Broken)
        );
        let fine = fake("fine", "echo '{\"protocol\":2}'; cat > /dev/null");
        assert!(Helper::start_checked(&fine, wait).is_ok());
        assert_eq!(StartFailure::NoAnswer.error(), EngineError::TimedOut);
        assert_eq!(StartFailure::Blocked.error(), EngineError::Crashed);
    }
}
