//! E31 (AJ-E3103): what a list of sessions' filters offer. For one list
//! (a page's Sessions, in the state and search it shows), the projects,
//! Apps, Accounts and Macs its rows are on, each with how many rows it
//! would show, so a filter never offers a choice that shows nothing and
//! its numbers are the list's own (the same rows `sessions_page` pages
//! through, read by the same rules).

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Serialize;
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
/// (`which`), time and search show.
///
/// # Errors
///
/// Storage errors.
pub fn sessions_facets_impl(
    state: &Arc<AppState>,
    query: &SessionsQuery,
) -> AppResult<SessionsFacetsInfo> {
    let read = read(state, query, true)?;
    let which = super::rows::Which::of(query);
    let signed_in: BTreeMap<String, String> = lock_storage_arc(state)?
        .subscriptions()
        .folders()?
        .into_iter()
        .filter_map(|(folder, account)| Some((folder, account?)))
        .collect();
    let (mut projects, mut apps, mut subscriptions, mut machines) = (
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    for candidate in read.candidates.iter().filter(|c| c.shown_by(which)) {
        let of = &candidate.of;
        *projects
            .entry(of.project.clone().unwrap_or_default())
            .or_insert(0) += 1;
        *apps.entry(of.app.clone()).or_insert(0) += 1;
        *machines.entry(of.machine.clone()).or_insert(0) += 1;
        if let Some(account) = of.folder.as_ref().and_then(|folder| signed_in.get(folder)) {
            *subscriptions.entry(account.clone()).or_insert(0) += 1;
        }
    }
    Ok(SessionsFacetsInfo {
        projects: ranked(projects),
        apps: ranked(apps),
        subscriptions: ranked(subscriptions),
        machines: ranked(machines),
    })
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
