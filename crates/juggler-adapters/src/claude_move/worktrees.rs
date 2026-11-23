//! AJ-E2328: a move carries the conversation's **ownership** of what the
//! Claude app keeps for it outside its record and history, and a health
//! check finds and repairs what moves before 0.5.1 left split.
//!
//! What the app keeps, per profile (read from the files on disk and the
//! app's own logs on the Claude app 5.11492.5, 2026-11-23; the shapes are
//! in `known_formats.json`, explained in `docs/design/claude-known-formats.md`):
//! - **`git-worktrees.json`** (`schemaVersion` 2): `worktrees.<name>`, one
//!   entry per worktree the profile created and owns, naming the session
//!   record that holds it (`leasedBy`, a record id), its folder (`path`)
//!   and the branch it made it on (`branch`); and `untrackedDirGc`, the
//!   profile's cleanup of unused worktree folders: the worktree folders it
//!   looks after (`roots`) and the ones it has seen there without owning
//!   them (`sightings`, folder → when). Other top-level keys
//!   (`originUrls`, `originPins`, `pendingTombstones`) and any field this
//!   module doesn't know are kept exactly as they are.
//! - **`side-session-notes/<record id>/`**: a conversation's notes folder,
//!   which its side sessions are granted (a record's
//!   `sessionPermissionUpdates[].directories`, by full path).
//!
//! What the app's logs show it does with them: a profile resuming a
//! conversation whose worktree it doesn't own, or whose owned entry names
//! another branch than the worktree is on, makes a fresh worktree instead
//! ("was recycled; created fresh worktree"); a profile's cleanup removes
//! a worktree folder it saw without owning it after a day ("Removed
//! untracked worktree dir"); and a session may write only in the worktree
//! its own profile owns ("worktree-write-guard blocked Write
//! (`sibling_worktree`)"). So a move that carries only the record leaves the
//! conversation's work in a folder its new profile doesn't own.
//!
//! What a move does now: each moved conversation's worktree entry goes
//! from the source profile's file to the target's (the target owns it),
//! naming the branch the worktree is on (read from the worktree's own
//! `HEAD`, never checked out); the source forgets having seen it; the
//! notes folder moves beside the records; and paths in the records that
//! point into the source profile point into the target's. Both files are
//! rewritten whole by the engine, backed up first, so Undo restores them
//! byte for byte. No file inside a worktree is ever written.
//!
//! The gate (AJ-E2328 req 1 and 9): the Claude app's version must be one
//! `known_formats.json` accepts, and each worktree file must be the
//! schema it names, or the move and the repair refuse, in words.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use juggler_adapter::compat::Version;
use juggler_adapter::moving::{Carried, MoveCheck, MovePlan, MoveStep, Refusal, Split, Tidied};
use serde_json::{Map, Value};

use super::{Builder, LOCAL, Record, shown};

/// A profile's worktree file.
pub(super) const FILE: &str = "git-worktrees.json";
/// A profile's side-session notes, one folder per record id.
pub(super) const NOTES: &str = "side-session-notes";
/// The known formats, shared with `scripts/claude_compat_check.py`.
const MANIFEST: &str = include_str!("known_formats.json");
/// A `HEAD` is a line; one past this isn't read.
const HEAD_MAX: u64 = 4096;

/// What the manifest says, parsed once.
struct Known {
    schema: u64,
    accept: Vec<(Version, Option<Version>)>,
    checked: Vec<String>,
    required: Vec<(String, String)>,
}

fn known() -> Option<&'static Known> {
    static KNOWN: OnceLock<Option<Known>> = OnceLock::new();
    KNOWN.get_or_init(|| parse_manifest(MANIFEST)).as_ref()
}

fn parse_manifest(text: &str) -> Option<Known> {
    let value: Value = serde_json::from_str(text).ok()?;
    let app = value.get("app")?;
    let mut accept = Vec::new();
    for span in app.get("accept")?.as_array()? {
        let min = span.get("min")?.as_str()?.parse().ok()?;
        let below = match span.get("below").and_then(Value::as_str) {
            Some(text) => Some(text.parse().ok()?),
            None => None,
        };
        accept.push((min, below));
    }
    let file = value.get("entries")?.get("worktree_file")?;
    Some(Known {
        schema: file.get("schemaVersion")?.as_u64()?,
        accept,
        checked: app
            .get("checked")?
            .as_array()?
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        required: file
            .get("lease")?
            .get("required")?
            .as_object()?
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
            .collect(),
    })
}

/// An app's version as its bundle says it (`5.11492.5`, a two-part one
/// read as `.0`).
fn version_of(text: &str) -> Option<Version> {
    let word = text.split_whitespace().next()?;
    let word = word.strip_prefix('v').unwrap_or(word);
    word.parse()
        .ok()
        .or_else(|| format!("{word}.0").parse().ok())
}

