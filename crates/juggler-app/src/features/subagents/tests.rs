//! The Subagents tab's ordering, paging, honesty and read-only rules,
//! over synthetic Claude Code `subagents/` folders read by the real
//! adapter.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use juggler_adapter::HarnessAdapter;
use juggler_adapter::subagents::{SubagentScope, SubagentStatus};

use super::*;

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn at(second: u32) -> String {
    format!("2026-10-27T10:{:02}:{:02}Z", second / 60, second % 60)
}

/// A session that started `count` background subagents, one a second;
/// every third ended, every fifth failed, and the first started a nested
/// one. One more has files and no meta (unknown).
fn session(dir: &Path, count: u32) -> PathBuf {
    let path = dir.join("proj/s1.jsonl");
    let folder = dir.join("proj/s1/subagents");
    let mut lines = Vec::new();
    for n in 0..count {
        let call = format!("t{n}");
        let agent = format!("a{n}");
        lines.push(
            serde_json::json!({"type": "assistant", "timestamp": at(n),
                "message": {"role": "assistant", "content": [{"type": "tool_use", "id": call, "name": "Agent",
                    "input": {"description": format!("Task {n}"), "prompt": "p", "subagent_type": "worker"}}]}})
            .to_string(),
        );
        lines.push(
            serde_json::json!({"type": "user", "timestamp": at(n),
                "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": call, "content": "ok"}]},
                "toolUseResult": {"status": "async_launched", "agentId": agent}})
            .to_string(),
        );
        let status = if n % 5 == 0 {
            Some("failed")
        } else if n % 3 == 0 {
            Some("completed")
        } else {
            None
        };
        if let Some(status) = status {
            lines.push(
                serde_json::json!({"type": "user", "timestamp": at(n + 1), "origin": {"kind": "task-notification"},
                    "message": {"role": "user", "content": format!(
                        "<task-notification><task-id>{agent}</task-id><status>{status}</status></task-notification>")}})
                .to_string(),
            );
        }
        write(
            &folder.join(format!("agent-{agent}.meta.json")),
            &serde_json::json!({"agentType": "worker", "description": format!("Task {n}"),
                "toolUseId": call, "spawnDepth": 1, "requestShape": "background"})
            .to_string(),
        );
        write(
            &folder.join(format!("agent-{agent}.jsonl")),
            &(serde_json::json!({"type": "user", "isSidechain": true, "timestamp": at(n),
                "message": {"role": "user", "content": format!("Task {n}")}})
            .to_string()
                + "\n"
                + &serde_json::json!({"type": "assistant", "isSidechain": true, "timestamp": at(n),
                    "message": {"role": "assistant", "content": [{"type": "text", "text": format!("Done with {n}")}]}})
                .to_string()
                + "\n"),
        );
    }
    write(&path, &(lines.join("\n") + "\n"));
    // a0 started one of its own.
    write(
        &folder.join("agent-n0.meta.json"),
        &serde_json::json!({"agentType": "Explore", "description": "Nested", "toolUseId": "x0",
            "spawnDepth": 2, "parentAgentId": "a0"})
        .to_string(),
    );
    write(&folder.join("agent-n0.jsonl"), "");
    write(
        &folder.join("agent-a0.jsonl"),
        &(serde_json::json!({"type": "assistant", "timestamp": at(0), "isSidechain": true,
            "message": {"role": "assistant", "content": [{"type": "tool_use", "id": "x0", "name": "Agent",
                "input": {"description": "Nested", "prompt": "p"}}]}})
        .to_string()
            + "\n"),
    );
    write(&folder.join("agent-lost.jsonl"), "");
    path
}

fn found_for(path: &Path, live: bool) -> Found {
    let adapter = juggler_adapters::ClaudeCodeAdapter::new();
    let mut reader = adapter
        .subagent_reader(SubagentScope { path, id: "s1" })
        .unwrap();
    let caught_up = reader.refresh().unwrap();
    Found {
        support: adapter.subagent_support(),
        live,
        reading: !caught_up,
        agents: reader.subagents(),
    }
}

fn all_pages(path: &Path, live: bool, limit: usize) -> Vec<SubagentInfo> {
    let mut out = Vec::new();
    let mut request = PageRequest::first(limit);
    loop {
        let info = listed(found_for(path, live), &request).unwrap();
        assert_eq!(info.state, "listed");
        assert!(info.page.items.len() <= limit);
        out.extend(info.page.items);
        match info.page.next_cursor {
            Some(cursor) => request = PageRequest::after(&cursor, limit),
            None => break,
        }
    }
    out
}

