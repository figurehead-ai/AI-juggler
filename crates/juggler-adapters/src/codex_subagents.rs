//! AJ-E1520 (0.2): Codex's subagents, for the Subagents tab, from its
//! rollouts (read-only, D-28/D-37).
//!
//! A Codex subagent is a thread of its own: its rollout sits in the same
//! `sessions/YYYY/MM/DD/` tree as its parent's, and its first line (the
//! `session_meta`) says whose child it is
//! (`source.subagent.thread_spawn.parent_thread_id`), how deep it is
//! (`depth`), its path (`agent_path`), nickname and role (`agent_role`).
//! The same files serve Codex in the `ChatGPT` app, which runs the same
//! Codex on the same `~/.codex`.
//!
//! How each one stands comes from its parent's rollout, read by the shared
//! subagent log (`codex_transcript::subtask_line`): the `spawn_agent`
//! call (its task and type), and what `list_agents`, `interrupt_agent`
//! and the `SubAgentActivity` events say. A child whose parent never
//! says how it ended is unknown once its session has ended (D-06).
//!
//! Route: the rollout files, not the app-server's `thread/list` /
//! `thread/read`. Those read the same rollouts but need a Codex process
//! started on the user's `CODEX_HOME` (it writes caches there, and may
//! touch `config.toml`), which a read-only view of another app's session
//! must not do (D-28/D-37).

use std::collections::{BTreeSet, HashMap};
use std::io::{BufRead as _, Read as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use juggler_adapter::subagents::{
    Subagent, SubagentConversations, SubagentDetails, SubagentReader, SubagentScope,
    SubagentStatus, SubagentSupport, SubagentTokens, TokensCounted,
};

use crate::subtasks::{Subtask, SubtaskLog, SubtaskState, SubtaskTail};
use crate::transcript::is_compressed;

/// What the Codex adapter answers.
pub const SUPPORT: SubagentSupport = SubagentSupport::Supported {
    route: "Codex's rollout files: each child thread's session_meta names its parent; the parent's rollout says how each ended (the app-server would start Codex on the user's home, so it isn't used)",
    conversations: SubagentConversations::Read,
    stop_hint: None,
};

/// The most bytes read from a rollout's first line.
const META_MAX: u64 = 1024 * 1024;

/// How often the day folders are looked through again for new children
/// while a child the parent named isn't found.
const RESCAN: Duration = Duration::from_secs(5);

/// The most days after its start a session's children are looked for.
const DAYS_AFTER: i64 = 7;

/// The most children one session is read for.
const CHILDREN_MAX: usize = 2_000;

/// How deep children of children are followed.
const DEPTH_MAX: u32 = 8;

/// The most 16 MiB reads of rollouts one refresh takes (256 MiB).
const READS_PER_REFRESH: usize = 16;

/// A reader for the rollout `scope` names.
#[must_use]
pub fn reader(scope: SubagentScope<'_>) -> Option<Box<dyn SubagentReader>> {
    let name = scope.path.file_name()?.to_string_lossy();
    if !name.starts_with("rollout-") {
        return None;
    }
    Some(Box::new(CodexSubagents::new(scope.path, scope.id)))
}

/// What a child's first line says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Child {
    path: PathBuf,
    parent: String,
    depth: Option<u32>,
    agent_path: Option<String>,
    role: Option<String>,
    nickname: Option<String>,
    started_ms: Option<i64>,
}

/// A rollout, read as it grows (or whole, once, when compressed).
#[derive(Debug, Default)]
enum Source {
    #[default]
    Unread,
    Tail(SubtaskTail),
    Whole(SubtaskLog),
}

impl Source {
    /// True when it has caught up with the file.
    fn read(&mut self, path: &Path) -> std::io::Result<bool> {
        if is_compressed(path) {
            if matches!(self, Self::Unread) {
                let bytes = crate::read_session_file(path)?;
                *self = Self::Whole(crate::subtasks_of(&bytes));
            }
            return Ok(true);
        }
        if !matches!(self, Self::Tail(_)) {
            *self = Self::Tail(SubtaskTail::default());
        }
        match self {
            Self::Tail(tail) => tail.read_some(path, READS_PER_REFRESH),
            _ => Ok(true),
        }
    }

