//! ADR-0028 (`automation_handover`) for the Claude app.
//!
//! The app keeps what wakes a session by itself in its record of it
//! (`claude-code-sessions/<account>/<org>/local_<id>.json`), read when the
//! app starts:
//! - Auto-fix: each pull request the session made or watches is listed
//!   under `prs`; the app fixes one's failing checks while it has
//!   `autoFix: true` and isn't `inherited`, `dismissed`, unconfirmed or
//!   closed. `autoFixOwnPrs` turns it on for the session's next ones.
//! - Resuming by itself: after a usage limit resets, after the app quit
//!   mid-turn, after the Mac slept. The app skips all of these, and
//!   Auto-fix, for an archived session.
//!
//! The app's own moves work the same way: a session moved to the cloud is
//! archived by the app at its next start, and its forks get its pull
//! requests marked `inherited`, without Auto-fix. So a switch:
//! - gives the copy's record the original's Auto-fix ([`adopt`]);
//! - puts the original's record away: `isArchived`, Auto-fix off
//!   ([`quiet`]); only the record changes, never the conversation, and
//!   unlike the app's Archive nothing else is touched (its worktree
//!   stays);
//! - switching back unarchives it ([`wake`]) and adopts the copy's.
//!
//! Records are rewritten whole, atomically, keeping every field this code
//! doesn't know; a record of a shape it doesn't know is refused.

use std::path::{Path, PathBuf};

use juggler_adapter::automation::{
    Applied, Automation, AutomationHome, HostAutomation, renamed_to,
};
use juggler_adapter::error::AdapterError;
use serde_json::{Map, Value, json};

/// The Claude app's automation, for the adapter's
/// [`juggler_adapter::HarnessAdapter::host_automation`].
pub struct ClaudeAppAutomation;

impl HostAutomation for ClaudeAppAutomation {
    fn home(&self) -> AutomationHome {
        AutomationHome::AppProfile
    }

    fn read(&self, home: &Path, conversations: &[String]) -> Result<Automation, AdapterError> {
        Ok(read(home, conversations))
    }

    fn adopt(
        &self,
        home: &Path,
        carried: &Value,
        renamed: &[(String, String)],
    ) -> Result<Applied, AdapterError> {
        adopt(home, carried, renamed)
    }

    fn quiet(&self, home: &Path, conversations: &[String]) -> Result<Value, AdapterError> {
        quiet(home, conversations)
    }

    fn wake(&self, home: &Path, restore: &Value) -> Result<Applied, AdapterError> {
        wake(home, restore)
    }
}

/// Whether the app fixes `pr`'s checks by itself (its own rule).
fn fixes(pr: &Value) -> bool {
    let flag = |key: &str| pr.get(key).and_then(Value::as_bool) == Some(true);
    let open = !pr
        .get("state")
        .and_then(Value::as_str)
        .is_some_and(|state| matches!(state.to_ascii_uppercase().as_str(), "MERGED" | "CLOSED"));
    let github = pr
        .get("provider")
        .and_then(Value::as_str)
        .is_none_or(|provider| provider == "github" || provider == "github-enterprise");
    flag("autoFix")
        && !flag("inherited")
        && !flag("dismissed")
        && pr.get("unconfirmed").is_none()
        && open
        && github
}

/// The pull request's key: its number, repository and host.
fn key(pr: &Value) -> Option<(u64, String, String)> {
    Some((
        pr.get("prNumber")?.as_u64()?,
        pr.get("repo")?.as_str()?.to_ascii_lowercase(),
        pr.get("host")
            .and_then(Value::as_str)
            .unwrap_or("github.com")
            .to_ascii_lowercase(),
    ))
}

/// A record, checked to be one this code knows (its id matches its file,
/// it names its conversation).
fn known(path: &Path) -> Result<Map<String, Value>, AdapterError> {
    let refuse = || {
        AdapterError::Unsupported(format!(
            "the Claude app's record {} isn't of a shape AI Juggler knows, so its automation wasn't changed",
            path.display()
        ))
    };
    let value = crate::claude_app_family::read(path).ok_or_else(refuse)?;
    let Value::Object(object) = value else {
        return Err(refuse());
    };
    let stem = path.file_stem().and_then(|s| s.to_str());
    if object.get("sessionId").and_then(Value::as_str) != stem
        || object.get("cliSessionId").and_then(Value::as_str).is_none()
        || object.get("prs").is_some_and(|prs| !prs.is_array())
    {
        return Err(refuse());
    }
    Ok(object)
}

