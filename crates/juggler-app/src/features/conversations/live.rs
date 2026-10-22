//! AJ-E1529: which conversations a running session is on now. A process
//! is a session when its tool's adapter says so (never a helper, never a
//! desktop app's internals); its account comes from the folder its
//! environment points the tool at (else the tool's own default), and its
//! conversation from `--resume <id>`, else the newest one in its folder
//! that was written after it started.

use std::collections::HashSet;
use std::path::PathBuf;

use super::Conversation;

/// A running session, as the process table shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningSession {
    /// Its tool.
    pub tool: String,
    /// The account whose folder it uses, when one of the user's matches.
    pub account_id: Option<String>,
    /// Its working folder.
    pub cwd: Option<PathBuf>,
    /// The conversation its arguments resume.
    pub resumes: Option<String>,
    /// When it started, epoch ms.
    pub started_ms: Option<i64>,
}

/// A live conversation: `(account, tool, id)`.
pub type Live = (String, String, String);

/// A resumed session's key: on its own account.
fn live_key(tool: &str, id: &str, session: &RunningSession) -> Live {
    (
        session.account_id.clone().unwrap_or_default(),
        tool.to_owned(),
        id.to_owned(),
    )
}

/// The `(account, tool, id)` of every conversation a running session is
/// on. Each session is on one conversation at most, each conversation is
/// claimed once, and only on the session's own account (two accounts'
/// conversations can share an id).
#[must_use]
pub fn live_ids(conversations: &[Conversation], running: &[RunningSession]) -> HashSet<Live> {
    let mut live = HashSet::new();
    // A session on a folder that isn't one of the user's accounts is
    // nobody's here: never attributed to one.
    let running: Vec<&RunningSession> = running
        .iter()
        .filter(|session| session.account_id.is_some())
        .collect();
    let same_account = |conversation: &Conversation, session: &RunningSession| {
        session.account_id.as_deref() == Some(conversation.account_id.as_str())
    };
    // Resumed ones first: they name their conversation.
    for session in &running {
        if let Some(id) = &session.resumes
            && conversations
                .iter()
                .any(|c| c.tool == session.tool && c.summary.id == *id && same_account(c, session))
        {
            live.insert(live_key(&session.tool, id, session));
        }
    }
    for session in running.iter().filter(|session| session.resumes.is_none()) {
        let Some(cwd) = &session.cwd else { continue };
        let newest = conversations
            .iter()
            .filter(|c| c.tool == session.tool && same_account(c, session))
            .filter(|c| c.folder.as_ref() == Some(cwd))
            .filter(|c| {
                !live.contains(&(c.account_id.clone(), c.tool.clone(), c.summary.id.clone()))
            })
            .filter(|c| match (c.summary.last_ms, session.started_ms) {
                (Some(last), Some(started)) => last >= started,
                (Some(_), None) => true,
                _ => false,
            })
            .max_by_key(|c| c.summary.last_ms);
        if let Some(conversation) = newest {
            live.insert((
                conversation.account_id.clone(),
                conversation.tool.clone(),
                conversation.summary.id.clone(),
            ));
        }
    }
    live
}