    fn log(&self) -> Option<&SubtaskLog> {
        match self {
            Self::Unread => None,
            Self::Tail(tail) => Some(tail.log()),
            Self::Whole(log) => Some(log),
        }
    }
}

/// One session's subagents, read as its rollouts grow.
#[derive(Debug)]
pub struct CodexSubagents {
    session: PathBuf,
    thread: String,
    /// The session's rollout (key: its thread id) and each child's.
    sources: HashMap<String, Source>,
    /// Children found, by thread id.
    children: HashMap<String, Child>,
    /// Rollouts already looked at (children or not), by path.
    looked: BTreeSet<PathBuf>,
    /// Children whose parent isn't known yet (another session's, or one
    /// of a child not found yet), by thread id.
    waiting: Vec<(String, Child)>,
    scanned: Option<Instant>,
}

impl CodexSubagents {
    /// The subagents of the thread `thread`, whose rollout is `session`.
    #[must_use]
    pub fn new(session: &Path, thread: &str) -> Self {
        Self {
            session: session.to_path_buf(),
            thread: thread.to_owned(),
            sources: HashMap::new(),
            children: HashMap::new(),
            looked: BTreeSet::new(),
            waiting: Vec::new(),
            scanned: None,
        }
    }

    /// The day folders a child may be in: the session's own, and the
    /// days from the day before its start to [`DAYS_AFTER`] after it.
    fn day_folders(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let Some(own) = self.session.parent() else {
            return out;
        };
        out.push(own.to_path_buf());
        // `<root>/YYYY/MM/DD/rollout-…`: the tree's root and the day.
        let parts: Option<(i64, u32, u32)> = (|| {
            let day = own.file_name()?.to_str()?.parse().ok()?;
            let month_dir = own.parent()?;
            let month = month_dir.file_name()?.to_str()?.parse().ok()?;
            let year = month_dir.parent()?.file_name()?.to_str()?.parse().ok()?;
            Some((year, month, day))
        })();
        let root = own.parent().and_then(Path::parent).and_then(Path::parent);
        let (Some((year, month, day)), Some(root)) = (parts, root) else {
            return out;
        };
        let start = days_from_civil(year, month, day);
        for days in (start - 1)..=(start + DAYS_AFTER) {
            let (y, m, d) = civil_from_days(days);
            let folder = root
                .join(format!("{y:04}"))
                .join(format!("{m:02}"))
                .join(format!("{d:02}"));
            if folder != own {
                out.push(folder);
            }
        }
        out
    }

    /// Looks through the day folders for rollouts that are children of
    /// this session or of a child already found.
    fn scan(&mut self) {
        let mut found: Vec<Child> = Vec::new();
        for folder in self.day_folders() {
            let Ok(entries) = std::fs::read_dir(&folder) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.starts_with("rollout-")
                    || path == self.session
                    || self.looked.contains(&path)
                {
                    continue;
                }
                // A child's first line is written when it starts: one not
                // there yet is looked at again next time.
                let Some(meta) = first_line(&path) else {
                    continue;
                };
                self.looked.insert(path.clone());
                if let Some(child) = child_of(&meta, &path) {
                    found.push(child);
                }
            }
        }
        // Children of this session, then theirs (to the depth kept).
        let mut known: BTreeSet<String> = self.children.keys().cloned().collect();
        known.insert(self.thread.clone());
        let mut pending: Vec<(String, Child)> = std::mem::take(&mut self.waiting);
        pending.extend(
            found
                .into_iter()
                .filter_map(|child| Some((thread_of(&child.path)?, child))),
        );
        loop {
            let before = pending.len();
            pending.retain(|(thread, child)| {
                if self.children.len() >= CHILDREN_MAX
                    || !known.contains(&child.parent)
                    || child.depth.is_some_and(|depth| depth > DEPTH_MAX)
                {
                    return true;
                }
                known.insert(thread.clone());
                self.children.insert(thread.clone(), child.clone());
                false
            });
            if pending.len() == before {
                break;
            }
        }
        // Not this session's (yet): kept, never read from disk again.
        self.waiting = pending;
        self.scanned = Some(Instant::now());
    }

    fn unresolved(&self) -> bool {
        self.sources.values().filter_map(Source::log).any(|log| {
            log.subtasks()
                .iter()
                .any(|item| self.child_for(item).is_none() && item.state != SubtaskState::Failed)
        })
    }

    /// AJ-E1520 (0.3): what a child's start and its own rollout say: its
    /// prompt and result, and (exact, apart from its parent's) its tokens
    /// and model.
    fn details_of(&self, item: Option<&Subtask>, thread: Option<&str>) -> SubagentDetails {
        let own = thread
            .and_then(|thread| self.sources.get(thread))
            .and_then(Source::log);
        SubagentDetails {
            prompt: item.and_then(|item| item.prompt.clone()),
            result: item.and_then(|item| item.result.clone()),
            failure: item.and_then(|item| item.failure.clone()),
            asked: item.and_then(|item| item.asked.clone()),
            model: own.and_then(|log| log.own_model().map(str::to_owned)),
            tokens: own.and_then(SubtaskLog::own_tokens),
            ..SubagentDetails::default()
        }
    }

    /// The child a start names (by thread id, or by its path).
    fn child_for(&self, item: &Subtask) -> Option<(&String, &Child)> {
        let agent = item.agent.as_deref()?;
        if let Some((thread, child)) = self.children.get_key_value(agent) {
            return Some((thread, child));
        }
        self.children
            .iter()
            .find(|(_, child)| child.agent_path.as_deref() == Some(agent))
    }
}

