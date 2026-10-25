//! app-server's messages as neutral runtime events, and AI Juggler's
//! requests as app-server's. Pure: no I/O, so the recorded fixtures drive
//! it directly (`tests/codex_app_server.rs`). The session (`session.rs`)
//! only moves lines.

use std::collections::HashMap;

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::book::AskBook;
use juggler_adapter::runtime::events::bounded;
use juggler_adapter::runtime::{
    AskAnswer, AskId, ErrorCategory, ItemKind, ItemSummary, LimitWindow, LimitsSnapshot,
    LiveStatusWord, ModelChoice, RuntimeEvent, TurnEnd, TurnInput, TurnOrigin, TurnUsage,
};
use serde_json::{Value, json};

use super::asks::{Answering, ask_of, response};
use super::rpc::{self, Incoming};

/// What one of AI Juggler's requests was for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sent {
    /// `initialize`.
    Initialize,
    /// `thread/start` or `thread/resume`.
    Thread,
    /// `turn/start`.
    TurnStart,
    /// `turn/steer`.
    Steer,
    /// `turn/interrupt`.
    Interrupt,
    /// `thread/compact/start`.
    Compact,
    /// `model/list`.
    Models,
    /// `account/rateLimits/read`.
    RateLimits,
}

/// What the handshake learned.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Handshake {
    /// The server's `codexHome` (`initialize`).
    pub codex_home: Option<String>,
    /// The thread's id, once started or resumed.
    pub thread: Option<String>,
}

/// The codec's state for one session.
#[derive(Debug, Default)]
pub struct Codec {
    next_id: i64,
    sent: HashMap<i64, Sent>,
    /// What the handshake learned.
    pub handshake: Handshake,
    turn: Option<String>,
    turn_running: bool,
    origin: Option<TurnOrigin>,
    next_model: Option<String>,
    limits: Option<LimitsSnapshot>,
    last_usage: Option<TurnUsage>,
    open: HashMap<String, (Value, Answering)>,
    book: AskBook,
    /// Messages of a kind the codec doesn't know (ignored, never guessed).
    pub unknown: usize,
}

