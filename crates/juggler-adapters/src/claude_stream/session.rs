//! One headless Claude Code session: the codec over the CLI's stdin and
//! stdout. Lines in, lines out; one writer (this driver) at a time.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::book::drain_turn;
use juggler_adapter::runtime::lines::{Lines, Next, write_json};
use juggler_adapter::runtime::{
    AskAnswer, AskId, Connection, DrivenSession, LimitsSnapshot, LiveStatusWord, Release,
    ReleaseOutcome, RuntimeEvent, TurnInput,
};
use serde_json::Value;

use super::codec::Codec;

/// How long the CLI has to answer `initialize`.
pub const INITIALIZE_WAIT: Duration = Duration::from_secs(30);

/// A headless Claude Code session.
pub struct ClaudeSession {
    lines: Lines,
    writer: Box<dyn std::io::Write + Send>,
    codec: Codec,
    queue: VecDeque<RuntimeEvent>,
    native_id: String,
    exited: bool,
    /// The account's folder, where the transcript is looked for (listing
    /// `projects/` only; nothing is read).
    state_dir: std::path::PathBuf,
    linked: bool,
}

/// `projects/<folder>/<id>.jsonl` under `state_dir`, when it exists: the
/// conversation's own file (its name is its id; the folder is never
/// guessed from the working folder).
#[must_use]
pub fn transcript_of(state_dir: &std::path::Path, id: &str) -> Option<std::path::PathBuf> {
    let file = format!("{id}.jsonl");
    std::fs::read_dir(state_dir.join("projects"))
        .ok()?
        .flatten()
        .map(|entry| entry.path().join(&file))
        .find(|path| path.is_file())
}

impl ClaudeSession {
    fn send(&mut self, value: &Value) -> Result<(), AdapterError> {
        write_json(self.writer.as_mut(), value).map_err(AdapterError::Io)
    }

    fn pump(&mut self, timeout: Duration) -> Result<(), AdapterError> {
        if self.exited {
            return Ok(());
        }
        match self.lines.next(timeout) {
            Next::Line(line) => {
                let (events, replies) = self.codec.incoming(&line);
                for reply in &replies {
                    self.send(reply)?;
                }
                let ended = events
                    .iter()
                    .any(|event| matches!(event, RuntimeEvent::TurnEnded { .. }));
                // The file exists by the turn's end: named before it.
                if ended
                    && !self.linked
                    && let Some(path) = transcript_of(&self.state_dir, &self.native_id)
                {
                    self.linked = true;
                    self.queue.push_back(RuntimeEvent::Transcript {
                        path: path.to_string_lossy().into_owned(),
                    });
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

    /// The handshake: `initialize`, answered within [`INITIALIZE_WAIT`].
    ///
    /// # Errors
    ///
    /// When the CLI refuses, exits or doesn't answer.
    pub fn connect(
        io: Connection,
        native_id: &str,
        state_dir: &std::path::Path,
    ) -> Result<Self, AdapterError> {
        let mut session = Self {
            lines: Lines::spawn(io.reader),
            writer: io.writer,
            codec: Codec::default(),
            queue: VecDeque::new(),
            native_id: native_id.to_owned(),
            exited: false,
            state_dir: state_dir.to_path_buf(),
            linked: false,
        };
        session.codec.set_session(native_id);
        let initialize = session.codec.initialize();
        session.send(&initialize)?;
        let deadline = Instant::now() + INITIALIZE_WAIT;
        while session.codec.initializing.is_some() {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() || session.exited {
                return Err(AdapterError::InvalidLaunch(
                    "Claude Code didn't start its headless session".into(),
                ));
            }
            session.pump(left.min(Duration::from_millis(200)))?;
        }
        if !session.codec.initialized {
            return Err(AdapterError::InvalidLaunch(
                "Claude Code refused the headless session".into(),
            ));
        }
        let early: Vec<RuntimeEvent> = session.queue.drain(..).collect();
        session.queue.push_back(RuntimeEvent::Started {
            native_id: native_id.to_owned(),
            model: None,
            path: None,
        });
        session.queue.push_back(RuntimeEvent::State {
            status: LiveStatusWord::Idle,
        });
        session.queue.extend(early);
        Ok(session)
    }
}

impl DrivenSession for ClaudeSession {
    fn native_id(&self) -> Option<String> {
        Some(self.native_id.clone())
    }

    fn next_event(&mut self, timeout: Duration) -> Result<Option<RuntimeEvent>, AdapterError> {
        if self.queue.is_empty() {
            self.pump(timeout)?;
        }
        Ok(self.queue.pop_front())
    }

    fn send_turn(&mut self, input: TurnInput) -> Result<(), AdapterError> {
        let (line, events) = self.codec.turn(input)?;
        self.send(&line)?;
        self.queue.extend(events);
        Ok(())
    }

    fn interrupt(&mut self) -> Result<(), AdapterError> {
        if !self.codec.turn_running() {
            return Ok(());
        }
        let line = self.codec.interrupt();
        self.send(&line)
    }

    fn answer(&mut self, ask: &AskId, answer: AskAnswer) -> Result<(), AdapterError> {
        let (line, events) = self.codec.answer(ask, &answer)?;
        self.send(&line)?;
        self.queue.extend(events);
        Ok(())
    }

    fn set_model(&mut self, model: &str) -> Result<(), AdapterError> {
        let line = self.codec.set_model(model);
        self.send(&line)?;
        self.queue.push_back(RuntimeEvent::ModelChanged {
            model: model.to_owned(),
        });
        Ok(())
    }

    /// `/compact` as a turn, as headless Claude Code takes it.
    fn compact(&mut self) -> Result<(), AdapterError> {
        self.send_turn(TurnInput::from_user("/compact"))
    }

    fn limits(&mut self) -> Result<Option<LimitsSnapshot>, AdapterError> {
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
        // Its stdin closes with `self`: the CLI ends, one writer at a time.
        Ok(ReleaseOutcome {
            native_id: Some(self.native_id.clone()),
            at_boundary,
            tail,
        })
    }
}
