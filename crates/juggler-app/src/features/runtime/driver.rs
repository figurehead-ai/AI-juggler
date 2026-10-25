//! One driven session's driver: a thread that owns the adapter's
//! [`DrivenSession`], reads its events into a snapshot the UI reads, keeps
//! the session's stored state exact, and carries the user's commands to the
//! tool. Nothing here answers an ask by itself: an answer arrives only as a
//! [`Command::Answer`], sent by the IPC command the user's click runs.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::{
    AskAnswer, AskId, DrivenSession, ItemKind, LimitsSnapshot, LiveStatusWord, OptionEffect,
    Release, ReleaseOutcome, RuntimeEvent, TurnInput,
};
use juggler_core::model::{AttentionReason, SessionId, SessionStatus, Timestamp};

use super::dto::{
    AskInfo, AskOptionInfo, ChoiceInfo, ConversationItemInfo, LimitWindowInfo, QuestionPartInfo,
    RuntimeInfo, RuntimeViewInfo,
};
use crate::state::AppState;

/// How many conversation lines are held to show (never stored).
pub const ITEMS_KEPT: usize = 60;
/// The longest streaming text held.
const STREAM_MAX: usize = 16 * 1024;
/// How long the driver waits for an event before it looks for commands.
const POLL: Duration = Duration::from_millis(50);

/// What the user asks the driver to do.
pub enum Command {
    /// Start a turn (between turns only).
    Turn(TurnInput, Sender<Result<(), String>>),
    /// Add to the running turn.
    Steer(TurnInput, Sender<Result<(), String>>),
    /// Interrupt the running turn.
    Interrupt(Sender<Result<(), String>>),
    /// The user's own answer to an open ask.
    Answer(AskId, AskAnswer, Sender<Result<(), String>>),
    /// Change the model for the next turns.
    Model(String, Sender<Result<(), String>>),
    /// Compact the conversation.
    Compact(Sender<Result<(), String>>),
    /// Let go (at a turn boundary, or now).
    Release(Release, Sender<Result<ReleaseOutcome, String>>),
}

/// What the driver knows, for the UI.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// The tool's conversation id.
    pub native_id: Option<String>,
    /// The exact state, as the tool said.
    pub status: Option<LiveStatusWord>,
    /// The process or connection is gone.
    pub ended: bool,
    /// A turn runs.
    pub turn_running: bool,
    /// The open asks, oldest first.
    pub asks: Vec<AskInfo>,
    /// The recent conversation.
    pub items: VecDeque<ConversationItemInfo>,
    /// What the agent is writing now.
    pub streaming: String,
    /// The model.
    pub model: Option<String>,
    /// The last limits the tool reported.
    pub limits: Option<LimitsSnapshot>,
    /// The last error.
    pub error: Option<String>,
}

/// A driven session, as the app holds it.
pub struct Driver {
    /// The session.
    pub session: SessionId,
    /// How it runs, in words.
    pub label: &'static str,
    /// A Beta runtime.
    pub beta: bool,
    /// Its socket, for a tool whose terminal attaches.
    pub socket: Option<PathBuf>,
    /// The tool can add to a running turn.
    pub can_steer: bool,
    commands: Mutex<Option<Sender<Command>>>,
    snapshot: Arc<Mutex<Snapshot>>,
}

impl std::fmt::Debug for Driver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Driver")
            .field("session", &self.session)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

fn word(status: Option<LiveStatusWord>, ended: bool) -> Option<String> {
    if ended {
        return Some("ended".into());
    }
    status.map(|status| {
        match status {
            LiveStatusWord::Busy => "working",
            LiveStatusWord::Idle => "idle",
            LiveStatusWord::Waiting => "asks_you",
        }
        .to_owned()
    })
}

fn effect_word(effect: &OptionEffect) -> (&'static str, Option<String>) {
    match effect {
        OptionEffect::AllowOnce => ("allow_once", None),
        OptionEffect::AllowRule { rule } => ("allow_rule", Some(rule.clone())),
        OptionEffect::Deny => ("deny", None),
        OptionEffect::DenyAndStop => ("deny_and_stop", None),
    }
}

fn kind_word(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::UserMessage => "user_message",
        ItemKind::AgentMessage => "agent_message",
        ItemKind::Command => "command",
        ItemKind::FileChange => "file_change",
        ItemKind::Tool => "tool",
        ItemKind::Other => "note",
    }
}

