//! `juggler-local-ai`: AI Juggler's local-model helper (ADR-0033 §4.2,
//! ADR-0035 §11, D-131). It runs one downloaded GGUF file with llama.cpp
//! (llama-cpp-2 0.1.146, Metal on Apple silicon), out of the app's
//! process, so a C++ crash, a leak or a parser bug in a model file can't
//! take AI Juggler down, and the model's memory goes when it exits.
//!
//! **What it may touch.**
//! - stdin and stdout, the protocol (`juggler_local_ai_protocol`); stderr
//!   is unused (llama.cpp's logs are switched off).
//! - One model file at a time: `<sha256>.gguf` in the folder given by
//!   `--models-dir` at start, a plain file (no link) of the exact size
//!   asked for. The name can't hold a separator (the protocol's check).
//!   The app verified the file's SHA-256 before asking.
//! - Nothing else: no network code is called, no shell, no other process,
//!   no file written (but Metal's shader cache). The app starts it under
//!   its own deny-by-default sandbox profile
//!   (`juggler_local_ai_protocol::SANDBOX_PROFILE`, through
//!   `sandbox-exec`), which denies the network, every file outside the
//!   models' folder and the system's libraries, and every other program.
//!
//! **It checks it is sandboxed** before it says hello (the re-review's N3:
//! a sandbox that silently stopped applying must not fail open): it tries
//! to read `/etc/hosts` and to list the models' folder's parent, both of
//! which the profile denies. If either works, its hello says
//! `sandboxed: false` and it exits without serving (the app logs it and
//! hides the jobs). Only the helper's own tests, which run it without the
//! profile, set `UNSANDBOXED_FOR_TESTS` to have it serve anyway.
//!
//! **How it runs.** A reader thread takes request lines (bounded before
//! they are parsed) and hands a load or a generation to the worker, one
//! at a time; a second one meanwhile is answered `busy`. A cancel for the
//! running generation stops it between two tokens. It exits when stdin
//! closes, on `quit`, after `IDLE_EXIT_SECS` with nothing to do, and on a
//! line over its bound (the stream can't be trusted after it).
//!
//! **It ends cleanly.** The reader never exits the process: on quit, a
//! closed stdin or an oversized line it stops the running generation and
//! goes, the worker unwinds (freeing the model and its GPU buffers), and
//! `main` exits with the reader's code. Exiting with the model still
//! loaded would run the GPU backend's teardown under it (an abort).

mod llm;

use std::io::{BufRead, BufReader, Read as _, Write as _};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use juggler_local_ai_protocol::{
    Answer, Fault, IDLE_EXIT_SECS, MAX_REQUEST_BYTES, MODELS_DIR_ARG, PROTOCOL, Request,
    UNSANDBOXED_FOR_TESTS, parse_request,
};

/// What the reader and the worker share.
#[derive(Debug, Default)]
pub struct Shared {
    /// A load or a generation is queued or running.
    busy: AtomicBool,
    /// The running generation's id (0: none).
    running: AtomicU64,
    /// The running generation was asked to stop.
    cancel: AtomicBool,
    /// The answers' channel, one line at a time.
    out: Mutex<()>,
    /// The exit code the reader chose when it stopped.
    exit: AtomicI32,
}

impl Shared {
    /// Writes one answer line; exits when stdout is gone (the app is).
    pub fn send(&self, answer: &Answer) {
        let Ok(line) = answer.encode() else {
            // Every answer is built within its bound; never write a
            // broken line.
            std::process::exit(3);
        };
        let _guard = self.out.lock();
        let mut out = std::io::stdout().lock();
        if out.write_all(&line).and_then(|()| out.flush()).is_err() {
            std::process::exit(0);
        }
    }

    /// The running generation, for the worker: `id`, or none (`0`).
    pub fn set_running(&self, id: u64) {
        self.cancel.store(false, Ordering::SeqCst);
        self.running.store(id, Ordering::SeqCst);
    }

    /// Whether the running generation was asked to stop.
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    /// The reader stops: the running generation stops with it, and the
    /// process exits with `code` once the worker has let go.
    fn stop_with(&self, code: i32) {
        self.exit.store(code, Ordering::SeqCst);
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// The worker answered: the next request may come.
    pub fn done(&self) {
        self.running.store(0, Ordering::SeqCst);
        self.busy.store(false, Ordering::SeqCst);
    }
}

/// Whether the sandbox holds: what its profile denies can't be read.
fn sandboxed(dir: &std::path::Path) -> bool {
    let hosts = std::fs::read("/etc/hosts").is_ok();
    let parent = dir.parent().is_some_and(|p| std::fs::read_dir(p).is_ok());
    !hosts && !parent
}

/// The models' folder from the arguments: exactly `--models-dir <dir>`,
/// an absolute path to a directory that is no link.
fn models_dir() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    let (Some(flag), Some(dir), None) = (args.next(), args.next(), args.next()) else {
        return None;
    };
    if flag != MODELS_DIR_ARG {
        return None;
    }
    let dir = PathBuf::from(dir);
    let meta = std::fs::symlink_metadata(&dir).ok()?;
    (dir.is_absolute() && meta.is_dir()).then_some(dir)
}

