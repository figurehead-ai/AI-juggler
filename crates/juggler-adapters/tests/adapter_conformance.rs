//! AJ-E1521: the conformance kit (`conformance/mod.rs`) run for every
//! adapter. A new adapter adds a [`Subject`] here and one
//! `conformance!` line; it may not merge until its module passes.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

mod conformance;

use std::path::{Path, PathBuf};

use conformance::{Sandbox, Subject, UsageSubject};
use juggler_adapter::HarnessAdapter;
use juggler_adapters::{ClaudeCodeAdapter, CodexAdapter, FakeHarnessAdapter};
use juggler_core::quota::{UsageEvent, UsageSource};

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn jsonl_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(jsonl_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "jsonl") {
            out.push(path);
        }
    }
    out.sort();
    out
}

// ---------------------------------------------------------------------------
// Claude Code

struct Claude;

impl Subject for Claude {
    fn adapter(&self, sandbox: &Sandbox) -> Box<dyn HarnessAdapter> {
        Box::new(ClaudeCodeAdapter::with_bin(
            sandbox.stub("claude", "2.1.80 (Claude Code)"),
        ))
    }

    fn write_session(&self, state_dir: &Path, cwd: &Path, n: u32) -> String {
        let id = format!("0b1c2d3e-0000-4000-8000-00000000000{n}");
        let line = serde_json::json!({
            "type": "user",
            "sessionId": id,
            "cwd": cwd,
            "uuid": format!("u-{n}"),
            "timestamp": "2026-10-21T10:00:00.000Z",
            "message": {"role": "user", "content": "hello"},
        });
        write(
            &state_dir.join(format!("projects/-kit-project/{id}.jsonl")),
            &format!("{line}\n"),
        );
        id
    }

    fn seed_source(&self, sandbox: &Sandbox, canary: &str) -> Option<PathBuf> {
        let source = sandbox.home.join(".claude-work");
        write(&source.join("settings.json"), r#"{"model":"opus"}"#);
        write(
            &source.join(".credentials.json"),
            &format!(
                r#"{{"claudeAiOauth":{{"accessToken":"{canary}","refreshToken":"{canary}","expiresAt":4102444800000}}}}"#
            ),
        );
        self.write_session(&source, &sandbox.project, 1);
        Some(source)
    }

    fn credential_files(&self) -> &'static [&'static str] {
        &[".credentials.json"]
    }

    fn usage(&self) -> Option<&dyn UsageSubject> {
        Some(&ClaudeUsage)
    }

    fn sandboxed_default(&self) -> bool {
        false // its default folder is the real ~/.claude; never touched here
    }
}

struct ClaudeUsage;

impl UsageSubject for ClaudeUsage {
    fn seed(&self, state_dir: &Path) {
        let assistant = |uuid: &str, model: &str, input: u64| {
            serde_json::json!({
                "type": "assistant",
                "sessionId": "0b1c2d3e-0000-4000-8000-000000000001",
                "uuid": uuid,
                "timestamp": "2026-10-21T10:00:05.000Z",
                "message": {"model": model, "usage": {
                    "input_tokens": input, "output_tokens": 20,
                    "cache_read_input_tokens": 5, "cache_creation_input_tokens": 1}},
            })
            .to_string()
        };
        // A resumed file repeats history: one message appears twice.
        let lines = [
            assistant("m-1", "claude-opus-5-5", 100),
            assistant("m-2", "claude-sonnet-5-5", 200),
            assistant("m-1", "claude-opus-5-5", 100),
        ];
        write(
            &state_dir.join("projects/-kit-project/usage.jsonl"),
            &(lines.join("\n") + "\n"),
        );
    }

    fn read(&self, account: &str, state_dir: &Path) -> Vec<UsageEvent> {
        jsonl_files(&state_dir.join("projects"))
            .iter()
            .flat_map(|path| juggler_adapters::tail_transcript(path, account, 0).events)
            .collect()
    }

    fn source(&self) -> UsageSource {
        UsageSource::Transcript
    }

    fn distinct_messages(&self) -> usize {
        2
    }
}

// ---------------------------------------------------------------------------
// Codex

struct Codex;

fn codex_rollout(state_dir: &Path, n: u32, lines: &[serde_json::Value]) -> String {
    let id = format!("0199a2b3-0000-7000-8000-00000000000{n}");
    let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
    write(
        &state_dir.join(format!(
            "sessions/2026/10/21/rollout-2026-10-21T10-00-0{n}-{id}.jsonl"
        )),
        &text,
    );
    id
}