/// The ask an event opens, as the UI shows it.
fn ask_info(session: &SessionId, event: &RuntimeEvent) -> Option<AskInfo> {
    let now = Timestamp::now().millis();
    match event {
        RuntimeEvent::ApprovalRequested { approval } => Some(AskInfo {
            id: approval.id.0.clone(),
            session_id: session.to_string(),
            kind: "approval".into(),
            approval_kind: Some(
                serde_json::to_value(approval.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default(),
            ),
            summary: approval.summary.clone(),
            reason: approval.reason.clone(),
            options: approval
                .options
                .iter()
                .map(|option| {
                    let (effect, rule) = effect_word(&option.effect);
                    AskOptionInfo {
                        id: option.id.clone(),
                        label: option.label.clone(),
                        effect: effect.into(),
                        rule,
                    }
                })
                .collect(),
            parts: Vec::new(),
            asked_at_ms: now,
        }),
        RuntimeEvent::QuestionAsked { question } => Some(AskInfo {
            id: question.id.0.clone(),
            session_id: session.to_string(),
            kind: "question".into(),
            approval_kind: None,
            summary: question
                .parts
                .first()
                .map(|part| part.question.clone())
                .unwrap_or_default(),
            reason: None,
            options: Vec::new(),
            parts: question
                .parts
                .iter()
                .map(|part| QuestionPartInfo {
                    id: part.id.clone(),
                    header: part.header.clone(),
                    question: part.question.clone(),
                    choices: part
                        .choices
                        .iter()
                        .map(|choice| ChoiceInfo {
                            label: choice.label.clone(),
                            description: choice.description.clone(),
                        })
                        .collect(),
                    multi: part.multi,
                    free_text: part.free_text,
                    secret: part.secret,
                })
                .collect(),
            asked_at_ms: now,
        }),
        _ => None,
    }
}

impl Snapshot {
    fn push(&mut self, kind: &str, text: String) {
        if text.trim().is_empty() {
            return;
        }
        self.items.push_back(ConversationItemInfo {
            kind: kind.to_owned(),
            text,
            at_ms: Timestamp::now().millis(),
        });
        while self.items.len() > ITEMS_KEPT {
            self.items.pop_front();
        }
    }

    /// Takes in one event; true when the stored state should change.
    fn apply(&mut self, session: &SessionId, event: &RuntimeEvent) -> bool {
        match event {
            RuntimeEvent::Started { native_id, model } => {
                self.native_id = Some(native_id.clone());
                self.model.clone_from(model);
                false
            }
            RuntimeEvent::State { status } => {
                let changed = self.status != Some(*status);
                self.status = Some(*status);
                changed
            }
            RuntimeEvent::TurnStarted { .. } => {
                self.turn_running = true;
                self.streaming.clear();
                false
            }
            RuntimeEvent::TurnEnded { .. } => {
                self.turn_running = false;
                let text = std::mem::take(&mut self.streaming);
                if self.items.back().is_none_or(|last| last.text != text) {
                    self.push("agent_message", text);
                }
                self.asks.clear();
                false
            }
            RuntimeEvent::TextDelta { text } => {
                if self.streaming.len() + text.len() <= STREAM_MAX {
                    self.streaming.push_str(text);
                }
                false
            }
            RuntimeEvent::Item { item } => {
                if item.kind == ItemKind::AgentMessage {
                    self.streaming.clear();
                }
                self.push(kind_word(item.kind), item.text.clone());
                false
            }
            RuntimeEvent::ApprovalRequested { .. } | RuntimeEvent::QuestionAsked { .. } => {
                if let Some(ask) = ask_info(session, event) {
                    self.asks.retain(|open| open.id != ask.id);
                    self.asks.push(ask);
                }
                true
            }
            RuntimeEvent::AskResolved { id } => {
                self.asks.retain(|open| open.id != id.0);
                true
            }
            RuntimeEvent::LimitsUpdated { limits } => {
                self.limits = Some(limits.clone());
                false
            }
            RuntimeEvent::ModelChanged { model } => {
                self.model = Some(model.clone());
                false
            }
            RuntimeEvent::Error { message, .. } => {
                self.error = Some(message.clone());
                self.push("note", message.clone());
                false
            }
            RuntimeEvent::Exited => {
                self.ended = true;
                self.turn_running = false;
                self.asks.clear();
                true
            }
            RuntimeEvent::ToolDelta { .. }
            | RuntimeEvent::Usage { .. }
            | RuntimeEvent::Compacted
            | RuntimeEvent::Models { .. } => false,
        }
    }
}

impl Driver {
    /// Starts the driver's thread for `session`.
    pub fn spawn(
        state: &Arc<AppState>,
        session: SessionId,
        tool: Box<dyn DrivenSession>,
        meta: Meta,
    ) -> Arc<Self> {
        let (tx, rx) = std::sync::mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Snapshot {
            native_id: tool.native_id(),
            ..Snapshot::default()
        }));
        let socket = meta.socket.clone();
        let driver = Arc::new(Self {
            session: session.clone(),
            label: meta.label,
            beta: meta.beta,
            socket: meta.socket,
            can_steer: meta.can_steer,
            commands: Mutex::new(Some(tx)),
            snapshot: Arc::clone(&snapshot),
        });
        let weak = Arc::downgrade(state);
        let _ = std::thread::Builder::new()
            .name(format!("driver-{}", session.as_str()))
            .spawn(move || {
                run(&weak, &session, tool, &rx, &snapshot);
                // Never leave a socket behind the session.
                if let Some(socket) = socket {
                    super::launch::clear_stale(&socket);
                }
            });
        driver
    }

    /// What it knows now.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot
            .lock()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_default()
    }

    /// The runtime as the UI shows it.
    #[must_use]
    pub fn info(&self) -> RuntimeInfo {
        let snapshot = self.snapshot();
        RuntimeInfo {
            kind: "driven".into(),
            label: Some(self.label.to_owned()),
            beta: self.beta,
            state: word(snapshot.status, snapshot.ended),
            asks: snapshot.asks.clone(),
            can_steer: self.can_steer && snapshot.turn_running && !snapshot.ended,
            fell_back: None,
            error: snapshot.error.clone(),
            limits: snapshot
                .limits
                .map(|limits| {
                    limits
                        .windows
                        .into_iter()
                        .map(|w| LimitWindowInfo {
                            id: w.id,
                            window_mins: w.window_mins,
                            used_pct: w.used_pct,
                            resets_at_ms: w.resets_at_ms,
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// Its page.
    #[must_use]
    pub fn view(&self) -> RuntimeViewInfo {
        let snapshot = self.snapshot();
        RuntimeViewInfo {
            session_id: self.session.to_string(),
            runtime: self.info(),
            items: snapshot.items.iter().cloned().collect(),
            streaming: Some(snapshot.streaming.clone()).filter(|s| !s.is_empty()),
            model: snapshot.model,
        }
    }

    /// Whether the session has ended.
    #[must_use]
    pub fn ended(&self) -> bool {
        self.snapshot().ended
    }

    fn send<T>(
        &self,
        make: impl FnOnce(Sender<Result<T, String>>) -> Command,
        wait: Duration,
    ) -> Result<T, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        let sent = self
            .commands
            .lock()
            .ok()
            .and_then(|commands| commands.as_ref().map(|c| c.send(make(tx)).is_ok()));
        if sent != Some(true) {
            return Err("this session isn't running any more".into());
        }
        rx.recv_timeout(wait)
            .map_err(|_| "the session didn't answer in time".to_owned())?
    }

    /// Starts a turn.
    ///
    /// # Errors
    ///
    /// In words, when the tool can't take it now.
    pub fn turn(&self, input: TurnInput) -> Result<(), String> {
        self.send(|tx| Command::Turn(input, tx), Duration::from_secs(5))
    }

    /// Adds to the running turn.
    ///
    /// # Errors
    ///
    /// In words.
    pub fn steer(&self, input: TurnInput) -> Result<(), String> {
        self.send(|tx| Command::Steer(input, tx), Duration::from_secs(5))
    }

    /// Interrupts the running turn.
    ///
    /// # Errors
    ///
    /// In words.
    pub fn interrupt(&self) -> Result<(), String> {
        self.send(Command::Interrupt, Duration::from_secs(5))
    }

    /// Carries the user's own answer.
    ///
    /// # Errors
    ///
    /// In words, when the ask isn't open or the answer doesn't fit it.
    pub fn answer(&self, ask: AskId, answer: AskAnswer) -> Result<(), String> {
        self.send(
            |tx| Command::Answer(ask, answer, tx),
            Duration::from_secs(5),
        )
    }

    /// Changes the model.
    ///
    /// # Errors
    ///
    /// In words.
    pub fn model(&self, model: String) -> Result<(), String> {
        self.send(|tx| Command::Model(model, tx), Duration::from_secs(5))
    }

    /// Compacts the conversation.
    ///
    /// # Errors
    ///
    /// In words.
    pub fn compact(&self) -> Result<(), String> {
        self.send(Command::Compact, Duration::from_secs(5))
    }

    /// Lets go of the session.
    ///
    /// # Errors
    ///
    /// In words.
    pub fn release(&self, how: Release) -> Result<ReleaseOutcome, String> {
        let wait = match how {
            Release::AfterTurn(wait) => wait + Duration::from_secs(5),
            Release::Now => Duration::from_secs(15),
        };
        let outcome = self.send(|tx| Command::Release(how, tx), wait);
        if let Ok(mut commands) = self.commands.lock() {
            *commands = None;
        }
        outcome
    }
}

/// What the app knows about a driver when it starts it.
pub struct Meta {
    /// How it runs, in words.
    pub label: &'static str,
    /// A Beta runtime.
    pub beta: bool,
    /// Its socket.
    pub socket: Option<PathBuf>,
    /// The tool can add to a running turn.
    pub can_steer: bool,
}

fn words(error: &AdapterError) -> String {
    match error {
        AdapterError::InvalidLaunch(why) | AdapterError::Unsupported(why) => why.clone(),
        other => other.to_string(),
    }
}

fn execute(driven: &mut dyn DrivenSession, command: Command) -> Option<Command> {
    match command {
        Command::Turn(input, reply) => {
            let _ = reply.send(driven.send_turn(input).map_err(|e| words(&e)));
        }
        Command::Steer(input, reply) => {
            let _ = reply.send(driven.steer(input).map_err(|e| words(&e)));
        }
        Command::Interrupt(reply) => {
            let _ = reply.send(driven.interrupt().map_err(|e| words(&e)));
        }
        Command::Answer(ask, answer, reply) => {
            let _ = reply.send(driven.answer(&ask, answer).map_err(|e| words(&e)));
        }
        Command::Model(model, reply) => {
            let _ = reply.send(driven.set_model(&model).map_err(|e| words(&e)));
        }
        Command::Compact(reply) => {
            let _ = reply.send(driven.compact().map_err(|e| words(&e)));
        }
        release @ Command::Release(..) => return Some(release),
    }
    None
}

/// Keeps the stored session exact: Asks you while an ask is open, else
/// running; the notifier tells the UI.
fn store(state: &Arc<AppState>, session: &SessionId, snapshot: &Snapshot) {
    if snapshot.ended {
        state.runtime.notify(session.as_str(), true);
        return;
    }
    let asks = snapshot.asks.first();
    if let Ok(storage) = state.storage.lock()
        && let Ok(mut record) = storage.sessions().get(session)
        && matches!(
            record.status,
            SessionStatus::Starting
                | SessionStatus::Running
                | SessionStatus::WaitingUser
                | SessionStatus::Stuck
        )
    {
        let before = record.status;
        if let Some(ask) = asks {
            record.status = SessionStatus::WaitingUser;
            record.attention = Some(AttentionReason {
                rule: "tool_asks".into(),
                inputs: vec![ask.summary.clone()],
                since_ms: ask.asked_at_ms,
            });
        } else {
            record.status = SessionStatus::Running;
            record.attention = None;
        }
        record.last_activity = Some(Timestamp::now());
        if storage.sessions().update(&record).is_ok() && before != record.status {
            crate::usage_stats::instance_state_entered(&storage, &record);
            drop(storage);
            crate::features::notifications::session_status(
                state,
                session.as_str(),
                record.status.as_str(),
            );
        }
    }
    state.runtime.notify(session.as_str(), true);
}

fn run(
    state: &std::sync::Weak<AppState>,
    session: &SessionId,
    mut driven: Box<dyn DrivenSession>,
    commands: &Receiver<Command>,
    snapshot: &Arc<Mutex<Snapshot>>,
) {
    loop {
        // The user's commands first, so an answer never waits on events.
        loop {
            match commands.try_recv() {
                Ok(command) => {
                    if let Some(Command::Release(how, reply)) = execute(driven.as_mut(), command) {
                        let outcome = driven.release(how).map_err(|e| words(&e));
                        if let (Ok(outcome), Ok(mut snap)) = (&outcome, snapshot.lock()) {
                            for event in &outcome.tail {
                                snap.apply(session, event);
                            }
                        }
                        let _ = reply.send(outcome);
                        if let Some(state) = state.upgrade() {
                            state.runtime.notify(session.as_str(), true);
                        }
                        return;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        let Some(state) = state.upgrade() else { return };
        match driven.next_event(POLL) {
            Ok(Some(event)) => {
                let (stored, ended) = snapshot.lock().map_or((false, false), |mut snap| {
                    (snap.apply(session, &event), snap.ended)
                });
                if stored {
                    let snap = snapshot.lock().map(|s| s.clone()).unwrap_or_default();
                    store(&state, session, &snap);
                } else {
                    state.runtime.notify(session.as_str(), false);
                }
                if ended {
                    return;
                }
            }
            Ok(None) => {}
            Err(_) => {
                if let Ok(mut snap) = snapshot.lock() {
                    snap.apply(session, &RuntimeEvent::Exited);
                }
                state.runtime.notify(session.as_str(), true);
                return;
            }
        }
        // Commands that arrived while reading are taken at the loop's top.
        drop(state);
    }
}
