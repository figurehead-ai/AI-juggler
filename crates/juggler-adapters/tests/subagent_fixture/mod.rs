//! Synthetic subagent records for the conformance kit's check 11
//! (AJ-E1520, 0.2), one per tool, as each tool writes them (the layouts
//! its adapter's module documents). Every session started three
//! subagents — one done, one failed, one still running — and the running
//! one started one more, done. No real record was copied; every value is
//! made up.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use rusqlite::Connection;

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn lines(values: &[serde_json::Value]) -> String {
    values.iter().map(|value| format!("{value}\n")).collect()
}

/// The first file under `dir` that `tail` names: its folders (`/`
/// between them, compared by component, so any platform's separator
/// works) and the end of its file name.
pub fn find(dir: &Path, tail: &str) -> Option<PathBuf> {
    let mut parts: Vec<&str> = tail.split('/').filter(|part| !part.is_empty()).collect();
    let name = parts.pop()?;
    let matches = |path: &Path| {
        let file = path
            .file_name()
            .and_then(|file| file.to_str())
            .unwrap_or("");
        let mut folders = path.parent().into_iter().flat_map(Path::components).rev();
        file.ends_with(name)
            && parts.iter().rev().all(|part| {
                folders
                    .next()
                    .is_some_and(|folder| folder.as_os_str() == std::ffi::OsStr::new(part))
            })
    };
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if matches(&path) {
                return Some(path);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Claude Code: `projects/<project>/<id>.jsonl` and `<id>/subagents/`.

pub fn claude(state_dir: &Path, cwd: &Path) -> (PathBuf, String) {
    let id = "0b1c2d3e-0000-4000-8000-0000000000aa".to_owned();
    let project = state_dir.join("projects/-kit-project");
    let session = project.join(format!("{id}.jsonl"));
    let folder = project.join(&id).join("subagents");
    let call = |call: &str, at: &str| {
        serde_json::json!({"type": "assistant", "sessionId": id, "timestamp": at,
            "message": {"role": "assistant", "content": [{"type": "tool_use", "id": call, "name": "Agent",
                "input": {"description": format!("Task {call}"), "prompt": "p", "subagent_type": "worker"}}]}})
    };
    let answer = |call: &str, agent: &str, status: &str, at: &str| {
        serde_json::json!({"type": "user", "sessionId": id, "timestamp": at,
            "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": call, "content": "ok"}]},
            "toolUseResult": {"status": status, "agentId": agent}})
    };
    write(
        &session,
        &lines(&[
            serde_json::json!({"type": "user", "sessionId": id, "cwd": cwd, "timestamp": "2026-10-21T10:00:00Z",
                "message": {"role": "user", "content": "hello"}}),
            call("t1", "2026-10-21T10:00:01Z"),
            answer("t1", "a1", "completed", "2026-10-21T10:00:09Z"),
            call("t2", "2026-10-21T10:00:10Z"),
            answer("t2", "a2", "async_launched", "2026-10-21T10:00:10Z"),
            call("t3", "2026-10-21T10:00:11Z"),
            answer("t3", "a3", "async_launched", "2026-10-21T10:00:11Z"),
            serde_json::json!({"type": "user", "sessionId": id, "timestamp": "2026-10-21T10:00:20Z",
                "origin": {"kind": "task-notification"},
                "message": {"role": "user", "content":
                    "<task-notification><task-id>a2</task-id><status>failed</status></task-notification>"}}),
        ]),
    );
    for (agent, started_by, shape, depth, parent) in [
        ("a1", "t1", "foreground", 1, None),
        ("a2", "t2", "background", 1, None),
        ("a3", "t3", "background", 1, None),
        ("n1", "x1", "foreground", 2, Some("a3")),
    ] {
        let mut meta = serde_json::json!({"agentType": "worker", "description": format!("Task {started_by}"),
            "toolUseId": started_by, "spawnDepth": depth, "requestShape": shape, "requestNonInteractive": true});
        if let Some(parent) = parent {
            meta["parentAgentId"] = serde_json::json!(parent);
        }
        write(
            &folder.join(format!("agent-{agent}.meta.json")),
            &meta.to_string(),
        );
        let mut own = vec![
            serde_json::json!({"type": "user", "isSidechain": true, "agentId": agent,
            "timestamp": "2026-10-21T10:00:12Z", "message": {"role": "user", "content": "the task"}}),
        ];
        if agent == "a3" {
            own.push(call("x1", "2026-10-21T10:00:13Z"));
            own.push(answer("x1", "n1", "completed", "2026-10-21T10:00:15Z"));
        }
        write(&folder.join(format!("agent-{agent}.jsonl")), &lines(&own));
    }
    (session, id)
}