impl Subject for Codex {
    fn adapter(&self, sandbox: &Sandbox) -> Box<dyn HarnessAdapter> {
        Box::new(
            CodexAdapter::with_bin(sandbox.stub("codex", "codex-cli 0.163.0"))
                .with_home(&sandbox.home),
        )
    }

    fn write_session(&self, state_dir: &Path, cwd: &Path, n: u32) -> String {
        let id = format!("0199a2b3-0000-7000-8000-00000000000{n}");
        codex_rollout(
            state_dir,
            n,
            &[serde_json::json!({
                "timestamp": "2026-10-21T10:00:00.000Z",
                "type": "session_meta",
                "payload": {"id": id, "cwd": cwd, "originator": "codex_cli_rs"},
            })],
        )
    }

    fn seed_source(&self, sandbox: &Sandbox, canary: &str) -> Option<PathBuf> {
        let source = sandbox.home.join(".codex-work");
        write(&source.join("config.toml"), "model = \"gpt-6\"\n");
        write(
            &source.join("auth.json"),
            &format!(
                r#"{{"auth_mode":"chatgpt","tokens":{{"access_token":"{canary}","refresh_token":"{canary}","account_id":"acct-1"}}}}"#
            ),
        );
        write(&source.join("history.jsonl"), "{\"text\":\"hi\"}\n");
        self.write_session(&source, &sandbox.project, 1);
        Some(source)
    }

    fn credential_files(&self) -> &'static [&'static str] {
        &["auth.json"]
    }

    fn usage(&self) -> Option<&dyn UsageSubject> {
        Some(&CodexUsage)
    }

    fn sandboxed_default(&self) -> bool {
        true
    }
}

struct CodexUsage;

impl UsageSubject for CodexUsage {
    fn seed(&self, state_dir: &Path) {
        let record = |turn: &str, response: &str, input: u64| {
            serde_json::json!({
                "timestamp": "2026-10-21T10:00:05.000Z",
                "type": "token_usage_record",
                "payload": {
                    "thread_id": "0199a2b3-0000-7000-8000-000000000009",
                    "turn_id": turn, "response_id": response,
                    "usage": {"input_tokens": input, "output_tokens": 30,
                              "cached_input_tokens": 10, "reasoning_output_tokens": 4},
                },
            })
        };
        codex_rollout(
            state_dir,
            9,
            &[
                record("t1", "r1", 100),
                record("t2", "r2", 200),
                record("t1", "r1", 100),
            ],
        );
    }

    fn read(&self, account: &str, state_dir: &Path) -> Vec<UsageEvent> {
        jsonl_files(&state_dir.join("sessions"))
            .iter()
            .flat_map(|path| {
                juggler_adapters::tail_rollout(path, account, 0, &Default::default()).events
            })
            .collect()
    }

    fn source(&self) -> UsageSource {
        UsageSource::Rollout
    }

    fn distinct_messages(&self) -> usize {
        2
    }
}

// ---------------------------------------------------------------------------
// The fake harness

struct Fake;

impl Subject for Fake {
    fn adapter(&self, sandbox: &Sandbox) -> Box<dyn HarnessAdapter> {
        Box::new(FakeHarnessAdapter::new(
            sandbox.stub("fake-harness", "fake-harness 0.1.0"),
        ))
    }

    fn write_session(&self, state_dir: &Path, cwd: &Path, n: u32) -> String {
        // What `fake-harness run` writes (src/bin/fake_harness.rs).
        let id = format!("sess-kit-{n}");
        let meta = serde_json::json!({
            "type": "session_meta", "id": id, "cwd": cwd, "prompt": "",
        });
        write(
            &state_dir.join(format!("transcripts/{id}.jsonl")),
            &format!("{meta}\n"),
        );
        id
    }

    fn seed_source(&self, _sandbox: &Sandbox, _canary: &str) -> Option<PathBuf> {
        None
    }

    fn credential_files(&self) -> &'static [&'static str] {
        &[]
    }

    fn usage(&self) -> Option<&dyn UsageSubject> {
        None
    }

    fn sandboxed_default(&self) -> bool {
        false
    }
}

conformance!(claude_code, Claude);
conformance!(codex, Codex);
conformance!(fake, Fake);
