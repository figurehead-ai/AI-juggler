//! AJ-E2306: `juggler-agent hook <event>`, the command a tool's own hooks
//! run in the sessions AI Juggler starts (Claude Code's `PermissionRequest`,
//! `Notification`, `Stop`, …, from a per-launch `--settings` file).
//!
//! - It reads the JSON the tool hands its hooks on stdin (at most
//!   [`INPUT_MAX`]) and keeps only the fields of a [`HookReport`]: the
//!   event, the tool's session id and transcript path, the tool's name, a
//!   bounded summary of what it asks (a command, a file, a question), a
//!   notification's type word and a stop reason. Nothing else is kept or
//!   sent: no message text, no tool output, no environment.
//! - It sends one line to the app's socket with the session's **report
//!   key** (its own environment, [`REPORT_TOKEN_ENV`]) and waits at most
//!   [`WAIT`] for the answer.
//! - It always exits 0 and never decides anything: a `PermissionRequest`
//!   hook prints nothing, which leaves the tool's own dialog to the user,
//!   and it never holds the tool up.
//! - E18 (AJ-E1803): on `UserPromptSubmit` and `PostToolUse` the app may
//!   answer with a short facts-only note ("2 unread messages from
//!   Planner"); the hook prints it as the tool's `additionalContext` for
//!   those events ([`NOTE_EVENTS`]) and nothing else. Never a decision,
//!   never a "keep going", never a message's words.

use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use crate::client::send_line;
use crate::wire::{
    HOOK_EVENTS, HookReport, REPORT_TOKEN_ENV, Report, SOCKET_ENV, SUMMARY_MAX, VERSION,
};

/// The events whose hook may print a note (Claude Code adds a hook's
/// `additionalContext` to the model's context on these). AJ-E3013:
/// `SessionStart` too, so a desktop member learns of its unread messages
/// when it starts or resumes.
pub const NOTE_EVENTS: [&str; 3] = ["SessionStart", "UserPromptSubmit", "PostToolUse"];

/// The longest note printed.
pub const NOTE_MAX: usize = 300;

/// The most of the tool's input read.
pub const INPUT_MAX: usize = 256 * 1024;
/// How long the app has to answer.
pub const WAIT: Duration = Duration::from_secs(2);

fn text(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|s| !s.is_empty())
}

/// One bounded line: control characters as spaces, runs of space as one.
fn line(text: &str) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    flat.chars().take(SUMMARY_MAX).collect()
}

/// What a tool call asks, in one line: the first of its usual inputs.
fn summary_of(input: &Value) -> Option<String> {
    [
        "/command",
        "/file_path",
        "/path",
        "/url",
        "/pattern",
        "/questions/0/question",
        "/description",
    ]
    .iter()
    .find_map(|pointer| text(input, pointer))
    .map(|found| line(&found))
}

/// The report for `event` from the tool's hook input; `None` for an
/// event AI Juggler doesn't take or input that isn't JSON.
#[must_use]
pub fn report_of(event: &str, input: &str) -> Option<HookReport> {
    if !HOOK_EVENTS.contains(&event) {
        return None;
    }
    let value: Value = serde_json::from_str(input).ok()?;
    let tool_input = value.get("tool_input").unwrap_or(&Value::Null);
    let reason = match event {
        "StopFailure" => text(&value, "/error"),
        "SessionEnd" => text(&value, "/reason"),
        _ => None,
    };
    Some(HookReport {
        event: event.to_owned(),
        tool_session: text(&value, "/session_id"),
        transcript: text(&value, "/transcript_path"),
        tool: text(&value, "/tool_name").map(|tool| line(&tool)),
        summary: summary_of(tool_input),
        kind: text(&value, "/notification_type").map(|kind| line(&kind)),
        reason: reason.map(|reason| line(&reason)),
    })
}

/// The hook's input (at most [`INPUT_MAX`] bytes of `stdin`) as
/// `event`'s report.
fn read_report(event: &str, stdin: impl Read) -> Option<HookReport> {
    let mut input = String::new();
    stdin
        .take(u64::try_from(INPUT_MAX).unwrap_or(u64::MAX))
        .read_to_string(&mut input)
        .ok()?;
    report_of(event, &input)
}

