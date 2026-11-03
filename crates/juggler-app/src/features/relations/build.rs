//! AJ-E3102: the two parts of the relation-count cache, built from their
//! sources by the rules the lists narrow by (`features::sessions::rows`):
//! a session AI Juggler ran is on its launch settings' folder and that
//! folder's CLI App and Account; one another app started is on the
//! folder whose files hold it, the desktop App it was last seen in (else
//! its folder's CLI App), and the project its folder is in.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::{EntityKind, KINDS, Part, PartKind, Row};
use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::conversations::by_relation::ByRelation;
use crate::state::AppState;

/// The part `kind`, read now.
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn part(state: &Arc<AppState>, kind: PartKind) -> AppResult<Part> {
    match kind {
        PartKind::Own => own(state),
        PartKind::Outside => outside(state),
    }
}

/// Each folder's Account, where its sign-in was read (as an Account's
/// list narrows: folders not removed).
fn accounts_of_folders(
    storage: &juggler_core::storage::Storage,
) -> AppResult<BTreeMap<String, String>> {
    Ok(storage
        .subscriptions()
        .folders()?
        .into_iter()
        .filter_map(|(folder, account)| Some((folder, account?)))
        .collect())
}

/// A row from what each kind's entity is.
fn row(of: [(EntityKind, Option<String>); KINDS - 1], live: bool, archived: bool) -> Row {
    let mut out = Row {
        live,
        archived,
        ..Row::default()
    };
    out.of[EntityKind::All.index()] = Some(String::new());
    for (kind, value) in of {
        out.of[kind.index()] = value;
    }
    out
}

/// AI Juggler's sessions: one narrow pass over `instances` (no JSON
/// read), joined in memory to their folders, as History joins them.
fn own(state: &Arc<AppState>) -> AppResult<Part> {
    // The stamps first: whatever changes while this reads makes the part
    // stale at once, never silently included or missed.
    let events = state.relations.snapshot();
    let (stored, facets, accounts, recipes, lifecycle, signed_in) = {
        let storage = lock_storage_arc(state)?;
        let relations = storage.relations();
        let (accounts, recipes) = crate::commands::session_join_indexes(&storage)?;
        (
            relations.stamps()?,
            relations.session_facets()?,
            accounts,
            recipes,
            storage.sessions().lifecycle_latest().unwrap_or_default(),
            accounts_of_folders(&storage)?,
        )
    };
    let no_desktop = BTreeMap::new();
    let rows = facets.into_iter().map(|facet| {
        let (folder, _, tool, _) =
            crate::commands::resolved_join(&recipes, &accounts, &facet.launch_settings);
        let live = juggler_core::model::SessionStatus::parse(&facet.status)
            .is_some_and(crate::commands::is_active);
        let archived = lifecycle
            .get(&facet.id)
            .is_some_and(|(event, _)| event == "archived");
        row(
            [
                (EntityKind::Project, facet.project),
                (
                    EntityKind::App,
                    Some(crate::features::places::app_of(
                        &no_desktop,
                        folder.as_deref(),
                        None,
                    )),
                ),
                (
                    EntityKind::Subscription,
                    folder.as_ref().and_then(|id| signed_in.get(id).cloned()),
                ),
                (EntityKind::Folder, folder),
                (EntityKind::Tool, Some(tool)),
                (EntityKind::Machine, Some(facet.machine)),
                (EntityKind::LaunchSettings, Some(facet.launch_settings)),
            ],
            live,
            archived,
        )
    });
    Ok(Part::of(rows, events, stored))
}

/// The conversations other apps started, from the conversation index as
/// it stands, indexed by project, App and tool. Like the lists, the first
/// ask after a start reads the files itself unless the background pass
/// already is (then it counts what's read so far, and the pass's changes
/// make it stale as it goes).
fn outside(state: &Arc<AppState>) -> AppResult<Part> {
    crate::features::conversations::read_if_cold(state)?;
    let events = state.relations.snapshot();
    let (stored, signed_in) = {
        let storage = lock_storage_arc(state)?;
        (
            storage.relations().stamps()?,
            accounts_of_folders(&storage)?,
        )
    };
    // They're files on this Mac.
    let machine = crate::commands::local_machine_id(state).to_string();
    let desktop = crate::features::places::desktop_app_by_session(state);
    let index = ByRelation::of(
        crate::features::conversations::commands::list_indexed(state)?,
        &desktop,
    );
    let rows = index.facets().iter().map(|facet| {
        row(
            [
                (EntityKind::Project, facet.project.clone()),
                (EntityKind::Folder, Some(facet.folder.clone())),
                (EntityKind::App, Some(facet.app.clone())),
                (
                    EntityKind::Subscription,
                    signed_in.get(&facet.folder).cloned(),
                ),
                (EntityKind::Tool, Some(facet.tool.clone())),
                (EntityKind::Machine, Some(machine.clone())),
                (EntityKind::LaunchSettings, None),
            ],
            facet.live,
            false,
        )
    });
    let mut part = Part::of(rows, events, stored);
    part.conversations = Some(Arc::new(index));
    Ok(part)
}
