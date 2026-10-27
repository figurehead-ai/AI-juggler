//! What a conversation used, read from synthetic transcripts in the shapes
//! switch-setup-items.md §3 and §4 record: each tool's reader returns
//! exactly the skills, commands, MCP servers, subagents and plugins the
//! fixture used, and nothing it only mentions. Tools whose transcripts
//! AI Juggler doesn't read for this say nothing (`None`), never "no".

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};

use juggler_adapter::HarnessAdapter;
use juggler_adapter::inventory::ItemKind;
use juggler_adapters::{
    ClaudeCodeAdapter, CodexAdapter, GrokBuildAdapter, KimiCodeAdapter, OpenCodeAdapter,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn pairs(list: &[(ItemKind, &str)]) -> Vec<(ItemKind, String)> {
    let mut out: Vec<(ItemKind, String)> = list
        .iter()
        .map(|(kind, name)| (*kind, (*name).to_owned()))
        .collect();
    out.sort();
    out
}

#[test]
fn a_claude_transcript_says_what_it_used() {
    let adapter = ClaudeCodeAdapter::with_bin("/usr/bin/true");
    let mut used = adapter
        .used_in(&fixture("claude-code/session-used.jsonl"))
        .expect("Claude Code's transcripts say what they used");
    used.sort();
    assert_eq!(
        used,
        pairs(&[
            // The Skill call and its attachment.
            (ItemKind::Skill, "pdf"),
            // Typed: a synced skill and a command, either kind.
            (ItemKind::Skill, "docs"),
            (ItemKind::Command, "docs"),
            (ItemKind::Skill, "review"),
            (ItemKind::Command, "review"),
            // MCP: a server, and a plugin's server as its plugin.
            (ItemKind::McpServer, "github"),
            (ItemKind::Plugin, "notes"),
            // A subagent.
            (ItemKind::Subagent, "tester"),
        ])
    );
}

#[test]
fn a_codex_rollout_says_what_it_used() {
    let adapter = CodexAdapter::with_bin("/usr/bin/true");
    let mut used = adapter
        .used_in(&fixture("codex/rollout-used.jsonl"))
        .expect("Codex's rollouts say what they used");
    used.sort();
    assert_eq!(
        used,
        pairs(&[
            // An explicit pick, and a SKILL.md read; never the catalog.
            (ItemKind::Skill, "ship-it"),
            (ItemKind::Skill, "release-notes"),
            // MCP: a function call, and a legacy end event.
            (ItemKind::McpServer, "github"),
            (ItemKind::McpServer, "linear"),
            (ItemKind::Subagent, "reviewer"),
            (ItemKind::Plugin, "notes"),
        ])
    );
}

#[test]
fn tools_whose_transcripts_arent_read_say_nothing() {
    let any = fixture("claude-code/session-used.jsonl");
    let adapters: [Box<dyn HarnessAdapter>; 3] = [
        Box::new(OpenCodeAdapter::with_bin("/usr/bin/true")),
        Box::new(KimiCodeAdapter::with_bin("/usr/bin/true")),
        Box::new(GrokBuildAdapter::with_bin("/usr/bin/true")),
    ];
    for adapter in adapters {
        assert!(adapter.used_in(&any).is_none(), "unknown, never \"no\"");
    }
    let missing = Path::new("/nonexistent/none.jsonl");
    assert!(
        ClaudeCodeAdapter::with_bin("/usr/bin/true")
            .used_in(missing)
            .is_none()
    );
}
