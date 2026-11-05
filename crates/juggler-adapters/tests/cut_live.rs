//! D-76 (AJ-E211 req. 5): a cut conversation resumes in the real tools.
//!
//! Ignored, and a no-op unless `AIJ_LAB_PROJECT` (a new, empty folder) and
//! `AIJ_LAB_CLAUDE` (a lab Claude Code config folder) and/or
//! `AIJ_LAB_CODEX` (the Codex home) are set. It spends a little usage on
//! accounts whose owner agreed, and never signs in or reads a credential.
//!
//! Run it through `scripts/qa/lab_env.sh`, never straight from a shell:
//! started inside a Claude Code session, the tools inherit that session's
//! markers and routing and answer for it, not for the lab account. The
//! Claude half needs a terminal sign-in in that config folder (the Claude
//! app's own sign-in is separate; "OAuth session expired" means the folder
//! has none that works — its owner runs `/login` there once).
//!
//! ```sh
//! AIJ_LAB_PROJECT=<new folder> AIJ_LAB_CLAUDE=<lab config folder> \
//!   AIJ_LAB_CODEX=<codex home> scripts/qa/lab_env.sh \
//!   cargo test -p juggler-adapters --test cut_live -- --ignored --nocapture
//! ```
//!
//! For each tool: a four-turn conversation (a codename in the first turn,
//! another in the third), cut to its last two turns, installed beside the
//! original as a fork (the original keeps everything), then resumed and
//! asked which codenames it was told. It must know the late one and not
//! the early one. It prints what it ran and what each answered.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

use juggler_adapter::handover::{Cut, HandoverSupport};
use juggler_adapter::model::{Account, HarnessId};
use juggler_adapters::{ClaudeCodeAdapter, CodexAdapter};
use juggler_core::model::{AccountId, TranscriptRef};
use serde_json::Value;

const EARLY: &str = "EARLY-OWL";
const LATE: &str = "LATE-FINCH";
const ASK: &str = "Which codenames have I told you in this conversation? Answer with each codename on its own line and nothing else, or NONE.";

fn env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

/// A clean environment: the path and home, and only what's given.
fn tool(program: &str, project: &Path, extra: &[(&str, &Path)]) -> Command {
    let mut command = Command::new(program);
    command.env_clear().current_dir(project);
    for key in ["PATH", "HOME", "USER", "LANG", "TMPDIR"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    for (key, value) in extra {
        command.env(key, value);
    }
    command
}

fn account(harness: &str, state_dir: &Path) -> Account {
    Account {
        id: AccountId::new("lab"),
        harness: HarnessId::new(harness),
        label: "Lab".into(),
        plan: None,
        state_dir: state_dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: true,
    }
}

fn find(root: &Path, name_part: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find(&path, name_part) {
                return Some(found);
            }
        } else if path.file_name().is_some_and(|n| {
            n.to_string_lossy().contains(name_part) && n.to_string_lossy().ends_with(".jsonl")
        }) {
            return Some(path);
        }
    }
    None
}

fn judge(tool: &str, answer: &str) {
    println!("{tool} answered after the cut: {answer:?}");
    assert!(
        answer.contains(LATE),
        "{tool}: the kept turn's codename is known"
    );
    assert!(
        !answer.contains(EARLY),
        "{tool}: the left-out turn's codename is not"
    );
}

fn claude_turn(
    config: &Path,
    project: &Path,
    resume: Option<&str>,
    prompt: &str,
) -> (String, String) {
    let mut command = tool("claude", project, &[("CLAUDE_CONFIG_DIR", config)]);
    command.args(["-p", "--output-format", "json"]);
    if let Some(id) = resume {
        command.args(["--resume", id]);
    }
    let out = command.arg(prompt).output().unwrap();
    let value: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
    assert!(
        out.status.success() && value["is_error"] != true,
        "claude: {} {}",
        value["result"],
        String::from_utf8_lossy(&out.stderr)
    );
    (
        value["session_id"].as_str().unwrap().to_owned(),
        value["result"].as_str().unwrap_or_default().to_owned(),
    )
}

