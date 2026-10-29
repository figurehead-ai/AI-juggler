//! The Claude Code subagent reader's tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use juggler_adapter::subagents::{SubagentText, SubagentTokens, TokensCounted};

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn start(call: &str, kind: &str, task: &str, at: &str) -> String {
    serde_json::json!({
        "type": "assistant", "timestamp": at,
        "message": {"role": "assistant", "content": [{"type": "tool_use", "id": call, "name": "Agent",
            "input": {"description": task, "prompt": "do it", "subagent_type": kind}}]},
    })
    .to_string()
}

fn launched(call: &str, agent: &str, status: &str, at: &str) -> String {
    serde_json::json!({
        "type": "user", "timestamp": at,
        "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": call, "content": "ok"}]},
        "toolUseResult": {"status": status, "agentId": agent},
    })
    .to_string()
}

fn notified(agent: &str, status: &str, at: &str) -> String {
    serde_json::json!({
        "type": "user", "timestamp": at, "origin": {"kind": "task-notification"},
        "message": {"role": "user", "content": format!(
            "<task-notification>\n<task-id>{agent}</task-id>\n<status>{status}</status>\n<summary>s</summary>\n</task-notification>")},
    })
    .to_string()
}

fn meta(dir: &Path, agent: &str, value: &serde_json::Value) {
    write(
        &dir.join(format!("agent-{agent}.meta.json")),
        &value.to_string(),
    );
}

fn own(dir: &Path, agent: &str, lines: &[String]) {
    let first = serde_json::json!({"type": "user", "isSidechain": true, "timestamp": "2026-10-27T10:00:05Z",
        "message": {"role": "user", "content": "the task"}})
    .to_string();
    let mut all = vec![first];
    all.extend_from_slice(lines);
    write(
        &dir.join(format!("agent-{agent}.jsonl")),
        &(all.join("\n") + "\n"),
    );
}

fn by_id<'a>(list: &'a [Subagent], id: &str) -> &'a Subagent {
    list.iter()
        .find(|agent| agent.id == id)
        .expect("a subagent the fixture holds is listed")
}