/// Rewrites `path` with `object`, atomically, as private as it was.
fn rewrite(path: &Path, object: Map<String, Value>) -> Result<(), AdapterError> {
    let failed = |error: std::io::Error| AdapterError::AccountDir {
        path: path.to_path_buf(),
        message: error.to_string(),
    };
    let bytes =
        serde_json::to_vec(&Value::Object(object)).map_err(|e| failed(std::io::Error::other(e)))?;
    let staged = path.with_extension("json.aij-new");
    {
        use std::io::Write as _;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&staged).map_err(failed)?;
        file.write_all(&bytes).map_err(failed)?;
        file.sync_all().map_err(failed)?;
    }
    std::fs::rename(&staged, path).map_err(|error| {
        let _ = std::fs::remove_file(&staged);
        failed(error)
    })
}

/// What runs by itself for `conversations` in the app instance `home`.
#[must_use]
pub fn read(home: &Path, conversations: &[String]) -> Automation {
    let mut automation = Automation::default();
    let mut carried = Map::new();
    for conversation in conversations {
        let mut prs: Vec<Value> = Vec::new();
        let mut own_prs = false;
        for path in crate::claude_app_family::records_of(home, conversation) {
            let Some(record) = crate::claude_app_family::read(&path) else {
                continue;
            };
            if record.get("isArchived").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            for pr in record
                .get("prs")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|pr| fixes(pr))
            {
                if !prs.iter().any(|seen| key(seen) == key(pr)) {
                    prs.push(pr.clone());
                }
            }
            own_prs |= record.get("autoFixOwnPrs").and_then(Value::as_bool) == Some(true);
        }
        for pr in &prs {
            if let Some((number, repo, _)) = key(pr) {
                automation
                    .what
                    .push(format!("Auto-fix on pull request #{number} ({repo})"));
            }
        }
        if own_prs {
            automation
                .what
                .push("Auto-fix on the pull requests it opens next".to_owned());
        }
        if !prs.is_empty() || own_prs {
            carried.insert(
                conversation.clone(),
                json!({ "prs": prs, "ownPrs": own_prs }),
            );
        }
    }
    automation.carried = Value::Object(carried);
    automation
}

/// Gives the copies in `home` the automation `carried` says their
/// originals had. [`Applied::NotYet`] while a copy has no record there
/// (the app imports a conversation when it's opened).
///
/// # Errors
///
/// A record of a shape this code doesn't know; the write that failed.
pub fn adopt(
    home: &Path,
    carried: &Value,
    renamed: &[(String, String)],
) -> Result<Applied, AdapterError> {
    let Some(carried) = carried.as_object() else {
        return Ok(Applied::Done);
    };
    // All found first: none changes unless every one can be.
    let mut work: Vec<(PathBuf, &Value)> = Vec::new();
    for (conversation, what) in carried {
        let target = renamed_to(renamed, conversation);
        let Some(path) = crate::claude_app_family::records_of(home, target)
            .into_iter()
            .next()
        else {
            return Ok(Applied::NotYet);
        };
        work.push((path, what));
    }
    for (path, what) in work {
        let mut record = known(&path)?;
        let prs = record
            .entry("prs")
            .or_insert_with(|| Value::Array(Vec::new()));
        let Value::Array(prs) = prs else {
            continue;
        };
        for pr in what
            .get("prs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Value::Object(handed) = pr.clone() else {
                continue;
            };
            match prs.iter_mut().find(|held| key(held) == key(pr)) {
                Some(Value::Object(held)) => {
                    held.insert("autoFix".to_owned(), Value::Bool(true));
                    held.remove("inherited");
                    held.remove("dismissed");
                    if handed.get("own").and_then(Value::as_bool) == Some(true) {
                        held.insert("own".to_owned(), Value::Bool(true));
                    }
                }
                _ => prs.push(Value::Object(handed)),
            }
        }
        if what.get("ownPrs").and_then(Value::as_bool) == Some(true) {
            record.insert("autoFixOwnPrs".to_owned(), Value::Bool(true));
        }
        rewrite(&path, record)?;
    }
    Ok(Applied::Done)
}

