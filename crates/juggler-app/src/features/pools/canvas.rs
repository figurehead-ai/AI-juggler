//! 0.3 (redesign §5): a pool's canvas — where the user put each member,
//! who talks to whom, what waits on the user, and the pool's newest
//! activity in words.
//!
//! - The canvas names the members it shows (a page or a few of them); the
//!   answer is about those alone and never grows with the history (I6):
//!   edges are counted per pair since [`EDGE_WINDOW_MS`], what waits is
//!   the newest few, the stream the newest [`STREAM_LINES`].
//! - Positions are kept under what a member stands for (its Work item,
//!   else its session), so a switch keeps its node where it was.
//! - Only the app's own window calls these. Placing a node or tidying up
//!   changes nothing but the canvas: no pool, member, role or session.

use std::collections::HashMap;
use std::sync::Arc;

use juggler_core::agent_access::AgentAction;
use juggler_core::model::{SessionId, WorkItemId};
use juggler_core::storage::{HeldAsk, MemberRef, POSITION_LIMIT, SENDER_USER, Storage};

use super::dto::{
    CanvasEdgeInfo, NodeAskInfo, NodePositionInfo, PendingInfo, PoolCanvasInfo, PoolPreviewInfo,
};
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Edges count the last week of messages and hand-offs.
pub const EDGE_WINDOW_MS: i64 = 7 * 24 * 60 * 60 * 1000;
/// What waits is looked for over the last 30 days.
const PENDING_WINDOW_MS: i64 = 30 * 24 * 60 * 60 * 1000;
/// The most members one canvas read names.
pub const MOST_NODES: usize = 200;
/// The most pools one read of the Pools screen's cards names.
pub const MOST_PREVIEWS: usize = 60;
/// Lines of the live stream.
const STREAM_LINES: u32 = 30;
/// The most pairs, and the most waiting messages, read.
const MOST_PAIRS: u32 = 400;
const MOST_PENDING: u32 = 100;

/// Each member ref the canvas's sessions answer to, by session.
struct Refs {
    by_ref: HashMap<String, String>,
    works: HashMap<SessionId, WorkItemId>,
}

impl Refs {
    fn new(storage: &Storage, sessions: &[String]) -> AppResult<Self> {
        let works = storage.switch_graph().works_by_session()?;
        let mut by_ref = HashMap::new();
        for id in sessions {
            let session = SessionId::new(id.as_str());
            by_ref.insert(format!("session:{id}"), id.clone());
            by_ref.insert(
                MemberRef::of_session(&session, works.get(&session)).to_wire(),
                id.clone(),
            );
        }
        Ok(Self { by_ref, works })
    }

    /// The canvas's session a stored ref stands for now, when it's shown.
    fn session_of(&self, member_ref: &str) -> Option<String> {
        if member_ref == SENDER_USER {
            return None;
        }
        if let Some(id) = self.by_ref.get(member_ref) {
            return Some(id.clone());
        }
        let anchor = MemberRef::parse(member_ref)?.anchor(&self.works).to_wire();
        self.by_ref.get(&anchor).cloned()
    }

    /// What a session stands for (its positions' key).
    fn anchor_of(&self, session: &str) -> String {
        let id = SessionId::new(session);
        MemberRef::of_session(&id, self.works.get(&id)).to_wire()
    }
}

/// The edge from `from` to `to`, made when it's the first.
fn edge_in(
    edges: &mut HashMap<(String, String), CanvasEdgeInfo>,
    from: String,
    to: String,
) -> &mut CanvasEdgeInfo {
    edges
        .entry((from.clone(), to.clone()))
        .or_insert_with(|| CanvasEdgeInfo {
            from_session_id: from,
            to_session_id: to,
            messages: 0,
            tasks: 0,
            last_at_ms: 0,
            thread_id: None,
            pending: Vec::new(),
        })
}

/// A session Deliver can put a message into now: it runs, in AI Juggler.
fn deliverable(state: &Arc<AppState>, storage: &Storage, session: &str) -> bool {
    let id = SessionId::new(session);
    state.runtime.is_driven(&id)
        && storage
            .sessions()
            .get(&id)
            .is_ok_and(|record| crate::features::messages::is_live(record.status))
}

/// A list of session ids, deduplicated and capped.
fn checked(ids: Vec<String>, most: usize, what: &str) -> AppResult<Vec<String>> {
    if ids.len() > most {
        return Err(AppError::Refused(format!(
            "At most {most} {what} at a time"
        )));
    }
    let mut seen = std::collections::HashSet::new();
    Ok(ids
        .into_iter()
        .filter(|id| seen.insert(id.clone()))
        .collect())
}