/// Runs the hook: reads stdin, reports, and returns what to print (a
/// note's JSON on [`NOTE_EVENTS`], else nothing). The exit code is always
/// 0 (the caller's), whatever happened.
pub fn run(event: &str, stdin: impl Read) -> Option<String> {
    let hook = read_report(event, stdin)?;
    let token = std::env::var(REPORT_TOKEN_ENV).unwrap_or_default();
    let socket = std::env::var_os(SOCKET_ENV).map(PathBuf::from)?;
    if token.is_empty() {
        return None; // not a session AI Juggler started with reports on
    }
    let report = Report {
        v: VERSION,
        token,
        hook,
    };
    let line = serde_json::to_string(&report).ok()?;
    let answer = send_line(&socket, &line, WAIT).ok()?;
    note_output(event, &answer)
}

/// AJ-E3013: AI Juggler's connector's hook in a desktop app's session
/// (`hook <event> --connector`). In a session AI Juggler started (its key
/// or report key in the environment) it does nothing: the launch's own
/// hooks report. Elsewhere it sends the report with no key to the socket
/// the connector named; the app attests the caller from the system and,
/// on [`NOTE_EVENTS`], answers with the unread note (facts only). Always
/// silent when AI Juggler isn't running or the caller isn't attested.
pub fn run_connector(event: &str, stdin: impl Read) -> Option<String> {
    let keyed = [REPORT_TOKEN_ENV, crate::wire::TOKEN_ENV]
        .iter()
        .any(|name| std::env::var(name).is_ok_and(|value| !value.is_empty()));
    if keyed || !NOTE_EVENTS.contains(&event) {
        return None;
    }
    let socket = crate::client::connector_socket()?;
    connector_note(event, stdin, &socket)
}

/// [`run_connector`]'s exchange with the socket at `socket`.
fn connector_note(event: &str, stdin: impl Read, socket: &std::path::Path) -> Option<String> {
    let hook = read_report(event, stdin)?;
    let report = Report {
        v: VERSION,
        token: String::new(),
        hook: HookReport {
            // Only what names the event: nothing a tool asked or ran.
            event: hook.event,
            tool_session: hook.tool_session,
            ..HookReport::default()
        },
    };
    let line = serde_json::to_string(&report).ok()?;
    let answer = send_line(socket, &line, WAIT).ok()?;
    note_output(event, &answer)
}

/// What a hook prints for the app's answer: the note as the tool's
/// `additionalContext`, on [`NOTE_EVENTS`] only, one bounded line.
#[must_use]
pub fn note_output(event: &str, answer: &str) -> Option<String> {
    if !NOTE_EVENTS.contains(&event) {
        return None;
    }
    let response: crate::wire::Response = serde_json::from_str(answer.trim()).ok()?;
    let note = response.result?.get("note")?.as_str().map(|note| {
        note.chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .take(NOTE_MAX)
            .collect::<String>()
    })?;
    if note.trim().is_empty() {
        return None;
    }
    serde_json::to_string(&serde_json::json!({
        "hookSpecificOutput": {"hookEventName": event, "additionalContext": note}
    }))
    .ok()
}

#[cfg(test)]
mod tests {
    use super::report_of;

