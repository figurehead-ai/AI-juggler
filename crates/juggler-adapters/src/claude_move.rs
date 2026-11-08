//! ADR-0031 (D-93): moving a Claude conversation and its whole family to
//! another account: Claude app profile to profile, or Claude Code folder
//! to folder. This module only **plans** and **checks**; AI Juggler's
//! engine (`juggler-handover::moving`) applies the plan with backups, a
//! journal and Undo.
//!
//! What ties a saved conversation to an account (read on this Mac,
//! 2026-11-08; the research note is `docs/research/claude-move-switch.md`):
//! - its history and everything beside it live in the account's Claude
//!   Code folder: `projects/<project>/<id>.jsonl`, the `<id>/` folder
//!   (nested agents in `subagents/`, out-of-line outputs in
//!   `tool-results/`), `file-history/<id>/` (rewind) and `tasks/<id>/`;
//! - the Claude app lists it from its record,
//!   `<instance>/claude-code-sessions/<account>/<org>/local_<…>.json`,
//!   under the account the profile is signed in to. The record's own id
//!   is **not** the conversation's id (`cliSessionId`); a family hangs
//!   together by records' `spawnedFrom.sessionId` and
//!   `forkedFromSessionId`, which name records.
//!
//! So a move renames the family's files into the target's Claude Code
//! folder (one disk: same files, same inodes), moves each record into the
//! target's account folder, re-links any older record the target already
//! had of the same conversation (an alias) and its children to the moved
//! one, and puts aside what it replaces. It never edits a history's text,
//! a sign-in, a token or a cookie. Out-of-line outputs stay where they are:
//! old messages name them by their full path.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use juggler_adapter::error::AdapterError;
use juggler_adapter::moving::{
    Appended, HoldWhy, InPlaceMove, MemberSide, MoveCheck, MoveMember, MovePlan, MoveRequest,
    MoveStep, NotesChoice, Retained,
};

/// The prefix of every app record's id and file name.
const LOCAL: &str = "local_";
/// A record is a few kilobytes; one past this isn't read.
const RECORD_MAX: u64 = 4 * 1024 * 1024;
/// How deep a family is followed.
const MAX_DEPTH: usize = 32;
/// The Claude app versions (major) whose records and launch were proven
/// for a move (2026-11-08, the maintainer's case and the aij-test walk).
const PROVEN_APP_MAJORS: &[&str] = &["4"];
/// The Claude Code versions (major) whose folders were proven.
const PROVEN_CODE_MAJORS: &[&str] = &["2"];
/// Fields that belong to the account a record was made under (as the
/// copy's family carry drops them).
const ACCOUNT_BOUND: [&str; 6] = [
    "publishedArtifacts",
    "remoteMcpServersConfig",
    "peerInbound",
    "peerReceipts",
    "peerMessaged",
    "steeredByRemoteClient",
];
/// A moved record's runtime state of its last run, which the app makes
/// again (the record's backup keeps the originals; the saved error cards in
/// the history stay).
const STALE_RUNTIME: [&str; 5] = [
    "error",
    "errorAt",
    "errorRows",
    "priorErrorMark",
    "lastTurnCutShort",
];
/// Snapshots the app regenerates when a record's parent changed.
const ROUTING_SNAPSHOTS: [&str; 2] = ["promptAppendSnapshot", "toolSurfaceSnapshot"];
/// The structured keys that name another record (never message text).
const REFERENCE_KEYS: [&str; 7] = [
    "sessionId",
    "targetSessionId",
    "sourceSessionId",
    "parentSessionId",
    "rootSessionId",
    "forkedFromSessionId",
    "spawnedFromSessionId",
];
/// The session folder's entry that stays where it is (outputs named by
/// their full path in old messages).
const OUTPUTS: &str = "tool-results";
/// The account-level folders a session keeps a folder of its own in.
const SESSION_AREAS: [&str; 2] = ["file-history", "tasks"];

/// The Claude adapter's move planner.
pub struct ClaudeMove;

/// One app record, read whole.
#[derive(Clone, Debug)]
struct Record {
    path: PathBuf,
    id: String,
    conversation: String,
    value: serde_json::Value,
}

impl Record {
    fn parents(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(id) = self
            .value
            .get("spawnedFrom")
            .and_then(|p| p.get("sessionId"))
            .and_then(serde_json::Value::as_str)
        {
            out.push(id.to_owned());
        }
        if let Some(id) = self
            .value
            .get("forkedFromSessionId")
            .and_then(serde_json::Value::as_str)
        {
            out.push(id.to_owned());
        }
        out
    }

