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

use std::path::PathBuf;
use std::time::{Duration, Instant};

use juggler_adapter::subagents::LinesChanged;
use juggler_adapters::turn_changes::{FileChange, TurnChanges};
use juggler_app::features::ai::{
    engine::{AiEngine, Stop},
    engines::{apple::Apple, local::Local},
    run::Timed,
};
use juggler_app::features::feed::line::{self, APPLE, LOCAL, Options, TurnView};
use juggler_app::features::review::turn::{Run, Runs, TurnRead, is_test_runner};
use serde_json::{Value, json};

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
    let engine = Local::for_listed(
        &config,
        &model,
        dir,
        arg(args, "--helper").map(PathBuf::from),
    )?;
    Some((engine, model))
}

/// A turn as a recorded round's output holds it (its reading: request,
/// goal, messages, commands and their failed flags, files and lines),
/// assembled by the app's own code: the guard, the review's reading
/// (results first), the question. What the recording didn't keep: which
/// failed command was refused pending approval (the agent's "couldn't run
/// the tests" then decides, G48's backstop).
fn recorded(v: &Value) -> Option<TurnView> {
    let text = |x: &Value| x.as_str().map(str::to_owned);
    let messages: Vec<String> = v["messages"].as_array()?.iter().filter_map(text).collect();
    let mut runs = Runs {
        hidden: u32::try_from(v["calls"]["hidden"].as_u64().unwrap_or(0)).unwrap_or(0),
        ..Runs::default()
    };
    for pair in v["calls"]["by_tool"].as_array().into_iter().flatten() {
        if let (Some(tool), Some(n)) = (pair[0].as_str(), pair[1].as_u64()) {
            runs.by_tool
                .push((tool.to_owned(), u32::try_from(n).unwrap_or(0)));
        }
    }
    for pair in v["calls"]["commands"].as_array().into_iter().flatten() {
        let (Some(command), Some(failed)) = (pair[0].as_str(), pair[1].as_bool()) else {
            continue;
        };
        if is_test_runner(command) {
            runs.test_runs += 1;
            runs.last = Some(if failed { Run::Failed } else { Run::Passed });
        }
        runs.commands.push((command.to_owned(), failed));
    }
    let changes = v["changes"].as_object().map(|c| {
        let per_file: Vec<FileChange> = c["per_file"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|f| FileChange {
                name: f["name"].as_str().unwrap_or_default().to_owned(),
                lines: match (f["added"].as_u64(), f["removed"].as_u64()) {
                    (Some(added), Some(removed)) => Some(LinesChanged { added, removed }),
                    _ => None,
                },
            })
            .collect();
        // Summed only when every file counts its lines (the reader's rule).
        let zero = LinesChanged {
            added: 0,
            removed: 0,
        };
        let lines = per_file.iter().try_fold(zero, |sum, f| {
            f.lines.map(|l| LinesChanged {
                added: sum.added + l.added,
                removed: sum.removed + l.removed,
            })
        });
        TurnChanges {
            start: 0,
            files: u32::try_from(c["files"].as_u64().unwrap_or(0)).unwrap_or(0),
            names: per_file.iter().map(|f| f.name.clone()).collect(),
            lines,
            per_file,
        }
    });
    let turn = TurnRead {
        request: text(&v["request"]),
        opening: text(&v["goal"]),
        reply: messages.last().cloned(),
        messages,
        runs,
    };
    line::assemble(turn, changes, v["failed"].as_bool().unwrap_or(false))
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
        // `--set` rows name a transcript; a recorded round's rows (`view`)
        // are rebuilt through the app's own assembly (0.5.2 re-measure).
        let read = if t.get("view").is_some() {
            recorded(&t["view"])
        } else {
            line::read_at(
                &PathBuf::from(t["transcript"].as_str().unwrap_or_default()),
                t["until_ms"].as_i64(),
                t["ended"].as_str(),
                t["goal"].as_str(),
            )
        };
        let Some(view) = read else {
            results.push(json!({"id": id, "error": "no turn read"}));
            eprintln!("{id}: no turn read");
            continue;
        };
        let plain = line::plain(&view);
        let mut row = json!({"id": id, "view": view_json(&view), "plain": plain});
        if let Some(engine) = engine {
            let timed = Timed::new(Some(engine), &model);
            let stop = Stop::at(Instant::now() + Duration::from_secs(60));
            let began = Instant::now();
            let (shown, attempts) = line::line(&timed, &view, options, &stop);
            let total = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX);
            for failure in timed.failures() {
                eprintln!("engine: {failure}");
            }
            let ms = timed.took_ms();
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
