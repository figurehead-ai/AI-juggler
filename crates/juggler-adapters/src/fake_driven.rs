//! AJ-E2302: the fake adapter's driven runtime, over `fake-harness drive`
//! (`src/bin/support/drive.rs`). The reference codec adapters can mirror,
//! and the double the app's driver and the conformance kit run against.

use std::collections::VecDeque;
use std::path::Path;
use std::time::Duration;

use juggler_adapter::runtime::book::{AskBook, drain_turn};
use juggler_adapter::runtime::lines::{Lines, Next, write_json};
use juggler_adapter::runtime::{
    Approval, ApprovalKind, AskAnswer, AskId, AskOption, Choice, Connection, DrivenLaunch,
    DrivenRuntime, DrivenSession, DrivenSpec, ErrorCategory, LimitWindow, LimitsSnapshot,
    LiveStatusWord, Maturity, OptionEffect, Question, QuestionPart, Release, ReleaseOutcome,
    Runtime, RuntimeEvent, TerminalView, Transport, TurnEnd, TurnInput, TurnOrigin, TurnUsage,
    ViewOf, events::bounded,
};
use juggler_adapter::{AdapterError, HarnessAdapter, LaunchMode};
use juggler_core::model::{Account, Profile};
use serde_json::{Value, json};

use crate::fake::FakeHarnessAdapter;

impl DrivenRuntime for FakeHarnessAdapter {
    fn default_runtime(&self) -> Runtime {
        Runtime::Terminal
    }

    fn maturity(&self) -> Maturity {
        Maturity::Stable
    }

    fn unavailable(&self) -> Option<String> {
        None
    }

    /// The terminal launch's program and environment (the same isolation),
    /// with `drive` instead of `run`.
    fn driven_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        launch: &DrivenLaunch,
    ) -> Result<DrivenSpec, AdapterError> {
        let mode = launch
            .resume
            .as_ref()
            .map_or(LaunchMode::Interactive, |id| LaunchMode::Resume {
                session_id: id.clone(),
                delta_prompt: None,
            });
        let mut spec = self.launch_spec(account, profile, cwd, &mode)?;
        let id = launch
            .resume
            .clone()
            .unwrap_or_else(|| launch.fresh_id.clone());
        crate::codex::refuse_option_like(&id)?;
        let dir = account.state_dir.join("transcripts");
        let flag = if launch.resume.is_some() {
            "--resume"
        } else {
            "--session-id"
        };
        spec.args = vec![
            "drive".to_owned(),
            "--dir".to_owned(),
            dir.to_string_lossy().into_owned(),
            flag.to_owned(),
            id.clone(),
        ];
        spec.needs_pty = false;
        Ok(DrivenSpec {
            launch: spec,
            transport: Transport::Stdio,
            native_id: Some(id),
            model: None,
        })
    }

    fn connect(
        &self,
        io: Connection,
        spec: &DrivenSpec,
        _account: &Account,
    ) -> Result<Box<dyn DrivenSession>, AdapterError> {
        Ok(Box::new(FakeDrivenSession {
            lines: Lines::spawn(io.reader),
            writer: io.writer,
            native_id: spec.native_id.clone(),
            queue: VecDeque::new(),
            book: AskBook::default(),
            turn_running: false,
            origin: None,
            limits: None,
            exited: false,
        }))
    }

    fn terminal_view(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        of: &ViewOf,
    ) -> Result<TerminalView, AdapterError> {
        let mode = LaunchMode::Resume {
            session_id: of.native_id.clone(),
            delta_prompt: None,
        };
        self.launch_spec(account, profile, cwd, &mode)
            .map(TerminalView::AfterRelease)
    }
}

/// One session of the fake's protocol.
pub struct FakeDrivenSession {
    lines: Lines,
    writer: Box<dyn std::io::Write + Send>,
    native_id: Option<String>,
    queue: VecDeque<RuntimeEvent>,
    book: AskBook,
    turn_running: bool,
    origin: Option<TurnOrigin>,
    limits: Option<LimitsSnapshot>,
    exited: bool,
}

fn word(message: &Value, key: &str) -> String {
    message
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn state(status: LiveStatusWord) -> RuntimeEvent {
    RuntimeEvent::State { status }
}

/// The fake's `ask` as an approval, with the options it offered.
fn approval(message: &Value) -> RuntimeEvent {
    let options = message
        .get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|pair| {
            let id = pair.get(0)?.as_str()?;
            let label = pair.get(1)?.as_str()?;
            let effect = match id {
                "once" => OptionEffect::AllowOnce,
                "always" => OptionEffect::AllowRule {
                    rule: label.to_owned(),
                },
                _ => OptionEffect::Deny,
            };
            Some(AskOption {
                id: id.to_owned(),
                label: label.to_owned(),
                effect,
            })
        })
        .collect();
    RuntimeEvent::ApprovalRequested {
        approval: Approval {
            id: AskId(word(message, "id")),
            kind: ApprovalKind::Command,
            summary: bounded(&word(message, "summary")),
            reason: None,
            options,
        },
    }
}

/// The fake's `question`, one part with its choices and a free reply.
fn question(message: &Value) -> RuntimeEvent {
    let choices = message
        .get("choices")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|label| Choice {
            label: label.to_owned(),
            description: None,
        })
        .collect();
    RuntimeEvent::QuestionAsked {
        question: Question {
            id: AskId(word(message, "id")),
            parts: vec![QuestionPart {
                id: "answer".to_owned(),
                header: "Question".to_owned(),
                question: bounded(&word(message, "question")),
                choices,
                multi: false,
                free_text: true,
                secret: false,
            }],
        },
    }
}