/// The Claude app versions the manifest says moves were checked on.
#[must_use]
pub fn checked_versions() -> Vec<String> {
    known().map(|k| k.checked.clone()).unwrap_or_default()
}

/// Whether the Claude app `version` is one moves were checked on (the
/// manifest's accepted range).
#[must_use]
pub fn moves_checked(version: Option<&str>) -> bool {
    let (Some(known), Some(version)) = (known(), version.and_then(version_of)) else {
        return false;
    };
    known
        .accept
        .iter()
        .any(|(min, below)| version >= *min && below.as_ref().is_none_or(|b| version < *b))
}

/// Why `action` (Move, Repair) refuses the Claude app `version`, in words
/// with what to do; `None`: it is one the manifest accepts.
pub(super) fn version_refusal(version: Option<&str>, action: &str) -> Option<String> {
    if moves_checked(version) {
        return None;
    }
    let checked = checked_versions().join(", ");
    let copy = if action == "Move" {
        ". Copy still works"
    } else {
        ""
    };
    Some(match version.map(str::trim).filter(|v| !v.is_empty()) {
        Some(version) => format!(
            "AI Juggler hasn't checked moves on the Claude app {version} yet (checked: {checked}); update AI Juggler, and {action} becomes available{copy}"
        ),
        None => format!(
            "AI Juggler couldn't read the Claude app's version, so it can't tell whether moves are checked on it{copy}"
        ),
    })
}

/// A profile's worktree file as read.
pub(super) enum Worktrees {
    /// The profile has none (it made no worktree yet).
    Absent,
    /// In the format the manifest names.
    Known(Value),
    /// In another format: what didn't fit, in words.
    Unknown(String),
}

/// Reads `host`'s worktree file. Reads only.
pub(super) fn read(host: &Path) -> Worktrees {
    let path = host.join(FILE);
    let Ok(meta) = std::fs::symlink_metadata(&path) else {
        return Worktrees::Absent;
    };
    if !meta.is_file() || meta.len() > super::RECORD_MAX {
        return Worktrees::Unknown(format!("{FILE} isn't a plain file of a known size"));
    }
    let Some(value) = std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
    else {
        return Worktrees::Unknown(format!("{FILE} isn't JSON"));
    };
    match shape_of(&value) {
        Ok(()) => Worktrees::Known(value),
        Err(what) => Worktrees::Unknown(what),
    }
}

/// Whether `value` has the shape the manifest names.
fn shape_of(value: &Value) -> Result<(), String> {
    let known =
        known().ok_or_else(|| "AI Juggler's own list of formats can't be read".to_owned())?;
    let object = value
        .as_object()
        .ok_or_else(|| format!("{FILE} isn't an object"))?;
    if object.get("schemaVersion").and_then(Value::as_u64) != Some(known.schema) {
        return Err(format!(
            "{FILE} is of another schema version than {}",
            known.schema
        ));
    }
    if let Some(worktrees) = object.get("worktrees") {
        let worktrees = worktrees
            .as_object()
            .ok_or_else(|| format!("{FILE}'s worktrees aren't an object"))?;
        for entry in worktrees.values() {
            let entry = entry
                .as_object()
                .ok_or_else(|| format!("a worktree entry of {FILE} isn't an object"))?;
            for (key, kind) in &known.required {
                let fits = match kind.as_str() {
                    "string" => entry.get(key).is_some_and(Value::is_string),
                    _ => entry.contains_key(key),
                };
                if !fits {
                    return Err(format!(
                        "a worktree entry of {FILE} has no {key} as AI Juggler knows it"
                    ));
                }
            }
        }
    }
    if let Some(sweep) = object.get("untrackedDirGc") {
        let sweep = sweep
            .as_object()
            .ok_or_else(|| format!("{FILE}'s cleanup isn't an object"))?;
        for key in ["sightings", "roots"] {
            if let Some(map) = sweep.get(key)
                && !map
                    .as_object()
                    .is_some_and(|m| m.values().all(Value::is_number))
            {
                return Err(format!(
                    "{FILE}'s cleanup {key} aren't as AI Juggler knows them"
                ));
            }
        }
    }
    Ok(())
}

/// The worktree entries of `value`, by name.
fn leases(value: &Value) -> Vec<(String, &Map<String, Value>)> {
    value
        .get("worktrees")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(name, entry)| Some((name.clone(), entry.as_object()?)))
                .collect()
        })
        .unwrap_or_default()
}

fn text<'a>(entry: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    entry.get(key).and_then(Value::as_str)
}

/// `value`'s `worktrees` map, made when it has none.
fn worktrees_mut(value: &mut Value) -> Option<&mut Map<String, Value>> {
    let object = value.as_object_mut()?;
    object
        .entry("worktrees")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
}

