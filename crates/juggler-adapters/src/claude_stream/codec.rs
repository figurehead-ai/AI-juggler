//! Claude Code's stream-json protocol (research §3.1) as neutral runtime
//! events, and AI Juggler's messages in its shapes. Pure: the recorded
//! fixtures drive it directly. The session (`session.rs`) moves lines.
//!
//! - In: `system` (`init`), `assistant` (text and tool uses), `user` (tool
//!   results, ignored), `result` (the turn's end, with its usage),
//!   `rate_limit_event`, and `control_request`s — `can_use_tool` (an
//!   approval, or `AskUserQuestion`'s questions), anything else refused.
//! - Out: `user` turns, `control_request`s (`initialize`, `interrupt`,
//!   `set_model`) and `control_response`s carrying the user's own answer.

use std::collections::HashMap;

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::book::AskBook;
use juggler_adapter::runtime::events::bounded;
use juggler_adapter::runtime::{
    Approval, ApprovalKind, AskAnswer, AskId, AskOption, Choice, ErrorCategory, ItemKind,
    ItemSummary, LimitWindow, LimitsSnapshot, LiveStatusWord, OptionEffect, Question, QuestionPart,
    RuntimeEvent, TurnEnd, TurnInput, TurnUsage,
};
use serde_json::{Value, json};

/// How an open ask is answered.
#[derive(Clone, Debug, PartialEq)]
enum Answering {
    /// A tool's permission: its input, and the rules Claude Code suggested.
    Tool {
        input: Value,
        suggestions: Vec<Value>,
    },
    /// `AskUserQuestion`: its input (the answers go into it).
    Question { input: Value },
}

/// The codec's state for one session.
#[derive(Debug, Default)]
pub struct Codec {
    next_id: u64,
    session: Option<String>,
    turn_running: bool,
    limits: Option<LimitsSnapshot>,
    open: HashMap<String, Answering>,
    book: AskBook,
    /// The initialize request's id, until it is answered.
    pub initializing: Option<String>,
    /// Whether the CLI answered `initialize`.
    pub initialized: bool,
    /// Messages of a kind the codec doesn't know (ignored, never guessed).
    pub unknown: usize,
}

fn text(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn state(status: LiveStatusWord) -> RuntimeEvent {
    RuntimeEvent::State { status }
}

/// A window's length from its name.
fn window_mins(name: &str) -> Option<u32> {
    match name {
        "five_hour" => Some(300),
        "seven_day" | "seven_day_opus" | "seven_day_sonnet" => Some(10_080),
        _ => None,
    }
}

/// Epoch ms from seconds or ms.
fn epoch_ms(value: &Value) -> Option<i64> {
    // A fractional time is cut to its second (`as` saturates, never wraps).
    #[allow(clippy::cast_possible_truncation)]
    let number = value
        .as_i64()
        .or_else(|| value.as_f64().map(|f| f.trunc() as i64))?;
    Some(if number < 100_000_000_000 {
        number.saturating_mul(1000)
    } else {
        number
    })
}

/// A utilization as a percentage (a fraction is scaled; never over 100).
fn percent(value: &Value) -> Option<f64> {
    let used = value.as_f64()?;
    Some(if used <= 1.0 { used * 100.0 } else { used }.clamp(0.0, 100.0))
}

/// A `rate_limit_event`'s windows, as Claude Code named them.
#[must_use]
pub fn limits_of(info: &Value) -> LimitsSnapshot {
    let mut windows: Vec<LimitWindow> = info
        .get("unifiedWindows")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(name, window)| LimitWindow {
            id: name.clone(),
            window_mins: window_mins(name),
            used_pct: window.get("utilization").and_then(percent).unwrap_or(0.0),
            resets_at_ms: window.get("resetsAt").and_then(epoch_ms),
        })
        .collect();
    if windows.is_empty()
        && let Some(name) = text(info, "/rateLimitType")
    {
        windows.push(LimitWindow {
            window_mins: window_mins(&name),
            id: name,
            used_pct: info.get("utilization").and_then(percent).unwrap_or(0.0),
            resets_at_ms: info.get("resetsAt").and_then(epoch_ms),
        });
    }
    windows.sort_by_key(|w| w.window_mins.unwrap_or(u32::MAX));
    LimitsSnapshot {
        windows,
        plan: None,
        reached: text(info, "/status").as_deref() == Some("rejected"),
    }
}

