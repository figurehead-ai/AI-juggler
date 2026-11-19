//! The real helper binary against its protocol, with no model: its
//! arguments, the hello, malformed, unknown and oversized lines, a load of
//! a missing, mis-sized, linked or junk file, generate before load, cancel
//! and quit. Then one smoke test with a real model, `#[ignore]`d: it needs
//! a small GGUF file named by `AIJ_LOCAL_AI_GGUF` (nothing here downloads
//! one).
//!
//! The binary under test is unsigned, so it runs without its App Sandbox:
//! what the sandbox adds (no network, reads only the models' folders) is
//! checked on the signed copy by `scripts/build_local_ai_helper.sh check`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    reason = "a test fails loudly"
)]

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

const HELPER: &str = env!("CARGO_BIN_EXE_juggler-local-ai");
const SHA: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// A fresh folder under the system's temporary folder, removed when
/// dropped (no `tempfile` in this workspace's lock).
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("juggler-local-ai-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(std::fs::canonicalize(dir).unwrap())
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Running {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl Running {
    fn start(dir: &Path) -> (Self, serde_json::Value) {
        let mut child = Command::new(HELPER)
            .arg("--models-dir")
            .arg(dir)
            .env("GGML_NO_BACKTRACE", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut running = Self {
            child,
            stdin,
            stdout,
        };
        let hello = running.read();
        (running, hello)
    }

    fn send(&mut self, line: &str) {
        let stdin = self.stdin.as_mut().unwrap();
        stdin.write_all(line.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    }

    fn read(&mut self) -> serde_json::Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|_| panic!("not an answer: {line:?}"))
    }

    fn ask(&mut self, line: &str) -> serde_json::Value {
        self.send(line);
        self.read()
    }

    fn exit_code(mut self, within: Duration) -> Option<i32> {
        let until = Instant::now() + within;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status.code();
            }
            if Instant::now() > until {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return None;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

fn error(id: Option<u64>, error: &str) -> serde_json::Value {
    serde_json::json!({ "kind": "error", "id": id, "error": error })
}

fn load(id: u64, file: &str, size: u64) -> String {
    format!(
        r#"{{"op":"load","id":{id},"file":"{file}","size_bytes":{size},"context_tokens":4096}}"#
    )
}

#[test]
fn it_takes_only_an_absolute_models_folder() {
    let dir = TempDir::new();
    let file = dir.path().join("f");
    std::fs::write(&file, b"x").unwrap();
    let cases: [&[&str]; 5] = [
        &[],
        &["--models-dir"],
        &["--models-dir", "relative"],
        &["--models-dir", file.to_str().unwrap()],
        &["--model", dir.path().to_str().unwrap()],
    ];
    for args in cases {
        let out = Command::new(HELPER)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}: no hello");
    }
    let extra = Command::new(HELPER)
        .arg("--models-dir")
        .arg(dir.path())
        .arg("--more")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(extra.status.code(), Some(2));
}

#[test]
fn it_says_hello_then_refuses_what_isnt_a_request() {
    let dir = TempDir::new();
    let (mut helper, hello) = Running::start(dir.path());
    assert_eq!(hello["kind"], "hello");
    assert_eq!(hello["protocol"], 1);
    assert!(hello["gpu"].is_boolean());
    for line in [
        "not json",
        "{}",
        r#"{"op":"shell","id":1,"cmd":"ls"}"#,
        r#"{"op":"cancel","id":1,"path":"/etc/passwd"}"#,
        r#"{"op":"quit","now":true}"#,
        &load(1, "../../etc/passwd", 1),
        &load(1, &format!("/{SHA}.gguf"), 1),
        r#"{"op":"generate","id":2,"instructions":"a","prompt":"b","max_tokens":0,"timeout_ms":1000}"#,
        r#"{"op":"generate","id":2,"instructions":"a","prompt":"b","max_tokens":10,"timeout_ms":1000,"sampling":{"temperature_milli":9999}}"#,
    ] {
        assert_eq!(helper.ask(line), error(None, "bad_request"), "{line}");
    }
    assert_eq!(
        helper.ask(
            r#"{"op":"generate","id":3,"instructions":"a","prompt":"b","max_tokens":10,"timeout_ms":1000}"#
        ),
        error(Some(3), "not_loaded")
    );
    // A cancel of nothing running has no answer; quit ends it.
    helper.send(r#"{"op":"cancel","id":3}"#);
    helper.send(r#"{"op":"quit"}"#);
    assert_eq!(helper.exit_code(Duration::from_secs(5)), Some(0));
}

#[test]
fn a_line_over_its_bound_ends_the_helper() {
    let dir = TempDir::new();
    let (mut helper, _) = Running::start(dir.path());
    let long = format!(
        r#"{{"op":"cancel","id":1,"x":"{}"}}"#,
        "a".repeat(32 * 1024)
    );
    assert_eq!(helper.ask(&long), error(None, "too_large"));
    assert_eq!(helper.exit_code(Duration::from_secs(5)), Some(2));
}

#[test]
fn closing_stdin_ends_it() {
    let dir = TempDir::new();
    let (mut helper, _) = Running::start(dir.path());
    drop(helper.stdin.take());
    assert_eq!(helper.exit_code(Duration::from_secs(5)), Some(0));
}

#[test]
fn it_loads_only_a_plain_file_of_the_exact_size_and_survives_junk() {
    let dir = TempDir::new();
    let name = format!("{SHA}.gguf");
    let (mut helper, _) = Running::start(dir.path());
    assert_eq!(helper.ask(&load(1, &name, 8)), error(Some(1), "no_file"));

    std::fs::write(dir.path().join(&name), b"junk, not a model").unwrap();
    assert_eq!(helper.ask(&load(2, &name, 8)), error(Some(2), "wrong_file"));
    assert_eq!(
        helper.ask(&load(3, &name, 17)),
        error(Some(3), "load_failed"),
        "llama.cpp refuses a file that isn't GGUF"
    );

    // A GGUF header that claims more tensors than could exist.
    let mut header = b"GGUF".to_vec();
    header.extend_from_slice(&3_u32.to_le_bytes());
    header.extend_from_slice(&u64::MAX.to_le_bytes());
    header.extend_from_slice(&0_u64.to_le_bytes());
    std::fs::write(dir.path().join(&name), &header).unwrap();
    assert_eq!(
        helper.ask(&load(4, &name, header.len() as u64)),
        error(Some(4), "load_failed")
    );

    // A link is never followed, even to a file of the right size.
    let real = dir.path().join("real.bin");
    std::fs::write(&real, &header).unwrap();
    std::fs::remove_file(dir.path().join(&name)).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, dir.path().join(&name)).unwrap();
    #[cfg(unix)]
    assert_eq!(
        helper.ask(&load(5, &name, header.len() as u64)),
        error(Some(5), "no_file")
    );
    helper.send(r#"{"op":"quit"}"#);
    assert_eq!(helper.exit_code(Duration::from_secs(5)), Some(0));
}

/// A real model end to end: needs `AIJ_LOCAL_AI_GGUF` (a small GGUF file
/// with a chat template). UNVERIFIED until one is present: none is in the
/// repository and none may be downloaded for it.
#[test]
#[ignore = "needs a GGUF file in AIJ_LOCAL_AI_GGUF; nothing downloads one"]
fn a_real_model_loads_generates_under_a_schema_and_cancels() {
    use sha2_free::sha256_hex;

    let source = std::path::PathBuf::from(std::env::var("AIJ_LOCAL_AI_GGUF").unwrap());
    let dir = TempDir::new();
    let bytes = std::fs::read(&source).unwrap();
    let name = format!("{}.gguf", sha256_hex(&bytes));
    std::fs::write(dir.path().join(&name), &bytes).unwrap();
    let (mut helper, hello) = Running::start(dir.path());
    assert_eq!(hello["kind"], "hello");
    let loaded = helper.ask(&load(1, &name, bytes.len() as u64));
    assert_eq!(loaded["kind"], "loaded", "{loaded}");

    let schema = r#"{\"type\":\"object\",\"properties\":{\"line\":{\"type\":\"string\"}},\"required\":[\"line\"],\"additionalProperties\":false}"#;
    let answer = helper.ask(&format!(
        r#"{{"op":"generate","id":2,"instructions":"Answer in JSON.","prompt":"Say hello.","guide":{{"json_schema":"{schema}"}},"max_tokens":40,"timeout_ms":60000}}"#
    ));
    assert_eq!(answer["kind"], "text", "{answer}");
    if answer["finish"] == "stop" {
        let fields: serde_json::Value =
            serde_json::from_str(answer["text"].as_str().unwrap()).unwrap();
        assert!(fields["line"].is_string(), "{answer}");
    }

    helper.send(
        r#"{"op":"generate","id":3,"instructions":"Write at length.","prompt":"Count to a thousand.","max_tokens":4000,"timeout_ms":60000}"#,
    );
    std::thread::sleep(Duration::from_millis(300));
    helper.send(r#"{"op":"cancel","id":3}"#);
    let stopped = helper.read();
    assert!(
        stopped == error(Some(3), "stopped") || stopped["kind"] == "text",
        "{stopped}"
    );
    let timed = helper.ask(
        r#"{"op":"generate","id":4,"instructions":"Write at length.","prompt":"Count to a thousand.","max_tokens":4000,"timeout_ms":100}"#,
    );
    assert_eq!(timed, error(Some(4), "timed_out"));
    helper.send(r#"{"op":"quit"}"#);
    assert_eq!(helper.exit_code(Duration::from_secs(10)), Some(0));
}

/// A SHA-256 for the smoke test's file name, without a new dependency.
mod sha2_free {
    pub fn sha256_hex(bytes: &[u8]) -> String {
        let out = std::process::Command::new("/usr/bin/shasum")
            .args(["-a", "256"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write as _;
                child.stdin.take().unwrap().write_all(bytes)?;
                child.wait_with_output()
            })
            .unwrap();
        String::from_utf8(out.stdout).unwrap()[..64].to_owned()
    }
}
