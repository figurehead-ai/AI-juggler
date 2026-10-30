//! AJ-E1520 (0.2): OpenCode's subagents, for the Subagents tab, from its
//! session store, read-only (D-28/D-37; the store is opened as
//! `opencode_store::open_read_only` opens it: never a write, never a
//! migration).
//!
//! An OpenCode subagent is a child session: its `parent_id` names the
//! session that started it (1.x `session`, 2.x `session_v2`; source
//! `packages/core/src/session/sql.ts` at v1.19.4 and 2.2.0), its `agent`
//! column the agent (`explore`, `general`, …) and its title the task
//! (`<description> (@<agent> subagent)`, `packages/opencode/src/tool/
//! task.ts`). Children of children are followed the same way.
//!
//! How each one stands:
//!
//! - 2.x: the child's own `idle_outcome` (`succeeded`, `failed`,
//!   `interrupted`), at `time_idle`; none yet, running.
//! - 1.x: the parent's `task` tool part for it (`state.metadata.sessionId`
//!   names the child; `state.status` `pending`/`running`/`completed`/
//!   `error`, `state.time.start`/`end`). One started in the background
//!   answers at once, so its own last message says instead: an `error`,
//!   failed; `time.completed` with a `finish` other than `tool-calls`,
//!   done; else running. With neither, unknown (D-06).
//!
//! Route: the store, read-only. OpenCode's sanctioned readers don't fit a
//! read-only view: `opencode session list --format json` lists top-level
//! sessions only (no parent ids), `opencode export` one session without
//! its children, and both open the store with migrations and write logs;
//! `GET /session/:id/children` needs an OpenCode server running on the
//! user's store, which AI Juggler doesn't start for this. OpenCode
//! Desktop runs the same server on the same store, so its sessions read
//! the same way.
//!
//! A child's conversation is in the store too (`message`/`part` rows), not
//! in a transcript the session page's reader reads (OpenCode's own
//! session conversations aren't shown either), so it isn't offered yet.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use juggler_adapter::subagents::{
    Subagent, SubagentConversations, SubagentReader, SubagentScope, SubagentStatus, SubagentSupport,
};
use rusqlite::{Connection, OptionalExtension};

/// What the OpenCode adapter answers.
pub const SUPPORT: SubagentSupport = SubagentSupport::Supported {
    route: "OpenCode's session store, read-only: child sessions by parent_id (session / session_v2), each one's outcome (2.x) or its task tool part and last message (1.x)",
    conversations: SubagentConversations::NotRead(
        "OpenCode keeps a subagent's conversation in its database, which AI Juggler doesn't show as a conversation yet.",
    ),
    stop_hint: None,
};

/// The most child sessions one session is read for.
const CHILDREN_MAX: usize = 2_000;
/// How deep children of children are followed.
const DEPTH_MAX: u32 = 8;

/// A reader for the stored session `scope` names (`<db>#<id>`).
#[must_use]
pub fn reader(scope: SubagentScope<'_>) -> Option<Box<dyn SubagentReader>> {
    let id = crate::opencode_store::session_of_ref(scope.path)?;
    let text = scope.path.to_string_lossy();
    let (db, _) = text.rsplit_once('#')?;
    Some(Box::new(OpenCodeSubagents {
        db: PathBuf::from(db),
        session: id,
        found: Vec::new(),
    }))
}

/// One session's subagents, read from the store at each refresh.
#[derive(Debug)]
pub struct OpenCodeSubagents {
    db: PathBuf,
    session: String,
    found: Vec<Subagent>,
}

/// A child session's row.
#[derive(Debug, Clone)]
struct Child {
    id: String,
    parent: String,
    title: String,
    agent: Option<String>,
    created_ms: Option<i64>,
    /// 2.x: its outcome, and when it went idle.
    outcome: Option<String>,
    idle_ms: Option<i64>,
    /// Which table it came from: `session_v2` holds outcomes.
    v2: bool,
}

/// A `task` tool part of a 1.x parent, for one child.
#[derive(Debug, Clone, Default)]
struct TaskPart {
    status: Option<String>,
    description: Option<String>,
    kind: Option<String>,
    background: Option<bool>,
    start_ms: Option<i64>,
    end_ms: Option<i64>,
}

fn columns(conn: &Connection, table: &str) -> BTreeSet<String> {
    // The table name is one of two literals, never input.
    let Ok(mut stmt) = conn.prepare(&format!("PRAGMA table_info({table})")) else {
        return BTreeSet::new();
    };
    stmt.query_map([], |row| row.get::<_, String>(1))
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
}

