//! `juggler-agent` (AJ-E1704): AI Juggler's pool tools for the agent in
//! a session. See the library's docs; this only wires stdin, stdout and
//! the exit code.

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
            let given = std::env::var(juggler_agentbridge::wire::TOKEN_ENV)
                .is_ok_and(|token| !token.is_empty());
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            match serve_as(stdin.lock(), stdout.lock(), &mut app, Some(given)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => ExitCode::FAILURE,
            }
        }
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