/// Drops `path` from what `value`'s cleanup has seen. Whether it was there.
fn unsee(value: &mut Value, path: &str) -> bool {
    value
        .get_mut("untrackedDirGc")
        .and_then(|gc| gc.get_mut("sightings"))
        .and_then(Value::as_object_mut)
        .is_some_and(|seen| seen.remove(path).is_some())
}

/// Whether `value`'s cleanup looks after the folder `path` is in, or has
/// seen `path`.
fn sweeps(value: &Value, path: &Path) -> bool {
    let Some(gc) = value.get("untrackedDirGc") else {
        return false;
    };
    let seen = gc
        .get("sightings")
        .and_then(Value::as_object)
        .is_some_and(|m| m.keys().any(|k| Path::new(k) == path));
    let root = gc
        .get("roots")
        .and_then(Value::as_object)
        .is_some_and(|m| m.keys().any(|k| path.parent() == Some(Path::new(k))));
    seen || root
}

/// A new worktree file for a profile that has none (the shape of one the
/// app made before it owned a worktree).
fn fresh(schema: u64) -> Value {
    serde_json::json!({
        "schemaVersion": schema,
        "originUrls": {},
        "originPins": {},
        "worktrees": {},
        "untrackedDirGc": {"sightings": {}, "roots": {}, "cwds": {}}
    })
}

/// What a worktree's `HEAD` says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Head {
    /// On a branch.
    Branch(String),
    /// On no branch (a commit checked out).
    Detached,
    /// Not readable, and why.
    Unreadable(String),
}

/// The worktree at `worktree`'s `HEAD`, read from its git folder (`.git`
/// is a file naming it, or the folder itself). Reads only, never runs git.
pub(super) fn head(worktree: &Path) -> Head {
    let dot = worktree.join(".git");
    let Ok(meta) = std::fs::symlink_metadata(&dot) else {
        return Head::Unreadable("it has no .git".to_owned());
    };
    let git_dir = if meta.is_dir() {
        dot
    } else if meta.is_file() && meta.len() <= HEAD_MAX {
        let Some(named) = std::fs::read_to_string(&dot).ok().and_then(|t| {
            t.trim()
                .strip_prefix("gitdir:")
                .map(|p| p.trim().to_owned())
        }) else {
            return Head::Unreadable("its .git doesn't name its git folder".to_owned());
        };
        let named = PathBuf::from(named);
        if named.is_absolute() {
            named
        } else {
            worktree.join(named)
        }
    } else {
        return Head::Unreadable("its .git isn't a file or folder AI Juggler reads".to_owned());
    };
    let head = git_dir.join("HEAD");
    let line = std::fs::symlink_metadata(&head)
        .ok()
        .filter(|m| m.is_file() && m.len() <= HEAD_MAX)
        .and_then(|_| std::fs::read_to_string(&head).ok());
    let Some(line) = line else {
        return Head::Unreadable("its HEAD can't be read".to_owned());
    };
    let line = line.trim();
    if let Some(branch) = line.strip_prefix("ref: refs/heads/") {
        return Head::Branch(branch.to_owned());
    }
    if (line.len() == 40 || line.len() == 64) && line.chars().all(|c| c.is_ascii_hexdigit()) {
        return Head::Detached;
    }
    Head::Unreadable("its HEAD isn't a branch or a commit".to_owned())
}

/// The other Claude app profiles near `hosts` (beside them, and beside
/// their folder: `…/ClaudeProfiles/<name>` and `…/Claude`), each with a
/// worktree file, `hosts` left out.
fn profiles_near(hosts: &[&Path]) -> Vec<PathBuf> {
    let mut places: BTreeSet<PathBuf> = BTreeSet::new();
    for host in hosts {
        for up in host.ancestors().skip(1).take(2) {
            places.insert(up.to_path_buf());
        }
    }
    let mut out: BTreeSet<PathBuf> = BTreeSet::new();
    for place in places {
        let Ok(entries) = std::fs::read_dir(&place) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|k| k.is_dir())
                && !hosts.iter().any(|h| *h == path)
                && std::fs::symlink_metadata(path.join(FILE)).is_ok_and(|m| m.is_file())
            {
                out.insert(path);
            }
        }
    }
    out.into_iter().collect()
}

/// A profile's name in words: its folder's name.
fn profile_name(host: &Path) -> String {
    host.file_name()
        .map_or_else(|| shown(host), |n| n.to_string_lossy().into_owned())
}

/// "a", "a and b", "a, b and c".
fn and_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The note naming the other profiles that look after the folder `path`
/// is in (they are left as they are).
fn swept_note(others: &[PathBuf], name: &str) -> Option<String> {
    if others.is_empty() {
        return None;
    }
    let names: Vec<String> = others.iter().map(|p| profile_name(p)).collect();
    Some(format!(
        "the Claude app profile{} {} also clean{} up unused folders where the worktree {name} is; AI Juggler leaves that as it is",
        if names.len() == 1 { "" } else { "s" },
        and_list(&names),
        if names.len() == 1 { "s" } else { "" },
    ))
}