/// What a tool's input asks, in one line.
fn summary_of(tool: &str, input: &Value) -> String {
    let detail = [
        "/command",
        "/file_path",
        "/path",
        "/url",
        "/pattern",
        "/description",
    ]
    .iter()
    .find_map(|pointer| text(input, pointer));
    bounded(&detail.map_or_else(|| tool.to_owned(), |d| format!("{tool}: {d}")))
}

fn kind_of(tool: &str) -> ApprovalKind {
    match tool {
        "Bash" | "BashOutput" | "KillShell" => ApprovalKind::Command,
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => ApprovalKind::FileChange,
        _ => ApprovalKind::Tool,
    }
}

/// A folder in a few words: under the home folder `~/…/<name>` (or
/// `~/<name>`), else the path as it is.
fn folder_words(path: &str, home: Option<&std::path::Path>) -> String {
    let path = std::path::Path::new(path);
    let Some(rest) = home.and_then(|home| path.strip_prefix(home).ok()) else {
        return path.display().to_string();
    };
    let parts: Vec<_> = rest.components().collect();
    match parts.as_slice() {
        [] => "~".to_owned(),
        [one] => format!("~/{}", one.as_os_str().to_string_lossy()),
        [.., last] => format!("~/…/{}", last.as_os_str().to_string_lossy()),
    }
}

/// A suggestion as the option's label and the rule it keeps. Lab 2
/// (2026-10-25): an `addDirectories` suggestion (no `rules`, no `mode`)
/// names its folders, as Claude Code's own dialog does.
fn suggestion_words(suggestion: &Value, home: Option<&std::path::Path>) -> (String, String) {
    if text(suggestion, "/type").as_deref() == Some("addDirectories") {
        let folders: Vec<String> = suggestion
            .get("directories")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(|folder| folder_words(folder, home))
            .collect();
        let place = match text(suggestion, "/destination").as_deref() {
            Some("session") => " for this session",
            Some("localSettings") => " in this project (your local settings)",
            Some("projectSettings") => " in this project",
            Some("userSettings") => " everywhere",
            _ => "",
        };
        let rule = format!("edits in {}{place}", folders.join(", "));
        return (format!("Allow {rule}"), rule);
    }
    let rule = rule_words(suggestion);
    (format!("Always allow {rule}"), rule)
}

/// A suggested rule in words (`Bash(npm test:*)`, and where it is kept).
fn rule_words(suggestion: &Value) -> String {
    let rules: Vec<String> = suggestion
        .get("rules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(
            |rule| match (text(rule, "/toolName"), text(rule, "/ruleContent")) {
                (Some(tool), Some(content)) => format!("{tool}({content})"),
                (Some(tool), None) => tool,
                _ => "this".to_owned(),
            },
        )
        .collect();
    let what = if rules.is_empty() {
        text(suggestion, "/mode").map_or_else(|| "this".to_owned(), |mode| format!("{mode} mode"))
    } else {
        rules.join(", ")
    };
    match text(suggestion, "/destination").as_deref() {
        Some("session") => format!("{what}, for this session"),
        Some("localSettings") => format!("{what}, in this project (your local settings)"),
        Some("projectSettings") => format!("{what}, in this project"),
        Some("userSettings") => format!("{what}, everywhere"),
        _ => what,
    }
}

fn option(id: String, label: String, effect: OptionEffect) -> AskOption {
    AskOption { id, label, effect }
}

impl Codec {
    fn request_id(&mut self) -> String {
        self.next_id += 1;
        format!("aj-{}", self.next_id)
    }

    /// The `initialize` control request (the handshake).
    pub fn initialize(&mut self) -> Value {
        let id = self.request_id();
        self.initializing = Some(id.clone());
        json!({"type": "control_request", "request_id": id, "request": {"subtype": "initialize"}})
    }

