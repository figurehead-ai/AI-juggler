//! What Muse's session protocol (MSP, schema version 1) messages mean, and
//! the messages AI Juggler sends. Lines in, events and replies out; no I/O.
//!
//! Built from the MIT SDK's schema (`schema/msp/msp.d.ts`) and its golden
//! transcripts (`schema/msp/transcripts/`, read at `07e1548c`), and checked
//! against `muse serve --provider echo` 1.4.13 in a throwaway home.
//!
//! The wire is JSON-RPC 2.0, one message per line, `"jsonrpc":"2.0"` on
//! every frame. Every command carries a `commandId`, a `UUIDv7` the client
//! mints (Muse refuses any other). A `session/start` or `session/resume`
//! subscribes the connection to the session's view by itself (its result's
//! `viewCursor`), so no `view/subscribe` follows; the view's events arrive
//! as notifications (`turn/*`, `item/*`, `approval/*`, `userInput/*`,
//! `usage/changed`).
//!
//! Muse asks AI Juggler two things as requests of its own,
//! `approval/request` and `userInput/request`: each is acknowledged at once
//! (`{}`, as the SDK's transcripts do) and becomes an ask that only the
//! user answers ([`Codec::answer`]), with the choices Muse itself offered
//! (`approval/decide` with one of its `choiceId`s and the requirement it
//! named; `userInput/answer` with the labels it offered or the user's own
//! words). Anything else Muse asks is refused.

use std::collections::{BTreeMap, BTreeSet};

use juggler_adapter::{
    AdapterError,
    runtime::{
        Approval, ApprovalKind, AskAnswer, AskId, AskOption, ErrorCategory, ItemKind, ItemSummary,
        LimitWindow, LimitsSnapshot, LiveStatusWord, ModelChoice, OptionEffect, Question,
        QuestionPart, RuntimeEvent, TurnEnd, TurnInput, TurnUsage,
        book::{AskBook, OpenAsk},
        events::bounded,
        session::{LineAndEvents, LineCodec},
    },
};
use serde_json::{Value, json};

use crate::{
    codex_appserver::rpc::{self, Incoming},
    common::rpc_codec::{Outbox, choices, error, text, waiting},
};

/// MSP's methods and notifications AI Juggler uses.
pub mod methods {
    /// The handshake.
    pub const INITIALIZE: &str = "initialize";
    /// The handshake's end (a notification).
    pub const INITIALIZED: &str = "initialized";
    /// A new session.
    pub const START: &str = "session/start";
    /// A stored session.
    pub const RESUME: &str = "session/resume";
    /// The stored sessions.
    pub const LIST: &str = "session/list";
    /// One stored session.
    pub const READ: &str = "session/read";
    /// A turn.
    pub const TURN: &str = "turn/start";
    /// More input into the running turn.
    pub const STEER: &str = "turn/steer";
    /// Stops the running turn.
    pub const INTERRUPT: &str = "turn/interrupt";
    /// The model for the next turns.
    pub const SET_MODEL: &str = "session/setModel";
    /// The models offered.
    pub const MODELS: &str = "model/list";
    /// The plan's windows as last seen (stable, no network).
    pub const USAGE_READ: &str = "usage/read";
    /// One fresh read of them (experimental; a key mint, at most once a
    /// minute per host).
    pub const USAGE_REFRESH: &str = "usage/refresh";
    /// Which sign-in lane is in effect (experimental; never key material).
    pub const ACCOUNT_READ: &str = "account/read";
    /// The user's decision on an approval.
    pub const DECIDE: &str = "approval/decide";
    /// The user's answers to a question.
    pub const ANSWER: &str = "userInput/answer";
    /// Muse asks for an approval (a request).
    pub const APPROVAL: &str = "approval/request";
    /// The same, as a view event.
    pub const APPROVAL_SEEN: &str = "approval/requested";
    /// An approval decided (by anyone).
    pub const APPROVAL_DONE: &str = "approval/resolved";
    /// Muse asks the user something (a request).
    pub const QUESTION: &str = "userInput/request";
    /// The same, as a view event.
    pub const QUESTION_SEEN: &str = "userInput/requested";
    /// A question settled.
    pub const QUESTION_DONE: &str = "userInput/settled";
    /// The plan's windows changed.
    pub const USAGE_CHANGED: &str = "usage/changed";
}

