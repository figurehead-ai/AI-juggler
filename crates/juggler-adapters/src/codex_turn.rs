//! Whether a Codex thread is working, from its rollout's tail: Codex
//! records no status for a thread (neither the app nor a `codex` in a
//! terminal), so the last turn's markers say it.
//!
//! The markers, as Codex writes them (`event_msg` lines, by
//! `payload.type`; seen in the fixtures and on a real home, 0.1.5 check):
//! - `task_started` opens a turn, `task_complete` or `turn_aborted` ends
//!   it. A turn can run for many minutes with nothing written while a
//!   command runs: an open turn is working however quiet the file is.
//! - Since 0.158 every turn's items end in `item_completed`: a tail with
//!   those and no marker is the middle of a turn longer than the tail.
//! - An approval or a question waits on the user: `exec_approval_request`,
//!   `apply_patch_approval_request`, `request_user_input`,
//!   `elicitation_request` (the protocol's `EventMsg` names). Codex keeps
//!   them out of its rollouts today, so this rarely says "waiting"; when
//!   one is written, anything the turn does after it answers it.
//! - An older rollout with no markers: the last of a `turn_context` or a
//!   user message after the last assistant message is a running turn.
//!
//! Only `type` fields and roles are read, never text.

use juggler_adapter::live::TailTurn;

/// The events that wait on the user's answer.
const WAITS: &[&str] = &[
    "exec_approval_request",
    "apply_patch_approval_request",
    "request_user_input",
    "elicitation_request",
];

/// What a rollout's tail (whole lines) says of its last turn; `None` when
/// it says nothing (no turn in it).
#[must_use]
pub fn tail_turn(tail: &str) -> Option<TailTurn> {
    // The last marker's word: open or ended.
    let mut marked: Option<bool> = None;
    // The middle of a turn whose start is before the tail.
    let mut items = false;
    // An older rollout's last turn: open or ended.
    let mut legacy: Option<bool> = None;
    let mut waiting = false;
    for line in tail.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let kind = value.get("type").and_then(serde_json::Value::as_str);
        let item = value
            .pointer("/payload/type")
            .and_then(serde_json::Value::as_str);
        match (kind, item) {
            (Some("event_msg"), Some("task_started")) => {
                marked = Some(true);
                waiting = false;
            }
            (Some("event_msg"), Some("task_complete" | "turn_aborted")) => {
                marked = Some(false);
                waiting = false;
            }
            (Some("event_msg"), Some(event)) if WAITS.contains(&event) => waiting = true,
            (Some("event_msg"), Some("item_completed")) => {
                items = true;
                waiting = false;
            }
            (Some("turn_context"), _) => legacy = Some(true),
            (Some("response_item"), Some("message")) => {
                match value
                    .pointer("/payload/role")
                    .and_then(serde_json::Value::as_str)
                {
                    Some("user") => legacy = Some(true),
                    Some("assistant") => {
                        legacy = Some(false);
                        waiting = false;
                    }
                    _ => {}
                }
            }
            (Some("response_item"), _) => waiting = false,
            // Counts and limits (`token_count`, `token_usage_record`),
            // written any time, and the rest: not the turn moving on.
            _ => {}
        }
    }
    let open = marked.or(items.then_some(true)).or(legacy)?;
    Some(match (open, waiting) {
        (true, true) => TailTurn::Waiting,
        (true, false) => TailTurn::Running,
        (false, _) => TailTurn::Ended,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/tests/fixtures/codex/{name}.jsonl",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn a_turn_running_a_command_is_working_however_quiet() {
        assert_eq!(tail_turn(&fixture("turn-working")), Some(TailTurn::Running));
    }

    #[test]
    fn the_middle_of_a_turn_longer_than_the_tail_is_working() {
        // No marker in the tail: only items the open turn completed.
        assert_eq!(tail_turn(&fixture("turn-long")), Some(TailTurn::Running));
    }

    #[test]
    fn a_turn_waiting_on_an_approval_asks_you() {
        assert_eq!(tail_turn(&fixture("turn-asks")), Some(TailTurn::Waiting));
        // Once the turn moves on, the approval was answered.
        let answered = format!(
            "{}{}\n",
            fixture("turn-asks"),
            r#"{"timestamp":"2026-10-25T09:00:30.000Z","type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"c2","output":"ok"}}"#
        );
        assert_eq!(tail_turn(&answered), Some(TailTurn::Running));
    }

    #[test]
    fn a_finished_or_stopped_turn_is_idle() {
        assert_eq!(tail_turn(&fixture("turn-finished")), Some(TailTurn::Ended));
        let aborted = fixture("turn-working")
            + r#"{"timestamp":"2026-10-25T09:20:00.000Z","type":"event_msg","payload":{"type":"turn_aborted","turn_id":"t2","reason":"interrupted"}}"#
            + "\n";
        assert_eq!(tail_turn(&aborted), Some(TailTurn::Ended));
    }

    #[test]
    fn an_older_rollouts_turn_is_told_by_who_spoke_last() {
        assert_eq!(tail_turn(&fixture("turn-idle")), Some(TailTurn::Ended));
        assert_eq!(tail_turn(&fixture("rollout-legacy")), Some(TailTurn::Ended));
        let asked = fixture("turn-idle")
            + r#"{"timestamp":"2026-10-25T09:05:00.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"And the docs?"}]}}"#
            + "\n";
        assert_eq!(tail_turn(&asked), Some(TailTurn::Running));
    }

    #[test]
    fn a_tail_with_no_turn_says_nothing() {
        assert_eq!(tail_turn(""), None);
        assert_eq!(
            tail_turn(
                r#"{"type":"session_meta","payload":{"id":"x"}}
not json
{"type":"event_msg","payload":{"type":"token_count"}}"#
            ),
            None
        );
    }
}