    /// Notes the conversation's id (known before the start: AI Juggler
    /// passes `--session-id` or `--resume`).
    pub fn set_session(&mut self, id: &str) {
        self.session = Some(id.to_owned());
    }

    /// Whether a turn runs.
    #[must_use]
    pub fn turn_running(&self) -> bool {
        self.turn_running
    }

    /// The last limits Claude Code reported.
    #[must_use]
    pub fn limits(&self) -> Option<LimitsSnapshot> {
        self.limits.clone()
    }

    /// A user turn (only between turns): the line and the events it makes.
    ///
    /// # Errors
    ///
    /// While a turn runs.
    pub fn turn(&mut self, input: TurnInput) -> Result<(Value, Vec<RuntimeEvent>), AdapterError> {
        if self.turn_running {
            return Err(AdapterError::InvalidLaunch(
                "a turn is running: it can start when this one ends".into(),
            ));
        }
        self.turn_running = true;
        let mut line = json!({"type": "user", "message": {"role": "user", "content": input.text},
            "parent_tool_use_id": null});
        if let Some(session) = &self.session {
            line["session_id"] = json!(session);
        }
        let events = vec![
            RuntimeEvent::TurnStarted {
                turn: None,
                origin: Some(input.origin),
            },
            state(LiveStatusWord::Busy),
        ];
        Ok((line, events))
    }

    /// `interrupt`.
    pub fn interrupt(&mut self) -> Value {
        let id = self.request_id();
        json!({"type": "control_request", "request_id": id, "request": {"subtype": "interrupt"}})
    }

    /// `set_model`.
    pub fn set_model(&mut self, model: &str) -> Value {
        let id = self.request_id();
        json!({"type": "control_request", "request_id": id,
            "request": {"subtype": "set_model", "model": model}})
    }

    /// The user's answer to an open ask: the `control_response` line and
    /// the events it makes (the ask closes at once).
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
        let unfit = || AdapterError::InvalidLaunch("that answer doesn't fit this ask".into());
        let how = self.open.get(ask.as_str()).ok_or_else(unfit)?;
        let decision = match (how, answer) {
            (Answering::Tool { input, suggestions }, AskAnswer::Choose { option }) => {
                match option.as_str() {
                    "allow" => json!({"behavior": "allow", "updatedInput": input}),
                    "deny" => json!({"behavior": "deny",
                        "message": "The user denied this in AI Juggler.", "interrupt": false}),
                    "deny_stop" => json!({"behavior": "deny",
                        "message": "The user denied this in AI Juggler and stopped the turn.",
                        "interrupt": true}),
                    rule => {
                        let index: usize = rule
                            .strip_prefix("rule:")
                            .and_then(|n| n.parse().ok())
                            .ok_or_else(unfit)?;
                        let suggestion = suggestions.get(index).cloned().ok_or_else(unfit)?;
                        json!({"behavior": "allow", "updatedInput": input,
                            "updatedPermissions": [suggestion]})
                    }
                }
            }
            (Answering::Question { input }, AskAnswer::Reply { parts }) => {
                let answers: serde_json::Map<String, Value> = parts
                    .iter()
                    .map(|part| (part.part.clone(), json!(part.answers.join(", "))))
                    .collect();
                let mut updated = input.clone();
                updated["answers"] = Value::Object(answers);
                json!({"behavior": "allow", "updatedInput": updated})
            }
            _ => return Err(unfit()),
        };
        let line = json!({"type": "control_response", "response": {"subtype": "success",
            "request_id": ask.as_str(), "response": decision}});
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