#[test]
fn subagents_are_read_from_their_files_and_the_session_says_how_each_ended() {
    let dir = tempfile::tempdir().unwrap();
    let session = dir.path().join("proj/s1.jsonl");
    let folder = dir.path().join("proj/s1/subagents");
    write(
        &session,
        &([
            start("t1", "Explore", "Find the parser", "2026-10-27T10:00:00Z"),
            launched("t1", "a1", "completed", "2026-10-27T10:01:00Z"),
            start("t2", "worker", "Fix the test", "2026-10-27T10:02:00Z"),
            launched("t2", "a2", "async_launched", "2026-10-27T10:02:01Z"),
            start("t3", "worker", "Break it", "2026-10-27T10:03:00Z"),
            launched("t3", "a3", "async_launched", "2026-10-27T10:03:01Z"),
            start("t4", "worker", "Still going", "2026-10-27T10:04:00Z"),
            launched("t4", "a4", "async_launched", "2026-10-27T10:04:01Z"),
            notified("a2", "completed", "2026-10-27T10:09:00Z"),
            notified("a3", "failed", "2026-10-27T10:10:00Z"),
        ]
        .join("\n")
            + "\n"),
    );
    for (agent, call, shape) in [
        ("a1", "t1", "foreground"),
        ("a2", "t2", "background"),
        ("a3", "t3", "background"),
        ("a4", "t4", "background"),
    ] {
        meta(
            &folder,
            agent,
            &serde_json::json!({"agentType": "frontend-worker", "description": format!("task of {agent}"),
                "toolUseId": call, "spawnDepth": 1, "requestShape": shape, "requestNonInteractive": true}),
        );
        own(&folder, agent, &[]);
    }
    // A nested one, started by a4; how it stands is in a4's transcript.
    meta(
        &folder,
        "n1",
        &serde_json::json!({"agentType": "Explore", "description": "Look deeper", "toolUseId": "t9",
            "spawnDepth": 2, "parentAgentId": "a4", "requestShape": "foreground"}),
    );
    own(&folder, "n1", &[]);
    own(
        &folder,
        "a4",
        &[
            start("t9", "Explore", "Look deeper", "2026-10-27T10:05:00Z"),
            launched("t9", "n1", "completed", "2026-10-27T10:06:00Z"),
        ],
    );
    // Files with no meta that nothing names: unknown.
    own(&folder, "orphan", &[]);

    let mut reader = ClaudeSubagents::new(&session);
    reader.refresh().unwrap();
    let list = reader.subagents();
    assert_eq!(list.len(), 6);
    let a1 = by_id(&list, "a1");
    assert_eq!(a1.status, SubagentStatus::Done);
    assert_eq!(a1.kind.as_deref(), Some("frontend-worker"));
    assert_eq!(a1.task.as_deref(), Some("task of a1"));
    assert_eq!(a1.background, Some(false));
    assert_eq!(a1.depth, 1);
    assert_eq!(a1.parent, None);
    assert_eq!(
        a1.started_ms,
        juggler_core::time::timestamp_ms("2026-10-27T10:00:00Z")
    );
    assert_eq!(
        a1.ended_ms,
        juggler_core::time::timestamp_ms("2026-10-27T10:01:00Z")
    );
    assert!(a1.transcript.as_ref().unwrap().ends_with("agent-a1.jsonl"));
    let a2 = by_id(&list, "a2");
    assert_eq!(a2.status, SubagentStatus::Done);
    assert_eq!(a2.background, Some(true));
    assert_eq!(
        a2.ended_ms,
        juggler_core::time::timestamp_ms("2026-10-27T10:09:00Z")
    );
    assert_eq!(by_id(&list, "a3").status, SubagentStatus::Failed);
    let a4 = by_id(&list, "a4");
    assert_eq!(a4.status, SubagentStatus::Running);
    assert_eq!(a4.ended_ms, None);
    let n1 = by_id(&list, "n1");
    assert_eq!(n1.status, SubagentStatus::Done);
    assert_eq!(n1.parent.as_deref(), Some("a4"));
    assert_eq!(n1.depth, 2);
    let orphan = by_id(&list, "orphan");
    assert_eq!(orphan.status, SubagentStatus::Unknown);
    assert_eq!(orphan.kind, None);
    assert_eq!(orphan.task, None);
    assert_eq!(
        orphan.started_ms,
        juggler_core::time::timestamp_ms("2026-10-27T10:00:05Z")
    );
}

#[test]
fn a_later_notification_is_read_on_the_next_refresh_and_stopped_counts_as_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let session = dir.path().join("s.jsonl");
    let folder = dir.path().join("s/subagents");
    let head = [
        start("t1", "worker", "One", "2026-10-27T10:00:00Z"),
        launched("t1", "a1", "async_launched", "2026-10-27T10:00:01Z"),
    ]
    .join("\n")
        + "\n";
    write(&session, &head);
    meta(
        &folder,
        "a1",
        &serde_json::json!({"toolUseId": "t1", "requestShape": "background"}),
    );
    let mut reader = ClaudeSubagents::new(&session);
    reader.refresh().unwrap();
    assert_eq!(reader.subagents()[0].status, SubagentStatus::Running);
    // Queued for the session first: that is when it ended.
    let queued = serde_json::json!({"type": "queue-operation", "operation": "enqueue", "timestamp": "2026-10-27T10:05:00Z",
        "content": "<task-notification><task-id>a1</task-id><status>killed</status></task-notification>"})
    .to_string();
    write(&session, &format!("{head}{queued}\n"));
    reader.refresh().unwrap();
    let one = &reader.subagents()[0];
    assert_eq!(one.status, SubagentStatus::Stopped);
    assert_eq!(
        one.ended_ms,
        juggler_core::time::timestamp_ms("2026-10-27T10:05:00Z")
    );
    assert_eq!(one.transcript, None, "no file of its own yet");
}

