//! One headless Claude Code session: the codec over the CLI's stdin and
//! stdout. Lines in, lines out; one writer (this driver) at a time.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::session::{LineAndEvents, LineCodec, LineSession};
use juggler_adapter::runtime::{
    AskAnswer, AskId, Connection, LimitsSnapshot, LiveStatusWord, RuntimeEvent, TurnInput,
};
use serde_json::Value;

use super::codec::Codec;

/// How long the CLI has to answer `initialize`.
pub const INITIALIZE_WAIT: Duration = Duration::from_secs(30);

/// The stream-json codec of one headless session, naming the session's
/// transcript once it exists.
pub struct Headless {
    /// What the lines mean.
    pub codec: Codec,
    native_id: String,
    /// The account's folder, where the transcript is looked for (listing
    /// `projects/` only; nothing is read).
    state_dir: PathBuf,
    linked: bool,
}

/// A headless Claude Code session.
pub type ClaudeSession = LineSession<Headless>;

/// `projects/<folder>/<id>.jsonl` under `state_dir`, when it exists: the
/// conversation's own file (its name is its id; the folder is never
/// guessed from the working folder).
#[must_use]
pub fn transcript_of(state_dir: &Path, id: &str) -> Option<PathBuf> {
    let file = format!("{id}.jsonl");
    std::fs::read_dir(state_dir.join("projects"))
        .ok()?
        .flatten()
        .map(|entry| entry.path().join(&file))
        .find(|path| path.is_file())
}

/// The handshake: `initialize`, answered within [`INITIALIZE_WAIT`].
///
/// # Errors
///
/// When the CLI refuses, exits or doesn't answer.
pub fn connect(
    io: Connection,
    native_id: &str,
    state_dir: &Path,
) -> Result<ClaudeSession, AdapterError> {
    let mut codec = Codec::default();
    codec.set_session(native_id);
    let headless = Headless {
        codec,
        native_id: native_id.to_owned(),
        state_dir: state_dir.to_path_buf(),
        linked: false,
    };
    let mut session = LineSession::new(io, headless, "Claude Code");
    let initialize = session.codec.codec.initialize();
    session.send(&initialize)?;
    let deadline = Instant::now() + INITIALIZE_WAIT;
    while session.codec.codec.initializing.is_some() {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || session.pipe.exited() {
            return Err(AdapterError::InvalidLaunch(
                "Claude Code didn't start its headless session".into(),
            ));
        }
        session.pump(left.min(Duration::from_millis(200)))?;
    }
    if !session.codec.codec.initialized {
        return Err(AdapterError::InvalidLaunch(
            "Claude Code refused the headless session".into(),
        ));
    }
    let early = session.pipe.drain();
    session.pipe.queue.push_back(RuntimeEvent::Started {
        native_id: native_id.to_owned(),
        model: None,
        path: None,
    });
    session.pipe.queue.push_back(RuntimeEvent::State {
        status: LiveStatusWord::Idle,
    });
    session.pipe.queue.extend(early);
    Ok(session)
}

impl LineCodec for Headless {
    fn take_line(&mut self, line: &str) -> (Vec<RuntimeEvent>, Vec<Value>) {
        let (mut events, replies) = self.codec.incoming(line);
        let ended = events
            .iter()
            .any(|event| matches!(event, RuntimeEvent::TurnEnded { .. }));
        // The file exists by the turn's end: named before it.
        if ended
            && !self.linked
            && let Some(path) = transcript_of(&self.state_dir, &self.native_id)
        {
            self.linked = true;
            let path = path.to_string_lossy().into_owned();
            events.insert(0, RuntimeEvent::Transcript { path });
        }
        (events, replies)
    }

    fn native_id(&self) -> Option<String> {
        Some(self.native_id.clone())
    }

    fn turn_running(&self) -> bool {
        self.codec.turn_running()
    }

    fn turn(&mut self, input: TurnInput) -> Result<LineAndEvents, AdapterError> {
        self.codec.turn(input)
    }

    fn stop(&mut self) -> Option<Value> {
        self.codec.turn_running().then(|| self.codec.interrupt())
    }

    fn answer(&mut self, ask: &AskId, answer: &AskAnswer) -> Result<LineAndEvents, AdapterError> {
        self.codec.answer(ask, answer)
    }

    fn set_model(&mut self, model: &str) -> Result<(Option<Value>, RuntimeEvent), AdapterError> {
        let line = self.codec.set_model(model);
        let changed = RuntimeEvent::ModelChanged {
            model: model.to_owned(),
        };
        Ok((Some(line), changed))
    }

    /// `/compact` as a turn, as headless Claude Code takes it.
    fn compact(&mut self) -> Result<LineAndEvents, AdapterError> {
        self.codec.turn(TurnInput::from_user("/compact"))
    }

    fn limits(&self) -> Option<LimitsSnapshot> {
        self.codec.limits()
    }
}
