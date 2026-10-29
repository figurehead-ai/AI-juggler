//! What the bridge and the app say to each other over the socket: one
//! JSON line each way per call. Shared by both ends, so they can't drift.

use serde::{Deserialize, Serialize};

/// The program's name, in the app bundle's `Contents/MacOS` beside AI
/// Juggler (and in the process list, where the ledger knows it as an MCP
/// server owned by its session).
pub const PROGRAM: &str = "juggler-agent";

/// The session's key, in the environment of the session AI Juggler
/// starts (never in its arguments, never in a file).
pub const TOKEN_ENV: &str = "JUGGLER_AGENT_TOKEN";

/// The app's socket for this edition, in the same environment.
pub const SOCKET_ENV: &str = "JUGGLER_AGENT_SOCKET";

/// Where this program is, for a tool without MCP to call from its shell.
pub const PROGRAM_ENV: &str = "JUGGLER_AGENT";

/// AJ-E2306: the session's report key, for its tool's hooks only (it can
/// report the session's state and nothing else; never the agent tools'
/// key). In the environment of the session AI Juggler starts.
pub const REPORT_TOKEN_ENV: &str = "JUGGLER_REPORT_TOKEN";

/// The hook events `juggler-agent hook` reports (Claude Code's names).
pub const HOOK_EVENTS: [&str; 9] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "Notification",
    "Stop",
    "StopFailure",
    "SessionEnd",
];

/// The longest summary a report carries.
pub const SUMMARY_MAX: usize = 300;

/// The MCP server's name in a tool's settings (`mcp__juggler__whoami`).
pub const SERVER_NAME: &str = "juggler";

/// AI Juggler's own tools, all of them: the ones a launch pre-allows
/// (D-30) and nothing else. E18 adds messages and shared docs; E19 the
/// pool's tasks.
pub const TOOLS: [&str; 14] = [
    "whoami",
    "pool_roster",
    "set_status",
    "send_message",
    "read_inbox",
    "ack",
    "share_doc",
    "read_doc",
    "tasks_list",
    "task_get",
    "task_create",
    "task_claim",
    "task_update",
    "task_assign",
];

/// The resource with the caller's pool's context cards (AJ-E1703).
pub const CARDS_URI: &str = "juggler://pool/cards";

/// The longest status line an agent may set (a line, not a note).
pub const STATUS_MAX: usize = 140;

/// E18: the longest message body, in characters (the app's rule,
/// `juggler_core::messaging::BODY_MAX`; a test keeps them equal).
pub const BODY_MAX: usize = 4_000;

/// E18: the longest subject, in characters (`messaging::SUBJECT_MAX`).
pub const SUBJECT_MAX: usize = 120;

/// E18: the largest shared doc, in bytes (`messaging::DOC_MAX`).
pub const DOC_MAX: usize = 64 * 1024;

/// E19: the longest task title, in characters (`tasks::TITLE_MAX`).
pub const TASK_TITLE_MAX: usize = 120;

/// E19: the longest acceptance criteria, in characters
/// (`tasks::ACCEPTANCE_MAX`).
pub const ACCEPTANCE_MAX: usize = 2_000;

/// E19: the longest note with a task's change (`tasks::NOTE_MAX`).
pub const NOTE_MAX: usize = 500;

/// The longest request the app reads (a shared doc of 64 KiB, written
/// out as JSON, fits; everything else is a few hundred bytes).
pub const MAX_REQUEST: usize = 256 * 1024;

/// The longest answer the bridge reads.
pub const MAX_RESPONSE: usize = 512 * 1024;

/// The wire's version: a request of another is refused.
pub const VERSION: u32 = 1;

/// What an agent asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Call {
    /// Who am I: my session, my pool, my role and what the pool allows.
    Whoami,
    /// Who else is in my pool.
    PoolRoster,
    /// Set my own status line (my card's "says").
    SetStatus,
    /// My pool's context cards.
    Cards,
    /// E18: write to a member, the pool or the user.
    SendMessage,
    /// E18: my messages, one page (or one message in full).
    ReadInbox,
    /// E18: say I handled messages.
    Ack,
    /// E18: share a document with the pool.
    ShareDoc,
    /// E18: the pool's shared docs, or one of them.
    ReadDoc,
    /// E19: the pool's tasks, one page (ready work first).
    TasksList,
    /// E19: one task in full.
    TaskGet,
    /// E19: a new task (an epic or a story: the lead's).
    TaskCreate,
    /// E19: take a ready task.
    TaskClaim,
    /// E19: say where a task is, change it, or give it back.
    TaskUpdate,
    /// E19: the lead gives a task to a member, to no one, or asks the
    /// user for a new session for it.
    TaskAssign,
}

