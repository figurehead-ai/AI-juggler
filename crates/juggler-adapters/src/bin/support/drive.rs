//! `fake-harness drive` — the fake tool's own protocol (AJ-E2302), so the
//! runtime contract and the app's driver are tested end to end without a
//! real tool:
//!
//! ```text
//! fake-harness drive --dir DIR [--session-id ID | --resume ID]
//! ```
//!
//! One JSON object per line each way, on stdin and stdout. AI Juggler
//! sends `{"op":"turn","text":…}`, `{"op":"answer","id":…,"option":…}`,
//! `{"op":"reply","id":…,"answers":[…]}`, `{"op":"interrupt"}`,
//! `{"op":"model","model":…}`, `{"op":"limits"}`. The fake sends
//! `ready`, `turn_started`, `text`, `ask` (an approval, when the turn's
//! text holds `ask:`), `question` (when it holds `question:`), `resolved`,
//! `turn_end`, `limits` and `model`. Every turn lands in the session's
//! transcript as `run` writes it, so the readers find a driven session's
//! conversation like any other. It exits when stdin closes.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

struct Pending {
    id: String,
    question: bool,
    turn: u32,
}

struct Fake {
    transcript: File,
    turns: u32,
    pending: Option<Pending>,
    used: u32,
}

fn say(value: &Value) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{value}").and_then(|()| out.flush());
}

fn text(message: &Value, key: &str) -> String {
    message
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

impl Fake {
    fn record(&mut self, line: &str) {
        let _ = writeln!(self.transcript, "{line}").and_then(|()| self.transcript.flush());
    }

    fn finish(&mut self, turn: u32, outcome: &str, said: &str) {
        say(&json!({"ev": "text", "text": said}));
        let line = json!({"type": "assistant", "turn": turn, "text": said,
            "usage": {"input_tokens": 100, "output_tokens": 50}});
        self.record(&line.to_string());
        say(&json!({"ev": "turn_end", "outcome": outcome, "input": 100, "output": 50}));
    }

    fn turn(&mut self, prompt: &str) {
        self.turns += 1;
        let turn = self.turns;
        say(&json!({"ev": "turn_started", "turn": turn}));
        self.record(&json!({"type": "user", "turn": turn, "text": prompt}).to_string());
        if let Some(rest) = prompt.split("ask:").nth(1) {
            let id = format!("ask-{turn}");
            say(&json!({"ev": "ask", "id": id, "summary": rest.trim(),
                "options": [["once", "Allow once"], ["always", "Always allow this"], ["deny", "Deny"]]}));
            self.pending = Some(Pending {
                id,
                question: false,
                turn,
            });
        } else if let Some(rest) = prompt.split("question:").nth(1) {
            let id = format!("q-{turn}");
            say(
                &json!({"ev": "question", "id": id, "question": rest.trim(), "choices": ["Red", "Blue"]}),
            );
            self.pending = Some(Pending {
                id,
                question: true,
                turn,
            });
        } else {
            self.finish(turn, "completed", "done");
        }
    }

    fn answer(&mut self, message: &Value) {
        let Some(open) = self.pending.take_if(|open| open.id == text(message, "id")) else {
            say(&json!({"ev": "error", "message": "no such ask"}));
            return;
        };
        say(&json!({"ev": "resolved", "id": open.id}));
        let said = if open.question {
            let answers: Vec<&str> = message
                .get("answers")
                .and_then(Value::as_array)
                .map(|list| list.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            format!("you said {}", answers.join(", "))
        } else {
            format!("you chose {}", text(message, "option"))
        };
        let outcome = if text(message, "option") == "deny" {
            "interrupted"
        } else {
            "completed"
        };
        self.finish(open.turn, outcome, &said);
    }

    fn handle(&mut self, message: &Value) {
        match text(message, "op").as_str() {
            "turn" if self.pending.is_none() => self.turn(&text(message, "text")),
            "turn" => say(&json!({"ev": "error", "message": "a turn is running"})),
            "answer" | "reply" => self.answer(message),
            "interrupt" => {
                if let Some(open) = self.pending.take() {
                    say(&json!({"ev": "resolved", "id": open.id}));
                }
                say(&json!({"ev": "turn_end", "outcome": "interrupted", "input": 0, "output": 0}));
            }
            "model" => say(&json!({"ev": "model", "model": text(message, "model")})),
            "limits" => {
                self.used += 1;
                say(
                    &json!({"ev": "limits", "used": self.used, "window_mins": 300,
                    "resets_at": 4_102_444_800_000_i64}),
                );
            }
            _ => say(&json!({"ev": "error", "message": "unknown op"})),
        }
    }
}

/// `(transcript folder, session id, resumed)` from the arguments.
fn parse(args: &[String]) -> Result<(PathBuf, String, bool), String> {
    let mut dir = PathBuf::from(".");
    let mut session: Option<String> = None;
    let mut resume: Option<String> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = || rest.next().cloned().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--dir" => dir = PathBuf::from(value()?),
            "--session-id" => session = Some(value()?),
            "--resume" => resume = Some(value()?),
            // The agent bridge's file (E18: a driven session with the
            // agent tools on); this fake speaks no MCP in `drive`.
            "--mcp-config" => {
                value()?;
            }
            other => return Err(format!("unknown option {other}")),
        }
    }
    let resumed = resume.is_some();
    let id = resume
        .or(session)
        .unwrap_or_else(|| "fake-driven".to_owned());
    Ok((dir, id, resumed))
}

/// Runs the protocol until stdin closes.
pub fn run(args: &[String], header: &dyn Fn(&str, bool) -> String) -> Result<(), String> {
    let (dir, id, resumed) = parse(args)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{id}.jsonl"));
    let existed = path.is_file();
    let transcript = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let mut fake = Fake {
        transcript,
        turns: 0,
        pending: None,
        used: 10,
    };
    fake.record(&header(&id, resumed && existed));
    say(&json!({"ev": "ready", "id": id, "model": "fake-1"}));
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        match serde_json::from_str::<Value>(&line) {
            Ok(message) => fake.handle(&message),
            Err(_) => say(&json!({"ev": "error", "message": "unreadable"})),
        }
    }
    fake.record(&json!({"type": "ended", "id": id}).to_string());
    Ok(())
}
