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
mod grok_fixture;
mod kimi_fixture;
mod opencode_fixture;

use std::path::{Path, PathBuf};

use conformance::{Sandbox, Subject, UsageSubject};
use juggler_adapter::HarnessAdapter;
use juggler_adapters::{
    ClaudeCodeAdapter, CodexAdapter, FakeHarnessAdapter, GrokBuildAdapter, KimiCodeAdapter,
    OpenCodeAdapter,
};
use juggler_core::quota::{UsageEvent, UsageSource};
use opencode_fixture::Line;

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

    /// Claude Code 2.1.302: "When using --print, --output-format=stream-json
    /// requires --verbose" (the lab of 2026-10-25).
    fn flag_rules(&self) -> &'static [(&'static str, &'static str)] {
        &[("stream-json", "--verbose")]
    }

    /// Claude Code keeps the trust answer in `.claude.json`.
    fn trust(&self, state_dir: &Path, cwd: &Path) {
        let projects =
            serde_json::json!({ cwd.to_string_lossy(): {"hasTrustDialogAccepted": true} });
        write(
            &state_dir.join(".claude.json"),
            &serde_json::json!({ "projects": projects }).to_string(),
        );
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

// ---------------------------------------------------------------------------
// OpenCode (AJ-E1602), once per line: the database differs.

struct OpenCode(Line);

fn opencode_db(state_dir: &Path) -> PathBuf {
    // A managed account keeps it under data/; the default folder is the
    // data folder itself.
    let managed = state_dir.join("data/opencode");
    if managed.is_dir() {
        managed.join("opencode.db")
    } else {
        state_dir.join("opencode.db")
    }
}

impl Subject for OpenCode {
    fn adapter(&self, sandbox: &Sandbox) -> Box<dyn HarnessAdapter> {
        let version = match self.0 {
            Line::V1 => "1.19.3",
            Line::V2 => "2.1.2",
        };
        Box::new(
            OpenCodeAdapter::with_bin(sandbox.stub("opencode", version)).with_home(&sandbox.home),
        )
    }

    fn write_session(&self, state_dir: &Path, cwd: &Path, n: u32) -> String {
        let id = format!("ses_kit{n:04}");
        let conn = opencode_fixture::create(&opencode_db(state_dir), self.0);
        opencode_fixture::add_session(&conn, self.0, &id, cwd, None, opencode_fixture::now_ms());
        // A sub-agent's session in the same folder is never the one.
        opencode_fixture::add_session(
            &conn,
            self.0,
            &format!("ses_kit{n:04}_child"),
            cwd,
            Some(&id),
            opencode_fixture::now_ms(),
        );
        id
    }

    fn seed_source(&self, sandbox: &Sandbox, canary: &str) -> Option<PathBuf> {
        let source = sandbox.home.join(".local/share/opencode");
        write(
            &source.join("auth.json"),
            &opencode_fixture::auth_json(canary),
        );
        write(
            &source.join("mcp-auth.json"),
            &format!(r#"{{"github":{{"tokens":{{"accessToken":"{canary}"}}}}}}"#),
        );
        write(&source.join("log/2026-10-21.log"), "started\n");
        let conn = opencode_fixture::create(&source.join("opencode.db"), self.0);
        opencode_fixture::add_session(&conn, self.0, "ses_old", &sandbox.project, None, 1);
        opencode_fixture::add_sign_ins(&conn, canary, true);
        drop(conn);
        write(
            &sandbox.home.join(".config/opencode/opencode.json"),
            r#"{"$schema":"https://opencode.ai/config.json","theme":"system"}"#,
        );
        // A key written into a settings file is a sign-in too.
        write(
            &sandbox.home.join(".config/opencode/opencode.jsonc"),
            &format!(
                "{{\n  // mine\n  \"provider\": {{\"x\": {{\"options\": {{\"apiKey\": \"{canary}\"}}}}}}\n}}\n"
            ),
        );
        Some(source)
    }

    fn credential_files(&self) -> &'static [&'static str] {
        &["auth.json", "mcp-auth.json"]
    }

    fn usage(&self) -> Option<&dyn UsageSubject> {
        match self.0 {
            Line::V1 => Some(&OpenCodeUsage(Line::V1)),
            Line::V2 => Some(&OpenCodeUsage(Line::V2)),
        }
    }

    fn sandboxed_default(&self) -> bool {
        true
    }
}

struct OpenCodeUsage(Line);

impl UsageSubject for OpenCodeUsage {
    fn seed(&self, state_dir: &Path) {
        let conn = opencode_fixture::create(&opencode_db(state_dir), self.0);
        let at = 1_792_580_400_000; // 2026-10-21
        opencode_fixture::add_session(&conn, self.0, "ses_use", Path::new("/w"), None, at);
        opencode_fixture::add_user(&conn, self.0, "msg_u1", "ses_use", at);
        for (id, model, input, completed) in [
            ("msg_a1", "claude-sonnet-5-5", 1000, true),
            ("msg_a2", "kimi-k3", 400, true),
            // Still running: its counts may grow, so it isn't read yet.
            ("msg_a3", "kimi-k3", 50, false),
        ] {
            opencode_fixture::add_assistant(
                &conn,
                self.0,
                &opencode_fixture::Assistant {
                    id,
                    session: "ses_use",
                    created_ms: at + 10,
                    provider: "opencode-go",
                    model,
                    input,
                    output: 120,
                    cache_read: 30,
                    cost: 0.012,
                    completed,
                },
            );
        }
    }

    /// What the usage pipeline runs: the adapter's store read.
    fn read(&self, account: &str, state_dir: &Path) -> Vec<UsageEvent> {
        let adapter = OpenCodeAdapter::new();
        let mut record = conformance::account(&adapter, state_dir);
        record.id = juggler_core::model::AccountId::new(account);
        adapter
            .usage_since(&record, 0)
            .expect("OpenCode reads its usage from its store")
            .unwrap()
    }

    fn source(&self) -> UsageSource {
        UsageSource::Sqlite
    }

    fn distinct_messages(&self) -> usize {
        2
    }
}

// Kimi Code (AJ-E1603): exact tokens from each session's event log
// (`wire.jsonl`); its plan's quota stays unread (the maintainer's ruling).

struct Kimi;

impl Subject for Kimi {
    fn adapter(&self, sandbox: &Sandbox) -> Box<dyn HarnessAdapter> {
        Box::new(KimiCodeAdapter::with_bin(sandbox.stub("kimi", "2.3.0")).with_home(&sandbox.home))
    }

    fn write_session(&self, state_dir: &Path, cwd: &Path, n: u32) -> String {
        let id = format!("kit-kimi-session-{n}");
        kimi_fixture::add_session(state_dir, &id, cwd, "kit session");
        id
    }

    fn seed_source(&self, sandbox: &Sandbox, canary: &str) -> Option<PathBuf> {
        let source = sandbox.home.join(".kimi-code");
        kimi_fixture::settings(&source, canary);
        kimi_fixture::sign_in_membership(&source, canary);
        write(
            &source.join("credentials/mcp/files.json"),
            &format!(r#"{{"token":"{canary}"}}"#),
        );
        kimi_fixture::add_session(&source, "ses-old", &sandbox.project, "old");
        Some(source)
    }

    fn credential_files(&self) -> &'static [&'static str] {
        &["credentials/kimi-code.json", "server.token"]
    }

    fn usage(&self) -> Option<&dyn UsageSubject> {
        Some(&KimiUsage)
    }

    fn sandboxed_default(&self) -> bool {
        true
    }
}

struct KimiUsage;

impl UsageSubject for KimiUsage {
    fn seed(&self, state_dir: &Path) {
        let at = 1_792_580_400_000; // 2026-10-21
        let first = kimi_fixture::add_session(state_dir, "ses-use", Path::new("/w"), "use");
        kimi_fixture::append_wire(
            &first,
            "main",
            &[
                kimi_fixture::WIRE_META.to_owned(),
                kimi_fixture::usage_line("kimi-k3", at + 10, 1000, 120, 30),
                kimi_fixture::usage_line("kimi-k3", at + 20, 400, 80, 0),
            ],
        );
        // A fork's copy carries the first call again: one message, twice.
        let fork = kimi_fixture::add_session(state_dir, "ses-fork", Path::new("/w"), "fork");
        kimi_fixture::append_wire(
            &fork,
            "main",
            &[
                kimi_fixture::WIRE_META.to_owned(),
                kimi_fixture::usage_line("kimi-k3", at + 10, 1000, 120, 30),
            ],
        );
    }

    /// What the usage pipeline runs: the adapter's store read.
    fn read(&self, account: &str, state_dir: &Path) -> Vec<UsageEvent> {
        let adapter = KimiCodeAdapter::new();
        let mut record = conformance::account(&adapter, state_dir);
        record.id = juggler_core::model::AccountId::new(account);
        adapter
            .usage_since(&record, 0)
            .expect("Kimi Code reads its usage from its event logs")
            .unwrap()
    }

    fn source(&self) -> UsageSource {
        UsageSource::Transcript
    }

    fn distinct_messages(&self) -> usize {
        2
    }
}

// Grok Build (AJ-E1604): no usage reader yet (`signals.json`'s fields
// aren't documented), so it must claim none.

struct Grok;

impl Subject for Grok {
    fn adapter(&self, sandbox: &Sandbox) -> Box<dyn HarnessAdapter> {
        Box::new(
            GrokBuildAdapter::with_bin(sandbox.stub("grok", "grok 1.0.45"))
                .with_home(&sandbox.home),
        )
    }

    fn write_session(&self, state_dir: &Path, cwd: &Path, n: u32) -> String {
        let id = format!("0199aa00-0000-7000-8000-00000000000{n}");
        grok_fixture::add_session(state_dir, &id, cwd, "kit session");
        id
    }

    fn seed_source(&self, sandbox: &Sandbox, canary: &str) -> Option<PathBuf> {
        let source = sandbox.home.join(".grok");
        grok_fixture::settings(&source, canary);
        grok_fixture::sign_in(&source, canary);
        grok_fixture::add_session(&source, "ses-old", &sandbox.project, "old");
        Some(source)
    }

    fn credential_files(&self) -> &'static [&'static str] {
        &["auth.json", "mcp_credentials.json"]
    }

    fn usage(&self) -> Option<&dyn UsageSubject> {
        None
    }

    fn sandboxed_default(&self) -> bool {
        true
    }
}

conformance!(claude_code, Claude);
conformance!(grok_build, Grok);
conformance!(kimi_code, Kimi);
conformance!(opencode_v1, OpenCode(Line::V1));
conformance!(opencode_v2, OpenCode(Line::V2));
conformance!(codex, Codex);
conformance!(fake, Fake);