impl Call {
    /// The call a tool's name names, when it names one.
    #[must_use]
    pub fn of_tool(name: &str) -> Option<Self> {
        match name {
            "whoami" => Some(Self::Whoami),
            "pool_roster" => Some(Self::PoolRoster),
            "set_status" => Some(Self::SetStatus),
            "send_message" => Some(Self::SendMessage),
            "read_inbox" => Some(Self::ReadInbox),
            "ack" => Some(Self::Ack),
            "share_doc" => Some(Self::ShareDoc),
            "read_doc" => Some(Self::ReadDoc),
            "tasks_list" => Some(Self::TasksList),
            "task_get" => Some(Self::TaskGet),
            "task_create" => Some(Self::TaskCreate),
            "task_claim" => Some(Self::TaskClaim),
            "task_update" => Some(Self::TaskUpdate),
            "task_assign" => Some(Self::TaskAssign),
            _ => None,
        }
    }
}

/// E18: `send_message`'s arguments. `to` is a member's `session_id`
/// (from `pool_roster`), `pool` for everyone in the pool, or `user`; it
/// can name a member of another pool only when the user linked the two.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendArgs {
    /// Who it's for.
    pub to: String,
    /// Its kind (`note`, `question`, `answer`, `handoff`,
    /// `review_request`, `status`); a note when not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Its subject (a new thread's; a reply keeps its thread's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// What it says.
    pub body: String,
    /// The thread it belongs to (a new one when not said).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<String>,
    /// The message it answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
}

/// E18: `read_inbox`'s arguments.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxArgs {
    /// Read ones too (unread only when not said).
    #[serde(default)]
    pub all: bool,
    /// Only this thread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<String>,
    /// One message, in full.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// The `next` of the previous page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// E18: `share_doc`'s arguments.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocArgs {
    /// Its name (`api-schema.md`).
    pub name: String,
    /// Its text.
    pub content: String,
}

/// E18: `read_doc`'s arguments: none for the list, a doc's id for it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadDocArgs {
    /// The doc (the list when not said).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// Where to go on reading it, or the list's `next`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// E19: `tasks_list`'s arguments.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TasksListArgs {
    /// Only this session's (what it holds or was given).
    #[serde(default)]
    pub mine: bool,
    /// Only those ready to claim.
    #[serde(default)]
    pub ready: bool,
    /// Only those in this status (`todo`, `ready`, `in_progress`,
    /// `blocked`, `in_review`, `done`, `cancelled`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Only those under this epic or story.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The `next` of the previous page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// E19: `task_get`'s and `task_claim`'s argument: the task.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskArgs {
    /// Its short id (`aj-7f3c`).
    pub task: String,
}

/// E19: `task_create`'s arguments. Nothing names who makes it, its pool
/// or who it's given to: the caller makes it, in its own pool, for no one.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskCreateArgs {
    /// One line.
    pub title: String,
    /// What it asks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// When it's done.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<String>,
    /// `task` (the default), `story` or `epic`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The epic or story it belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// `urgent`, `high`, `normal` (the default) or `low`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    /// `ready` (the default) or `todo` (not ready to start).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// The tasks that must be done first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<String>,
    /// The issue it comes from (`https://…`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// E19: `task_update`'s arguments.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskUpdateArgs {
    /// The task.
    pub task: String,
    /// Its new status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Give it back to the pool (no one holds it; ready again).
    #[serde(default)]
    pub release: bool,
    /// A note for its history (a hand-off's: what's done, what's left).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// A new title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A new body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// New acceptance criteria.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<String>,
    /// A new priority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    /// More tasks that must be done first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<String>,
    /// The issue it comes from (`https://…`; empty removes it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// E19: `task_assign`'s arguments. `to` is a member's `session_id` (from
/// `pool_roster`), `nobody`, or `new_session`: a request the user sees,
/// to start a session for it on the account they pick (D-31).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskAssignArgs {
    /// The task.
    pub task: String,
    /// Who.
    pub to: String,
    /// What the new session should know (with `new_session`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// What a call carries besides what it is: at most one of these is used,
