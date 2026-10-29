//! `fake-codex` — a stand-in for the Codex CLI's app-server (AJ-E2303), so
//! the driven runtime is tested end to end without a real Codex or
//! account:
//!
//! ```text
//! fake-codex --version                                   codex-cli 0.163.0
//! fake-codex app-server generate-json-schema --out DIR   the method lists
//! fake-codex app-server [--listen stdio://]               the protocol, on stdio
//! ```
//!
//! `FAKE_CODEX_VERSION` changes the version it reports; `FAKE_CODEX_DROP`
//! names a method its schema leaves out (the drift fixture);
//! `FAKE_CODEX_HOME` makes `initialize` report another `codexHome` (the
//! wrong-home fixture).
//!
//! The protocol (one JSON-RPC message per line, no `jsonrpc` field) serves
//! the subset AI Juggler uses, shaped as the 0.163.0 schema says
//! (`tests/fixtures/codex-app-server/surface-0.163.0.json`). A turn whose
//! text holds `ask:` asks to run a command; `question:` asks the user;
//! `file:` asks to change files; `fail:` fails with a usage limit. Threads
//! are written as rollouts under `CODEX_HOME/sessions/`, so the readers
//! find them like any other. It exits when its stdin closes, so a leaked
//! fixture never lingers. (Only stdio is faked: the real `--listen
//! unix://` speaks WebSocket, and AI Juggler doesn't use it.)

#[path = "support/app_server.rs"]
mod app_server;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The tests read what each launch passed (`FAKE_CODEX_ARGS_LOG`, one
    // line a launch).
    if let Some(log) = std::env::var_os("FAKE_CODEX_ARGS_LOG") {
        use std::io::Write as _;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
        {
            let _ = writeln!(file, "{}", args.join(" "));
        }
    }
    // Root options in front (`-c key=value`), as Codex takes them.
    let mut words: Vec<&str> = args.iter().map(String::as_str).collect();
    while words.first() == Some(&"-c") && words.len() >= 2 {
        words.drain(..2);
    }
    match words.as_slice() {
        ["--version" | "-V"] => {
            let version =
                std::env::var("FAKE_CODEX_VERSION").unwrap_or_else(|_| "0.163.0".to_owned());
            println!("codex-cli {version}");
            ExitCode::SUCCESS
        }
        ["app-server", "generate-json-schema", "--out", dir] => {
            match app_server::write_schema(std::path::Path::new(dir)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("fake-codex: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        ["app-server"] | ["app-server", "--listen", "stdio://"] => {
            app_server::serve_stdio();
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("fake-codex: {words:?} isn't faked");
            ExitCode::from(2)
        }
    }
}