#[test]
fn a_tool_description_that_mentions_the_tag_ends_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let session = dir.path().join("s.jsonl");
    let snapshot = serde_json::json!({"type": "attachment", "attachment": {"type": "prompt_snapshot",
        "tools": [{"description": "<task-notification><task-id>a1</task-id><status>completed</status></task-notification>"}]}})
    .to_string();
    let pasted = serde_json::json!({"type": "user", "message": {"role": "user",
        "content": "<task-notification><task-id>a1</task-id><status>completed</status></task-notification>"}})
    .to_string();
    write(
        &session,
        &[
            start("t1", "worker", "One", "2026-10-27T10:00:00Z"),
            launched("t1", "a1", "async_launched", "2026-10-27T10:00:01Z"),
            snapshot,
            pasted,
        ]
        .join("\n"),
    );
    let mut reader = ClaudeSubagents::new(&session);
    reader.refresh().unwrap();
    let list = reader.subagents();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].status, SubagentStatus::Running);
    assert_eq!(list[0].id, "a1");
    assert_eq!(list[0].background, Some(true));
}

#[test]
fn a_session_with_none_lists_none_and_a_missing_transcript_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let session = dir.path().join("s.jsonl");
    write(
        &session,
        "{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}\n",
    );
    let mut reader = ClaudeSubagents::new(&session);
    reader.refresh().unwrap();
    assert!(reader.subagents().is_empty());
    let mut gone = ClaudeSubagents::new(&dir.path().join("gone.jsonl"));
    assert!(gone.refresh().is_err());
    assert!(reader_of(&dir.path().join("x.json")).is_none());
}

fn reader_of(path: &Path) -> Option<Box<dyn SubagentReader>> {
    reader(SubagentScope { path, id: "x" })
}

// AJ-E1520 (0.3): what each subagent's records add.

fn asked(text: &str, uuid: &str, at: &str) -> String {
    serde_json::json!({"type": "user", "uuid": uuid, "timestamp": at, "origin": {"kind": "human"},
        "message": {"role": "user", "content": text}})
    .to_string()
}

fn started(call: &str, task: &str, prompt: &str, at: &str) -> String {
    serde_json::json!({
        "type": "assistant", "timestamp": at,
        "message": {"role": "assistant", "content": [{"type": "tool_use", "id": call, "name": "Agent",
            "input": {"description": task, "prompt": prompt, "subagent_type": "Explore"}}]},
    })
    .to_string()
}

fn answered(call: &str, agent: &str, text: &str, at: &str) -> String {
    serde_json::json!({
        "type": "user", "timestamp": at,
        "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": call,
            "content": [{"type": "text", "text": text}]}]},
        "toolUseResult": {"status": "completed", "agentId": agent, "resolvedModel": "claude-haiku-4-5",
            "totalTokens": 900, "totalToolUseCount": 5, "totalDurationMs": 32_000,
            "toolStats": {"readCount": 2, "searchCount": 1, "bashCount": 1, "editFileCount": 1,
                "otherToolCount": 0, "linesAdded": 12, "linesRemoved": 3}},
    })
    .to_string()
}

fn reply(id: &str, model: &str, usage: [u64; 4], tools: &[serde_json::Value], at: &str) -> String {
    serde_json::json!({
        "type": "assistant", "timestamp": at, "requestId": format!("req-{id}"),
        "message": {"id": id, "role": "assistant", "model": model,
            "usage": {"input_tokens": usage[0], "output_tokens": usage[1],
                "cache_read_input_tokens": usage[2], "cache_creation_input_tokens": usage[3]},
            "content": tools},
    })
    .to_string()
}

fn tool(id: &str, name: &str, input: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({"type": "tool_use", "id": id, "name": name, "input": input})
}

