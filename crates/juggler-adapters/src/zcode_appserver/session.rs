//! One driven `ZCode` session: the codec over a connection to the
//! session's own app-server. Lines in, lines out; the codec decides what
//! they mean.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::book::drain_turn;
use juggler_adapter::runtime::lines::{Lines, Next, write_json};
use juggler_adapter::runtime::{
    AskAnswer, AskId, Connection, DrivenSession, ImportedHistory, LimitsSnapshot, Release,
    ReleaseOutcome, RuntimeEvent, TurnInput,
};
use serde_json::Value;

use super::codec::Codec;
use crate::codex_appserver::rpc;

/// How long the engine may take to open (or import) a session: it starts
/// its store and its provider settings first.
pub const OPEN_WAIT: Duration = Duration::from_secs(60);

/// How a session opens.
#[derive(Clone, Copy, Debug)]
pub enum Open<'a> {
    /// A new session.
    New,
    /// A stored one, by id.
    Resume(&'a str),
    /// A new one, `id`, with a conversation imported into it.
    Import {
        /// Its id (`sess_<uuid>`).
        id: &'a str,
        /// The conversation.
        history: &'a ImportedHistory,
    },
}

/// A driven `ZCode` session.
pub struct ZCodeSession {
    lines: Lines,
    writer: Box<dyn std::io::Write + Send>,
    codec: Codec,
    queue: VecDeque<RuntimeEvent>,
    exited: bool,
}

impl ZCodeSession {
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

    /// Reads until `done` holds, an error comes, or `wait` passes.
    fn wait_for(
        &mut self,
        wait: Duration,
        what: &str,
        done: impl Fn(&Codec) -> bool,
    ) -> Result<(), AdapterError> {
        let deadline = Instant::now() + wait;
        while !done(&self.codec) {
            let why = self.queue.iter().find_map(|event| match event {
                RuntimeEvent::Error { message, .. } => Some(message.clone()),
                _ => None,
            });
            if let Some(why) = why {
                return Err(AdapterError::InvalidLaunch(format!(
                    "ZCode's app server didn't {what}: {why}"
                )));
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() || self.exited {
                return Err(AdapterError::InvalidLaunch(format!(
                    "ZCode's app server didn't {what} in time"
                )));
            }
            self.pump(left.min(Duration::from_millis(200)))?;
        }
        Ok(())
    }

    /// Opens the session (`session/create` or `session/resume`) in `cwd`,
    /// then subscribes to its events. No `initialize`: the engine has none.
    ///
    /// # Errors
    ///
    /// When the engine doesn't answer in time or refuses.
    pub fn connect(io: Connection, cwd: &str, open: Open<'_>) -> Result<Self, AdapterError> {
        let mut session = Self {
            lines: Lines::spawn(io.reader),
            writer: io.writer,
            codec: Codec::default(),
            queue: VecDeque::new(),
            exited: false,
        };
        let first = match open {
            Open::New => session.codec.create(cwd, None, None),
            Open::Resume(id) => session.codec.resume(id, cwd),
            Open::Import { id, history } => session.codec.create(cwd, Some(id), Some(history)),
        };
        session.send(&first)?;
        session.wait_for(OPEN_WAIT, "open the session", |codec| {
            codec.handshake.session.is_some()
        })?;
        let subscribe = session.codec.subscribe();
        session.send(&subscribe)?;
        session.wait_for(OPEN_WAIT, "send the session's events", |codec| {
            codec.handshake.subscribed
        })?;
        Ok(session)
    }
}

impl DrivenSession for ZCodeSession {
    fn native_id(&self) -> Option<String> {
        self.codec.handshake.session.clone()
    }

    fn next_event(&mut self, timeout: Duration) -> Result<Option<RuntimeEvent>, AdapterError> {
        if self.queue.is_empty() {
            self.pump(timeout)?;
        }
        Ok(self.queue.pop_front())
    }

    fn send_turn(&mut self, input: TurnInput) -> Result<(), AdapterError> {
        let line = self.codec.send(&input)?;
        self.send(&line)
    }

    fn interrupt(&mut self) -> Result<(), AdapterError> {
        match self.codec.stop() {
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

    /// The model is the App's own choice in `ZCode` (its tree's provider
    /// settings), not one AI Juggler sets.
    fn set_model(&mut self, _model: &str) -> Result<(), AdapterError> {
        Err(AdapterError::Unsupported(
            "ZCode keeps the model as this App's own choice: change it in ZCode".into(),
        ))
    }

    /// The engine reports no plan limits over its protocol; a refusal at
    /// the limit arrives as a rate-limit error.
    fn limits(&mut self) -> Result<Option<LimitsSnapshot>, AdapterError> {
        Ok(None)
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
            native_id: self.codec.handshake.session.clone(),
            at_boundary,
            tail,
        })
    }
}
