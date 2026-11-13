//! What `ZCode`'s app-server messages mean, and the messages AI Juggler
//! sends (engine 0.16.9's own schemas; research `docs/research/zcode.md`
//! § E). Lines in, events and replies out; no I/O here.
//!
//! The wire is Codex's ([`crate::codex_appserver::rpc`]): JSON-RPC without
//! the `jsonrpc` field, one message per line, no `initialize`. The
//! session's events come as `session/event` notifications once
//! `session/subscribe` asked for them; the engine asks AI Juggler three
//! things as requests of its own:
//!
//! - `interaction/requestPermission` and `interaction/requestUserInput`:
//!   asks, answered only by the user ([`Codec::answer`]), with the
//!   options the engine itself offered (each option carries the engine's
//!   own decision, which is what an answer returns);
//! - `interaction/requestProviderRuntimeHeaders`, before every model
//!   call: answered at once with `{headersApplied: true, requestAuth: {}}`
//!   — no credential, ever (ADR-0003), so the engine is left to its own
//!   sign-in in the App's tree. **UNCONFIRMED** that it then uses that
//!   sign-in (the lab check, AJ-E1612); a sign-in failure shows as
//!   "`ZCode` couldn't use this App's sign-in".
//!
//! Anything else the engine asks is refused (an MCP server's sign-in
//! headers, the desktop app's browser).

use std::collections::BTreeMap;

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::book::{AskBook, OpenAsk};
use juggler_adapter::runtime::events::bounded;
use juggler_adapter::runtime::{
    Approval, ApprovalKind, AskAnswer, AskId, AskOption, Choice, ErrorCategory, ImportedHistory,
    ItemKind, ItemSummary, LiveStatusWord, OptionEffect, Question, QuestionPart, RuntimeEvent,
    TurnEnd, TurnInput,
};
use serde_json::{Value, json};

use crate::codex_appserver::rpc::{self, Incoming};

/// The engine's methods AI Juggler calls.
pub mod methods {
    /// Starts a session (or imports one: `importedHistory`).
    pub const CREATE: &str = "session/create";
    /// Opens a stored session.
    pub const RESUME: &str = "session/resume";
    /// Asks for the session's events.
    pub const SUBSCRIBE: &str = "session/subscribe";
    /// Starts a turn.
    pub const SEND: &str = "session/send";
    /// Stops the running turn.
    pub const STOP: &str = "session/stop";
    /// The engine's events.
    pub const EVENT: &str = "session/event";
    /// The engine asks to use a tool.
    pub const PERMISSION: &str = "interaction/requestPermission";
    /// The engine asks the user something.
    pub const USER_INPUT: &str = "interaction/requestUserInput";
    /// The engine asks for a model call's headers.
    pub const HEADERS: &str = "interaction/requestProviderRuntimeHeaders";
}

/// The engine's word for the events a desktop client gets.
const DELIVERY: &str = "desktop-continuous";
/// The answer to a headers request: none applied by AI Juggler, no
/// credential (see the module docs).
#[must_use]
pub fn headers_answer() -> Value {
    json!({"headersApplied": true, "requestAuth": {}})
}

/// What `session/create` imports: the pack's turns as `ZCode`'s own
/// Claude Code import takes them (the only shape it takes a plain chat
/// in; `source` names the import's format, not where the pack came from).
#[must_use]
pub fn imported(history: &ImportedHistory) -> Value {
    let messages: Vec<Value> = history
        .turns
        .iter()
        .filter(|(role, text)| (role == "user" || role == "assistant") && !text.is_empty())
        .map(|(role, text)| json!({"role": role, "content": text}))
        .collect();
    json!({"source": "claudeCode", "title": history.title, "messages": messages})
}

/// The `session/create` parameters: a new session in `cwd`, or, with
/// `history`, that conversation imported under `id` (`sess_<uuid>`;
/// the engine takes a caller's id only for an import).
#[must_use]
pub fn create_params(cwd: &str, id: Option<&str>, history: Option<&ImportedHistory>) -> Value {
    let mut params = json!({
        "workspace": {"workspacePath": cwd, "workspaceKey": cwd},
        "persistence": "immediate",
    });
    if let (Some(id), Some(history)) = (id, history) {
        params["sessionId"] = json!(id);
        params["titleGenerationEnabled"] = json!(false);
        params["importedHistory"] = imported(history);
    }
    params
}

