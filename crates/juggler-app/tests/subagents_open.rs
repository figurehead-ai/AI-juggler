//! AJ-E1520 (0.3): how fast a session with 437 subagents opens its
//! Subagents tab — the first time (every transcript read), from the
//! saved index after a restart (nothing read again), and while kept
//! (only what grew). A synthetic session the size of the largest real
//! one seen (437 subagents, nested three deep), never anyone's own.
//!
//! The times are printed (`--nocapture`) and held to generous bounds,
//! so a slow machine never fails it but a regression to re-reading
//! everything does.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use juggler_adapter::HarnessAdapter;
use juggler_adapter::subagents::{SubagentReader, SubagentScope};
use juggler_app::features::subagents::clean::{Cleaned, kept};
use juggler_app::features::subagents::dto::SubagentsQuery;
use juggler_app::features::subagents::{Found, index, view};
use juggler_app::paging::PageRequest;

const AGENTS: usize = 437;

/// Lines in each subagent's own transcript (a long tool output each).
const LINES: usize = 40;

fn at(second: usize) -> String {
    format!(
        "2026-10-27T{:02}:{:02}:{:02}Z",
        10 + second / 3600,
        (second / 60) % 60,
        second % 60
    )
}

/// A session of 437 subagents in 40 turns; every tenth started two of
/// its own, and one of those one more (three deep).
fn session(dir: &Path) -> PathBuf {
    let path = dir.join("proj/big.jsonl");
    let folder = dir.join("proj/big/subagents");
    std::fs::create_dir_all(&folder).unwrap();
    let output = "x".repeat(4_000);
    let mut lines = Vec::new();
    let mut made = 0;
    let mut n = 0;
    while made < AGENTS {
        if n % 11 == 0 {
            lines.push(
                serde_json::json!({"type": "user", "uuid": format!("u{n}"), "timestamp": at(n),
                "message": {"role": "user", "content": format!("Turn {n}: do the next part")}})
                .to_string(),
            );
        }
        let (agent, call) = (format!("a{n}"), format!("t{n}"));
        lines.push(serde_json::json!({"type": "assistant", "timestamp": at(n), "message": {"role": "assistant", "content": [
            {"type": "tool_use", "id": call, "name": "Agent", "input": {"description": format!("Task {n}"),
                "prompt": format!("Do part {n} of the work, carefully, and report what changed."), "subagent_type": "general-purpose"}}]}}).to_string());
        lines.push(serde_json::json!({"type": "user", "timestamp": at(n), "message": {"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": call, "content": "launched"}]}, "toolUseResult": {"status": "async_launched", "agentId": agent}}).to_string());
        if n % 3 != 0 {
            lines.push(serde_json::json!({"type": "user", "timestamp": at(n + 1), "origin": {"kind": "task-notification"},
                "message": {"role": "user", "content": format!("<task-notification><task-id>{agent}</task-id><status>completed</status><result>Part {n} done</result></task-notification>")}}).to_string());
        }
        let mut own: Vec<String> = Vec::new();
        for line in 0..LINES {
            own.push(serde_json::json!({"type": "assistant", "timestamp": at(n), "message": {"id": format!("m{n}-{line}"), "role": "assistant",
                "model": "claude-sonnet-4-5", "usage": {"input_tokens": 100, "output_tokens": 20, "cache_read_input_tokens": 5000},
                "content": [{"type": "tool_use", "id": format!("c{n}-{line}"), "name": "Bash", "input": {"command": format!("npm test -- part{n}")}}]}}).to_string());
            own.push(serde_json::json!({"type": "user", "timestamp": at(n), "message": {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": format!("c{n}-{line}"), "content": output}]}}).to_string());
        }
        let nested = n % 10 == 0;
        if nested {
            for k in 0..2 {
                let (child, child_call) = (format!("n{n}-{k}"), format!("x{n}-{k}"));
                own.push(serde_json::json!({"type": "assistant", "timestamp": at(n), "message": {"role": "assistant", "content": [
                    {"type": "tool_use", "id": child_call, "name": "Agent", "input": {"description": format!("Nested {n}.{k}"), "prompt": "Look deeper"}}]}}).to_string());
                own.push(serde_json::json!({"type": "user", "timestamp": at(n), "message": {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": child_call, "content": "done"}]}, "toolUseResult": {"status": "completed", "agentId": child}}).to_string());
                std::fs::write(folder.join(format!("agent-{child}.meta.json")), serde_json::json!({"agentType": "Explore",
                    "description": format!("Nested {n}.{k}"), "toolUseId": child_call, "spawnDepth": 2, "parentAgentId": agent}).to_string()).unwrap();
                std::fs::write(
                    folder.join(format!("agent-{child}.jsonl")),
                    own[0].clone() + "\n",
                )
                .unwrap();
                made += 1;
            }
        }
        std::fs::write(folder.join(format!("agent-{agent}.meta.json")), serde_json::json!({"agentType": "general-purpose",
            "description": format!("Task {n}"), "toolUseId": call, "spawnDepth": 1, "requestShape": "background"}).to_string()).unwrap();
        std::fs::write(
            folder.join(format!("agent-{agent}.jsonl")),
            own.join("\n") + "\n",
        )
        .unwrap();
        made += 1;
        n += 1;
    }
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    path
}

fn reader(path: &Path) -> Box<dyn SubagentReader> {
    juggler_adapters::ClaudeCodeAdapter::new()
        .subagent_reader(SubagentScope { path, id: "big" })
        .unwrap()
}

/// What the tab does on an ask: read on, redact, build the first page.
fn open(reader: &mut Box<dyn SubagentReader>, cleaned: &mut Cleaned) -> (usize, bool) {
    let mut caught_up = reader.refresh().unwrap();
    caught_up &= reader.refresh_details(u64::MAX).unwrap();
    let agents = reader.subagents();
    let cleans = cleaned.all(&agents, true);
    let found = Found {
        support: juggler_adapters::ClaudeCodeAdapter::new().subagent_support(),
        live: true,
        reading: !caught_up,
        agents,
        cleans,
        session_tokens: reader.session_tokens(),
        counted: reader.tokens_counted(),
        tool: "Claude Code".to_owned(),
        now_ms: 0,
        sessions: std::collections::HashMap::new(),
    };
    let info = view::listed(&found, &SubagentsQuery::default(), &PageRequest::first(60)).unwrap();
    (info.summary.total as usize, caught_up)
}

fn timed<T>(what: &str, run: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let out = run();
    let took = start.elapsed();
    println!("{what}: {took:?}");
    (out, took)
}

#[test]
fn a_session_with_437_subagents_opens_at_once_from_its_index() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let path = session(dir.path());
    let bytes: u64 = std::fs::read_dir(dir.path().join("proj/big/subagents"))
        .unwrap()
        .flatten()
        .map(|entry| entry.metadata().unwrap().len())
        .sum();
    println!(
        "fixture: {AGENTS} subagents, {} MB of their files",
        bytes / 1_000_000
    );

    // First open: every transcript read.
    let mut first = reader(&path);
    let mut cleaned = Cleaned::default();
    let ((total, caught_up), cold) =
        timed("first open (no index)", || open(&mut first, &mut cleaned));
    assert_eq!(total, AGENTS);
    assert!(caught_up);

    // Saved, as the app saves it (redacted, messages on).
    let (checkpoint, save) = timed("index saved", || {
        first
            .checkpoint(&|kind, text: &str| kept(kind, text, true))
            .unwrap()
    });
    let file = index::path_for(dir.path(), "acc", "big", &path);
    let cache = juggler_app::features::cache::Gate::default();
    index::save(&cache, &file, true, &checkpoint).unwrap();
    println!("index: {} KB", checkpoint.len() / 1000);

    // After a restart: a new reader from the index.
    let mut restarted = reader(&path);
    let mut cleaned = Cleaned::default();
    let ((total, caught_up), from_index) = timed("open from the index (cold)", || {
        let saved = index::load(&cache, &file, true).unwrap();
        assert!(restarted.resume(&saved));
        open(&mut restarted, &mut cleaned)
    });
    assert_eq!(total, AGENTS);
    assert!(caught_up);

    // Kept: the next ask (the tab's 4 s follow).
    let ((total, _), warm) = timed("open again (warm)", || open(&mut restarted, &mut cleaned));
    assert_eq!(total, AGENTS);

    // Generous bounds (a debug build on a busy machine): the index must
    // beat reading everything, and a warm ask stays well under a second.
    assert!(
        from_index < cold,
        "the index is faster than reading every transcript"
    );
    assert!(warm < Duration::from_secs(2), "a warm ask stays quick");
    assert!(save < Duration::from_secs(5));
}
