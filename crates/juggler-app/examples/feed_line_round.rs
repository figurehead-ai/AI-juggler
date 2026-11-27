//! AJ-E3831, harness v14: the app's own feed line (`feed::line`) over a
//! set of recorded lab turns, on a real engine, so a round measures the
//! code that ships: the app's transcript reader, its facts, the plain
//! excerpt, the prompt, the shared check, the retries and the engine
//! (Apple's helper, or the local helper with a downloaded model).
//!
//! ```text
//! cargo run --offline -p juggler-app --example feed_line_round -- \
//!   --set <set.json> --out <out.json> [--engine none|apple|local] \
//!   [--helper <path>] [--model <id> --models-dir <dir> --config <nightly.json>] \
//!   [--datamark on|off] [--prompt apple|local]
//! ```
//!
//! `<set.json>`: `[{"id","transcript","until_ms"?,"ended"?,"goal"?}]`.
//! `--engine none` reads only (the views and the plain excerpts). The
//! local engine reads `<models-dir>/<sha256>.gguf` (a plain file, as the
//! app's store keeps it); nothing is downloaded. Output: one JSON object
//! per turn: its view, the plain excerpt, the line (or none), every
//! attempt with its time.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example's output is what it prints"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use juggler_app::features::ai::engine::AiEngine;
use juggler_app::features::ai::engine::{Draft, Request, Stop};
use juggler_app::features::ai::engines::apple::Apple;
use juggler_app::features::ai::engines::local::{Local, Setup};
use juggler_app::features::ai::models::read_raw;
use juggler_app::features::ai::models::store::Installed;
use juggler_app::features::ai::run::{AiFailure, JobModel};
use juggler_app::features::feed::line::{self, APPLE, LOCAL, Options, TurnView};
use serde_json::{Value, json};

/// One engine and model, timed.
struct Timed<'a> {
    engine: &'a dyn AiEngine,
    model: String,
    ms: std::cell::RefCell<Vec<u64>>,
}

impl JobModel for Timed<'_> {
    fn generate(&self, request: &Request, stop: &Stop) -> Result<Draft, AiFailure> {
        let began = Instant::now();
        let got = self.engine.respond(&self.model, request, stop);
        let ms = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.ms.borrow_mut().push(ms);
        got.map_err(|e| {
            eprintln!("engine: {e:?}");
            AiFailure::TimedOut
        })
    }
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn view_json(v: &TurnView) -> Value {
    json!({
        "request": v.request,
        "goal": v.goal,
        "messages": v.messages,
        "calls": {
            "by_tool": v.calls.by_tool,
            "commands": v.calls.commands,
            "test_runs": v.calls.test_runs,
            "last": v.calls.last.map(|l| format!("{l:?}")),
            "hidden": v.calls.hidden,
        },
        "question": v.question,
        "changes": v.changes.as_ref().map(|c| json!({
            "files": c.files,
            "per_file": c.per_file.iter().map(|f| json!({
                "name": f.name,
                "added": f.lines.map(|l| l.added),
                "removed": f.lines.map(|l| l.removed),
            })).collect::<Vec<_>>(),
        })),
        "facts": {
            "question": v.facts.question,
            "failures": v.facts.failures,
            "tests": v.facts.tests,
            "reverted": v.facts.reverted,
        },
        "failed": v.failed,
        "german": v.german,
        "undone": v.undone(),
        "allowed": line::allowed(v).iter().map(|s| s.key()).collect::<Vec<_>>(),
    })
}