/// `clientInfo.name`: how AI Juggler names itself to Muse
/// (`[a-z0-9_]+`), never another client's name.
pub const CLIENT_NAME: &str = "ai_juggler";

/// A `UUIDv7`: the time in its first 48 bits, the rest random (MSP's
/// `commandId`).
#[must_use]
pub fn uuid_v7() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    let mut random = [0u8; 10];
    // No random source leaves zeros: the time still makes it unique enough
    // for one connection's commands, which also carry a counter below.
    let _ = getrandom::fill(&mut random);
    let mut bytes = [0u8; 16];
    bytes[..6].copy_from_slice(&ms.to_be_bytes()[10..16]);
    bytes[6..].copy_from_slice(&random);
    bytes[6] = 0x70 | (bytes[6] & 0x0f);
    bytes[8] = 0x80 | (bytes[8] & 0x3f);
    let hex = bytes.iter().fold(String::new(), |mut hex, byte| {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
        hex
    });
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// A frame with `"jsonrpc":"2.0"` (MSP sends and expects it on every one).
fn framed(mut line: Value) -> Value {
    line["jsonrpc"] = json!("2.0");
    line
}

/// The `initialize` request: AI Juggler's name, and the experimental API
/// only when asked for (`account/read`, `usage/refresh`).
#[must_use]
pub fn initialize(id: i64, experimental: bool) -> Value {
    let mut params = json!({
        "clientInfo": {
            "name": CLIENT_NAME,
            "title": "AI Juggler",
            "version": env!("CARGO_PKG_VERSION"),
        },
    });
    if experimental {
        params["capabilities"] = json!({"experimentalApi": true});
    }
    framed(rpc::request(id, methods::INITIALIZE, params))
}

/// The `initialized` notification.
#[must_use]
pub fn initialized() -> Value {
    framed(rpc::notification(methods::INITIALIZED))
}

/// A request with no parameters of its own beyond `params`.
#[must_use]
pub fn request(id: i64, method: &str, params: Value) -> Value {
    framed(rpc::request(id, method, params))
}

/// The plan's windows in a `usage/read` or `usage/refresh` result (or a
/// `usage/changed` event's parameters): the current window, named by its
/// length, and the week. `None` when Muse observed none yet. A share over
/// 100 is kept as Muse reports it.
#[must_use]
pub fn limits_of(usage: &Value, plan: Option<String>) -> Option<LimitsSnapshot> {
    let usage = usage.get("usage").unwrap_or(usage);
    let pct = |value: &Value| value.get("usedPercent").and_then(Value::as_f64);
    let resets = |value: &Value| value.get("resetsAtMs").and_then(Value::as_i64);
    let mut windows = Vec::new();
    if let Some(window) = usage.get("window")
        && let Some(used_pct) = pct(window)
    {
        let mins = window.get("windowDurationMins").and_then(Value::as_u64);
        windows.push(LimitWindow {
            id: crate::codex_appserver::codec::window_id(mins),
            window_mins: mins.and_then(|mins| u32::try_from(mins).ok()),
            used_pct,
            resets_at_ms: resets(window),
        });
    }
    if let Some(weekly) = usage.get("weekly")
        && let Some(used_pct) = pct(weekly)
    {
        windows.push(LimitWindow {
            id: crate::codex_appserver::codec::window_id(Some(10_080)),
            window_mins: Some(10_080),
            used_pct,
            resets_at_ms: resets(weekly),
        });
    }
    (!windows.is_empty()).then(|| LimitsSnapshot {
        reached: windows.iter().any(|window| window.used_pct >= 100.0),
        windows,
        plan,
    })
}

/// A turn's input: the text as one part.
fn input(text: &str) -> Value {
    json!([{"type": "text", "text": text}])
}