    fn approval(&mut self, id: &str, request: &Value) -> Vec<RuntimeEvent> {
        let tool = text(request, "/tool_name").unwrap_or_else(|| "A tool".into());
        let input = request.get("input").cloned().unwrap_or(Value::Null);
        let suggestions: Vec<Value> = request
            .get("permission_suggestions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut options = vec![option(
            "allow".into(),
            "Allow once".into(),
            OptionEffect::AllowOnce,
        )];
        let home = crate::inventory_files::user_home();
        for (index, suggestion) in suggestions.iter().enumerate() {
            let (label, rule) = suggestion_words(suggestion, home.as_deref());
            options.push(option(
                format!("rule:{index}"),
                label,
                OptionEffect::AllowRule { rule },
            ));
        }
        options.push(option("deny".into(), "Deny".into(), OptionEffect::Deny));
        options.push(option(
            "deny_stop".into(),
            "Deny and stop".into(),
            OptionEffect::DenyAndStop,
        ));
        let summary = text(request, "/title")
            .map_or_else(|| summary_of(&tool, &input), |title| bounded(&title));
        self.open.insert(
            id.to_owned(),
            Answering::Tool {
                input: input.clone(),
                suggestions,
            },
        );
        vec![
            RuntimeEvent::ApprovalRequested {
                approval: Approval {
                    id: AskId(id.to_owned()),
                    kind: kind_of(&tool),
                    summary,
                    reason: text(request, "/description").map(|d| bounded(&d)),
                    options,
                },
            },
            state(LiveStatusWord::Waiting),
        ]
    }

    fn question(&mut self, id: &str, request: &Value) -> Vec<RuntimeEvent> {
        let input = request.get("input").cloned().unwrap_or(Value::Null);
        let parts = input
            .get("questions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|q| {
                let question = text(q, "/question").unwrap_or_default();
                QuestionPart {
                    // AskUserQuestion's answers are keyed by the question.
                    id: question.clone(),
                    header: bounded(&text(q, "/header").unwrap_or_default()),
                    question: bounded(&question),
                    choices: q
                        .get("options")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .map(|o| Choice {
                            label: text(o, "/label").unwrap_or_default(),
                            description: text(o, "/description"),
                        })
                        .collect(),
                    multi: q
                        .get("multiSelect")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    free_text: true,
                    secret: false,
                }
            })
            .collect();
        self.open
            .insert(id.to_owned(), Answering::Question { input });
        vec![
            RuntimeEvent::QuestionAsked {
                question: Question {
                    id: AskId(id.to_owned()),
                    parts,
                },
            },
            state(LiveStatusWord::Waiting),
        ]
    }

    fn result(&mut self, message: &Value) -> Vec<RuntimeEvent> {
        self.turn_running = false;
        let count = |key: &str| {
            message
                .pointer(&format!("/usage/{key}"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        };
        let usage = TurnUsage {
            input: count("input_tokens"),
            cached_input: count("cache_read_input_tokens"),
            output: count("output_tokens"),
            reasoning: 0,
            context_window: None,
        };
        let subtype = text(message, "/subtype").unwrap_or_default();
        let failed = message
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || subtype != "success";
        let mut events = Vec::new();
        if failed {
            let words = text(message, "/result").unwrap_or_else(|| subtype.clone());
            let category = if words.contains("limit") {
                ErrorCategory::RateLimit
            } else if words.contains("billing") || words.contains("credit") {
                ErrorCategory::Billing
            } else if words.contains("auth") || words.contains("log in") {
                ErrorCategory::SignIn
            } else {
                ErrorCategory::Other
            };
            events.push(RuntimeEvent::Error {
                category,
                message: bounded(&words),
                will_retry: false,
            });
        }
        let outcome = if !failed {
            TurnEnd::Completed
        } else if subtype == "error_during_execution" {
            TurnEnd::Interrupted
        } else {
            TurnEnd::Failed
        };
        events.push(RuntimeEvent::TurnEnded {
            outcome,
            usage: Some(usage),
        });
        events.push(state(LiveStatusWord::Idle));
        events
    }

    fn assistant(message: &Value) -> Vec<RuntimeEvent> {
        message
            .pointer("/message/content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|block| match text(block, "/type").as_deref() {
                Some("text") => Some(ItemSummary {
                    kind: ItemKind::AgentMessage,
                    text: bounded(&text(block, "/text")?),
                }),
                Some("tool_use") => {
                    let tool = text(block, "/name")?;
                    let input = block.get("input").unwrap_or(&Value::Null);
                    Some(ItemSummary {
                        kind: match kind_of(&tool) {
                            ApprovalKind::Command => ItemKind::Command,
                            ApprovalKind::FileChange => ItemKind::FileChange,
                            _ => ItemKind::Tool,
                        },
                        text: summary_of(&tool, input),
                    })
                }
                _ => None,
            })
            .map(|item| RuntimeEvent::Item { item })
            .collect()
    }

    /// One line from the CLI: its events, and any line to send back at once
    /// (a refusal of a control request AI Juggler doesn't handle; never an
    /// answer to an ask).
    pub fn incoming(&mut self, line: &str) -> (Vec<RuntimeEvent>, Vec<Value>) {
        let Ok(message) = serde_json::from_str::<Value>(line) else {
            self.unknown += 1;
            return (Vec::new(), Vec::new());
        };
        let events = match text(&message, "/type").as_deref() {
            Some("control_request") => {
                let id = text(&message, "/request_id").unwrap_or_default();
                let request = message.get("request").cloned().unwrap_or(Value::Null);
                match text(&request, "/subtype").as_deref() {
                    Some("can_use_tool")
                        if text(&request, "/tool_name").as_deref() == Some("AskUserQuestion") =>
                    {
                        self.question(&id, &request)
                    }
                    Some("can_use_tool") => self.approval(&id, &request),
                    _ => {
                        self.unknown += 1;
                        let refusal = json!({"type": "control_response", "response": {
                            "subtype": "error", "request_id": id,
                            "error": "AI Juggler doesn't handle this request"}});
                        return (Vec::new(), vec![refusal]);
                    }
                }
            }
            Some("control_response") => {
                let id = text(&message, "/response/request_id");
                if id.is_some() && id == self.initializing {
                    self.initializing = None;
                    self.initialized =
                        text(&message, "/response/subtype").as_deref() == Some("success");
                }
                match text(&message, "/response/subtype").as_deref() {
                    Some("error") => vec![RuntimeEvent::Error {
                        category: ErrorCategory::Other,
                        message: bounded(&text(&message, "/response/error").unwrap_or_default()),
                        will_retry: false,
                    }],
                    _ => Vec::new(),
                }
            }
            Some("system") if text(&message, "/subtype").as_deref() == Some("init") => {
                text(&message, "/model")
                    .map(|model| vec![RuntimeEvent::ModelChanged { model }])
                    .unwrap_or_default()
            }
            Some("assistant") => Self::assistant(&message),
            Some("result") => self.result(&message),
            Some("rate_limit_event") => {
                let limits = limits_of(message.get("rate_limit_info").unwrap_or(&Value::Null));
                self.limits = Some(limits.clone());
                vec![RuntimeEvent::LimitsUpdated { limits }]
            }
            Some("user" | "system" | "stream_event") => Vec::new(),
            _ => {
                self.unknown += 1;
                Vec::new()
            }
        };
        for event in &events {
            self.book.note(event);
            if matches!(event, RuntimeEvent::TurnEnded { .. }) {
                self.open.clear();
            }
        }
        (events, Vec::new())
    }
}

#[cfg(test)]
mod words {
    use super::{folder_words, suggestion_words};
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn a_folder_suggestion_names_its_folder() {
        let home = Path::new("/Users/me");
        assert_eq!(folder_words("/Users/me/lab", Some(home)), "~/lab");
        assert_eq!(folder_words("/Users/me/work/lab", Some(home)), "~/…/lab");
        assert_eq!(folder_words("/kit/project", Some(home)), "/kit/project");
        let live = json!({"type": "addDirectories", "directories": ["/Users/me/work/lab"], "destination": "session"});
        assert_eq!(
            suggestion_words(&live, Some(home)),
            (
                "Allow edits in ~/…/lab for this session".to_owned(),
                "edits in ~/…/lab for this session".to_owned()
            )
        );
        let rule = json!({"type": "addRules", "destination": "localSettings",
            "rules": [{"toolName": "Bash", "ruleContent": "echo lab *"}]});
        assert_eq!(
            suggestion_words(&rule, Some(home)).0,
            "Always allow Bash(echo lab *), in this project (your local settings)"
        );
    }
}