/// Where the user put the shown members.
fn positions(storage: &Storage, pool_id: &str, refs: &Refs) -> AppResult<Vec<NodePositionInfo>> {
    let mut out: Vec<NodePositionInfo> = Vec::new();
    for row in storage.pool_canvas().positions(pool_id)? {
        if let Some(session_id) = refs.session_of(&row.member_ref) {
            out.retain(|known| known.session_id != session_id);
            out.push(NodePositionInfo {
                session_id,
                x: row.x,
                y: row.y,
            });
        }
    }
    out.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    Ok(out)
}

/// Who talked to whom among the shown members since `since`, and what
/// waits on each edge.
fn edges_of(
    state: &Arc<AppState>,
    storage: &Storage,
    pool_id: &str,
    refs: &Refs,
    asks: &[HeldAsk],
    since: i64,
) -> AppResult<Vec<CanvasEdgeInfo>> {
    let canvas = storage.pool_canvas();
    let deliver = AgentAction::DeliverToSession.as_str();
    let pair = |from: &str, to: &str| match (refs.session_of(from), refs.session_of(to)) {
        (Some(from), Some(to)) if from != to => Some((from, to)),
        _ => None,
    };
    let mut folded: HashMap<(String, String), CanvasEdgeInfo> = HashMap::new();
    for row in canvas.message_traffic(pool_id, since, MOST_PAIRS)? {
        let Some((from, to)) = pair(&row.from_ref, &row.to_ref) else {
            continue;
        };
        let edge = edge_in(&mut folded, from, to);
        edge.messages += row.count;
        if row.last_at >= edge.last_at_ms {
            edge.last_at_ms = row.last_at;
            edge.thread_id = Some(row.thread_id);
        }
    }
    for row in canvas.task_handoffs(pool_id, since, MOST_PAIRS)? {
        let Some((from, to)) = pair(&row.from_ref, &row.to_ref) else {
            continue;
        };
        let edge = edge_in(&mut folded, from, to);
        edge.tasks += row.count;
        edge.last_at_ms = edge.last_at_ms.max(row.last_at);
    }
    let pending_since = since + EDGE_WINDOW_MS - PENDING_WINDOW_MS;
    for row in canvas.pending(pool_id, pending_since, MOST_PENDING)? {
        let Some((from, to)) = pair(&row.from_ref, &row.to_ref) else {
            continue;
        };
        let ask = asks.iter().find(|ask| ask.object_ref == row.message_id);
        let kind = match (ask, row.state.as_str()) {
            (Some(ask), _) if ask.action == deliver => "hop_limit",
            (Some(_), "held") => "asked",
            (None, "unread") if deliverable(state, storage, &to) => "waiting",
            _ => continue,
        };
        let edge = edge_in(&mut folded, from, to);
        edge.last_at_ms = edge.last_at_ms.max(row.at);
        if edge.thread_id.is_none() {
            edge.thread_id = Some(row.thread_id.clone());
        }
        edge.pending.push(PendingInfo {
            message_id: row.message_id,
            thread_id: row.thread_id,
            subject: row.subject,
            recipient: row.to_ref,
            kind: kind.to_owned(),
            ask_id: ask.map(|ask| ask.id.clone()),
            at_ms: row.at,
        });
    }
    let mut edges: Vec<CanvasEdgeInfo> = folded.into_values().collect();
    edges.sort_by(|a, b| {
        (a.from_session_id.as_str(), a.to_session_id.as_str())
            .cmp(&(b.from_session_id.as_str(), b.to_session_id.as_str()))
    });
    Ok(edges)
}

/// The agents' asks in the pool that wait on the user (a hop-limit hold
/// is its edge's), each on the member that asked, oldest first.
fn asks_of(
    storage: &Storage,
    pool_id: &str,
    refs: &Refs,
    asks: &[HeldAsk],
) -> AppResult<Vec<NodeAskInfo>> {
    let deliver = AgentAction::DeliverToSession.as_str();
    let offer = |action: &str| {
        AgentAction::parse(action).is_some_and(|action| {
            storage
                .agent_access()
                .approvals(pool_id, action)
                .unwrap_or(0)
                >= 1
        })
    };
    let mut out: Vec<NodeAskInfo> = asks
        .iter()
        .filter(|ask| ask.action != deliver)
        .map(|ask| NodeAskInfo {
            session_id: refs.session_of(&ask.actor_ref),
            source: "message".to_owned(),
            id: ask.id.clone(),
            title: ask.summary.clone(),
            offer_always: offer(&ask.action),
            at_ms: ask.created_at,
        })
        .collect();
    out.extend(
        storage
            .tasks()
            .open_needs(0, 200)?
            .into_iter()
            .filter(|need| need.pool_id == pool_id && need.kind == "ask")
            .map(|need| NodeAskInfo {
                session_id: refs.session_of(&need.actor_ref),
                source: "task".to_owned(),
                offer_always: offer(&need.action),
                id: need.id,
                title: need.summary,
                at_ms: need.created_at,
            }),
    );
    out.sort_by_key(|ask| ask.at_ms);
    Ok(out)
}