/// Puts the originals' records in `home` away: archived, Auto-fix off.
/// Returns, per record, what it was, for [`wake`].
///
/// # Errors
///
/// A record of a shape this code doesn't know (nothing is changed then);
/// the write that failed.
pub fn quiet(home: &Path, conversations: &[String]) -> Result<Value, AdapterError> {
    let mut records: Vec<(PathBuf, Map<String, Value>)> = Vec::new();
    for conversation in conversations {
        for path in crate::claude_app_family::records_of(home, conversation) {
            let record = known(&path)?;
            records.push((path, record));
        }
    }
    let mut restore = Vec::new();
    for (path, mut record) in records {
        let archived = record.get("isArchived").and_then(Value::as_bool) == Some(true);
        let own_prs = record.get("autoFixOwnPrs").and_then(Value::as_bool) == Some(true);
        let mut fixed: Vec<Value> = Vec::new();
        if let Some(Value::Array(prs)) = record.get_mut("prs") {
            for pr in prs.iter_mut() {
                if pr.get("autoFix").and_then(Value::as_bool) == Some(true)
                    && let Some(object) = pr.as_object_mut()
                {
                    object.insert("autoFix".to_owned(), Value::Bool(false));
                    if let (Some(number), Some(repo)) = (object.get("prNumber"), object.get("repo"))
                    {
                        fixed.push(
                            json!({ "prNumber": number, "repo": repo, "host": object.get("host") }),
                        );
                    }
                }
            }
        }
        if archived && fixed.is_empty() && !own_prs {
            continue;
        }
        record.insert("isArchived".to_owned(), Value::Bool(true));
        if own_prs {
            record.insert("autoFixOwnPrs".to_owned(), Value::Bool(false));
        }
        rewrite(&path, record)?;
        restore.push(json!({
            "record": path,
            "archived": archived,
            "autoFix": fixed,
            "ownPrs": own_prs,
        }));
    }
    Ok(Value::Array(restore))
}

/// Undoes [`quiet`]: each record as it was (unarchived unless it was
/// archived before, its Auto-fix back). A record that's gone is skipped.
///
/// # Errors
///
/// As [`quiet`].
pub fn wake(home: &Path, restore: &Value) -> Result<Applied, AdapterError> {
    for entry in restore.as_array().into_iter().flatten() {
        let Some(path) = entry
            .get("record")
            .and_then(Value::as_str)
            .map(PathBuf::from)
        else {
            continue;
        };
        // Only a record inside this app instance.
        if !path.starts_with(home) || !path.is_file() {
            continue;
        }
        let mut record = known(&path)?;
        let was_archived = entry.get("archived").and_then(Value::as_bool) == Some(true);
        record.insert("isArchived".to_owned(), Value::Bool(was_archived));
        if entry.get("ownPrs").and_then(Value::as_bool) == Some(true) {
            record.insert("autoFixOwnPrs".to_owned(), Value::Bool(true));
        }
        let keys: Vec<_> = entry
            .get("autoFix")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(key)
            .collect();
        if let Some(Value::Array(prs)) = record.get_mut("prs") {
            for pr in prs.iter_mut() {
                if key(pr).is_some_and(|k| keys.contains(&k))
                    && let Some(object) = pr.as_object_mut()
                {
                    object.insert("autoFix".to_owned(), Value::Bool(true));
                }
            }
        }
        rewrite(&path, record)?;
    }
    Ok(Applied::Done)
}

/// A carried record's pull requests, as the app's own fork carries them:
/// the open ones, marked `inherited`, without Auto-fix (which only
/// [`adopt`] gives, once the original's is turned off).
pub(crate) fn as_inherited(record: &mut Map<String, Value>) {
    if let Some(Value::Array(prs)) = record.get_mut("prs") {
        prs.retain(|pr| {
            !pr.get("state")
                .and_then(Value::as_str)
                .is_some_and(|state| {
                    matches!(state.to_ascii_uppercase().as_str(), "MERGED" | "CLOSED")
                })
        });
        for pr in prs.iter_mut() {
            if let Some(object) = pr.as_object_mut() {
                object.remove("autoFix");
                object.remove("own");
                object.insert("inherited".to_owned(), Value::Bool(true));
            }
        }
    }
    for flag in [
        "autoFixOwnPrs",
        "autoFixOwnPrsOrigin",
        "autoFixNextOwnPr",
        "autoFixNoticeSent",
    ] {
        record.remove(flag);
    }
}

#[cfg(test)]
mod tests {
    use super::{adopt, quiet, read, wake};
    use serde_json::{Value, json};
    use std::path::{Path, PathBuf};

