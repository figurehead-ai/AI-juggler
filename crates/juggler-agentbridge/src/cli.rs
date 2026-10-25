//! The program's two faces: the MCP server (`--mcp`), and a plain
//! command line for tools without MCP, called from their shell. There is
//! no option for the key: it comes from the environment only, so it is
//! never in a process's arguments.

use std::io::{BufRead, Write};

use serde_json::Value;

use crate::client::{CallError, Target, call};
use crate::mcp::{Backend, handle_line};
use crate::wire::{Call, Refusal, clean_status};

/// The app, reached with this process's own key.
pub struct App {
    target: Result<Target, CallError>,
}

impl App {
    /// From this process's environment.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            target: Target::from_env(),
        }
    }
}

impl Backend for App {
    fn call(&mut self, what: Call, text: Option<String>) -> Result<Value, CallError> {
        call(self.target.as_ref().map_err(Clone::clone)?, what, text)
    }
}

/// The help text.
pub const USAGE: &str = "juggler-agent: AI Juggler's pool tools for the agent in this session.

  juggler-agent whoami          this session, its pool, its role and what the pool allows
  juggler-agent roster          the other sessions in the pool
  juggler-agent cards           the pool's context cards
  juggler-agent status <text>   set this session's one-line status (empty clears it)
  juggler-agent --mcp           serve the same as an MCP server on stdin and stdout

Works in sessions AI Juggler started with its agent tools on. What the pool
lets agents do is set by the user, in AI Juggler.";

/// What the command line asked for.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// `--mcp`.
    Mcp,
    /// `--help`.
    Help,
    /// `--version`.
    Version,
    /// One call.
    Call(Call, Option<String>),
}

/// Reads the arguments (without the program's name).
///
/// # Errors
///
/// The words to show when they don't make a command.
pub fn parse(args: &[String]) -> Result<Command, String> {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--mcp"] => Ok(Command::Mcp),
        [] | ["--help" | "-h" | "help"] => Ok(Command::Help),
        ["--version" | "-V"] => Ok(Command::Version),
        ["whoami"] => Ok(Command::Call(Call::Whoami, None)),
        ["roster" | "pool_roster"] => Ok(Command::Call(Call::PoolRoster, None)),
        ["cards"] => Ok(Command::Call(Call::Cards, None)),
        ["status" | "set_status", text @ ..] => Ok(Command::Call(
            Call::SetStatus,
            Some(clean_status(&text.join(" "))),
        )),
        _ => Err(format!("{USAGE}\n\nNot a command: {}", words.join(" "))),
    }
}

/// Serves MCP until the tool closes stdin (the session ended).
///
/// # Errors
///
/// When stdin or stdout fails.
pub fn serve(
    input: impl BufRead,
    mut output: impl Write,
    backend: &mut dyn Backend,
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if let Some(reply) = handle_line(&line, backend) {
            output.write_all(reply.as_bytes())?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
    Ok(())
}

/// The exit code for a call's outcome: 0 done, 2 refused, 1 not reached.
#[must_use]
pub fn exit_code(outcome: &Result<Value, CallError>) -> u8 {
    match outcome {
        Ok(_) => 0,
        Err(CallError::Refused(Refusal::Ask | Refusal::Deny | Refusal::Off, _)) => 2,
        Err(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, parse, serve};
    use crate::client::CallError;
    use crate::mcp::Backend;
    use crate::wire::Call;
    use serde_json::{Value, json};

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn the_command_line_has_no_way_to_pass_a_key() {
        assert_eq!(parse(&words("--mcp")), Ok(Command::Mcp));
        assert_eq!(
            parse(&words("whoami")),
            Ok(Command::Call(Call::Whoami, None))
        );
        assert_eq!(
            parse(&words("status Fixing   the checkout")),
            Ok(Command::Call(
                Call::SetStatus,
                Some("Fixing the checkout".into())
            ))
        );
        for bad in [
            "--token abc",
            "--mcp --token abc",
            "whoami --token abc",
            "set_level full",
        ] {
            assert!(parse(&words(bad)).is_err(), "{bad}");
        }
    }

    struct Echo;
    impl Backend for Echo {
        fn call(&mut self, _call: Call, _text: Option<String>) -> Result<Value, CallError> {
            Ok(json!({"ok": true}))
        }
    }

    #[test]
    fn serving_answers_each_request_on_its_own_line() {
        let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n";
        let mut out = Vec::new();
        serve(input.as_bytes(), &mut out, &mut Echo).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert_eq!(out.lines().count(), 2, "{out}");
    }
}
