//! AJ-E2307 (D-28 as amended by D-40): AI Juggler never answers a tool's
//! prompt by itself. The source holds one path to an answer: the
//! `runtime_answer` command, run by the user's click, through `answer_impl`
//! to the driver. No timer, rule, agent or default reaches it: nothing
//! else builds an answer command or calls the driver's answer, and the
//! agent bridge and the pools never touch the runtime.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Each source file under `src/`, without its inline test module.
fn shipped() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &mut files);
    files
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).unwrap();
            let text = text
                .split("\n#[cfg(test)]\nmod tests")
                .next()
                .unwrap()
                .to_owned();
            let rel = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (rel, text)
        })
        .collect()
}

fn files_with(needle: &str) -> Vec<String> {
    shipped()
        .into_iter()
        .filter(|(_, text)| text.contains(needle))
        .map(|(rel, _)| rel)
        .collect()
}

#[test]
fn only_the_users_click_reaches_an_answer() {
    assert_eq!(
        files_with("Command::Answer("),
        ["features/runtime/driver.rs"],
        "only the driver builds an answer command (from Driver::answer)"
    );
    assert_eq!(
        files_with(".answer(AskId("),
        ["features/runtime/commands.rs"],
        "only answer_impl calls the driver's answer"
    );
    assert!(
        files_with("runtime::commands::answer_impl").is_empty()
            && files_with("runtime::commands::*").is_empty(),
        "the runtime's answer_impl is reached only from its own IPC command"
    );
    let commands = shipped()
        .into_iter()
        .find(|(rel, _)| rel == "features/runtime/commands.rs")
        .unwrap()
        .1;
    let callers = commands.matches("answer_impl(").count();
    assert_eq!(callers, 2, "its definition and runtime_answer's one call");
    assert!(commands.contains("pub async fn runtime_answer("));
}

#[test]
fn agents_and_pools_never_reach_the_runtime() {
    for (rel, text) in shipped() {
        if rel.starts_with("features/agent_bridge/") || rel.starts_with("features/pools/") {
            // A tool hook's report may set its own session's state
            // (`runtime::reports::handle`); nothing else of the runtime.
            let other = text
                .replace("crate::features::runtime::reports::handle(", "")
                .contains("features::runtime");
            assert!(
                !other && !text.contains("state.runtime"),
                "{rel} reaches the runtime"
            );
        }
    }
    // The bridge program itself has no word for an answer.
    let bridge = Path::new(env!("CARGO_MANIFEST_DIR")).join("../juggler-agentbridge/src");
    let mut files = Vec::new();
    sources(&bridge, &mut files);
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            !text.contains("runtime_answer") && !text.contains("AskAnswer"),
            "{} can answer a tool's prompt",
            path.display()
        );
    }
}

#[test]
fn no_launch_bypasses_a_tools_prompts() {
    // A driven or terminal launch never passes a bypass flag: the
    // adapters' specs are checked by the conformance kit; here, nothing in
    // the app adds one.
    for (rel, text) in shipped() {
        for bad in juggler_adapter::runtime::FORBIDDEN_ARGS {
            assert!(
                !text.contains(&format!("\"{bad}\"")),
                "{rel} names the bypass {bad}"
            );
        }
    }
}
