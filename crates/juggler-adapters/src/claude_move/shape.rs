//! D-128: a move checks the **shape** of what it reads and rewrites in a
//! Claude app profile, on whatever version is installed, instead of
//! gating on the version number. It proceeds whenever every part it
//! relies on is as it knows it; anything else hides Move (a refusal the
//! user can't clear, D-127) and names what didn't fit, so the gate watch
//! lists it with the app's version.
//!
//! What a move relies on (read on the Claude app 4.x, 2026-11-08, and
//! 5.11492.5, 2026-11-18; `docs/research/claude-move-switch.md`):
//! - the layout `claude-code-sessions/<account>/<org>/`, holding one
//!   `local_<…>.json` record per conversation and, beside them, the app's
//!   own account-level files it knows: the archived-sessions hint (5.x,
//!   `{"v":1,"archived":[…]}`, which the app rebuilds from the records),
//!   scheduled tasks, artifacts, and the task backlog folder;
//! - each record's own id (`sessionId`, its file's name) and its
//!   conversation (`cliSessionId`), and the typed fields the plan reads
//!   (`spawnedFrom.sessionId`, `forkedFromSessionId`, `isArchived`, `cwd`);
//! - every link from one record to another under a key the plan knows
//!   (`super::REFERENCE_KEYS`, or inside an account-bound field a move
//!   drops). A record naming another under a key it doesn't know could tie
//!   a family together in a way the plan would cut.

use std::collections::BTreeSet;
use std::path::Path;

use super::{ACCOUNT_BOUND, LOCAL, RECORD_MAX, REFERENCE_KEYS};

/// The archived-sessions hint: a list the app keeps beside the records
/// and rebuilds from them (it loads without one).
pub(super) const ARCHIVE_HINT: &str = "archived-sessions.idx";
/// The hint's format the move knows.
const ARCHIVE_HINT_V: u64 = 1;
/// Account-level files beside the records a move leaves where they are.
const ACCOUNT_FILES: [&str; 2] = ["scheduled-tasks.json", "artifacts.json"];
/// Account-level folders beside the records a move leaves where they are.
const ACCOUNT_FOLDERS: [&str; 1] = ["backlog"];

/// What in `instance`'s session records doesn't have the shape a move
/// relies on, in words (empty: it fits). Reads only; a profile with no
/// records folder yet fits (nothing to rely on).
pub(super) fn app_mismatches(instance: &Path) -> Vec<String> {
    let root = crate::claude_live::app_root(instance);
    let mut out = BTreeSet::new();
    let Ok(accounts) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    for account in accounts.flatten() {
        if ignored(&account.file_name().to_string_lossy()) {
            continue;
        }
        if !account.file_type().is_ok_and(|kind| kind.is_dir()) {
            out.insert(format!(
                "a file where account folders are ({})",
                account.file_name().to_string_lossy()
            ));
            continue;
        }
        let Ok(orgs) = std::fs::read_dir(account.path()) else {
            continue;
        };
        for org in orgs.flatten() {
            if ignored(&org.file_name().to_string_lossy()) {
                continue;
            }
            if org.file_type().is_ok_and(|kind| kind.is_dir()) {
                org_folder(&org.path(), &mut out);
            } else {
                out.insert(format!(
                    "a file where organization folders are ({})",
                    org.file_name().to_string_lossy()
                ));
            }
        }
    }
    out.into_iter().collect()
}

/// A name the check passes over: the system's own (`.DS_Store`) and a
/// write in progress.
fn ignored(name: &str) -> bool {
    name.starts_with('.') || name.contains(".tmp")
}

/// One `<account>/<org>/` folder: every entry known, every record shaped.
fn org_folder(folder: &Path, out: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
        if ignored(&name) {
            continue;
        }
        let account_level = if is_dir {
            ACCOUNT_FOLDERS.contains(&name.as_str())
        } else {
            ACCOUNT_FILES.contains(&name.as_str())
        };
        if name.starts_with(LOCAL)
            && Path::new(&name)
                .extension()
                .is_some_and(|ext| ext == "json")
            && !is_dir
        {
            record(&entry.path(), out);
        } else if name == ARCHIVE_HINT && !is_dir {
            archive_hint(&entry.path(), out);
        } else if !account_level {
            out.insert(format!(
                "a {} beside the records AI Juggler doesn't know ({name})",
                if is_dir { "folder" } else { "file" }
            ));
        }
    }
}

/// The archived-sessions hint, in the format the move knows.
fn archive_hint(path: &Path, out: &mut BTreeSet<String>) {
    let value = std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let known = value.as_ref().is_some_and(|value| {
        value.get("v").and_then(serde_json::Value::as_u64) == Some(ARCHIVE_HINT_V)
            && value
                .get("archived")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|ids| ids.iter().all(serde_json::Value::is_string))
    });
    if !known {
        out.insert(format!(
            "the archived-sessions list is in a format AI Juggler doesn't know ({ARCHIVE_HINT})"
        ));
    }
}

