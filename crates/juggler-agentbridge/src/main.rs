//! `juggler-agent` (AJ-E1704): AI Juggler's pool tools for the agent in
//! a session. See the library's docs; this only wires stdin, stdout and
//! the exit code.

use std::io::Write;
use std::process::ExitCode;

use juggler_agentbridge::cli::{App, Command, USAGE, exit_code, parse, serve};
use juggler_agentbridge::mcp::Backend;

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
        Ok(Command::Help) => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::Hook(event)) => {
            // Silent and always 0: a hook never decides for the tool.
            juggler_agentbridge::hook::run(&event, std::io::stdin().lock());
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("juggler-agent {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::Call(call, text)) => {
            let outcome = app.call(call, text);
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
        Err(words) => {
            let _ = writeln!(std::io::stderr(), "{words}");
            ExitCode::from(64)
        }
    }
}
