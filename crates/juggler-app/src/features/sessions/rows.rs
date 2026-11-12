//! Every session, AI Juggler's and the ones started outside it, a page at
//! a time (0.1.5 review: History and a project's Sessions tab read every
//! session and drew hundreds of rows at once).
//!
//! One list for History and every page's Sessions (E31, AJ-E3103): what
//! needs you first, then what's working, what's open, then by last
//! activity (D-62 decision 21), then by key; narrowed to a project (or to
//! none), a folder, an App, an Account or a Mac, to what's open, ended or
//! archived, to what was active since a time, and to rows whose title,
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
    /// One project's sessions; `""` the ones in no project (E31: a
    /// list's "No project" filter).
    #[serde(default)]
    pub project_id: Option<String>,
    /// One account's sessions.
    #[serde(default)]
    pub account_id: Option<String>,
    /// D-62 (AJ-E3204): one App's sessions (`cli:<folder>`,
    /// `app:<host>:<instance folder>`), as the rail groups them: where a
    /// desktop app was last seen showing it, else its tool and folder.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub app_id: Option<String>,
    /// D-62 (AJ-E3205): one Account's sessions: those on any folder whose
    /// sign-in was last read as it (`acct:<vendor>:<hash>`).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub subscription_id: Option<String>,
    /// E31 (AJ-E3103): one Mac's sessions (the Mac AI Juggler's ran on;
    /// the tools' conversations are this Mac's files).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub machine_id: Option<String>,
    /// E31 (AJ-E3104): one tool's sessions, by its adapter id (a Tool's
    /// page).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub tool_id: Option<String>,
    /// E31: only rows active at or after this time, epoch ms (a list's
    /// "Recent": the last 7 days).
    #[serde(default)]
    #[ts(optional = nullable, type = "number | null")]
    pub since_ms: Option<i64>,
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
    /// Archived (AI Juggler's sessions, and other apps' ones archived here).
    #[ts(type = "number")]
    pub archived: u64,
}

/// What a row is, before its page is built.
enum Source {
    /// An index into the session records.
    Session(usize),
    Conversation(Box<ConversationInfo>),
}

pub(super) struct Candidate {
    pub(super) key: String,
    pub(super) live: bool,
    pub(super) archived: bool,
    /// Where it stands in the order ([`Rank`]).
    rank: Rank,
    last_ms: i64,
    source: Source,
    snippet: Option<String>,
    /// What it belongs to, for a list's filters (AJ-E3103).
    pub(super) of: Belongs,
}

/// What a row belongs to: the values a list's filters offer.
#[derive(Clone, Debug, Default)]
pub(super) struct Belongs {
    pub(super) project: Option<String>,
    pub(super) folder: Option<String>,
    pub(super) app: String,
    pub(super) machine: String,
    pub(super) tool: String,
}

/// E31 (D-62 decision 21): the order every list of sessions opens on.
/// Ended ones follow by last activity, so the last 7 days come before the
/// rest by themselves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    /// It asks you something, is stuck or hit a limit.
    NeedsYou = 0,
    /// It's working.
    Working = 1,
    /// Open, and waiting for you to say something (or done, to review).
    Open = 2,
    /// It has ended.
    Ended = 3,
}

impl Rank {
    /// One of AI Juggler's sessions, by its status, or the exact state its
    /// tool reported (`runtime`: `asks_you`, `working`, `idle`), as its
    /// rail row reads it (`ui/src/lib/rail-items.ts`, `sessionItem`).
    #[must_use]
    pub fn of_session(status: juggler_core::model::SessionStatus, runtime: Option<&str>) -> Self {
        use juggler_core::model::SessionStatus as S;
        if !crate::features::sessions::read_model::is_active(status) {
            return Self::Ended;
        }
        // The tool's own word wins over the guess from the status.
        match runtime {
            Some("asks_you") => Self::NeedsYou,
            Some("working") => Self::Working,
            Some("idle" | "done_review") => Self::Open,
            Some("ended") => Self::Ended,
            _ => match status {
                S::WaitingUser | S::Stuck | S::QuotaBlocked => Self::NeedsYou,
                S::Running | S::Starting => Self::Working,
                _ => Self::Open,
            },
        }
    }