/// by the call it belongs to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Args {
    /// `set_status`'s line.
    pub text: Option<String>,
    /// `send_message`'s.
    pub send: Option<SendArgs>,
    /// `read_inbox`'s.
    pub inbox: Option<InboxArgs>,
    /// `ack`'s message ids.
    pub ack: Option<Vec<String>>,
    /// `share_doc`'s.
    pub doc: Option<DocArgs>,
    /// `read_doc`'s.
    pub read_doc: Option<ReadDocArgs>,
    /// E19: `tasks_list`'s.
    pub tasks: Option<TasksListArgs>,
    /// E19: `task_get`'s and `task_claim`'s.
    pub task: Option<TaskArgs>,
    /// E19: `task_create`'s.
    pub new_task: Option<TaskCreateArgs>,
    /// E19: `task_update`'s.
    pub task_change: Option<TaskUpdateArgs>,
    /// E19: `task_assign`'s.
    pub assign: Option<TaskAssignArgs>,
}

/// One call, bridge to app.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// [`VERSION`].
    pub v: u32,
    /// The session's key, from the bridge's own environment.
    pub token: String,
    /// What it asks for.
    pub call: Call,
    /// `set_status`'s line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// `send_message`'s arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send: Option<SendArgs>,
    /// `read_inbox`'s arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbox: Option<InboxArgs>,
    /// `ack`'s message ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ack: Option<Vec<String>>,
    /// `share_doc`'s arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<DocArgs>,
    /// `read_doc`'s arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_doc: Option<ReadDocArgs>,
    /// E19: `tasks_list`'s arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tasks: Option<TasksListArgs>,
    /// E19: `task_get`'s and `task_claim`'s argument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskArgs>,
    /// E19: `task_create`'s arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_task: Option<TaskCreateArgs>,
    /// E19: `task_update`'s arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_change: Option<TaskUpdateArgs>,
    /// E19: `task_assign`'s arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assign: Option<TaskAssignArgs>,
}

impl Request {
    /// A call with its arguments.
    #[must_use]
    pub fn new(token: String, call: Call, args: Args) -> Self {
        Self {
            v: VERSION,
            token,
            call,
            text: args.text,
            send: args.send,
            inbox: args.inbox,
            ack: args.ack,
            doc: args.doc,
            read_doc: args.read_doc,
            tasks: args.tasks,
            task: args.task,
            new_task: args.new_task,
            task_change: args.task_change,
            assign: args.assign,
        }
    }
}

/// AJ-E2306: what a tool's hook reports about its own session, from the
/// fields the tool hands its hooks (never anything else).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookReport {
    /// One of [`HOOK_EVENTS`].
    pub event: String,
    /// The tool's own id for its session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_session: Option<String>,
    /// Its transcript's path, as the tool gave it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
    /// The tool it uses or asks for (`Bash`, `AskUserQuestion`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// What it asks, in one bounded line (a command, a file, a question).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// A notification's type word (`permission_prompt`, `idle_prompt`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Why it stopped or ended, as a word (`rate_limit`, `prompt_input_exit`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One hook report, bridge to app (a shape of its own: never an agent's
/// call, and its key is the report key).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    /// [`VERSION`].
    pub v: u32,
    /// The session's report key, from the hook's own environment.
    pub token: String,
    /// What happened.
    pub hook: HookReport,
}

/// Why a call was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// Above the pool's level: the user decides (D-28). A message is held
    /// for the user's OK in Needs you (E18); anything else is a refusal
    /// that says so.
    Ask,
    /// Not at all: not in the pool, or the pool is gone.
    Deny,
    /// The user switched the agent tools off for this project or profile.
    Off,
    /// The key isn't a current session's (it ended, moved on, or AI
    /// Juggler restarted and the session wasn't continued).
    Unknown,
    /// Too many calls too fast, or a request too big.
    Limit,
    /// A request the app can't read.
    Bad,
}

/// The app's answer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Response {
    /// The call's result (`None` when refused).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Why not, when refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<Refusal>,
    /// The refusal in words, for the agent to pass on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Response {
    /// A result.
    #[must_use]
    pub fn ok(result: serde_json::Value) -> Self {
        Self {
            result: Some(result),
            refused: None,
            reason: None,
        }
    }

    /// A refusal, in words.
    #[must_use]
    pub fn refused(refusal: Refusal, reason: impl Into<String>) -> Self {
        Self {
            result: None,
            refused: Some(refusal),
            reason: Some(reason.into()),
        }
    }
}

