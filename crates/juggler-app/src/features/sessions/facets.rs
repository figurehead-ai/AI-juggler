//! E31 (AJ-E3103): what a list of sessions' filters offer. For one list
//! (a page's Sessions, in the state and search it shows), the projects,
//! Apps, Accounts, Macs and tools its rows are on, each with how many rows
//! it would show, so a filter never offers a choice that shows nothing and
//! its numbers are the list's own (the same rows `sessions_page` pages
//! through, read by the same rules).
//!
//! Each kind's values are counted with the other filters set (the 0.4.6
//! RC walk K2, F-K2-5: with App = Codex CLI set, History's Tool menu still
//! offered Claude Code, which would show nothing): a menu says what each
//! of its choices would show with the rest as they are, never its own
//! choice's narrowing.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::rows::{SessionsQuery, read};
use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::state::AppState;

/// One value a filter offers, and how many rows of the list it shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FacetInfo {
    /// Its id, as the query names it (`""`: a project filter's "No
    /// project").
    pub id: String,
    /// The rows it shows.
    #[ts(type = "number")]
    pub count: u64,
}

/// A list's filters: each kind's values, the most rows first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SessionsFacetsInfo {
    /// The projects its rows are in (`""` for the rows in none).
    pub projects: Vec<FacetInfo>,
    /// The Apps (`cli:<folder>`, `app:<host>:<instance folder>`).
    pub apps: Vec<FacetInfo>,
    /// The Accounts their folders are signed in to (rows on a folder
    /// whose sign-in isn't known are on none of them).
    pub subscriptions: Vec<FacetInfo>,
    /// The Macs they ran on.
    pub machines: Vec<FacetInfo>,
    /// Their tools, by adapter id.
    pub tools: Vec<FacetInfo>,
}

/// What a list's filters have chosen, over its scope (the query: its page's
/// own entity, its state and search). Each narrows the other kinds'
/// values, never its own.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FacetFilters {
    /// A project (`""`: "No project").
    #[serde(default)]
    #[ts(optional = nullable)]
    pub project_id: Option<String>,
    /// An App.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub app_id: Option<String>,
    /// An Account (`""`: "No account").
    #[serde(default)]
    #[ts(optional = nullable)]
    pub subscription_id: Option<String>,
    /// A Mac.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub machine_id: Option<String>,
    /// A tool.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub tool_id: Option<String>,
}

/// The kinds a list's filters offer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Project,
    App,
    Account,
    Machine,
    Tool,
}

const KINDS: [Kind; 5] = [
    Kind::Project,
    Kind::App,
    Kind::Account,
    Kind::Machine,
    Kind::Tool,
];

impl FacetFilters {
    /// The value chosen for `kind`.
    fn of(&self, kind: Kind) -> Option<&String> {
        match kind {
            Kind::Project => self.project_id.as_ref(),
            Kind::App => self.app_id.as_ref(),
            Kind::Account => self.subscription_id.as_ref(),
            Kind::Machine => self.machine_id.as_ref(),
            Kind::Tool => self.tool_id.as_ref(),
        }
    }

    /// `scope` narrowed by every chosen value but `except`'s.
    fn narrow(&self, scope: &SessionsQuery, except: Option<Kind>) -> SessionsQuery {
        let mut query = scope.clone();
        for kind in KINDS {
            let Some(value) = self.of(kind).filter(|_| Some(kind) != except) else {
                continue;
            };
            let field = match kind {
                Kind::Project => &mut query.project_id,
                Kind::App => &mut query.app_id,
                Kind::Account => &mut query.subscription_id,
                Kind::Machine => &mut query.machine_id,
                Kind::Tool => &mut query.tool_id,
            };
            *field = Some(value.clone());
        }
        query
    }
}

/// Each kind's tally over one read.
#[derive(Default)]
struct Tallies([BTreeMap<String, u64>; 5]);

impl Tallies {
    fn of(&mut self, kind: Kind) -> Vec<FacetInfo> {
        ranked(std::mem::take(&mut self.0[kind as usize]))
    }
}

/// The values counted, the most rows first, then by id.
fn ranked(tally: BTreeMap<String, u64>) -> Vec<FacetInfo> {
    let mut out: Vec<FacetInfo> = tally
        .into_iter()
        .map(|(id, count)| FacetInfo { id, count })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.id.cmp(&b.id)));
    out
}

/// What `query`'s list's filters offer, over the rows its state
/// (`which`), time and search show, each kind's values counted with every
/// other kind's chosen value in `filters` (its own left out).
///
/// # Errors
///
/// Storage errors.
pub fn sessions_facets_impl(
    state: &Arc<AppState>,
    query: &SessionsQuery,
    filters: &FacetFilters,
) -> AppResult<SessionsFacetsInfo> {
    let signed_in: BTreeMap<String, String> = lock_storage_arc(state)?
        .subscriptions()
        .folders()?
        .into_iter()
        .filter_map(|(folder, account)| Some((folder, account?)))
        .collect();
    // One read for every kind with nothing chosen, one more for each kind
    // with a choice (its own left out).
    let mut every = tally(state, &filters.narrow(query, None), &signed_in)?;
    let mut info = SessionsFacetsInfo::default();
    for kind in KINDS {
        let values = if filters.of(kind).is_some() {
            tally(state, &filters.narrow(query, Some(kind)), &signed_in)?.of(kind)
        } else {
            every.of(kind)
        };
        *match kind {
            Kind::Project => &mut info.projects,
            Kind::App => &mut info.apps,
            Kind::Account => &mut info.subscriptions,
            Kind::Machine => &mut info.machines,
            Kind::Tool => &mut info.tools,
        } = values;
    }
    Ok(info)
}

/// Each kind's values over `query`'s rows (`signed_in`: each folder's
/// Account).
fn tally(
    state: &Arc<AppState>,
    query: &SessionsQuery,
    signed_in: &BTreeMap<String, String>,
) -> AppResult<Tallies> {
    let read = read(state, query, true)?;
    let which = super::rows::Which::of(query);
    let mut tallies = Tallies::default();
    for candidate in read.candidates.iter().filter(|c| c.shown_by(which)) {
        let of = &candidate.of;
        let mut count =
            |kind: Kind, id: String| *tallies.0[kind as usize].entry(id).or_insert(0) += 1;
        count(Kind::Project, of.project.clone().unwrap_or_default());
        // 0.4.3: "" is in no App (its desktop App's records don't list it).
        if !of.app.is_empty() {
            count(Kind::App, of.app.clone());
        }
        count(Kind::Machine, of.machine.clone());
        count(Kind::Tool, of.tool.clone());
        // "" is "No account": a folder no sign-in was read for.
        let account = of.folder.as_ref().and_then(|folder| signed_in.get(folder));
        count(Kind::Account, account.cloned().unwrap_or_default());
    }
    Ok(tallies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_most_rows_come_first_then_by_id() {
        let tally: BTreeMap<String, u64> = [
            ("b".to_owned(), 2),
            ("a".to_owned(), 2),
            ("c".to_owned(), 5),
        ]
        .into();
        let ids: Vec<String> = ranked(tally).into_iter().map(|facet| facet.id).collect();
        assert_eq!(ids, ["c", "a", "b"]);
    }
}