/// A session whose one turn started a foreground subagent that read,
/// edited, wrote and ran a command, and a background one still running.
fn busy_session(dir: &Path) -> PathBuf {
    let session = dir.join("proj/s1.jsonl");
    let folder = dir.join("proj/s1/subagents");
    write(
        &session,
        &([
            asked(
                "fix the totals\nand add tests",
                "u1",
                "2026-10-27T10:00:00Z",
            ),
            reply(
                "m0",
                "claude-opus-4-1",
                [10, 20, 30, 40],
                &[],
                "2026-10-27T10:00:01Z",
            ),
            started(
                "t1",
                "Audit the cart",
                "Find every place the cart rounds.",
                "2026-10-27T10:00:02Z",
            ),
            answered(
                "t1",
                "a1",
                "## Found\nThree places.",
                "2026-10-27T10:01:00Z",
            ),
            started("t2", "Run the suite", "Run it all.", "2026-10-27T10:02:00Z"),
            launched("t2", "a2", "async_launched", "2026-10-27T10:02:01Z"),
        ]
        .join("\n")
            + "\n"),
    );
    meta(
        &folder,
        "a1",
        &serde_json::json!({"agentType": "Explore", "description": "Audit the cart", "toolUseId": "t1",
            "spawnDepth": 1, "requestShape": "foreground", "worktreePath": "/w/a1", "worktreeBranch": "wt-a1"}),
    );
    meta(
        &folder,
        "a2",
        &serde_json::json!({"agentType": "general-purpose", "description": "Run the suite", "toolUseId": "t2",
            "spawnDepth": 1, "requestShape": "background"}),
    );
    // a1's own transcript: one response written as two lines (same id,
    // the same usage), then a second; a request record that must be
    // skipped; an edit's patch.
    let read = tool(
        "c1",
        "Read",
        &serde_json::json!({"file_path": "/p/cart.ts"}),
    );
    let edit = tool(
        "c2",
        "Edit",
        &serde_json::json!({"file_path": "/p/cart.ts", "old_string": "a", "new_string": "b"}),
    );
    let write_ = tool(
        "c3",
        "Write",
        &serde_json::json!({"file_path": "/p/tax.ts", "content": "x\ny"}),
    );
    let bash = tool(
        "c4",
        "Bash",
        &serde_json::json!({"command": "npm run dev -- --port 5173", "run_in_background": true}),
    );
    let blob = serde_json::json!({"type": "api-request-blob", "message": {"role": "assistant", "model": "x",
        "usage": {"input_tokens": 99_999}, "content": [tool("z", "Bash", &serde_json::json!({"command": "never"}))]}})
    .to_string();
    let patch = serde_json::json!({"type": "user", "timestamp": "2026-10-27T10:00:40Z",
        "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "c2", "content": "ok"}]},
        "toolUseResult": {"structuredPatch": [{"lines": [" a", "-b", "+c", "+d"]}]}})
    .to_string();
    own(
        &folder,
        "a1",
        &[
            reply(
                "m1",
                "claude-haiku-4-5",
                [100, 10, 1000, 50],
                &[read],
                "2026-10-27T10:00:10Z",
            ),
            reply(
                "m1",
                "claude-haiku-4-5",
                [100, 10, 1000, 50],
                &[edit],
                "2026-10-27T10:00:11Z",
            ),
            blob,
            patch,
            reply(
                "m2",
                "claude-haiku-4-5",
                [5, 7, 2000, 0],
                &[write_, bash],
                "2026-10-27T10:00:50Z",
            ),
        ],
    );
    own(&folder, "a2", &[]);
    session
}

