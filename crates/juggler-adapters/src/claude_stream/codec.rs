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
//!   Claude Code echoes no turn it is sent (only with
//!   `--replay-user-messages`, which isn't asked for), so each turn's text is
//!   reported here as it is sent: the conversation shows what it was given
//!   (F-rc3g-9).

use std::collections::HashMap;

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::book::AskBook;
use juggler_adapter::runtime::events::bounded;
use juggler_adapter::runtime::{
    Approval, ApprovalKind, AskAnswer, AskId, AskOption, Choice, ErrorCategory, ItemKind,
    ItemSummary, LimitWindow, LimitsSnapshot, LiveStatusWord, OptionEffect, Question, QuestionPart,
    RuntimeEvent, TurnEnd, TurnInput, TurnStop, TurnUsage,
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

/// A window's length from its name, and the one model it limits when it
/// is a week on one model only (`seven_day_opus`), by the name people
/// know it by ("Opus", as Claude's own usage page writes it).
fn window_of(name: &str) -> (Option<u32>, Option<String>) {
    match name {
        "five_hour" => (Some(300), None),
        "seven_day" => (Some(10_080), None),
        "seven_day_opus" => (Some(10_080), Some("Opus".to_owned())),
        "seven_day_sonnet" => (Some(10_080), Some("Sonnet".to_owned())),
        _ => (None, None),
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

/// A utilization as a percentage, 0–100. Claude Code sends it as the
/// fraction of the window used, always: lab 2 read 0.01 where `/usage`
/// said 1%, and its own schema says "the fraction of the window used
/// (usually 0-1 …; values above 1 occur when usage legitimately runs
/// past a window's cap)". So 1 is all of it, and anything above 1 is past
/// it: all used, never a small whole percent (AJ-E2311 item 8).
fn percent(value: &Value) -> Option<f64> {
    let used = value.as_f64().filter(|used| used.is_finite())?;
    Some((used * 100.0).clamp(0.0, 100.0))
}

/// One window of a `rate_limit_event`: its use as the tool gave it, or
/// all of it when the tool says this window's limit is reached (`hit`);
/// `None` when the tool didn't say how much is used (F-rc3g-1: never read
/// as 0% used, which reads as all of it left).
fn window(name: &str, window: &Value, hit: bool) -> Option<LimitWindow> {
    let used_pct = window
        .get("utilization")
        .and_then(percent)
        .or(hit.then_some(100.0))?;
    let (window_mins, model) = window_of(name);
    Some(LimitWindow {
        id: name.to_owned(),
        window_mins,
        used_pct,
        resets_at_ms: window.get("resetsAt").and_then(epoch_ms),
        model,
    })
}

/// A `rate_limit_event`'s windows, as Claude Code named them.
#[must_use]
pub fn limits_of(info: &Value) -> LimitsSnapshot {
    let reached = text(info, "/status").as_deref() == Some("rejected");
    let hit_window = text(info, "/rateLimitType").filter(|_| reached);
    let mut windows: Vec<LimitWindow> = info
        .get("unifiedWindows")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(name, value)| window(name, value, hit_window.as_ref() == Some(name)))
        .collect();
    if windows.is_empty()
        && let Some(name) = text(info, "/rateLimitType")
    {
        windows.extend(window(&name, info, reached));
    }
    windows.sort_by_key(|w| w.window_mins.unwrap_or(u32::MAX));
    LimitsSnapshot {
        windows,
        plan: None,
        reached,
    }
}

/// A `result`'s subtype (Claude Code names a turn that ended early only
/// by it) as why the turn ended: `error_during_execution` is a turn
/// stopped before it finished (an interrupt, or "Deny and stop").
fn turn_stop(subtype: &str) -> TurnStop {
    match subtype {
        "error_during_execution" => TurnStop::Stopped,
        "error_max_turns" => TurnStop::TurnLimit,
        "error_max_budget_usd" => TurnStop::SpendLimit,
        "error_max_structured_output_retries" => TurnStop::AnswerForm,
        _ => TurnStop::Failed,
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
        let given = ItemSummary {
            kind: ItemKind::UserMessage,
            text: bounded(&input.text),
        };
        let events = vec![
            RuntimeEvent::TurnStarted {
                turn: None,
                origin: Some(input.origin),
            },
            RuntimeEvent::Item { item: given },
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
        Ok((line, self.book.answered(ask)))
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
        let home = juggler_adapter::home();
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
            let said = text(message, "/result");
            // Only its code: the app says why in its own words.
            let turn_end = said.is_none().then(|| turn_stop(&subtype));
            let words = said.unwrap_or_else(|| subtype.clone());
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
                turn_end,
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
                        turn_end: None,
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

/// F-rc3g-1: a driven session's limits are kept as the account's reading,
/// so a window says only what Claude Code said: no 0% for a use it didn't
/// give, and a week on one model only never reads as the account's week.
#[cfg(test)]
mod limits {
    use super::limits_of;
    use serde_json::json;

    #[test]
    fn a_window_without_its_use_is_left_out_never_read_as_none_used() {
        // Claude Code's own shape when it gives no utilization.
        let quiet = json!({"status": "allowed", "rateLimitType": "five_hour",
            "resetsAt": 1_761_397_200, "overageStatus": "rejected"});
        assert!(limits_of(&quiet).windows.is_empty());
        let unified = json!({"status": "allowed", "unifiedWindows": {
            "five_hour": {"resetsAt": 1_761_397_200},
            "seven_day": {"utilization": 0.05, "resetsAt": 1_761_900_000}}});
        let limits = limits_of(&unified);
        assert_eq!(limits.windows.len(), 1);
        assert_eq!(limits.windows[0].id, "seven_day");
        assert!((limits.windows[0].used_pct - 5.0).abs() < 1e-9);
    }

    #[test]
    fn a_limit_reached_is_all_used_even_without_its_use() {
        let hit = json!({"status": "rejected", "rateLimitType": "five_hour",
            "resetsAt": 1_761_397_200});
        let limits = limits_of(&hit);
        assert!(limits.reached);
        assert_eq!(limits.windows.len(), 1);
        assert!((limits.windows[0].used_pct - 100.0).abs() < 1e-9);
        assert_eq!(limits.windows[0].resets_at_ms, Some(1_761_397_200_000));
        // Only the window that was hit: the other's use isn't known.
        let unified = json!({"status": "rejected", "rateLimitType": "seven_day",
            "unifiedWindows": {"five_hour": {"resetsAt": 1}, "seven_day": {"resetsAt": 2}}});
        let ids: Vec<String> = limits_of(&unified)
            .windows
            .into_iter()
            .map(|w| format!("{}={}", w.id, w.used_pct))
            .collect();
        assert_eq!(ids, ["seven_day=100"]);
    }

    #[test]
    fn a_week_on_one_model_names_its_model() {
        let info = json!({"status": "allowed", "unifiedWindows": {
            "five_hour": {"utilization": 0.36, "resetsAt": 1_761_397_200},
            "seven_day": {"utilization": 0.05},
            "seven_day_opus": {"utilization": 0.4}}});
        let windows = limits_of(&info).windows;
        let models: Vec<(&str, Option<u32>, Option<&str>)> = windows
            .iter()
            .map(|w| (w.id.as_str(), w.window_mins, w.model.as_deref()))
            .collect();
        assert_eq!(
            models,
            [
                ("five_hour", Some(300), None),
                ("seven_day", Some(10_080), None),
                ("seven_day_opus", Some(10_080), Some("Opus")),
            ]
        );
    }

    /// AJ-E2311 item 8: `utilization` is always a fraction of the window.
    /// A whole 1 is all of it, as 1.0 is; past the cap (above 1) is all
    /// of it too, never read as a small whole percent.
    #[test]
    fn a_use_is_always_a_fraction_of_its_window() {
        let used = |utilization: serde_json::Value| {
            let info = json!({"status": "allowed",
                "unifiedWindows": {"five_hour": {"utilization": utilization}}});
            limits_of(&info).windows.first().map(|w| w.used_pct)
        };
        let cases = [
            (json!(0), 0.0),
            (json!(0.0), 0.0),
            (json!(0.01), 1.0),
            (json!(0.5), 50.0),
            (json!(0.99), 99.0),
            (json!(1), 100.0),
            (json!(1.0), 100.0),
            (json!(1.2), 100.0),
            (json!(99), 100.0),
            (json!(100), 100.0),
            (json!(-0.1), 0.0),
        ];
        for (utilization, want) in cases {
            let got = used(utilization.clone()).expect("a reading");
            assert!(
                (got - want).abs() < 1e-9,
                "{utilization} read as {got}%, not {want}%"
            );
        }
        assert_eq!(used(json!("0.5")), None, "not a number: no reading");
    }
}

/// AJ-E2025 item 1: a turn Claude Code names only by its `result`
/// subtype ends with a neutral kind on its error, so the app says why in
/// its own words; a turn the tool put in words keeps them, with no kind.
#[cfg(test)]
mod turn_end {
    use super::{Codec, RuntimeEvent, TurnStop};
    use serde_json::json;

    fn ended(result: &serde_json::Value) -> Option<(String, Option<TurnStop>)> {
        let mut codec = Codec::default();
        let (events, _) = codec.incoming(&result.to_string());
        events.into_iter().find_map(|event| match event {
            RuntimeEvent::Error {
                message, turn_end, ..
            } => Some((message, turn_end)),
            _ => None,
        })
    }

    #[test]
    fn each_code_is_a_kind_and_words_stay_words() {
        let kinds = [
            ("error_during_execution", TurnStop::Stopped),
            ("error_max_turns", TurnStop::TurnLimit),
            ("error_max_budget_usd", TurnStop::SpendLimit),
            ("error_max_structured_output_retries", TurnStop::AnswerForm),
            ("error_some_new_code", TurnStop::Failed),
        ];
        for (code, kind) in kinds {
            // The recorded shape: an empty `result` beside the subtype.
            let result = json!({"type": "result", "subtype": code, "is_error": true, "result": ""});
            assert_eq!(
                ended(&result).and_then(|(_, end)| end),
                Some(kind),
                "{code}"
            );
        }
        let said = json!({"type": "result", "subtype": "success", "is_error": true,
            "result": "Credit balance is too low"});
        assert_eq!(
            ended(&said),
            Some(("Credit balance is too low".to_owned(), None))
        );
        let fine =
            json!({"type": "result", "subtype": "success", "is_error": false, "result": "Done"});
        assert_eq!(ended(&fine), None);
        assert_eq!(TurnStop::TurnLimit.as_str(), "turn_limit");
    }
}
