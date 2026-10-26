//! Every session, AI Juggler's and the ones started outside it, a page at
//! a time (0.1.5 review: History and a project's Sessions tab read every
//! session and drew hundreds of rows at once).
//!
//! One list for History, a project's and an account's Sessions: open ones
//! first, then by last activity, then by key; narrowed to a project or an
//! account, to what's open, ended or archived, and to rows whose title,
//! project, account, tool, model, branch, date (YYYY-MM-DD, local) or
//! transcript words match a search. Only a page's sessions are built in
//! full (their git state, live stats and titles); the rest are matched
//! from what's already in memory or one storage pass.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::dto::InstanceInfo;
use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::conversations::dto::ConversationInfo;
use crate::paging::{Page, PageRequest, SortKey};
use crate::state::AppState;

/// What a list of sessions shows.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SessionsQuery {
    /// One project's sessions.
    #[serde(default)]
    pub project_id: Option<String>,
    /// One account's sessions.
    #[serde(default)]
    pub account_id: Option<String>,
    /// `all` (the default), `open`, `ended` (not archived) or `archived`.
    #[serde(default)]
    pub which: Option<String>,
    /// Words that must each match the row (its title, project, account,
    /// tool, model, branch, local dates) or its transcript.
    #[serde(default)]
    pub text: Option<String>,
    /// The UI's minutes east of UTC, for dates in words.
    #[serde(default)]
    pub utc_offset_min: Option<i32>,
}

/// One row: one of AI Juggler's sessions, or a conversation started
/// outside it.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SessionRowInfo {
    /// `s/<session>` or `c/<account>/<conversation>`: unique in the list.
    pub key: String,
    /// AI Juggler's session.
    pub session: Option<InstanceInfo>,
    /// A conversation started outside AI Juggler.
    pub conversation: Option<ConversationInfo>,
    /// The search's words in context, when its transcript matched.
    pub snippet: Option<String>,
}

/// How many sessions a query's list holds, by kind.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SessionsCountsInfo {
    /// Every row the list holds (open, ended and archived).
    #[ts(type = "number")]
    pub all: u64,
    /// Open now.
    #[ts(type = "number")]
    pub open: u64,
    /// Ended, not archived.
    #[ts(type = "number")]
    pub ended: u64,
    /// Archived (AI Juggler's sessions only).
    #[ts(type = "number")]
    pub archived: u64,
}

/// What a row is, before its page is built.
enum Source {
    /// An index into the session records.
    Session(usize),
    Conversation(Box<ConversationInfo>),
}

struct Candidate {
    key: String,
    live: bool,
    archived: bool,
    last_ms: i64,
    source: Source,
    snippet: Option<String>,
}

impl Candidate {
    /// Open first, newest first, then by key.
    fn sort_key(&self) -> SortKey {
        SortKey::new([i64::from(!self.live), -self.last_ms], self.key.clone())
    }