#[test]
fn each_subagent_says_what_it_was_given_used_and_did() {
    let dir = tempfile::tempdir().unwrap();
    let session = busy_session(dir.path());
    let mut reader = ClaudeSubagents::new(&session);
    reader.refresh().unwrap();
    assert!(reader.refresh_details(u64::MAX).unwrap());
    let list = reader.subagents();
    let a1 = by_id(&list, "a1");
    let d = &a1.details;
    assert_eq!(
        d.prompt.as_deref(),
        Some("Find every place the cart rounds.")
    );
    assert_eq!(d.result.as_deref(), Some("## Found\nThree places."));
    assert_eq!(d.model.as_deref(), Some("claude-haiku-4-5"));
    let tokens = d.tokens.expect("its tokens");
    assert_eq!(
        (
            tokens.input,
            tokens.output,
            tokens.cache_read,
            tokens.cache_write
        ),
        (105, 17, 3000, 50),
        "each response once, the request record skipped"
    );
    let tools = d.tools.expect("its tools");
    assert_eq!(
        (tools.total, tools.reads, tools.edits),
        (5, 2, 1),
        "Claude Code's own count wins"
    );
    assert_eq!(d.lines.map(|l| (l.added, l.removed)), Some((12, 3)));
    assert_eq!(
        d.files,
        vec!["/p/cart.ts".to_owned(), "/p/tax.ts".to_owned()]
    );
    assert_eq!(d.files_total, 2);
    assert_eq!(d.commands.len(), 1);
    assert_eq!(d.commands[0].command, "npm run dev -- --port 5173");
    assert!(d.commands[0].background);
    assert_eq!(
        d.commands[0].at_ms,
        juggler_core::time::timestamp_ms("2026-10-27T10:00:50Z")
    );
    let asked = d.asked.as_ref().expect("the turn's message");
    assert_eq!(asked.text, "fix the totals");
    assert_eq!(asked.key, "u1");
    assert_eq!(d.worktree.as_deref(), Some("/w/a1"));
    assert_eq!(d.branch.as_deref(), Some("wt-a1"));
    assert_eq!(d.duration_ms, Some(32_000));
    assert_eq!(
        d.last_activity_ms,
        juggler_core::time::timestamp_ms("2026-10-27T10:00:50Z")
    );
    let a2 = by_id(&list, "a2");
    assert_eq!(a2.status, SubagentStatus::Running);
    assert_eq!(
        a2.details.asked.as_ref().map(|a| a.key.as_str()),
        Some("u1")
    );
    assert_eq!(a2.details.result, None);
    assert_eq!(reader.running(), 1);
    assert_eq!(reader.tokens_counted(), TokensCounted::InSession);
    assert_eq!(
        reader.session_tokens().map(SubagentTokens::total),
        Some(100)
    );
}

#[test]
fn derived_counts_stand_in_where_claude_code_reported_none() {
    let dir = tempfile::tempdir().unwrap();
    let session = busy_session(dir.path());
    // a2 ends with a notification: no tool stats, a result and its usage.
    let note = serde_json::json!({"type": "user", "timestamp": "2026-10-27T10:09:00Z", "origin": {"kind": "task-notification"},
        "message": {"role": "user", "content": "<task-notification>\n<task-id>a2</task-id>\n<tool-use-id>t2</tool-use-id>\n<status>failed</status>\n<summary>Suite failed</summary>\n<result>3 tests failed: rounding\nmore</result>\n<usage><subagent_tokens>4200</subagent_tokens><tool_uses>9</tool_uses><duration_ms>61000</duration_ms></usage>\n</task-notification>"}})
    .to_string();
    let mut text = std::fs::read_to_string(&session).unwrap();
    text.push_str(&note);
    text.push('\n');
    write(&session, &text);
    let folder = dir.path().join("proj/s1/subagents");
    let bash = tool("b1", "Bash", &serde_json::json!({"command": "cargo test"}));
    let grep = tool("b2", "Grep", &serde_json::json!({"pattern": "x"}));
    own(
        &folder,
        "a2",
        &[reply(
            "n1",
            "claude-sonnet-4-5",
            [1, 2, 3, 4],
            &[bash, grep],
            "2026-10-27T10:03:00Z",
        )],
    );
    let mut reader = ClaudeSubagents::new(&session);
    reader.refresh().unwrap();
    reader.refresh_details(u64::MAX).unwrap();
    let list = reader.subagents();
    let a2 = by_id(&list, "a2");
    assert_eq!(a2.status, SubagentStatus::Failed);
    assert_eq!(
        a2.details.failure.as_deref(),
        Some("3 tests failed: rounding")
    );
    assert_eq!(a2.details.result, None);
    assert_eq!(a2.details.duration_ms, Some(61_000));
    let tools = a2.details.tools.expect("counted from its transcript");
    assert_eq!((tools.total, tools.commands, tools.searches), (2, 1, 1));
    assert_eq!(a2.details.lines, None, "no edits: no lines said");
    assert_eq!(a2.details.model.as_deref(), Some("claude-sonnet-4-5"));
}