/// A status line as it is kept: one line, no control characters, at most
/// [`STATUS_MAX`] characters (cut on a character, never mid-way).
#[must_use]
pub fn clean_status(text: &str) -> String {
    let line: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    line.chars().take(STATUS_MAX).collect()
}

#[cfg(test)]
mod tests {
    use super::{Call, Request, Response, STATUS_MAX, clean_status};

    #[test]
    fn a_request_round_trips_and_refuses_strangers() {
        let request = Request::new(
            "t".into(),
            Call::SetStatus,
            super::Args {
                text: Some("Fixing checkout".into()),
                ..super::Args::default()
            },
        );
        let line = serde_json::to_string(&request).unwrap();
        assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), request);
        assert!(
            serde_json::from_str::<Request>(
                r#"{"v":1,"token":"t","call":"whoami","pool":"other"}"#
            )
            .is_err(),
            "no field picks another pool or identity"
        );
        assert!(
            serde_json::from_str::<Request>(r#"{"v":1,"token":"t","call":"set_level"}"#).is_err()
        );
        let response = Response::refused(super::Refusal::Ask, "why");
        let line = serde_json::to_string(&response).unwrap();
        assert_eq!(serde_json::from_str::<Response>(&line).unwrap(), response);
    }

    #[test]
    fn only_the_tools_name_calls() {
        for tool in super::TOOLS {
            assert!(Call::of_tool(tool).is_some(), "{tool}");
        }
        assert_eq!(Call::of_tool("whoami"), Some(Call::Whoami));
        assert_eq!(Call::of_tool("send_message"), Some(Call::SendMessage));
        assert_eq!(Call::of_tool("cards"), None, "the cards are a resource");
        assert_eq!(Call::of_tool("set_level"), None);
        assert_eq!(Call::of_tool("deliver"), None, "only the user delivers");
    }

    #[test]
    fn a_message_names_its_recipient_and_nothing_else() {
        let send: super::SendArgs =
            serde_json::from_str(r#"{"to":"sess-2","body":"Start on the API","kind":"handoff"}"#)
                .unwrap();
        assert_eq!(send.to, "sess-2");
        for sneaky in [
            r#"{"to":"sess-2","body":"x","from":"user"}"#,
            r#"{"to":"sess-2","body":"x","hop":0}"#,
            r#"{"to":"sess-2","body":"x","deliver":true}"#,
            r#"{"to":"sess-2","body":"x","pool":"other"}"#,
        ] {
            assert!(
                serde_json::from_str::<super::SendArgs>(sneaky).is_err(),
                "{sneaky}"
            );
        }
    }

    #[test]
    fn a_task_call_names_its_task_and_nothing_else() {
        let create: super::TaskCreateArgs = serde_json::from_str(
            r#"{"title":"Build the API","kind":"story","blocked_by":["aj-1a2b"]}"#,
        )
        .unwrap();
        assert_eq!(create.blocked_by, ["aj-1a2b"]);
        for sneaky in [
            r#"{"title":"x","pool":"other"}"#,
            r#"{"title":"x","assignee":"sess-2"}"#,
            r#"{"title":"x","created_by":"user"}"#,
        ] {
            assert!(
                serde_json::from_str::<super::TaskCreateArgs>(sneaky).is_err(),
                "{sneaky}"
            );
        }
        for sneaky in [
            r#"{"task":"aj-1","to":"new_session","account":"Work"}"#,
            r#"{"task":"aj-1","to":"new_session","profile":"p-1"}"#,
            r#"{"task":"aj-1","to":"sess-2","start":true}"#,
        ] {
            assert!(
                serde_json::from_str::<super::TaskAssignArgs>(sneaky).is_err(),
                "no account, profile or start: {sneaky}"
            );
        }
        assert!(serde_json::from_str::<super::TaskArgs>(r#"{"task":"aj-1","as":"user"}"#).is_err());
        assert!(
            serde_json::from_str::<super::TaskUpdateArgs>(r#"{"task":"aj-1","assignee":"x"}"#)
                .is_err(),
            "assigning is task_assign's"
        );
    }

    #[test]
    fn a_status_is_one_short_line() {
        assert_eq!(
            clean_status("  Fixing\ncheckout\t now "),
            "Fixing checkout now"
        );
        assert_eq!(clean_status("\u{1b}[31mred"), "[31mred");
        let long = "é".repeat(STATUS_MAX + 9);
        assert_eq!(clean_status(&long).chars().count(), STATUS_MAX);
    }
}