    #[test]
    fn the_connector_hook_is_silent_without_ai_juggler() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("bridge.sock");
        let input =
            r#"{"session_id":"s","hook_event_name":"UserPromptSubmit","prompt":"secret plan"}"#;
        assert_eq!(
            super::connector_note("UserPromptSubmit", input.as_bytes(), &missing),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_connector_hook_sends_no_key_and_no_words_and_prints_the_note() {
        use std::io::{BufRead, Write};
        let root = tempfile::tempdir().unwrap();
        let (socket, listener) = crate::test_support::agent_socket(root.path());
        let app = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            std::io::BufReader::new(&stream)
                .read_line(&mut line)
                .unwrap();
            let mut stream = stream;
            stream
                .write_all(
                    b"{\"result\":{\"note\":\"AI Juggler: 1 unread message from Planner.\"}}\n",
                )
                .unwrap();
            line
        });
        let input = r#"{"session_id":"s","hook_event_name":"UserPromptSubmit","prompt":"secret plan","tool_input":{"command":"rm -rf x"}}"#;
        let printed = super::connector_note("UserPromptSubmit", input.as_bytes(), &socket).unwrap();
        let sent: crate::wire::Report = serde_json::from_str(app.join().unwrap().trim()).unwrap();
        assert_eq!(sent.token, "", "the connector carries no key");
        assert_eq!(sent.hook.tool_session.as_deref(), Some("s"));
        assert_eq!((sent.hook.summary, sent.hook.tool), (None, None));
        let value: serde_json::Value = serde_json::from_str(&printed).unwrap();
        assert_eq!(
            value["hookSpecificOutput"]["additionalContext"],
            "AI Juggler: 1 unread message from Planner."
        );
    }

    #[test]
    fn only_the_reported_fields_are_kept() {
        let input = r#"{"session_id":"abc","transcript_path":"/h/p/abc.jsonl","cwd":"/w",
            "hook_event_name":"PermissionRequest","permission_mode":"default",
            "tool_name":"Bash","tool_input":{"command":"rm -rf build\nand more","description":"clean"},
            "permission_suggestions":[{"type":"addRules"}]}"#;
        let report = report_of("PermissionRequest", input).unwrap();
        assert_eq!(report.tool_session.as_deref(), Some("abc"));
        assert_eq!(report.tool.as_deref(), Some("Bash"));
        assert_eq!(report.summary.as_deref(), Some("rm -rf build and more"));
        assert_eq!(report.transcript.as_deref(), Some("/h/p/abc.jsonl"));
        let json = serde_json::to_string(&report).unwrap();
        assert!(!json.contains("permission_suggestions") && !json.contains("/w\""));
    }

    #[test]
    fn a_note_is_printed_only_where_it_adds_context_and_never_decides() {
        let answer = r#"{"result":{"note":"AI Juggler: 2 unread messages from Planner."}}"#;
        let printed = super::note_output("UserPromptSubmit", answer).unwrap();
        let value: serde_json::Value = serde_json::from_str(&printed).unwrap();
        assert_eq!(
            value["hookSpecificOutput"]["additionalContext"],
            "AI Juggler: 2 unread messages from Planner."
        );
        assert_eq!(
            value["hookSpecificOutput"]["hookEventName"],
            "UserPromptSubmit"
        );
        assert!(value.get("decision").is_none() && value.get("continue").is_none());
        for event in ["PermissionRequest", "Stop", "PreToolUse", "Notification"] {
            assert_eq!(super::note_output(event, answer), None, "{event}");
        }
        assert_eq!(super::note_output("PostToolUse", r#"{"result":{}}"#), None);
        assert_eq!(super::note_output("PostToolUse", "not json"), None);
    }

    #[test]
    fn notifications_carry_their_type_word_never_their_message() {
        let input = r#"{"session_id":"abc","hook_event_name":"Notification",
            "message":"Claude needs your permission to use Bash","notification_type":"permission_prompt"}"#;
        let report = report_of("Notification", input).unwrap();
        assert_eq!(report.kind.as_deref(), Some("permission_prompt"));
        assert!(
            !serde_json::to_string(&report)
                .unwrap()
                .contains("needs your permission")
        );
    }

    #[test]
    fn questions_stops_and_strangers() {
        let ask = r#"{"session_id":"s","tool_name":"AskUserQuestion",
            "tool_input":{"questions":[{"question":"Which database?","header":"DB","options":[]}]}}"#;
        assert_eq!(
            report_of("PreToolUse", ask).unwrap().summary.as_deref(),
            Some("Which database?")
        );
        let failed = r#"{"session_id":"s","error":"rate_limit","error_details":"x"}"#;
        assert_eq!(
            report_of("StopFailure", failed).unwrap().reason.as_deref(),
            Some("rate_limit")
        );
        let long = format!(r#"{{"tool_input":{{"command":"{}"}}}}"#, "x".repeat(900));
        assert_eq!(
            report_of("PreToolUse", &long)
                .unwrap()
                .summary
                .unwrap()
                .len(),
            super::SUMMARY_MAX
        );
        assert!(report_of("SomethingNew", "{}").is_none());
        assert!(report_of("Stop", "not json").is_none());
    }
}