fn local_engine(args: &[String]) -> Option<(Local, String)> {
    let model = arg(args, "--model")?;
    let dir = PathBuf::from(arg(args, "--models-dir")?);
    let config: Value =
        serde_json::from_str(&std::fs::read_to_string(arg(args, "--config")?).ok()?).ok()?;
    let mut section = config["sections"]["models"].clone();
    if let Some(items) = section["items"].as_array_mut() {
        for item in items {
            if let Some(o) = item.as_object_mut() {
                o.remove("when");
            }
        }
    }
    let facts = juggler_remote_config::Facts {
        app_version: juggler_remote_config::Version::new(0, 5, 2),
        channel: juggler_remote_config::Channel::Nightly,
        os: juggler_remote_config::Os::Macos,
        os_version: None,
        tools: BTreeMap::new(),
        install: juggler_remote_config::InstallId::from_bytes([0; 16]),
    };
    let entry = read_raw(&section, &facts)?
        .into_iter()
        .find(|e| e.id == model)?;
    let sha256 = entry.download.as_ref()?.sha256.clone();
    let size_bytes = entry.download.as_ref()?.size_bytes;
    let installed = Installed {
        entry,
        sha256,
        size_bytes,
        installed_at_ms: 0,
    };
    let engine = Local::with(Setup {
        program: arg(args, "--helper").map(PathBuf::from),
        models: Some((dir, vec![installed])),
        ..Setup::default()
    });
    Some((engine, model))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (Some(set), Some(out)) = (arg(&args, "--set"), arg(&args, "--out")) else {
        eprintln!("usage: --set <set.json> --out <out.json> [--engine none|apple|local] …");
        std::process::exit(2);
    };
    let turns: Vec<Value> = std::fs::read_to_string(&set)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let options = Options {
        instructions: if arg(&args, "--prompt").as_deref() == Some("local") {
            LOCAL
        } else {
            APPLE
        },
        datamark: arg(&args, "--datamark").as_deref() != Some("off"),
    };
    let which = arg(&args, "--engine").unwrap_or_else(|| "none".into());
    let apple;
    let local;
    let (engine, model): (Option<&dyn AiEngine>, String) = match which.as_str() {
        "apple" => {
            apple = Apple::with_program(arg(&args, "--helper").map(PathBuf::from));
            apple.probe();
            (Some(&apple), "apple-on-device".into())
        }
        "local" => {
            let Some((engine, model)) = local_engine(&args) else {
                eprintln!("local: --model, --models-dir and --config name an entry that reads");
                std::process::exit(2);
            };
            local = engine;
            local.probe();
            (Some(&local), model)
        }
        _ => (None, String::new()),
    };
    let mut results = Vec::new();
    for t in &turns {
        let id = t["id"].as_str().unwrap_or_default();
        let Some(view) = line::read_at(
            &PathBuf::from(t["transcript"].as_str().unwrap_or_default()),
            t["until_ms"].as_i64(),
            t["ended"].as_str(),
            t["goal"].as_str(),
        ) else {
            results.push(json!({"id": id, "error": "no turn read"}));
            eprintln!("{id}: no turn read");
            continue;
        };
        let plain = line::plain(&view);
        let mut row = json!({"id": id, "view": view_json(&view), "plain": plain});
        if let Some(engine) = engine {
            let timed = Timed {
                engine,
                model: model.clone(),
                ms: std::cell::RefCell::new(Vec::new()),
            };
            let stop = Stop::at(Instant::now() + Duration::from_secs(60));
            let began = Instant::now();
            let (shown, attempts) = line::line(&timed, &view, options, &stop);
            let total = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX);
            let ms = timed.ms.borrow().clone();
            row["line"] = json!(shown);
            row["attempts"] = json!(
                attempts
                    .iter()
                    .zip(ms.iter().chain(std::iter::repeat(&0)))
                    .map(|(a, ms)| json!({"draft": a.draft, "refused": a.refused, "ms": ms}))
                    .collect::<Vec<_>>()
            );
            row["ms_total"] = json!(total);
            eprintln!(
                "{id}: {} ({} attempts, {total} ms)",
                if shown.is_some() { "line" } else { "plain" },
                attempts.len()
            );
        }
        results.push(row);
    }
    let text = serde_json::to_string_pretty(&results).unwrap_or_default();
    if std::fs::write(&out, text).is_err() {
        eprintln!("could not write {out}");
        std::process::exit(1);
    }
}
