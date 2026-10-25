//! The fake app-server's protocol (see `fake_codex.rs`).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use juggler_adapters::codex_appserver::surface::{Kind, SURFACE};
use serde_json::{Value, json};

/// Writes the method lists `generate-json-schema` would, without
/// `FAKE_CODEX_DROP`.
pub fn write_schema(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let drop = std::env::var("FAKE_CODEX_DROP").unwrap_or_default();
    for kind in [Kind::ClientRequest, Kind::ServerRequest, Kind::Notification] {
        let variants: Vec<Value> = SURFACE
            .iter()
            .filter(|(k, method)| *k == kind && *method != drop)
            .map(|(_, method)| json!({"properties": {"method": {"enum": [method]}}}))
            .collect();
        std::fs::write(
            dir.join(kind.schema_file()),
            json!({ "oneOf": variants }).to_string(),
        )?;
    }
    Ok(())
}

fn codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME").map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".codex"),
        PathBuf::from,
    )
}

/// What waits on the client's answer.
enum Pending {
    Approval { id: String, turn: String },
}

struct Conn {
    out: Box<dyn Write>,
    thread: Option<String>,
    rollout: Option<PathBuf>,
    turn: Option<String>,
    pending: Option<Pending>,
    count: u32,
}

fn text(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

impl Conn {
    fn send(&mut self, value: &Value) {
        let _ = writeln!(self.out, "{value}").and_then(|()| self.out.flush());
    }

    fn notify(&mut self, method: &str, params: &Value) {
        self.send(&json!({"method": method, "params": params}));
    }

    fn reply(&mut self, id: &Value, result: &Value) {
        self.send(&json!({"id": id, "result": result}));
    }

    fn refuse(&mut self, id: &Value, message: &str) {
        self.send(&json!({"id": id, "error": {"code": -32600, "message": message}}));
    }

    /// Appends a rollout line, stamped as Codex stamps them.
    fn record(&self, line: &Value) {
        let mut line = line.clone();
        line["timestamp"] = json!("2026-10-25T10:00:01.000Z");
        if let Some(path) = &self.rollout
            && let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(path)
        {
            let _ = writeln!(file, "{line}");
        }
    }

    fn thread_json(&self, cwd: &str) -> Value {
        json!({"id": self.thread, "cwd": cwd, "cliVersion": "0.163.0", "createdAt": 1,
            "updatedAt": 1, "ephemeral": false, "modelProvider": "openai", "preview": "",
            "projectId": null, "sessionId": self.thread, "source": "appServer",
            "status": {"type": "idle"}, "turns": [],
            "path": self.rollout.as_ref().map(|p| p.to_string_lossy().into_owned())})
    }

    fn rollout_of(id: &str) -> PathBuf {
        codex_home()
            .join("sessions/2026/10/25")
            .join(format!("rollout-2026-10-25T10-00-00-{id}.jsonl"))
    }

    fn thread(&mut self, id: &Value, method: &str, params: &Value) {
        let cwd = text(params, "/cwd");
        if method == "thread/resume" {
            let wanted = text(params, "/threadId");
            if !Self::rollout_of(&wanted).is_file() {
                self.refuse(id, "no rollout for that thread");
                return;
            }
            self.thread = Some(wanted.clone());
            self.rollout = Some(Self::rollout_of(&wanted));
        } else {
            self.count += 1;
            let thread = format!("0199aa00-0000-7000-8000-{:012x}", self.count);
            let path = Self::rollout_of(&thread);
            let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
            let meta = json!({"timestamp": "2026-10-25T10:00:00.000Z", "type": "session_meta",
                "payload": {"id": thread, "cwd": cwd, "cli_version": "0.163.0"}});
            let _ = std::fs::write(&path, format!("{meta}\n"));
            self.thread = Some(thread);
            self.rollout = Some(path);
        }
        let model = params
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("gpt-fake");
        let result = json!({"thread": self.thread_json(&cwd), "model": model,
            "modelProvider": "openai", "cwd": cwd, "approvalPolicy": "on-request",
            "approvalsReviewer": "user", "sandbox": {"type": "readOnly"}});
        self.reply(id, &result);
        let started = json!({"thread": self.thread_json(&cwd)});
        self.notify("thread/started", &started);
    }

    fn status(&mut self, kind: &str, flags: &[&str]) {
        let thread = self.thread.clone();
        let status = if kind == "active" {
            json!({"type": "active", "activeFlags": flags})
        } else {
            json!({"type": kind})
        };
        self.notify(
            "thread/status/changed",
            &json!({"threadId": thread, "status": status}),
        );
    }

    fn finish(&mut self, turn: &str, status: &str, said: &str, error: Option<&Value>) {
        let thread = self.thread.clone();
        let ids = json!({"threadId": thread, "turnId": turn, "itemId": "msg-1"});
        if error.is_none() {
            let mut delta = ids.clone();
            delta["delta"] = json!(said);
            self.notify("item/agentMessage/delta", &delta);
            let item = json!({"threadId": thread, "turnId": turn, "completedAtMs": 1,
                "item": {"type": "agentMessage", "id": "msg-1", "text": said}});
            self.notify("item/completed", &item);
            self.record(
                &json!({"type": "response_item", "payload": {"type": "message", "role": "assistant",
                    "content": [{"type": "output_text", "text": said}]}}),
            );
        }
        let usage = json!({"inputTokens": 100, "cachedInputTokens": 20, "outputTokens": 50,
            "reasoningOutputTokens": 5, "totalTokens": 150});
        self.notify(
            "thread/tokenUsage/updated",
            &json!({"threadId": thread, "turnId": turn,
            "tokenUsage": {"total": usage, "last": usage, "modelContextWindow": 200_000}}),
        );
        self.notify(
            "turn/completed",
            &json!({"threadId": thread,
            "turn": {"id": turn, "status": status, "items": [], "error": error}}),
        );
        self.status("idle", &[]);
        self.notify(
            "account/rateLimits/updated",
            &json!({"rateLimits": limits(13)}),
        );
        self.turn = None;
    }

    fn turn(&mut self, id: &Value, params: &Value) {
        if self.turn.is_some() {
            self.refuse(id, "a turn is running");
            return;
        }
        self.count += 1;
        let turn = format!("turn-{}", self.count);
        self.turn = Some(turn.clone());
        let prompt = text(params, "/input/0/text");
        self.reply(
            id,
            &json!({"turn": {"id": turn, "status": "inProgress", "items": []}}),
        );
        let thread = self.thread.clone();
        self.notify(
            "turn/started",
            &json!({"threadId": thread,
            "turn": {"id": turn, "status": "inProgress", "items": []}}),
        );
        self.status("active", &[]);
        // As a rollout records it (`response_item` messages with content).
        self.record(
            &json!({"type": "response_item", "payload": {"type": "message", "role": "user",
            "content": [{"type": "input_text", "text": prompt}]}}),
        );
        let ids = json!({"threadId": thread, "turnId": turn, "itemId": "item-1", "startedAtMs": 1});
        let request = |method: &str, extra: Value| {
            let mut params = ids.clone();
            if let (Some(params), Some(extra)) = (params.as_object_mut(), extra.as_object()) {
                params.extend(extra.clone());
            }
            (method.to_owned(), params)
        };
        let ask = if let Some(rest) = prompt.split("ask:").nth(1) {
            let words: Vec<&str> = rest.split_whitespace().take(2).collect();
            Some(request(
                "item/commandExecution/requestApproval",
                json!({"command": rest.trim(), "proposedExecpolicyAmendment": words}),
            ))
        } else if let Some(rest) = prompt.split("question:").nth(1) {
            Some(request(
                "item/tool/requestUserInput",
                json!({"isBlocking": true, "questions": [
                {"id": "colour", "header": "Colour", "question": rest.trim(),
                 "options": [{"label": "Red", "description": "warm"}, {"label": "Blue", "description": "cool"}]}]}),
            ))
        } else if prompt.contains("file:") {
            Some(request(
                "item/fileChange/requestApproval",
                json!({"reason": "edit README"}),
            ))
        } else {
            None
        };
        match ask {
            Some((method, params)) => {
                let ask_id = format!("srv-{}", self.count);
                self.status("active", &["waitingOnApproval"]);
                self.send(&json!({"id": ask_id, "method": method, "params": params}));
                self.pending = Some(Pending::Approval { id: ask_id, turn });
            }
            None if prompt.contains("fail:") => {
                let error = json!({"message": "You've hit your usage limit.",
                    "codexErrorInfo": "usageLimitExceeded"});
                self.finish(&turn, "failed", "", Some(&error));
            }
            None => self.finish(&turn, "completed", "done", None),
        }
    }

    fn answered(&mut self, id: &Value, result: &Value) {
        let Some(Pending::Approval { id: ask, turn }) = self.pending.take() else {
            return;
        };
        if id.as_str() != Some(ask.as_str()) {
            return;
        }
        let thread = self.thread.clone();
        self.notify(
            "serverRequest/resolved",
            &json!({"threadId": thread, "requestId": ask}),
        );
        self.status("active", &[]);
        let decision = result.get("decision").cloned().unwrap_or(Value::Null);
        if decision == json!("cancel") {
            self.finish(&turn, "interrupted", "", None);
        } else {
            let said = format!(
                "answer: {}",
                serde_json::to_string(result).unwrap_or_default()
            );
            self.finish(&turn, "completed", &said, None);
        }
    }

    fn handle(&mut self, message: &Value) {
        let id = message.get("id").cloned();
        let method = message.get("method").and_then(Value::as_str);
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        match (id, method) {
            (Some(id), None) => {
                let result = message.get("result").cloned().unwrap_or(Value::Null);
                self.answered(&id, &result);
            }
            (Some(id), Some(method)) => self.request(&id, method, &params),
            (None, _) => {} // `initialized`
        }
    }

    fn request(&mut self, id: &Value, method: &str, params: &Value) {
        match method {
            "initialize" => {
                let home = std::env::var_os("FAKE_CODEX_HOME")
                    .map_or_else(codex_home, PathBuf::from);
                self.reply(id, &json!({"codexHome": home, "userAgent": "fake-codex/0.163.0",
                    "platformFamily": "unix", "platformOs": "macos"}));
            }
            "thread/start" | "thread/resume" => self.thread(id, method, params),
            "turn/start" => self.turn(id, params),
            "turn/steer" => {
                let running = self.turn.clone();
                if running.is_some() && running.as_deref() == params.get("expectedTurnId").and_then(Value::as_str) {
                    self.reply(id, &json!({"turnId": running}));
                    let said = format!("steered: {}", text(params, "/input/0/text"));
                    let thread = self.thread.clone();
                    self.notify("item/agentMessage/delta", &json!({"threadId": thread,
                        "turnId": running, "itemId": "msg-2", "delta": said}));
                } else {
                    self.refuse(id, "no such turn");
                }
            }
            "turn/interrupt" => {
                self.reply(id, &json!({}));
                if let Some(Pending::Approval { id: ask, .. }) = self.pending.take() {
                    let thread = self.thread.clone();
                    self.notify("serverRequest/resolved", &json!({"threadId": thread, "requestId": ask}));
                }
                if let Some(turn) = self.turn.clone() {
                    self.finish(&turn, "interrupted", "", None);
                }
            }
            "thread/compact/start" => {
                self.reply(id, &json!({}));
                let thread = self.thread.clone();
                self.notify("thread/compacted", &json!({"threadId": thread, "turnId": "compact"}));
            }
            "model/list" => self.reply(id, &json!({"nextCursor": null, "data": [
                {"id": "gpt-fake", "model": "gpt-fake", "displayName": "GPT Fake", "isDefault": true,
                 "hidden": false, "description": "", "defaultReasoningEffort": "medium", "supportedReasoningEfforts": []},
                {"id": "gpt-hidden", "model": "gpt-hidden", "displayName": "Hidden", "isDefault": false,
                 "hidden": true, "description": "", "defaultReasoningEffort": "medium", "supportedReasoningEfforts": []}]})),
            "account/rateLimits/read" => {
                self.reply(id, &json!({"rateLimits": limits(12), "rateLimitsByLimitId": null}));
            }
            _ => self.send(&json!({"id": id, "error": {"code": -32601, "message": "not faked"}})),
        }
    }
}

/// As the lab saw it (2026-10-25): `primary` is not the 5-hour window.
fn limits(used: u32) -> Value {
    json!({"limitId": "codex", "planType": "plus",
        "primary": {"usedPercent": 40, "windowDurationMins": 10_080, "resetsAt": 4_102_444_800_i64},
        "secondary": {"usedPercent": used, "windowDurationMins": 300, "resetsAt": 4_102_444_800_i64}})
}

/// Serves the protocol on stdin and stdout until stdin closes (the
/// session's driver let go), as `codex app-server --listen stdio://` does.
pub fn serve_stdio() {
    let mut conn = Conn {
        out: Box::new(std::io::stdout()),
        thread: None,
        rollout: None,
        turn: None,
        pending: None,
        count: 0,
    };
    for line in BufReader::new(std::io::stdin()).lines() {
        let Ok(line) = line else { break };
        if let Ok(message) = serde_json::from_str::<Value>(&line) {
            conn.handle(&message);
        }
    }
}
