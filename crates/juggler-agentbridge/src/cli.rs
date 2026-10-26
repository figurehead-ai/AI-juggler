//! The program's two faces: the MCP server (`--mcp`), and a plain
//! command line for tools without MCP, called from their shell. There is
//! no option for the key: it comes from the environment only, so it is
//! never in a process's arguments.

use std::io::{BufRead, Write};

use serde_json::Value;

use crate::client::{CallError, Target, call};
use crate::mcp::{Backend, handle_line};
use crate::wire::{
    Args, Call, InboxArgs, ReadDocArgs, Refusal, SendArgs, TaskArgs, TaskAssignArgs,
    TaskCreateArgs, TaskUpdateArgs, TasksListArgs, clean_status,
};

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
    fn call(&mut self, what: Call, args: Args) -> Result<Value, CallError> {
        call(self.target.as_ref().map_err(Clone::clone)?, what, args)
    }
}

/// The help text.
pub const USAGE: &str = "juggler-agent: AI Juggler's pool tools for the agent in this session.

  juggler-agent whoami          this session, its pool, its role and what the pool allows
  juggler-agent roster          the other sessions in the pool
  juggler-agent cards           the pool's context cards
  juggler-agent status <text>   set this session's one-line status (empty clears it)
  juggler-agent send <to> <text>
                                write to a member (its session_id), \"pool\" or \"user\"
  juggler-agent reply <thread> <to> <text>
                                answer in a thread
  juggler-agent inbox [--all]   this session's messages (unread only without --all)
  juggler-agent read <message>  one message in full
  juggler-agent ack <message>…  mark messages handled
  juggler-agent share <name>    share the text on stdin with the pool
  juggler-agent docs            the pool's shared documents
  juggler-agent doc <id>        one shared document
  juggler-agent tasks [--mine | --ready]
                                the pool's tasks, ready work first
  juggler-agent task <id>       one task in full
  juggler-agent task-create <title>
                                add a task to the pool (ready, no one's)
  juggler-agent claim <id>      take a ready task for this session
  juggler-agent task-update <id> <status> [note]
                                say where a task is (in_review when it's ready for a look)
  juggler-agent release <id> [note]
                                give a task back to the pool, with a note
  juggler-agent assign <id> <to>
                                (the lead) give a task to a member, \"nobody\" or \"new_session\"
  juggler-agent --mcp           serve the same as an MCP server on stdin and stdout
  juggler-agent --mcp --plugin  the same, as the Claude Code plugin's server
  juggler-agent hook <event>    (run by the tool's own hooks) report this session's state

Works in sessions AI Juggler started with its agent tools on. What the pool
lets agents do is set by the user, in AI Juggler.";

/// What the command line asked for.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// `--mcp`.
    Mcp,
    /// E19: `--mcp --plugin` (the Claude Code plugin's server).
    McpPlugin,
    /// `--help`.
    Help,
    /// `--version`.
    Version,
    /// One call.
    Call(Call, Box<Args>),
    /// E18: share the text on stdin as a doc called this.
    Share(String),
    /// AJ-E2306: a tool's hook reporting its session's state
    /// (`hook <event>`, the tool's JSON on stdin).
    Hook(String),
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
        ["--mcp", "--plugin"] => Ok(Command::McpPlugin),
        [] | ["--help" | "-h" | "help"] => Ok(Command::Help),
        ["--version" | "-V"] => Ok(Command::Version),
        ["hook", event] => Ok(Command::Hook((*event).to_owned())),
        ["whoami"] => Ok(call_of(Call::Whoami, Args::default())),
        ["roster" | "pool_roster"] => Ok(call_of(Call::PoolRoster, Args::default())),
        ["cards"] => Ok(call_of(Call::Cards, Args::default())),
        ["status" | "set_status", text @ ..] => Ok(call_of(
            Call::SetStatus,
            Args {
                text: Some(clean_status(&text.join(" "))),
                ..Args::default()
            },
        )),
        ["send", to, text @ ..] if !text.is_empty() => Ok(send(to, None, text)),
        ["reply", thread, to, text @ ..] if !text.is_empty() => Ok(send(to, Some(thread), text)),
        ["inbox"] | ["inbox", "--all"] => Ok(call_of(
            Call::ReadInbox,
            Args {
                inbox: Some(InboxArgs {
                    all: words.len() == 2,
                    ..InboxArgs::default()
                }),
                ..Args::default()
            },
        )),
        ["read", message] => Ok(call_of(
            Call::ReadInbox,
            Args {
                inbox: Some(InboxArgs {
                    message: Some((*message).to_owned()),
                    all: true,
                    ..InboxArgs::default()
                }),
                ..Args::default()
            },
        )),
        ["ack", ids @ ..] if !ids.is_empty() => Ok(call_of(
            Call::Ack,
            Args {
                ack: Some(ids.iter().map(|id| (*id).to_owned()).collect()),
                ..Args::default()
            },
        )),
        ["share", name] => Ok(Command::Share((*name).to_owned())),
        ["docs"] => Ok(call_of(
            Call::ReadDoc,
            Args {
                read_doc: Some(ReadDocArgs::default()),
                ..Args::default()
            },
        )),
        ["doc", id] => Ok(call_of(
            Call::ReadDoc,
            Args {
                read_doc: Some(ReadDocArgs {
                    doc: Some((*id).to_owned()),
                    cursor: None,
                }),
                ..Args::default()
            },
        )),
        _ => task_command(words.as_slice())
            .ok_or_else(|| format!("{USAGE}\n\nNot a command: {}", words.join(" "))),
    }
}

/// E19: the task commands.
fn task_command(words: &[&str]) -> Option<Command> {
    let task = |id: &str| TaskArgs {
        task: id.to_owned(),
    };
    let note = |rest: &[&str]| (!rest.is_empty()).then(|| rest.join(" "));
    let args = match words {
        ["tasks"] | ["tasks", "--mine" | "--ready"] => (
            Call::TasksList,
            Args {
                tasks: Some(TasksListArgs {
                    mine: words.get(1) == Some(&"--mine"),
                    ready: words.get(1) == Some(&"--ready"),
                    ..TasksListArgs::default()
                }),
                ..Args::default()
            },
        ),
        ["task", id] => (
            Call::TaskGet,
            Args {
                task: Some(task(id)),
                ..Args::default()
            },
        ),
        ["claim", id] => (
            Call::TaskClaim,
            Args {
                task: Some(task(id)),
                ..Args::default()
            },
        ),
        ["task-create", title @ ..] if !title.is_empty() => (
            Call::TaskCreate,
            Args {
                new_task: Some(TaskCreateArgs {
                    title: title.join(" "),
                    ..TaskCreateArgs::default()
                }),
                ..Args::default()
            },
        ),
        ["task-update", id, status, rest @ ..] => (
            Call::TaskUpdate,
            Args {
                task_change: Some(TaskUpdateArgs {
                    task: (*id).to_owned(),
                    status: Some((*status).to_owned()),
                    note: note(rest),
                    ..TaskUpdateArgs::default()
                }),
                ..Args::default()
            },
        ),
        ["release", id, rest @ ..] => (
            Call::TaskUpdate,
            Args {
                task_change: Some(TaskUpdateArgs {
                    task: (*id).to_owned(),
                    release: true,
                    note: note(rest),
                    ..TaskUpdateArgs::default()
                }),
                ..Args::default()
            },
        ),
        ["assign", id, to] => (
            Call::TaskAssign,
            Args {
                assign: Some(TaskAssignArgs {
                    task: (*id).to_owned(),
                    to: (*to).to_owned(),
                    note: None,
                }),
                ..Args::default()
            },
        ),
        _ => return None,
    };
    Some(call_of(args.0, args.1))
}

/// One call with its arguments.
fn call_of(what: Call, args: Args) -> Command {
    Command::Call(what, Box::new(args))
}

fn send(to: &str, thread: Option<&str>, text: &[&str]) -> Command {
    call_of(
        Call::SendMessage,
        Args {
            send: Some(SendArgs {
                to: to.to_owned(),
                body: text.join(" "),
                thread: thread.map(str::to_owned),
                ..SendArgs::default()
            }),
            ..Args::default()
        },
    )
}

/// Serves MCP until the tool closes stdin (the session ended).
///
/// # Errors
///
/// When stdin or stdout fails.
pub fn serve(
    input: impl BufRead,
    output: impl Write,
    backend: &mut dyn Backend,
) -> std::io::Result<()> {
    serve_as(input, output, backend, None)
}

/// [`serve`], as the Claude Code plugin's server when `plugin` says
/// whether the launch already gave this session its tools (E19).
///
/// # Errors
///
/// When stdin or stdout fails.
pub fn serve_as(
    input: impl BufRead,
    mut output: impl Write,
    backend: &mut dyn Backend,
    plugin: Option<bool>,
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        let reply = match plugin {
            Some(given) => crate::plugin::handle_line(&line, backend, given),
            None => handle_line(&line, backend),
        };
        if let Some(reply) = reply {
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
    use super::{Command, call_of, parse, serve};
    use crate::client::CallError;
    use crate::mcp::Backend;
    use crate::wire::{Args, Call};
    use serde_json::{Value, json};

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn the_command_line_has_no_way_to_pass_a_key() {
        assert_eq!(parse(&words("--mcp")), Ok(Command::Mcp));
        assert_eq!(
            parse(&words("whoami")),
            Ok(call_of(Call::Whoami, Args::default()))
        );
        assert_eq!(
            parse(&words("status Fixing   the checkout")),
            Ok(call_of(
                Call::SetStatus,
                Args {
                    text: Some("Fixing the checkout".into()),
                    ..Args::default()
                }
            ))
        );
        let Ok(Command::Call(Call::SendMessage, args)) =
            parse(&words("send sess-2 Start on the API"))
        else {
            panic!("send");
        };
        let send = args.send.unwrap();
        assert_eq!(
            (send.to.as_str(), send.body.as_str()),
            ("sess-2", "Start on the API")
        );
        assert_eq!(
            parse(&words("share schema.md")),
            Ok(Command::Share("schema.md".into()))
        );
        assert!(parse(&words("send sess-2")).is_err(), "nothing to say");
        let Ok(Command::Call(Call::TaskUpdate, change)) =
            parse(&words("release aj-7f3c the schema moved"))
        else {
            panic!("release");
        };
        let change = change.task_change.unwrap();
        assert!(change.release);
        assert_eq!(change.note.as_deref(), Some("the schema moved"));
        assert!(matches!(
            parse(&words("claim aj-7f3c")),
            Ok(Command::Call(Call::TaskClaim, _))
        ));
        assert!(parse(&words("assign aj-7f3c new_session --account Work")).is_err());
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
        fn call(&mut self, _call: Call, _args: Args) -> Result<Value, CallError> {
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