// ---------------------------------------------------------------------------
// Codex: child threads' rollouts beside the parent's, named by their
// `session_meta`; the parent's rollout says how each ended (v2 shape).

fn rollout(state_dir: &Path, thread: &str, values: &[serde_json::Value]) -> PathBuf {
    let path = state_dir.join(format!(
        "sessions/2026/10/21/rollout-2026-10-21T11-00-00-{thread}.jsonl"
    ));
    write(&path, &lines(values));
    path
}

fn child_meta(thread: &str, parent: &str, depth: u32, agent_path: &str) -> serde_json::Value {
    serde_json::json!({"timestamp": "2026-10-21T11:00:02Z", "type": "session_meta", "payload": {
        "id": thread, "timestamp": "2026-10-21T11:00:02Z", "agent_path": agent_path,
        "source": {"subagent": {"thread_spawn": {"parent_thread_id": parent, "depth": depth,
            "agent_path": agent_path, "agent_nickname": "Kit", "agent_role": "explorer"}}}}})
}

fn spawn(call: &str, at: &str) -> serde_json::Value {
    serde_json::json!({"timestamp": at, "type": "response_item", "payload": {"type": "function_call",
        "name": "spawn_agent", "call_id": call,
        "arguments": serde_json::json!({"message": format!("Task {call}"), "agent_type": "explorer"}).to_string()}})
}

fn output(call: &str, value: &serde_json::Value, at: &str) -> serde_json::Value {
    serde_json::json!({"timestamp": at, "type": "response_item", "payload": {"type": "function_call_output",
        "call_id": call, "output": value.to_string()}})
}

fn activity(path: &str, thread: &str, kind: &str, at: &str) -> serde_json::Value {
    serde_json::json!({"timestamp": at, "type": "event_msg", "payload": {"type": "item_completed",
        "item": {"type": "SubAgentActivity", "agent_path": path, "agent_thread_id": thread, "kind": kind}}})
}

pub fn codex(state_dir: &Path, cwd: &Path) -> (PathBuf, String) {
    let parent = "0199a2b3-0000-7000-8000-0000000000a0";
    let (a, b, c, g) = (
        "0199a2b3-0000-7000-8000-0000000000a1",
        "0199a2b3-0000-7000-8000-0000000000a2",
        "0199a2b3-0000-7000-8000-0000000000a3",
        "0199a2b3-0000-7000-8000-0000000000a4",
    );
    let session = rollout(
        state_dir,
        parent,
        &[
            serde_json::json!({"timestamp": "2026-10-21T11:00:00Z", "type": "session_meta",
                "payload": {"id": parent, "cwd": cwd, "originator": "codex_cli_rs"}}),
            spawn("c1", "2026-10-21T11:00:01Z"),
            output(
                "c1",
                &serde_json::json!({"task_name": "/root/a"}),
                "2026-10-21T11:00:01Z",
            ),
            spawn("c2", "2026-10-21T11:00:02Z"),
            output(
                "c2",
                &serde_json::json!({"task_name": "/root/b"}),
                "2026-10-21T11:00:02Z",
            ),
            spawn("c3", "2026-10-21T11:00:03Z"),
            output(
                "c3",
                &serde_json::json!({"task_name": "/root/c"}),
                "2026-10-21T11:00:03Z",
            ),
            activity("/root/a", a, "completed", "2026-10-21T11:00:05Z"),
            activity("/root/c", c, "started", "2026-10-21T11:00:05Z"),
            serde_json::json!({"timestamp": "2026-10-21T11:00:06Z", "type": "response_item", "payload": {
                "type": "function_call", "name": "list_agents", "call_id": "l1", "arguments": "{}"}}),
            output(
                "l1",
                &serde_json::json!({"agents": [
                {"agent_name": "/root/b", "agent_status": {"errored": "boom"}},
                {"agent_name": "/root/c", "agent_status": "running"}]}),
                "2026-10-21T11:00:06Z",
            ),
        ],
    );
    rollout(state_dir, a, &[child_meta(a, parent, 1, "/root/a")]);
    rollout(state_dir, b, &[child_meta(b, parent, 1, "/root/b")]);
    rollout(
        state_dir,
        c,
        &[
            child_meta(c, parent, 1, "/root/c"),
            spawn("g1", "2026-10-21T11:00:07Z"),
            output(
                "g1",
                &serde_json::json!({"task_name": "/root/c/g"}),
                "2026-10-21T11:00:07Z",
            ),
            activity("/root/c/g", g, "completed", "2026-10-21T11:00:09Z"),
        ],
    );
    rollout(state_dir, g, &[child_meta(g, c, 2, "/root/c/g")]);
    (session, parent.to_owned())
}

