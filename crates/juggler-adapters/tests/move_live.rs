//! ADR-0031 (AJ-E212 R14), live: Move to account between the sanctioned
//! `aij-test` Claude app profile and a scratch profile made for the test,
//! through the real planner and the real engine. Opt-in (`--ignored`), run
//! by hand, one phase at a time, with both profiles quit by their pid for
//! `seed`, `move` and `undo` (the harness refuses while either runs):
//!
//! ```sh
//! AIJ_MOVE_PHASE=seed|move|link|after|undo \
//! AIJ_MOVE_FROM_HOST=… AIJ_MOVE_FROM_CODE=… \
//! AIJ_MOVE_TO_HOST=…   AIJ_MOVE_TO_CODE=… \
//! AIJ_MOVE_STATE=<a folder for the run's notes and moves> \
//! [AIJ_MOVE_TEMPLATE=<a lab history to fork, for seed>] [AIJ_MOVE_PID=<pid, for link>] \
//! cargo test -p juggler-adapters --test move_live -- --ignored --nocapture
//! ```
//!
//! Every path must be one of the two sanctioned profiles' folders. Nothing
//! sends a model prompt; `link` only opens the conversation by its id.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    clippy::print_stdout,
    reason = "a live harness run by hand: it prints what it checked"
)]

use std::path::{Path, PathBuf};

use juggler_adapter::moving::{InPlaceMove, MoveRequest, MoveSide, NotesChoice};
use juggler_adapters::claude_move::ClaudeMove;
use juggler_handover::moving::{Stopped, Transaction};

const SANCTIONED: [&str; 2] = ["/aij-test", "/aij-move-scratch"];

fn var(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("set {name}")))
}

fn sanctioned(path: &Path) {
    let text = path.to_string_lossy();
    assert!(
        SANCTIONED.iter().any(|s| text.ends_with(s)),
        "the live move touches only the sanctioned test profiles, not {text}"
    );
}

