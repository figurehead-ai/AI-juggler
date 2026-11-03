//! QA on a real setup, read only: Usage's numbers as this build would show
//! them, recomputed from the tools' files, for comparing with an
//! independent recount (0.3.6, the usage audit).
//!
//! Ignored, and a no-op unless `AIJ_QA_USAGE_DB` and `AIJ_QA_USAGE_OUT` are
//! set, so it never runs in CI or on a plain `cargo test`:
//!
//! ```sh
//! cargo test -p juggler-app --test qa_usage_recount --no-run
//! AIJ_QA_USAGE_DB="$HOME/Library/Application Support/AI Juggler/juggler.sqlite" \
//!   AIJ_QA_USAGE_OUT=<folder outside the repository> <test binary> --ignored --nocapture
//! ```
//!
//! It opens AI Juggler's database strictly read-only (`mode=ro&immutable=1`)
//! for its accounts and the keys of the calls it counts, reads each
//! account's transcripts or rollouts with the adapters' own readers, keeps
//! the calls that database counts (so history a switch carried, or a call
//! written after it, is left out as the app leaves it out), stores them in
//! a temporary store (pricing each as the app does), and folds them with
//! Usage's own fold. It writes per-account aggregates (`summary.json`) and
//! one line per call (`calls.csv`: key, model, time, classes, detail, cost;
//! numbers and ids, no message text) into the output folder. It never
//! writes into a tool's folder or the database it reads.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use juggler_adapters::quota_ingest::{RolloutCarry, tail_rollout, tail_transcript};
use juggler_app::features::usage_history::fold;
use juggler_core::cost_ledger::PricingCatalog;
use juggler_core::quota::UsageEvent;
use juggler_core::storage::Storage;

/// As the pipeline's walk (`usage_pipeline::MAX_WALK_DEPTH`).
const MAX_WALK_DEPTH: u32 = 5;

fn walk(root: &Path, out: &mut Vec<PathBuf>, keep: &dyn Fn(&Path) -> bool, depth: u32) {
    if depth > MAX_WALK_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry
            .file_type()
            .is_ok_and(|kind| kind.is_dir() || (kind.is_symlink() && path.is_dir()))
        {
            walk(&path, out, keep, depth + 1);
        } else if keep(&path) {
            out.push(path);
        }
    }
}

/// Every event of one file, read to its end as the pipeline's passes do.
fn read_all(path: &Path, account: &str, codex: bool) -> Vec<UsageEvent> {
    let len = std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    let mut events = Vec::new();
    let mut offset = 0;
    let mut carry = RolloutCarry::default();
    loop {
        let outcome = if codex {
            tail_rollout(path, account, offset, &carry)
        } else {
            tail_transcript(path, account, offset)
        };
        events.extend(outcome.events);
        carry = outcome.carry;
        if outcome.offset <= offset || outcome.offset >= len || outcome.unread.is_some() {
            break;
        }
        offset = outcome.offset;
    }
    events
}

