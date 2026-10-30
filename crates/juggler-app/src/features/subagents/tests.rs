//! The Subagents tab's ordering, paging, honesty and read-only rules,
//! over synthetic Claude Code `subagents/` folders read by the real
//! adapter.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use juggler_adapter::HarnessAdapter;
use juggler_adapter::subagents::{SubagentScope, SubagentStatus};

use super::dto::{SubagentInfo, SubagentsQuery};
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
    reader.refresh_details(u64::MAX).unwrap();
    let agents = reader.subagents();
    let cleans = clean::Cleaned::default().all(&agents, true);
    Found {
        support: adapter.subagent_support(),
        live,
        reading: !caught_up,
        agents,
        cleans,
        session_tokens: reader.session_tokens(),
        counted: reader.tokens_counted(),
        tool: "Claude Code".to_owned(),
        now_ms: juggler_core::time::timestamp_ms("2026-10-27T11:00:00Z").unwrap(),
        sessions: std::collections::HashMap::new(),
    }
}

/// Every row of every page, for `query`.
fn rows_for(found: &Found, query: &SubagentsQuery, limit: usize) -> Vec<dto::SubagentRowInfo> {
    let mut out = Vec::new();
    let mut request = PageRequest::first(limit);
    loop {
        let info = view::listed(found, query, &request).unwrap();
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

/// Every subagent row, every group open.
fn all_pages(path: &Path, live: bool, limit: usize) -> Vec<SubagentInfo> {
    let found = found_for(path, live);
    let groups: Vec<String> = rows_for(&found, &SubagentsQuery::default(), 200)
        .into_iter()
        .filter_map(|row| row.group.map(|g| g.key))
        .collect();
    let query = SubagentsQuery {
        open: groups,
        ..SubagentsQuery::default()
    };
    rows_for(&found, &query, limit)
        .into_iter()
        .filter_map(|row| row.agent)
        .collect()
}

#[test]
fn two_hundred_subagents_page_newest_first_with_each_nested_under_its_parent() {
    let dir = tempfile::tempdir().unwrap();
    let path = session(dir.path(), 200);
    let first = view::listed(
        &found_for(&path, true),
        &SubagentsQuery::default(),
        &PageRequest::first(50),
    )
    .unwrap();
    assert_eq!(first.summary.total, 202);
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
    let info = view::listed(
        &found_for(&path, true),
        &SubagentsQuery::default(),
        &PageRequest::first(200),
    )
    .unwrap();
    assert_eq!(info.running, 15 - 3 - 4 + 1, "a1..a14 not ended, and n0");
    let ended = words(false);
    assert_eq!(ended["a1"], "unknown", "never said how it ended");
    assert_eq!(ended["a0"], "failed");
    assert_eq!(
        view::listed(
            &found_for(&path, false),
            &SubagentsQuery::default(),
            &PageRequest::first(10)
        )
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
    found.cleans = clean::Cleaned::default().all(&found.agents, true);
    let info = view::listed(&found, &SubagentsQuery::default(), &PageRequest::first(10)).unwrap();
    let a0 = info
        .page
        .items
        .iter()
        .filter_map(|row| row.agent.as_ref())
        .find(|a| a.id == "a0")
        .unwrap();
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
    assert!(view::listed(&found_for(&path, true), &SubagentsQuery::default(), &bad).is_err());
    assert_eq!(status_word(SubagentStatus::Unknown, true), "unknown");
    assert_eq!(status_word(SubagentStatus::Stopped, false), "stopped");
}

// AJ-E1520 (0.3): groups, views, search, order, tree lines, shares, a
// subagent's page, a project's view and the saved index.

/// Two turns: the first asked for a1 (which started n1) and a2; the
/// second for a3. Each subagent's transcript has its tokens.
fn turns(dir: &Path) -> PathBuf {
    let path = dir.join("proj/s2.jsonl");
    let folder = dir.join("proj/s2/subagents");
    let ask = |text: &str, uuid: &str, at: &str| {
        serde_json::json!({"type": "user", "uuid": uuid, "timestamp": at, "message": {"role": "user", "content": text}})
            .to_string()
    };
    let start = |call: &str, task: &str, kind: &str, at: &str| {
        serde_json::json!({"type": "assistant", "timestamp": at, "message": {"role": "assistant", "content": [
            {"type": "tool_use", "id": call, "name": "Agent", "input": {"description": task, "prompt": format!("Prompt for {task}"), "subagent_type": kind}}]}})
        .to_string()
    };
    let launched = |call: &str, agent: &str, at: &str| {
        serde_json::json!({"type": "user", "timestamp": at, "message": {"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": call, "content": "ok"}]}, "toolUseResult": {"status": "async_launched", "agentId": agent}})
        .to_string()
    };
    let done = |agent: &str, status: &str, at: &str| {
        serde_json::json!({"type": "user", "timestamp": at, "origin": {"kind": "task-notification"}, "message": {"role": "user",
            "content": format!("<task-notification><task-id>{agent}</task-id><status>{status}</status><result>Result of {agent}</result></task-notification>")}})
        .to_string()
    };
    let reply = |id: &str, tokens: u64, at: &str, tools: serde_json::Value| {
        serde_json::json!({"type": "assistant", "timestamp": at, "message": {"id": id, "role": "assistant", "model": "claude-sonnet-4-5",
            "usage": {"input_tokens": tokens, "output_tokens": 0}, "content": tools}})
        .to_string()
    };
    write(
        &path,
        &([
            ask("fix the totals", "u1", &at(0)),
            reply("s-m1", 1000, &at(1), serde_json::json!([])),
            start("t1", "Audit the cart", "general-purpose", &at(2)),
            launched("t1", "a1", &at(2)),
            start("t2", "Write rounding tests", "general-purpose", &at(3)),
            launched("t2", "a2", &at(3)),
            done("a2", "failed", &at(30)),
            ask("split the page", "u2", &at(100)),
            start("t3", "Split checkout", "Explore", &at(101)),
            launched("t3", "a3", &at(101)),
            done("a3", "completed", &at(160)),
        ]
        .join("\n")
            + "\n"),
    );
    for (agent, call, kind, task) in [
        ("a1", "t1", "general-purpose", "Audit the cart"),
        ("a2", "t2", "general-purpose", "Write rounding tests"),
        ("a3", "t3", "Explore", "Split checkout"),
    ] {
        write(
            &folder.join(format!("agent-{agent}.meta.json")),
            &serde_json::json!({"agentType": kind, "description": task, "toolUseId": call, "spawnDepth": 1, "requestShape": "background"}).to_string(),
        );
    }
    write(
        &folder.join("agent-n1.meta.json"),
        &serde_json::json!({"agentType": "Explore", "description": "Read the tax tables", "toolUseId": "x1", "spawnDepth": 2, "parentAgentId": "a1"}).to_string(),
    );
    let edit = serde_json::json!([{"type": "tool_use", "id": "e1", "name": "Edit", "input": {"file_path": "/p/src/tax.ts"}},
        {"type": "tool_use", "id": "e2", "name": "Bash", "input": {"command": "npm run dev"}}]);
    write(
        &folder.join("agent-a1.jsonl"),
        &([
            reply("a1-m1", 400, &at(5), serde_json::json!([])),
            start("x1", "Read the tax tables", "Explore", &at(6)),
            launched("x1", "n1", &at(6)),
        ]
        .join("\n")
            + "\n"),
    );
    write(
        &folder.join("agent-n1.jsonl"),
        &(reply("n1-m1", 50, &at(7), edit) + "\n"),
    );
    write(
        &folder.join("agent-a2.jsonl"),
        &(reply("a2-m1", 300, &at(8), serde_json::json!([])) + "\n"),
    );
    write(
        &folder.join("agent-a3.jsonl"),
        &(reply("a3-m1", 250, &at(102), serde_json::json!([])) + "\n"),
    );
    path
}

fn found_turns(dir: &Path) -> Found {
    let mut found = found_for(&turns(dir), true);
    found.now_ms = juggler_core::time::timestamp_ms(&at(200)).unwrap();
    found
}

fn keys(rows: &[dto::SubagentRowInfo]) -> Vec<String> {
    rows.iter().map(|row| row.key.clone()).collect()
}

#[test]
fn subagents_group_under_the_message_that_started_them_newest_open_older_folded() {
    let dir = tempfile::tempdir().unwrap();
    let found = found_turns(dir.path());
    let rows = rows_for(&found, &SubagentsQuery::default(), 50);
    assert_eq!(
        keys(&rows),
        ["g:u2", "a:a3", "g:u1"],
        "the newest group open, the older folded"
    );
    let group = rows[0].group.as_ref().unwrap();
    assert_eq!(group.asked.as_deref(), Some("split the page"));
    assert_eq!((group.count, group.tokens), (1, Some(250)));
    let older = rows[2].group.as_ref().unwrap();
    assert!(!older.open);
    assert_eq!(
        (older.count, older.failed, older.running),
        (3, 1, 2),
        "a1 and the one it started run"
    );
    assert_eq!(older.tokens, Some(750));
    // Opened: a tree with its lines.
    let query = SubagentsQuery {
        open: vec!["u1".to_owned()],
        closed: vec!["u2".to_owned()],
        ..SubagentsQuery::default()
    };
    let rows = rows_for(&found, &query, 50);
    assert_eq!(keys(&rows), ["g:u2", "g:u1", "a:a2", "a:a1", "a:n1"]);
    let (a2, a1, n1) = (&rows[2], &rows[3], &rows[4]);
    assert!(!a2.last && a1.last, "a1 is the last of the turn's");
    assert_eq!(n1.guides, vec![false], "under the last: no line runs on");
    assert!(n1.last);
    let a1 = a1.agent.as_ref().unwrap();
    assert_eq!(
        (a1.tokens, a1.own_tokens, a1.children),
        (Some(450), Some(400), 1),
        "a parent's tokens include its children's"
    );
    assert_eq!(a1.model.as_deref(), Some("claude-sonnet-4-5"));
}

#[test]
fn views_search_type_and_order_narrow_and_keep_the_tree() {
    let dir = tempfile::tempdir().unwrap();
    let found = found_turns(dir.path());
    let ask = |query: SubagentsQuery| {
        let info = view::listed(&found, &query, &PageRequest::first(50)).unwrap();
        (keys(&info.page.items), info.matched, info.page.items)
    };
    let (rows, matched, items) = ask(SubagentsQuery {
        view: Some("running".to_owned()),
        ..SubagentsQuery::default()
    });
    assert_eq!(matched, 2, "a1 and n1 run");
    assert_eq!(rows, ["g:u1", "a:a1", "a:n1"], "a filter opens its groups");
    assert!(items.iter().all(|row| !row.context));
    let (rows, _, items) = ask(SubagentsQuery {
        view: Some("attention".to_owned()),
        ..SubagentsQuery::default()
    });
    assert_eq!(rows, ["g:u1", "a:a2"]);
    assert!(!items[1].context);
    // A nested match shows its parent, dimmed.
    let (rows, matched, items) = ask(SubagentsQuery {
        search: Some("file:tax.ts".to_owned()),
        ..SubagentsQuery::default()
    });
    assert_eq!(
        (rows, matched),
        (
            vec!["g:u1".to_owned(), "a:a1".to_owned(), "a:n1".to_owned()],
            1
        )
    );
    assert!(items[1].context && !items[2].context);
    let (rows, _, _) = ask(SubagentsQuery {
        search: Some("type:explore checkout".to_owned()),
        ..SubagentsQuery::default()
    });
    assert_eq!(rows, ["g:u2", "a:a3"]);
    let (rows, _, _) = ask(SubagentsQuery {
        search: Some("result of a3".to_owned()),
        ..SubagentsQuery::default()
    });
    assert_eq!(rows, ["g:u2", "a:a3"], "the result is searched");
    let (rows, _, _) = ask(SubagentsQuery {
        kind: Some("Explore".to_owned()),
        ..SubagentsQuery::default()
    });
    assert_eq!(rows, ["g:u2", "a:a3", "g:u1", "a:a1", "a:n1"]);
    // Most tokens first: the group, then within it.
    let (rows, _, _) = ask(SubagentsQuery {
        sort: Some("tokens".to_owned()),
        ..SubagentsQuery::default()
    });
    assert_eq!(rows, ["g:u1", "a:a1", "a:n1", "a:a2", "g:u2"]);
    let info = view::listed(&found, &SubagentsQuery::default(), &PageRequest::first(5)).unwrap();
    assert_eq!(
        info.kinds
            .iter()
            .map(|k| (k.kind.as_str(), k.count))
            .collect::<Vec<_>>(),
        [("Explore", 2), ("general-purpose", 2)]
    );
}

#[test]
fn subagents_tokens_are_a_share_of_the_sessions_never_added_on_top() {
    let dir = tempfile::tempdir().unwrap();
    let found = found_turns(dir.path());
    let info = view::listed(&found, &SubagentsQuery::default(), &PageRequest::first(5)).unwrap();
    let summary = info.summary;
    assert_eq!(
        (
            summary.total,
            summary.running,
            summary.failed,
            summary.attention
        ),
        (4, 2, 1, 1)
    );
    assert_eq!(summary.tokens, Some(1000));
    assert_eq!(summary.counted, "in_session");
    assert_eq!(
        summary.share_pct,
        Some(50.0),
        "1,000 of the session's 2,000"
    );
}

#[test]
fn a_subagents_page_says_what_it_was_given_did_and_used() {
    let dir = tempfile::tempdir().unwrap();
    let found = found_turns(dir.path());
    let price = |model: &str, _: i64, tokens: juggler_adapter::subagents::SubagentTokens| {
        (model == "claude-sonnet-4-5").then(|| tokens.total() as f64 / 1000.0)
    };
    let around = |messages: bool| detail::Around {
        price: &price,
        processes: Vec::new(),
        process_session: Some("s2".to_owned()),
        messages,
    };
    let page = detail::detail_of(&found, "n1", &around(true));
    assert_eq!(page.state, "shown");
    assert_eq!(
        page.chain.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["a1"]
    );
    assert_eq!(page.chain[0].title.as_deref(), Some("Audit the cart"));
    assert_eq!(
        page.prompt.as_deref(),
        Some("Prompt for Read the tax tables")
    );
    assert_eq!(
        page.asked.as_deref(),
        Some("fix the totals"),
        "its first ancestor's turn"
    );
    assert_eq!(page.files, ["/p/src/tax.ts"]);
    assert_eq!(
        page.tools.map(|t| (t.total, t.edits, t.commands)),
        Some((2, 1, 1))
    );
    assert_eq!(page.commands, 1);
    assert_eq!(page.tokens.map(|t| t.input), Some(50));
    assert_eq!(page.cost_usd, Some(0.05));
    assert_eq!(page.share_pct, Some(2.5));
    assert_eq!(
        page.stop_hint.as_deref(),
        Some("In Claude Code: /tasks, select it, press x.")
    );
    let a1 = detail::detail_of(&found, "a1", &around(true));
    assert_eq!(
        a1.children
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["n1"]
    );
    assert_eq!(a1.children_total, 1);
    let a3 = detail::detail_of(&found, "a3", &around(true));
    assert_eq!(a3.result.as_deref(), Some("Result of a3"));
    let off = detail::detail_of(&found, "a3", &around(false));
    assert!(off.messages_off);
    assert_eq!(
        (off.prompt, off.result, off.asked),
        (None, None, None),
        "last messages off: none shown"
    );
    assert_eq!(
        detail::detail_of(&found, "nope", &around(true)).state,
        "none"
    );
}

#[test]
fn a_projects_view_is_its_sessions_together_each_group_saying_its_session() {
    let dir = tempfile::tempdir().unwrap();
    let one = found_turns(dir.path());
    let other_dir = tempfile::tempdir().unwrap();
    let mut two = found_for(&session(other_dir.path(), 3), false);
    two.tool = "Claude Code".to_owned();
    let session = |id: &str| dto::SubagentSessionInfo {
        account_id: "acc".to_owned(),
        conversation_id: id.to_owned(),
        title: Some(format!("Session {id}")),
    };
    let found = project::merged(vec![(session("s2"), one), (session("s1"), two)]).unwrap();
    let info = view::listed(&found, &SubagentsQuery::default(), &PageRequest::first(50)).unwrap();
    assert_eq!(info.summary.total, 4 + 5);
    let groups: Vec<_> = info
        .page
        .items
        .iter()
        .filter_map(|row| row.group.as_ref())
        .collect();
    assert!(groups.iter().all(|g| g.session.is_some()));
    assert!(
        groups
            .iter()
            .any(|g| g.session.as_ref().unwrap().conversation_id == "s1")
    );
    // The ended session's still-running one is unknown, not running.
    assert_eq!(info.summary.running, 2, "only the open session's");
}

#[test]
fn a_saved_index_opens_the_tab_without_reading_the_transcripts_again() {
    let dir = tempfile::tempdir().unwrap();
    let index = tempfile::tempdir().unwrap();
    let path = turns(dir.path());
    let adapter = juggler_adapters::ClaudeCodeAdapter::new();
    let make = || {
        adapter
            .subagent_reader(SubagentScope {
                path: &path,
                id: "s2",
            })
            .unwrap()
    };
    let key = ("acc".to_owned(), "s2".to_owned(), path.clone());
    let cache = crate::features::cache::Gate::default();
    let first = Entry::resumed(&cache, index.path(), true, key.clone(), make());
    let listed = first.list(u64::MAX).unwrap();
    let cleans = first.clean.lock().unwrap().all(&listed.agents, true);
    first.remember(&listed.agents, &cleans);
    first.save(&cache, index.path(), true);
    let saved = index::path_for(index.path(), &key.0, &key.1, &key.2);
    assert!(saved.exists(), "the index was saved");
    // Its commands, for the Processes view.
    let commands = first.commands.lock().unwrap().clone();
    assert_eq!(commands.len(), 1);
    assert_eq!(
        (commands[0].id.as_str(), commands[0].command.as_str()),
        ("n1", "npm run dev")
    );
    // A new reader from the index: nothing left to read, the same list.
    let second = Entry::resumed(&cache, index.path(), true, key.clone(), make());
    {
        let mut reader = second.reader.lock().unwrap();
        assert!(reader.refresh().unwrap());
        assert!(
            reader.refresh_details(0).unwrap(),
            "caught up from the index alone"
        );
    }
    let again = second.list(0).unwrap();
    assert_eq!(again.agents.len(), listed.agents.len());
    // Saved with last messages on: not used with them off.
    let third = Entry::resumed(&cache, index.path(), false, key, make());
    let mut reader = third.reader.lock().unwrap();
    reader.refresh().unwrap();
    assert!(!reader.refresh_details(0).unwrap(), "read from the start");
}

#[test]
fn turning_last_messages_off_keeps_no_prompt_in_the_index_and_cache_off_saves_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let index = tempfile::tempdir().unwrap();
    let path = turns(dir.path());
    let adapter = juggler_adapters::ClaudeCodeAdapter::new();
    let key = ("acc".to_owned(), "s2".to_owned(), path.clone());
    let saved = index::path_for(index.path(), &key.0, &key.1, &key.2);
    let cache = crate::features::cache::Gate::default();
    let entry = Entry::resumed(
        &cache,
        index.path(),
        true,
        key,
        adapter
            .subagent_reader(SubagentScope {
                path: &path,
                id: "s2",
            })
            .unwrap(),
    );
    let listed = entry.list(u64::MAX).unwrap();
    assert!(!listed.agents.is_empty());
    entry.save(&cache, index.path(), true);
    let text = String::from_utf8_lossy(&std::fs::read(&saved).unwrap()).into_owned();
    assert!(text.contains("Prompt for Audit the cart"), "kept while on");
    // Last messages off: the same reader saves again at once, and the
    // prompts its memo redacted while they were on are not kept.
    entry.save(&cache, index.path(), false);
    let text = String::from_utf8_lossy(&std::fs::read(&saved).unwrap()).into_owned();
    assert!(!text.contains("Prompt for"), "no prompt once they're off");
    assert!(!text.contains("Result of"), "no result either");
    assert!(text.contains("Audit the cart"), "the task stays");
    // The cache off: nothing is saved.
    std::fs::remove_file(&saved).unwrap();
    let off = crate::features::cache::Manager::new(
        index.path().to_path_buf(),
        false,
        crate::features::cache::Limit::Automatic,
        None,
    );
    entry.save(&off.gate(), index.path(), true);
    assert!(!saved.exists(), "nothing written while the cache is off");
}