    fn archived(&self) -> bool {
        self.value.get("isArchived").and_then(serde_json::Value::as_bool) == Some(true)
    }

    fn title(&self) -> Option<String> {
        self.value
            .get("title")
            .and_then(serde_json::Value::as_str)
            .filter(|t| !t.trim().is_empty())
            .map(str::to_owned)
    }

    fn cwd(&self) -> Option<PathBuf> {
        self.value
            .get("cwd")
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from)
    }

    fn activity(&self) -> i64 {
        self.value
            .get("lastActivityAt")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0)
    }
}

/// Every record of an app instance (every account folder), by id; records
/// of an unknown shape are listed in `odd`.
fn records(instance: &Path, odd: &mut Vec<PathBuf>) -> BTreeMap<String, Record> {
    let mut out = BTreeMap::new();
    for path in crate::claude_live::app_files(instance) {
        let value = std::fs::symlink_metadata(&path)
            .ok()
            .filter(|meta| meta.is_file() && meta.len() <= RECORD_MAX)
            .and_then(|_| std::fs::read(&path).ok())
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned();
        let shaped = value.as_ref().and_then(|value| {
            let id = value.get("sessionId")?.as_str()?;
            let conversation = value.get("cliSessionId")?.as_str()?;
            (id == stem && id.starts_with(LOCAL)).then(|| (id.to_owned(), conversation.to_owned()))
        });
        match (shaped, value) {
            (Some((id, conversation)), Some(value)) => {
                out.insert(
                    id.clone(),
                    Record {
                        path,
                        id,
                        conversation,
                        value,
                    },
                );
            }
            _ => odd.push(path),
        }
    }
    out
}

/// The records reached from `roots` (record ids) by `spawnedFrom` and
/// `forkedFromSessionId`, recursively, each once (cycle-safe).
fn descendants(all: &BTreeMap<String, Record>, roots: &BTreeSet<String>) -> BTreeSet<String> {
    let mut found = roots.clone();
    for _ in 0..MAX_DEPTH {
        let next: BTreeSet<String> = all
            .values()
            .filter(|record| !found.contains(&record.id))
            .filter(|record| record.parents().iter().any(|p| found.contains(p)))
            .map(|record| record.id.clone())
            .collect();
        if next.is_empty() {
            break;
        }
        found.extend(next);
    }
    found
}

/// Where a conversation's history is in a Claude Code folder: every
/// `projects/*/<id>.jsonl` (a worktree has a project folder of its own).
fn histories(tool_dir: &Path, conversation: &str) -> Vec<PathBuf> {
    let name = format!("{conversation}.jsonl");
    let Ok(projects) = std::fs::read_dir(tool_dir.join("projects")) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = projects
        .flatten()
        .map(|entry| entry.path().join(&name))
        .filter(|path| std::fs::symlink_metadata(path).is_ok())
        .collect();
    out.sort();
    out
}

/// The account folder the target app lists its sessions under: the one
/// its Claude Code folder is signed in to, else its only one.
fn target_records_folder(request: &MoveRequest, host: &Path) -> Result<PathBuf, String> {
    let root = crate::claude_live::app_root(host);
    if let Some(sign_in) =
        crate::claude_limits::profile_limits(&request.target.tool_dir).and_then(|l| l.sign_in)
    {
        let folder = root.join(&sign_in.account).join(&sign_in.org);
        return if folder.is_dir() {
            Ok(folder)
        } else {
            Err("the target profile has no session folder for its account yet: open it once, then switch".to_owned())
        };
    }
    let found = crate::claude_limits::instance_sign_ins(host);
    match found.as_slice() {
        [one] => Ok(root.join(&one.account).join(&one.org)),
        [] => Err("the target profile isn't signed in to an account yet".to_owned()),
        _ => Err("the target profile has sessions of more than one account and none is signed in, so AI Juggler can't tell where they go".to_owned()),
    }
}

/// Whether `version`'s major is one of `proven`.
fn proven(version: Option<&str>, proven: &[&str]) -> bool {
    version
        .and_then(|v| v.trim().split('.').next())
        .is_some_and(|major| proven.contains(&major))
}

/// The plan as it is built.
struct Builder<'a> {
    request: &'a MoveRequest,
    plan: MovePlan,
    made: BTreeSet<PathBuf>,
}