/// What a request of AI Juggler's was for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asked {
    Open,
    Subscribe,
    Send,
    Other,
}

/// An open ask: the engine's request id (what the answer names) and, for
/// an approval, each option's own decision.
#[derive(Clone, Debug)]
struct Pending {
    request: Value,
    decisions: BTreeMap<String, Value>,
    /// Each question's text, by part id (a reply names them).
    questions: BTreeMap<String, String>,
}

/// The handshake's state.
#[derive(Clone, Debug, Default)]
pub struct Handshake {
    /// The session's id, once the engine opened it.
    pub session: Option<String>,
    /// Its events were subscribed to.
    pub subscribed: bool,
}

/// The codec's state.
#[derive(Debug, Default)]
pub struct Codec {
    next: i64,
    asked: BTreeMap<i64, Asked>,
    /// The handshake.
    pub handshake: Handshake,
    book: AskBook,
    pending: BTreeMap<String, Pending>,
    /// The engine's own request ids of open asks, to the ask's id.
    engine_ids: BTreeMap<String, String>,
    /// The engine's request ids of asks answered or withdrawn: a
    /// re-announcement crossing the answer isn't asked again.
    closed: std::collections::BTreeSet<String>,
    running: bool,
    /// Messages that weren't understood (counted, never kept).
    pub unknown: usize,
}

impl Codec {
    fn request(&mut self, method: &str, params: Value, what: Asked) -> Value {
        self.next += 1;
        self.asked.insert(self.next, what);
        rpc::request(self.next, method, params)
    }

    /// `session/create`.
    pub fn create(
        &mut self,
        cwd: &str,
        id: Option<&str>,
        history: Option<&ImportedHistory>,
    ) -> Value {
        self.request(
            methods::CREATE,
            create_params(cwd, id, history),
            Asked::Open,
        )
    }

    /// `session/resume`.
    pub fn resume(&mut self, id: &str, cwd: &str) -> Value {
        self.handshake.session = None;
        self.request(
            methods::RESUME,
            json!({"sessionId": id, "workspace": {"workspacePath": cwd, "workspaceKey": cwd}}),
            Asked::Open,
        )
    }

    /// `session/subscribe` for the open session.
    pub fn subscribe(&mut self) -> Value {
        let id = self.handshake.session.clone().unwrap_or_default();
        self.request(
            methods::SUBSCRIBE,
            json!({"sessionId": id, "deliveryKind": DELIVERY}),
            Asked::Subscribe,
        )
    }

    /// Whether a turn runs.
    #[must_use]
    pub fn turn_running(&self) -> bool {
        self.running
    }

    fn session_id(&self) -> Result<String, AdapterError> {
        self.handshake
            .session
            .clone()
            .ok_or_else(|| AdapterError::InvalidLaunch("ZCode's session isn't open".into()))
    }

    /// `session/send`: a turn, only between turns.
    ///
    /// # Errors
    ///
    /// While a turn runs, or before the session is open.
    pub fn send(&mut self, input: &TurnInput) -> Result<Value, AdapterError> {
        if self.running {
            return Err(AdapterError::InvalidLaunch(
                "ZCode is still answering: wait for its turn to end".into(),
            ));
        }
        let id = self.session_id()?;
        self.running = true;
        Ok(self.request(
            methods::SEND,
            json!({"sessionId": id, "content": input.text}),
            Asked::Send,
        ))
    }

    /// `session/stop`, when a turn runs.
    pub fn stop(&mut self) -> Option<Value> {
        let id = self.handshake.session.clone()?;
        self.running
            .then(|| self.request(methods::STOP, json!({"sessionId": id}), Asked::Other))
    }