/// What a request of AI Juggler's was for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asked {
    Initialize,
    Open,
    Turn,
    Models,
    Usage,
    Other,
}

/// An open approval: the requirement its decision must name.
#[derive(Clone, Debug)]
struct Pending {
    requirement: Value,
}

/// The handshake's state.
#[derive(Clone, Debug, Default)]
pub struct Handshake {
    /// Muse's home for its sessions (`museHome`), once it answered
    /// `initialize`.
    pub muse_home: Option<String>,
    /// The session's id, once Muse opened it.
    pub session: Option<String>,
}

/// The codec's state.
#[derive(Debug, Default)]
pub struct Codec {
    outbox: Outbox<Asked>,
    /// The handshake.
    pub handshake: Handshake,
    /// The model a new session starts with.
    pub model: Option<String>,
    /// MCP servers for this session only (`session/start`'s
    /// `config.mcpServers`, MSP's `SessionMcpServerConfig` by name).
    pub mcp_servers: BTreeMap<String, Value>,
    book: AskBook,
    approvals: BTreeMap<String, Pending>,
    questions: BTreeSet<String>,
    /// Asks answered or withdrawn: a re-announcement isn't asked again.
    closed: BTreeSet<String>,
    running: bool,
    turn: Option<String>,
    /// Agent messages whose text came in deltas.
    streamed: BTreeSet<String>,
    limits: Option<LimitsSnapshot>,
    /// Messages that weren't understood (counted, never kept).
    pub unknown: usize,
}

impl Codec {
    fn request(&mut self, method: &str, params: Value, what: Asked) -> Value {
        request(self.outbox.note(what), method, params)
    }

    /// `initialize`, stable API only (a driven session needs nothing
    /// experimental).
    pub fn initialize(&mut self) -> Value {
        initialize(self.outbox.note(Asked::Initialize), false)
    }

    /// `session/start` in `cwd`, with the codec's model and per-session
    /// MCP servers. Muse names the session.
    pub fn start(&mut self, cwd: &str) -> Value {
        let mut params = json!({"commandId": uuid_v7(), "workspaceRoot": cwd});
        if let Some(model) = &self.model {
            params["modelId"] = json!(model);
        }
        if !self.mcp_servers.is_empty() {
            params["config"] = json!({"mcpServers": self.mcp_servers});
        }
        self.request(methods::START, params, Asked::Open)
    }

    /// `session/resume` of a stored session, without its history (the
    /// feed starts from now).
    pub fn resume(&mut self, id: &str) -> Value {
        self.handshake.session = None;
        let mut params = json!({"commandId": uuid_v7(), "sessionId": id, "excludeItems": true});
        if !self.mcp_servers.is_empty() {
            params["config"] = json!({"mcpServers": self.mcp_servers});
        }
        self.request(methods::RESUME, params, Asked::Open)
    }

    fn session_id(&self) -> Result<String, AdapterError> {
        self.handshake
            .session
            .clone()
            .ok_or_else(|| AdapterError::InvalidLaunch("Muse's session isn't open".into()))
    }

    /// `turn/start`, only between turns.
    ///
    /// # Errors
    ///
    /// While a turn runs, or before the session is open.
    pub fn send(&mut self, text: &str) -> Result<Value, AdapterError> {
        if self.running {
            return Err(AdapterError::InvalidLaunch(
                "Muse is still answering: wait for its turn to end".into(),
            ));
        }
        let id = self.session_id()?;
        self.running = true;
        Ok(self.request(
            methods::TURN,
            json!({"sessionId": id, "commandId": uuid_v7(), "input": input(text)}),
            Asked::Turn,
        ))
    }