impl SubagentReader for CodexSubagents {
    fn refresh(&mut self) -> std::io::Result<bool> {
        let session = self.session.clone();
        let mut caught_up = self
            .sources
            .entry(self.thread.clone())
            .or_default()
            .read(&session)?;
        let due = self
            .scanned
            .is_none_or(|at| at.elapsed() >= RESCAN && self.unresolved());
        if due {
            self.scan();
        }
        let children: Vec<(String, PathBuf)> = self
            .children
            .iter()
            .map(|(thread, child)| (thread.clone(), child.path.clone()))
            .collect();
        for (thread, path) in children {
            // A child's rollout gone or unreadable: its own children stand
            // as their files say.
            caught_up &= self
                .sources
                .entry(thread)
                .or_default()
                .read(&path)
                .unwrap_or(true);
        }
        Ok(caught_up)
    }

    fn session_tokens(&self) -> Option<SubagentTokens> {
        self.sources
            .get(&self.thread)
            .and_then(Source::log)
            .and_then(SubtaskLog::own_tokens)
    }

    fn tokens_counted(&self) -> TokensCounted {
        TokensCounted::Separate
    }

    fn subagents(&self) -> Vec<Subagent> {
        let mut out: Vec<Subagent> = Vec::new();
        let mut listed: BTreeSet<&str> = BTreeSet::new();
        for (spawner, source) in &self.sources {
            let Some(log) = source.log() else {
                continue;
            };
            let parent = (spawner != &self.thread).then(|| spawner.clone());
            for item in log.subtasks() {
                let child = self.child_for(item);
                if let Some((thread, _)) = child {
                    listed.insert(thread.as_str());
                }
                let status = match item.state {
                    SubtaskState::Running | SubtaskState::Background => SubagentStatus::Running,
                    SubtaskState::Done => SubagentStatus::Done,
                    SubtaskState::Failed => SubagentStatus::Failed,
                    SubtaskState::Stopped => SubagentStatus::Stopped,
                };
                let id = child
                    .map(|(thread, _)| thread.clone())
                    .or_else(|| item.agent.clone())
                    .unwrap_or_else(|| item.id.clone());
                out.push(Subagent {
                    id,
                    kind: item
                        .kind
                        .clone()
                        .or_else(|| child.and_then(|(_, child)| child.role.clone()))
                        .or_else(|| child.and_then(|(_, child)| child.nickname.clone())),
                    task: Some(item.task.clone()).filter(|task| !task.is_empty()),
                    status,
                    started_ms: item
                        .started_ms
                        .or_else(|| child.and_then(|(_, child)| child.started_ms)),
                    ended_ms: if status.is_final() {
                        item.ended_ms
                    } else {
                        None
                    },
                    parent: parent.clone(),
                    depth: child.and_then(|(_, child)| child.depth).unwrap_or(0),
                    background: None,
                    transcript: child.map(|(_, child)| child.path.clone()),
                    details: self.details_of(Some(item), child.map(|(thread, _)| thread.as_str())),
                });
            }
        }
        // Children whose start their parent's rollout doesn't hold.
        for (thread, child) in &self.children {
            if listed.contains(thread.as_str()) {
                continue;
            }
            out.push(Subagent {
                id: thread.clone(),
                kind: child.role.clone().or_else(|| child.nickname.clone()),
                task: None,
                status: SubagentStatus::Unknown,
                started_ms: child.started_ms,
                ended_ms: None,
                parent: (child.parent != self.thread).then(|| child.parent.clone()),
                depth: child.depth.unwrap_or(0),
                background: None,
                transcript: Some(child.path.clone()),
                details: self.details_of(None, Some(thread)),
            });
        }
        crate::subtasks::fill_depths(&mut out);
        out
    }
}

