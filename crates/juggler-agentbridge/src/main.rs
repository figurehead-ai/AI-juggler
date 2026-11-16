//! `juggler-agent` (AJ-E1704): AI Juggler's pool tools for the agent in
//! a session. See the library's docs; this only wires stdin, stdout and
//! the exit code.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the agent bridge's command line: its answers are what it prints"
)]

use std::io::{Read, Write};
use std::process::ExitCode;

use juggler_agentbridge::cli::{App, Command, USAGE, exit_code, parse, serve, serve_as};
use juggler_agentbridge::mcp::Backend;
use juggler_agentbridge::wire::{Args, Call, DOC_MAX, DocArgs};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut app = App::from_env();
    match parse(&args) {
        Ok(Command::Mcp) => {
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            match serve(stdin.lock(), stdout.lock(), &mut app) {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => ExitCode::FAILURE,
            }
        }
        Ok(Command::McpPlugin) => {
            // E19: in a session AI Juggler started, its launch already
            // added the tools; the plugin's copy then stays empty.
            // AJ-E3013: elsewhere, the connector's keyless calls, which
            // the app attests (or refuses) on every call.
            let given = std::env::var(juggler_agentbridge::wire::TOKEN_ENV)
                .is_ok_and(|token| !token.is_empty());
            let mut app = App::for_connector();
            // D-119: in a desktop app's session (no key), a pool message the
            // pool's waking rule allows wakes it through its own inbox.
            if !given {
                juggler_agentbridge::waker::start(
                    juggler_agentbridge::client::connector_socket(),
                    juggler_agentbridge::waker::Inbox::from_env(),
                );
            }
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            match serve_as(stdin.lock(), stdout.lock(), &mut app, Some(given)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => ExitCode::FAILURE,
            }
        }
        Ok(Command::ConnectorHook(event)) => {
            // Silent and 0 when AI Juggler isn't running, the caller isn't
            // attested, or this is a session AI Juggler started.
            if let Some(note) =
                juggler_agentbridge::hook::run_connector(&event, std::io::stdin().lock())
            {
                let _ = writeln!(std::io::stdout(), "{note}");
            }
            ExitCode::SUCCESS
        }
        Ok(Command::McpProbe(out)) => probe_mcp(&out),
        Ok(Command::ProbeShell) => probe_shell(),
        Ok(Command::Help) => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::Hook(event)) => {
            // Always 0, and never a decision: at most a facts-only note
            // for the tool's context (E18), on the events that take one.
            if let Some(note) = juggler_agentbridge::hook::run(&event, std::io::stdin().lock()) {
                let _ = writeln!(std::io::stdout(), "{note}");
            }
            ExitCode::SUCCESS
        }
        Ok(Command::Share(name)) => {
            let mut content = String::new();
            let read = std::io::stdin()
                .lock()
                .take(u64::try_from(DOC_MAX + 1).unwrap_or(u64::MAX))
                .read_to_string(&mut content);
            if read.is_err() || content.len() > DOC_MAX {
                let _ = writeln!(
                    std::io::stderr(),
                    "A shared document is text of at most {} KiB.",
                    DOC_MAX / 1024
                );
                return ExitCode::from(64);
            }
            let args = Args {
                doc: Some(DocArgs { name, content }),
                ..Args::default()
            };
            print_outcome(app.call(Call::ShareDoc, args))
        }
        Ok(Command::Version) => {
            println!("juggler-agent {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::Call(call, args)) => print_outcome(app.call(call, *args)),
        Err(words) => {
            let _ = writeln!(std::io::stderr(), "{words}");
            ExitCode::from(64)
        }
    }
}

/// AJ-E3013 (lab proof C2): the probe's MCP server (debug builds only).
#[cfg(all(debug_assertions, unix))]
fn probe_mcp(out: &str) -> ExitCode {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    match juggler_agentbridge::probe::serve(
        std::path::Path::new(out),
        home.as_deref(),
        stdin.lock(),
        stdout.lock(),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            let _ = writeln!(std::io::stderr(), "juggler-agent: {why}");
            ExitCode::from(64)
        }
    }
}

/// AJ-E3013 (lab proof C3): `CODEX_THREAD_ID` and the parent chain.
#[cfg(all(debug_assertions, unix))]
fn probe_shell() -> ExitCode {
    for line in juggler_agentbridge::probe::shell_lines() {
        println!("{line}");
    }
    ExitCode::SUCCESS
}

/// A release build has no probe.
#[cfg(not(all(debug_assertions, unix)))]
fn probe_mcp(_out: &str) -> ExitCode {
    not_in_this_build()
}

/// A release build has no probe.
#[cfg(not(all(debug_assertions, unix)))]
fn probe_shell() -> ExitCode {
    not_in_this_build()
}

/// What a release build says to the probe's flags.
#[cfg(not(all(debug_assertions, unix)))]
fn not_in_this_build() -> ExitCode {
    let _ = writeln!(
        std::io::stderr(),
        "juggler-agent: the probe is not in this build (debug builds only)"
    );
    ExitCode::from(64)
}

/// Prints a call's result (stdout) or why not (stderr), and its code.
fn print_outcome(
    outcome: Result<serde_json::Value, juggler_agentbridge::client::CallError>,
) -> ExitCode {
    let code = exit_code(&outcome);
    match outcome {
        Ok(result) => {
            let text = serde_json::to_string_pretty(&result).unwrap_or_default();
            let _ = writeln!(std::io::stdout(), "{text}");
        }
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{}", error.words());
        }
    }
    ExitCode::from(code)
}