impl FakeDrivenSession {
    fn send(&mut self, value: &Value) -> Result<(), AdapterError> {
        write_json(self.writer.as_mut(), value).map_err(AdapterError::Io)
    }

    /// The fake's message as neutral events (first one returned, the rest
    /// queued).
    fn map(&mut self, message: &Value) -> Vec<RuntimeEvent> {
        match word(message, "ev").as_str() {
            "ready" => {
                let id = word(message, "id");
                self.native_id = Some(id.clone());
                vec![
                    RuntimeEvent::Started {
                        native_id: id,
                        model: Some(word(message, "model")),
                    },
                    state(LiveStatusWord::Idle),
                ]
            }
            "turn_started" => {
                self.turn_running = true;
                vec![
                    RuntimeEvent::TurnStarted {
                        turn: message.get("turn").map(Value::to_string),
                        origin: self.origin.take(),
                    },
                    state(LiveStatusWord::Busy),
                ]
            }
            "text" => vec![RuntimeEvent::TextDelta {
                text: bounded(&word(message, "text")),
            }],
            "ask" => vec![approval(message), state(LiveStatusWord::Waiting)],
            "question" => vec![question(message), state(LiveStatusWord::Waiting)],
            "resolved" => vec![
                RuntimeEvent::AskResolved {
                    id: AskId(word(message, "id")),
                },
                state(LiveStatusWord::Busy),
            ],
            "turn_end" => {
                self.turn_running = false;
                let outcome = match word(message, "outcome").as_str() {
                    "completed" => TurnEnd::Completed,
                    "interrupted" => TurnEnd::Interrupted,
                    _ => TurnEnd::Failed,
                };
                let count = |key: &str| message.get(key).and_then(Value::as_u64).unwrap_or(0);
                vec![
                    RuntimeEvent::TurnEnded {
                        outcome,
                        usage: Some(TurnUsage {
                            input: count("input"),
                            output: count("output"),
                            ..TurnUsage::default()
                        }),
                    },
                    state(LiveStatusWord::Idle),
                ]
            }
            "limits" => {
                let limits = LimitsSnapshot {
                    windows: vec![LimitWindow {
                        id: "primary".to_owned(),
                        window_mins: message
                            .get("window_mins")
                            .and_then(Value::as_u64)
                            .and_then(|mins| u32::try_from(mins).ok()),
                        used_pct: message.get("used").and_then(Value::as_f64).unwrap_or(0.0),
                        resets_at_ms: message.get("resets_at").and_then(Value::as_i64),
                    }],
                    plan: None,
                    reached: false,
                };
                self.limits = Some(limits.clone());
                vec![RuntimeEvent::LimitsUpdated { limits }]
            }
            "model" => vec![RuntimeEvent::ModelChanged {
                model: word(message, "model"),
            }],
            "error" => vec![RuntimeEvent::Error {
                category: ErrorCategory::Other,
                message: bounded(&word(message, "message")),
                will_retry: false,
            }],
            _ => Vec::new(), // unknown: ignored, never guessed
        }
    }
}

impl DrivenSession for FakeDrivenSession {
    fn native_id(&self) -> Option<String> {
        self.native_id.clone()
    }

    fn next_event(&mut self, timeout: Duration) -> Result<Option<RuntimeEvent>, AdapterError> {
        if let Some(event) = self.queue.pop_front() {
            return Ok(Some(event));
        }
        if self.exited {
            return Ok(None);
        }
        match self.lines.next(timeout) {
            Next::Line(line) => {
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    return Ok(None);
                };
                let events = self.map(&message);
                for event in &events {
                    self.book.note(event);
                }
                self.queue.extend(events);
                Ok(self.queue.pop_front())
            }
            Next::Timeout => Ok(None),
            Next::Closed => {
                self.exited = true;
                self.book.note(&RuntimeEvent::Exited);
                Ok(Some(RuntimeEvent::Exited))
            }
        }
    }

    fn send_turn(&mut self, input: TurnInput) -> Result<(), AdapterError> {
        if self.turn_running {
            return Err(AdapterError::InvalidLaunch(
                "a turn is running: it can start when this one ends".into(),
            ));
        }
        self.origin = Some(input.origin);
        self.send(&json!({"op": "turn", "text": input.text}))
    }

    fn interrupt(&mut self) -> Result<(), AdapterError> {
        self.send(&json!({"op": "interrupt"}))
    }

    fn answer(&mut self, ask: &AskId, answer: AskAnswer) -> Result<(), AdapterError> {
        self.book.check(ask, &answer)?;
        let message = match answer {
            AskAnswer::Choose { option } => {
                json!({"op": "answer", "id": ask.as_str(), "option": option})
            }
            AskAnswer::Reply { parts } => {
                let answers: Vec<String> =
                    parts.into_iter().flat_map(|part| part.answers).collect();
                json!({"op": "reply", "id": ask.as_str(), "answers": answers})
            }
        };
        self.send(&message)
    }

    fn set_model(&mut self, model: &str) -> Result<(), AdapterError> {
        self.send(&json!({"op": "model", "model": model}))
    }

    fn limits(&mut self) -> Result<Option<LimitsSnapshot>, AdapterError> {
        self.send(&json!({"op": "limits"}))?;
        Ok(self.limits.clone())
    }

    fn release(mut self: Box<Self>, how: Release) -> Result<ReleaseOutcome, AdapterError> {
        let mut tail: Vec<RuntimeEvent> = self.queue.drain(..).collect();
        let mut at_boundary = !self.turn_running;
        if self.turn_running {
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
        // Closing stdin ends the fake (the writer drops with `self`).
        Ok(ReleaseOutcome {
            native_id: self.native_id.clone(),
            at_boundary,
            tail,
        })
    }
}