    /// `turn/steer` into the running turn (the turn it believes runs, so
    /// input never lands in the next one).
    ///
    /// # Errors
    ///
    /// Between turns.
    pub fn steer_line(&mut self, text: &str) -> Result<Value, AdapterError> {
        let (Some(turn), true) = (self.turn.clone(), self.running) else {
            return Err(AdapterError::InvalidLaunch(
                "Muse isn't answering now: send it as a new message".into(),
            ));
        };
        let id = self.session_id()?;
        Ok(self.request(
            methods::STEER,
            json!({"sessionId": id, "commandId": uuid_v7(), "expectedTurnId": turn, "input": input(text)}),
            Asked::Other,
        ))
    }

    /// What Muse's answer to a request of `what` kind means.
    fn response(&mut self, what: Asked, result: Result<Value, String>) -> Vec<RuntimeEvent> {
        match (what, result) {
            (Asked::Initialize, Ok(value)) => {
                self.handshake.muse_home = Some(
                    value
                        .get("museHome")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                );
                Vec::new()
            }
            (Asked::Open, Ok(value)) => {
                let session = value.get("session").cloned().unwrap_or(Value::Null);
                let Some(id) = session.get("sessionId").and_then(Value::as_str) else {
                    return vec![error(ErrorCategory::Other, "Muse opened no session")];
                };
                self.handshake.session = Some(id.to_owned());
                self.turn = session
                    .get("activeTurnId")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                self.running = self.turn.is_some();
                vec![RuntimeEvent::Started {
                    native_id: id.to_owned(),
                    model: session
                        .get("modelId")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    path: session
                        .get("path")
                        .and_then(Value::as_str)
                        .filter(|path| !path.is_empty())
                        .map(str::to_owned),
                }]
            }
            (Asked::Turn, Ok(value)) => {
                self.turn = value
                    .get("turnId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .or_else(|| self.turn.clone());
                Vec::new()
            }
            (Asked::Turn, Err(message)) => {
                self.running = false;
                vec![
                    error(categorize("", &message), &message),
                    RuntimeEvent::State {
                        status: LiveStatusWord::Idle,
                    },
                ]
            }
            (Asked::Models, Ok(value)) => vec![RuntimeEvent::Models {
                models: models_of(&value),
            }],
            (Asked::Usage, Ok(value)) => self.usage(&value),
            (_, Ok(_)) => Vec::new(),
            (_, Err(message)) => vec![error(categorize("", &message), &message)],
        }
    }

    fn usage(&mut self, value: &Value) -> Vec<RuntimeEvent> {
        let plan = value
            .get("tierName")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| self.limits.as_ref().and_then(|limits| limits.plan.clone()));
        match limits_of(value, plan) {
            Some(limits) => {
                self.limits = Some(limits.clone());
                vec![RuntimeEvent::LimitsUpdated { limits }]
            }
            None => Vec::new(),
        }
    }

    /// Muse's request: acknowledged at once; an approval or a question
    /// becomes the user's ask.
    fn muse_request(
        &mut self,
        id: &Value,
        method: &str,
        params: &Value,
    ) -> (Vec<RuntimeEvent>, Vec<Value>) {
        let ack = framed(rpc::answer(id, json!({})));
        match method {
            methods::APPROVAL => (self.approval(params), vec![ack]),
            methods::QUESTION => (self.question(params), vec![ack]),
            _ => (
                Vec::new(),
                vec![framed(rpc::refusal(id, "AI Juggler doesn't answer this"))],
            ),
        }
    }

    fn ours(&self, params: &Value) -> bool {
        let ours = self.handshake.session.as_deref();
        ours.is_some() && params.get("sessionId").and_then(Value::as_str) == ours
    }