    /// The user's answer to an open ask: the engine's own decision for
    /// the chosen option, or the question's answers.
    ///
    /// # Errors
    ///
    /// An ask that isn't open, or an answer that doesn't fit it.
    pub fn answer(
        &mut self,
        ask: &AskId,
        answer: &AskAnswer,
    ) -> Result<(Value, Vec<RuntimeEvent>), AdapterError> {
        let open = self.book.check(ask, answer)?.clone();
        let pending = self.pending.remove(ask.as_str()).ok_or_else(|| {
            AdapterError::InvalidLaunch("that question isn't open any more".into())
        })?;
        let result = match (&open, answer) {
            (OpenAsk::Approval(_), AskAnswer::Choose { option }) => {
                pending.decisions.get(option).cloned().ok_or_else(|| {
                    AdapterError::InvalidLaunch("that option wasn't offered".into())
                })?
            }
            (OpenAsk::Question(_), AskAnswer::Reply { parts }) => {
                let mut answers = serde_json::Map::new();
                for part in parts {
                    if let Some(question) = pending.questions.get(&part.part) {
                        answers.insert(question.clone(), json!(part.answers.join(", ")));
                    }
                }
                json!({"action": "accept", "content": {"answers": answers}})
            }
            _ => {
                return Err(AdapterError::InvalidLaunch(
                    "that answer doesn't fit this ask".into(),
                ));
            }
        };
        self.engine_ids.retain(|_, id| id != ask.as_str());
        self.closed.insert(ask.as_str().to_owned());
        let resolved = RuntimeEvent::AskResolved { id: ask.clone() };
        self.book.note(&resolved);
        Ok((rpc::answer(&pending.request, result), vec![resolved]))
    }

    /// What one message means: events for the session, and replies.
    pub fn incoming(&mut self, message: Incoming) -> (Vec<RuntimeEvent>, Vec<Value>) {
        let (events, replies) = match message {
            Incoming::Response { id, result } => (self.response(id, result), Vec::new()),
            Incoming::Request { id, method, params } => self.engine_request(id, &method, &params),
            Incoming::Notification { method, params } if method == methods::EVENT => {
                (self.event(&params), Vec::new())
            }
            Incoming::Notification { .. } => (Vec::new(), Vec::new()),
        };
        for event in &events {
            self.book.note(event);
        }
        (events, replies)
    }

    fn response(&mut self, id: i64, result: Result<Value, String>) -> Vec<RuntimeEvent> {
        let what = self.asked.remove(&id).unwrap_or(Asked::Other);
        match (what, result) {
            (Asked::Open, Ok(value)) => {
                let id = value
                    .pointer("/session/sessionId")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let Some(id) = id else {
                    return vec![error(ErrorCategory::Other, "ZCode opened no session")];
                };
                self.handshake.session = Some(id.clone());
                vec![RuntimeEvent::Started {
                    native_id: id,
                    model: None,
                    path: None,
                }]
            }
            (Asked::Subscribe, Ok(_)) => {
                self.handshake.subscribed = true;
                Vec::new()
            }
            (Asked::Send, Err(message)) => {
                self.running = false;
                vec![
                    error(categorize("", &message), &message),
                    RuntimeEvent::State {
                        status: LiveStatusWord::Idle,
                    },
                ]
            }
            (_, Ok(_)) => Vec::new(),
            (_, Err(message)) => vec![error(categorize("", &message), &message)],
        }
    }

    fn engine_request(
        &mut self,
        id: Value,
        method: &str,
        params: &Value,
    ) -> (Vec<RuntimeEvent>, Vec<Value>) {
        match method {
            methods::HEADERS => (Vec::new(), vec![rpc::answer(&id, headers_answer())]),
            methods::PERMISSION => (self.approval(id, params), Vec::new()),
            methods::USER_INPUT => (self.question(id, params), Vec::new()),
            _ => (
                Vec::new(),
                vec![rpc::refusal(&id, "AI Juggler doesn't answer this")],
            ),
        }
    }

    /// The ask's id: the engine's request id (stable across the engine's
    /// re-announcements of the same ask), else its message id.
    fn open_ask(&mut self, id: &Value, params: &Value) -> Option<String> {
        let wire = id.as_str().map_or_else(|| id.to_string(), str::to_owned);
        let engine = params
            .get("requestId")
            .and_then(Value::as_str)
            .unwrap_or(&wire)
            .to_owned();
        if self.closed.contains(&engine) {
            return None;
        }
        if let Some(open) = self.engine_ids.get(&engine)
            && let Some(pending) = self.pending.get_mut(open)
        {
            // Asked again: answer the latest message.
            pending.request = id.clone();
            return None;
        }
        self.engine_ids.insert(engine.clone(), engine.clone());
        Some(engine)
    }

