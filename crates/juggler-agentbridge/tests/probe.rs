//! AJ-E3013 (lab proofs C2, C3): the probe, through the real debug
//! binary: it writes only in a lab folder, a `0600` file, the names of its
//! environment and never a value but the three ids, its parent chain, the
//! client's name and each request's `_meta` keys; `--probe-shell` prints
//! the thread id and the chain, nothing else. Nothing real is read.
#![cfg(all(unix, debug_assertions))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test: a step that fails is the failure"
)]

use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BRIDGE: &str = env!("CARGO_BIN_EXE_juggler-agent");

/// A throwaway lab folder under /tmp, removed on drop.
struct Lab(PathBuf);

impl Lab {
    fn new(tag: &str) -> Self {
        let dir = PathBuf::from(format!("/tmp/aij-lab-probe-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(dir)
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs `--mcp --probe <out>` with `lines` on its stdin; its exit and stdout.
fn probe(out: &Path, lines: &[&str]) -> (bool, String) {
    let mut child = Command::new(BRIDGE)
        .args(["--mcp", "--probe"])
        .arg(out)
        .env("CODEX_THREAD_ID", "thr-123")
        .env("AIJ_PROBE_TEST_SECRET", "do-not-write-me")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for line in lines {
            writeln!(stdin, "{line}").unwrap();
        }
    }
    let output = child.wait_with_output().unwrap();
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

#[test]
fn the_probe_records_names_ids_its_chain_the_client_and_meta_keys_in_a_lab_file() {
    let lab = Lab::new("mcp");
    let out = lab.0.join("probe.jsonl");
    let (ok, answers) = probe(
        &out,
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"codex-app-test","version":"9"}}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"probe","arguments":{},"_meta":{"progressToken":7,"x-thread":"t-9","x-codex-turn-metadata":{"threadId":"thr-123","note":"do-not-write-me"}}}}"#,
        ],
    );
    assert!(ok, "{answers}");
    assert!(
        answers.contains("\"probe\""),
        "lists its one tool: {answers}"
    );
    assert!(answers.contains("recorded"), "{answers}");
    let mode = std::fs::metadata(&out).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.lines().all(|line| line.contains("AIJ_PROBE_V1")));
    assert!(
        text.lines().all(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .is_ok_and(|line| line["pid"].as_u64().is_some_and(|pid| pid > 0))
        }),
        "every line names the probe's process: {text}"
    );
    assert!(
        !text.contains("do-not-write-me"),
        "never a value but the ids"
    );
    let record: serde_json::Value = text
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|line| line.get("env_names").is_some())
        .expect("the probe call's record");
    let names = record["env_names"].as_array().unwrap();
    assert!(names.iter().any(|name| name == "AIJ_PROBE_TEST_SECRET"));
    assert_eq!(record["ids"]["CODEX_THREAD_ID"], "thr-123");
    assert!(record["ids"].get("CLAUDE_CODE_SESSION_ID").is_none());
    assert_eq!(record["client_info"]["name"], "codex-app-test");
    let meta: Vec<&str> = record["meta_keys"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert_eq!(meta, ["progressToken", "x-codex-turn-metadata", "x-thread"]);
    assert_eq!(
        record["meta_ids"]["x-codex-turn-metadata.threadId"], "thr-123",
        "a thread id one object down is written, its neighbours' values never"
    );
    let chain = record["chain"].as_array().unwrap();
    assert!(
        chain.iter().any(|step| step["pid"] == std::process::id()),
        "its parent chain reaches this test: {chain:?}"
    );
}

#[test]
fn the_probe_refuses_any_folder_but_a_labs_and_a_link() {
    let (ok, _) = probe(Path::new("/tmp/aij-probe-not-a-lab.jsonl"), &[]);
    assert!(!ok, "/tmp itself isn't a lab");
    assert!(!Path::new("/tmp/aij-probe-not-a-lab.jsonl").exists());
    let lab = Lab::new("link");
    let elsewhere = tempfile::tempdir().unwrap();
    let target = elsewhere.path().join("target");
    std::fs::write(&target, "keep").unwrap();
    std::os::unix::fs::symlink(&target, lab.0.join("probe.jsonl")).unwrap();
    let (ok, _) = probe(&lab.0.join("probe.jsonl"), &[]);
    assert!(!ok, "never through a link");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "keep");
}

#[test]
fn probe_shell_prints_the_thread_id_and_the_chain_only() {
    let output = Command::new(BRIDGE)
        .arg("--probe-shell")
        .env("CODEX_THREAD_ID", "thr-456")
        .env("AIJ_PROBE_TEST_SECRET", "do-not-print-me")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("CODEX_THREAD_ID=thr-456"));
    let me = std::process::id().to_string();
    assert!(
        lines.any(|line| line.split_whitespace().next() == Some(me.as_str())),
        "{text}"
    );
    assert!(!text.contains("do-not-print-me"));
}