/// What the worktree carry decided for the records it writes: the branch
/// each record's worktree is really on, by record id.
#[derive(Default)]
pub(super) struct Fixes {
    pub branches: HashMap<String, String>,
}

/// Whether `record` names a worktree (its name or folder).
fn names_worktree(record: &Record) -> bool {
    ["worktreeName", "worktreePath"]
        .iter()
        .any(|key| record.value.get(*key).is_some_and(Value::is_string))
}

/// The branch fix for `record` working in worktree `name` on `branch`.
fn record_branch_fix(record: &Record, name: &str, branch: &str) -> Option<String> {
    let works_there = record.value.get("worktreeName").and_then(Value::as_str) == Some(name);
    let had = record.value.get("branch").and_then(Value::as_str)?;
    (works_there && had != branch).then(|| branch.to_owned())
}

/// AJ-E2328 req 2–4: carries each moved record's worktree entry from the
/// source profile's file to the target's, with the branch its worktree is
/// on, and drops it from the source's cleanup sightings; the plan says
/// so, names other profiles that look after the same folder, and refuses
/// what it can't carry exactly. `members`: the moved records (source).
#[allow(clippy::too_many_lines, reason = "one carry, read top to bottom")]
pub(super) fn carry(b: &mut Builder<'_>, members: &[&Record], family: &BTreeSet<String>) -> Fixes {
    let mut fixes = Fixes::default();
    let (Some(source), Some(target)) = (
        b.request.source.host_dir.clone(),
        b.request.target.host_dir.clone(),
    ) else {
        return fixes;
    };
    let in_worktree = members.iter().any(|r| names_worktree(r));
    let (mut src, tgt) = match (read(&source), read(&target)) {
        (Worktrees::Unknown(what), _) | (_, Worktrees::Unknown(what)) => {
            let holds = in_worktree
                || std::fs::read(source.join(FILE)).is_ok_and(|bytes| {
                    family
                        .iter()
                        .any(|id| memchr::memmem::find(&bytes, id.as_bytes()).is_some())
                });
            if holds {
                if !b.plan.shape.contains(&what) {
                    b.plan.shape.push(what.clone());
                }
                b.refuse(format!(
                    "the Claude app keeps its worktrees in a format AI Juggler doesn't know yet ({what}); update AI Juggler, and Move becomes available. Copy still works"
                ));
            }
            return fixes;
        }
        (Worktrees::Absent, _) => return fixes,
        (Worktrees::Known(src), tgt) => (src, tgt),
    };
    let schema = known().map_or(2, |k| k.schema);
    let mut tgt = match tgt {
        Worktrees::Known(value) => value,
        _ => fresh(schema),
    };
    let near = profiles_near(&[&source, &target]);
    let mut changed = false;
    for record in members {
        let names: Vec<String> = leases(&src)
            .into_iter()
            .filter(|(_, e)| text(e, "leasedBy") == Some(record.id.as_str()))
            .map(|(name, _)| name)
            .collect();
        let title = record
            .title()
            .unwrap_or_else(|| record.conversation.clone());
        for name in names {
            let Some(mut entry) = src.get("worktrees").and_then(|w| w.get(&name)).cloned() else {
                continue;
            };
            let path = entry
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if let Some(theirs) = leases(&tgt).into_iter().find(|(n, _)| *n == name) {
                let by = text(theirs.1, "leasedBy").unwrap_or_default();
                if !family.contains(by) {
                    b.refuse(format!(
                        "another conversation in the target profile works in the worktree {name} of “{title}”, so the move would take it away; Copy carries it"
                    ));
                    continue;
                }
            }
            let folder = Path::new(&path);
            let mut branch = entry
                .get("branch")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if folder.is_dir() {
                match head(folder) {
                    Head::Branch(now) => {
                        if branch.as_deref() != Some(now.as_str()) {
                            b.note(format!(
                                "“{title}”'s worktree {name} is on branch {now}, not {} as the Claude app had it; the move records {now} (nothing is checked out)",
                                branch.as_deref().unwrap_or("none")
                            ));
                            entry["branch"] = Value::String(now.clone());
                        }
                        if let Some(fix) = record_branch_fix(record, &name, &now) {
                            fixes.branches.insert(record.id.clone(), fix);
                        }
                        branch = Some(now);
                    }
                    Head::Detached => {
                        b.wait(format!(
                            "“{title}”'s worktree {name} isn't on a branch; check out its branch there, and Move becomes available"
                        ));
                        continue;
                    }
                    Head::Unreadable(why) => {
                        b.wait(format!(
                            "AI Juggler can't tell which branch “{title}”'s worktree {name} is on ({why}); make sure the folder is a worktree AI Juggler can read, and Move becomes available"
                        ));
                        continue;
                    }
                }
            }
            if let Some(map) = worktrees_mut(&mut tgt) {
                map.insert(name.clone(), entry);
            }
            if let Some(map) = src.get_mut("worktrees").and_then(Value::as_object_mut) {
                map.remove(&name);
            }
            unsee(&mut src, &path);
            changed = true;
            let others: Vec<PathBuf> = near
                .iter()
                .filter(|p| matches!(read(p), Worktrees::Known(v) if sweeps(&v, folder)))
                .cloned()
                .collect();
            if let Some(note) = swept_note(&others, &name) {
                b.note(note);
            }
            b.plan.carried.push(Carried::Worktree { name, branch });
        }
    }
    if changed {
        b.plan.steps.push(MoveStep::WriteAppFile {
            path: source.join(FILE),
            value: src,
        });
        b.plan.steps.push(MoveStep::WriteAppFile {
            path: target.join(FILE),
            value: tgt,
        });
    }
    fixes
}

