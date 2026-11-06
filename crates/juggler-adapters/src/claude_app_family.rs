//! ADR-0028: a Claude app conversation's family, and carrying it into
//! another profile of the app so it nests there as it did here.
//!
//! The app keeps one record per session in
//! `<instance>/claude-code-sessions/<account>/<org>/local_<id>.json`; a
//! session another started names it in `spawnedFrom.sessionId`, and the
//! app builds its whole tree from those links, reading the records only
//! when it starts. Nothing in a conversation's own files says who started
//! it, and the app's resume link imports one conversation on its own. So
//! a family is carried by copying the app's own record of each child,
//! with the fewest fields changed:
//! - its parent link points at the parent's record on the target;
//! - its conversation id follows a fork;
//! - what belongs to the old account goes (published claude.ai pages,
//!   remote MCP configuration, peer messages, remote control).
//!
//! Records are placed only while the target app isn't running (the
//! caller makes sure), next to the parent's record there. They are never
//! built from nothing, never replace a file, and a record of a shape this
//! code doesn't know is refused rather than guessed at.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use juggler_adapter::error::AdapterError;

/// A record is a few kilobytes; one past this isn't read.
const RECORD_MAX: u64 = 4 * 1024 * 1024;
/// How deep a family is followed.
const MAX_DEPTH: usize = 16;
/// The prefix of every session record's id (and file name).
const LOCAL: &str = "local_";
/// Fields that belong to the account a record was made under.
const ACCOUNT_BOUND: [&str; 6] = [
    "publishedArtifacts",
    "remoteMcpServersConfig",
    "peerInbound",
    "peerReceipts",
    "peerMessaged",
    "steeredByRemoteClient",
];

/// One child in the family, as the source app records it.
pub type Member = juggler_adapter::live::HostFamilyMember;

/// The Claude app's family records, for the adapter's
/// [`juggler_adapter::HarnessAdapter::host_family`].
pub struct ClaudeAppFamily;

impl juggler_adapter::live::HostFamily for ClaudeAppFamily {
    fn descendants(&self, instance_dir: &Path, conversation: &str) -> Vec<Member> {
        descendants(instance_dir, conversation)
    }

    fn records_folder_of(&self, instance_dir: &Path, conversation: &str) -> Option<PathBuf> {
        records_folder_of(instance_dir, conversation)
    }

    fn place(
        &self,
        members: &[Member],
        folder: &Path,
        renamed: &[(String, String)],
    ) -> Result<Vec<PathBuf>, AdapterError> {
        place(members, folder, renamed)
    }
}

/// A record's fields this module reads (the rest are carried untouched).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Shape {
    session_id: Option<String>,
    cli_session_id: Option<String>,
    title: Option<String>,
    spawned_from: Option<Parent>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Parent {
    session_id: Option<String>,
}