fn main() {
    let Some(dir) = models_dir() else {
        std::process::exit(2);
    };
    let Some(backend) = llm::backend() else {
        std::process::exit(1);
    };
    let sandboxed = sandboxed(&dir);
    let shared = Arc::new(Shared::default());
    shared.send(&Answer::Hello {
        protocol: PROTOCOL,
        gpu: backend.supports_gpu_offload(),
        sandboxed,
    });
    if !sandboxed && std::env::var_os(UNSANDBOXED_FOR_TESTS).is_none_or(|v| v != "1") {
        // Never serve outside the sandbox (fails closed).
        drop(backend);
        std::process::exit(4);
    }
    let (work, queue) = channel();
    let reader = Arc::clone(&shared);
    if std::thread::Builder::new()
        .name("requests".to_owned())
        .spawn(move || read_requests(&reader, &work))
        .is_err()
    {
        std::process::exit(1);
    }
    llm::serve(&backend, &dir, &Wait::new(queue), &shared);
    drop(backend);
    std::process::exit(shared.exit.load(Ordering::SeqCst));
}

/// The worker's queue, with the idle exit.
#[derive(Debug)]
pub struct Wait {
    queue: Receiver<Request>,
}

impl Wait {
    const fn new(queue: Receiver<Request>) -> Self {
        Self { queue }
    }

    /// The next load or generation; `None` after `IDLE_EXIT_SECS` with
    /// none, or when the reader is gone (the worker then unwinds).
    #[must_use]
    pub fn next(&self) -> Option<Request> {
        self.queue
            .recv_timeout(Duration::from_secs(IDLE_EXIT_SECS))
            .ok()
    }
}

/// The reader: one bounded line at a time.
fn read_requests(shared: &Shared, work: &Sender<Request>) {
    let mut input = BufReader::new(std::io::stdin().lock());
    loop {
        let line = match bounded_line(&mut input) {
            Line::End => return shared.stop_with(0),
            Line::TooLong => {
                shared.send(&Answer::Error {
                    id: None,
                    error: Fault::TooLarge,
                });
                return shared.stop_with(2);
            }
            Line::Bytes(bytes) => bytes,
        };
        let request = match parse_request(&line) {
            Ok(request) => request,
            Err(error) => {
                shared.send(&Answer::Error { id: None, error });
                continue;
            }
        };
        match request {
            Request::Quit {} => return shared.stop_with(0),
            Request::Cancel { id } => {
                // Only the running generation; anything else is a no-op
                // (it has answered, or never ran). A cancel has no answer
                // of its own: the generation's answer says `stopped`.
                if id != 0 && shared.running.load(Ordering::SeqCst) == id {
                    shared.cancel.store(true, Ordering::SeqCst);
                }
            }
            Request::Load { id, .. } | Request::Generate { id, .. } => {
                if shared.busy.swap(true, Ordering::SeqCst) {
                    shared.send(&Answer::Error {
                        id: Some(id),
                        error: Fault::Busy,
                    });
                } else if work.send(request).is_err() {
                    return shared.stop_with(0);
                }
            }
        }
    }
}

/// One line read from stdin.
enum Line {
    /// Its bytes, without the newline.
    Bytes(Vec<u8>),
    /// Over `MAX_REQUEST_BYTES`.
    TooLong,
    /// stdin closed (or failed).
    End,
}

fn bounded_line(input: &mut impl BufRead) -> Line {
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_REQUEST_BYTES).unwrap_or(u64::MAX) + 1;
    match input.by_ref().take(limit).read_until(b'\n', &mut bytes) {
        Ok(0) | Err(_) => Line::End,
        Ok(_) => {
            if bytes.last() == Some(&b'\n') {
                bytes.pop();
                Line::Bytes(bytes)
            } else if bytes.len() > MAX_REQUEST_BYTES {
                Line::TooLong
            } else {
                // The last line, cut by the end of stdin: never a request.
                Line::End
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(input: &[u8]) -> Vec<String> {
        let mut reader = input;
        let mut out = Vec::new();
        loop {
            match bounded_line(&mut reader) {
                Line::Bytes(bytes) => out.push(String::from_utf8_lossy(&bytes).into_owned()),
                Line::TooLong => {
                    out.push("<too long>".to_owned());
                    break;
                }
                Line::End => break,
            }
        }
        out
    }

    #[test]
    fn lines_are_bounded_and_a_cut_last_line_is_dropped() {
        assert_eq!(lines(b"a\nb\n"), ["a", "b"]);
        assert_eq!(lines(b"a\nb"), ["a"]);
        let mut long = vec![b'x'; MAX_REQUEST_BYTES + 1];
        long.push(b'\n');
        assert_eq!(lines(&long), ["<too long>"]);
        let mut fits = vec![b'x'; MAX_REQUEST_BYTES];
        fits.push(b'\n');
        assert_eq!(lines(&fits).len(), 1);
    }
}
