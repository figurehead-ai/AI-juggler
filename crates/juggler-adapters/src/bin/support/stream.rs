//! The fake Claude Code's stream-json protocol (see `fake_claude.rs`).

use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

fn say(value: &Value) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{value}").and_then(|()| out.flush());
}

fn text(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

struct Fake {
    id: String,
    model: String,
    cwd: String,
    transcript: PathBuf,
    /// The permission request waiting for an answer, and its kind.
    waiting: Option<(String, &'static str)>,
    slow: bool,
    count: u32,
}

impl Fake {
    fn record(&self, line: &Value) {
        if let Some(dir) = self.transcript.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.transcript)
        {
            let _ = writeln!(file, "{line}");
        }
    }

    fn finish(&mut self, said: &str, subtype: &str) {
        if !said.is_empty() {
            say(&json!({"type": "assistant", "session_id": self.id,
                "message": {"role": "assistant", "content": [{"type": "text", "text": said}]}}));
            self.record(
                &json!({"type": "assistant", "sessionId": self.id, "cwd": self.cwd,
                "uuid": format!("a-{}", self.count), "timestamp": "2026-10-25T10:00:05.000Z",
                "message": {"role": "assistant", "content": [{"type": "text", "text": said}],
                    "usage": {"input_tokens": 100, "output_tokens": 20}}}),
            );
        }
        say(
            &json!({"type": "rate_limit_event", "session_id": self.id, "rate_limit_info": {
            "status": "allowed", "rateLimitType": "five_hour", "resetsAt": 1_761_397_200,
            "unifiedWindows": {"five_hour": {"utilization": 0.27, "resetsAt": 1_761_397_200}},
            "isUsingOverage": false}}),
        );
        say(
            &json!({"type": "result", "subtype": subtype, "is_error": subtype != "success",
            "session_id": self.id, "result": said, "num_turns": 1, "total_cost_usd": 0.01,
            "usage": {"input_tokens": 100, "output_tokens": 20, "cache_read_input_tokens": 50}}),
        );
    }

    fn turn(&mut self, message: &Value) {
        self.count += 1;
        let prompt = text(message, "/message/content");
        say(
            &json!({"type": "system", "subtype": "init", "session_id": self.id, "model": self.model,
            "cwd": self.cwd, "apiKeySource": "none", "tools": ["Bash", "AskUserQuestion"],
            "permissionMode": "default"}),
        );
        self.record(
            &json!({"type": "user", "sessionId": self.id, "cwd": self.cwd,
            "uuid": format!("u-{}", self.count), "timestamp": "2026-10-25T10:00:00.000Z",
            "message": {"role": "user", "content": prompt}}),
        );
        let request = format!("req-{}", self.count);
        if let Some(rest) = prompt.split("ask:").nth(1) {
            let command = rest.trim();
            let first = command.split_whitespace().next().unwrap_or_default();
            say(
                &json!({"type": "control_request", "request_id": request, "request": {
                "subtype": "can_use_tool", "tool_name": "Bash", "input": {"command": command},
                "tool_use_id": format!("tu-{}", self.count),
                "permission_suggestions": [{"type": "addRules", "behavior": "allow",
                    "destination": "localSettings",
                    "rules": [{"toolName": "Bash", "ruleContent": format!("{first}:*")}]}]}}),
            );
            self.waiting = Some((request, "tool"));
        } else if let Some(rest) = prompt.split("question:").nth(1) {
            say(
                &json!({"type": "control_request", "request_id": request, "request": {
                "subtype": "can_use_tool", "tool_name": "AskUserQuestion",
                "input": {"questions": [{"question": rest.trim(), "header": "Colour",
                    "options": [{"label": "Red", "description": "warm"}, {"label": "Blue", "description": "cool"}],
                    "multiSelect": false}]}}}),
            );
            self.waiting = Some((request, "question"));
        } else if prompt.contains("slow:") {
            self.slow = true;
        } else {
            self.finish("done", "success");
        }
    }

    fn response(&mut self, message: &Value) {
        let id = text(message, "/response/request_id");
        let Some((waiting, kind)) = self.waiting.take_if(|(waiting, _)| *waiting == id) else {
            return;
        };
        let _ = waiting;
        let decision = message
            .pointer("/response/response")
            .cloned()
            .unwrap_or(Value::Null);
        let behavior = text(&decision, "/behavior");
        let said = match (kind, behavior.as_str()) {
            ("question", "allow") => format!(
                "you said {}",
                decision
                    .pointer("/updatedInput/answers")
                    .map(Value::to_string)
                    .unwrap_or_default()
            ),
            (_, "allow") => format!(
                "ran it{}",
                if decision.get("updatedPermissions").is_some() {
                    " and kept the rule"
                } else {
                    ""
                }
            ),
            _ => "didn't".to_owned(),
        };
        let stop = decision
            .get("interrupt")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        self.finish(
            &said,
            if stop {
                "error_during_execution"
            } else {
                "success"
            },
        );
    }

    fn control(&mut self, message: &Value) {
        let id = text(message, "/request_id");
        let ok = |id: &str, response: Value| {
            say(
                &json!({"type": "control_response", "response": {"subtype": "success",
                "request_id": id, "response": response}}),
            );
        };
        match text(message, "/request/subtype").as_str() {
            "initialize" => ok(
                &id,
                json!({"commands": [], "models": [{"value": "claude-fake"}]}),
            ),
            "interrupt" => {
                ok(&id, json!({}));
                if self.slow || self.waiting.is_some() {
                    self.slow = false;
                    self.waiting = None;
                    self.finish("", "error_during_execution");
                }
            }
            "set_model" => {
                self.model = text(message, "/request/model");
                ok(&id, json!({}));
            }
            _ => say(
                &json!({"type": "control_response", "response": {"subtype": "error",
                "request_id": id, "error": "not faked"}}),
            ),
        }
    }
}

/// Runs the protocol until stdin closes.
pub fn run(id: &str, model: Option<String>) {
    let config = std::env::var_os("CLAUDE_CONFIG_DIR").map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".claude"),
        PathBuf::from,
    );
    let cwd = std::env::current_dir()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_default();
    let folder = cwd.replace(['/', '.'], "-");
    let mut fake = Fake {
        id: id.to_owned(),
        model: model.unwrap_or_else(|| "claude-fake".to_owned()),
        transcript: config
            .join("projects")
            .join(folder)
            .join(format!("{id}.jsonl")),
        cwd,
        waiting: None,
        slow: false,
        count: 0,
    };
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        match text(&message, "/type").as_str() {
            "control_request" => fake.control(&message),
            "control_response" => fake.response(&message),
            "user" if fake.waiting.is_none() && !fake.slow => fake.turn(&message),
            _ => {}
        }
    }
}