    fn approval(&mut self, params: &Value) -> Vec<RuntimeEvent> {
        let Some(ask) = params.get("approvalId").and_then(Value::as_str) else {
            return Vec::new();
        };
        if !self.ours(params) || self.closed.contains(ask) {
            return Vec::new();
        }
        let requirement = params
            .get("currentRequirementId")
            .cloned()
            .unwrap_or_else(|| json!({"approvalId": ask, "sourceIndex": 0}));
        if let Some(pending) = self.approvals.get_mut(ask) {
            // Asked again (a re-send, or a later stage): the latest
            // requirement is the one a decision names.
            pending.requirement = requirement;
            return Vec::new();
        }
        let tool = text(params, "toolName");
        let subject = params.get("subject").cloned().unwrap_or(Value::Null);
        let options: Vec<AskOption> = params
            .get("availableChoices")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(choice_of)
            .collect();
        if options.is_empty() {
            return vec![error(
                ErrorCategory::Other,
                &format!("Muse asked to use {tool} with no choices AI Juggler can show"),
            )];
        }
        self.approvals
            .insert(ask.to_owned(), Pending { requirement });
        waiting(RuntimeEvent::ApprovalRequested {
            approval: Approval {
                id: AskId(ask.to_owned()),
                kind: approval_kind(&subject),
                summary: bounded(&summary(&tool, &subject)),
                reason: params
                    .get("reviewerReason")
                    .and_then(Value::as_str)
                    .filter(|reason| !reason.is_empty())
                    .map(bounded),
                options,
            },
        })
    }