/// The children of `parent` in `table`.
fn children(conn: &Connection, table: &str, parent: &str) -> Vec<Child> {
    let have = columns(conn, table);
    let pick = |name: &str| {
        if have.contains(name) {
            name.to_owned()
        } else {
            "NULL".to_owned()
        }
    };
    let v2 = table == "session_v2";
    // Every name is a literal or a column the table says it has.
    let sql = format!(
        "SELECT id, COALESCE(title, ''), {agent}, {created}, {outcome}, {idle}
         FROM {table} WHERE parent_id = ?1 LIMIT {CHILDREN_MAX}",
        agent = pick("agent"),
        created = pick("time_created"),
        outcome = if v2 {
            pick("idle_outcome")
        } else {
            "NULL".to_owned()
        },
        idle = if v2 {
            pick("time_idle")
        } else {
            "NULL".to_owned()
        },
    );
    let Ok(mut stmt) = conn.prepare(&sql) else {
        return Vec::new();
    };
    stmt.query_map([parent], |row| {
        Ok(Child {
            id: row.get(0)?,
            parent: parent.to_owned(),
            title: row.get(1)?,
            agent: row.get::<_, Option<String>>(2).ok().flatten(),
            created_ms: row.get::<_, Option<i64>>(3).ok().flatten(),
            outcome: row.get::<_, Option<String>>(4).ok().flatten(),
            idle_ms: row.get::<_, Option<i64>>(5).ok().flatten(),
            v2,
        })
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

/// 1.x: the `task` tool parts of session `parent`, by the child they
/// started.
fn task_parts(conn: &Connection, parent: &str) -> HashMap<String, TaskPart> {
    let mut out = HashMap::new();
    let Ok(mut stmt) =
        conn.prepare("SELECT data FROM part WHERE session_id = ?1 AND instr(data, '\"task\"') > 0")
    else {
        return out;
    };
    let Ok(rows) = stmt.query_map([parent], |row| row.get::<_, String>(0)) else {
        return out;
    };
    for data in rows.flatten() {
        let Ok(part) = serde_json::from_str::<serde_json::Value>(&data) else {
            continue;
        };
        if part.get("type").and_then(serde_json::Value::as_str) != Some("tool")
            || part.get("tool").and_then(serde_json::Value::as_str) != Some("task")
        {
            continue;
        }
        let state = part.get("state").unwrap_or(&serde_json::Value::Null);
        let metadata = state.get("metadata").unwrap_or(&serde_json::Value::Null);
        let Some(child) = metadata
            .get("sessionId")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let text = |value: &serde_json::Value, key: &str| {
            value
                .get(key)
                .and_then(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        let input = state.get("input").unwrap_or(&serde_json::Value::Null);
        let time = state.get("time").unwrap_or(&serde_json::Value::Null);
        out.insert(
            child.to_owned(),
            TaskPart {
                status: text(state, "status"),
                description: text(input, "description").or_else(|| text(state, "title")),
                kind: text(input, "subagent_type"),
                background: metadata
                    .get("background")
                    .and_then(serde_json::Value::as_bool)
                    .or_else(|| input.get("background").and_then(serde_json::Value::as_bool)),
                start_ms: time.get("start").and_then(serde_json::Value::as_i64),
                end_ms: time.get("end").and_then(serde_json::Value::as_i64),
            },
        );
    }
    out
}

/// 1.x: how a child stands by its own last message, and when it ended.
fn last_message(conn: &Connection, child: &str) -> Option<(SubagentStatus, Option<i64>)> {
    let data: String = conn
        .query_row(
            "SELECT data FROM message WHERE session_id = ?1 ORDER BY time_created DESC LIMIT 1",
            [child],
            |row| row.get(0),
        )
        .optional()
        .ok()
        .flatten()?;
    let message: serde_json::Value = serde_json::from_str(&data).ok()?;
    if message.get("role").and_then(serde_json::Value::as_str) != Some("assistant") {
        return Some((SubagentStatus::Running, None));
    }
    let completed = message
        .get("time")
        .and_then(|time| time.get("completed"))
        .and_then(serde_json::Value::as_i64);
    if message.get("error").is_some_and(|error| !error.is_null()) {
        return Some((SubagentStatus::Failed, completed));
    }
    let finish = message.get("finish").and_then(serde_json::Value::as_str);
    match (completed, finish) {
        (Some(at), finish) if finish != Some("tool-calls") => {
            Some((SubagentStatus::Done, Some(at)))
        }
        _ => Some((SubagentStatus::Running, None)),
    }
}

/// `<description> (@<agent> subagent)` → the description.
fn task_of(title: &str) -> Option<String> {
    let title = title.trim();
    let task = match title.rfind(" (@") {
        Some(at) if title.ends_with(" subagent)") => &title[..at],
        _ => title,
    };
    (!task.is_empty()).then(|| task.to_owned())
}

fn described(
    conn: &Connection,
    child: &Child,
    part: Option<&TaskPart>,
    depth: u32,
    session: &str,
) -> Subagent {
    let (status, ended_ms) = if child.v2 {
        match child.outcome.as_deref() {
            Some("succeeded") => (SubagentStatus::Done, child.idle_ms),
            Some("failed") => (SubagentStatus::Failed, child.idle_ms),
            Some("interrupted") => (SubagentStatus::Stopped, child.idle_ms),
            None => (SubagentStatus::Running, None),
            Some(_) => (SubagentStatus::Unknown, None),
        }
    } else {
        let by_part = part
            .filter(|part| part.background != Some(true))
            .and_then(|part| match part.status.as_deref() {
                Some("pending" | "running") => Some((SubagentStatus::Running, None)),
                Some("completed") => Some((SubagentStatus::Done, part.end_ms)),
                Some("error") => Some((SubagentStatus::Failed, part.end_ms)),
                _ => None,
            });
        by_part
            .or_else(|| last_message(conn, &child.id))
            .unwrap_or((SubagentStatus::Unknown, None))
    };
    Subagent {
        id: child.id.clone(),
        kind: child
            .agent
            .clone()
            .filter(|agent| !agent.is_empty())
            .or_else(|| part.and_then(|part| part.kind.clone())),
        task: part
            .and_then(|part| part.description.clone())
            .or_else(|| task_of(&child.title)),
        status,
        started_ms: part.and_then(|part| part.start_ms).or(child.created_ms),
        ended_ms,
        parent: (child.parent != session).then(|| child.parent.clone()),
        depth,
        background: part.and_then(|part| part.background),
        transcript: None,
        details: juggler_adapter::subagents::SubagentDetails::default(),
    }
}

impl SubagentReader for OpenCodeSubagents {
    fn refresh(&mut self) -> std::io::Result<bool> {
        let conn =
            crate::opencode_store::open_read_only(&self.db).map_err(std::io::Error::other)?;
        let tables: Vec<&str> = ["session_v2", "session"]
            .into_iter()
            .filter(|table| crate::opencode_store::has_table(&conn, table))
            .collect();
        if tables.is_empty() {
            return Err(std::io::Error::other("no OpenCode session table"));
        }
        let parts_table = crate::opencode_store::has_table(&conn, "part");
        let mut found: Vec<Subagent> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut level: Vec<String> = vec![self.session.clone()];
        for depth in 1..=DEPTH_MAX {
            let mut next = Vec::new();
            for parent in &level {
                let parts = if parts_table {
                    task_parts(&conn, parent)
                } else {
                    HashMap::new()
                };
                for table in &tables {
                    for child in children(&conn, table, parent) {
                        if found.len() >= CHILDREN_MAX || !seen.insert(child.id.clone()) {
                            continue;
                        }
                        found.push(described(
                            &conn,
                            &child,
                            parts.get(&child.id),
                            depth,
                            &self.session,
                        ));
                        next.push(child.id);
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            level = next;
        }
        self.found = found;
        Ok(true)
    }

    fn subagents(&self) -> Vec<Subagent> {
        self.found.clone()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use std::path::Path;

    fn db_with(dir: &Path, v2: bool) -> PathBuf {
        let db = dir.join("opencode.db");
        let conn = Connection::open(&db).unwrap();
        if v2 {
            conn.execute_batch(
                "CREATE TABLE session_v2 (id TEXT PRIMARY KEY, parent_id TEXT, title TEXT, agent TEXT,
                   time_created INTEGER, time_updated INTEGER, idle_outcome TEXT, time_idle INTEGER);",
            )
            .unwrap();
        } else {
            conn.execute_batch(
                "CREATE TABLE session (id TEXT PRIMARY KEY, parent_id TEXT, title TEXT, agent TEXT,
                   time_created INTEGER, time_updated INTEGER);
                 CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT, time_created INTEGER, data TEXT);
                 CREATE TABLE part (id TEXT PRIMARY KEY, message_id TEXT, session_id TEXT, time_created INTEGER, data TEXT);",
            )
            .unwrap();
        }
        db
    }

    fn task_part(
        conn: &Connection,
        id: &str,
        parent: &str,
        child: &str,
        status: &str,
        background: bool,
    ) {
        let data = serde_json::json!({"type": "tool", "tool": "task", "callID": id, "state": {
            "status": status, "title": "t",
            "input": {"description": format!("task {child}"), "prompt": "p", "subagent_type": "explore"},
            "metadata": {"sessionId": child, "parentSessionId": parent, "background": background},
            "time": {"start": 1_000, "end": 2_000}}});
        conn.execute(
            "INSERT INTO part (id, message_id, session_id, time_created, data) VALUES (?1, 'm', ?2, 1, ?3)",
            rusqlite::params![id, parent, data.to_string()],
        )
        .unwrap();
    }

    fn scope_reader(db: &Path, id: &str) -> Box<dyn SubagentReader> {
        let key = crate::opencode_store::session_ref(db, id);
        reader(SubagentScope { path: &key, id }).expect("a stored session's reader")
    }

    #[test]
    fn one_x_children_stand_as_their_task_part_or_their_last_message_says() {
        let dir = tempfile::tempdir().unwrap();
        let db = db_with(dir.path(), false);
        {
            let conn = Connection::open(&db).unwrap();
            for (id, parent) in [
                ("ses_p", None),
                ("ses_a", Some("ses_p")),
                ("ses_b", Some("ses_p")),
                ("ses_c", Some("ses_p")),
                ("ses_d", Some("ses_a")),
                ("ses_other", None),
            ] {
                conn.execute(
                    "INSERT INTO session (id, parent_id, title, agent, time_created, time_updated) VALUES (?1, ?2, ?3, 'explore', 500, 600)",
                    rusqlite::params![id, parent, format!("Look at {id} (@explore subagent)")],
                )
                .unwrap();
            }
            task_part(&conn, "c1", "ses_p", "ses_a", "completed", false);
            task_part(&conn, "c2", "ses_p", "ses_b", "error", false);
            // In the background: its part completes at once; its own last
            // message says it's still going.
            task_part(&conn, "c3", "ses_p", "ses_c", "completed", true);
            conn.execute(
                "INSERT INTO message (id, session_id, time_created, data) VALUES ('m1', 'ses_c', 5, ?1)",
                [serde_json::json!({"role": "assistant", "time": {"created": 5}, "finish": "tool-calls"}).to_string()],
            )
            .unwrap();
            // ses_d (started by ses_a) has no part and no message: unknown.
        }
        let mut reader = scope_reader(&db, "ses_p");
        assert!(reader.refresh().unwrap());
        let list = reader.subagents();
        let by = |id: &str| list.iter().find(|a| a.id == id).expect("listed");
        assert_eq!(list.len(), 4);
        assert_eq!(by("ses_a").status, SubagentStatus::Done);
        assert_eq!(by("ses_a").task.as_deref(), Some("task ses_a"));
        assert_eq!(by("ses_a").ended_ms, Some(2_000));
        assert_eq!(by("ses_b").status, SubagentStatus::Failed);
        assert_eq!(by("ses_c").status, SubagentStatus::Running);
        assert_eq!(by("ses_c").background, Some(true));
        assert_eq!(by("ses_d").status, SubagentStatus::Unknown);
        assert_eq!(by("ses_d").parent.as_deref(), Some("ses_a"));
        assert_eq!(by("ses_d").depth, 2);
        assert_eq!(by("ses_d").task.as_deref(), Some("Look at ses_d"));
    }

    #[test]
    fn two_x_children_stand_as_their_outcome_says() {
        let dir = tempfile::tempdir().unwrap();
        let db = db_with(dir.path(), true);
        {
            let conn = Connection::open(&db).unwrap();
            for (id, outcome) in [
                ("a", Some("succeeded")),
                ("b", Some("failed")),
                ("c", Some("interrupted")),
                ("d", None),
            ] {
                conn.execute(
                    "INSERT INTO session_v2 (id, parent_id, title, agent, time_created, idle_outcome, time_idle) VALUES (?1, 'p', 'Do it (@general subagent)', 'general', 100, ?2, 900)",
                    rusqlite::params![id, outcome],
                )
                .unwrap();
            }
        }
        let before = std::fs::read(&db).unwrap();
        let mut reader = scope_reader(&db, "p");
        reader.refresh().unwrap();
        let list = reader.subagents();
        let by = |id: &str| list.iter().find(|a| a.id == id).expect("listed").status;
        assert_eq!(by("a"), SubagentStatus::Done);
        assert_eq!(by("b"), SubagentStatus::Failed);
        assert_eq!(by("c"), SubagentStatus::Stopped);
        assert_eq!(by("d"), SubagentStatus::Running);
        assert_eq!(list[0].kind.as_deref(), Some("general"));
        assert_eq!(list[0].task.as_deref(), Some("Do it"));
        // Read-only: the store is as it was, and no log appeared.
        assert_eq!(std::fs::read(&db).unwrap(), before);
        assert!(!dir.path().join("opencode.db-wal").exists());
    }
}
