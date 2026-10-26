//! AJ-E1905: the Claude Code plugin, installed into a fixture home.
//!
//! - Its manifests are what Claude Code reads: the plugin's name and
//!   version, one MCP server run from the plugin's own folder, a skill
//!   with its name and description, and a marketplace that lists it. No
//!   key, socket or account appears anywhere in it.
//! - Copied into a fixture home as an install puts it, its server runs
//!   the real `juggler-agent`: in a session AI Juggler started (a key in
//!   the environment) it lists nothing, so the launch's own tools aren't
//!   listed twice; anywhere else it lists the tools and each call says it
//!   needs a session AI Juggler started; with no AI Juggler on the Mac it
//!   says so and exits.
//!
//! Nothing real is launched: the home and the apps folder are temporary.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    missing_docs
)]
#![cfg(unix)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

const BRIDGE: &str = env!("CARGO_BIN_EXE_juggler-agent");

fn plugin_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/claude-code")
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Installs the plugin into a fixture home, as Claude Code's plugin
/// cache keeps it; its folder there.
fn install(home: &Path) -> PathBuf {
    let root = home.join(".claude/plugins/cache/ai-juggler/ai-juggler/0.2.0");
    copy_dir(&plugin_dir().join("ai-juggler"), &root);
    root
}

/// Runs the plugin's server as Claude Code would, with `lines` on its
/// stdin; its replies and exit code.
fn run(
    root: &Path,
    home: &Path,
    env: &[(&str, &str)],
    lines: &[&str],
) -> (Vec<Value>, Option<i32>, String) {
    let mcp = json(&root.join(".mcp.json"));
    let command = mcp["mcpServers"]["ai-juggler"]["command"]
        .as_str()
        .unwrap()
        .replace("${CLAUDE_PLUGIN_ROOT}", &root.to_string_lossy());
    let apps = home.join("Applications-none");
    std::fs::create_dir_all(&apps).unwrap();
    let mut child = Command::new(command)
        .env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .env("JUGGLER_APPS", &apps)
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for line in lines {
            writeln!(stdin, "{line}").unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    let replies = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    (
        replies,
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const HELLO: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"claude-code","version":"0"}}}"#;
const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
const CALL: &str = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"tasks_list","arguments":{"ready":true}}}"#;

#[test]
fn the_plugins_manifests_are_what_claude_code_reads_and_hold_no_secret() {
    let root = plugin_dir();
    let plugin = json(&root.join("ai-juggler/.claude-plugin/plugin.json"));
    assert_eq!(plugin["name"], "ai-juggler");
    assert_eq!(
        plugin["version"],
        env!("CARGO_PKG_VERSION"),
        "released with the app"
    );
    let market = json(&root.join(".claude-plugin/marketplace.json"));
    assert_eq!(market["plugins"][0]["source"], "./ai-juggler");
    let mcp = json(&root.join("ai-juggler/.mcp.json"));
    let servers = mcp["mcpServers"].as_object().unwrap();
    assert_eq!(servers.len(), 1);
    assert!(
        servers["ai-juggler"]["command"]
            .as_str()
            .unwrap()
            .starts_with("${CLAUDE_PLUGIN_ROOT}/"),
        "run from the plugin's own folder"
    );
    let skill =
        std::fs::read_to_string(root.join("ai-juggler/skills/pool-tasks/SKILL.md")).unwrap();
    assert!(skill.starts_with("---\nname: pool-tasks\ndescription: "));
    assert!(skill.contains("task_claim") && skill.contains("release: true"));
    // No key, socket, account or folder of the user's anywhere in it.
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            if entry.file_type().unwrap().is_dir() {
                stack.push(entry.path());
                continue;
            }
            let text = std::fs::read_to_string(entry.path()).unwrap();
            for word in [
                "JUGGLER_AGENT_TOKEN=",
                "bridge.sock",
                "@",
                "/Users/",
                "CLAUDE_CONFIG_DIR",
            ] {
                assert!(
                    !text.contains(word),
                    "{} holds {word}",
                    entry.path().display()
                );
            }
        }
    }
}

#[test]
fn installed_in_a_fixture_home_it_runs_the_bridge_and_says_why_not() {
    let home = tempfile::tempdir().unwrap();
    let root = install(home.path());
    // Outside AI Juggler's launches: the tools, and each says why not.
    let (replies, code, _) = run(
        &root,
        home.path(),
        &[("JUGGLER_AGENT", BRIDGE)],
        &[HELLO, LIST, CALL],
    );
    assert_eq!(code, Some(0));
    assert_eq!(replies[0]["result"]["serverInfo"]["name"], "juggler");
    assert_eq!(
        replies[1]["result"]["tools"].as_array().unwrap().len(),
        juggler_agentbridge::wire::TOOLS.len()
    );
    assert_eq!(replies[2]["result"]["isError"], true);
    let why = replies[2]["result"]["content"][0]["text"].as_str().unwrap();
    assert!(why.contains("wasn't started by AI Juggler"), "{why}");
    // In a session AI Juggler started: its launch gave it the tools.
    let (replies, _, _) = run(
        &root,
        home.path(),
        &[
            ("JUGGLER_AGENT", BRIDGE),
            ("JUGGLER_AGENT_TOKEN", "a-launch-key"),
        ],
        &[HELLO, LIST],
    );
    assert_eq!(replies[1]["result"]["tools"], serde_json::json!([]));
    assert!(
        replies[0]["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("already gave this session its pool tools")
    );
    // No AI Juggler on this Mac: it says so, and stops.
    let (replies, code, stderr) = run(&root, home.path(), &[], &[HELLO]);
    assert!(replies.is_empty());
    assert_eq!(code, Some(1));
    assert!(stderr.contains("AI Juggler isn't installed"), "{stderr}");
}