#[test]
#[ignore = "live: real Claude Code on a lab account"]
fn a_cut_claude_conversation_resumes_knowing_only_what_was_kept() {
    let (Some(project), Some(config)) = (env("AIJ_LAB_PROJECT"), env("AIJ_LAB_CLAUDE")) else {
        println!("AIJ_LAB_PROJECT and AIJ_LAB_CLAUDE aren't set: skipped");
        return;
    };
    std::fs::create_dir_all(&project).unwrap();
    let (mut id, _) = claude_turn(
        &config,
        &project,
        None,
        &format!("Remember the codename {EARLY} for later. Reply only OK."),
    );
    for prompt in [
        "Reply only OK.".to_owned(),
        format!("Remember the codename {LATE} for later. Reply only OK."),
        "Reply only OK.".to_owned(),
    ] {
        id = claude_turn(&config, &project, Some(&id), &prompt).0;
    }
    let transcript = find(&config.join("projects"), &id).expect("the conversation's transcript");
    println!(
        "claude: {} turns in {}",
        4,
        transcript.file_name().unwrap().to_string_lossy()
    );
    let adapter = ClaudeCodeAdapter::new();
    let reference = TranscriptRef {
        path: transcript.clone(),
        native_id: Some(id.clone()),
    };
    let preview = adapter.cut_preview(&reference, Cut::KeepLast(2)).unwrap();
    println!("claude: preview {preview:?}");
    assert_eq!(preview.turns_left_out, 2);
    let bundle = adapter
        .bundle_session_cut(&reference, Cut::KeepLast(2))
        .unwrap();
    assert_eq!(bundle.manifest.fidelity.level, "F2");
    let installed = adapter
        .install_bundle(&bundle, &account("claude-code", &config))
        .unwrap();
    assert!(
        installed.forked,
        "beside the original, which keeps everything"
    );
    let forked = installed.reference.native_id.clone().unwrap();
    let (_, answer) = claude_turn(&config, &project, Some(&forked), ASK);
    judge("Claude Code", &answer);
    let (_, whole) = claude_turn(&config, &project, Some(&id), ASK);
    println!("Claude Code's original answered: {whole:?}");
    assert!(whole.contains(EARLY), "the original still knows everything");
}

fn codex_turn(
    home: &Path,
    project: &Path,
    resume: Option<&str>,
    prompt: &str,
) -> (Option<String>, String) {
    let mut command = tool("codex", project, &[("CODEX_HOME", home)]);
    command.args(["exec", "--json", "--skip-git-repo-check"]);
    if let Some(id) = resume {
        command.args(["resume", id]);
    }
    let out = command.arg(prompt).output().unwrap();
    assert!(
        out.status.success(),
        "codex: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let (mut thread, mut text) = (None, String::new());
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(t) = event["thread_id"].as_str() {
            thread = Some(t.to_owned());
        }
        if event["item"]["type"] == "agent_message" {
            text = event["item"]["text"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
        }
    }
    (thread, text)
}

#[test]
#[ignore = "live: real Codex on the lab's Codex home"]
fn a_cut_codex_rollout_resumes_knowing_only_what_was_kept() {
    let (Some(project), Some(home)) = (env("AIJ_LAB_PROJECT"), env("AIJ_LAB_CODEX")) else {
        println!("AIJ_LAB_PROJECT and AIJ_LAB_CODEX aren't set: skipped");
        return;
    };
    std::fs::create_dir_all(&project).unwrap();
    let id = codex_turn(
        &home,
        &project,
        None,
        &format!("Remember the codename {EARLY} for later. Reply only OK."),
    )
    .0
    .expect("a thread id");
    for prompt in [
        "Reply only OK.".to_owned(),
        format!("Remember the codename {LATE} for later. Reply only OK."),
        "Reply only OK.".to_owned(),
    ] {
        codex_turn(&home, &project, Some(&id), &prompt);
    }
    let rollout = find(&home.join("sessions"), &id).expect("the thread's rollout");
    let adapter = CodexAdapter::new();
    let reference = TranscriptRef {
        path: rollout.clone(),
        native_id: Some(id.clone()),
    };
    let preview = adapter.cut_preview(&reference, Cut::KeepLast(2)).unwrap();
    println!("codex: preview {preview:?}");
    assert_eq!(preview.turns_left_out, 2);
    let bundle = adapter
        .bundle_session_cut(&reference, Cut::KeepLast(2))
        .unwrap();
    let installed = adapter
        .install_bundle(&bundle, &account("codex", &home))
        .unwrap();
    assert!(
        installed.forked,
        "beside the original, which keeps everything"
    );
    let forked = installed.reference.native_id.clone().unwrap();
    let (_, answer) = codex_turn(&home, &project, Some(&forked), ASK);
    judge("Codex", &answer);
}