impl Builder<'_> {
    fn refuse(&mut self, why: impl Into<String>) {
        let why = why.into();
        if !self.plan.refusals.contains(&why) {
            self.plan.refusals.push(why);
        }
    }

    fn note(&mut self, what: impl Into<String>) {
        let what = what.into();
        if !self.plan.notes.contains(&what) {
            self.plan.notes.push(what);
        }
    }

    /// A folder for moved things, made once (with its parents inside the
    /// target's roots).
    fn folder(&mut self, dir: &Path) {
        let mut missing = Vec::new();
        for at in dir.ancestors() {
            if at == self.request.target.tool_dir || at.exists() || self.made.contains(at) {
                break;
            }
            missing.push(at.to_path_buf());
        }
        for path in missing.into_iter().rev() {
            self.made.insert(path.clone());
            self.plan.steps.push(MoveStep::MakeDir { path });
        }
    }

    /// Moves `from` to `to`, putting aside what is at `to` first.
    fn carry(&mut self, from: &Path, to: &Path, history: bool, held: HoldWhy) {
        let Ok(meta) = std::fs::symlink_metadata(from) else {
            return;
        };
        if meta.file_type().is_symlink() {
            self.refuse(format!(
                "{} is a link, and a move never follows or recreates links",
                shown(from)
            ));
            return;
        }
        if meta.is_dir()
            && let Err(error) = juggler_adapter::transfer::tree(from)
        {
            self.refuse(format!("{} can't be moved: {error}", shown(from)));
            return;
        }
        if meta.is_file() && juggler_adapter::transfer::link_count(from).unwrap_or(1) > 1 {
            self.refuse(format!(
                "{} has a second name (a hard link), and the app refuses such histories",
                shown(from)
            ));
            return;
        }
        if std::fs::symlink_metadata(to).is_ok() {
            self.plan.steps.push(MoveStep::Hold {
                path: to.to_path_buf(),
                why: held,
            });
        }
        if let Some(parent) = to.parent() {
            self.folder(parent);
        }
        self.plan.steps.push(MoveStep::Rename {
            from: from.to_path_buf(),
            to: to.to_path_buf(),
            history,
        });
    }
}