/// One record: the typed fields the plan reads, and every link to another
/// record under a key it knows. A record that isn't JSON at all is the
/// plan's own business (it refuses when such a record names the family).
fn record(path: &Path, out: &mut BTreeSet<String>) {
    let Some(value) = std::fs::symlink_metadata(path)
        .ok()
        .filter(|meta| meta.len() <= RECORD_MAX)
        .and_then(|_| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
    else {
        return;
    };
    let Some(object) = value.as_object() else {
        out.insert("a session record that isn't an object".to_owned());
        return;
    };
    let typed = |key: &str, ok: fn(&serde_json::Value) -> bool| {
        object.get(key).is_none_or(|v| v.is_null() || ok(v))
    };
    for (key, ok) in [
        (
            "sessionId",
            serde_json::Value::is_string as fn(&serde_json::Value) -> bool,
        ),
        ("cliSessionId", serde_json::Value::is_string),
        ("forkedFromSessionId", serde_json::Value::is_string),
        ("isArchived", serde_json::Value::is_boolean),
        ("cwd", serde_json::Value::is_string),
        ("title", serde_json::Value::is_string),
        ("lastActivityAt", serde_json::Value::is_number),
        ("spawnedFrom", serde_json::Value::is_object),
    ] {
        if !typed(key, ok) {
            out.insert(format!("a session record's {key} is of another type"));
        }
    }
    if let Some(parent) = object.get("spawnedFrom").and_then(|v| v.get("sessionId"))
        && !parent.is_string()
    {
        out.insert("a session record's spawnedFrom.sessionId is of another type".to_owned());
    }
    for (key, inner) in object {
        if ACCOUNT_BOUND.contains(&key.as_str()) || key == "sessionId" {
            continue;
        }
        unknown_links(inner, key, out);
    }
}

/// Every place under `value` (at `at`) where a record is named by a key
/// the plan doesn't rebind.
fn unknown_links(value: &serde_json::Value, at: &str, out: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, inner) in map {
                unknown_links(inner, &format!("{at}.{key}"), out);
            }
        }
        serde_json::Value::Array(items) => {
            for inner in items {
                unknown_links(inner, &format!("{at}[]"), out);
            }
        }
        serde_json::Value::String(text) if text.starts_with(LOCAL) => {
            let key = at.rsplit(['.', '[']).next().unwrap_or(at);
            if !REFERENCE_KEYS.contains(&key) {
                out.insert(format!(
                    "session records name each other under a field AI Juggler doesn't know ({at})"
                ));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn org(dir: &Path) -> std::path::PathBuf {
        let org = dir.join("claude-code-sessions/acct/org");
        std::fs::create_dir_all(&org).unwrap();
        org
    }

    fn write(path: &Path, value: &serde_json::Value) {
        std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
    }

    /// The 5.x shape seen on 2026-11-18: a side session with its start
    /// chain, the archived-sessions hint, scheduled tasks, the backlog.
    #[test]
    fn the_app_5_shape_fits() {
        let tmp = tempfile::tempdir().unwrap();
        let org = org(tmp.path());
        write(
            &org.join("local_a.json"),
            &serde_json::json!({"sessionId": "local_a", "cliSessionId": "a", "isArchived": false, "cwd": "/w", "lastActivityAt": 1}),
        );
        write(
            &org.join("local_b.json"),
            &serde_json::json!({
                "sessionId": "local_b", "cliSessionId": "b", "isArchived": true,
                "spawnedFrom": {"sessionId": "local_a", "taskId": "t1", "sideSession": true},
                "startChain": {"depth": 1, "firstSessionId": "local_a"},
                "sideSessionNotes": [{"sessionId": "local_a"}],
                "peerReceipts": [{"targetSessionId": "local_z"}],
            }),
        );
        write(
            &org.join(ARCHIVE_HINT),
            &serde_json::json!({"v": 1, "archived": ["local_b"]}),
        );
        write(
            &org.join("scheduled-tasks.json"),
            &serde_json::json!({"scheduledTasks": []}),
        );
        std::fs::create_dir_all(org.join("backlog")).unwrap();
        std::fs::write(org.join(".DS_Store"), b"").unwrap();
        assert_eq!(app_mismatches(tmp.path()), Vec::<String>::new());
    }

    #[test]
    fn a_link_under_an_unknown_key_a_new_file_or_hint_format_doesnt_fit() {
        let tmp = tempfile::tempdir().unwrap();
        let org = org(tmp.path());
        write(
            &org.join("local_b.json"),
            &serde_json::json!({"sessionId": "local_b", "cliSessionId": "b", "family": {"rootRecord": "local_a"}}),
        );
        write(
            &org.join(ARCHIVE_HINT),
            &serde_json::json!({"v": 2, "archived": []}),
        );
        std::fs::write(org.join("sessions.db"), b"").unwrap();
        let found = app_mismatches(tmp.path());
        assert!(
            found.iter().any(|m| m.contains("(family.rootRecord)")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|m| m.contains("archived-sessions")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|m| m.contains("(sessions.db)")),
            "{found:?}"
        );
    }

    #[test]
    fn a_field_of_another_type_doesnt_fit() {
        let tmp = tempfile::tempdir().unwrap();
        let org = org(tmp.path());
        write(
            &org.join("local_b.json"),
            &serde_json::json!({"sessionId": "local_b", "cliSessionId": "b", "spawnedFrom": "local_a", "isArchived": "yes"}),
        );
        let found = app_mismatches(tmp.path());
        assert!(found.iter().any(|m| m.contains("spawnedFrom")), "{found:?}");
        assert!(found.iter().any(|m| m.contains("isArchived")), "{found:?}");
    }

    #[test]
    fn a_profile_without_records_yet_fits() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(app_mismatches(tmp.path()).is_empty());
    }
}