#[test]
#[ignore = "QA on a real setup; needs AIJ_QA_USAGE_DB and AIJ_QA_USAGE_OUT"]
fn usage_numbers_recomputed_from_the_real_files() {
    let (Some(db), Some(out)) = (
        std::env::var_os("AIJ_QA_USAGE_DB").map(PathBuf::from),
        std::env::var_os("AIJ_QA_USAGE_OUT").map(PathBuf::from),
    ) else {
        eprintln!("AIJ_QA_USAGE_DB / AIJ_QA_USAGE_OUT not set: nothing to do");
        return;
    };
    std::fs::create_dir_all(&out).unwrap();
    // Strictly read-only: the app may be running and writing it.
    let uri = format!(
        "file:{}?mode=ro&immutable=1",
        db.to_string_lossy().replace(' ', "%20")
    );
    let real = rusqlite::Connection::open_with_flags(
        &uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .unwrap();
    let accounts: Vec<(String, String, String)> = real
        .prepare("SELECT id, harness, state_dir FROM accounts ORDER BY state_dir")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let counted: HashSet<(String, String)> = real
        .prepare("SELECT account, msg_uuid FROM usage_events WHERE msg_uuid IS NOT NULL")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    drop(real);

    let scratch = tempfile::tempdir().unwrap();
    let storage = Storage::open(&scratch.path().join("recount.sqlite")).unwrap();
    let catalog = PricingCatalog::with_overrides(Vec::new());
    let mut summary = serde_json::Map::new();
    let mut calls = std::io::BufWriter::new(std::fs::File::create(out.join("calls.csv")).unwrap());
    writeln!(
        calls,
        "folder,key,model,ts,input,output,cache_read,cache_write,reasoning,cache_write_1h,prompt,speed,geo,kind,root,usd"
    )
    .unwrap();
    for (id, harness, state_dir) in &accounts {
        let codex = harness == "codex";
        if !codex && harness != "claude-code" {
            continue;
        }
        let folder = Path::new(state_dir)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let root = Path::new(state_dir).join(if codex { "sessions" } else { "projects" });
        let mut files = Vec::new();
        let keep: &dyn Fn(&Path) -> bool = if codex {
            &|path: &Path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("rollout-"))
            }
        } else {
            &|path: &Path| path.extension().is_some_and(|ext| ext == "jsonl")
        };
        walk(&root, &mut files, keep, 0);
        files.sort();
        let mut read = 0usize;
        for chunk in files.chunks(200) {
            let mut events = Vec::new();
            for path in chunk {
                events.extend(read_all(path, id, codex).into_iter().filter(|event| {
                    event
                        .msg_uuid
                        .as_ref()
                        .is_some_and(|key| counted.contains(&(id.clone(), key.clone())))
                }));
            }
            read += events.len();
            storage.usage().store_events(&events).unwrap();
        }
        let (rows, cut) = storage
            .usage_history()
            .hourly(id, 0, i64::MAX, 10_000_000)
            .unwrap();
        assert!(!cut);
        let labelled: Vec<(String, juggler_core::storage::HourRow)> =
            rows.into_iter().map(|row| (id.clone(), row)).collect();
        let price = |model: Option<&str>, at: i64, tokens| catalog.price_tokens(model?, at, tokens);
        let folded = fold::fold(
            &labelled,
            &[0, i64::MAX],
            "profile",
            &fold::Lookups::default(),
            &price,
        );
        let in_db = counted.iter().filter(|(account, _)| account == id).count();
        let events = storage.usage().events_for_account(id, 0).unwrap();
        let mut kinds: BTreeMap<&str, (u64, u64)> = BTreeMap::new();
        let mut unpriced_models: BTreeMap<String, u64> = BTreeMap::new();
        for event in &events {
            let entry = kinds.entry(event.detail.kind.as_str()).or_default();
            entry.0 += 1;
            entry.1 += fold::io(&event.tokens);
            if event.usd_est.is_none() {
                *unpriced_models
                    .entry(event.model.clone().unwrap_or_default())
                    .or_default() += fold::io(&event.tokens) + event.tokens.cache_read;
            }
            writeln!(
                calls,
                "{folder},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                event.msg_uuid.as_deref().unwrap_or_default(),
                event.model.as_deref().unwrap_or_default(),
                event.ts,
                event.tokens.input,
                event.tokens.output,
                event.tokens.cache_read,
                event.tokens.cache_write,
                event.tokens.reasoning,
                event.detail.cache_write_1h,
                event.detail.prompt_tokens,
                event.detail.speed.as_deref().unwrap_or_default(),
                event.detail.geo.as_deref().unwrap_or_default(),
                event.detail.kind.as_str(),
                event.detail.root_session.as_deref().unwrap_or_default(),
                event
                    .usd_est
                    .map(|usd| format!("{usd:.12}"))
                    .unwrap_or_default(),
            )
            .unwrap();
        }
        let threads: HashSet<&str> = events
            .iter()
            .filter_map(|event| event.session.as_deref())
            .collect();
        let totals = &folded.totals;
        summary.insert(
            folder.clone(),
            serde_json::json!({
                "tool": harness,
                "files": files.len(),
                "calls_in_db": in_db,
                "calls_read": read,
                "calls_stored": events.len(),
                "tokens": {
                    "input": totals.tokens.input, "output": totals.tokens.output,
                    "cache_read": totals.tokens.cache_read, "cache_write": totals.tokens.cache_write,
                    "reasoning": totals.tokens.reasoning,
                },
                "new_tokens": folded.by_profile.first().map_or(0, |share| share.io),
                "cache_reads_shown_apart": folded.by_profile.first().map_or(0, |share| share.cache),
                "usd": totals.usd,
                "unpriced": totals.unpriced,
                "unpriced_tokens_by_model": unpriced_models,
                "sessions": totals.sessions,
                "threads_with_usage": threads.len(),
                "calls_and_new_tokens_by_kind": kinds,
                "active_hours": totals.active_hours,
            }),
        );
        eprintln!("{folder}: {} calls", events.len());
    }
    calls.flush().unwrap();
    std::fs::write(
        out.join("summary.json"),
        serde_json::to_string_pretty(&summary).unwrap(),
    )
    .unwrap();
}
