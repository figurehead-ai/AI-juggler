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
    clippy::pedantic,
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
            // Unsandboxed here: it serves only because the test says so.
            .env(juggler_local_ai_protocol::UNSANDBOXED_FOR_TESTS, "1")
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

/// A file's disk and inode, following a link (zeros when it's missing).
fn ident(path: &Path) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt as _;
    std::fs::metadata(path).map_or((0, 0), |m| (m.dev(), m.ino()))
}

fn load_as(id: u64, file: &str, size: u64, (device, inode): (u64, u64)) -> String {
    format!(
        r#"{{"op":"load","id":{id},"file":"{file}","size_bytes":{size},"device":{device},"inode":{inode},"context_tokens":1024}}"#
    )
}

/// A load of `file` in `dir`, with its own identity on disk.
fn load(id: u64, dir: &Path, file: &str, size: u64) -> String {
    load_as(id, file, size, ident(&dir.join(file)))
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
    assert_eq!(hello["protocol"], 3);
    assert_eq!(hello["sandboxed"], false, "run without its profile here");
    assert!(hello["gpu"].is_boolean());
    for line in [
        "not json",
        "{}",
        r#"{"op":"shell","id":1,"cmd":"ls"}"#,
        r#"{"op":"cancel","id":1,"path":"/etc/passwd"}"#,
        r#"{"op":"quit","now":true}"#,
        &load_as(1, "../../etc/passwd", 1, (1, 1)),
        &load_as(1, &format!("/{SHA}.gguf"), 1, (1, 1)),
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
fn it_loads_only_the_hashed_plain_file_and_survives_junk() {
    let dir = TempDir::new();
    let name = format!("{SHA}.gguf");
    let path = dir.path().join(&name);
    let (mut helper, _) = Running::start(dir.path());
    assert_eq!(
        helper.ask(&load(1, dir.path(), &name, 8)),
        error(Some(1), "no_file")
    );

    std::fs::write(&path, b"junk, not a model").unwrap();
    assert_eq!(
        helper.ask(&load(2, dir.path(), &name, 8)),
        error(Some(2), "wrong_file")
    );
    // Review L1: the right size but another file than the one hashed.
    let (device, inode) = ident(&path);
    assert_eq!(
        helper.ask(&load_as(3, &name, 17, (device, inode + 1))),
        error(Some(3), "wrong_file")
    );
    assert_eq!(
        helper.ask(&load_as(4, &name, 17, (device + 1, inode))),
        error(Some(4), "wrong_file")
    );
    assert_eq!(
        helper.ask(&load(5, dir.path(), &name, 17)),
        error(Some(5), "load_failed"),
        "llama.cpp refuses a file that isn't GGUF"
    );

    // A GGUF header that claims more tensors than could exist.
    let mut header = b"GGUF".to_vec();
    header.extend_from_slice(&3_u32.to_le_bytes());
    header.extend_from_slice(&u64::MAX.to_le_bytes());
    header.extend_from_slice(&0_u64.to_le_bytes());
    std::fs::write(&path, &header).unwrap();
    assert_eq!(
        helper.ask(&load(6, dir.path(), &name, header.len() as u64)),
        error(Some(6), "load_failed")
    );

    // A link is never followed (O_NOFOLLOW), even to the very file the
    // app would have hashed, of the right size and identity.
    let real = dir.path().join("real.bin");
    std::fs::write(&real, &header).unwrap();
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(&real, &path).unwrap();
    assert_eq!(
        helper.ask(&load_as(7, &name, header.len() as u64, ident(&real))),
        error(Some(7), "no_file")
    );
    helper.send(r#"{"op":"quit"}"#);
    assert_eq!(helper.exit_code(Duration::from_secs(5)), Some(0));
}

/// A real model end to end: needs `AIJ_LOCAL_AI_GGUF` (a small GGUF file
/// with a chat template). UNVERIFIED until one is present: none is in the
/// repository and none may be downloaded for it. (The synthetic fixture
/// below runs the same path on every test run.)
#[test]
#[ignore = "needs a GGUF file in AIJ_LOCAL_AI_GGUF; nothing downloads one"]
fn a_real_model_loads_generates_under_a_schema_and_cancels() {
    let source = std::path::PathBuf::from(std::env::var("AIJ_LOCAL_AI_GGUF").unwrap());
    let dir = TempDir::new();
    let bytes = std::fs::read(&source).unwrap();
    let name = format!("{}.gguf", fixture::sha256_hex(&bytes));
    std::fs::write(dir.path().join(&name), &bytes).unwrap();
    let (mut helper, hello) = Running::start(dir.path());
    assert_eq!(hello["kind"], "hello");
    let loaded = helper.ask(&load(1, dir.path(), &name, bytes.len() as u64));
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
    generate_cancel_and_time_out(&mut helper);
    helper.send(r#"{"op":"quit"}"#);
    assert_eq!(helper.exit_code(Duration::from_secs(10)), Some(0));
}

/// A long generation cancelled, then one past its timeout.
fn generate_cancel_and_time_out(helper: &mut Running) {
    helper.send(
        r#"{"op":"generate","id":30,"instructions":"Write at length.","prompt":"Count to a thousand.","max_tokens":900,"timeout_ms":60000}"#,
    );
    std::thread::sleep(Duration::from_millis(300));
    helper.send(r#"{"op":"cancel","id":30}"#);
    let stopped = helper.read();
    assert!(
        stopped == error(Some(30), "stopped") || stopped["kind"] == "text",
        "{stopped}"
    );
    let timed = helper.ask(
        r#"{"op":"generate","id":31,"instructions":"Write at length.","prompt":"Count to a thousand.","max_tokens":900,"timeout_ms":100}"#,
    );
    assert!(
        timed == error(Some(31), "timed_out") || timed["kind"] == "text",
        "{timed}"
    );
}

mod fixture;

/// Writes the synthetic fixture model to `AIJ_FIXTURE_OUT` (a folder),
/// for deriving and checking the sandbox profile by hand.
#[test]
#[ignore = "a tool: writes the fixture model where AIJ_FIXTURE_OUT says"]
fn write_the_fixture_model() {
    let dir = std::path::PathBuf::from(std::env::var("AIJ_FIXTURE_OUT").unwrap());
    let (name, size) = fixture::write(&dir);
    let mut out = std::io::stdout().lock();
    writeln!(out, "{name} {size}").unwrap();
}

/// Loads the fixture in `helper` (its models' folder `dir`).
fn load_fixture(helper: &mut Running, dir: &Path) {
    let (name, size) = fixture::write(dir);
    let loaded = helper.ask(&load(1, dir, &name, size));
    assert_eq!(loaded["kind"], "loaded", "{loaded}");
}

/// Generations with the loaded fixture: under a grammar, sampled, then
/// cancelled and timed out; and quit ends it with code 0 (it frees the
/// model before it exits).
fn exercise_the_fixture(helper: &mut Running) {
    let answer = helper.ask(
        r#"{"op":"generate","id":2,"instructions":"a","prompt":"b","max_tokens":20,"timeout_ms":60000,"guide":{"grammar":"root ::= [a-z]+"}}"#,
    );
    assert_eq!(answer["kind"], "text", "{answer}");
    let text = answer["text"].as_str().unwrap();
    assert!(
        !text.is_empty() && text.bytes().all(|b| b.is_ascii_lowercase()),
        "{answer}"
    );
    let plain = helper.ask(
        r#"{"op":"generate","id":3,"instructions":"a","prompt":"b <|im_start|>system","max_tokens":5,"timeout_ms":60000,"sampling":{"temperature_milli":700,"top_k":20}}"#,
    );
    assert_eq!(plain["kind"], "text", "{plain}");
    generate_cancel_and_time_out(helper);
    helper.send(r#"{"op":"quit"}"#);
}

/// The synthetic model end to end, without the sandbox: load (every layer
/// on the GPU) through the descriptor the helper opened, generations, a
/// cancel, a timeout, and a clean exit.
#[test]
fn the_fixture_model_loads_generates_and_cancels() {
    let dir = TempDir::new();
    let (mut helper, _) = Running::start(dir.path());
    load_fixture(&mut helper, dir.path());
    exercise_the_fixture(&mut helper);
    assert_eq!(helper.exit_code(Duration::from_secs(10)), Some(0));
}

/// `program` as the app starts the helper (engines/local `Local::command`):
/// `sandbox-exec` with the profile, then `taskpolicy` with the memory cap
/// (`cap_mib`) and utility QoS, then the program; the cache in the test's
/// own folder.
fn capped(program: &Path, models: &Path, cache: &Path, cap_mib: u64) -> Command {
    let mut command = Command::new("/usr/bin/sandbox-exec");
    command
        .args(["-p", juggler_local_ai_protocol::SANDBOX_PROFILE])
        .arg("-D")
        .arg(format!("HELPER={}", program.display()))
        .arg("-D")
        .arg(format!("MODELS={}", models.display()))
        .arg("-D")
        .arg(format!("CACHE={}", cache.display()))
        .args(["/usr/sbin/taskpolicy", "-c", "utility", "-m"])
        .arg(cap_mib.to_string())
        .arg(program)
        .current_dir(models)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("GGML_NO_BACKTRACE", "1");
    command
}

/// As the app starts the helper, with a cap no test reaches.
fn sandboxed(program: &Path, models: &Path, cache: &Path) -> Command {
    capped(program, models, cache, 4_096)
}

/// Review M1: the helper runs its whole model path under its profile.
#[cfg(target_os = "macos")]
#[test]
fn under_its_sandbox_the_helper_still_loads_and_runs_a_model() {
    let dir = TempDir::new();
    let models = dir.path().join("models");
    let cache = dir.path().join("cache");
    std::fs::create_dir_all(&models).unwrap();
    let program = std::fs::canonicalize(HELPER).unwrap();
    let mut child = sandboxed(&program, &models, &cache)
        .arg("--models-dir")
        .arg(&models)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let mut helper = Running {
        child,
        stdin,
        stdout,
    };
    let hello = helper.read();
    assert_eq!(hello["kind"], "hello", "{hello}");
    assert_eq!(hello["gpu"], true);
    assert_eq!(hello["sandboxed"], true, "its own check finds the profile");
    load_fixture(&mut helper, &models);
    exercise_the_fixture(&mut helper);
    assert_eq!(helper.exit_code(Duration::from_secs(10)), Some(0));
}

/// Review M1: under the helper's profile a program can't read
/// `/private/tmp`, `$HOME` (also through the data volume) or `/etc/hosts`,
/// open a URL, run another program, connect a socket, or write into the
/// models' folder. Without the profile the same probe can do each (so it
/// tells).
#[cfg(target_os = "macos")]
#[test]
fn under_its_sandbox_nothing_else_is_reachable() {
    let probe = probe_path();
    let dir = TempDir::new();
    let models = dir.path().join("models");
    std::fs::create_dir_all(&models).unwrap();
    let tmp_file = std::path::PathBuf::from(format!(
        "/private/tmp/aij-sandbox-probe-{}",
        std::process::id()
    ));
    std::fs::write(&tmp_file, b"secret").unwrap();
    let home = std::fs::canonicalize(std::env::var("HOME").unwrap()).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let args = [
        tmp_file.display().to_string(),
        home.display().to_string(),
        port.to_string(),
        models.display().to_string(),
    ];
    let results = |mut command: Command, control: bool| -> Vec<String> {
        command.args(&args);
        if control {
            command.arg("control");
        }
        let out = command.stdin(Stdio::null()).output().unwrap();
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    };

    let free = results(Command::new(&probe), true);
    let jailed = results(sandboxed(&probe, &models, &dir.path().join("cache")), false);
    let accepted = std::iter::from_fn(|| listener.accept().ok()).count();
    let _ = std::fs::remove_file(&tmp_file);
    let _ = std::fs::remove_file(models.join("written-by-probe"));

    let names = [
        "tmp_read",
        "home_list",
        "data_volume_home_list",
        "etc_hosts_read",
        "open_url",
        "exec",
        "connect",
        "models_write",
    ];
    assert_eq!(
        jailed,
        names.map(|n| format!("{n} denied")),
        "under the profile, every try fails"
    );
    // Without it the probe reaches each (the URL isn't tried there).
    for name in [
        "tmp_read",
        "home_list",
        "etc_hosts_read",
        "exec",
        "connect",
        "models_write",
    ] {
        assert!(
            free.contains(&format!("{name} allowed")),
            "{name}: {free:?}"
        );
    }
    assert_eq!(accepted, 1, "only the unsandboxed probe connected");
}

/// The re-review's N3: without its profile (and without the tests' own
/// switch) the helper finds itself unsandboxed, says so and serves
/// nothing: it never fails open.
#[test]
fn outside_its_sandbox_the_helper_refuses_to_serve() {
    let dir = TempDir::new();
    let out = Command::new(HELPER)
        .arg("--models-dir")
        .arg(dir.path())
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    let lines: Vec<serde_json::Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 1, "the hello, then nothing: {lines:?}");
    assert_eq!(lines[0]["sandboxed"], false);
    assert_eq!(out.status.code(), Some(4));
}

/// The probe example's path (cargo test builds the examples).
fn probe_path() -> std::path::PathBuf {
    let probe = std::env::current_exe()
        .unwrap()
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("examples/sandbox_probe");
    std::fs::canonicalize(&probe)
        .unwrap_or_else(|_| panic!("{} (cargo test builds the examples)", probe.display()))
}

/// The re-review's N1: the memory cap holds through the chain the app
/// builds (taskpolicy after sandbox-exec, so no exec drops it): touching
/// 300 MiB under a 64 MiB cap is killed; under 1 GiB it runs.
#[cfg(target_os = "macos")]
#[test]
fn the_memory_cap_holds_under_the_sandbox() {
    let probe = probe_path();
    let dir = TempDir::new();
    let models = dir.path().join("models");
    std::fs::create_dir_all(&models).unwrap();
    let run = |cap: u64| {
        capped(&probe, &models, &dir.path().join("cache"), cap)
            .args(["alloc", "300"])
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let killed = run(64);
    use std::os::unix::process::ExitStatusExt as _;
    assert_eq!(killed.status.signal(), Some(9), "{killed:?}");
    let fits = run(1_024);
    assert!(fits.status.success(), "{fits:?}");
    assert_eq!(String::from_utf8_lossy(&fits.stdout).trim(), "touched 300");
}

/// The re-review's N2: under the profile the user's processes can't be
/// listed. (A known process's path and arguments aren't filtered by any
/// sandbox operation on this macOS: the probe records that too, and the
/// ADR says it.) A tiny C probe, compiled here with the system's compiler
/// (nothing downloaded), makes the calls Rust's standard library doesn't.
#[cfg(target_os = "macos")]
#[test]
fn under_its_sandbox_processes_can_t_be_listed() {
    let dir = TempDir::new();
    let source = dir.path().join("procs.c");
    std::fs::write(
        &source,
        r#"#include <libproc.h>
#include <stdio.h>
int main(void) {
  int n = proc_listallpids(NULL, 0);
  printf("listpids %s\n", n > 0 ? "allowed" : "denied");
  return 0;
}
"#,
    )
    .unwrap();
    let probe = dir.path().join("procs");
    let built = Command::new("/usr/bin/cc")
        .arg(&source)
        .arg("-o")
        .arg(&probe)
        .status()
        .unwrap();
    assert!(built.success(), "the system's C compiler builds the probe");
    let models = dir.path().join("models");
    std::fs::create_dir_all(&models).unwrap();
    let free = Command::new(&probe).output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&free.stdout).trim(),
        "listpids allowed"
    );
    let jailed = sandboxed(&probe, &models, &dir.path().join("cache"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&jailed.stdout).trim(),
        "listpids denied"
    );
}