// ---------------------------------------------------------------------------
// Kimi Code: `state.json`'s agents map and the `subagent.*` records in each
// spawning agent's `agents/<id>/wire.jsonl`.

pub fn kimi(state_dir: &Path, cwd: &Path) -> (PathBuf, String) {
    let id = "kit-kimi-subagents".to_owned();
    let state = crate::kimi_fixture::add_session(state_dir, &id, cwd, "kit session");
    let folder = state.parent().unwrap().to_path_buf();
    write(
        &state,
        &serde_json::json!({"title": "kit session", "lastPrompt": "kit session", "agents": {
            "main": {"type": "main"},
            "agent-0": {"type": "sub", "parentAgentId": "main", "labels": {"parentAgentId": "main", "profileName": "explore"}},
            "agent-1": {"type": "sub", "parentAgentId": "main", "labels": {"parentAgentId": "main", "profileName": "coder"}},
            "agent-2": {"type": "sub", "parentAgentId": "main", "labels": {"parentAgentId": "main", "profileName": "coder"}},
            "agent-3": {"type": "sub", "parentAgentId": "main", "labels": {"parentAgentId": "agent-2", "profileName": "explore"}},
        }})
        .to_string(),
    );
    let record = |kind: &str, agent: &str, time: i64| {
        serde_json::json!({"type": kind, "subagentId": agent, "subagentName": "coder",
            "description": format!("Task {agent}"), "runInBackground": false, "time": time})
    };
    write(
        &folder.join("agents/main/wire.jsonl"),
        &lines(&[
            serde_json::json!({"type": "metadata", "protocol_version": "1.5"}),
            record("subagent.spawned", "agent-0", 1_000),
            record("subagent.completed", "agent-0", 2_000),
            record("subagent.spawned", "agent-1", 3_000),
            record("subagent.failed", "agent-1", 4_000),
            record("subagent.spawned", "agent-2", 5_000),
            record("subagent.started", "agent-2", 5_001),
        ]),
    );
    write(
        &folder.join("agents/agent-2/wire.jsonl"),
        &lines(&[
            record("subagent.spawned", "agent-3", 6_000),
            record("subagent.completed", "agent-3", 7_000),
        ]),
    );
    for agent in ["agent-0", "agent-1", "agent-3"] {
        write(&folder.join("agents").join(agent).join("wire.jsonl"), "");
    }
    (state, id)
}

// ---------------------------------------------------------------------------
// Grok Build: `subagents/<id>/meta.json` in the session's folder, and in
// the child session's for a nested one.