#[test]
fn two_hundred_subagents_page_newest_first_with_each_nested_under_its_parent() {
    let dir = tempfile::tempdir().unwrap();
    let path = session(dir.path(), 200);
    let first = listed(found_for(&path, true), &PageRequest::first(50)).unwrap();
    assert_eq!(first.page.total, Some(202));
    assert_eq!(first.page.items.len(), 50);
    assert!(first.page.next_cursor.is_some());
    let all = all_pages(&path, true, 50);
    assert_eq!(all.len(), 202, "every one once, across the pages");
    let ids: Vec<&str> = all.iter().map(|a| a.id.as_str()).collect();
    let unique: std::collections::BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len(), "none repeated");
    assert_eq!(ids[0], "a199", "newest first");
    // The nested one comes right after the one that started it.
    let parent_at = ids.iter().position(|id| *id == "a0").unwrap();
    assert_eq!(ids[parent_at + 1], "n0");
    assert_eq!(all[parent_at + 1].parent.as_deref(), Some("a0"));
    assert_eq!(all[parent_at + 1].depth, 2);
    // Unknown starts come last.
    assert_eq!(*ids.last().unwrap(), "lost");
}

#[test]
fn running_is_said_only_while_the_session_is_open_and_never_guessed() {
    let dir = tempfile::tempdir().unwrap();
    let path = session(dir.path(), 15);
    let words = |live: bool| -> BTreeMap<String, String> {
        all_pages(&path, live, 200)
            .into_iter()
            .map(|a| (a.id, a.status))
            .collect()
    };
    let open = words(true);
    assert_eq!(open["a0"], "failed");
    assert_eq!(open["a3"], "done");
    assert_eq!(open["a1"], "running");
    assert_eq!(open["lost"], "unknown", "files with no record");
    assert_eq!(open["n0"], "running");
    let info = listed(found_for(&path, true), &PageRequest::first(200)).unwrap();
    assert_eq!(info.running, 15 - 3 - 4 + 1, "a1..a14 not ended, and n0");
    let ended = words(false);
    assert_eq!(ended["a1"], "unknown", "never said how it ended");
    assert_eq!(ended["a0"], "failed");
    assert_eq!(
        listed(found_for(&path, false), &PageRequest::first(10))
            .unwrap()
            .running,
        0
    );
}

#[test]
fn a_task_is_redacted_and_a_row_says_whether_its_conversation_opens() {
    let dir = tempfile::tempdir().unwrap();
    let path = session(dir.path(), 1);
    let mut found = found_for(&path, true);
    for agent in &mut found.agents {
        if agent.id == "a0" {
            agent.task = Some("Use sk-ant-api03-abcdefghijklmnop to call it".to_owned());
        }
    }
    let info = listed(found, &PageRequest::first(10)).unwrap();
    let a0 = info.page.items.iter().find(|a| a.id == "a0").unwrap();
    let task = a0.task.as_deref().unwrap();
    assert!(
        !task.contains("abcdefghijklmnop"),
        "a test key leaked into a shown task"
    );
    assert!(a0.has_conversation);
    assert_eq!(info.conversations_note, None);
    assert_eq!(a0.kind.as_deref(), Some("worker"));
    assert_eq!(a0.background, Some(true));
}

#[test]
fn a_tool_that_keeps_none_says_why_and_lists_nothing() {
    let info = without(Missing::Unsupported("No record."));
    assert_eq!(info.state, "unsupported");
    assert_eq!(info.reason.as_deref(), Some("No record."));
    assert!(info.page.items.is_empty());
    assert_eq!(without(Missing::None).state, "none");
    assert_eq!(without(Missing::Unreadable).state, "unreadable");
}

#[test]
fn a_subagents_conversation_reads_from_its_own_transcript() {
    let dir = tempfile::tempdir().unwrap();
    let path = session(dir.path(), 2);
    let found = found_for(&path, true);
    let transcript = found
        .agents
        .iter()
        .find(|a| a.id == "a1")
        .and_then(|a| a.transcript.clone())
        .unwrap();
    let info = last_messages::read_subagent(Some(&transcript), &Ask::default());
    assert_eq!(info.state, "shown");
    assert_eq!(info.turns.len(), 2);
    assert_eq!(info.turns[0].speaker, "you");
    assert_eq!(info.turns[1].blocks[0].text, "Done with 1");
}

#[test]
fn listing_and_reading_change_nothing_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = session(dir.path(), 30);
    let snapshot = |root: &Path| -> BTreeMap<PathBuf, (Vec<u8>, Option<std::time::SystemTime>)> {
        let mut out = BTreeMap::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(at) = stack.pop() {
            for entry in std::fs::read_dir(&at).unwrap().flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    let modified = entry.metadata().unwrap().modified().ok();
                    out.insert(p.clone(), (std::fs::read(&p).unwrap(), modified));
                }
            }
        }
        out
    };
    let before = snapshot(dir.path());
    let found = found_for(&path, true);
    let _ = all_pages(&path, true, 7);
    for agent in &found.agents {
        let _ = last_messages::read_subagent(agent.transcript.as_deref(), &Ask::default());
    }
    assert!(
        snapshot(dir.path()) == before,
        "a read wrote to the tool's folder"
    );
}

#[test]
fn a_cursor_from_elsewhere_is_refused_and_an_unknown_status_stays_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let path = session(dir.path(), 3);
    let bad = PageRequest::after("not-a-cursor", 10);
    assert!(listed(found_for(&path, true), &bad).is_err());
    assert_eq!(status_word(SubagentStatus::Unknown, true), "unknown");
    assert_eq!(status_word(SubagentStatus::Stopped, false), "stopped");
}