#[test]
fn a_checkpoint_resumes_where_it_left_off_with_its_words_filtered() {
    let dir = tempfile::tempdir().unwrap();
    let session = busy_session(dir.path());
    let mut reader = ClaudeSubagents::new(&session);
    reader.refresh().unwrap();
    reader.refresh_details(u64::MAX).unwrap();
    let filter = |kind: SubagentText, text: &str| match kind {
        SubagentText::Result => None,
        _ => Some(text.replace("cart", "[x]")),
    };
    let saved = reader.checkpoint(&filter).expect("a checkpoint");
    let mut again = ClaudeSubagents::new(&session);
    assert!(again.resume(&saved));
    // Nothing grew: nothing to read, and the same list (filtered).
    assert!(again.refresh().unwrap());
    assert!(
        again.refresh_details(0).unwrap(),
        "caught up without reading"
    );
    let list = again.subagents();
    let a1 = by_id(&list, "a1");
    assert_eq!(
        a1.details.prompt.as_deref(),
        Some("Find every place the [x] rounds.")
    );
    assert_eq!(a1.task.as_deref(), Some("Audit the [x]"));
    assert_eq!(a1.details.result, None, "a dropped word isn't saved");
    assert_eq!(a1.details.tokens.map(SubagentTokens::total), Some(3172));
    assert_eq!(
        a1.details.files,
        vec!["/p/[x].ts".to_owned(), "/p/tax.ts".to_owned()]
    );
    // Another session's checkpoint is refused.
    let mut other = ClaudeSubagents::new(&dir.path().join("proj/other.jsonl"));
    assert!(!other.resume(&saved));
    assert!(!other.resume(b"not json"));
    // What grows after a resume is read on.
    let folder = dir.path().join("proj/s1/subagents");
    let path = folder.join("agent-a1.jsonl");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str(&reply(
        "m3",
        "claude-haiku-4-5",
        [1, 1, 1, 1],
        &[],
        "2026-10-27T10:05:00Z",
    ));
    text.push('\n');
    std::fs::write(&path, text).unwrap();
    again.refresh().unwrap();
    again.refresh_details(u64::MAX).unwrap();
    assert_eq!(
        by_id(&again.subagents(), "a1")
            .details
            .tokens
            .map(SubagentTokens::total),
        Some(3176)
    );
}

#[test]
fn a_half_written_line_is_not_saved_and_is_read_whole_after_a_resume() {
    let dir = tempfile::tempdir().unwrap();
    let session = busy_session(dir.path());
    let path = dir.path().join("proj/s1/subagents/agent-a2.jsonl");
    let line = reply(
        "h1",
        "claude-opus-4-1",
        [7, 0, 0, 0],
        &[],
        "2026-10-27T10:04:00Z",
    );
    let (head, rest) = line.split_at(30);
    let base = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{base}{head}")).unwrap();
    let mut reader = ClaudeSubagents::new(&session);
    reader.refresh().unwrap();
    reader.refresh_details(u64::MAX).unwrap();
    let saved = reader
        .checkpoint(&|_, text: &str| Some(text.to_owned()))
        .unwrap();
    std::fs::write(&path, format!("{base}{head}{rest}\n")).unwrap();
    let mut again = ClaudeSubagents::new(&session);
    assert!(again.resume(&saved));
    again.refresh().unwrap();
    again.refresh_details(u64::MAX).unwrap();
    assert_eq!(
        by_id(&again.subagents(), "a2")
            .details
            .tokens
            .map(SubagentTokens::total),
        Some(7)
    );
}

#[test]
fn details_are_read_within_the_budget_asked_for() {
    let dir = tempfile::tempdir().unwrap();
    let session = busy_session(dir.path());
    let mut reader = ClaudeSubagents::new(&session);
    reader.refresh().unwrap();
    assert!(
        !reader.refresh_details(100).unwrap(),
        "100 bytes don't cover them"
    );
    let mut rounds = 0;
    while !reader.refresh_details(100).unwrap() {
        rounds += 1;
        assert!(rounds < 1_000, "it gets there");
    }
    assert_eq!(
        by_id(&reader.subagents(), "a1")
            .details
            .tokens
            .map(SubagentTokens::total),
        Some(3172)
    );
}