/// A rollout's thread id: the end of its name
/// (`rollout-<time>-<thread id>.jsonl[.zst]`).
fn thread_of(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let stem = name.strip_suffix(".zst").unwrap_or(name);
    let stem = stem.strip_suffix(".jsonl")?;
    // A UUID: the last five dash-separated groups.
    let parts: Vec<&str> = stem.rsplitn(6, '-').collect();
    if parts.len() < 6 {
        return None;
    }
    let mut groups: Vec<&str> = parts[..5].to_vec();
    groups.reverse();
    Some(groups.join("-"))
}

/// The first line of a rollout, as JSON (at most [`META_MAX`] bytes).
fn first_line(path: &Path) -> Option<serde_json::Value> {
    let file = std::fs::File::open(path).ok()?;
    let mut line = Vec::new();
    if is_compressed(path) {
        let decoder = zstd::stream::read::Decoder::new(file).ok()?;
        std::io::BufReader::new(decoder.take(META_MAX))
            .read_until(b'\n', &mut line)
            .ok()?;
    } else {
        std::io::BufReader::new(file.take(META_MAX))
            .read_until(b'\n', &mut line)
            .ok()?;
    }
    if line.last() != Some(&b'\n') {
        return None;
    }
    serde_json::from_slice(&line).ok()
}

/// What a `session_meta` line says about a child thread, when it is one.
fn child_of(meta: &serde_json::Value, path: &Path) -> Option<Child> {
    if meta.get("type")?.as_str()? != "session_meta" {
        return None;
    }
    let payload = meta.get("payload")?;
    let spawn = payload
        .get("source")?
        .get("subagent")?
        .get("thread_spawn")?;
    let text = |value: &serde_json::Value, key: &str| {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    Some(Child {
        path: path.to_path_buf(),
        parent: text(spawn, "parent_thread_id")?,
        depth: spawn
            .get("depth")
            .and_then(serde_json::Value::as_u64)
            .and_then(|depth| u32::try_from(depth).ok()),
        agent_path: text(spawn, "agent_path").or_else(|| text(payload, "agent_path")),
        role: text(spawn, "agent_role").or_else(|| text(payload, "agent_role")),
        nickname: text(spawn, "agent_nickname").or_else(|| text(payload, "agent_nickname")),
        started_ms: text(payload, "timestamp")
            .or_else(|| text(meta, "timestamp"))
            .as_deref()
            .and_then(juggler_core::time::timestamp_ms),
    })
}

/// Days since 1970-01-01 of a civil date (proleptic Gregorian).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    juggler_core::renewal::CalendarDate { year, month, day }.civil_days()
}

