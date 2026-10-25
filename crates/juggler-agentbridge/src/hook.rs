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
//! - It prints nothing and always exits 0, so it never decides a
//!   permission (a `PermissionRequest` hook that prints nothing leaves the
//!   tool's own dialog to the user) and never holds the tool up.

use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use crate::client::send_line;
use crate::wire::{
    HOOK_EVENTS, HookReport, REPORT_TOKEN_ENV, Report, SOCKET_ENV, SUMMARY_MAX, VERSION,
};

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

/// Runs the hook: reads stdin, reports, prints nothing. The exit code is
/// always 0 (the caller's), whatever happened.
pub fn run(event: &str, stdin: impl Read) {
    let mut input = String::new();
    if stdin
        .take(u64::try_from(INPUT_MAX).unwrap_or(u64::MAX))
        .read_to_string(&mut input)
        .is_err()
    {
        return;
    }
    let Some(hook) = report_of(event, &input) else {
        return;
    };
    let token = std::env::var(REPORT_TOKEN_ENV).unwrap_or_default();
    let Some(socket) = std::env::var_os(SOCKET_ENV).map(PathBuf::from) else {
        return;
    };
    if token.is_empty() {
        return; // not a session AI Juggler started with reports on
    }
    let report = Report {
        v: VERSION,
        token,
        hook,
    };
    if let Ok(line) = serde_json::to_string(&report) {
        let _ = send_line(&socket, &line, WAIT);
    }
}

#[cfg(test)]
mod tests {
    use super::report_of;

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