/// A pool's canvas, for the members `session_ids` it shows.
///
/// # Errors
///
/// [`AppError::Refused`] for a pool that isn't there or too many members
/// named; storage failures.
pub fn canvas_impl(
    state: &Arc<AppState>,
    pool_id: &str,
    session_ids: Vec<String>,
) -> AppResult<PoolCanvasInfo> {
    let session_ids = checked(session_ids, MOST_NODES, "sessions")?;
    let storage = lock_storage_arc(state)?;
    super::live_pool(&storage, pool_id)?;
    let refs = Refs::new(&storage, &session_ids)?;
    let since = crate::commands::now_millis() - EDGE_WINDOW_MS;
    let canvas = storage.pool_canvas();
    // Open asks in this pool: by message (a held one), and by member.
    let asks: Vec<HeldAsk> = storage
        .messages()
        .open_asks(0, 200)?
        .into_iter()
        .filter(|ask| ask.pool_id == pool_id)
        .collect();
    let edges = edges_of(state, &storage, pool_id, &refs, &asks, since)?;
    let node_asks = asks_of(&storage, pool_id, &refs, &asks)?;
    let stream = canvas
        .stream(pool_id, STREAM_LINES)?
        .into_iter()
        .map(|entry| super::canvas_words::line(&entry, |r| refs.session_of(r)))
        .collect();
    Ok(PoolCanvasInfo {
        pool_id: pool_id.to_owned(),
        positions: positions(&storage, pool_id, &refs)?,
        edges,
        asks: node_asks,
        stream,
        since_ms: since,
    })
}

/// Puts members' nodes where the user dropped them (one, or every shown
/// one the first time the user arranges a pool, so the rest stay put).
/// Nothing is written unless every node is in the pool and on the canvas.
///
/// # Errors
///
/// [`AppError::Refused`] for a pool that isn't there, a session that
/// isn't in it, a place off the canvas, or too many nodes; storage
/// failures.
pub fn place_impl(
    state: &Arc<AppState>,
    pool_id: &str,
    nodes: &[NodePositionInfo],
) -> AppResult<()> {
    if nodes.len() > MOST_NODES {
        return Err(AppError::Refused(format!(
            "At most {MOST_NODES} sessions at a time"
        )));
    }
    let storage = lock_storage_arc(state)?;
    super::live_pool(&storage, pool_id)?;
    let memberships = super::members::memberships(&storage)?;
    for node in nodes {
        let session = SessionId::new(node.session_id.as_str());
        let in_pool = storage
            .sessions()
            .get(&session)
            .is_ok_and(|record| memberships.place(&session, record.project.as_ref()).0 == pool_id);
        if !in_pool {
            return Err(AppError::Refused(
                "That session isn't in this pool any more".to_owned(),
            ));
        }
        let on_canvas = node.x.is_finite()
            && node.y.is_finite()
            && node.x.abs() <= POSITION_LIMIT
            && node.y.abs() <= POSITION_LIMIT;
        if !on_canvas {
            return Err(AppError::Refused("That place is off the canvas".to_owned()));
        }
    }
    let ids: Vec<String> = nodes.iter().map(|node| node.session_id.clone()).collect();
    let refs = Refs::new(&storage, &ids)?;
    let canvas = storage.pool_canvas();
    for node in nodes {
        canvas.place(pool_id, &refs.anchor_of(&node.session_id), node.x, node.y)?;
    }
    Ok(())
}

/// Forgets where the user put a pool's members ("Tidy up").
///
/// # Errors
///
/// [`AppError::Refused`] for a pool that isn't there; storage failures.
pub fn tidy_impl(state: &Arc<AppState>, pool_id: &str) -> AppResult<()> {
    let storage = lock_storage_arc(state)?;
    super::live_pool(&storage, pool_id)?;
    storage.pool_canvas().forget(pool_id)?;
    Ok(())
}

/// The Pools screen's cards: each pool's open members' positions and its
/// tasks in progress.
///
/// # Errors
///
/// [`AppError::Refused`] for too many pools named; storage failures.
pub fn previews_impl(
    state: &Arc<AppState>,
    pool_ids: Vec<String>,
) -> AppResult<Vec<PoolPreviewInfo>> {
    let pool_ids = checked(pool_ids, MOST_PREVIEWS, "pools")?;
    let storage = lock_storage_arc(state)?;
    let working: Vec<String> = crate::commands::working_records(&storage)?
        .into_iter()
        .map(|record| record.id.as_str().to_owned())
        .collect();
    let refs = Refs::new(&storage, &working)?;
    let moving: HashMap<String, u32> = storage
        .pool_canvas()
        .moving(&pool_ids)?
        .into_iter()
        .collect();
    pool_ids
        .into_iter()
        .map(|pool_id| {
            Ok(PoolPreviewInfo {
                positions: positions(&storage, &pool_id, &refs)?,
                moving: moving.get(&pool_id).copied().unwrap_or(0),
                pool_id,
            })
        })
        .collect()
}