/// The civil date `days` after 1970-01-01 (H. Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    // Both are in 1..=31 by construction.
    (
        year,
        u32::try_from(month).unwrap_or(1),
        u32::try_from(day).unwrap_or(1),
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    const PARENT: &str = "019a0000-0000-7000-8000-000000000001";
    const KID: &str = "019a0000-0000-7000-8000-0000000000a1";
    const GRANDKID: &str = "019a0000-0000-7000-8000-0000000000b1";
    const LOST: &str = "019a0000-0000-7000-8000-0000000000c1";

    fn rollout(root: &Path, day: &str, thread: &str, lines: &[serde_json::Value]) -> PathBuf {
        let path = root
            .join(day)
            .join(format!("rollout-2026-10-27T10-00-00-{thread}.jsonl"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let body: Vec<String> = lines.iter().map(ToString::to_string).collect();
        std::fs::write(&path, body.join("\n") + "\n").unwrap();
        path
    }

    fn meta(thread: &str, parent: Option<(&str, u32, &str, &str)>, at: &str) -> serde_json::Value {
        let source = match parent {
            Some((parent, depth, path, role)) => serde_json::json!({"subagent": {"thread_spawn": {
                "parent_thread_id": parent, "depth": depth, "agent_path": path,
                "agent_nickname": "Nick", "agent_role": role}}}),
            None => serde_json::json!("cli"),
        };
        serde_json::json!({"timestamp": at, "type": "session_meta",
            "payload": {"id": thread, "timestamp": at, "source": source}})
    }

    fn call(name: &str, call: &str, arguments: &serde_json::Value, at: &str) -> serde_json::Value {
        serde_json::json!({"timestamp": at, "type": "response_item", "payload": {"type": "function_call",
            "name": name, "call_id": call, "arguments": arguments.to_string()}})
    }

    fn output(call: &str, output: &serde_json::Value, at: &str) -> serde_json::Value {
        serde_json::json!({"timestamp": at, "type": "response_item", "payload": {"type": "function_call_output",
            "call_id": call, "output": output.to_string()}})
    }

    fn activity(path: &str, thread: &str, kind: &str, at: &str) -> serde_json::Value {
        serde_json::json!({"timestamp": at, "type": "event_msg", "payload": {"type": "item_completed",
            "item": {"type": "SubAgentActivity", "agent_path": path, "agent_thread_id": thread, "kind": kind}}})
    }

    fn by_id<'a>(list: &'a [Subagent], id: &str) -> &'a Subagent {
        list.iter()
            .find(|agent| agent.id == id)
            .expect("a child the fixture holds is listed")
    }

    #[test]
    fn children_are_found_by_their_parent_and_the_parent_says_how_each_ended() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("sessions");
        let session = rollout(
            &root,
            "2026/10/27",
            PARENT,
            &[
                meta(PARENT, None, "2026-10-27T10:00:00Z"),
                call(
                    "spawn_agent",
                    "c1",
                    &serde_json::json!({"message": "Explore the parser", "agent_type": "explorer", "task_name": "parser"}),
                    "2026-10-27T10:01:00Z",
                ),
                output(
                    "c1",
                    &serde_json::json!({"task_name": "/root/parser"}),
                    "2026-10-27T10:01:01Z",
                ),
                activity("/root/parser", KID, "started", "2026-10-27T10:01:02Z"),
                call(
                    "spawn_agent",
                    "c2",
                    &serde_json::json!({"message": "Check the docs", "agent_type": "validator", "task_name": "docs"}),
                    "2026-10-27T10:02:00Z",
                ),
                output(
                    "c2",
                    &serde_json::json!({"task_name": "/root/docs"}),
                    "2026-10-27T10:02:01Z",
                ),
                call(
                    "list_agents",
                    "l1",
                    &serde_json::json!({}),
                    "2026-10-27T10:05:00Z",
                ),
                output(
                    "l1",
                    &serde_json::json!({"agents": [
                    {"agent_name": "/root/docs", "agent_status": {"errored": "boom"}},
                    {"agent_name": "/root/parser", "agent_status": "running"}]}),
                    "2026-10-27T10:05:01Z",
                ),
            ],
        );
        // The child started the next day (past midnight), and one of its own.
        rollout(
            &root,
            "2026/10/28",
            KID,
            &[
                meta(
                    KID,
                    Some((PARENT, 1, "/root/parser", "explorer")),
                    "2026-10-28T00:01:00Z",
                ),
                call(
                    "spawn_agent",
                    "g1",
                    &serde_json::json!({"message": "Deeper", "agent_type": "explorer"}),
                    "2026-10-28T00:02:00Z",
                ),
                output(
                    "g1",
                    &serde_json::json!({"task_name": "/root/parser/deeper"}),
                    "2026-10-28T00:02:01Z",
                ),
                activity(
                    "/root/parser/deeper",
                    GRANDKID,
                    "completed",
                    "2026-10-28T00:03:00Z",
                ),
            ],
        );
        rollout(
            &root,
            "2026/10/28",
            GRANDKID,
            &[meta(
                GRANDKID,
                Some((KID, 2, "/root/parser/deeper", "explorer")),
                "2026-10-28T00:02:00Z",
            )],
        );
        // A child whose start the parent doesn't hold: unknown.
        rollout(
            &root,
            "2026/10/27",
            LOST,
            &[meta(
                LOST,
                Some((PARENT, 1, "/root/lost", "default")),
                "2026-10-27T11:00:00Z",
            )],
        );
        // Another session's child, and a session of its own: never listed.
        rollout(
            &root,
            "2026/10/27",
            "019a0000-0000-7000-8000-0000000000d1",
            &[meta(
                "019a0000-0000-7000-8000-0000000000d1",
                Some((
                    "019a0000-0000-7000-8000-00000000ffff",
                    1,
                    "/root/x",
                    "default",
                )),
                "2026-10-27T11:00:00Z",
            )],
        );

        let mut reader = CodexSubagents::new(&session, PARENT);
        reader.refresh().unwrap();
        let list = reader.subagents();
        assert_eq!(list.len(), 4);
        let kid = by_id(&list, KID);
        assert_eq!(kid.status, SubagentStatus::Running);
        assert_eq!(kid.kind.as_deref(), Some("explorer"));
        assert_eq!(kid.task.as_deref(), Some("Explore the parser"));
        assert_eq!(kid.depth, 1);
        assert_eq!(kid.parent, None);
        assert!(
            kid.transcript
                .as_ref()
                .unwrap()
                .to_string_lossy()
                .contains(KID)
        );
        // Named by its path only (no activity linked its thread): its
        // status is still the parent's word.
        let docs = by_id(&list, "/root/docs");
        assert_eq!(docs.status, SubagentStatus::Failed);
        assert_eq!(docs.transcript, None);
        assert_eq!(
            docs.ended_ms,
            juggler_core::time::timestamp_ms("2026-10-27T10:05:01Z")
        );
        let grandkid = by_id(&list, GRANDKID);
        assert_eq!(grandkid.status, SubagentStatus::Done);
        assert_eq!(grandkid.parent.as_deref(), Some(KID));
        assert_eq!(grandkid.depth, 2);
        let lost = by_id(&list, LOST);
        assert_eq!(lost.status, SubagentStatus::Unknown);
        assert_eq!(lost.kind.as_deref(), Some("default"));
    }

    #[test]
    fn the_first_shape_names_agents_by_thread_and_says_how_each_ended() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("sessions");
        let session = rollout(
            &root,
            "2026/10/27",
            PARENT,
            &[
                meta(PARENT, None, "2026-10-27T10:00:00Z"),
                call(
                    "spawn_agent",
                    "c1",
                    &serde_json::json!({"message": "One", "agent_type": "worker"}),
                    "2026-10-27T10:01:00Z",
                ),
                output(
                    "c1",
                    &serde_json::json!({"agent_id": KID, "nickname": "n"}),
                    "2026-10-27T10:01:01Z",
                ),
                call(
                    "close_agent",
                    "x1",
                    &serde_json::json!({"target": KID}),
                    "2026-10-27T10:03:00Z",
                ),
                output(
                    "x1",
                    &serde_json::json!({"previous_status": {"completed": "ok"}}),
                    "2026-10-27T10:03:01Z",
                ),
            ],
        );
        rollout(
            &root,
            "2026/10/27",
            KID,
            &[meta(
                KID,
                Some((PARENT, 1, "", "worker")),
                "2026-10-27T10:01:00Z",
            )],
        );
        let mut reader = CodexSubagents::new(&session, PARENT);
        reader.refresh().unwrap();
        let list = reader.subagents();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, KID);
        assert_eq!(list[0].status, SubagentStatus::Done);
        assert!(list[0].transcript.is_some());
    }

    #[test]
    fn an_interrupted_one_is_stopped_and_dates_wrap_months() {
        let mut log = SubtaskLog::default();
        let lines = [
            call(
                "spawn_agent",
                "c1",
                &serde_json::json!({"message": "One"}),
                "2026-10-27T10:01:00Z",
            ),
            output(
                "c1",
                &serde_json::json!({"task_name": "/root/one"}),
                "2026-10-27T10:01:01Z",
            ),
            call(
                "interrupt_agent",
                "i1",
                &serde_json::json!({"target": "/root/one"}),
                "2026-10-27T10:02:00Z",
            ),
            output(
                "i1",
                &serde_json::json!({"previous_status": "running"}),
                "2026-10-27T10:02:01Z",
            ),
        ];
        let body: Vec<String> = std::iter::once(meta(PARENT, None, "2026-10-27T10:00:00Z"))
            .chain(lines)
            .map(|line| line.to_string())
            .collect();
        log.feed((body.join("\n") + "\n").as_bytes());
        assert_eq!(log.subtasks()[0].state, SubtaskState::Stopped);
        assert_eq!(
            civil_from_days(days_from_civil(2026, 10, 31) + 1),
            (2026, 11, 1)
        );
        assert_eq!(
            civil_from_days(days_from_civil(2024, 2, 28) + 1),
            (2024, 2, 29)
        );
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(
            thread_of(Path::new(&format!(
                "rollout-2026-10-27T10-00-00-{KID}.jsonl.zst"
            )))
            .as_deref(),
            Some(KID)
        );
    }

    #[test]
    fn a_childs_tokens_and_model_are_its_own_threads_apart_from_its_parent() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("sessions");
        let usage = |input: u64, cached: u64, output: u64, at: &str| {
            serde_json::json!({"timestamp": at, "type": "event_msg", "payload": {"type": "token_count",
                "info": {"total_token_usage": {"input_tokens": input, "cached_input_tokens": cached,
                    "output_tokens": output, "reasoning_output_tokens": 0, "total_tokens": input + output}}}})
        };
        let session = rollout(
            &root,
            "2026/10/27",
            PARENT,
            &[
                meta(PARENT, None, "2026-10-27T10:00:00Z"),
                usage(500, 100, 50, "2026-10-27T10:00:30Z"),
                call(
                    "spawn_agent",
                    "c1",
                    &serde_json::json!({"message": "Explore the parser in depth", "agent_type": "explorer"}),
                    "2026-10-27T10:01:00Z",
                ),
                output(
                    "c1",
                    &serde_json::json!({"agent_id": KID}),
                    "2026-10-27T10:01:01Z",
                ),
            ],
        );
        rollout(
            &root,
            "2026/10/27",
            KID,
            &[
                meta(
                    KID,
                    Some((PARENT, 1, "/root/parser", "explorer")),
                    "2026-10-27T10:01:02Z",
                ),
                serde_json::json!({"timestamp": "2026-10-27T10:01:03Z", "type": "turn_context", "payload": {"model": "gpt-5-codex"}}),
                usage(1_000, 400, 80, "2026-10-27T10:02:00Z"),
                usage(3_000, 2_000, 120, "2026-10-27T10:03:00Z"),
            ],
        );
        let mut reader = CodexSubagents::new(&session, PARENT);
        reader.refresh().unwrap();
        let list = reader.subagents();
        let kid = by_id(&list, KID);
        assert_eq!(kid.details.model.as_deref(), Some("gpt-5-codex"));
        let tokens = kid.details.tokens.expect("its own thread's total");
        assert_eq!(
            (tokens.input, tokens.cache_read, tokens.output),
            (1_000, 2_000, 120)
        );
        assert_eq!(
            kid.details.prompt.as_deref(),
            Some("Explore the parser in depth")
        );
        assert_eq!(reader.tokens_counted(), TokensCounted::Separate);
        assert_eq!(
            reader.session_tokens().map(SubagentTokens::total),
            Some(550)
        );
    }
}
