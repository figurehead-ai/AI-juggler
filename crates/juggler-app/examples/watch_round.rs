//! AJ-E3830, harness v11: the app's own watch drafting
//! (`watches::draft::draft`) for the harness's rounds, so a round measures
//! the code that ships rather than a copy of it. It reads one JSON object
//! per line on stdin and writes one per line on stdout:
//!
//! - `{"op":"draft","id","request","candidates":[{kind,id,name,aliases,
//!   app,project,offered}],"midnight_ms","now_ms"}`: drafts. Each model
//!   call is written as `{"op":"call","step","system","prompt","schema",
//!   "max_tokens"}` and answered on stdin with `{"op":"reply","text",
//!   "finish","error","refused"}` (the harness's runner makes the call);
//!   then `{"op":"final","id","outcome","calls","cover","slots"}`.
//! - `{"op":"read",…}` (the same fields): code's reading only, no call.
//!
//! No model, network or file is touched here: the harness owns the engine.
//! `cargo run --offline -p juggler-app --example watch_round` (harness/v11's
//! `watch7.py` starts it).
#![allow(clippy::print_stdout, reason = "an example's output is what it prints")]

use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::time::{Duration, Instant};

use juggler_app::features::ai::engine::{Draft, Finish, Request, Stop, Usage};
use juggler_app::features::ai::run::{AiFailure, JobModel};
use juggler_app::features::watches::draft::{self, Outcome, cover};
use serde_json::{Value, json};

/// The harness on the other end of stdin and stdout.
struct Harness {
    lines: RefCell<std::io::Lines<std::io::StdinLock<'static>>>,
}

fn say(v: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

impl JobModel for Harness {
    fn generate(&self, request: &Request, _stop: &Stop) -> Result<Draft, AiFailure> {
        let shape = request.shape.clone().unwrap_or(Value::Null);
        let step = match shape["name"].as_str() {
            Some("WatchCandidate") => "target",
            Some("WatchCondition") => "when",
            Some("WatchSpans" | "WatchWordSlots") => "label",
            _ => "other",
        };
        say(&json!({
            "op": "call", "step": step, "system": request.instructions, "prompt": request.prompt,
            "schema": shape, "max_tokens": request.max_tokens,
        }));
        let line = self
            .lines
            .borrow_mut()
            .next()
            .and_then(Result::ok)
            .ok_or(AiFailure::HelperDidntStart)?;
        let reply: Value = serde_json::from_str(&line).map_err(|_| AiFailure::HelperDidntStart)?;
        if reply["refused"].as_bool() == Some(true) {
            return Err(AiFailure::Refused);
        }
        let text = reply["text"].as_str().unwrap_or_default();
        if text.is_empty() {
            return Err(AiFailure::TimedOut);
        }
        Ok(Draft {
            text: text.to_owned(),
            finish: if reply["finish"].as_str() == Some("length") {
                Finish::Length
            } else {
                Finish::Stop
            },
            usage: Usage::default(),
        })
    }
}

fn slot_name(s: Option<cover::Slot>) -> &'static str {
    s.map_or("unplaced", cover::Slot::name)
}

fn main() {
    let harness = Harness {
        lines: RefCell::new(std::io::stdin().lock().lines()),
    };
    loop {
        let Some(Ok(line)) = harness.lines.borrow_mut().next() else {
            break;
        };
        let Ok(job) = serde_json::from_str::<Value>(&line) else {
            say(&json!({ "op": "error", "error": "not JSON" }));
            continue;
        };
        let read = draft::Job::from_json(&job);
        let ask = read.ask();
        let reading = cover::read(&ask);
        let covered = match reading.cover(&ask) {
            cover::Cover::Claimed => json!("claimed"),
            cover::Cover::NoCondition => json!("no_condition"),
            cover::Cover::Untaken(w) => json!({ "untaken": w }),
            cover::Cover::Leftover(left) => json!({ "leftover": left }),
        };
        let slots: Vec<Value> = reading
            .words
            .iter()
            .zip(&reading.slots)
            .map(|(w, s)| json!([w, slot_name(*s)]))
            .collect();
        if job["op"].as_str() == Some("read") {
            say(
                &json!({ "op": "reading", "id": job["id"], "cover": covered, "slots": slots, "quiet_on": reading.quiet_on }),
            );
            continue;
        }
        let stop = Stop::at(Instant::now() + Duration::from_secs(120));
        let (out, calls) = draft::draft(&harness, &ask, &stop, &|_| None);
        let outcome = match out {
            Outcome::NotAWatch => json!("not_a_watch"),
            Outcome::Drafted(d) => serde_json::to_value(&d).unwrap_or(Value::Null),
        };
        say(&json!({
            "op": "final", "id": job["id"], "outcome": outcome, "calls": calls, "cover": covered,
            "slots": slots,
        }));
    }
}