/// Whether a Claude app process runs on `host` (its `--user-data-dir`).
fn running(host: &Path) -> bool {
    let out = std::process::Command::new("/bin/ps")
        .args(["-axo", "command"])
        .output()
        .unwrap();
    let needle = format!("--user-data-dir={}", host.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .any(|line| line.contains("Claude.app/Contents/MacOS/Claude ") && line.ends_with(&needle))
}

fn version() -> String {
    let plist = std::fs::read_to_string("/Applications/Claude.app/Contents/Info.plist").unwrap();
    let at = plist.find("<key>CFBundleShortVersionString</key>").unwrap();
    let rest = &plist[at..];
    let open = rest.find("<string>").unwrap() + 8;
    rest[open..open + rest[open..].find("</string>").unwrap()].to_owned()
}

/// A fresh id in a conversation id's shape.
fn new_id(seed: &str) -> String {
    let (hex, _) = juggler_adapter::handover::sha256_reader(
        format!("{seed}-{:?}", std::time::SystemTime::now()).as_bytes(),
        None,
    )
    .unwrap();
    format!(
        "{}-{}-4{}-a{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32]
    )
}

struct Sides {
    request: MoveRequest,
    state: PathBuf,
}

fn sides() -> Sides {
    let [fh, fc, th, tc] = [
        "AIJ_MOVE_FROM_HOST",
        "AIJ_MOVE_FROM_CODE",
        "AIJ_MOVE_TO_HOST",
        "AIJ_MOVE_TO_CODE",
    ]
    .map(var);
    for path in [&fh, &fc, &th, &tc] {
        sanctioned(path);
    }
    let state = var("AIJ_MOVE_STATE");
    std::fs::create_dir_all(&state).unwrap();
    let conversation = std::fs::read_to_string(state.join("family.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|v| v["parent"].as_str().map(str::to_owned))
        .unwrap_or_default();
    let side = |host: PathBuf, code: PathBuf| MoveSide {
        tool_dir: code,
        host_dir: Some(host),
        version: Some(version()),
    };
    Sides {
        request: MoveRequest {
            conversation,
            source: side(fh, fc),
            target: side(th, tc),
            notes: NotesChoice::KeepTarget,
        },
        state,
    }
}

fn quit_first(s: &Sides) {
    for host in [&s.request.source.host_dir, &s.request.target.host_dir]
        .into_iter()
        .flatten()
    {
        assert!(!running(host), "quit {} by its pid first", host.display());
    }
}

/// The account folder the profile's app lists: the one its Claude Code
/// folder is signed in to (two ids read, nothing else), else its only one.
fn records_folder(host: &Path, code: &Path) -> PathBuf {
    let root = host.join("claude-code-sessions");
    if let Some(sign_in) =
        juggler_adapters::claude_limits::profile_limits(code).and_then(|l| l.sign_in)
    {
        return root.join(sign_in.account).join(sign_in.org);
    }
    let mut found = Vec::new();
    for account in std::fs::read_dir(&root).unwrap().flatten() {
        for org in std::fs::read_dir(account.path()).unwrap().flatten() {
            found.push(org.path());
        }
    }
    assert_eq!(found.len(), 1, "one account folder in {}", root.display());
    found.pop().unwrap()
}

/// Seeds a small family in the source: a parent, two children (one
/// archived), a grandchild, each a fork of a lab history, with a nested
/// agent and rewind for the parent.
fn seed(s: &Sides) {
    quit_first(s);
    let template = var("AIJ_MOVE_TEMPLATE");
    sanctioned(template.ancestors().nth(3).unwrap());
    let bytes = std::fs::read(&template).unwrap();
    let project = template.parent().unwrap();
    let cwd = juggler_adapter::handover::embedded_cwd(&bytes).unwrap();
    let folder = records_folder(
        s.request.source.host_dir.as_ref().unwrap(),
        &s.request.source.tool_dir,
    );
    let ids: Vec<String> = ["parent", "child-a", "child-b", "grandchild"]
        .iter()
        .map(|n| new_id(n))
        .collect();
    let titles = [
        "Move live: parent",
        "Move live: child A",
        "Move live: child B (archived)",
        "Move live: grandchild",
    ];
    let parents = [None, Some(0), Some(0), Some(1)];
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    for (n, id) in ids.iter().enumerate() {
        std::fs::write(
            project.join(format!("{id}.jsonl")),
            juggler_adapter::handover::fork_session_id(&bytes, id),
        )
        .unwrap();
        let mut record = serde_json::json!({
            "sessionId": format!("local_{id}"),
            "cliSessionId": id,
            "cwd": cwd,
            "originCwd": cwd,
            "title": titles[n],
            "model": "claude-sonnet-5-5",
            "permissionMode": "default",
            "isArchived": n == 2,
            "createdAt": now - 60_000 + n as i64,
            "lastActivityAt": now - 30_000 + n as i64,
            "lastFocusedAt": now - 30_000 + n as i64,
            "fromCliTranscript": true,
        });
        if let Some(p) = parents[n] {
            record["spawnedFrom"] = serde_json::json!({"sessionId": format!("local_{}", ids[p]), "sideSession": true, "name": titles[n]});
        }
        std::fs::write(
            folder.join(format!("local_{id}.json")),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
    }
    let parent = &ids[0];
    std::fs::create_dir_all(project.join(parent).join("subagents")).unwrap();
    std::fs::write(
        project.join(parent).join("subagents/agent-live.jsonl"),
        b"{\"type\":\"user\",\"isSidechain\":true}\n",
    )
    .unwrap();
    std::fs::write(
        project.join(parent).join("subagents/agent-live.meta.json"),
        b"{\"agentType\":\"general-purpose\"}",
    )
    .unwrap();
    let rewind = s.request.source.tool_dir.join("file-history").join(parent);
    std::fs::create_dir_all(&rewind).unwrap();
    std::fs::write(rewind.join("v1"), b"rewind point").unwrap();
    std::fs::write(
        s.state.join("family.json"),
        serde_json::to_vec_pretty(&serde_json::json!({"parent": parent, "ids": ids, "folder": folder, "project": project})).unwrap(),
    )
    .unwrap();
    println!("seeded {ids:?}");
}

fn moves_dir(s: &Sides) -> PathBuf {
    s.state.join("moves")
}

fn do_move(s: &Sides) {
    quit_first(s);
    let plan = ClaudeMove.plan(&s.request).unwrap();
    println!(
        "plan: {} members, {} steps, notes {:?}, refusals {:?}",
        plan.members.len(),
        plan.steps.len(),
        plan.notes,
        plan.refusals
    );
    assert!(plan.can_move());
    let id = format!("live-{}", std::process::id());
    let mut txn = Transaction::begin(
        &moves_dir(s),
        &id,
        "claude-code",
        s.request.clone(),
        plan.clone(),
        None,
        0,
    )
    .unwrap();
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    let report = txn
        .verify_cold(ClaudeMove.verify(&s.request, &plan))
        .unwrap();
    println!(
        "cold checks: {} passed of {}",
        report.checks.iter().filter(|c| c.pass).count(),
        report.checks.len()
    );
    for failure in report.failures() {
        println!("FAILED: {failure}");
    }
    assert!(report.passed());
    std::fs::write(s.state.join("last-move"), &id).unwrap();
    println!("move {id} applied and checked; start both profiles, then phase link and after");
}

fn link(s: &Sides) {
    let pid: u32 = std::env::var("AIJ_MOVE_PID").unwrap().parse().unwrap();
    let url = ClaudeMove.resume_link(&s.request.conversation).unwrap();
    // The exact process, by a GURL event (the app's own link handling).
    juggler_platform::apps::send_link(pid, &url).unwrap();
    println!("sent {url} to pid {pid}");
}

fn after(s: &Sides) {
    let id = std::fs::read_to_string(s.state.join("last-move")).unwrap();
    let mut txn = Transaction::open(&moves_dir(s).join(id.trim())).unwrap();
    let plan = txn.manifest.plan.clone();
    let report = txn
        .verify_opened(
            &|line| ClaudeMove.appended(line),
            ClaudeMove.verify(&s.request, &plan),
        )
        .unwrap();
    println!(
        "after checks: {} passed of {}; added lines: {:?}; phase {:?}",
        report.checks.iter().filter(|c| c.pass).count(),
        report.checks.len(),
        report.added,
        txn.manifest.phase
    );
    for failure in report.failures() {
        println!("FAILED: {failure}");
    }
    assert!(report.passed());
    assert_eq!(report.added.turns, 0, "no turn written by the move");
}

fn undo(s: &Sides) {
    quit_first(s);
    let id = std::fs::read_to_string(s.state.join("last-move")).unwrap();
    let mut txn = Transaction::open(&moves_dir(s).join(id.trim())).unwrap();
    let kept = txn.undo(&Stopped::checked(), 1).unwrap();
    println!("undone; kept aside in {}", kept.display());
}

#[test]
#[ignore = "live: aij-test and a scratch profile, run by hand"]
fn move_live() {
    let s = sides();
    match std::env::var("AIJ_MOVE_PHASE").unwrap().as_str() {
        "seed" => seed(&s),
        "move" => do_move(&s),
        "link" => link(&s),
        "after" => after(&s),
        "undo" => undo(&s),
        other => panic!("unknown phase {other}"),
    }
}