    /// A conversation another app runs, by the state its tool's files say
    /// (`conversationItem`).
    #[must_use]
    pub fn of_conversation(live: bool, state: Option<&str>) -> Self {
        match (live, state) {
            (false, _) => Self::Ended,
            (true, Some("asks_you")) => Self::NeedsYou,
            (true, Some("working")) => Self::Working,
            (true, _) => Self::Open,
        }
    }
}

impl Candidate {
    /// What needs you, what's working, what's open, then newest first,
    /// then by key.
    fn sort_key(&self) -> SortKey {
        SortKey::new([self.rank as i64, -self.last_ms], self.key.clone())
    }

    /// Does it pass the filters a row is built without (its Mac, since)?
    fn passes(&self, query: &SessionsQuery) -> bool {
        query
            .machine_id
            .as_ref()
            .is_none_or(|machine| *machine == self.of.machine)
            && query
                .tool_id
                .as_ref()
                .is_none_or(|tool| *tool == self.of.tool)
            && query.since_ms.is_none_or(|since| self.last_ms >= since)
    }

    pub(super) fn shown_by(&self, which: Which) -> bool {
        match which {
            Which::All => true,
            Which::Open => self.live,
            Which::Ended => !self.live && !self.archived,
            Which::Archived => !self.live && self.archived,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Which {
    All,
    Open,
    Ended,
    Archived,
}

impl Which {
    pub(super) fn of(query: &SessionsQuery) -> Self {
        match query.which.as_deref() {
            Some("open") => Self::Open,
            Some("ended") => Self::Ended,
            Some("archived") => Self::Archived,
            _ => Self::All,
        }
    }
}

/// What the rows are matched and built from: one storage pass.
pub(super) struct Read {
    records: Vec<juggler_core::model::SessionRecord>,
    pub(super) candidates: Vec<Candidate>,
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
        let (model, _) = crate::features::sessions::read_model::display_model(
            record.running.as_ref(),
            launch_model.as_ref(),
        );
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

/// D-62: a query's App and Account (AJ-E3204, AJ-E3205), as the rail
/// groups sessions: a session is in the desktop App it was last seen open
/// in, else in its folder's CLI App; on the Account its folder signed in
/// to.
struct Narrow {
    app: Option<String>,
    desktop: crate::features::places::DesktopApps,
    /// The Account filter: none, one Account (`folders`: the folders
    /// signed in as it), or "No account" (`folders`: every signed-in one).
    account: AccountNarrow,
    folders: std::collections::BTreeSet<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AccountNarrow {
    Any,
    One,
    None,
}

impl Narrow {
    fn new(state: &Arc<AppState>, query: &SessionsQuery, facets: bool) -> AppResult<Self> {
        let desktop = if query.app_id.is_some() || facets {
            crate::features::places::desktop_apps(state)
        } else {
            crate::features::places::DesktopApps::default()
        };
        // "" is "No account": the sessions on a folder no sign-in was read
        // for (as "" is "No project"), so the filter's values cover the list.
        let account = match query.subscription_id.as_deref() {
            None => AccountNarrow::Any,
            Some("") => AccountNarrow::None,
            Some(_) => AccountNarrow::One,
        };
        let folders = match query.subscription_id.as_deref() {
            Some(want) => lock_storage_arc(state)?
                .subscriptions()
                .folders()?
                .into_iter()
                .filter(|(_, of)| match of.as_deref() {
                    Some(account) => want.is_empty() || account == want,
                    None => false,
                })
                .map(|(folder, _)| folder)
                .collect(),
            None => std::collections::BTreeSet::new(),
        };
        Ok(Self {
            app: query.app_id.clone(),
            desktop,
            account,
            folders,
        })
    }

    /// The App a session on folder `folder` is in (`launched_in`: the
    /// desktop App AI Juggler launched it in; `key`: `<tool>/<id>` for
    /// one another app runs, which may be a desktop App's; `in_terminal`:
    /// it runs in a terminal), by `places::app_of`.
    fn app(
        &self,
        launched_in: Option<&str>,
        folder: Option<&str>,
        key: Option<&str>,
        in_terminal: bool,
    ) -> String {
        crate::features::places::app_of(&self.desktop, launched_in, folder, key, in_terminal)
    }

    /// A session on `folder` in App `app` passes.
    fn passes(&self, folder: Option<&str>, app: &str) -> bool {
        let on_account = match self.account {
            AccountNarrow::Any => true,
            AccountNarrow::One => folder.is_some_and(|id| self.folders.contains(id)),
            AccountNarrow::None => folder.is_none_or(|id| !self.folders.contains(id)),
        };
        let in_app = self.app.as_ref().is_none_or(|want| want == app);
        on_account && in_app
    }
}

/// Is a row in project `project` in `query`'s project (`""`: none)?
fn in_project(query: &SessionsQuery, project: Option<&str>) -> bool {
    match query.project_id.as_deref() {
        None => true,
        Some("") => project.is_none(),
        Some(want) => project == Some(want),
    }
}

/// AJ-E3102 R1: a list scoped to a project, an App or a folder keeps only
/// the outside conversations the conversation index puts there (the same
/// index the relation counts are made from, so a count and its list agree),
/// before any row is built. An unscoped list (or "no project") keeps all.
fn narrowed_outside(
    state: &Arc<AppState>,
    query: &SessionsQuery,
    outside: Vec<ConversationInfo>,
) -> AppResult<Vec<ConversationInfo>> {
    let project = query.project_id.as_deref().filter(|id| !id.is_empty());
    if project.is_none() && query.app_id.is_none() && query.account_id.is_none() {
        return Ok(outside);
    }
    let index = crate::features::relations::conversation_index(state)?;
    let mut keep: Option<std::collections::HashSet<(String, String)>> = None;
    let mut narrow = |found: Vec<(String, String)>| {
        let found: std::collections::HashSet<_> = found.into_iter().collect();
        keep = Some(match keep.take() {
            Some(before) => before.intersection(&found).cloned().collect(),
            None => found,
        });
    };
    let pair = |facet: &crate::features::conversations::by_relation::Facet| {
        (facet.folder.clone(), facet.id.clone())
    };
    if let Some(project) = project {
        narrow(index.of_project(project).map(pair).collect());
    }
    if let Some(app) = &query.app_id {
        narrow(index.of_app(app).map(pair).collect());
    }
    if let Some(folder) = &query.account_id {
        narrow(index.of_folder(folder).map(pair).collect());
    }
    let keep = keep.unwrap_or_default();
    Ok(outside
        .into_iter()
        .filter(|conversation| {
            keep.contains(&(conversation.account_id.clone(), conversation.id.clone()))
        })
        .collect())
}

/// Every row in `query`'s scope that matches its search and its filters
/// (not yet narrowed to open, ended or archived); `facets`: each row's
/// App worked out whether or not the query names one (a list's filters,
/// [`super::facets`]).
pub(super) fn read(state: &Arc<AppState>, query: &SessionsQuery, facets: bool) -> AppResult<Read> {
    let mut lookup = Lookup::new(state, query)?;
    // Outside conversations from the index (in memory), before storage
    // is held.
    let outside: Vec<ConversationInfo> = narrowed_outside(
        state,
        query,
        crate::features::conversations::commands::list_impl(state)?
            .into_iter()
            .filter(|conversation| conversation.session_id.is_none())
            .collect(),
    )?;
    let (records, accounts, recipes, lifecycle, launched) = {
        let storage = lock_storage_arc(state)?;
        let (accounts, recipes) =
            crate::features::sessions::read_model::session_join_indexes(&storage)?;
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
            storage.apps().session_apps()?,
        )
    };
    let narrow = Narrow::new(state, query, facets)?;
    // The tools' files are on this Mac.
    let here = crate::commands::local_machine_id(state).to_string();
    let mut candidates = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let join = crate::features::sessions::read_model::resolved_join(
            &recipes,
            &accounts,
            &record.profile.to_string(),
        );
        let project = record.project.as_ref().map(ToString::to_string);
        let in_terminal = record.kind != juggler_core::model::InstanceKind::Desktop;
        let launched_in = launched.get(record.id.as_str()).map(String::as_str);
        let app = narrow.app(launched_in, join.0.as_deref(), None, in_terminal);
        if !in_project(query, project.as_deref())
            || (query.account_id.is_some() && join.0 != query.account_id)
            || !narrow.passes(join.0.as_deref(), &app)
        {
            continue;
        }
        let key = format!("s/{}", record.id);
        let snippet = lookup.snippets.get(&key).cloned();
        if snippet.is_none() && !lookup.session_matches(record, &join) {
            continue;
        }
        let live = crate::features::sessions::read_model::is_active(record.status);
        // The tool's own word only for an open one (the runtime knows no
        // other).
        let runtime = live
            .then(|| state.runtime.info(record.id.as_str()))
            .flatten()
            .and_then(|info| info.state);
        let candidate = Candidate {
            key,
            live,
            archived: lifecycle
                .get(record.id.as_str())
                .is_some_and(|(event, _)| event == "archived"),
            rank: Rank::of_session(record.status, runtime.as_deref()),
            last_ms: record
                .last_activity
                .as_ref()
                .map_or(record.started_at.millis(), |at| at.millis()),
            source: Source::Session(index),
            snippet,
            of: Belongs {
                app,
                project,
                tool: join.2.clone(),
                folder: join.0,
                machine: record.machine.to_string(),
            },
        };
        if candidate.passes(query) {
            candidates.push(candidate);
        }
    }
    candidates.extend(outside.into_iter().filter_map(|conversation| {
        conversation_candidate(query, &narrow, &lookup, conversation, &here)
    }));
    Ok(Read {
        records,
        candidates,
    })
}

/// A conversation another app started, as a row of `query`'s list, when
/// it's in its scope and matches its search and filters (`here`: this
/// Mac, whose files hold it).
fn conversation_candidate(
    query: &SessionsQuery,
    narrow: &Narrow,
    lookup: &Lookup,
    conversation: ConversationInfo,
    here: &str,
) -> Option<Candidate> {
    // ADR-0032: its App as the list gives it, never worked out again.
    let app = conversation.owner(&narrow.desktop);
    if !in_project(query, conversation.project_id.as_deref())
        || query
            .account_id
            .as_ref()
            .is_some_and(|account| *account != conversation.account_id)
        || !narrow.passes(Some(&conversation.account_id), &app)
    {
        return None;
    }
    let key = format!("c/{}/{}", conversation.account_id, conversation.id);
    let snippet = lookup.snippets.get(&key).cloned();
    if snippet.is_none() && !lookup.conversation_matches(&conversation) {
        return None;
    }
    let candidate = Candidate {
        key,
        live: conversation.live,
        // 0.4.1: one archived in AI Juggler is under Archived.
        archived: !conversation.live && conversation.archived_here,
        rank: Rank::of_conversation(conversation.live, conversation.state.as_deref()),
        last_ms: conversation
            .last_activity_ms
            .or(conversation.started_ms)
            .unwrap_or_default(),
        of: Belongs {
            app,
            project: conversation.project_id.clone(),
            folder: Some(conversation.account_id.clone()),
            machine: here.to_owned(),
            tool: conversation.tool.clone(),
        },
        source: Source::Conversation(Box::new(conversation)),
        snippet,
    };
    candidate.passes(query).then_some(candidate)
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
    } = read(state, query, false)?;
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
    let mut built: HashMap<String, InstanceInfo> =
        crate::features::sessions::list::instances_of(state, wanted)?
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

/// How many rows `query`'s list holds, open, ended and archived: from
/// the relation counts (AJ-E3102) when it's a whole list (no search, at
/// most one of project, folder, App or Account), else counted from the
/// rows.
///
/// # Errors
///
/// Storage errors.
pub fn sessions_counts_impl(
    state: &Arc<AppState>,
    query: &SessionsQuery,
) -> AppResult<SessionsCountsInfo> {
    match whole_list(query) {
        Some(entity) => Ok(crate::features::relations::counts(state, &entity)?.sessions),
        None => sessions_counts_scanned(state, query),
    }
}

/// The entity whose sessions `query`'s list is, when it is a whole one.
fn whole_list(query: &SessionsQuery) -> Option<crate::features::relations::EntityRef> {
    use crate::features::relations::{EntityKind, EntityRef};
    // A search, a Mac, a time, "No project" or "No account" narrow what no entity's
    // counts hold.
    if query
        .text
        .as_deref()
        .is_some_and(|text| !text.trim().is_empty())
        || query.machine_id.is_some()
        || query.since_ms.is_some()
        || query.project_id.as_deref() == Some("")
        || query.subscription_id.as_deref() == Some("")
    {
        return None;
    }
    let mut scopes = [
        (EntityKind::Project, &query.project_id),
        (EntityKind::Folder, &query.account_id),
        (EntityKind::App, &query.app_id),
        (EntityKind::Subscription, &query.subscription_id),
        (EntityKind::Tool, &query.tool_id),
    ]
    .into_iter()
    .filter_map(|(kind, id)| Some((kind, id.clone()?)));
    match (scopes.next(), scopes.next()) {
        (None, _) => Some(EntityRef {
            kind: EntityKind::All,
            id: String::new(),
        }),
        (Some((kind, id)), None) => Some(EntityRef { kind, id }),
        _ => None,
    }
}

/// [`sessions_counts_impl`] counted from the rows themselves, whatever the
/// query (the relation counts' reference).
///
/// # Errors
///
/// Storage errors.
pub fn sessions_counts_scanned(
    state: &Arc<AppState>,
    query: &SessionsQuery,
) -> AppResult<SessionsCountsInfo> {
    let read = read(state, query, false)?;
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

    /// D-62: an App's list holds what the rail puts under it (a session
    /// last seen in a desktop App is that App's, else its folder's CLI
    /// App's); an Account's, what's on any folder signed in to it.
    #[test]
    fn an_app_and_an_account_narrow_like_the_rail() {
        let desktop = crate::features::places::DesktopApps {
            by_session: [(
                "claude-code/c-1".to_owned(),
                "app:claude-app:/p/Work".to_owned(),
            )]
            .into(),
            ..Default::default()
        };
        let app = |id: &str| Narrow {
            app: Some(id.to_owned()),
            desktop: desktop.clone(),
            account: AccountNarrow::Any,
            folders: std::collections::BTreeSet::new(),
        };
        let work = app("app:claude-app:/p/Work");
        assert!(work.passes(
            Some("work"),
            &work.app(None, Some("work"), Some("claude-code/c-1"), true)
        ));
        assert!(!work.passes(
            Some("work"),
            &work.app(None, Some("work"), Some("claude-code/c-2"), true)
        ));
        assert!(
            !work.passes(Some("work"), &work.app(None, Some("work"), None, true)),
            "AI Juggler's own: its CLI App"
        );
        let cli = app("cli:work");
        assert!(cli.passes(
            Some("work"),
            &cli.app(None, Some("work"), Some("claude-code/c-2"), true)
        ));
        assert!(cli.passes(Some("work"), &cli.app(None, Some("work"), None, true)));
        assert!(!cli.passes(
            Some("work"),
            &cli.app(None, Some("work"), Some("claude-code/c-1"), true)
        ));
        let account = Narrow {
            app: None,
            desktop: crate::features::places::DesktopApps::default(),
            account: AccountNarrow::One,
            folders: ["work".to_owned(), "side".to_owned()].into(),
        };
        assert!(account.passes(Some("side"), &account.app(None, Some("side"), None, true)));
        assert!(!account.passes(Some("home"), &account.app(None, Some("home"), None, true)));
        assert!(!account.passes(None, &account.app(None, None, None, true)));
        // "No account": a folder no sign-in was read for (`folders`: every
        // signed-in one), and AI Juggler's own rows without a folder.
        let none = Narrow {
            account: AccountNarrow::None,
            ..account
        };
        assert!(none.passes(Some("home"), &none.app(None, Some("home"), None, true)));
        assert!(none.passes(None, &none.app(None, None, None, true)));
        assert!(!none.passes(Some("work"), &none.app(None, Some("work"), None, true)));
    }

    /// AJ-E3102: a whole list's heading comes from the relation counts; a
    /// search or two scopes at once are counted from the rows.
    #[test]
    fn only_a_whole_list_is_counted_from_the_cache() {
        use crate::features::relations::EntityKind;
        let query = |over: fn(&mut SessionsQuery)| {
            let mut query = SessionsQuery::default();
            over(&mut query);
            whole_list(&query).map(|entity| (entity.kind, entity.id))
        };
        assert_eq!(query(|_| {}), Some((EntityKind::All, String::new())));
        assert_eq!(
            query(|q| q.project_id = Some("p".into())),
            Some((EntityKind::Project, "p".into()))
        );
        assert_eq!(
            query(|q| q.account_id = Some("work".into())),
            Some((EntityKind::Folder, "work".into()))
        );
        assert_eq!(
            query(|q| q.app_id = Some("cli:work".into())),
            Some((EntityKind::App, "cli:work".into()))
        );
        assert_eq!(
            query(|q| q.subscription_id = Some("acct:a:1".into())),
            Some((EntityKind::Subscription, "acct:a:1".into()))
        );
        assert_eq!(
            query(|q| q.text = Some("  ".into())),
            Some((EntityKind::All, String::new()))
        );
        assert_eq!(query(|q| q.text = Some("fix".into())), None);
        assert_eq!(
            query(|q| {
                q.project_id = Some("p".into());
                q.account_id = Some("work".into());
            }),
            None
        );
    }

    /// E31: the order's ranks, as the rail reads a session's state (the
    /// tool's own word first).
    #[test]
    fn a_rows_rank_reads_its_state_as_the_rail_does() {
        use juggler_core::model::SessionStatus as S;
        assert_eq!(Rank::of_session(S::WaitingUser, None), Rank::NeedsYou);
        assert_eq!(Rank::of_session(S::QuotaBlocked, None), Rank::NeedsYou);
        assert_eq!(Rank::of_session(S::Running, None), Rank::Working);
        assert_eq!(Rank::of_session(S::Running, Some("idle")), Rank::Open);
        assert_eq!(
            Rank::of_session(S::WaitingUser, Some("working")),
            Rank::Working
        );
        assert_eq!(
            Rank::of_session(S::Running, Some("asks_you")),
            Rank::NeedsYou
        );
        assert_eq!(
            Rank::of_session(S::Running, Some("something new")),
            Rank::Working
        );
        assert_eq!(Rank::of_session(S::Failed, Some("asks_you")), Rank::Ended);
        assert_eq!(
            Rank::of_conversation(true, Some("asks_you")),
            Rank::NeedsYou
        );
        assert_eq!(Rank::of_conversation(true, Some("done_review")), Rank::Open);
        assert_eq!(Rank::of_conversation(true, None), Rank::Open);
        assert_eq!(Rank::of_conversation(false, Some("working")), Rank::Ended);
        assert!(
            Rank::NeedsYou < Rank::Working
                && Rank::Working < Rank::Open
                && Rank::Open < Rank::Ended
        );
    }

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
