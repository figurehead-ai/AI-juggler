//! AJ-E3830 (0.5.2): the app's own pre-fill (`watches::draft::prefill`)
//! over a set of watch requests, on a real engine through the app's own
//! engine code (Apple's helper, or the local helper with a model file the
//! app's store would keep), so the per-field measurement is of the code
//! that ships.
//!
//! ```text
//! cargo run --offline -p juggler-app --example watch_prefill_round -- \
//!   --jobs <jobs.json> --out <out.json> [--engine none|apple|local] \
//!   [--helper <path>] [--model <id> --models-dir <dir> --config <nightly.json>]
//! ```
//!
//! `<jobs.json>`: the harness's jobs (`[{"id","request","candidates":[…],
//! "midnight_ms","now_ms"}]`, `docs/qa/evidence/AJ-E3830/prefill/
//! prefill_jobs.py`). `--engine none` is code alone. The local engine reads
//! `<models-dir>/<sha256>.gguf`; nothing is downloaded. Output: one JSON
//! object per request: the pre-fill, the labels, every call's time.
#![allow(
    clippy::print_stderr,
    reason = "the round's progress, one line a request"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use juggler_app::features::ai::engine::{AiEngine, Stop};
use juggler_app::features::ai::engines::apple::Apple;
use juggler_app::features::ai::engines::local::Local;
use juggler_app::features::ai::run::Timed;
use juggler_app::features::watches::draft::{self, Job, prefill};
use serde_json::{Value, json};

/// The engine the flags name, and its model id (none: code alone).
enum Engine {
    None,
    Apple(Box<Apple>),
    Local(Box<Local>, String),
}

impl Engine {
    fn of(flags: &BTreeMap<String, String>) -> Result<Self, String> {
        let helper = flags.get("--helper").map(PathBuf::from);
        match flags.get("--engine").map(String::as_str) {
            Some("apple") => Ok(Self::Apple(Box::new(Apple::with_program(helper)))),
            Some("local") => {
                let model = flags.get("--model").cloned().unwrap_or_default();
                let config: Value = flags
                    .get("--config")
                    .and_then(|p| std::fs::read_to_string(p).ok())
                    .and_then(|t| serde_json::from_str(&t).ok())
                    .unwrap_or_default();
                let dir = PathBuf::from(flags.get("--models-dir").cloned().unwrap_or_default());
                Local::for_listed(&config, &model, dir, helper)
                    .map(|engine| Self::Local(Box::new(engine), model))
                    .ok_or_else(|| {
                        "--model, --models-dir and --config name no entry that reads".to_owned()
                    })
            }
            None | Some("none") => Ok(Self::None),
            Some(other) => Err(format!("no engine {other}")),
        }
    }

    fn parts(&self) -> (Option<&dyn AiEngine>, &str) {
        match self {
            Self::None => (None, ""),
            Self::Apple(apple) => (Some(apple.as_ref()), "apple-on-device"),
            Self::Local(local, model) => (Some(local.as_ref()), model),
        }
    }
}

/// One request through the app's own path: the traced draft, then the
/// pre-fill from it (`card` here: every field verified; the app also asks
/// the form's own check and the user's Beta).
fn one(job: &Value, engine: Option<&dyn AiEngine>, model: &str) -> Value {
    let read = Job::from_json(job);
    let ask = read.ask();
    let timed = Timed::new(engine, model);
    let began = Instant::now();
    let stop = Stop::at(Instant::now() + Duration::from_secs(120));
    let (outcome, calls, trace) = draft::draft_traced(&timed, &ask, &stop, &|_| None);
    let kind = match &outcome {
        draft::Outcome::NotAWatch => "not_a_watch".to_owned(),
        draft::Outcome::Drafted(d) => serde_json::to_value(d)
            .ok()
            .and_then(|v| v["kind"].as_str().map(str::to_owned))
            .unwrap_or_default(),
    };
    let pre = prefill::from_trace(&ask, outcome, &trace).map(|mut p| {
        p.card = p.left.is_empty();
        p
    });
    let unplaced: Vec<&str> = trace.reading.as_ref().map_or_else(Vec::new, |r| {
        r.words
            .iter()
            .zip(&r.slots)
            .filter(|(_, s)| s.is_none())
            .map(|(w, _)| w.as_str())
            .collect()
    });
    json!({
        "id": job["id"],
        "draft_kind": kind,
        "prefill": pre,
        "calls": calls,
        "call_ms": timed.took_ms(),
        "ms": u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX),
        "unplaced": unplaced,
        "labels": trace.labels.as_ref().map(|l| l.iter().map(|s| s.name()).collect::<Vec<_>>()),
        "negated": trace.negated.is_some(),
        "picked": trace.picked,
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flags: BTreeMap<String, String> = args
        .chunks(2)
        .filter_map(|pair| Some((pair.first()?.clone(), pair.get(1)?.clone())))
        .collect();
    let (Some(jobs_path), Some(out)) = (flags.get("--jobs"), flags.get("--out")) else {
        eprintln!("usage: --jobs <jobs.json> --out <out.json> [--engine none|apple|local] …");
        std::process::exit(2);
    };
    let engine = Engine::of(&flags).unwrap_or_else(|why| {
        eprintln!("{why}");
        std::process::exit(2);
    });
    let (dyn_engine, model) = engine.parts();
    if let Some(e) = dyn_engine {
        e.probe();
    }
    let jobs: Vec<Value> = std::fs::read_to_string(jobs_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let began = Instant::now();
    let results: Vec<Value> = jobs
        .iter()
        .map(|job| {
            let r = one(job, dyn_engine, model);
            eprintln!("{} {} ms", r["id"], r["ms"]);
            r
        })
        .collect();
    let meta = json!({
        "engine": flags.get("--engine").map_or("none", String::as_str),
        "model": model,
        "items": results.len(),
        "wall_ms": u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX),
    });
    let text = serde_json::to_string_pretty(&json!({ "meta": meta, "results": results }))
        .unwrap_or_default();
    if let Err(e) = std::fs::write(out, text) {
        eprintln!("write {out}: {e}");
        std::process::exit(1);
    }
}