    fn question(&mut self, params: &Value) -> Vec<RuntimeEvent> {
        let Some(ask) = params.get("userInputId").and_then(Value::as_str) else {
            return Vec::new();
        };
        if !self.ours(params) || self.closed.contains(ask) || self.questions.contains(ask) {
            return Vec::new();
        }
        let parts: Vec<QuestionPart> = params
            .get("questions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|part| {
                Some(QuestionPart {
                    id: part.get("id").and_then(Value::as_str)?.to_owned(),
                    header: bounded(&text(part, "header")),
                    question: bounded(&text(part, "question")),
                    choices: choices(part),
                    multi: part.pointer("/selection/mode").and_then(Value::as_str)
                        == Some("multiple"),
                    free_text: true,
                    secret: false,
                })
            })
            .collect();
        if parts.is_empty() {
            return Vec::new();
        }
        self.questions.insert(ask.to_owned());
        waiting(RuntimeEvent::QuestionAsked {
            question: Question {
                id: AskId(ask.to_owned()),
                parts,
            },
        })
    }

    /// An ask Muse settled by itself (another client, a timeout, the
    /// turn's end).
    fn settled(&mut self, id: Option<&str>) -> Vec<RuntimeEvent> {
        let Some(id) = id else { return Vec::new() };
        let was_open = self.approvals.remove(id).is_some() | self.questions.remove(id);
        if !was_open || !self.closed.insert(id.to_owned()) {
            return Vec::new();
        }
        vec![RuntimeEvent::AskResolved {
            id: AskId(id.to_owned()),
        }]
    }

    fn notification(&mut self, method: &str, params: &Value) -> Vec<RuntimeEvent> {
        if method == methods::USAGE_CHANGED {
            return self.usage(params);
        }
        if !self.ours(params) {
            // Another session's, or a sub-agent's of its own: not this feed.
            return Vec::new();
        }
        match method {
            "turn/started" => {
                self.running = true;
                self.turn = params
                    .get("turnId")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                vec![
                    RuntimeEvent::TurnStarted {
                        turn: self.turn.clone(),
                        origin: None,
                    },
                    RuntimeEvent::State {
                        status: LiveStatusWord::Busy,
                    },
                ]
            }
            "turn/completed" => self.turn_completed(params),
            "item/delta" => {
                let item = text(params, "itemId");
                let delta = text(params, "delta");
                match params.get("field").and_then(Value::as_str) {
                    Some("text") => {
                        self.streamed.insert(item);
                        vec![RuntimeEvent::TextDelta {
                            text: bounded(&delta),
                        }]
                    }
                    Some("output") => vec![RuntimeEvent::ToolDelta {
                        item,
                        text: bounded(&delta),
                    }],
                    _ => Vec::new(),
                }
            }
            "item/started" => {
                let item = params.get("item").cloned().unwrap_or(Value::Null);
                match item.get("kind").and_then(Value::as_str) {
                    Some("toolCall") => vec![RuntimeEvent::ItemStarted {
                        item: text(&item, "itemId"),
                        summary: tool_summary(&item),
                    }],
                    _ => Vec::new(),
                }
            }
            "item/completed" => {
                let item = params.get("item").cloned().unwrap_or(Value::Null);
                match item.get("kind").and_then(Value::as_str) {
                    Some("toolCall") => vec![RuntimeEvent::Item {
                        item: tool_summary(&item),
                    }],
                    // Text that came whole, not in deltas, is shown whole.
                    Some("agentMessage") if !self.streamed.remove(&text(&item, "itemId")) => {
                        let said = text(&item, "text");
                        if said.is_empty() {
                            Vec::new()
                        } else {
                            vec![RuntimeEvent::TextDelta {
                                text: bounded(&said),
                            }]
                        }
                    }
                    _ => Vec::new(),
                }
            }
            methods::APPROVAL_SEEN => self.approval(params),
            methods::QUESTION_SEEN => self.question(params),
            methods::APPROVAL_DONE => {
                self.settled(params.get("approvalId").and_then(Value::as_str))
            }
            methods::QUESTION_DONE => {
                self.settled(params.get("userInputId").and_then(Value::as_str))
            }
            "session/modelChanged" => params
                .get("modelId")
                .or_else(|| params.pointer("/model/modelId"))
                .and_then(Value::as_str)
                .map(|model| {
                    vec![RuntimeEvent::ModelChanged {
                        model: model.to_owned(),
                    }]
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn turn_completed(&mut self, params: &Value) -> Vec<RuntimeEvent> {
        self.running = false;
        self.turn = None;
        self.streamed.clear();
        let usage = params.get("usage").map(|usage| {
            let count = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
            TurnUsage {
                input: count("inputTokens"),
                cached_input: count("cachedTokens"),
                output: count("outputTokens"),
                reasoning: count("reasoningTokens"),
                context_window: None,
            }
        });
        let mut events = Vec::new();
        let outcome = match params.get("terminal").and_then(Value::as_str) {
            Some("completed") => TurnEnd::Completed,
            Some("cancelled") => TurnEnd::Interrupted,
            _ => {
                let failure = params.get("error").cloned().unwrap_or(Value::Null);
                let message = failure
                    .get("message")
                    .and_then(Value::as_str)
                    .or_else(|| params.get("reason").and_then(Value::as_str))
                    .unwrap_or("Muse's turn failed")
                    .to_owned();
                let category = if failure.get("quota").is_some() {
                    ErrorCategory::RateLimit
                } else {
                    categorize(&text(&failure, "kind"), &message)
                };
                let words = if category == ErrorCategory::SignIn {
                    format!("Muse couldn't use this account's sign-in: {message}")
                } else {
                    message
                };
                events.push(error(category, &words));
                TurnEnd::Failed
            }
        };
        events.push(RuntimeEvent::TurnEnded { outcome, usage });
        events.push(RuntimeEvent::State {
            status: LiveStatusWord::Idle,
        });
        events
    }

    /// What one message means: events for the session, and replies.
    pub fn incoming(&mut self, message: Incoming) -> (Vec<RuntimeEvent>, Vec<Value>) {
        let (events, replies) = match message {
            Incoming::Response { id, result } => {
                let what = self.outbox.answered(id).unwrap_or(Asked::Other);
                (self.response(what, result), Vec::new())
            }
            Incoming::Request { id, method, params } => self.muse_request(&id, &method, &params),
            Incoming::Notification { method, params } => {
                (self.notification(&method, &params), Vec::new())
            }
        };
        for event in &events {
            self.book.note(event);
        }
        if events
            .iter()
            .any(|event| matches!(event, RuntimeEvent::TurnEnded { .. }))
        {
            self.approvals.clear();
            self.questions.clear();
        }
        (events, replies)
    }
}

impl LineCodec for Codec {
    /// `turn/start` with the input's text.
    fn turn(&mut self, input: TurnInput) -> Result<LineAndEvents, AdapterError> {
        Ok((self.send(&input.text)?, Vec::new()))
    }

    /// One MSP frame: a response, a request of Muse's, or an event.
    fn take_line(&mut self, line: &str) -> (Vec<RuntimeEvent>, Vec<Value>) {
        rpc::message(line, &mut self.unknown).map_or_else(Default::default, |m| self.incoming(m))
    }

    /// `turn/interrupt` of the running turn.
    fn stop(&mut self) -> Option<Value> {
        let id = self.handshake.session.clone()?;
        if !self.running {
            return None;
        }
        let mut params = json!({"sessionId": id, "commandId": uuid_v7()});
        if let Some(turn) = &self.turn {
            params["turnId"] = json!(turn);
        }
        Some(self.request(methods::INTERRUPT, params, Asked::Other))
    }

    /// The user's answer to an open ask: one of the choices Muse offered,
    /// or the question's answers.
    ///
    /// # Errors
    ///
    /// An ask that isn't open, or an answer that doesn't fit it.
    fn answer(&mut self, ask: &AskId, answer: &AskAnswer) -> Result<LineAndEvents, AdapterError> {
        let open = self.book.check(ask, answer)?.clone();
        let session = self.session_id()?;
        let line = match (&open, answer) {
            (OpenAsk::Approval(_), AskAnswer::Choose { option }) => {
                let pending = self.approvals.remove(ask.as_str()).ok_or_else(|| {
                    AdapterError::InvalidLaunch("that question isn't open any more".into())
                })?;
                self.request(
                    methods::DECIDE,
                    json!({
                        "sessionId": session,
                        "commandId": uuid_v7(),
                        "approvalId": ask.as_str(),
                        "requirementId": pending.requirement,
                        "choiceId": option,
                    }),
                    Asked::Other,
                )
            }
            (OpenAsk::Question(question), AskAnswer::Reply { parts }) => {
                self.questions.remove(ask.as_str());
                let answers: Vec<Value> = parts
                    .iter()
                    .map(|part| {
                        let asked = question.parts.iter().find(|known| known.id == part.part);
                        answer_of(&part.part, asked, &part.answers)
                    })
                    .collect();
                self.request(
                    methods::ANSWER,
                    json!({
                        "sessionId": session,
                        "commandId": uuid_v7(),
                        "userInputId": ask.as_str(),
                        "answers": answers,
                    }),
                    Asked::Other,
                )
            }
            _ => {
                return Err(AdapterError::InvalidLaunch(
                    "that answer doesn't fit this ask".into(),
                ));
            }
        };
        self.closed.insert(ask.as_str().to_owned());
        let resolved = RuntimeEvent::AskResolved { id: ask.clone() };
        self.book.note(&resolved);
        Ok((line, vec![resolved]))
    }

    /// `session/setModel`: applies from the next model call.
    fn set_model(&mut self, model: &str) -> Result<(Option<Value>, RuntimeEvent), AdapterError> {
        let id = self.session_id()?;
        let line = self.request(
            methods::SET_MODEL,
            json!({"sessionId": id, "commandId": uuid_v7(), "model": {"modelId": model}}),
            Asked::Other,
        );
        Ok((
            Some(line),
            RuntimeEvent::ModelChanged {
                model: model.to_owned(),
            },
        ))
    }

    fn steer(&mut self, input: &TurnInput) -> Result<Value, AdapterError> {
        self.steer_line(&input.text)
    }

    /// `model/list`: the session's models once it is open, else the
    /// host's.
    fn models(&mut self) -> Result<Value, AdapterError> {
        let params = self
            .handshake
            .session
            .as_ref()
            .map_or_else(|| json!({}), |id| json!({"sessionId": id}));
        Ok(self.request(methods::MODELS, params, Asked::Models))
    }

    /// `usage/read`: the windows Muse last saw, no network.
    fn limits_request(&mut self) -> Option<Value> {
        Some(self.request(methods::USAGE_READ, json!({}), Asked::Usage))
    }

    fn limits(&self) -> Option<LimitsSnapshot> {
        self.limits.clone()
    }

    /// The session Muse named.
    fn native_id(&self) -> Option<String> {
        self.handshake.session.clone()
    }

    fn turn_running(&self) -> bool {
        self.running
    }
}

/// One of Muse's approval choices as an option, by the decision it applies
/// (a choice whose decision isn't known isn't offered).
fn choice_of(choice: &Value) -> Option<AskOption> {
    let id = choice.get("choiceId").and_then(Value::as_str)?;
    let label = text(choice, "label");
    let effect = match choice.get("decision").and_then(Value::as_str)? {
        "approved" => OptionEffect::AllowOnce,
        "approvedForSession" | "approvedPolicyAmendment" => OptionEffect::AllowRule {
            rule: bounded(
                choice
                    .get("rulePreview")
                    .and_then(Value::as_str)
                    .unwrap_or(&label),
            ),
        },
        "denied" | "deniedPolicyAmendment" => OptionEffect::Deny,
        "abort" => OptionEffect::DenyAndStop,
        _ => return None,
    };
    Some(AskOption {
        id: id.to_owned(),
        label: bounded(&label),
        effect,
    })
}

/// One answer as `userInput/answer` takes it: an offered label (one, or
/// several for a multiple choice), else the user's own words.
fn answer_of(part: &str, asked: Option<&QuestionPart>, answers: &[String]) -> Value {
    let offered = |answer: &String| {
        asked.is_some_and(|asked| asked.choices.iter().any(|choice| &choice.label == answer))
    };
    let multi = asked.is_some_and(|asked| asked.multi);
    match answers {
        [one] if offered(one) && !multi => json!({"questionId": part, "selectedLabel": one}),
        many if multi && !many.is_empty() && many.iter().all(offered) => {
            json!({"questionId": part, "selectedLabels": many})
        }
        words => {
            let text: String = words.join(", ").chars().take(500).collect();
            json!({"questionId": part, "freeText": text})
        }
    }
}

fn models_of(value: &Value) -> Vec<ModelChoice> {
    value
        .get("models")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|model| {
            let id = model.get("modelId").and_then(Value::as_str)?;
            Some(ModelChoice {
                id: id.to_owned(),
                name: model
                    .get("displayLabel")
                    .and_then(Value::as_str)
                    .unwrap_or(id)
                    .to_owned(),
                default: model.get("isDefault").and_then(Value::as_bool) == Some(true),
            })
        })
        .collect()
}

/// An error's category: Muse's sign-in failure (`authRequired`), a limit,
/// or anything else.
#[must_use]
pub fn categorize(kind: &str, message: &str) -> ErrorCategory {
    let lower = format!("{kind} {message}").to_lowercase();
    if lower.contains("usage limit") || lower.contains("rate_limit") || lower.contains("quota") {
        ErrorCategory::RateLimit
    } else if lower.contains("authrequired")
        || lower.contains("not logged in")
        || lower.contains("unauthorized")
        || lower.contains("401")
    {
        ErrorCategory::SignIn
    } else {
        ErrorCategory::Other
    }
}

fn approval_kind(subject: &Value) -> ApprovalKind {
    match subject.get("kind").and_then(Value::as_str) {
        Some("shell" | "process") => ApprovalKind::Command,
        Some("fileAccess") if subject.get("access").and_then(Value::as_str) == Some("write") => {
            ApprovalKind::FileChange
        }
        Some("network" | "unixSocket" | "fileAccess") => ApprovalKind::Permissions,
        _ => ApprovalKind::Tool,
    }
}

/// What an approval asks, in one line: its command, path or host, else
/// the tool's name.
fn summary(tool: &str, subject: &Value) -> String {
    ["command", "path", "host"]
        .iter()
        .find_map(|key| subject.get(key)?.as_str())
        .map_or_else(|| tool.to_owned(), |what| format!("{tool}: {what}"))
}

fn tool_summary(item: &Value) -> ItemSummary {
    ItemSummary {
        kind: ItemKind::Tool,
        text: bounded(item.get("tool").and_then(Value::as_str).unwrap_or("a tool")),
    }
}