/// `path` without the home folder, for words.
fn shown(path: &Path) -> String {
    let home = crate::inventory_files::user_home();
    match home.as_deref().and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// The bytes of a folder or file (links not followed).
fn bytes_of(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    juggler_adapter::transfer::tree(path)
        .map(|(files, _)| {
            files
                .iter()
                .filter_map(|f| std::fs::symlink_metadata(path.join(f)).ok())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0)
}

/// How many nested agents' histories a session folder holds.
fn subagents_in(dir: &Path) -> usize {
    juggler_adapter::transfer::tree(&dir.join("subagents"))
        .map(|(files, _)| {
            files
                .iter()
                .filter(|f| f.extension().is_some_and(|e| e == "jsonl"))
                .count()
        })
        .unwrap_or(0)
}

/// Whether two folders hold the same files with the same bytes.
fn same_tree(a: &Path, b: &Path) -> bool {
    let (Ok((fa, _)), Ok((fb, _))) = (
        juggler_adapter::transfer::tree(a),
        juggler_adapter::transfer::tree(b),
    ) else {
        return false;
    };
    fa == fb
        && fa
            .iter()
            .all(|f| std::fs::read(a.join(f)).ok() == std::fs::read(b.join(f)).ok())
}

/// `record` as it goes to the target: the account-bound and stale runtime
/// fields out, its parent links pointed at `parent_of`'s answer.
fn moved_record(
    record: &Record,
    rebind: &HashMap<String, String>,
    root: bool,
    family: &BTreeSet<String>,
) -> serde_json::Value {
    let mut value = record.value.clone();
    if let Some(object) = value.as_object_mut() {
        for key in ACCOUNT_BOUND.iter().chain(STALE_RUNTIME.iter()) {
            object.remove(*key);
        }
        object.retain(|key, _| !key.starts_with("remoteControl"));
        // The moved conversation's own parent stays behind: on the target
        // it stands at the top.
        if root
            && !record
                .parents()
                .iter()
                .any(|p| family.contains(p) || rebind.contains_key(p))
        {
            object.remove("spawnedFrom");
            object.remove("forkedFromSessionId");
        }
    }
    if rebound(&mut value, rebind)
        && let Some(object) = value.as_object_mut()
    {
        for key in ROUTING_SNAPSHOTS {
            object.remove(key);
        }
    }
    value
}

/// Points every structured reference to an old record id at its new one,
/// at any depth, never the record's own top-level id and never text.
/// Whether anything changed.
fn rebound(value: &mut serde_json::Value, rebind: &HashMap<String, String>) -> bool {
    fn walk(value: &mut serde_json::Value, rebind: &HashMap<String, String>, top: bool) -> bool {
        let mut changed = false;
        match value {
            serde_json::Value::Object(map) => {
                for (key, inner) in map.iter_mut() {
                    if REFERENCE_KEYS.contains(&key.as_str())
                        && !(top && key == "sessionId")
                        && let Some(new) = inner.as_str().and_then(|old| rebind.get(old))
                    {
                        *inner = serde_json::Value::String(new.clone());
                        changed = true;
                    } else if inner.is_object() || inner.is_array() {
                        changed |= walk(inner, rebind, false);
                    }
                }
            }
            serde_json::Value::Array(items) => {
                for inner in items {
                    changed |= walk(inner, rebind, false);
                }
            }
            _ => {}
        }
        changed
    }
    walk(value, rebind, true)
}

impl InPlaceMove for ClaudeMove {
    #[allow(clippy::too_many_lines, reason = "one plan, read top to bottom")]
    fn plan(&self, request: &MoveRequest) -> Result<MovePlan, AdapterError> {
        let source = &request.source;
        let target = &request.target;
        let mut b = Builder {
            request,
            plan: MovePlan {
                conversation: request.conversation.clone(),
                resume: request.conversation.clone(),
                ..MovePlan::default()
            },
            made: BTreeSet::new(),
        };
        for dir in [&source.tool_dir, &target.tool_dir] {
            if !dir.join("projects").is_dir() {
                return Err(AdapterError::AccountDir {
                    path: dir.clone(),
                    message: "this isn't a Claude Code folder with conversations".into(),
                });
            }
        }
        b.plan.roots = [
            Some(source.tool_dir.clone()),
            Some(target.tool_dir.clone()),
            source.host_dir.clone(),
            target.host_dir.clone(),
        ]
        .into_iter()
        .flatten()
        .collect();
        if source.tool_dir == target.tool_dir {
            b.refuse("both accounts use the same Claude Code folder, so there is nothing to move");
        }
        // Gates: one kind of side to the same kind, proven versions, one
        // disk that clones.
        match (&source.host_dir, &target.host_dir) {
            (Some(a), Some(z)) if a == z => {
                b.refuse("both accounts use the same Claude app profile");
            }
            (Some(_), Some(_)) => {
                for side in [source, target] {
                    if !proven(side.version.as_deref(), PROVEN_APP_MAJORS) {
                        b.refuse(format!(
                            "a move between Claude app profiles is proven for version {}.x; this one is {}. Copy still works",
                            PROVEN_APP_MAJORS.join(", "),
                            side.version.as_deref().unwrap_or("of a version AI Juggler couldn't read")
                        ));
                    }
                }
            }
            (None, None) => {
                for side in [source, target] {
                    if !proven(side.version.as_deref(), PROVEN_CODE_MAJORS) {
                        b.refuse(format!(
                            "a move between Claude Code folders is proven for version {}.x; this one is {}. Copy still works",
                            PROVEN_CODE_MAJORS.join(", "),
                            side.version.as_deref().unwrap_or("of a version AI Juggler couldn't read")
                        ));
                    }
                }
            }
            _ => b.refuse(
                "a move between the Claude app and a terminal isn't proven yet: Copy carries it",
            ),
        }
        match juggler_adapter::transfer::same_device(&source.tool_dir, &target.tool_dir) {
            Ok(true) => {}
            Ok(false) => b.refuse(
                "the two accounts' folders are on different disks, and a move never copies gigabytes in its place: Copy carries it",
            ),
            Err(error) => b.refuse(format!("AI Juggler couldn't tell which disk the folders are on: {error}")),
        }
        if !juggler_adapter::transfer::clones_supported(&source.tool_dir).unwrap_or(false) {
            b.refuse(
                "this disk can't make the clone backups a move keeps for Undo (it needs APFS on a Mac)",
            );
        }

        // The family, by app records (an app side) or the one conversation.
        let mut odd = Vec::new();
        let source_records = source
            .host_dir
            .as_deref()
            .map(|dir| records(dir, &mut odd))
            .unwrap_or_default();
        let target_records = target
            .host_dir
            .as_deref()
            .map(|dir| records(dir, &mut odd))
            .unwrap_or_default();
        let claims = |all: &BTreeMap<String, Record>, conversation: &str| -> Vec<String> {
            all.values()
                .filter(|r| r.conversation == conversation)
                .map(|r| r.id.clone())
                .collect()
        };
        // Source: the canonical record (the app's own `local_<id>`, else
        // the one active last) and any alias of it.
        let source_root_ids = claims(&source_records, &request.conversation);
        let canonical = source_root_ids
            .iter()
            .find(|id| **id == format!("{LOCAL}{}", request.conversation))
            .or_else(|| {
                source_root_ids
                    .iter()
                    .max_by_key(|id| source_records[*id].activity())
            })
            .cloned();
        if source.host_dir.is_some() && canonical.is_none() {
            b.refuse("the Claude app has no record of this conversation in the source profile");
        }
        let mut rebind: HashMap<String, String> = HashMap::new();
        let mut family: BTreeSet<String> = BTreeSet::new();
        if let Some(canonical) = &canonical {
            let roots: BTreeSet<String> = source_root_ids.iter().cloned().collect();
            family = descendants(&source_records, &roots);
            for alias in source_root_ids.iter().filter(|id| *id != canonical) {
                rebind.insert(alias.clone(), canonical.clone());
                family.remove(alias);
            }
        }
        // A conversation claimed twice in the source (beyond the root's
        // aliases) can't be told apart.
        let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
        for id in &family {
            let record = &source_records[id];
            if let Some(other) = seen.insert(&record.conversation, id) {
                b.refuse(format!(
                    "two of the source app's records ({other} and {id}) claim one conversation, so AI Juggler can't tell which is current"
                ));
            }
        }
        // The target: older records of the moved conversations (aliases)
        // and what hangs from them.
        let moved_conversations: BTreeSet<String> = if family.is_empty() {
            BTreeSet::from([request.conversation.clone()])
        } else {
            family
                .iter()
                .map(|id| source_records[id].conversation.clone())
                .collect()
        };
        let mut target_aliases: BTreeSet<String> = BTreeSet::new();
        for conversation in &moved_conversations {
            for alias in claims(&target_records, conversation) {
                let now = family
                    .iter()
                    .find(|id| source_records[*id].conversation == *conversation)
                    .cloned();
                if let Some(now) = now {
                    rebind.insert(alias.clone(), now);
                }
                target_aliases.insert(alias);
            }
        }
        // A moved record whose id the target uses for another conversation.
        for id in &family {
            if let Some(there) = target_records.get(id)
                && !target_aliases.contains(id)
            {
                b.refuse(format!(
                    "the target app already has a record {id} of another conversation ({}), so the move would replace it",
                    there.conversation
                ));
            }
        }
        let target_children: BTreeSet<String> = descendants(&target_records, &target_aliases)
            .into_iter()
            .filter(|id| !target_aliases.contains(id))
            .filter(|id| !moved_conversations.contains(&target_records[id].conversation))
            .collect();
        if !odd.is_empty() {
            b.note(format!(
                "{} app record(s) of a shape AI Juggler doesn't know were left as they are",
                odd.len()
            ));
        }

        // The payload, member by member: the moved conversation first,
        // then parents before children.
        let mut order: Vec<Option<String>> = Vec::new();
        if let Some(canonical) = &canonical {
            order.push(Some(canonical.clone()));
            let mut rest: Vec<&String> = family.iter().filter(|id| *id != canonical).collect();
            rest.sort_by_key(|id| depth(&source_records, id, canonical));
            order.extend(rest.into_iter().map(|id| Some(id.clone())));
        } else {
            order.push(None);
        }
        for record_id in &order {
            let record = record_id.as_ref().map(|id| &source_records[id]);
            let conversation = record.map_or(request.conversation.clone(), |r| r.conversation.clone());
            let root = conversation == request.conversation;
            let found = histories(&source.tool_dir, &conversation);
            let mut member = MoveMember {
                record_id: record_id.clone(),
                conversation: conversation.clone(),
                parent: record.and_then(|r| r.parents().into_iter().next()),
                title: record.and_then(Record::title),
                archived: record.is_some_and(Record::archived),
                side: MemberSide::Source,
                transcript: None,
                transcript_to: None,
                subagents: 0,
                bytes: 0,
                folder_missing: record
                    .and_then(Record::cwd)
                    .is_some_and(|cwd| !cwd.exists()),
            };
            let name = member.title.clone().unwrap_or_else(|| conversation.clone());
            if member.folder_missing {
                b.note(format!(
                    "“{name}” worked in a folder that's gone; its history moves, and nothing is recreated"
                ));
            }
            match found.as_slice() {
                [] if root => b.refuse("AI Juggler couldn't find this conversation's history in the source account"),
                [] => b.note(format!("“{name}” has no saved history in the source account; its app record moves alone")),
                [history] => {
                    let project = history.parent().map(Path::to_path_buf).unwrap_or_default();
                    let slug = project.file_name().map(PathBuf::from).unwrap_or_default();
                    let to_project = target.tool_dir.join("projects").join(&slug);
                    let to = to_project.join(format!("{conversation}.jsonl"));
                    member.transcript = Some(history.clone());
                    member.transcript_to = Some(to.clone());
                    member.bytes += bytes_of(history);
                    // The target's own version of it is kept aside, whole.
                    if let Ok(theirs) = std::fs::read(&to) {
                        let ours = std::fs::read(history).unwrap_or_default();
                        if ours.starts_with(&theirs) {
                            b.note(format!("the target had an older copy of “{name}”; it is kept aside, and Undo brings it back"));
                        } else {
                            b.note(format!("the target had its own different version of “{name}”; it is kept aside, never joined to this one, and Undo brings it back"));
                        }
                    }
                    b.carry(history, &to, true, HoldWhy::DivergentBranch);
                    // Its folder: everything but its outputs, which old
                    // messages name by their full path.
                    let folder = project.join(&conversation);
                    if folder.is_dir() {
                        member.subagents = subagents_in(&folder);
                        let to_folder = to_project.join(&conversation);
                        let mut entries: Vec<PathBuf> = std::fs::read_dir(&folder)
                            .map(|e| e.flatten().map(|e| e.path()).collect())
                            .unwrap_or_default();
                        entries.sort();
                        for entry in entries {
                            let leaf = entry.file_name().map(PathBuf::from).unwrap_or_default();
                            if leaf.as_os_str() == OUTPUTS {
                                b.plan.retained.push(Retained {
                                    path: entry.clone(),
                                    conversation: conversation.clone(),
                                    why: "old messages name these outputs by their full path".into(),
                                });
                                continue;
                            }
                            member.bytes += bytes_of(&entry);
                            b.carry(&entry, &to_folder.join(&leaf), false, HoldWhy::DivergentBranch);
                        }
                    }
                    // Files beside it named after it (older layouts).
                    let mut beside: Vec<PathBuf> = std::fs::read_dir(&project)
                        .map(|e| e.flatten().map(|e| e.path()).collect())
                        .unwrap_or_default();
                    beside.sort();
                    for path in beside {
                        let leaf = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        if leaf.starts_with(conversation.as_str())
                            && path != *history
                            && path != folder
                            && std::fs::symlink_metadata(&path).is_ok_and(|m| !m.is_dir())
                        {
                            member.bytes += bytes_of(&path);
                            b.carry(&path, &to_project.join(&leaf), false, HoldWhy::DivergentBranch);
                        }
                    }
                }
                many => b.refuse(format!(
                    "“{name}” has {} saved histories in the source account, so AI Juggler can't tell which is current",
                    many.len()
                )),
            }
            for area in SESSION_AREAS {
                let from = source.tool_dir.join(area).join(&conversation);
                if std::fs::symlink_metadata(&from).is_ok() {
                    member.bytes += bytes_of(&from);
                    b.carry(&from, &target.tool_dir.join(area).join(&conversation), false, HoldWhy::DivergentBranch);
                }
            }
            b.plan.members.push(member);
        }

        // The project's shared notes, for the moved conversation's project.
        if let Some(project) = b.plan.members.first().and_then(|m| m.transcript.clone()).and_then(|t| t.parent().map(Path::to_path_buf)) {
            let notes = project.join("memory");
            let slug = project.file_name().map(PathBuf::from).unwrap_or_default();
            let theirs = target.tool_dir.join("projects").join(slug).join("memory");
            if notes.is_dir() {
                if !theirs.exists() {
                    if let Some(parent) = theirs.parent() {
                        b.folder(parent);
                    }
                    b.plan.steps.push(MoveStep::Clone { from: notes, to: theirs });
                } else if !same_tree(&notes, &theirs) {
                    b.plan.notes_conflict = Some(
                        "this project's notes differ in the two accounts; they are shared by every conversation of the project there".into(),
                    );
                    if request.notes == NotesChoice::TakeSource {
                        b.plan.steps.push(MoveStep::Hold { path: theirs.clone(), why: HoldWhy::ReplacedNotes });
                        b.plan.steps.push(MoveStep::Clone { from: notes, to: theirs });
                    }
                }
            }
        }

        // The records: the target's aliases aside, the moved ones placed
        // under the target's account, the target's children re-linked, the
        // source's put away last.
        if let Some(host) = target.host_dir.as_deref() {
            let folder = match target_records_folder(request, host) {
                Ok(folder) => Some(folder),
                Err(why) => {
                    b.refuse(why);
                    None
                }
            };
            for alias in &target_aliases {
                b.plan.steps.push(MoveStep::Hold {
                    path: target_records[alias].path.clone(),
                    why: HoldWhy::DivergentBranch,
                });
                b.note(format!(
                    "the target's older record {alias} of “{}” is kept aside; what was started from it now hangs from the moved one",
                    target_records[alias].title().unwrap_or_else(|| target_records[alias].conversation.clone())
                ));
            }
            if let Some(folder) = folder {
                for id in order.iter().flatten() {
                    let record = &source_records[id];
                    let root = record.conversation == request.conversation;
                    b.plan.steps.push(MoveStep::WriteRecord {
                        path: folder.join(format!("{id}.json")),
                        value: moved_record(record, &rebind, root, &family),
                    });
                }
            }
            for id in &target_children {
                let record = &target_records[id];
                let mut value = record.value.clone();
                if rebound(&mut value, &rebind) {
                    if let Some(object) = value.as_object_mut() {
                        for key in ROUTING_SNAPSHOTS {
                            object.remove(key);
                        }
                    }
                    b.plan.steps.push(MoveStep::WriteRecord { path: record.path.clone(), value });
                }
                b.plan.members.push(MoveMember {
                    record_id: Some(id.clone()),
                    conversation: record.conversation.clone(),
                    parent: record.parents().into_iter().next().map(|p| rebind.get(&p).cloned().unwrap_or(p)),
                    title: record.title(),
                    archived: record.archived(),
                    side: MemberSide::Target,
                    transcript: histories(&target.tool_dir, &record.conversation).into_iter().next(),
                    transcript_to: None,
                    subagents: 0,
                    bytes: 0,
                    folder_missing: record.cwd().is_some_and(|cwd| !cwd.exists()),
                });
            }
            // Source records of the family and its aliases, put away so one
            // account owns the conversation.
            let mut retire: Vec<&Record> = family
                .iter()
                .chain(rebind.keys().filter(|id| source_records.contains_key(*id)))
                .map(|id| &source_records[id])
                .collect();
            retire.sort_by(|a, b| a.id.cmp(&b.id));
            retire.dedup_by(|a, b| a.id == b.id);
            for record in retire {
                b.plan.steps.push(MoveStep::Hold { path: record.path.clone(), why: HoldWhy::RetiredRecord });
            }
        }
        Ok(b.plan)
    }

    fn verify(&self, request: &MoveRequest, plan: &MovePlan) -> Vec<MoveCheck> {
        let mut checks = Vec::new();
        let mut check = |what: String, pass: bool| checks.push(MoveCheck { what, pass });
        for member in plan.members.iter().filter(|m| m.side == MemberSide::Source) {
            if let Some(to) = &member.transcript_to {
                check(format!("{} history is in the target", member.conversation), to.is_file());
            }
            if let Some(from) = &member.transcript {
                check(
                    format!("{} history left the source", member.conversation),
                    std::fs::symlink_metadata(from).is_err(),
                );
            }
        }
        let (Some(source_host), Some(target_host)) =
            (request.source.host_dir.as_deref(), request.target.host_dir.as_deref())
        else {
            return checks;
        };
        let mut odd = Vec::new();
        let source_records = records(source_host, &mut odd);
        let target_records = records(target_host, &mut odd);
        let planned: BTreeMap<&str, &MoveMember> = plan
            .members
            .iter()
            .filter_map(|m| m.record_id.as_deref().map(|id| (id, m)))
            .collect();
        // Records the move put aside (retired source records, the target's
        // aliases): nothing may still hang from one of them.
        let held: BTreeSet<String> = plan
            .steps
            .iter()
            .filter_map(|s| match s {
                MoveStep::Hold { path, .. } => path.file_stem().and_then(|s| s.to_str()).map(str::to_owned),
                _ => None,
            })
            .filter(|stem| stem.starts_with(LOCAL))
            .collect();
        let alive = |id: &str| target_records.contains_key(id);
        for (id, member) in &planned {
            let Some(record) = target_records.get(*id) else {
                check(format!("{id} is listed by the target app"), false);
                continue;
            };
            check(format!("{id} is listed by the target app"), true);
            check(
                format!("{id} keeps its conversation {}", member.conversation),
                record.conversation == member.conversation,
            );
            check(format!("{id} keeps its archived state"), record.archived() == member.archived);
            for parent in record.parents() {
                let ok = alive(&parent) || (!held.contains(&parent) && !planned.contains_key(parent.as_str()));
                check(format!("{id}'s parent {parent} is a record the target lists"), ok);
            }
            if member.side == MemberSide::Source {
                let still = source_records.values().any(|r| r.conversation == member.conversation);
                check(format!("no source record still claims {}", member.conversation), !still);
            }
        }
        // Closure: everything that hangs from the moved conversation in the
        // target is a member of the plan.
        let roots: BTreeSet<String> = planned
            .keys()
            .filter(|id| target_records.get(**id).is_some_and(|r| r.conversation == request.conversation))
            .map(|id| (*id).to_owned())
            .collect();
        let reached = descendants(&target_records, &roots);
        let unplanned: Vec<&String> = reached.iter().filter(|id| !planned.contains_key(id.as_str())).collect();
        check(
            format!("the target's family is exactly the planned one ({} unplanned)", unplanned.len()),
            unplanned.is_empty(),
        );
        let claimed = target_records
            .values()
            .filter(|r| r.conversation == request.conversation)
            .count();
        check("one target record claims the moved conversation".into(), claimed == 1);
        checks
    }

    fn appended(&self, line: &[u8]) -> Appended {
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
            return Appended::Unknown;
        };
        let kind = value.get("type").and_then(serde_json::Value::as_str);
        match kind {
            Some("assistant") => Appended::Turn,
            Some("user") => {
                let system = value.get("promptSource").and_then(serde_json::Value::as_str) == Some("system");
                let notice = value
                    .get("origin")
                    .and_then(|o| o.get("kind"))
                    .and_then(serde_json::Value::as_str)
                    == Some("task-notification");
                let transcript_only = value.get("queueTranscriptOnly").and_then(serde_json::Value::as_bool) == Some(true);
                if system && notice && transcript_only {
                    Appended::SystemNotice
                } else {
                    Appended::Turn
                }
            }
            Some(_) => Appended::Bookkeeping,
            None => Appended::Unknown,
        }
    }

    fn resume_link(&self, conversation: &str) -> Option<String> {
        conversation
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
            .then(|| format!("claude://resume?session={conversation}"))
    }
}

/// How far `id` is below `root` (parents first).
fn depth(all: &BTreeMap<String, Record>, id: &str, root: &str) -> usize {
    let mut at = id.to_owned();
    for steps in 0..MAX_DEPTH {
        if at == root {
            return steps;
        }
        match all.get(&at).and_then(|r| r.parents().into_iter().next()) {
            Some(parent) => at = parent,
            None => return steps,
        }
    }
    MAX_DEPTH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_system_notice_is_not_a_turn() {
        let notice = br#"{"type":"user","promptSource":"system","origin":{"kind":"task-notification"},"queueTranscriptOnly":true}"#;
        assert_eq!(ClaudeMove.appended(notice), Appended::SystemNotice);
        let typed = br#"{"type":"user","message":{"content":"hi"}}"#;
        assert_eq!(ClaudeMove.appended(typed), Appended::Turn);
        // Only the full mark makes a notice.
        let partly = br#"{"type":"user","promptSource":"system","origin":{"kind":"task-notification"}}"#;
        assert_eq!(ClaudeMove.appended(partly), Appended::Turn);
        assert_eq!(ClaudeMove.appended(br#"{"type":"custom-title"}"#), Appended::Bookkeeping);
        assert_eq!(ClaudeMove.appended(b"{torn"), Appended::Unknown);
    }

    #[test]
    fn references_are_rebound_in_structured_keys_only() {
        let mut value = serde_json::json!({
            "sessionId": "local_child",
            "spawnedFrom": {"sessionId": "local_old", "name": "local_old"},
            "routing": [{"targetSessionId": "local_old"}],
            "title": "about local_old",
        });
        let rebind = HashMap::from([("local_old".to_owned(), "local_new".to_owned())]);
        assert!(rebound(&mut value, &rebind));
        assert_eq!(value["spawnedFrom"]["sessionId"], "local_new");
        assert_eq!(value["spawnedFrom"]["name"], "local_old");
        assert_eq!(value["routing"][0]["targetSessionId"], "local_new");
        assert_eq!(value["title"], "about local_old");
        assert_eq!(value["sessionId"], "local_child");
    }

    #[test]
    fn the_resume_link_takes_only_an_id() {
        assert_eq!(
            ClaudeMove.resume_link("9e09-ab").as_deref(),
            Some("claude://resume?session=9e09-ab")
        );
        assert!(ClaudeMove.resume_link("x&y=z").is_none());
    }

    #[test]
    fn versions_are_gated_by_major() {
        assert!(proven(Some("4.16336.0"), PROVEN_APP_MAJORS));
        assert!(!proven(Some("5.0.1"), PROVEN_APP_MAJORS));
        assert!(!proven(None, PROVEN_APP_MAJORS));
    }
}