    fn shown_by(&self, which: Which) -> bool {
        match which {
            Which::All => true,
            Which::Open => self.live,
            Which::Ended => !self.live && !self.archived,
            Which::Archived => !self.live && self.archived,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Which {
    All,
    Open,
    Ended,
    Archived,
}

impl Which {
    fn of(query: &SessionsQuery) -> Self {
        match query.which.as_deref() {
            Some("open") => Self::Open,
            Some("ended") => Self::Ended,
            Some("archived") => Self::Archived,
            _ => Self::All,
        }
    }
}

/// What the rows are matched and built from: one storage pass.
struct Read {
    records: Vec<juggler_core::model::SessionRecord>,
    candidates: Vec<Candidate>,
}

/// A day in words, local to the UI (`YYYY-MM-DD`).
fn local_day(ms: i64, offset_min: i32) -> String {
    juggler_core::cost_ledger::day_of(ms + i64::from(offset_min) * 60_000)
}

/// Does every word of `words` (lower case) appear in `fields`?
fn matches(fields: &[&str], words: &[String]) -> bool {
    let text = fields.join(" ").to_lowercase();
    words.iter().all(|word| text.contains(word.as_str()))
}

/// What a search matches rows against: read once per query.
struct Lookup {
    words: Vec<String>,
    offset: i32,
    /// Transcript hits' words in context, by row key.
    snippets: HashMap<String, String>,
    /// Conversation titles, by (account, conversation).
    titles: HashMap<(String, String), String>,
    projects: HashMap<String, String>,
    tools: BTreeMap<String, String>,
    marks: juggler_core::storage::SessionMarks,
}

impl Lookup {
    fn new(state: &Arc<AppState>, query: &SessionsQuery) -> AppResult<Self> {
        let text = query.text.as_deref().unwrap_or_default().trim();
        // Transcript words: the local indexes, for a search of two letters
        // or more (as History always did).
        let mut snippets = HashMap::new();
        if text.chars().count() >= 2 {
            for hit in crate::history::transcript_search_impl(state, text)? {
                let key = match (&hit.session_id, &hit.account_id, &hit.conversation_id) {
                    (Some(session), _, _) => format!("s/{session}"),
                    (None, Some(account), Some(conversation)) => {
                        format!("c/{account}/{conversation}")
                    }
                    _ => continue,
                };
                snippets.entry(key).or_insert(hit.snippet);
            }
        }
        let titles = state
            .conversations
            .all()?
            .into_iter()
            .filter_map(|c| Some(((c.account_id, c.summary.id), c.summary.title?)))
            .collect();
        let tools = state
            .registry
            .ids()
            .into_iter()
            .map(|id| {
                let name = crate::features::tools::display_name(&state.registry, id.as_str());
                (id.to_string(), name)
            })
            .collect();
        Ok(Self {
            words: text.split_whitespace().map(str::to_lowercase).collect(),
            offset: query.utc_offset_min.unwrap_or_default(),
            snippets,
            titles,
            projects: HashMap::new(),
            tools,
            marks: BTreeMap::new(),
        })
    }

    fn project(&self, id: Option<&str>) -> &str {
        id.and_then(|id| self.projects.get(id))
            .map_or("", String::as_str)
    }

    fn tool(&self, id: &str) -> &str {
        self.tools.get(id).map_or("", String::as_str)
    }

    fn day(&self, ms: Option<i64>) -> String {
        ms.map(|ms| local_day(ms, self.offset)).unwrap_or_default()
    }

    /// Does session `record` match the search (no search: yes)?
    fn session_matches(
        &self,
        record: &juggler_core::model::SessionRecord,
        join: &(Option<String>, String, String, Option<String>),
    ) -> bool {
        if self.words.is_empty() {
            return true;
        }
        let (account_id, account_label, provider, launch_model) = join;
        let label = self
            .marks
            .get(record.id.as_str())
            .and_then(|(label, _)| label.as_deref())
            .unwrap_or_default();
        let title = account_id
            .as_ref()
            .zip(record.conversation.as_ref())
            .and_then(|(account, conversation)| {
                self.titles
                    .get(&(account.clone(), conversation.as_str().to_owned()))
            })
            .map_or("", String::as_str);
        let (model, _) =
            crate::commands::display_model(record.running.as_ref(), launch_model.as_ref());
        let started = self.day(Some(record.started_at.millis()));
        let last = self.day(record.last_activity.as_ref().map(|at| at.millis()));
        let project = record.project.as_ref().map(ToString::to_string);
        let mut fields = vec![
            label,
            title,
            self.project(project.as_deref()),
            account_label.as_str(),
            provider.as_str(),
            self.tool(provider),
            model.as_deref().unwrap_or_default(),
            started.as_str(),
            last.as_str(),
        ];
        if matches(&fields, &self.words) {
            return true;
        }
        // The branch is read from the folder: only when nothing else matched.
        let branch = crate::worktree::branch_of(&record.cwd).unwrap_or_default();
        fields.push(branch.as_str());
        matches(&fields, &self.words)
    }

    /// Does an outside conversation match the search (no search: yes)?
    fn conversation_matches(&self, conversation: &ConversationInfo) -> bool {
        if self.words.is_empty() {
            return true;
        }
        let folder = conversation
            .cwd
            .as_deref()
            .and_then(|cwd| std::path::Path::new(cwd).file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let started = self.day(conversation.started_ms);
        let last = self.day(conversation.last_activity_ms);
        let fields = [
            conversation.title.as_deref().unwrap_or_default(),
            conversation.git_branch.as_deref().unwrap_or_default(),
            folder.as_str(),
            self.project(conversation.project_id.as_deref()),
            conversation.account_label.as_str(),
            conversation.tool.as_str(),
            self.tool(&conversation.tool),
            started.as_str(),
            last.as_str(),
        ];
        matches(&fields, &self.words)
    }
}

/// Every row in `query`'s scope that matches its search (not yet narrowed
/// to open, ended or archived).
fn read(state: &Arc<AppState>, query: &SessionsQuery) -> AppResult<Read> {
    let mut lookup = Lookup::new(state, query)?;
    // Outside conversations from the index (in memory), before storage
    // is held.
    let outside: Vec<ConversationInfo> =
        crate::features::conversations::commands::list_impl(state)?
            .into_iter()
            .filter(|conversation| conversation.session_id.is_none())
            .collect();
    let (records, accounts, recipes, lifecycle) = {
        let storage = lock_storage_arc(state)?;
        let (accounts, recipes) = crate::commands::session_join_indexes(&storage)?;
        lookup.projects = storage
            .projects()
            .list()?
            .into_iter()
            .map(|project| (project.id.to_string(), project.name))
            .collect();
        lookup.marks = storage.sessions().marks().unwrap_or_default();
        (
            storage.sessions().list(None)?,
            accounts,
            recipes,
            storage.sessions().lifecycle_latest().unwrap_or_default(),
        )
    };
    let mut candidates = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let join = crate::commands::resolved_join(&recipes, &accounts, &record.profile.to_string());
        let project = record.project.as_ref().map(ToString::to_string);
        if (query.project_id.is_some() && project != query.project_id)
            || (query.account_id.is_some() && join.0 != query.account_id)
        {
            continue;
        }
        let key = format!("s/{}", record.id);
        let snippet = lookup.snippets.get(&key).cloned();
        if snippet.is_none() && !lookup.session_matches(record, &join) {
            continue;
        }
        candidates.push(Candidate {
            key,
            live: crate::commands::is_active(record.status),
            archived: lifecycle
                .get(record.id.as_str())
                .is_some_and(|(event, _)| event == "archived"),
            last_ms: record
                .last_activity
                .as_ref()
                .map_or(record.started_at.millis(), |at| at.millis()),
            source: Source::Session(index),
            snippet,
        });
    }
    for conversation in outside {
        if (query.project_id.is_some() && conversation.project_id != query.project_id)
            || query
                .account_id
                .as_ref()
                .is_some_and(|account| *account != conversation.account_id)
        {
            continue;
        }
        let key = format!("c/{}/{}", conversation.account_id, conversation.id);
        let snippet = lookup.snippets.get(&key).cloned();
        if snippet.is_none() && !lookup.conversation_matches(&conversation) {
            continue;
        }
        candidates.push(Candidate {
            key,
            live: conversation.live,
            archived: false,
            last_ms: conversation
                .last_activity_ms
                .or(conversation.started_ms)
                .unwrap_or_default(),
            source: Source::Conversation(Box::new(conversation)),
            snippet,
        });
    }
    Ok(Read {
        records,
        candidates,
    })
}

/// One page of `query`'s sessions.
///
/// # Errors
///
/// A cursor this app didn't make; storage errors.
pub fn sessions_page_impl(
    state: &Arc<AppState>,
    query: &SessionsQuery,
    request: &PageRequest,
) -> AppResult<Page<SessionRowInfo>> {
    let which = Which::of(query);
    let Read {
        records,
        candidates,
    } = read(state, query)?;
    let shown: Vec<Candidate> = candidates
        .into_iter()
        .filter(|candidate| candidate.shown_by(which))
        .collect();
    let page = crate::paging::page(shown, Candidate::sort_key, request)?;
    // Only this page's sessions are built in full.
    let wanted: Vec<juggler_core::model::SessionRecord> = page
        .items
        .iter()
        .filter_map(|candidate| match candidate.source {
            Source::Session(index) => records.get(index).cloned(),
            Source::Conversation(_) => None,
        })
        .collect();
    let mut built: HashMap<String, InstanceInfo> = crate::commands::instances_of(state, wanted)?
        .into_iter()
        .map(|info| (info.id.clone(), info))
        .collect();
    Ok(page.map(|candidate| {
        let (session, conversation) = match candidate.source {
            Source::Session(_) => (
                candidate
                    .key
                    .strip_prefix("s/")
                    .and_then(|id| built.remove(id)),
                None,
            ),
            Source::Conversation(conversation) => (None, Some(*conversation)),
        };
        SessionRowInfo {
            key: candidate.key,
            session,
            conversation,
            snippet: candidate.snippet,
        }
    }))
}

/// How many rows `query`'s list holds, open, ended and archived.
///
/// # Errors
///
/// Storage errors.
pub fn sessions_counts_impl(
    state: &Arc<AppState>,
    query: &SessionsQuery,
) -> AppResult<SessionsCountsInfo> {
    let read = read(state, query)?;
    let mut counts = SessionsCountsInfo::default();
    for candidate in &read.candidates {
        counts.all += 1;
        if candidate.live {
            counts.open += 1;
        } else if candidate.archived {
            counts.archived += 1;
        } else {
            counts.ended += 1;
        }
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_word_must_match_some_field() {
        let words = vec!["fix".to_owned(), "2026-10-21".to_owned()];
        assert!(matches(&["Fix the login", "2026-10-21"], &words));
        assert!(!matches(&["Fix the login", "2026-10-20"], &words));
        assert!(matches(&["anything"], &[]));
    }

    #[test]
    fn a_day_in_words_is_the_uis_local_day() {
        // 2025-10-21 23:30 UTC is 2025-10-22 in Sarajevo (UTC+2).
        let at = 1_761_089_400_000;
        assert_eq!(local_day(at, 0), "2025-10-21");
        assert_eq!(local_day(at, 120), "2025-10-22");
        assert_eq!(local_day(at, -60), "2025-10-21");
    }
}
