//! AJ-E1520 (0.2): the rail's subagent count (I-08, a row's
//! `subtask_count`): how many subagents each live session (and its
//! subagents) has running now, read by the same readers as the session's
//! Subagents tab — one source, for every tool whose adapter lists
//! subagents (`HarnessAdapter::subagent_reader`), none named here.
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
//! rail's list never reads a tool's files.

use std::collections::HashMap;
use std::sync::Arc;

use juggler_adapter::subagents::{Subagent, SubagentReader, SubagentScope, SubagentStatus};
use juggler_core::model::{HarnessId, ProfileId};

use super::{Entry, Key, Readers};
use crate::features::sessions::dto::InstanceInfo;
use crate::state::AppState;

/// How often the count is read: the 2 s the process sampling uses.
pub const EVERY: std::time::Duration = std::time::Duration::from_secs(2);

/// A live session the count follows.
pub(crate) struct Followed {
    /// AI Juggler's id for the session (its row's).
    pub session: String,
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
        row.subtask_count = counts.get(&row.id).copied();
    }
    rows
}

/// Reads each live session's subagents and publishes the counts; true
/// when one changed (the caller tells the UI).
pub fn tick(state: &Arc<AppState>) -> bool {
    let followed = live_sessions(state);
    pass(&state.subagents, followed, &|tool, scope| {
        let adapter = state.registry.get(&HarnessId::new(tool))?;
        if !adapter.subagent_support().is_supported() {
            return None;
        }
        adapter.subagent_reader(scope)
    })
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
                session,
                tool: conversation.tool,
                key: (account, id, conversation.path),
            })
        })
        .collect()
}

/// One pass over `followed`: refreshes each one's reader (the tab's when
/// it keeps one, else its own, made by `make`), publishes the counts and
/// lets the readers of sessions no longer followed go. True when a
/// count changed.
pub(crate) fn pass(readers: &Readers, followed: Vec<Followed>, make: MakeReader<'_>) -> bool {
    let before: HashMap<String, u64> = readers
        .counts
        .lock()
        .map(|counts| counts.clone())
        .unwrap_or_default();
    let mut counts: HashMap<String, u64> = HashMap::with_capacity(followed.len());
    let mut live: HashMap<String, Arc<Entry>> = HashMap::with_capacity(followed.len());
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
                counts.insert(one.session.clone(), running(&agents));
            }
            // Still catching up: the last count stands until it has.
            Some((Ok(false), _)) => {
                if let Some(&last) = before.get(&one.session) {
                    counts.insert(one.session.clone(), last);
                }
            }
            // No record yet, or not readable now: not counted.
            Some((Err(_), _)) | None => {}
        }
        live.insert(one.session, entry);
    }
    if let Ok(mut kept) = readers.kept.lock() {
        kept.live = live;
    }
    let changed = counts != before;
    if let Ok(mut published) = readers.counts.lock() {
        *published = counts;
    }
    changed
}

/// How many are running: started, and nothing recorded says they ended
/// (the session is live, so running is what the tab says too).
fn running(agents: &[Subagent]) -> u64 {
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
            session: session.to_owned(),
            key: (
                "acc".to_owned(),
                format!("conv-{session}"),
                PathBuf::from(format!("/nowhere/{session}.jsonl")),
            ),
            tool: "any-tool".to_owned(),
        }
    }

    fn count(readers: &Readers, session: &str) -> Option<u64> {
        readers.counts.lock().unwrap().get(session).copied()
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
            !pass(&readers, vec![followed("s1")], &make),
            "nothing read yet"
        );
        assert_eq!(
            count(&readers, "s1"),
            None,
            "still catching up: never a guess"
        );
        assert!(pass(&readers, vec![followed("s1")], &make));
        assert_eq!(
            count(&readers, "s1"),
            Some(2),
            "done and unknown aren't running"
        );
        assert!(pass(&readers, vec![followed("s1")], &make));
        assert_eq!(count(&readers, "s1"), Some(1));
        assert!(
            !pass(&readers, vec![followed("s1")], &make),
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
        assert!(pass(&readers, vec![followed("s1"), followed("s2")], &make));
        assert_eq!(count(&readers, "s1"), Some(1));
        assert!(
            pass(&readers, vec![followed("s2")], &make),
            "s1 left the count"
        );
        assert_eq!(count(&readers, "s1"), None, "ended: unknown, not 0");
        assert_eq!(count(&readers, "s2"), Some(1));
        let kept = readers.kept.lock().unwrap();
        assert_eq!(kept.live.len(), 1);
        assert!(kept.live.contains_key("s2"));
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
        assert!(pass(&readers, vec![one], &never));
        assert_eq!(count(&readers, "s1"), Some(2));
        assert!(Arc::ptr_eq(&readers.kept.lock().unwrap().live["s1"], &tab));
        assert!(pass(&readers, Vec::new(), &never));
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
        assert!(!pass(&readers, vec![followed("s1"), other], &make));
        assert_eq!(count(&readers, "s1"), None, "unreadable: not a 0");
        assert_eq!(count(&readers, "s2"), None, "no reader: not a 0");
        let kept = readers.kept.lock().unwrap();
        assert!(
            kept.live.contains_key("s1"),
            "kept, to read again next pass"
        );
        assert!(!kept.live.contains_key("s2"));
    }
}