pub(crate) fn read(path: &Path) -> Option<serde_json::Value> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.len() > RECORD_MAX {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Every record of an app instance, by its id.
fn records(instance_dir: &Path) -> HashMap<String, (PathBuf, Shape)> {
    crate::claude_live::app_files(instance_dir)
        .into_iter()
        .filter_map(|path| {
            let value = read(&path)?;
            let shape: Shape = serde_json::from_value(value).ok()?;
            let id = shape.session_id.clone()?;
            let stem = path.file_stem()?.to_str()?;
            (stem == id && id.starts_with(LOCAL)).then_some((id, (path, shape)))
        })
        .collect()
}

/// Every record in the app instance at `instance_dir` of conversation
/// `conversation` (usually one: the app's own, or the one it made when it
/// imported the conversation), sorted.
#[must_use]
pub fn records_of(instance_dir: &Path, conversation: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = records(instance_dir)
        .into_values()
        .filter(|(_, shape)| shape.cli_session_id.as_deref() == Some(conversation))
        .map(|(path, _)| path)
        .collect();
    paths.sort();
    paths
}

/// The sessions started from conversation `conversation` in the app
/// instance at `instance_dir`, and theirs: parents before children, each
/// once. Archived ones too: they move with the family and keep their
/// state ([`place`]), so nothing is lost.
#[must_use]
pub fn descendants(instance_dir: &Path, conversation: &str) -> Vec<Member> {
    let all = records(instance_dir);
    let conversation_of = |local: &str| {
        all.get(local)
            .and_then(|(_, shape)| shape.cli_session_id.clone())
    };
    let mut children_of: HashMap<String, Vec<Member>> = HashMap::new();
    for (id, (path, shape)) in &all {
        let (Some(parent), Some(own)) = (
            shape
                .spawned_from
                .as_ref()
                .and_then(|p| p.session_id.clone()),
            shape.cli_session_id.clone(),
        ) else {
            continue;
        };
        let Some(parent_conversation) = conversation_of(&parent) else {
            continue;
        };
        children_of
            .entry(parent_conversation.clone())
            .or_default()
            .push(Member {
                record: path.clone(),
                local_id: id.clone(),
                conversation: own,
                parent_conversation,
                title: shape.title.clone().filter(|t| !t.trim().is_empty()),
            });
    }
    let mut out: Vec<Member> = Vec::new();
    let mut level = vec![conversation.to_owned()];
    for _ in 0..MAX_DEPTH {
        let mut next = Vec::new();
        for parent in &level {
            let mut children = children_of.remove(parent).unwrap_or_default();
            children.sort_by(|a, b| a.local_id.cmp(&b.local_id));
            for child in children {
                if child.conversation != conversation
                    && !out
                        .iter()
                        .any(|seen| seen.conversation == child.conversation)
                {
                    next.push(child.conversation.clone());
                    out.push(child);
                }
            }
        }
        if next.is_empty() {
            break;
        }
        level = next;
    }
    out
}

/// The app's own record of `conversation` itself, as the root of its
/// family (no parent), for a target app that has no record of it yet.
#[must_use]
pub fn root(instance_dir: &Path, conversation: &str) -> Option<Member> {
    records(instance_dir)
        .into_iter()
        .filter(|(_, (_, shape))| shape.cli_session_id.as_deref() == Some(conversation))
        .map(|(id, (path, shape))| Member {
            record: path,
            local_id: id,
            conversation: conversation.to_owned(),
            parent_conversation: String::new(),
            title: shape.title.filter(|t| !t.trim().is_empty()),
        })
        .min_by(|a, b| a.local_id.cmp(&b.local_id))
}

/// The folder of the target app's record of `conversation` (where the
/// app put it when it imported it), the folder a family's records go to.
#[must_use]
pub fn records_folder_of(instance_dir: &Path, conversation: &str) -> Option<PathBuf> {
    let name = format!("{LOCAL}{conversation}.json");
    crate::claude_live::app_files(instance_dir)
        .into_iter()
        .find(|path| {
            path.file_name()
                .is_some_and(|n| n.to_string_lossy() == name)
        })
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

/// Places the records of `members` in `folder` (the target's records
/// folder, beside its parent's), each child pointing at its parent's
/// record there: an imported conversation's record is `local_<its id>`,
/// a carried child keeps its own record id. `renamed` maps a
/// conversation id the move forked to its new one. All or nothing: a
/// record that can't be read, isn't one this code knows, or would
/// replace a file stops it, and what was placed is taken back.
///
/// # Errors
///
/// The refusal, in words; the write that failed.
pub fn place(
    members: &[Member],
    folder: &Path,
    renamed: &[(String, String)],
) -> Result<Vec<PathBuf>, AdapterError> {
    let local_of: HashMap<&str, &str> = members
        .iter()
        .map(|m| (m.conversation.as_str(), m.local_id.as_str()))
        .collect();
    let mut placed: Vec<PathBuf> = Vec::new();
    let undo = |placed: &[PathBuf]| {
        for path in placed {
            let _ = std::fs::remove_file(path);
        }
    };
    for member in members {
        let record = match carried(member, &local_of, renamed) {
            Ok(record) => record,
            Err(error) => {
                undo(&placed);
                return Err(error);
            }
        };
        let to = folder.join(format!("{}.json", member.local_id));
        if let Err(error) = write_new(&to, &record) {
            undo(&placed);
            return Err(AdapterError::AccountDir {
                path: to,
                message: error.to_string(),
            });
        }
        placed.push(to);
    }
    Ok(placed)
}

/// `member`'s record as it goes to the target.
fn carried(
    member: &Member,
    local_of: &HashMap<&str, &str>,
    renamed: &[(String, String)],
) -> Result<serde_json::Value, AdapterError> {
    let refuse = |why: &str| {
        AdapterError::Unsupported(format!(
            "the Claude app's record of “{}” {why}, so its place in the family wasn't carried",
            member.title.as_deref().unwrap_or("a session")
        ))
    };
    let mut value = read(&member.record).ok_or_else(|| refuse("couldn't be read"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| refuse("isn't a record"))?;
    // The family's root (the conversation the switch moved, carried as
    // its own record when the target app has none yet) has no parent in
    // the family: on the target it stands at the top.
    let root = member.parent_conversation.is_empty();
    if object.get("sessionId").and_then(|v| v.as_str()) != Some(member.local_id.as_str())
        || object.get("cliSessionId").and_then(|v| v.as_str()) != Some(member.conversation.as_str())
        || (!root
            && !object
                .get("spawnedFrom")
                .is_some_and(serde_json::Value::is_object))
    {
        return Err(refuse("isn't of a shape AI Juggler knows"));
    }
    if root {
        object.remove("spawnedFrom");
        object.remove("forkedFromSessionId");
    } else if parent_archived(member) {
        // Archiving a session archives the ones it started; the parent
        // arrives active (the app imports it so), and so do they. One
        // archived on its own, under an active parent, stays archived.
        object.insert("isArchived".to_owned(), serde_json::Value::Bool(false));
    }
    // Its parent on the target: a carried sibling keeps its record id;
    // the conversation the switch moved was imported as `local_<id>`.
    let renamed_to = |id: &str| {
        renamed
            .iter()
            .find(|(old, _)| old == id)
            .map(|(_, new)| new.clone())
    };
    let parent_target = local_of
        .get(member.parent_conversation.as_str())
        .map_or_else(
            || {
                let id = renamed_to(&member.parent_conversation)
                    .unwrap_or_else(|| member.parent_conversation.clone());
                format!("{LOCAL}{id}")
            },
            |local| (*local).to_owned(),
        );
    if let Some(parent) = object
        .get_mut("spawnedFrom")
        .and_then(|v| v.as_object_mut())
        .filter(|_| !root)
    {
        parent.insert(
            "sessionId".to_owned(),
            serde_json::Value::String(parent_target),
        );
    }
    if let Some(new) = renamed_to(&member.conversation) {
        for key in ["cliSessionId", "lastAssistantCliSessionId"] {
            if object.get(key).and_then(|v| v.as_str()) == Some(member.conversation.as_str()) {
                object.insert(key.to_owned(), serde_json::Value::String(new.clone()));
            }
        }
    }
    for key in ACCOUNT_BOUND {
        object.remove(key);
    }
    // Auto-fix moves only by the switch's hand-over, once the original's
    // is off (never two copies fixing one pull request).
    crate::claude_app_automation::as_inherited(object);
    object.retain(|key, _| !key.starts_with("remoteControl"));
    Ok(value)
}

/// Whether the source record `member` was started from is archived.
fn parent_archived(member: &Member) -> bool {
    let Some(folder) = member.record.parent() else {
        return false;
    };
    read(&member.record)
        .and_then(|record| {
            record
                .get("spawnedFrom")?
                .get("sessionId")?
                .as_str()
                .map(str::to_owned)
        })
        .and_then(|parent| read(&folder.join(format!("{parent}.json"))))
        .is_some_and(|parent| {
            parent
                .get("isArchived")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        })
}

/// Writes `value` to `to`, never over an existing file, readable by the
/// user alone.
fn write_new(to: &Path, value: &serde_json::Value) -> std::io::Result<()> {
    juggler_adapter::private_fs::create_new_private(
        to,
        &serde_json::to_vec(value).map_err(std::io::Error::other)?,
    )
}

#[cfg(test)]
mod tests {
    use super::{Member, descendants, place, records_folder_of};
    use std::path::Path;

    fn record(folder: &Path, local: &str, cli: &str, parent: Option<&str>, archived: bool) {
        let spawned = parent.map_or_else(String::new, |p| {
            format!(r#","spawnedFrom":{{"sessionId":"{p}","sideSession":true,"name":"n"}}"#)
        });
        std::fs::write(
            folder.join(format!("{local}.json")),
            format!(
                r#"{{"sessionId":"{local}","cliSessionId":"{cli}","title":"T {cli}","isArchived":{archived},"cwd":"/w/p","publishedArtifacts":[{{"id":"x"}}],"remoteControlAutoEligible":true,"model":"m"{spawned}}}"#
            ),
        )
        .unwrap();
    }

    fn app() -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let folder = tmp.path().join("claude-code-sessions/acct/org");
        std::fs::create_dir_all(&folder).unwrap();
        (tmp, folder)
    }

    #[test]
    fn a_family_is_its_children_and_theirs_parents_first_archived_ones_too() {
        let (tmp, folder) = app();
        record(&folder, "local_p", "P", None, false);
        for n in 1..=20 {
            record(
                &folder,
                &format!("local_c{n:02}"),
                &format!("C{n}"),
                Some("local_p"),
                false,
            );
        }
        record(&folder, "local_g", "G", Some("local_c03"), false);
        record(&folder, "local_old", "OLD", Some("local_p"), true);
        record(&folder, "local_other", "O", None, false);
        let family = descendants(tmp.path(), "P");
        assert_eq!(family.len(), 22);
        let at = |c: &str| family.iter().position(|m| m.conversation == c).unwrap();
        assert!(at("C3") < at("G"));
        assert_eq!(family[at("G")].parent_conversation, "C3");
        assert!(family.iter().any(|m| m.conversation == "OLD"));
        assert!(family.iter().all(|m| m.conversation != "O"));
    }

    #[test]
    fn carried_records_point_at_the_imported_parent_and_their_carried_parents() {
        let (source, from) = app();
        record(&from, "local_p", "P", None, false);
        record(&from, "local_c", "C", Some("local_p"), false);
        record(&from, "local_g", "G", Some("local_c"), false);
        let family = descendants(source.path(), "P");
        let (target, to) = app();
        // The switch imported the parent through the app's own link.
        std::fs::write(
            to.join("local_P.json"),
            r#"{"sessionId":"local_P","cliSessionId":"P"}"#,
        )
        .unwrap();
        assert_eq!(
            records_folder_of(target.path(), "P").as_deref(),
            Some(to.as_path())
        );
        let placed = place(&family, &to, &[]).unwrap();
        assert_eq!(placed.len(), 2);
        let read = |name: &str| -> serde_json::Value {
            serde_json::from_slice(&std::fs::read(to.join(name)).unwrap()).unwrap()
        };
        let child = read("local_c.json");
        assert_eq!(child["spawnedFrom"]["sessionId"], "local_P");
        assert_eq!(child["spawnedFrom"]["sideSession"], true);
        assert_eq!(
            child["model"], "m",
            "the rest is carried as the app wrote it"
        );
        assert!(
            child.get("publishedArtifacts").is_none()
                && child.get("remoteControlAutoEligible").is_none()
        );
        assert_eq!(read("local_g.json")["spawnedFrom"]["sessionId"], "local_c");
        // The source records are unchanged.
        let source_child: serde_json::Value =
            serde_json::from_slice(&std::fs::read(from.join("local_c.json")).unwrap()).unwrap();
        assert_eq!(source_child["spawnedFrom"]["sessionId"], "local_p");
    }

    #[test]
    fn the_root_travels_with_its_record_at_the_top_and_its_children_keep_their_link() {
        let (source, from) = app();
        record(&from, "local_p", "P", Some("local_elsewhere"), false);
        record(&from, "local_c", "C", Some("local_p"), false);
        let mut family = vec![super::root(source.path(), "P").unwrap()];
        family.extend(descendants(source.path(), "P"));
        let (_target, to) = app();
        place(&family, &to, &[]).unwrap();
        let read = |name: &str| -> serde_json::Value {
            serde_json::from_slice(&std::fs::read(to.join(name)).unwrap()).unwrap()
        };
        assert!(
            read("local_p.json").get("spawnedFrom").is_none(),
            "at the top"
        );
        assert_eq!(read("local_c.json")["spawnedFrom"]["sessionId"], "local_p");
    }

    #[test]
    fn an_archived_child_stays_archived_unless_its_parent_was_archived_with_it() {
        let read = |path: &Path| -> serde_json::Value {
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
        };
        // Archived on its own, under an active parent: stays archived.
        let (source, from) = app();
        record(&from, "local_p", "P", None, false);
        record(&from, "local_c", "C", Some("local_p"), true);
        let (_target, to) = app();
        place(&descendants(source.path(), "P"), &to, &[]).unwrap();
        assert_eq!(read(&to.join("local_c.json"))["isArchived"], true);
        // Archived with its parent (the app archives the family): active.
        let (source, from) = app();
        record(&from, "local_p", "P", None, true);
        record(&from, "local_c", "C", Some("local_p"), true);
        let (_target, to) = app();
        place(&descendants(source.path(), "P"), &to, &[]).unwrap();
        assert_eq!(read(&to.join("local_c.json"))["isArchived"], false);
    }

    #[test]
    fn a_forked_parent_and_child_are_followed() {
        let (source, from) = app();
        record(&from, "local_p", "P", None, false);
        record(&from, "local_c", "C", Some("local_p"), false);
        let family = descendants(source.path(), "P");
        let (_target, to) = app();
        let renamed = [
            ("P".to_owned(), "P2".to_owned()),
            ("C".to_owned(), "C2".to_owned()),
        ];
        place(&family, &to, &renamed).unwrap();
        let child: serde_json::Value =
            serde_json::from_slice(&std::fs::read(to.join("local_c.json")).unwrap()).unwrap();
        assert_eq!(child["spawnedFrom"]["sessionId"], "local_P2");
        assert_eq!(child["cliSessionId"], "C2");
    }

    #[test]
    fn an_unknown_record_or_a_taken_name_places_nothing() {
        let (source, from) = app();
        record(&from, "local_p", "P", None, false);
        record(&from, "local_a", "A", Some("local_p"), false);
        record(&from, "local_b", "B", Some("local_p"), false);
        let family = descendants(source.path(), "P");
        let (_target, to) = app();
        std::fs::write(to.join("local_b.json"), "{}").unwrap();
        assert!(place(&family, &to, &[]).is_err());
        assert!(
            !to.join("local_a.json").exists(),
            "the first was taken back"
        );
        let odd = Member {
            record: from.join("local_p.json"),
            local_id: "local_p".into(),
            conversation: "P".into(),
            parent_conversation: "X".into(),
            title: None,
        };
        let error = place(&[odd], &to, &[]).unwrap_err().to_string();
        assert!(
            error.contains("isn't of a shape AI Juggler knows"),
            "{error}"
        );
    }
}