    fn approval(&mut self, id: Value, params: &Value) -> Vec<RuntimeEvent> {
        let Some(ask) = self.open_ask(&id, params) else {
            return Vec::new();
        };
        let tool = text(params, "toolName");
        let mut options = Vec::new();
        let mut decisions = BTreeMap::new();
        for option in params
            .get("options")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let (Some(option_id), Some(response)) = (
                option.get("optionId").and_then(Value::as_str),
                option.get("response"),
            ) else {
                continue;
            };
            let label = text(option, "name");
            let kind = text(option, "kind");
            let effect = match response.get("decision").and_then(Value::as_str) {
                Some("deny") => OptionEffect::Deny,
                Some("allow") if kind == "allow_once" => OptionEffect::AllowOnce,
                Some("allow") => OptionEffect::AllowRule {
                    rule: bounded(
                        option
                            .get("description")
                            .and_then(Value::as_str)
                            .unwrap_or(&label),
                    ),
                },
                // An option whose effect isn't known isn't offered.
                _ => continue,
            };
            decisions.insert(option_id.to_owned(), response.clone());
            options.push(AskOption {
                id: option_id.to_owned(),
                label: bounded(&label),
                effect,
            });
        }
        if options.is_empty() {
            // Nothing the user could pick: the engine waits on its own
            // timeout, and the user sees why.
            self.engine_ids.remove(&ask);
            return vec![error(
                ErrorCategory::Other,
                &format!("ZCode asked to use {tool} with no options AI Juggler can show"),
            )];
        }
        self.pending.insert(
            ask.clone(),
            Pending {
                request: id,
                decisions,
                questions: BTreeMap::new(),
            },
        );
        vec![
            RuntimeEvent::ApprovalRequested {
                approval: Approval {
                    id: AskId(ask),
                    kind: approval_kind(&tool),
                    summary: bounded(&summary(&tool, params.get("input"))),
                    reason: params
                        .get("reason")
                        .and_then(Value::as_str)
                        .filter(|reason| !reason.is_empty())
                        .map(bounded),
                    options,
                },
            },
            RuntimeEvent::State {
                status: LiveStatusWord::Waiting,
            },
        ]
    }

    fn question(&mut self, id: Value, params: &Value) -> Vec<RuntimeEvent> {
        let Some(ask) = self.open_ask(&id, params) else {
            return Vec::new();
        };
        let mut parts = Vec::new();
        let mut questions = BTreeMap::new();
        let listed = params.get("questions").and_then(Value::as_array);
        for (index, part) in listed.into_iter().flatten().enumerate() {
            let question = text(part, "question");
            let part_id = index.to_string();
            questions.insert(part_id.clone(), question.clone());
            parts.push(QuestionPart {
                id: part_id,
                header: bounded(&text(part, "header")),
                question: bounded(&question),
                choices: part
                    .get("options")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|choice| Choice {
                        label: bounded(&text(choice, "label")),
                        description: choice
                            .get("description")
                            .and_then(Value::as_str)
                            .map(bounded),
                    })
                    .collect(),
                multi: part.get("multiSelect").and_then(Value::as_bool) == Some(true),
                free_text: true,
                secret: false,
            });
        }
        if parts.is_empty() {
            let prompt = text(params, "prompt");
            questions.insert("0".into(), prompt.clone());
            parts.push(QuestionPart {
                id: "0".into(),
                header: "ZCode".into(),
                question: bounded(&prompt),
                choices: Vec::new(),
                multi: false,
                free_text: true,
                secret: false,
            });
        }
        self.pending.insert(
            ask.clone(),
            Pending {
                request: id,
                decisions: BTreeMap::new(),
                questions,
            },
        );
        vec![
            RuntimeEvent::QuestionAsked {
                question: Question {
                    id: AskId(ask),
                    parts,
                },
            },
            RuntimeEvent::State {
                status: LiveStatusWord::Waiting,
            },
        ]
    }

    fn event(&mut self, params: &Value) -> Vec<RuntimeEvent> {
        let ours = self.handshake.session.as_deref();
        if params.get("sessionId").and_then(Value::as_str) != ours || ours.is_none() {
            // A sub-agent's session, or another's: not this feed.
            return Vec::new();
        }
        let payload = params.get("payload").cloned().unwrap_or(Value::Null);
        match params.get("type").and_then(Value::as_str).unwrap_or("") {
            "turn.started" => {
                self.running = true;
                vec![
                    RuntimeEvent::TurnStarted {
                        turn: params
                            .get("turnId")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                        origin: None,
                    },
                    RuntimeEvent::State {
                        status: LiveStatusWord::Busy,
                    },
                ]
            }
            "turn.completed" => {
                self.running = false;
                let outcome = match payload.get("resultType").and_then(Value::as_str) {
                    Some("success") => TurnEnd::Completed,
                    Some("cancelled") => TurnEnd::Interrupted,
                    _ => TurnEnd::Failed,
                };
                ended(outcome, Vec::new())
            }
            "turn.failed" => {
                self.running = false;
                let error_value = payload.get("error").cloned().unwrap_or(Value::Null);
                let message = text(&error_value, "message");
                let kind = text(&error_value, "type");
                let category = categorize(&kind, &message);
                let words = if category == ErrorCategory::SignIn {
                    format!("ZCode couldn't use this App's sign-in: {message}")
                } else {
                    message
                };
                ended(TurnEnd::Failed, vec![error(category, &words)])
            }
            "part.delta" => {
                let delta = text(&payload, "delta");
                match payload.get("field").and_then(Value::as_str) {
                    None | Some("text") => vec![RuntimeEvent::TextDelta {
                        text: bounded(&delta),
                    }],
                    Some("output") => vec![RuntimeEvent::ToolDelta {
                        item: text(&payload, "partId"),
                        text: bounded(&delta),
                    }],
                    _ => Vec::new(),
                }
            }
            "tool.updated" => {
                let item = text(&payload, "toolCallId");
                let name = payload
                    .get("toolName")
                    .and_then(Value::as_str)
                    .unwrap_or("a tool")
                    .to_owned();
                let summary = ItemSummary {
                    kind: ItemKind::Tool,
                    text: bounded(&name),
                };
                match payload.get("kind").and_then(Value::as_str) {
                    Some("started") => vec![RuntimeEvent::ItemStarted { item, summary }],
                    Some("result" | "error") => vec![RuntimeEvent::Item { item: summary }],
                    _ => Vec::new(),
                }
            }
            "permission.resolved" | "userInput.resolved" => payload
                .get("requestId")
                .and_then(Value::as_str)
                .and_then(|engine| self.engine_ids.remove(engine))
                .map(|ask| {
                    self.pending.remove(&ask);
                    self.closed.insert(ask.clone());
                    vec![RuntimeEvent::AskResolved { id: AskId(ask) }]
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }
}

fn ended(outcome: TurnEnd, mut before: Vec<RuntimeEvent>) -> Vec<RuntimeEvent> {
    before.push(RuntimeEvent::TurnEnded {
        outcome,
        usage: None,
    });
    before.push(RuntimeEvent::State {
        status: LiveStatusWord::Idle,
    });
    before
}

fn error(category: ErrorCategory, message: &str) -> RuntimeEvent {
    RuntimeEvent::Error {
        category,
        message: bounded(message),
        will_retry: false,
    }
}

/// An engine error's category: a plan-limit refusal (Z.ai's own message,
/// [`crate::zcode_limits::read_message`], or the engine's
/// `rate_limited`), a sign-in failure, or anything else.
#[must_use]
pub fn categorize(kind: &str, message: &str) -> ErrorCategory {
    let lower = format!("{kind} {message}").to_lowercase();
    if crate::zcode_limits::read_message(message).is_some() || lower.contains("rate_limit") {
        ErrorCategory::RateLimit
    } else if [
        "auth",
        "unauthorized",
        "401",
        "credential",
        "sign in",
        "login",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        ErrorCategory::SignIn
    } else {
        ErrorCategory::Other
    }
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn approval_kind(tool: &str) -> ApprovalKind {
    match tool.to_ascii_lowercase().as_str() {
        "bash" | "shell" | "exec" => ApprovalKind::Command,
        "edit" | "write" | "multiedit" | "applypatch" | "apply_patch" => ApprovalKind::FileChange,
        _ => ApprovalKind::Tool,
    }
}

/// What a tool call asks, in one line: its command or file, else its name.
fn summary(tool: &str, input: Option<&Value>) -> String {
    ["command", "file_path", "path", "url"]
        .iter()
        .find_map(|key| input?.get(key)?.as_str())
        .map_or_else(|| tool.to_owned(), |what| format!("{tool}: {what}"))
}
