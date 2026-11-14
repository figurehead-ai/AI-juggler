//! `fake-kimi` — a stand-in for Kimi Code's `kimi web` (AJ-E1603 R3), so
//! the self-started usage check is tested end to end without a real Kimi
//! Code, sign-in or network:
//!
//! ```text
//! fake-kimi --version                         2.6.1
//! fake-kimi web --no-open --port 0            the server, until stopped
//! ```
//!
//! As the real one does (`kap-server/src/instanceRegistry.ts`,
//! `start.ts`), it records itself in `$KIMI_CODE_HOME/server/instances/`
//! with its pid and port 0, listens on 127.0.0.1 at a free port, then
//! writes that port into the record; it prints a banner holding a token
//! (which the caller must never read), answers `GET /api/v1/healthz` and,
//! with `Authorization: Bearer $KIMI_CODE_PASSWORD`, `GET
//! /api/v1/oauth/usage` (anything else: 401).
//!
//! `$KIMI_CODE_HOME/fake-kimi.json` sets its behaviour: `{"mode": "ok" |
//! "never-register" | "exit" | "ignore-term", "usage": "<body>"}`. What it
//! saw goes to `$KIMI_CODE_HOME/fake-kimi-seen.json` (its pid, the variable
//! names it was given, whether each request carried the secret, never a
//! value), and each start adds a line to `fake-kimi-launches.log`. It
//! ends by itself after a minute, so a leaked fixture never lingers.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool: its output is what it prints"
)]

use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The longest it lives.
const LIFETIME: Duration = Duration::from_secs(60);

const USAGE: &str = r#"{"code":0,"msg":"ok","request_id":"fake","data":{"kind":"ok","quota":{"usages":{"limit5h":{"usedRatio":0.42,"resetAt":"2026-11-13T12:00:00Z"},"limit7d":{"usedRatio":0.1},"monthTotal":{"usedRatio":0.2},"monthCode":{"usedRatio":0.15}},"extraUsage":null}}}"#;

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

fn config(home: &Path) -> (String, String) {
    let raw = std::fs::read_to_string(home.join("fake-kimi.json")).unwrap_or_default();
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    let mode = value
        .get("mode")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("ok")
        .to_owned();
    let usage = value
        .get("usage")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(USAGE)
        .to_owned();
    (mode, usage)
}

fn record(dir: &Path, port: u16, started: u128) {
    let body = format!(
        r#"{{"server_id":"01FAKE","pid":{},"host":"127.0.0.1","port":{port},"started_at":{started},"heartbeat_at":{},"host_version":"2.6.1"}}"#,
        std::process::id(),
        now_ms()
    );
    let tmp = dir.join(".01FAKE.tmp");
    let _ = std::fs::write(&tmp, body);
    let _ = std::fs::rename(&tmp, dir.join("01FAKE.json"));
}

#[derive(serde::Serialize, Default)]
struct Seen {
    pid: u32,
    env: Vec<String>,
    password_set: bool,
    requests: Vec<SeenRequest>,
}

#[derive(serde::Serialize)]
struct SeenRequest {
    path: String,
    host: Option<String>,
    origin: bool,
    authorized: Option<bool>,
}

fn save(home: &Path, seen: &Seen) {
    let _ = std::fs::write(
        home.join("fake-kimi-seen.json"),
        serde_json::to_string(seen).unwrap_or_default(),
    );
}

fn header(head: &str, name: &str) -> Option<String> {
    head.lines().skip(1).find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim()
            .eq_ignore_ascii_case(name)
            .then(|| value.trim().to_owned())
    })
}