pub fn grok(state_dir: &Path, cwd: &Path) -> (PathBuf, String) {
    let id = "0199aa00-0000-7000-8000-0000000000a0".to_owned();
    let summary = crate::grok_fixture::add_session(state_dir, &id, cwd, "kit session");
    let folder = summary.parent().unwrap().to_path_buf();
    let meta = |folder: &Path, agent: &str, status: &str| {
        let mut value = serde_json::json!({"subagent_id": agent, "child_session_id": agent,
            "parent_session_id": "x", "subagent_type": "general", "description": format!("Task {agent}"),
            "prompt": "p", "status": status, "started_at": "2026-10-21T12:00:00Z", "context_normalized": false});
        if status != "running" {
            value["completed_at"] = serde_json::json!("2026-10-21T12:01:00Z");
        }
        write(
            &folder.join("subagents").join(agent).join("meta.json"),
            &value.to_string(),
        );
    };
    meta(&folder, "0199aa00-0000-7000-8000-0000000000a1", "completed");
    meta(&folder, "0199aa00-0000-7000-8000-0000000000a2", "failed");
    let running = "0199aa00-0000-7000-8000-0000000000a3";
    meta(&folder, running, "running");
    // The running one's own session (another cwd's group) started one.
    let child =
        crate::grok_fixture::add_session(state_dir, running, &cwd.join("worktree"), "child");
    meta(
        child.parent().unwrap(),
        "0199aa00-0000-7000-8000-0000000000a4",
        "completed",
    );
    (summary, id)
}

// ---------------------------------------------------------------------------
// OpenCode: child sessions by `parent_id`; 1.x says how each stands in its
// parent's `task` tool part, 2.x in the child's `idle_outcome`.

pub fn opencode(db: &Path, line: crate::opencode_fixture::Line, cwd: &Path) -> (PathBuf, String) {
    use crate::opencode_fixture::{Line, add_session, create, now_ms};
    let conn: Connection = create(db, line);
    let now = now_ms();
    let parent = "ses_kitparent";
    add_session(&conn, line, parent, cwd, None, now);
    for (child, of) in [
        ("ses_kita", parent),
        ("ses_kitb", parent),
        ("ses_kitc", parent),
        ("ses_kitd", "ses_kitc"),
    ] {
        add_session(&conn, line, child, cwd, Some(of), now + 1);
    }
    match line {
        Line::V1 => {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS part (id TEXT PRIMARY KEY, message_id TEXT NOT NULL,
                   session_id TEXT NOT NULL, time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL,
                   data TEXT NOT NULL);",
            )
            .unwrap();
            for (part, session, child, status) in [
                ("prt_a", parent, "ses_kita", "completed"),
                ("prt_b", parent, "ses_kitb", "error"),
                ("prt_c", parent, "ses_kitc", "running"),
                ("prt_d", "ses_kitc", "ses_kitd", "completed"),
            ] {
                let mut state = serde_json::json!({"status": status,
                    "input": {"description": format!("Task {child}"), "prompt": "p", "subagent_type": "explore"},
                    "metadata": {"sessionId": child, "parentSessionId": session},
                    "time": {"start": now + 1}});
                if status != "running" {
                    state["time"]["end"] = serde_json::json!(now + 5);
                }
                let data = serde_json::json!({"type": "tool", "tool": "task", "callID": part, "state": state});
                conn.execute(
                    "INSERT INTO part (id, message_id, session_id, time_created, time_updated, data)
                     VALUES (?1, 'msg_x', ?2, ?3, ?3, ?4)",
                    rusqlite::params![part, session, now, data.to_string()],
                )
                .unwrap();
            }
        }
        Line::V2 => {
            for column in ["idle_outcome TEXT", "time_idle INTEGER"] {
                // Columns 2.2.0 added; the fixture's older table lacks them.
                let _ = conn.execute_batch(&format!("ALTER TABLE session_v2 ADD COLUMN {column};"));
            }
            for (child, outcome) in [
                ("ses_kita", Some("succeeded")),
                ("ses_kitb", Some("failed")),
                ("ses_kitd", Some("succeeded")),
            ] {
                conn.execute(
                    "UPDATE session_v2 SET idle_outcome = ?1, time_idle = ?2 WHERE id = ?3",
                    rusqlite::params![outcome, now + 5, child],
                )
                .unwrap();
            }
        }
    }
    drop(conn);
    (
        juggler_adapters::opencode_store::session_ref(db, parent),
        parent.to_owned(),
    )
}