    fn app() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let folder = tmp.path().join("claude-code-sessions/acct/org");
        std::fs::create_dir_all(&folder).unwrap();
        (tmp, folder)
    }

    fn write(folder: &Path, local: &str, record: &Value) -> PathBuf {
        let path = folder.join(format!("{local}.json"));
        std::fs::write(&path, serde_json::to_vec(record).unwrap()).unwrap();
        path
    }

    fn get(path: &Path) -> Value {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    fn original(folder: &Path) -> PathBuf {
        write(
            folder,
            "local_orig",
            &json!({
                "sessionId": "local_orig", "cliSessionId": "P", "title": "Release", "model": "m",
                "prs": [
                    {"prNumber": 210, "repo": "Org/Repo", "host": "github.com", "state": "OPEN", "own": true, "autoFix": true},
                    {"prNumber": 208, "repo": "Org/Repo", "state": "MERGED", "own": true, "autoFix": true},
                    {"prNumber": 300, "repo": "Org/Repo", "state": "OPEN", "inherited": true, "autoFix": true},
                    {"prNumber": 301, "repo": "Org/Repo", "state": "OPEN"}
                ]
            }),
        )
    }

    #[test]
    fn reads_only_what_the_app_would_run() {
        let (home, folder) = app();
        original(&folder);
        let found = read(home.path(), &["P".to_owned()]);
        assert_eq!(
            found.what,
            vec!["Auto-fix on pull request #210 (org/repo)".to_owned()]
        );
        assert_eq!(found.carried["P"]["prs"].as_array().unwrap().len(), 1);
        assert!(read(home.path(), &["NONE".to_owned()]).what.is_empty());
    }

    #[test]
    fn the_copy_takes_it_over_and_the_original_is_put_away_then_back() {
        let (source, from) = app();
        let orig = original(&from);
        let found = read(source.path(), &["P".to_owned()]);

        // The copy, as the target app imported it (no pull requests yet).
        let (target, to) = app();
        let renamed = [("P".to_owned(), "P2".to_owned())];
        assert_eq!(
            adopt(target.path(), &found.carried, &renamed).unwrap(),
            juggler_adapter::automation::Applied::NotYet,
            "waits for the app to import it"
        );
        let copy = write(
            &to,
            "local_P2",
            &json!({"sessionId": "local_P2", "cliSessionId": "P2"}),
        );
        adopt(target.path(), &found.carried, &renamed).unwrap();
        let copied = get(&copy);
        assert_eq!(copied["prs"][0]["prNumber"], 210);
        assert_eq!(copied["prs"][0]["autoFix"], true);
        assert_eq!(read(target.path(), &["P2".to_owned()]).what.len(), 1);

        let restore = quiet(source.path(), &["P".to_owned()]).unwrap();
        let put_away = get(&orig);
        assert_eq!(put_away["isArchived"], true);
        assert!(
            put_away["prs"]
                .as_array()
                .unwrap()
                .iter()
                .all(|pr| pr["autoFix"] != true)
        );
        assert_eq!(put_away["model"], "m", "everything else as it was");
        assert!(read(source.path(), &["P".to_owned()]).what.is_empty());

        wake(source.path(), &restore).unwrap();
        let back = get(&orig);
        assert_eq!(back["isArchived"], false);
        assert_eq!(back["prs"][0]["autoFix"], true);
        assert_eq!(back["prs"][1]["autoFix"], true, "as it was, merged or not");
    }

    #[test]
    fn an_unknown_record_changes_nothing() {
        let (home, folder) = app();
        let good = original(&folder);
        // The app's own record, but its pull requests aren't a list.
        write(
            &folder,
            "local_odd",
            &json!({"sessionId": "local_odd", "cliSessionId": "P", "prs": "x"}),
        );
        assert!(quiet(home.path(), &["P".to_owned()]).is_err());
        assert_eq!(get(&good).get("isArchived"), None, "untouched");
    }

    #[test]
    fn wake_never_writes_outside_its_app() {
        let (home, _) = app();
        let (_other, elsewhere) = app();
        let stray = write(
            &elsewhere,
            "local_x",
            &json!({"sessionId": "local_x", "cliSessionId": "X", "isArchived": true}),
        );
        wake(home.path(), &json!([{"record": stray, "archived": false}])).unwrap();
        assert_eq!(get(&stray)["isArchived"], true);
    }
}