fn serve(home: &Path, listener: &TcpListener, usage: &str, seen: &mut Seen) {
    let password = std::env::var("KIMI_CODE_PASSWORD").ok();
    let started = Instant::now();
    let _ = listener.set_nonblocking(true);
    while started.elapsed() < LIFETIME {
        let Ok((mut stream, _)) = listener.accept() else {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        };
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut raw = Vec::new();
        let mut buffer = [0_u8; 2048];
        while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => raw.extend_from_slice(&buffer[..read]),
            }
        }
        let head = String::from_utf8_lossy(&raw).into_owned();
        let path = head
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
            .to_owned();
        let auth = header(&head, "authorization");
        let authorized = auth.as_ref().map(|value| {
            password
                .as_ref()
                .is_some_and(|password| *value == format!("Bearer {password}"))
        });
        let (status, body) = match path.as_str() {
            "/api/v1/healthz" => ("200 OK", r#"{"code":0,"data":{"ok":true}}"#.to_owned()),
            "/api/v1/oauth/usage" if authorized == Some(true) => ("200 OK", usage.to_owned()),
            _ => (
                "401 Unauthorized",
                r#"{"code":40101,"msg":"Unauthorized"}"#.to_owned(),
            ),
        };
        seen.requests.push(SeenRequest {
            path,
            host: header(&head, "host"),
            origin: header(&head, "origin").is_some(),
            authorized,
        });
        save(home, seen);
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        );
    }
}

#[cfg(unix)]
fn ignore_term() -> ExitCode {
    use std::os::unix::process::CommandExt;
    // Becomes a shell that ignores SIGTERM (same pid), for the kill step.
    let error = std::process::Command::new("/bin/sh")
        .args([
            "-c",
            "trap '' TERM; i=0; while [ $i -lt 60 ]; do sleep 1; i=$((i+1)); done",
        ])
        .exec();
    eprintln!("fake-kimi: {error}");
    ExitCode::FAILURE
}

#[cfg(not(unix))]
fn ignore_term() -> ExitCode {
    std::thread::sleep(LIFETIME);
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--version") {
        println!("2.6.1");
        return ExitCode::SUCCESS;
    }
    if args != ["web", "--no-open", "--port", "0"] {
        eprintln!("fake-kimi: unexpected arguments {args:?}");
        return ExitCode::from(2);
    }
    let Some(home) = std::env::var_os("KIMI_CODE_HOME").map(PathBuf::from) else {
        eprintln!("fake-kimi: no KIMI_CODE_HOME");
        return ExitCode::from(2);
    };
    let (mode, usage) = config(&home);
    let mut env: Vec<String> = std::env::vars_os()
        .map(|(key, _)| key.to_string_lossy().into_owned())
        .collect();
    env.sort();
    // One line a start, so a test can count them.
    let launches = home.join("fake-kimi-launches.log");
    let before = std::fs::read_to_string(&launches).unwrap_or_default();
    let _ = std::fs::write(&launches, format!("{before}{}\n", std::process::id()));
    let mut seen = Seen {
        pid: std::process::id(),
        env,
        password_set: std::env::var_os("KIMI_CODE_PASSWORD").is_some(),
        requests: Vec::new(),
    };
    save(&home, &seen);
    // The banner the real one prints, with a token the caller must never use.
    println!("Kimi server: http://127.0.0.1:0/#token=fake-banner-token-0000");
    match mode.as_str() {
        "exit" => return ExitCode::from(3),
        "never-register" => {
            std::thread::sleep(LIFETIME);
            return ExitCode::SUCCESS;
        }
        "ignore-term" => return ignore_term(),
        _ => {}
    }
    let dir = home.join("server").join("instances");
    if std::fs::create_dir_all(&dir).is_err() {
        return ExitCode::FAILURE;
    }
    let started = now_ms();
    record(&dir, 0, started);
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else {
        return ExitCode::FAILURE;
    };
    let port = listener.local_addr().map_or(0, |address| address.port());
    // Listening a moment before the port is recorded, as the real one.
    std::thread::sleep(Duration::from_millis(150));
    record(&dir, port, started);
    serve(&home, &listener, &usage, &mut seen);
    ExitCode::SUCCESS
}