/// AJ-E2328 req 5: points the paths of `value`'s permission grants and
/// shell folders that are inside `from` at the same place inside `to`.
/// Whether anything changed.
pub(super) fn rehome(value: &mut Value, from: &Path, to: &Path) -> bool {
    let mut changed = false;
    let mut fix = |item: &mut Value| {
        if let Some(rest) = item
            .as_str()
            .and_then(|p| Path::new(p).strip_prefix(from).ok())
        {
            *item = Value::String(to.join(rest).display().to_string());
            changed = true;
        }
    };
    if let Some(updates) = value
        .get_mut("sessionPermissionUpdates")
        .and_then(Value::as_array_mut)
    {
        for update in updates {
            if let Some(dirs) = update.get_mut("directories").and_then(Value::as_array_mut) {
                dirs.iter_mut().for_each(&mut fix);
            }
        }
    }
    if let Some(folders) = value.get_mut("shellFolders").and_then(Value::as_array_mut) {
        folders.iter_mut().for_each(&mut fix);
    }
    changed
}

/// The adapter's checks of the carried worktrees once a move is applied
/// (and again once the apps started): each is owned by the target, for a
/// moved conversation, and no longer by the source.
pub(super) fn checks(
    plan: &MovePlan,
    source: &Path,
    target: &Path,
    ids: &BTreeSet<String>,
) -> Vec<MoveCheck> {
    let mut out = Vec::new();
    let (src, tgt) = (read(source), read(target));
    for carried in &plan.carried {
        let Carried::Worktree { name, .. } = carried else {
            continue;
        };
        let owned = |file: &Worktrees| match file {
            Worktrees::Known(v) => leases(v)
                .into_iter()
                .find(|(n, _)| n == name)
                .map(|(_, e)| text(e, "leasedBy").unwrap_or_default().to_owned()),
            _ => None,
        };
        out.push(MoveCheck {
            what: format!("the target profile owns the worktree {name}"),
            pass: owned(&tgt).is_some_and(|by| ids.contains(&by)),
        });
        out.push(MoveCheck {
            what: format!("the source profile no longer owns the worktree {name}"),
            pass: owned(&src).is_none(),
        });
    }
    out
}

/// One profile's records' org folders (`claude-code-sessions/<account>/<org>`).
fn org_folders(host: &Path) -> Vec<PathBuf> {
    let list = |dir: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .map(|e| {
                e.flatten()
                    .filter(|e| e.file_type().is_ok_and(|k| k.is_dir()))
                    .map(|e| e.path())
                    .collect()
            })
            .unwrap_or_default()
    };
    list(&crate::claude_live::app_root(host))
        .iter()
        .flat_map(|account| list(account))
        .collect()
}

/// The record file `id` among `orgs`, when there is one.
fn record_in(orgs: &[PathBuf], id: &str) -> Option<PathBuf> {
    if !id.starts_with(LOCAL) || id.contains(['/', '\\']) || id.contains("..") {
        return None;
    }
    orgs.iter()
        .map(|org| org.join(format!("{id}.json")))
        .find(|path| std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file()))
}

