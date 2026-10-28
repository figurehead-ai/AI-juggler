//! AJ-E1520 (0.2): the rail's subagent count (I-08, a row's
//! `subtask_count`): how many subagents each live session (and its
//! subagents) has running now, read by the same readers as the session's
//! Subagents tab — one source, for every tool whose adapter lists
//! subagents (`HarnessAdapter::subagent_reader`), none named here.
//!
//! Every live session the rail shows is followed: AI Juggler's own (its
//! row, `InstanceInfo`) and the ones other apps run (their conversation's
//! row, `ConversationInfo`), so a Codex or Kimi Code session open in a
//! terminal is counted as a Claude Code session in AI Juggler is.
//!
//! Its own tick, every [`EVERY`], follows only the sessions that haven't
//! ended: each one's reader is refreshed once a pass, which reads only
//! what its records gained, within the reader's own budget (a very long
//! transcript is read over several passes). A count is published once
//! its reader has caught up, and kept while a later pass catches up
//! again; until the first, it is `null` — never a guessed 0. A session
//! that ends leaves the count (D-06: once it has ended, a subagent its
//! records never saw end is unknown), and its reader goes with it unless
//! its tab still keeps it. Rows only read what the tick published: the
//! rail's lists never read a tool's files.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapter::subagents::{Subagent, SubagentReader, SubagentScope, SubagentStatus};
use juggler_core::model::{HarnessId, ProfileId};

use super::{Entry, Key, Readers};
use crate::features::conversations::dto::ConversationInfo;
use crate::features::sessions::dto::InstanceInfo;
use crate::state::AppState;

/// How often the count is read: the 2 s the process sampling uses.
pub const EVERY: std::time::Duration = std::time::Duration::from_secs(2);

/// Starts the count's own tick: each pass refreshes the live sessions'
/// readers (within their byte budgets) and tells the rail only when a
/// count changed — the list whose row it is — so a slow store never
/// holds up the process sampling.
pub fn start(app: tauri::AppHandle, state: Arc<AppState>) {
    crate::ticks::every("subagents", EVERY, move || {
        let changed = tick(&state);
        if changed.sessions {
            let _ = crate::commands::emit_sessions(&app, &state);
        }
        if changed.conversations {
            use tauri::Emitter;
            let _ = app.emit(
                crate::features::conversations::EVENT_CHANGED,
                crate::features::conversations::CHANGED_LIVE,
            );
        }
    });
}

/// The rail row a count is for.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Row {
    /// AI Juggler's own session, by its id.
    Session(String),
    /// A session another app runs: its conversation, by account and the
    /// tool's id for it.
    Conversation(String, String),
}

/// Which lists' counts a pass changed (each is told on its own).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Changed {
    /// A count on AI Juggler's own sessions' rows.
    pub sessions: bool,
    /// A count on the rows of sessions other apps run.
    pub conversations: bool,
}

impl Changed {
    /// Whether any count changed.
    #[must_use]
    pub fn any(self) -> bool {
        self.sessions || self.conversations
    }
}

/// A live session the count follows.
pub(crate) struct Followed {
    /// The row its count is for.
    pub row: Row,
    /// Its conversation, as the tab keeps its reader.
    pub key: Key,
    /// Its tool, whose adapter reads it.
    pub tool: String,
}

/// Makes a reader for a tool's conversation; `None` when its tool lists
/// no subagents.
pub(crate) type MakeReader<'a> =
    &'a dyn Fn(&str, SubagentScope<'_>) -> Option<Box<dyn SubagentReader>>;

/// Each row's running subagents, as the tick last read them.
#[must_use]
pub fn counted(state: &Arc<AppState>, mut rows: Vec<InstanceInfo>) -> Vec<InstanceInfo> {
    let Ok(counts) = state.subagents.counts.lock() else {
        return rows;
    };
    for row in &mut rows {
        row.subtask_count = counts.get(&Row::Session(row.id.clone())).copied();
    }
    rows
}

/// Each conversation row's running subagents, as the tick last read
/// them: the sessions other apps run (AI Juggler's own are counted on
/// their session's row).
#[must_use]
pub fn counted_conversations(
    state: &Arc<AppState>,
    mut rows: Vec<ConversationInfo>,
) -> Vec<ConversationInfo> {
    let Ok(counts) = state.subagents.counts.lock() else {
        return rows;
    };
    for row in &mut rows {
        row.subtask_count = counts
            .get(&Row::Conversation(row.account_id.clone(), row.id.clone()))
            .copied();
    }
    rows
}

/// Reads each live session's subagents and publishes the counts; says
/// which lists' counts changed (the caller tells the UI).
pub fn tick(state: &Arc<AppState>) -> Changed {
    let mut followed = live_sessions(state);
    followed.extend(live_conversations(state, &followed));
    pass(&state.subagents, followed, &|tool, scope| {
        reader_for(&state.registry, tool, scope)
    })
}