/// The sessions running on this Mac now, by every registered tool's own
/// rule. Two passes: the whole table with executables and arguments,
/// then the environment and folder of the few that are sessions. Only
/// the tool's isolation variable is read from an environment.
#[must_use]
pub fn running_sessions(
    state: &std::sync::Arc<crate::state::AppState>,
    accounts: &[juggler_core::model::Account],
) -> Vec<RunningSession> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .without_tasks()
            .with_exe(UpdateKind::Always)
            .with_cmd(UpdateKind::Always),
    );
    let adapters: Vec<_> = state
        .registry
        .ids()
        .into_iter()
        .filter_map(|id| state.registry.get(&id).map(|adapter| (id, adapter)))
        .collect();
    let mut found: Vec<(Pid, Option<Pid>, usize, Option<String>)> = Vec::new();
    for (pid, process) in system.processes() {
        let args: Vec<String> = process
            .cmd()
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        for (index, (_, adapter)) in adapters.iter().enumerate() {
            if let Some(session) = adapter.session_process(process.exe(), &args) {
                found.push((*pid, process.parent(), index, session.resumes));
                break;
            }
        }
    }
    // A session started by another of its tool's processes (a launcher,
    // a node shim) is that one's: the outermost is the session.
    let pids: HashSet<(Pid, usize)> = found
        .iter()
        .map(|(pid, _, tool, _)| (*pid, *tool))
        .collect();
    found.retain(|(_, parent, tool, _)| {
        parent.is_none_or(|parent| !pids.contains(&(parent, *tool)))
    });
    let wanted: Vec<Pid> = found.iter().map(|(pid, ..)| *pid).collect();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&wanted),
        false,
        ProcessRefreshKind::nothing()
            .without_tasks()
            .with_environ(UpdateKind::Always)
            .with_cwd(UpdateKind::Always),
    );
    let canonical =
        |path: &std::path::Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    found
        .into_iter()
        .filter_map(|(pid, _, tool, resumes)| {
            let process = system.process(pid)?;
            let (id, adapter) = &adapters[tool];
            let keys = adapter.isolation_env_keys();
            let pointed = process.environ().iter().find_map(|entry| {
                let entry = entry.to_str()?;
                keys.iter()
                    .find_map(|key| {
                        entry
                            .strip_prefix(key)?
                            .strip_prefix('=')
                            .filter(|value| !value.is_empty())
                    })
                    .map(std::path::PathBuf::from)
            });
            let folder = pointed
                .or_else(|| adapter.vendor_default_dir())
                .map(|path| canonical(&path));
            let account_id = folder.and_then(|folder| {
                accounts
                    .iter()
                    .find(|account| {
                        account.harness == *id && canonical(&account.state_dir) == folder
                    })
                    .map(|account| account.id.to_string())
            });
            Some(RunningSession {
                tool: id.to_string(),
                account_id,
                cwd: process.cwd().map(canonical),
                resumes,
                started_ms: i64::try_from(process.start_time())
                    .ok()
                    .map(|secs| secs * 1000),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_adapter::conversation::ConversationSummary;

    fn conversation(id: &str, account: &str, cwd: &str, last_ms: i64) -> Conversation {
        Conversation {
            summary: ConversationSummary {
                id: id.into(),
                cwd: Some(cwd.into()),
                last_ms: Some(last_ms),
                ..ConversationSummary::default()
            },
            account_id: account.into(),
            tool: "t".into(),
            path: format!("/x/{id}.jsonl").into(),
            folder: Some(cwd.into()),
        }
    }

    fn running(
        account: Option<&str>,
        cwd: &str,
        resumes: Option<&str>,
        started_ms: i64,
    ) -> RunningSession {
        RunningSession {
            tool: "t".into(),
            account_id: account.map(Into::into),
            cwd: Some(cwd.into()),
            resumes: resumes.map(Into::into),
            started_ms: Some(started_ms),
        }
    }

    #[test]
    fn a_resumed_session_names_its_conversation() {
        let all = [
            conversation("old", "a", "/w", 10),
            conversation("new", "a", "/w", 50),
        ];
        let live = live_ids(&all, &[running(Some("a"), "/w", Some("old"), 100)]);
        assert_eq!(
            live,
            HashSet::from([("a".into(), "t".into(), "old".into())])
        );
    }

    #[test]
    fn otherwise_the_newest_in_its_folder_written_since_it_started() {
        let all = [
            conversation("before", "a", "/w", 10),
            conversation("since", "a", "/w", 150),
            conversation("elsewhere", "a", "/v", 200),
        ];
        assert_eq!(
            live_ids(&all, &[running(Some("a"), "/w", None, 100)]),
            HashSet::from([("a".into(), "t".into(), "since".into())])
        );
        // Nothing written since it started (it hasn't said anything yet): none.
        assert!(live_ids(&all[..1], &[running(Some("a"), "/w", None, 100)]).is_empty());
    }

    #[test]
    fn a_session_on_none_of_the_users_accounts_is_nobodys() {
        let all = [conversation("c", "a", "/w", 150)];
        assert!(live_ids(&all, &[running(None, "/w", None, 100)]).is_empty());
        assert!(live_ids(&all, &[running(None, "/w", Some("c"), 100)]).is_empty());
    }

    #[test]
    fn two_sessions_in_one_folder_claim_two_conversations_on_their_own_accounts() {
        let all = [
            conversation("a1", "a", "/w", 150),
            conversation("b1", "b", "/w", 160),
            conversation("a2", "a", "/w", 170),
        ];
        let live = live_ids(
            &all,
            &[
                running(Some("a"), "/w", None, 100),
                running(Some("a"), "/w", None, 100),
                running(Some("b"), "/w", None, 100),
            ],
        );
        assert_eq!(live.len(), 3);
    }

    #[test]
    fn a_live_id_is_live_only_on_its_own_account() {
        // Two accounts' conversations with one id: only the running
        // session's account's is live.
        let all = [
            conversation("same", "a", "/w", 150),
            conversation("same", "b", "/v", 150),
        ];
        let live = live_ids(&all, &[running(Some("a"), "/w", Some("same"), 100)]);
        assert_eq!(
            live,
            HashSet::from([("a".into(), "t".into(), "same".into())])
        );
    }
}
