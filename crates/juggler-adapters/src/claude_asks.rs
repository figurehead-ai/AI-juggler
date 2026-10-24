//! AJ-E2107: what an open Claude Code session waits on, from its
//! transcript's tail, for versions whose session file doesn't record it
//! (newer ones write `status: "waiting"`, read in [`crate::claude_live`]).
//!
//! The tail is at most the transcript's last 64 KB. Of each line only the
//! line's `type`, `isSidechain`, and each content part's `type`, `id`,
//! `name` and `tool_use_id` are kept: never message text or a tool's
//! input (serde skips them without keeping them). A tool call whose result
//! hasn't been written yet is pending; the user's question tools make it
//! a question. Sub-agents' lines (`isSidechain`) are left out: their calls
//! are the parent's business.

use juggler_adapter::live::Ask;

/// The tools that put a question to the user and wait for the answer.
const QUESTION_TOOLS: &[&str] = &["AskUserQuestion", "ExitPlanMode"];

/// One transcript line, the fields kept.
#[derive(serde::Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(rename = "isSidechain", default)]
    sidechain: bool,
    message: Option<Message>,
}

#[derive(serde::Deserialize)]
struct Message {
    #[serde(default)]
    content: Content,
}

/// A message's content: its parts, or anything else (a plain prompt's
/// text, which is never kept).
#[derive(serde::Deserialize, Default)]
#[serde(untagged)]
enum Content {
    Parts(Vec<Part>),
    #[default]
    #[serde(skip)]
    Nothing,
    Other(serde::de::IgnoredAny),
}

/// One content part, the fields kept.
#[derive(serde::Deserialize)]
struct Part {
    #[serde(rename = "type")]
    kind: Option<String>,
    id: Option<String>,
    name: Option<String>,
    tool_use_id: Option<String>,
}

/// What the tail says the session waits on: a question tool or another
/// tool call with no result yet; `None` when nothing is pending (or the
/// last thing is the user's own prompt).
#[must_use]
pub fn pending_ask(tail: &str) -> Option<Ask> {
    // Pending calls in order: (id, tool).
    let mut pending: Vec<(String, String)> = Vec::new();
    for raw in tail.lines() {
        let Ok(line) = serde_json::from_str::<Line>(raw) else {
            continue; // the cut first line, or another shape
        };
        if line.sidechain {
            continue;
        }
        let Some(message) = line.message else {
            continue;
        };
        match (line.kind.as_deref(), message.content) {
            (Some("assistant"), Content::Parts(parts)) => {
                for part in parts {
                    if part.kind.as_deref() == Some("tool_use")
                        && let Some(id) = part.id
                    {
                        pending.push((id, part.name.unwrap_or_default()));
                    }
                }
            }
            (Some("user"), Content::Parts(parts)) => {
                let mut prompt = false;
                for part in parts {
                    match part.kind.as_deref() {
                        Some("tool_result") => {
                            if let Some(id) = part.tool_use_id {
                                pending.retain(|(known, _)| *known != id);
                            }
                        }
                        Some("text") => prompt = true,
                        _ => {}
                    }
                }
                if prompt {
                    pending.clear();
                }
            }
            // A plain prompt: whatever was pending was answered or dropped.
            (Some("user"), Content::Other(_)) => pending.clear(),
            _ => {}
        }
    }
    if pending.is_empty() {
        None
    } else if pending
        .iter()
        .any(|(_, tool)| QUESTION_TOOLS.contains(&tool.as_str()))
    {
        Some(Ask::Question)
    } else {
        Some(Ask::Pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assistant_call(id: &str, tool: &str) -> String {
        format!(
            r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"secret words"}},{{"type":"tool_use","id":"{id}","name":"{tool}","input":{{"command":"ls"}}}}]}}}}"#
        )
    }

    fn result(id: &str) -> String {
        format!(
            r#"{{"type":"user","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"{id}","content":"ok"}}]}}}}"#
        )
    }

    #[test]
    fn a_question_tool_with_no_answer_is_a_question() {
        let tail = [assistant_call("t1", "AskUserQuestion")].join("\n");
        assert_eq!(pending_ask(&tail), Some(Ask::Question));
    }

    #[test]
    fn a_tool_call_with_no_result_is_pending() {
        let tail = [
            assistant_call("t1", "Read"),
            result("t1"),
            assistant_call("t2", "Bash"),
        ]
        .join("\n");
        assert_eq!(pending_ask(&tail), Some(Ask::Pending));
    }

    #[test]
    fn answered_calls_and_a_new_prompt_leave_nothing_pending() {
        let tail = [assistant_call("t1", "Bash"), result("t1")].join("\n");
        assert_eq!(pending_ask(&tail), None);
        let prompt = r#"{"type":"user","message":{"role":"user","content":"do the next thing"}}"#;
        let tail = [assistant_call("t1", "Bash"), prompt.to_owned()].join("\n");
        assert_eq!(pending_ask(&tail), None);
        let parts = r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"go on"}]}}"#;
        let tail = [assistant_call("t1", "AskUserQuestion"), parts.to_owned()].join("\n");
        assert_eq!(pending_ask(&tail), None);
    }

    #[test]
    fn sub_agents_calls_and_cut_or_odd_lines_are_left_out() {
        let side = r#"{"type":"assistant","isSidechain":true,"message":{"content":[{"type":"tool_use","id":"s1","name":"Bash","input":{}}]}}"#;
        let tail = [
            "ol\":\"cut line\"}".to_owned(),
            side.to_owned(),
            "[]".to_owned(),
            "not json".to_owned(),
        ]
        .join("\n");
        assert_eq!(pending_ask(&tail), None);
        assert_eq!(pending_ask(""), None);
    }
}