/// The reader the count makes for `tool`'s conversation `scope`: its
/// adapter's, when that adapter lists subagents (the seam: no tool is
/// named here); `None` otherwise.
#[must_use]
pub fn reader_for(
    registry: &AdapterRegistry,
    tool: &str,
    scope: SubagentScope<'_>,
) -> Option<Box<dyn SubagentReader>> {
    let adapter = registry.get(&HarnessId::new(tool))?;
    if !adapter.subagent_support().is_supported() {
        return None;
    }
    adapter.subagent_reader(scope)
}

/// The sessions that haven't ended, each with its conversation as the
/// conversations index has it (the tab's own key). One not indexed yet
/// (a session that just started) is followed from the next pass that
/// finds it.
fn live_sessions(state: &Arc<AppState>) -> Vec<Followed> {
    let wanted: Vec<(String, String, String)> = {
        let Ok(storage) = state.storage.lock() else {
            return Vec::new();
        };
        let Ok(live) = storage.sessions().live() else {
            return Vec::new();
        };
        let mut accounts: HashMap<ProfileId, Option<String>> = HashMap::new();
        live.into_iter()
            .filter_map(|record| {
                let conversation = record.conversation?;
                let account = accounts
                    .entry(record.profile.clone())
                    .or_insert_with(|| {
                        storage
                            .profiles()
                            .get(&record.profile)
                            .ok()
                            .map(|profile| profile.account.to_string())
                    })
                    .clone()?;
                Some((
                    record.id.to_string(),
                    account,
                    conversation.as_str().to_owned(),
                ))
            })
            .collect()
    };
    wanted
        .into_iter()
        .filter_map(|(session, account, id)| {
            let conversation = state.conversations.conversation_of(&account, &id)?;
            Some(Followed {
                row: Row::Session(session),
                tool: conversation.tool,
                key: (account, id, conversation.path),
            })
        })
        .collect()
}

/// The conversations a running session is on that no session in `own`
/// follows: the sessions other apps run (and an own session the tool
/// never named its conversation to), each by its conversation's row.
fn live_conversations(state: &Arc<AppState>, own: &[Followed]) -> Vec<Followed> {
    let followed: HashSet<(&str, &str)> = own
        .iter()
        .map(|one| (one.key.0.as_str(), one.key.1.as_str()))
        .collect();
    state
        .conversations
        .live_keys()
        .into_iter()
        .filter(|(account, _, id)| !followed.contains(&(account.as_str(), id.as_str())))
        .filter_map(|(account, tool, id)| {
            let conversation = state.conversations.conversation_of(&account, &id)?;
            (conversation.tool == tool).then(|| Followed {
                row: Row::Conversation(account.clone(), id.clone()),
                tool,
                key: (account, id, conversation.path),
            })
        })
        .collect()
}

/// One pass over `followed`: refreshes each one's reader (the tab's when
/// it keeps one, else its own, made by `make`), publishes the counts and
/// lets the readers of sessions no longer followed go. Says which lists'
/// counts changed.
pub(crate) fn pass(readers: &Readers, followed: Vec<Followed>, make: MakeReader<'_>) -> Changed {
    let before: HashMap<Row, u64> = readers
        .counts
        .lock()
        .map(|counts| counts.clone())
        .unwrap_or_default();
    let mut counts: HashMap<Row, u64> = HashMap::with_capacity(followed.len());
    let mut live: HashMap<Row, Arc<Entry>> = HashMap::with_capacity(followed.len());
    for one in followed {
        let kept = readers.find(&one.key).or_else(|| {
            let scope = SubagentScope {
                path: &one.key.2,
                id: &one.key.1,
            };
            make(&one.tool, scope).map(|reader| Entry::new(one.key.clone(), reader))
        });
        let Some(entry) = kept else {
            continue;
        };
        match entry.read() {
            Some((Ok(true), agents)) => {
                counts.insert(one.row.clone(), running(&agents));
            }
            // Still catching up: the last count stands until it has.
            Some((Ok(false), _)) => {
                if let Some(&last) = before.get(&one.row) {
                    counts.insert(one.row.clone(), last);
                }
            }
            // No record yet, or not readable now: not counted.
            Some((Err(_), _)) | None => {}
        }
        live.insert(one.row, entry);
    }
    if let Ok(mut kept) = readers.kept.lock() {
        kept.live = live;
    }
    let changed = changed(&before, &counts);
    if let Ok(mut published) = readers.counts.lock() {
        *published = counts;
    }
    changed
}