fn text(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn count(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn usage_of(breakdown: &Value, window: Option<u64>) -> TurnUsage {
    TurnUsage {
        input: count(breakdown, "inputTokens"),
        cached_input: count(breakdown, "cachedInputTokens"),
        output: count(breakdown, "outputTokens"),
        reasoning: count(breakdown, "reasoningOutputTokens"),
        context_window: window,
    }
}

/// A `RateLimitSnapshot` as the windows it reports.
#[must_use]
pub fn limits_of(snapshot: &Value) -> LimitsSnapshot {
    let window = |key: &str| {
        let w = snapshot.get(key).filter(|w| !w.is_null())?;
        Some(LimitWindow {
            id: key.to_owned(),
            window_mins: w
                .get("windowDurationMins")
                .and_then(Value::as_u64)
                .and_then(|m| u32::try_from(m).ok()),
            used_pct: w
                .get("usedPercent")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .clamp(0.0, 100.0),
            // app-server's `resetsAt` is epoch seconds.
            resets_at_ms: w
                .get("resetsAt")
                .and_then(Value::as_i64)
                .map(|s| s.saturating_mul(1000)),
        })
    };
    LimitsSnapshot {
        windows: ["primary", "secondary"]
            .iter()
            .filter_map(|key| window(key))
            .collect(),
        plan: text(snapshot, "/planType"),
        reached: snapshot
            .get("rateLimitReachedType")
            .is_some_and(|v| !v.is_null()),
    }
}

/// Codex's error info as a category (ADR-0003 point 3).
fn category(error: &Value) -> ErrorCategory {
    let info = error.get("codexErrorInfo").unwrap_or(&Value::Null);
    let word = info
        .as_str()
        .map(str::to_owned)
        .or_else(|| info.as_object().and_then(|o| o.keys().next().cloned()))
        .unwrap_or_default();
    match word.as_str() {
        "usageLimitExceeded" | "rateLimitExceeded" => ErrorCategory::RateLimit,
        "sessionBudgetExceeded" => ErrorCategory::Billing,
        "unauthorized" => ErrorCategory::SignIn,
        "internalServerError" | "serverOverloaded" => ErrorCategory::Crash,
        _ => ErrorCategory::Other,
    }
}

fn item(value: &Value) -> Option<ItemSummary> {
    let kind = text(value, "/type")?;
    let (kind, summary) = match kind.as_str() {
        "userMessage" => (
            ItemKind::UserMessage,
            value
                .get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|c| text(c, "/text"))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        "agentMessage" => (ItemKind::AgentMessage, text(value, "/text")?),
        "commandExecution" => (ItemKind::Command, text(value, "/command")?),
        "fileChange" => (
            ItemKind::FileChange,
            value
                .get("changes")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|c| text(c, "/path"))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        "mcpToolCall" => (
            ItemKind::Tool,
            format!(
                "{}.{}",
                text(value, "/server").unwrap_or_default(),
                text(value, "/tool").unwrap_or_default()
            ),
        ),
        "webSearch" => (ItemKind::Tool, text(value, "/query")?),
        _ => return None,
    };
    Some(ItemSummary {
        kind,
        text: bounded(&summary),
    })
}

fn state(status: LiveStatusWord) -> RuntimeEvent {
    RuntimeEvent::State { status }
}

/// `thread/status/changed` as a state (or a crash for `systemError`).
fn status_changed(params: &Value) -> Vec<RuntimeEvent> {
    match text(params, "/status/type").as_deref() {
        Some("active") => {
            let flags = params
                .pointer("/status/activeFlags")
                .and_then(Value::as_array)
                .is_some_and(|f| !f.is_empty());
            vec![state(if flags {
                LiveStatusWord::Waiting
            } else {
                LiveStatusWord::Busy
            })]
        }
        Some("idle") => vec![state(LiveStatusWord::Idle)],
        Some("systemError") => vec![
            RuntimeEvent::Error {
                category: ErrorCategory::Crash,
                message: "Codex reported a system error".into(),
                will_retry: false,
            },
            state(LiveStatusWord::Idle),
        ],
        _ => Vec::new(),
    }
}

impl Codec {
    /// A request line, remembering what it was for.
    pub fn request(&mut self, method: &str, params: Value, sent: Sent) -> Value {
        self.next_id += 1;
        self.sent.insert(self.next_id, sent);
        rpc::request(self.next_id, method, params)
    }

    /// The `initialize` request: AI Juggler named honestly, stable methods
    /// only.
    pub fn initialize(&mut self) -> Value {
        let params = json!({
            "clientInfo": {
                "name": super::CLIENT_NAME,
                "title": super::CLIENT_TITLE,
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": {"experimentalApi": false},
        });
        self.request("initialize", params, Sent::Initialize)
    }

    /// `thread/start` in `cwd`, or `thread/resume` of `resume`; the model
    /// only when the profile names one (else Codex's own config decides).
    pub fn thread(&mut self, cwd: &str, resume: Option<&str>, model: Option<&str>) -> Value {
        let mut params = json!({ "cwd": cwd });
        if let Some(model) = model {
            params["model"] = json!(model);
        }
        match resume {
            Some(id) => {
                params["threadId"] = json!(id);
                self.request("thread/resume", params, Sent::Thread)
            }
            None => self.request("thread/start", params, Sent::Thread),
        }
    }

    fn thread_id(&self) -> Result<String, AdapterError> {
        self.handshake
            .thread
            .clone()
            .ok_or_else(|| AdapterError::InvalidLaunch("the conversation hasn't started".into()))
    }

    fn input(input: &TurnInput) -> Value {
        json!([{"type": "text", "text": input.text, "text_elements": []}])
    }

    /// `turn/start` (only between turns).
    ///
    /// # Errors
    ///
    /// While a turn runs, or before the thread exists.
    pub fn turn_start(&mut self, input: TurnInput) -> Result<Value, AdapterError> {
        if self.turn_running {
            return Err(AdapterError::InvalidLaunch(
                "a turn is running: it can start when this one ends".into(),
            ));
        }
        let mut params = json!({"threadId": self.thread_id()?, "input": Self::input(&input)});
        if let Some(model) = self.next_model.take() {
            params["model"] = json!(model);
        }
        self.origin = Some(input.origin);
        Ok(self.request("turn/start", params, Sent::TurnStart))
    }

    /// `turn/steer` into the running turn.
    ///
    /// # Errors
    ///
    /// When no turn runs.
    pub fn steer(&mut self, input: &TurnInput) -> Result<Value, AdapterError> {
        let turn = self
            .turn
            .clone()
            .filter(|_| self.turn_running)
            .ok_or_else(|| AdapterError::InvalidLaunch("no turn is running".into()))?;
        let params = json!({"threadId": self.thread_id()?, "expectedTurnId": turn,
            "input": Self::input(input)});
        Ok(self.request("turn/steer", params, Sent::Steer))
    }

    /// `turn/interrupt`, when a turn runs.
    pub fn interrupt(&mut self) -> Option<Value> {
        let turn = self.turn.clone().filter(|_| self.turn_running)?;
        let thread = self.handshake.thread.clone()?;
        Some(self.request(
            "turn/interrupt",
            json!({"threadId": thread, "turnId": turn}),
            Sent::Interrupt,
        ))
    }

    /// `thread/compact/start`.
    ///
    /// # Errors
    ///
    /// Before the thread exists.
    pub fn compact(&mut self) -> Result<Value, AdapterError> {
        let params = json!({"threadId": self.thread_id()?});
        Ok(self.request("thread/compact/start", params, Sent::Compact))
    }

    /// `model/list` (the answer is [`RuntimeEvent::Models`]).
    pub fn models(&mut self) -> Value {
        self.request("model/list", json!({}), Sent::Models)
    }

    /// `account/rateLimits/read` (the answer is
    /// [`RuntimeEvent::LimitsUpdated`]).
    pub fn rate_limits(&mut self) -> Value {
        self.request("account/rateLimits/read", Value::Null, Sent::RateLimits)
    }

    /// Notes the model for the next turn (Codex takes it per turn).
    pub fn set_model(&mut self, model: &str) -> RuntimeEvent {
        self.next_model = Some(model.to_owned());
        RuntimeEvent::ModelChanged {
            model: model.to_owned(),
        }
    }

    /// The last limits Codex reported.
    #[must_use]
    pub fn limits(&self) -> Option<LimitsSnapshot> {
        self.limits.clone()
    }

    /// Whether a turn runs.
    #[must_use]
    pub fn turn_running(&self) -> bool {
        self.turn_running
    }

    /// The response line carrying the user's answer to an open ask, and
    /// the events it makes (the ask is closed at once, so it is never
    /// answered twice).
    ///
    /// # Errors
    ///
    /// For an ask that isn't open, or an answer it doesn't take.
    pub fn answer(
        &mut self,
        ask: &AskId,
        answer: &AskAnswer,
    ) -> Result<(Value, Vec<RuntimeEvent>), AdapterError> {
        self.book.check(ask, answer)?;
        let (rpc_id, how) = self.open.get(ask.as_str()).ok_or_else(|| {
            AdapterError::InvalidLaunch("that question isn't open any more".into())
        })?;
        let line = rpc::answer(rpc_id, response(how, answer)?);
        self.open.remove(ask.as_str());
        let events = vec![
            RuntimeEvent::AskResolved { id: ask.clone() },
            state(LiveStatusWord::Busy),
        ];
        for event in &events {
            self.book.note(event);
        }
        Ok((line, events))
    }

    /// One message from the server: the events it makes, and any line to
    /// send back at once (a refusal of a request AI Juggler doesn't handle;
    /// never an answer to an ask).
    pub fn incoming(&mut self, message: Incoming) -> (Vec<RuntimeEvent>, Vec<Value>) {
        let events = match message {
            Incoming::Response { id, result } => self.response(id, result),
            Incoming::Request { id, method, params } => {
                let ask = AskId(match &id {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                });
                let Some((event, how)) = ask_of(&ask, &method, &params) else {
                    self.unknown += 1;
                    let refusal = rpc::refusal(&id, "AI Juggler doesn't handle this request");
                    return (Vec::new(), vec![refusal]);
                };
                self.open.insert(ask.0.clone(), (id, how));
                vec![event, state(LiveStatusWord::Waiting)]
            }
            Incoming::Notification { method, params } => self.notification(&method, &params),
        };
        for event in &events {
            self.book.note(event);
            if let RuntimeEvent::AskResolved { id } = event {
                self.open.remove(id.as_str());
            }
            if matches!(event, RuntimeEvent::TurnEnded { .. }) {
                self.open.clear();
            }
        }
        (events, Vec::new())
    }

    fn response(&mut self, id: i64, result: Result<Value, String>) -> Vec<RuntimeEvent> {
        let Some(sent) = self.sent.remove(&id) else {
            self.unknown += 1;
            return Vec::new();
        };
        let value = match result {
            Ok(value) => value,
            Err(message) => {
                return vec![RuntimeEvent::Error {
                    category: ErrorCategory::Other,
                    message: bounded(&message),
                    will_retry: false,
                }];
            }
        };
        match sent {
            Sent::Initialize => {
                self.handshake.codex_home = text(&value, "/codexHome");
                Vec::new()
            }
            Sent::Thread => {
                let Some(thread) = text(&value, "/thread/id") else {
                    return Vec::new();
                };
                self.handshake.thread = Some(thread.clone());
                vec![
                    RuntimeEvent::Started {
                        native_id: thread,
                        model: text(&value, "/model"),
                    },
                    state(LiveStatusWord::Idle),
                ]
            }
            Sent::RateLimits => {
                let snapshot = value.get("rateLimits").cloned().unwrap_or(Value::Null);
                let limits = limits_of(&snapshot);
                self.limits = Some(limits.clone());
                vec![RuntimeEvent::LimitsUpdated { limits }]
            }
            Sent::Models => {
                let models = value
                    .get("data")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter(|m| !m.get("hidden").and_then(Value::as_bool).unwrap_or(false))
                    .filter_map(|m| {
                        Some(ModelChoice {
                            id: text(m, "/model").or_else(|| text(m, "/id"))?,
                            name: text(m, "/displayName").unwrap_or_default(),
                            default: m.get("isDefault").and_then(Value::as_bool).unwrap_or(false),
                        })
                    })
                    .collect();
                vec![RuntimeEvent::Models { models }]
            }
            Sent::TurnStart | Sent::Steer | Sent::Interrupt | Sent::Compact => Vec::new(),
        }
    }

    fn turn_completed(&mut self, params: &Value) -> Vec<RuntimeEvent> {
        self.turn_running = false;
        let outcome = match text(params, "/turn/status").as_deref() {
            Some("completed") => TurnEnd::Completed,
            Some("interrupted") => TurnEnd::Interrupted,
            _ => TurnEnd::Failed,
        };
        let mut events = Vec::new();
        if let Some(error) = params.pointer("/turn/error").filter(|e| !e.is_null()) {
            events.push(RuntimeEvent::Error {
                category: category(error),
                message: bounded(&text(error, "/message").unwrap_or_default()),
                will_retry: false,
            });
        }
        events.push(RuntimeEvent::TurnEnded {
            outcome,
            usage: self.last_usage,
        });
        events.push(state(LiveStatusWord::Idle));
        events
    }

    fn notification(&mut self, method: &str, params: &Value) -> Vec<RuntimeEvent> {
        match method {
            "thread/status/changed" => status_changed(params),
            "turn/started" => {
                self.turn = text(params, "/turn/id");
                self.turn_running = true;
                self.last_usage = None;
                vec![
                    RuntimeEvent::TurnStarted {
                        turn: self.turn.clone(),
                        origin: self.origin.take(),
                    },
                    state(LiveStatusWord::Busy),
                ]
            }
            "turn/completed" => self.turn_completed(params),
            "item/agentMessage/delta" => vec![RuntimeEvent::TextDelta {
                text: bounded(&text(params, "/delta").unwrap_or_default()),
            }],
            "item/commandExecution/outputDelta" => vec![RuntimeEvent::ToolDelta {
                item: text(params, "/itemId").unwrap_or_default(),
                text: bounded(&text(params, "/delta").unwrap_or_default()),
            }],
            "item/completed" => params
                .get("item")
                .and_then(item)
                .map(|item| vec![RuntimeEvent::Item { item }])
                .unwrap_or_default(),
            "serverRequest/resolved" => {
                let id = match params.get("requestId") {
                    Some(Value::String(s)) => s.clone(),
                    Some(other) => other.to_string(),
                    None => return Vec::new(),
                };
                if !self.open.contains_key(&id) {
                    return Vec::new();
                }
                vec![
                    RuntimeEvent::AskResolved { id: AskId(id) },
                    state(LiveStatusWord::Busy),
                ]
            }
            "account/rateLimits/updated" => {
                let limits = limits_of(params.get("rateLimits").unwrap_or(&Value::Null));
                self.limits = Some(limits.clone());
                vec![RuntimeEvent::LimitsUpdated { limits }]
            }
            "thread/tokenUsage/updated" => {
                let window = params
                    .pointer("/tokenUsage/modelContextWindow")
                    .and_then(Value::as_u64);
                let last = params.pointer("/tokenUsage/last").unwrap_or(&Value::Null);
                self.last_usage = Some(usage_of(last, window));
                let total = params.pointer("/tokenUsage/total").unwrap_or(&Value::Null);
                vec![RuntimeEvent::Usage {
                    total: usage_of(total, window),
                }]
            }
            "thread/compacted" => vec![RuntimeEvent::Compacted],
            "error" => {
                let error = params.get("error").unwrap_or(&Value::Null);
                vec![RuntimeEvent::Error {
                    category: category(error),
                    message: bounded(&text(error, "/message").unwrap_or_default()),
                    will_retry: params
                        .get("willRetry")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                }]
            }
            "thread/started" => Vec::new(), // the thread response says it
            _ => {
                self.unknown += 1;
                Vec::new()
            }
        }
    }
}
