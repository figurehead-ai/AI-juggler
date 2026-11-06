//! What following a session with many subagents costs, on a synthetic
//! session (nothing of an account is read): the rail count's pass (every
//! 2 s) and the details pass (every 10 s), each timed on its own.
//!
//! `cargo run --release -p juggler-adapters --example subagents_cost -- [subagents] [KiB per transcript]`

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::unwrap_used,
    reason = "a measuring tool on its own synthetic files: a failure ends the run"
)]

use std::io::Write as _;
use std::path::Path;
use std::time::Instant;

use juggler_adapter::subagents::{SubagentScope, SubagentText};

fn line(value: &serde_json::Value, out: &mut impl std::io::Write) {
    let _ = writeln!(out, "{value}");
}

fn build(dir: &Path, agents: usize, kib: usize) -> std::path::PathBuf {
    let id = "0b1c2d3e-0000-4000-8000-0000000000aa";
    let project = dir.join("projects/-cost");
    let folder = project.join(id).join("subagents");
    std::fs::create_dir_all(&folder).unwrap();
    let session = project.join(format!("{id}.jsonl"));
    let mut out = std::io::BufWriter::new(std::fs::File::create(&session).unwrap());
    for n in 0..agents {
        let call = format!("t{n}");
        line(
            &serde_json::json!({"type": "assistant", "sessionId": id, "timestamp": "2026-10-21T10:00:01Z",
            "message": {"role": "assistant", "content": [{"type": "tool_use", "id": call, "name": "Agent",
                "input": {"description": format!("Task {n}"), "prompt": "do the thing", "subagent_type": "worker"}}]}}),
            &mut out,
        );
        let status = if n % 10 == 0 {
            "async_launched"
        } else {
            "completed"
        };
        line(
            &serde_json::json!({"type": "user", "sessionId": id, "timestamp": "2026-10-21T10:00:09Z",
            "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": call, "content": "ok"}]},
            "toolUseResult": {"status": status, "agentId": format!("a{n}")}}),
            &mut out,
        );
        let meta = serde_json::json!({"agentType": "worker", "description": format!("Task {n}"),
            "toolUseId": call, "spawnDepth": 1, "requestShape": "foreground"});
        std::fs::write(
            folder.join(format!("agent-a{n}.meta.json")),
            meta.to_string(),
        )
        .unwrap();
        let mut own = std::io::BufWriter::new(
            std::fs::File::create(folder.join(format!("agent-a{n}.jsonl"))).unwrap(),
        );
        let mut written = 0;
        let mut turn = 0;
        while written < kib * 1024 {
            let value = serde_json::json!({"type": "assistant", "isSidechain": true, "agentId": format!("a{n}"),
                "timestamp": "2026-10-21T10:00:12Z", "message": {"role": "assistant", "model": "m",
                "usage": {"input_tokens": 10, "output_tokens": 20},
                "content": [{"type": "tool_use", "id": format!("b{n}-{turn}"), "name": "Bash",
                    "input": {"command": format!("cargo test -p crate{turn} -- a long enough command line {}", "x".repeat(200))}}]}});
            let text = value.to_string();
            written += text.len() + 1;
            let _ = writeln!(own, "{text}");
            turn += 1;
        }
    }
    session
}

fn main() {
    let mut args = std::env::args().skip(1);
    let agents: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(275);
    let kib: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(200);
    let dir = std::env::temp_dir().join(format!("aij-subagents-cost-{}", std::process::id()));
    let session = build(&dir, agents, kib);
    let scope = SubagentScope {
        path: &session,
        id: "0b1c2d3e-0000-4000-8000-0000000000aa",
    };
    let mut reader = juggler_adapters::claude_subagents::reader(scope).unwrap();

    let started = Instant::now();
    while !reader.refresh().unwrap() {}
    println!("first refresh: {:?}", started.elapsed());

    let passes = 50;
    let started = Instant::now();
    for _ in 0..passes {
        let _ = reader.refresh();
        let _ = reader.running();
    }
    let count = started.elapsed() / passes;
    println!("count pass (refresh + running), at rest: {count:?}");

    let started = Instant::now();
    while !reader.refresh_details(8 * 1024 * 1024).unwrap() {}
    println!("details caught up: {:?}", started.elapsed());

    let filter = |_: SubagentText, text: &str| Some(text.to_owned());
    let mut details = std::time::Duration::ZERO;
    let mut listing = std::time::Duration::ZERO;
    let mut saving = std::time::Duration::ZERO;
    let rounds = 10;
    let mut bytes = 0;
    for _ in 0..rounds {
        let started = Instant::now();
        let _ = reader.refresh();
        let _ = reader.refresh_details(8 * 1024 * 1024);
        details += started.elapsed();
        let started = Instant::now();
        let listed = reader.subagents();
        listing += started.elapsed();
        assert_eq!(listed.len(), agents);
        let started = Instant::now();
        bytes = reader.checkpoint(&filter).map_or(0, |saved| saved.len());
        saving += started.elapsed();
    }
    println!(
        "details pass, at rest: refresh {:?}, list {:?}, checkpoint {:?} ({} KiB)",
        details / rounds,
        listing / rounds,
        saving / rounds,
        bytes / 1024
    );
    let at_rest = count * 5 + (details + listing) / rounds;
    println!(
        "per 10 s at rest: {at_rest:?} ({:.2}% of a core), plus a checkpoint every 15 s",
        at_rest.as_secs_f64() / 10.0 * 100.0
    );
    let _ = std::fs::remove_dir_all(&dir);
}