/// Which lists' rows have a count that differs between `before` and
/// `after` (one gained, lost or changed).
fn changed(before: &HashMap<Row, u64>, after: &HashMap<Row, u64>) -> Changed {
    let mut out = Changed::default();
    let differs = before
        .iter()
        .filter(|(row, count)| after.get(*row) != Some(*count))
        .chain(after.iter().filter(|(row, _)| !before.contains_key(*row)))
        .map(|(row, _)| row);
    for row in differs {
        match row {
            Row::Session(_) => out.sessions = true,
            Row::Conversation(..) => out.conversations = true,
        }
    }
    out
}

/// How many are running: started, and nothing recorded says they ended
/// (the session is live, so running is what the tab says too).
#[must_use]
pub fn running(agents: &[Subagent]) -> u64 {
    let count = agents
        .iter()
        .filter(|agent| agent.status == SubagentStatus::Running)
        .count();
    u64::try_from(count).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use std::cell::Cell;
    use std::path::PathBuf;
    use std::sync::Mutex;

    use super::*;

    /// What a fake tool's records say on each refresh: whether it has
    /// caught up, and the statuses of its subagents.
    type Script = Arc<Mutex<Vec<std::io::Result<(bool, Vec<SubagentStatus>)>>>>;

    struct Fake {
        script: Script,
        agents: Vec<Subagent>,
    }

    impl SubagentReader for Fake {
        fn refresh(&mut self) -> std::io::Result<bool> {
            let next = {
                let mut script = self.script.lock().unwrap();
                if script.len() > 1 {
                    script.remove(0)
                } else {
                    match script.first() {
                        Some(Ok(step)) => Ok(step.clone()),
                        Some(Err(_)) | None => Err(std::io::Error::other("gone")),
                    }
                }
            };
            let (caught_up, statuses) = next?;
            self.agents = statuses
                .into_iter()
                .enumerate()
                .map(|(n, status)| Subagent {
                    id: format!("a{n}"),
                    kind: None,
                    task: None,
                    status,
                    started_ms: None,
                    ended_ms: None,
                    parent: None,
                    depth: 1,
                    background: None,
                    transcript: None,
                })
                .collect();
            Ok(caught_up)
        }

        fn subagents(&self) -> Vec<Subagent> {
            self.agents.clone()
        }
    }

    fn followed(session: &str) -> Followed {
        Followed {
            row: Row::Session(session.to_owned()),
            key: (
                "acc".to_owned(),
                format!("conv-{session}"),
                PathBuf::from(format!("/nowhere/{session}.jsonl")),
            ),
            tool: "any-tool".to_owned(),
        }
    }

    fn count(readers: &Readers, session: &str) -> Option<u64> {
        readers
            .counts
            .lock()
            .unwrap()
            .get(&Row::Session(session.to_owned()))
            .copied()
    }

    /// A session another app runs, on `tool`: its conversation's row.
    fn elsewhere(id: &str, tool: &str) -> Followed {
        Followed {
            row: Row::Conversation("acc".to_owned(), id.to_owned()),
            key: (
                "acc".to_owned(),
                id.to_owned(),
                PathBuf::from(format!("/nowhere/{id}")),
            ),
            tool: tool.to_owned(),
        }
    }

    use SubagentStatus::{Done, Running, Unknown};

    #[test]
    fn any_tool_with_a_reader_is_counted_running_only_and_null_until_caught_up() {
        let script: Script = Arc::new(Mutex::new(vec![
            Ok((false, vec![Running])),
            Ok((true, vec![Running, Running, Done, Unknown])),
            Ok((true, vec![Running, Done, Done, Unknown])),
        ]));
        let made = Cell::new(0);
        let make = |tool: &str, _: SubagentScope<'_>| -> Option<Box<dyn SubagentReader>> {
            assert_eq!(tool, "any-tool");
            made.set(made.get() + 1);
            Some(Box::new(Fake {
                script: Arc::clone(&script),
                agents: Vec::new(),
            }))
        };
        let readers = Readers::default();
        assert!(
            !pass(&readers, vec![followed("s1")], &make).any(),
            "nothing read yet"
        );
        assert_eq!(
            count(&readers, "s1"),
            None,
            "still catching up: never a guess"
        );
        assert!(pass(&readers, vec![followed("s1")], &make).any());
        assert_eq!(
            count(&readers, "s1"),
            Some(2),
            "done and unknown aren't running"
        );
        assert!(pass(&readers, vec![followed("s1")], &make).any());
        assert_eq!(count(&readers, "s1"), Some(1));
        assert!(
            !pass(&readers, vec![followed("s1")], &make).any(),
            "no change, no emit"
        );
        assert_eq!(made.get(), 1, "one reader, refreshed each pass");
    }

    #[test]
    fn a_session_that_ends_leaves_the_count_and_its_reader_goes() {
        let script: Script = Arc::new(Mutex::new(vec![Ok((true, vec![Running]))]));
        let make = |_: &str, _: SubagentScope<'_>| -> Option<Box<dyn SubagentReader>> {
            Some(Box::new(Fake {
                script: Arc::clone(&script),
                agents: Vec::new(),
            }))
        };
        let readers = Readers::default();
        assert!(pass(&readers, vec![followed("s1"), followed("s2")], &make).any());
        assert_eq!(count(&readers, "s1"), Some(1));
        assert!(
            pass(&readers, vec![followed("s2")], &make).any(),
            "s1 left the count"
        );
        assert_eq!(count(&readers, "s1"), None, "ended: unknown, not 0");
        assert_eq!(count(&readers, "s2"), Some(1));
        let kept = readers.kept.lock().unwrap();
        assert_eq!(kept.live.len(), 1);
        assert!(kept.live.contains_key(&Row::Session("s2".to_owned())));
    }

    #[test]
    fn the_tabs_reader_is_the_counts_and_outlives_the_session() {
        let script: Script = Arc::new(Mutex::new(vec![Ok((true, vec![Running, Running]))]));
        let readers = Readers::default();
        let one = followed("s1");
        let tab = Entry::new(
            one.key.clone(),
            Box::new(Fake {
                script: Arc::clone(&script),
                agents: Vec::new(),
            }),
        );
        readers.used(&tab);
        let never = |_: &str, _: SubagentScope<'_>| -> Option<Box<dyn SubagentReader>> {
            panic!("the tab's reader is reused, not a second one made")
        };
        assert!(pass(&readers, vec![one], &never).any());
        assert_eq!(count(&readers, "s1"), Some(2));
        assert!(Arc::ptr_eq(
            &readers.kept.lock().unwrap().live[&Row::Session("s1".to_owned())],
            &tab
        ));
        assert!(pass(&readers, Vec::new(), &never).any());
        assert_eq!(
            readers.kept.lock().unwrap().tab.len(),
            1,
            "the tab keeps its own"
        );
    }

    #[test]
    fn no_record_or_no_reader_is_not_counted() {
        let gone: Script = Arc::new(Mutex::new(vec![Err(std::io::Error::other("no file"))]));
        let make = |tool: &str, _: SubagentScope<'_>| -> Option<Box<dyn SubagentReader>> {
            (tool == "any-tool").then(|| {
                Box::new(Fake {
                    script: Arc::clone(&gone),
                    agents: Vec::new(),
                }) as Box<dyn SubagentReader>
            })
        };
        let readers = Readers::default();
        let mut other = followed("s2");
        other.tool = "keeps-none".to_owned();
        assert!(!pass(&readers, vec![followed("s1"), other], &make).any());
        assert_eq!(count(&readers, "s1"), None, "unreadable: not a 0");
        assert_eq!(count(&readers, "s2"), None, "no reader: not a 0");
        let kept = readers.kept.lock().unwrap();
        assert!(
            kept.live.contains_key(&Row::Session("s1".to_owned())),
            "kept, to read again next pass"
        );
        assert!(!kept.live.contains_key(&Row::Session("s2".to_owned())));
    }

    #[test]
    fn a_session_another_app_runs_is_counted_on_its_conversations_row_for_any_tool() {
        let script: Script = Arc::new(Mutex::new(vec![Ok((true, vec![Running, Running, Done]))]));
        let made: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let make = |tool: &str, _: SubagentScope<'_>| -> Option<Box<dyn SubagentReader>> {
            made.lock().unwrap().push(tool.to_owned());
            Some(Box::new(Fake {
                script: Arc::clone(&script),
                agents: Vec::new(),
            }))
        };
        let readers = Readers::default();
        let changed = pass(
            &readers,
            vec![elsewhere("thread-1", "codex"), elsewhere("ses-2", "kimi")],
            &make,
        );
        assert_eq!(
            changed,
            Changed {
                sessions: false,
                conversations: true
            },
            "only the conversations' list is told"
        );
        let counts = readers.counts.lock().unwrap().clone();
        assert_eq!(
            counts.get(&Row::Conversation("acc".into(), "thread-1".into())),
            Some(&2)
        );
        assert_eq!(
            counts.get(&Row::Conversation("acc".into(), "ses-2".into())),
            Some(&2)
        );
        assert_eq!(
            *made.lock().unwrap(),
            ["codex", "kimi"],
            "each by its own tool"
        );

        // An own session changing tells only the sessions' list.
        let changed = pass(
            &readers,
            vec![
                elsewhere("thread-1", "codex"),
                elsewhere("ses-2", "kimi"),
                followed("s1"),
            ],
            &make,
        );
        assert_eq!(
            changed,
            Changed {
                sessions: true,
                conversations: false
            }
        );
    }
}