fn read_record(path: &Path) -> Option<Value> {
    std::fs::symlink_metadata(path)
        .ok()
        .filter(|m| m.is_file() && m.len() <= super::RECORD_MAX)
        .and_then(|_| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

/// AJ-E2328 req 7: the conversations among `hosts` whose worktree is owned
/// by another profile than the one holding them, or whose owned entry
/// names another branch than the worktree is on. Only worktrees whose
/// folder is there count (a gone one is made again by the app). Reads
/// only, never inside a worktree but its `.git` and `HEAD`.
///
/// # Errors
///
/// A profile's worktree file is in a format the manifest doesn't name.
pub(super) fn splits(hosts: &[PathBuf]) -> Result<Vec<Split>, String> {
    let mut files: Vec<(&PathBuf, Value)> = Vec::new();
    for host in hosts {
        match read(host) {
            Worktrees::Absent => {}
            Worktrees::Known(value) => files.push((host, value)),
            Worktrees::Unknown(what) => {
                return Err(format!(
                    "{} keeps its worktrees in a format AI Juggler doesn't know yet ({what})",
                    profile_name(host)
                ));
            }
        }
    }
    let orgs: Vec<(&PathBuf, Vec<PathBuf>)> = hosts.iter().map(|h| (h, org_folders(h))).collect();
    let mut out = Vec::new();
    for (owner, value) in &files {
        for (name, entry) in leases(value) {
            let (Some(path), Some(by)) = (text(entry, "path"), text(entry, "leasedBy")) else {
                continue;
            };
            let folder = Path::new(path);
            if !folder.is_dir() {
                continue;
            }
            let holders: Vec<(&PathBuf, PathBuf)> = orgs
                .iter()
                .filter_map(|(host, orgs)| record_in(orgs, by).map(|file| (*host, file)))
                .collect();
            let holder = if let Some(here) = holders.iter().find(|(h, _)| h == owner) {
                here
            } else if let [one] = holders.as_slice() {
                one
            } else {
                continue;
            };
            let branch = match head(folder) {
                Head::Branch(now) => Some(now),
                _ => None,
            };
            let record = read_record(&holder.1).unwrap_or(Value::Null);
            let had = text(entry, "branch");
            let record_branch = record
                .get("worktreeName")
                .and_then(Value::as_str)
                .filter(|n| *n == name)
                .and(record.get("branch").and_then(Value::as_str));
            let branch_differs = branch
                .as_deref()
                .is_some_and(|now| had != Some(now) || record_branch.is_some_and(|b| b != now));
            let elsewhere = holder.0 != *owner;
            // One conversation, two worktrees: its record names a fresh
            // one the app made.
            let other_worktree = record
                .get("worktreeName")
                .and_then(Value::as_str)
                .filter(|n| *n != name)
                .map(str::to_owned);
            if !elsewhere && !branch_differs {
                continue;
            }
            let also_swept_by = files
                .iter()
                .filter(|(h, v)| *h != *owner && *h != holder.0 && sweeps(v, folder))
                .map(|(h, _)| (*h).clone())
                .collect();
            out.push(Split {
                key: format!("{name}@{}", profile_name(owner)),
                worktree: name.clone(),
                owner: (*owner).clone(),
                holder: holder.0.clone(),
                conversation: record
                    .get("cliSessionId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                title: record
                    .get("title")
                    .and_then(Value::as_str)
                    .filter(|t| !t.trim().is_empty())
                    .map(str::to_owned),
                branch,
                branch_differs,
                also_swept_by,
                other_worktree,
            });
        }
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(out)
}

/// The repair's note when a conversation has two worktrees.
pub(super) const TWO_WORKTREES: &str = "This conversation has two worktrees; both are kept";

/// AJ-E2328 (the maintainer's ruling): the worktrees among `hosts` that one
/// profile owns and another profile's cleanup has seen (its sightings name
/// the folder), so that cleanup may remove it once it looks unused there.
/// Only folders that are there count. Reads only.
///
/// # Errors
///
/// As [`splits`].
pub(super) fn also_tidied(hosts: &[PathBuf]) -> Result<Vec<Tidied>, String> {
    let mut files: Vec<(&PathBuf, Value)> = Vec::new();
    for host in hosts {
        match read(host) {
            Worktrees::Absent => {}
            Worktrees::Known(value) => files.push((host, value)),
            Worktrees::Unknown(what) => {
                return Err(format!(
                    "{} keeps its worktrees in a format AI Juggler doesn't know yet ({what})",
                    profile_name(host)
                ));
            }
        }
    }
    let seen = |value: &Value, folder: &Path| {
        value
            .get("untrackedDirGc")
            .and_then(|gc| gc.get("sightings"))
            .and_then(Value::as_object)
            .is_some_and(|m| m.keys().any(|k| Path::new(k) == folder))
    };
    let mut out = Vec::new();
    for (owner, value) in &files {
        for (name, entry) in leases(value) {
            let Some(path) = text(entry, "path") else {
                continue;
            };
            let folder = Path::new(path);
            if !folder.is_dir() {
                continue;
            }
            let by: Vec<PathBuf> = files
                .iter()
                .filter(|(h, v)| *h != *owner && seen(v, folder))
                .map(|(h, _)| (*h).clone())
                .collect();
            if !by.is_empty() {
                out.push(Tidied {
                    worktree: name,
                    owner: (*owner).clone(),
                    by,
                });
            }
        }
    }
    out.sort_by(|a, b| (&a.worktree, &a.owner).cmp(&(&b.worktree, &b.owner)));
    Ok(out)
}

/// AJ-E2328 req 7: the plan that puts `split` right, made again from the
/// disk now: the worktree's entry moved from its owner's file to the
/// holder's (one file when they are one profile) naming the branch it is
/// on, the owner's sighting of it dropped, and the holder's record naming
/// that branch. Never a step inside the worktree. Reads only.
#[allow(clippy::too_many_lines, reason = "one repair, read top to bottom")]
pub(super) fn repair(split: &Split, version: Option<&str>) -> MovePlan {
    let mut plan = MovePlan {
        conversation: split.conversation.clone(),
        resume: split.conversation.clone(),
        roots: vec![split.owner.clone(), split.holder.clone()],
        ..MovePlan::default()
    };
    plan.roots.dedup();
    let refuse = |plan: &mut MovePlan, refusal: Refusal| {
        if !plan.refusals.contains(&refusal) {
            plan.refusals.push(refusal);
        }
    };
    if let Some(why) = version_refusal(version, "Repair") {
        refuse(&mut plan, Refusal::setup(why));
        return plan;
    }
    let now = match splits(&[split.owner.clone(), split.holder.clone()]) {
        Ok(found) => found.into_iter().find(|s| s.key == split.key),
        Err(why) => {
            refuse(
                &mut plan,
                Refusal::setup(format!(
                    "{why}; update AI Juggler, and Repair becomes available"
                )),
            );
            return plan;
        }
    };
    let Some(now) = now else {
        refuse(
            &mut plan,
            Refusal::setup("this worktree's ownership is already right; nothing to repair"),
        );
        return plan;
    };
    let (Worktrees::Known(mut owner), holder) = (read(&now.owner), read(&now.holder)) else {
        refuse(
            &mut plan,
            Refusal::setup("the profile that owns the worktree has no worktree file any more"),
        );
        return plan;
    };
    let name = now.worktree.clone();
    let Some(mut entry) = owner.get("worktrees").and_then(|w| w.get(&name)).cloned() else {
        refuse(
            &mut plan,
            Refusal::setup("the worktree's entry is gone; nothing to repair"),
        );
        return plan;
    };
    let path = entry
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let by = entry
        .get("leasedBy")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let title = now
        .title
        .clone()
        .unwrap_or_else(|| now.conversation.clone());
    let branch = match head(Path::new(&path)) {
        Head::Branch(branch) => branch,
        Head::Detached => {
            refuse(
                &mut plan,
                Refusal::user_can_clear(format!(
                    "“{title}”'s worktree {name} isn't on a branch; check out its branch there, and Repair becomes available"
                )),
            );
            return plan;
        }
        Head::Unreadable(why) => {
            refuse(
                &mut plan,
                Refusal::user_can_clear(format!(
                    "AI Juggler can't tell which branch “{title}”'s worktree {name} is on ({why}); Repair becomes available once it can"
                )),
            );
            return plan;
        }
    };
    entry["branch"] = Value::String(branch.clone());
    if now.owner == now.holder {
        if let Some(map) = worktrees_mut(&mut owner) {
            map.insert(name.clone(), entry);
        }
        plan.steps.push(MoveStep::WriteAppFile {
            path: now.owner.join(FILE),
            value: owner,
        });
    } else {
        let mut holder = match holder {
            Worktrees::Known(value) => value,
            Worktrees::Absent => fresh(known().map_or(2, |k| k.schema)),
            Worktrees::Unknown(what) => {
                refuse(
                    &mut plan,
                    Refusal::setup(format!(
                        "{} keeps its worktrees in a format AI Juggler doesn't know yet ({what}); update AI Juggler, and Repair becomes available",
                        profile_name(&now.holder)
                    )),
                );
                return plan;
            }
        };
        if let Some((_, theirs)) = leases(&holder).into_iter().find(|(n, _)| *n == name)
            && text(theirs, "leasedBy") != Some(by.as_str())
        {
            refuse(
                &mut plan,
                Refusal::setup(format!(
                    "another conversation in {} works in the worktree {name}, so AI Juggler can't tell which one owns it",
                    profile_name(&now.holder)
                )),
            );
            return plan;
        }
        if let Some(map) = worktrees_mut(&mut holder) {
            map.insert(name.clone(), entry);
        }
        if let Some(map) = owner.get_mut("worktrees").and_then(Value::as_object_mut) {
            map.remove(&name);
        }
        unsee(&mut owner, &path);
        plan.steps.push(MoveStep::WriteAppFile {
            path: now.owner.join(FILE),
            value: owner,
        });
        plan.steps.push(MoveStep::WriteAppFile {
            path: now.holder.join(FILE),
            value: holder,
        });
    }
    // The holder's record names the branch the worktree is on.
    let orgs = org_folders(&now.holder);
    if let Some(file) = record_in(&orgs, &by)
        && let Some(mut record) = read_record(&file)
        && record.get("worktreeName").and_then(Value::as_str) == Some(name.as_str())
        && record
            .get("branch")
            .and_then(Value::as_str)
            .is_some_and(|had| had != branch)
    {
        record["branch"] = Value::String(branch.clone());
        plan.steps.push(MoveStep::WriteRecord {
            path: file,
            value: record,
        });
    }
    if let Some(note) = swept_note(&now.also_swept_by, &name) {
        plan.notes.push(note);
    }
    // The maintainer's ruling: a claim on another worktree than the one the
    // record names is moved too, never dropped (an unowned folder is
    // removed by a cleanup, and may hold unpushed work).
    if now.other_worktree.is_some() {
        plan.notes.push(TWO_WORKTREES.to_owned());
    }
    plan.carried.push(Carried::Worktree {
        name,
        branch: Some(branch),
    });
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_reads_and_accepts_what_it_checked() {
        let known = known().expect("the manifest parses");
        assert_eq!(known.schema, 2);
        for version in &known.checked {
            assert!(moves_checked(Some(version)), "{version}");
        }
        assert!(
            moves_checked(Some("5.11492.9")),
            "a patch of the checked build"
        );
        assert!(
            !moves_checked(Some("5.11610.2")),
            "a newer build isn't checked yet"
        );
        assert!(!moves_checked(Some("4.16336.0")));
        assert!(!moves_checked(None));
        let words = version_refusal(Some("5.11610.2"), "Move").unwrap();
        assert!(
            words.contains("5.11610.2") && words.contains("update AI Juggler"),
            "{words}"
        );
        assert!(words.ends_with("Copy still works"), "{words}");
        let repair = version_refusal(None, "Repair").unwrap();
        assert!(
            repair.contains("couldn't read") && !repair.contains("Copy"),
            "{repair}"
        );
        assert!(version_refusal(Some("5.11492.5"), "Move").is_none());
    }

    #[test]
    fn head_reads_a_branch_a_detached_commit_and_says_why_it_cant() {
        let tmp = tempfile::tempdir().unwrap();
        let git = tmp.path().join("repo/.git/worktrees/w");
        std::fs::create_dir_all(&git).unwrap();
        let wt = tmp.path().join("w");
        std::fs::create_dir_all(&wt).unwrap();
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", git.display())).unwrap();
        std::fs::write(git.join("HEAD"), "ref: refs/heads/release/0.5.1\n").unwrap();
        assert_eq!(head(&wt), Head::Branch("release/0.5.1".into()));
        std::fs::write(git.join("HEAD"), format!("{}\n", "a".repeat(40))).unwrap();
        assert_eq!(head(&wt), Head::Detached);
        std::fs::remove_file(git.join("HEAD")).unwrap();
        assert!(matches!(head(&wt), Head::Unreadable(_)));
        assert!(matches!(head(tmp.path()), Head::Unreadable(_)));
    }

    #[test]
    fn grants_and_shell_folders_inside_the_source_point_into_the_target() {
        let mut value = serde_json::json!({
            "sessionPermissionUpdates": [{"type": "addDirectories", "destination": "session",
                "directories": ["/p/a/side-session-notes/local_x", "/elsewhere"]}],
            "shellFolders": ["/p/a/x", "/p/ab/y"],
            "title": "/p/a/never"
        });
        assert!(rehome(&mut value, Path::new("/p/a"), Path::new("/p/b")));
        assert_eq!(
            value["sessionPermissionUpdates"][0]["directories"][0],
            "/p/b/side-session-notes/local_x"
        );
        assert_eq!(
            value["sessionPermissionUpdates"][0]["directories"][1],
            "/elsewhere"
        );
        assert_eq!(value["shellFolders"][0], "/p/b/x");
        assert_eq!(
            value["shellFolders"][1], "/p/ab/y",
            "by components, not text"
        );
        assert_eq!(value["title"], "/p/a/never");
    }

    #[test]
    fn an_unknown_worktree_file_is_named() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(FILE), r#"{"schemaVersion": 3}"#).unwrap();
        assert!(matches!(read(tmp.path()), Worktrees::Unknown(w) if w.contains("schema")));
        std::fs::write(
            tmp.path().join(FILE),
            r#"{"schemaVersion": 2, "worktrees": {"w": {"name": "w"}}}"#,
        )
        .unwrap();
        assert!(matches!(read(tmp.path()), Worktrees::Unknown(_)));
        std::fs::write(
            tmp.path().join(FILE),
            r#"{"schemaVersion": 2, "pendingTombstones": []}"#,
        )
        .unwrap();
        assert!(matches!(read(tmp.path()), Worktrees::Known(_)));
        assert!(splits(&[tmp.path().to_path_buf()]).unwrap().is_empty());
    }
}
