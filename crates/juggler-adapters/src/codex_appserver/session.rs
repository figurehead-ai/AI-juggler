//! One driven Codex session: the codec over a connection to the session's
//! own app-server. Lines in, lines out; the codec decides what they mean.

use std::collections::VecDeque;
use std::path::Path;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::book::drain_turn;
use juggler_adapter::runtime::lines::{Lines, Next, write_json};
use juggler_adapter::runtime::{
    AskAnswer, AskId, Connection, DrivenSession, LimitsSnapshot, Release, ReleaseOutcome,
    RuntimeEvent, TurnInput,
};
use serde_json::Value;

use super::codec::Codec;
use super::rpc;

/// How long the server has to answer `initialize`.
pub const INITIALIZE_WAIT: Duration = Duration::from_secs(10);
/// How long a thread may take to start or resume (a long thread loads).
pub const THREAD_WAIT: Duration = Duration::from_secs(30);

/// A driven Codex session.
pub struct CodexSession {
    lines: Lines,
    writer: Box<dyn std::io::Write + Send>,
    codec: Codec,
    queue: VecDeque<RuntimeEvent>,
    exited: bool,
}

fn same_folder(a: &Path, b: &Path) -> bool {
    let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    canonical(a) == canonical(b)
}

impl CodexSession {
    fn send(&mut self, value: &Value) -> Result<(), AdapterError> {
        write_json(self.writer.as_mut(), value).map_err(AdapterError::Io)
    }

    /// Reads one line (at most `timeout`) into the queue.
    fn pump(&mut self, timeout: Duration) -> Result<(), AdapterError> {
        if self.exited {
            return Ok(());
        }
        match self.lines.next(timeout) {
            Next::Line(line) => {
                let Some(message) = rpc::classify(&line) else {
                    self.codec.unknown += 1;
                    return Ok(());
                };
                let (events, replies) = self.codec.incoming(message);
                for reply in &replies {
                    self.send(reply)?;
                }
                self.queue.extend(events);
            }
            Next::Timeout => {}
            Next::Closed => {
                self.exited = true;
                self.queue.push_back(RuntimeEvent::Exited);
            }
        }
        Ok(())
    }

    /// Reads until `done` holds or `wait` passes.
    fn wait_for(
        &mut self,
        wait: Duration,
        what: &str,
        done: impl Fn(&Codec) -> bool,
    ) -> Result<(), AdapterError> {
        let deadline = Instant::now() + wait;
        while !done(&self.codec) {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() || self.exited {
                let why = self.queue.iter().find_map(|event| match event {
                    RuntimeEvent::Error { message, .. } => Some(message.clone()),
                    _ => None,
                });
                return Err(AdapterError::InvalidLaunch(why.map_or_else(
                    || format!("Codex's app-server didn't {what} in time"),
                    |why| format!("Codex's app-server didn't {what}: {why}"),
                )));
            }
            self.pump(left.min(Duration::from_millis(200)))?;
            if !done(&self.codec)
                && let Some(why) = self.queue.iter().find_map(|event| match event {
                    RuntimeEvent::Error { message, .. } => Some(message.clone()),
                    _ => None,
                })
            {
                return Err(AdapterError::InvalidLaunch(format!(
                    "Codex's app-server didn't {what}: {why}"
                )));
            }
        }
        Ok(())
    }

    /// The handshake: `initialize` (refusing a server on another
    /// `CODEX_HOME`), then the thread, then a first limits read.
    ///
    /// # Errors
    ///
    /// When the server doesn't answer in time, runs on another home, or
    /// refuses the thread.
    pub fn connect(
        io: Connection,
        home: &Path,
        cwd: &Path,
        resume: Option<&str>,
        model: Option<&str>,
    ) -> Result<Self, AdapterError> {
        let mut session = Self {
            lines: Lines::spawn(io.reader),
            writer: io.writer,
            codec: Codec::default(),
            queue: VecDeque::new(),
            exited: false,
        };
        let initialize = session.codec.initialize();
        session.send(&initialize)?;
        session.wait_for(INITIALIZE_WAIT, "answer", |codec| {
            codec.handshake.codex_home.is_some()
        })?;
        let served = session
            .codec
            .handshake
            .codex_home
            .clone()
            .unwrap_or_default();
        if !same_folder(Path::new(&served), home) {
            return Err(AdapterError::InvalidLaunch(format!(
                "Codex's app-server runs on {served}, not this profile's folder"
            )));
        }
        session.send(&rpc::notification("initialized"))?;
        let thread = session.codec.thread(&cwd.to_string_lossy(), resume, model);
        session.send(&thread)?;
        session.wait_for(THREAD_WAIT, "start the conversation", |codec| {
            codec.handshake.thread.is_some()
        })?;
        let limits = session.codec.rate_limits();
        session.send(&limits)?;
        Ok(session)
    }
}

impl DrivenSession for CodexSession {
    fn native_id(&self) -> Option<String> {
        self.codec.handshake.thread.clone()
    }

    fn next_event(&mut self, timeout: Duration) -> Result<Option<RuntimeEvent>, AdapterError> {
        if self.queue.is_empty() {
            self.pump(timeout)?;
        }
        Ok(self.queue.pop_front())
    }

    fn send_turn(&mut self, input: TurnInput) -> Result<(), AdapterError> {
        let line = self.codec.turn_start(input)?;
        self.send(&line)
    }

    fn steer(&mut self, input: TurnInput) -> Result<(), AdapterError> {
        let line = self.codec.steer(&input)?;
        self.send(&line)
    }

    fn interrupt(&mut self) -> Result<(), AdapterError> {
        match self.codec.interrupt() {
            Some(line) => self.send(&line),
            None => Ok(()),
        }
    }

    fn answer(&mut self, ask: &AskId, answer: AskAnswer) -> Result<(), AdapterError> {
        let (line, events) = self.codec.answer(ask, &answer)?;
        self.send(&line)?;
        self.queue.extend(events);
        Ok(())
    }

    fn set_model(&mut self, model: &str) -> Result<(), AdapterError> {
        let event = self.codec.set_model(model);
        self.queue.push_back(event);
        Ok(())
    }

    fn list_models(&mut self) -> Result<(), AdapterError> {
        let line = self.codec.models();
        self.send(&line)
    }

    fn compact(&mut self) -> Result<(), AdapterError> {
        let line = self.codec.compact()?;
        self.send(&line)
    }

    fn limits(&mut self) -> Result<Option<LimitsSnapshot>, AdapterError> {
        let line = self.codec.rate_limits();
        self.send(&line)?;
        Ok(self.codec.limits())
    }

    fn release(mut self: Box<Self>, how: Release) -> Result<ReleaseOutcome, AdapterError> {
        let mut tail: Vec<RuntimeEvent> = self.queue.drain(..).collect();
        let mut at_boundary = !self.codec.turn_running();
        if self.codec.turn_running() {
            let wait = match how {
                Release::AfterTurn(wait) => wait,
                Release::Now => {
                    self.interrupt()?;
                    Duration::from_secs(5)
                }
            };
            let (more, ended) = drain_turn(&mut |timeout| self.next_event(timeout), wait);
            tail.extend(more);
            at_boundary = ended && matches!(how, Release::AfterTurn(_));
        }
        // The connection closes with `self`; the app stops the server.
        Ok(ReleaseOutcome {
            native_id: self.codec.handshake.thread.clone(),
            at_boundary,
            tail,
        })
    }
}
