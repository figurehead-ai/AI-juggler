//! AJ-E209: the Switch Graph overview: epochs and edges per work.

use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::switch::dto::{EdgeInfo, EpochNodeInfo, WorkGraphInfo};
use crate::state::AppState;

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) per the command docs.
pub fn switch_graph_overview_impl(
    state: &Arc<AppState>,
    project_id: &str,
) -> AppResult<Vec<WorkGraphInfo>> {
    graph_works_for(state, project_id)
}

/// The graph walk behind [`switch_graph_overview_impl`].
fn graph_works_for(state: &Arc<AppState>, project_id: &str) -> AppResult<Vec<WorkGraphInfo>> {
    let storage = lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let accounts: std::collections::BTreeMap<_, _> = storage
        .accounts()
        .list()?
        .into_iter()
        .map(|account| (account.id.clone(), account.label.clone()))
        .collect();
    let works =
        graph.works_for_project(&juggler_core::model::ProjectId::new(project_id.to_owned()))?;
    let mut out = Vec::new();
    for work in works {
        out.push(work_graph_info(
            &storage,
            &graph,
            &accounts,
            &state.conversations,
            &work,
        )?);
    }
    drop(storage);
    // What each switch that wrote a handover told the other tool (read
    // without the storage lock: each read takes it).
    for edge in out.iter_mut().flat_map(|work| work.edges.iter_mut()) {
        if edge.failed_reason.is_none() {
            edge.handover = crate::features::handover::view::edge_summary(state, &edge.id);
        }
    }
    Ok(out)
}

/// Maps one work item to its timeline (collapse + nodes + edges +
/// the S-F9 hop note).
fn work_graph_info(
    storage: &juggler_core::storage::Storage,
    graph: &juggler_core::storage::SwitchGraphRepo<'_>,
    accounts: &std::collections::BTreeMap<juggler_core::model::AccountId, String>,
    conversations: &crate::features::conversations::ConversationIndex,
    work: &juggler_core::model::WorkItem,
) -> AppResult<WorkGraphInfo> {
    let epochs = graph.epochs_for_work(&work.id)?;
    let edges = graph.edges_for_work(&work.id)?;
    let has_branch = epochs.iter().any(|epoch| {
        epochs
            .iter()
            .filter(|other| other.resumed_from_epoch == Some(epoch.id.clone()))
            .count()
            > 1
    });
    // Collapse epochs older than the last fork/merge point.
    let cut = if has_branch {
        epochs
            .iter()
            .filter(|epoch| {
                epochs
                    .iter()
                    .filter(|other| other.resumed_from_epoch == Some(epoch.id.clone()))
                    .count()
                    > 1
            })
            .rev()
            .map(|epoch| epoch.id.clone())
            .next()
    } else {
        None
    };
    let keep_from = cut.or_else(|| epochs.first().map(|epoch| epoch.id.clone()));
    let visible_start = keep_from
        .and_then(|id| epochs.iter().position(|epoch| epoch.id == id))
        .unwrap_or(0);
    let collapsed = u32::try_from(visible_start).unwrap_or_default();
    let mut nodes = Vec::new();
    let newest = epochs.last().map(|epoch| epoch.id.clone());
    for epoch in &epochs[visible_start.min(epochs.len())..] {
        let native = epoch.native_id.as_deref().filter(|id| !id.is_empty());
        let resumable = native.is_some_and(|native| match &epoch.session_ref {
            Some(session) => storage
                .sessions()
                .get(session)
                .ok()
                .and_then(|record| {
                    record
                        .transcript_ref
                        .as_ref()
                        .map(|reference| reference.path.is_file())
                })
                .unwrap_or(false),
            // AJ-E2108: a conversation another app runs has no row; its
            // file is in the index, and Resume continues it here (live
            // lab 12: it read "no transcript" while switching back
            // resumed it).
            None => conversations
                .path_of(epoch.account.as_str(), native)
                .is_some_and(|path| path.is_file()),
        });
        nodes.push(EpochNodeInfo {
            id: epoch.id.to_string(),
            provider: epoch.provider.to_string(),
            account_label: accounts
                .get(&epoch.account)
                .cloned()
                .unwrap_or_else(|| "unknown".to_owned()),
            kind: "terminal".to_owned(),
            resumable,
            session_id: epoch.session_ref.as_ref().map(ToString::to_string),
            task: epoch.session_ref.as_ref().and_then(|session| {
                let last = newest.as_ref() == Some(&epoch.id);
                crate::features::tasks::views::epoch_task(storage, session.as_str(), last)
            }),
        });
    }
    let (wire_edges, hops, tokens) = graph_edge_summary(&edges);

    Ok(WorkGraphInfo {
        title: work.title.clone(),
        epochs: nodes,
        edges: wire_edges,
        collapsed,
        hop_note: juggler_handover::ranking::hop_note(hops, tokens),
    })
}

/// What an edge carried, in the user's words: the fidelity it showed
/// (its user label), else what its mechanism does.
pub(crate) fn edge_carried_words(edge: &juggler_core::model::SwitchEdge) -> &'static str {
    use juggler_core::model::SwitchMechanism;
    if let Some(level) = edge
        .fidelity_shown
        .as_deref()
        .and_then(juggler_handover::fidelity::FidelityLabel::from_wire)
    {
        return level.user_label();
    }
    match edge.mechanism {
        SwitchMechanism::NativeResume => "Continues exactly",
        SwitchMechanism::Transplant => "Same conversation, another app",
        SwitchMechanism::Replay => "Full story, fresh start",
        SwitchMechanism::Pack => "Handover notes",
    }
}

/// Wires one work's edges for the timeline: labels (what carried, a
/// switch back, the failed reason) and the hop/token counts feeding
/// the S-F9 note.
fn graph_edge_summary(edges: &[juggler_core::model::SwitchEdge]) -> (Vec<EdgeInfo>, u32, u64) {
    use juggler_core::model::EdgeState;
    let mut wire_edges = Vec::new();
    for edge in edges {
        let back = edge.parent_edge_a.is_some();
        let carried = edge_carried_words(edge);
        let label = match &edge.state {
            EdgeState::Failed { reason, .. } => format!("failed · {reason}"),
            _ if back => format!("Switched back · {carried}"),
            _ => carried.to_owned(),
        };
        wire_edges.push(EdgeInfo {
            id: edge.id.to_string(),
            from: edge.from_epoch.to_string(),
            to: edge.to_epoch.as_ref().map(ToString::to_string),
            label,
            failed_reason: match &edge.state {
                EdgeState::Failed { reason, .. } => Some(reason.clone()),
                _ => None,
            },
            switch_back: back,
            handover: None,
        });
    }
    let hops = u32::try_from(
        edges
            .iter()
            .filter(|edge| matches!(edge.state, EdgeState::Confirmed))
            .count(),
    )
    .unwrap_or_default();
    let tokens = edges
        .iter()
        .filter_map(|edge| edge.manifest_sha256.as_ref().map(|_| 0_u64))
        .sum::<u64>();
    (wire_edges, hops, tokens)
}
