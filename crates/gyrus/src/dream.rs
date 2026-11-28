//! The idle pass, "dreaming" (ADR-0038 §4): tiers, decay, archive,
//! duplicates, folds and weak links, planned from a snapshot as one
//! bounded [`Batch`] a host writes through its own gate in one short hold.
//! It never deletes what the user stated: an archived node is kept, out of
//! every walk, and a later use brings it back.
//!
//! Per node, in id order from a cursor, until the batch holds `max_ops`:
//! - **Promote** by use: short → mid after the kind's
//!   `promote_short_after` uses; mid → long after `promote_mid_after`
//!   (priority one step up, never to pinned).
//! - **Decay** when idle `demote_after_days`: priority one step down (pinned
//!   never decays); a long node not stated by the user goes to mid.
//! - **Salience** recomputed: half recency (the kind's half-life), half use
//!   (`ln(1 + uses) / ln(21)`, capped at 1); written when it moved by 0.05.
//! - **Archive** a low-priority node idle `archive_after_days` (never a
//!   long node the user stated).
//! - **Fold** a child idle the fold's days into its parent.
//! - **Duplicates** of a mergeable kind, both trusted and neither an
//!   agent's: two nodes with the same neighbours (Jaccard ≥ 0.8) and the
//!   same label or vectors at cosine ≥ 0.92 are proposed as a `same_as`
//!   link; a `same_as` pair with the same label, or cosine ≥ 0.95, is
//!   merged (the less used into the more used).
//! - **Prune** a link weaker than `prune_below` whose ends are both idle.

use std::collections::{BTreeSet, HashMap};

use rusqlite::Connection;

use crate::error::Result;
use crate::index::{NodeEntry, Snapshot};
use crate::ops::cosine;
use crate::pack::Pack;
use crate::types::{Authority, Batch, NewLink, NodeId, Op, Tier, normalise};

const DAY_MS: f64 = 86_400_000.0;
/// Neighbour overlap for a proposed duplicate.
pub const SAME_NEIGHBOURS: f32 = 0.8;
/// Vector similarity for a proposed duplicate.
pub const PROPOSE_COSINE: f32 = 0.92;
/// Vector similarity for a merge.
pub const MERGE_COSINE: f32 = 0.95;
/// The most candidates one node's duplicate search reads.
const CANDIDATES: usize = 64;

/// How a pass runs.
#[derive(Clone, Copy, Debug)]
pub struct DreamOpts {
    /// Now.
    pub now: i64,
    /// The most writes in the batch.
    pub max_ops: usize,
    /// Continue after this node id (0: from the start).
    pub after: NodeId,
    /// Links weaker than this, between idle nodes, are pruned.
    pub prune_below: f32,
}

impl DreamOpts {
    /// At `now`, at most `max_ops` writes, from the start.
    #[must_use]
    pub const fn new(now: i64, max_ops: usize) -> Self {
        Self {
            now,
            max_ops,
            after: 0,
            prune_below: 0.05,
        }
    }
}

/// What a pass planned, by kind of change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DreamCounts {
    /// Nodes read.
    pub scanned: usize,
    /// Tier moves up.
    pub promoted: usize,
    /// Priority or tier moves down.
    pub decayed: usize,
    /// Salience rewritten (alone).
    pub salience: usize,
    /// Archived.
    pub archived: usize,
    /// Folded into a parent.
    pub folded: usize,
    /// `same_as` links proposed.
    pub proposed: usize,
    /// Duplicates merged.
    pub merged: usize,
    /// Weak links pruned.
    pub pruned: usize,
}

/// A planned pass: the batch, where the next one starts (`None`: the
/// whole graph was read), and the counts (for a log: numbers only).
#[derive(Clone, Debug, Default)]
pub struct Dreamed {
    /// The writes.
    pub batch: Batch,
    /// The cursor for the next batch.
    pub next: Option<NodeId>,
    /// What it holds.
    pub counts: DreamCounts,
}

fn idle_days(entry: &NodeEntry, now: i64) -> f64 {
    let last = entry.last_used.unwrap_or(entry.as_of).max(entry.as_of);
    #[expect(
        clippy::cast_precision_loss,
        reason = "ages in ms fit an f64 well enough"
    )]
    let days = (now.saturating_sub(last).max(0) as f64) / DAY_MS;
    days
}

#[expect(clippy::cast_possible_truncation, reason = "0..=1")]
fn salience(pack: &Pack, entry: &NodeEntry, now: i64) -> f32 {
    let half = pack.kind(entry.kind).map_or(30.0, |k| k.half_life_days);
    let recency = 0.5f64.powf(idle_days(entry, now) / half);
    #[expect(clippy::cast_precision_loss, reason = "use counts are small")]
    let uses = ((1.0 + entry.use_count.max(0) as f64).ln() / 21f64.ln()).min(1.0);
    (0.5 * recency + 0.5 * uses).clamp(0.0, 1.0) as f32
}

fn mergeable(pack: &Pack, entry: &NodeEntry) -> bool {
    !entry.untrusted
        && entry.authority != Authority::Agent
        && !entry.archived
        && pack.kind(entry.kind).is_some_and(|k| k.mergeable)
}

fn neighbours(snap: &Snapshot, id: NodeId) -> BTreeSet<(u16, NodeId)> {
    snap.out_groups(id)
        .iter()
        .flat_map(|g| g.edges.iter().map(move |e| (g.link, e.other)))
        .collect()
}

#[expect(clippy::cast_precision_loss, reason = "set sizes are small")]
fn jaccard(a: &BTreeSet<(u16, NodeId)>, b: &BTreeSet<(u16, NodeId)>) -> f32 {
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    a.intersection(b).count() as f32 / union as f32
}

/// How alike two nodes' words are: 1 for the same label, else their
/// vectors' cosine (0 without vectors).
fn alike(conn: &Connection, a: NodeId, b: NodeId) -> Result<f32> {
    let labels = crate::store::labels(conn, &[a, b])?;
    let (la, lb) = (labels.get(&a), labels.get(&b));
    if let (Some(la), Some(lb)) = (la, lb)
        && !la.trim().is_empty()
        && normalise(la) == normalise(lb)
    {
        return Ok(1.0);
    }
    match (
        crate::store::vector_of(conn, a)?,
        crate::store::vector_of(conn, b)?,
    ) {
        (Some(va), Some(vb)) => Ok(cosine(&va, &vb)),
        _ => Ok(0.0),
    }
}

/// Plans one bounded pass (see the module).
///
/// # Errors
///
/// SQLite errors (labels and vectors of duplicate candidates are read).
#[expect(
    clippy::too_many_lines,
    reason = "one pass, each rule a short block in order"
)]
pub fn plan(conn: &Connection, snap: &Snapshot, pack: &Pack, opts: &DreamOpts) -> Result<Dreamed> {
    let mut out = Dreamed::default();
    let mut ids: Vec<NodeId> = snap.ids().filter(|id| *id > opts.after).collect();
    ids.sort_unstable();
    let same_as = pack.same_as_link();
    let mut merging: BTreeSet<NodeId> = BTreeSet::new();
    let mut proposed: HashMap<NodeId, usize> = HashMap::new();
    // The node before: where its writes began, and the counts before it,
    // so one that overflows the batch is left whole for the next pass.
    let mut last: Option<(NodeId, usize, DreamCounts)> = None;
    for id in ids {
        if let Some(over) = overflow(&mut out, last, opts.max_ops) {
            out.next = Some(over);
            return Ok(out);
        }
        if out.batch.ops.len() >= opts.max_ops {
            out.next = Some(id - 1);
            return Ok(out);
        }
        last = Some((id, out.batch.ops.len(), out.counts));
        out.counts.scanned += 1;
        let Some(entry) = snap.node(id) else {
            continue;
        };
        if merging.contains(&id) {
            continue;
        }
        let Some(spec) = pack.kind(entry.kind) else {
            continue;
        };
        let idle = idle_days(entry, opts.now);
        let key = entry.key.to_string();
        if !entry.archived {
            let rules = spec.tiers;
            let uses = u32::try_from(entry.use_count.max(0)).unwrap_or(u32::MAX);
            let (mut tier, mut priority) = (entry.tier, entry.priority);
            let mut moved = false;
            if (tier == Tier::Short && uses >= rules.promote_short_after)
                || (tier == Tier::Mid && uses >= rules.promote_mid_after)
            {
                tier = tier.up();
                if priority < 2 {
                    priority += 1;
                }
                out.counts.promoted += 1;
                moved = true;
            } else if idle >= rules.demote_after_days {
                if (1..=2).contains(&priority) {
                    priority -= 1;
                    moved = true;
                }
                if tier == Tier::Long && entry.authority != Authority::User {
                    tier = Tier::Mid;
                    moved = true;
                }
                if moved {
                    out.counts.decayed += 1;
                }
            }
            let fresh = salience(pack, entry, opts.now);
            if moved || (fresh - entry.salience).abs() >= 0.05 {
                if !moved {
                    out.counts.salience += 1;
                }
                out.batch.op(Op::Maintain {
                    key: key.clone(),
                    tier,
                    priority,
                    salience: fresh,
                });
            }
            let stated = tier == Tier::Long && entry.authority == Authority::User;
            if priority == 0 && !stated && rules.archive_after_days.is_some_and(|days| idle >= days)
            {
                out.batch.op(Op::Archive { key: key.clone() });
                out.counts.archived += 1;
            } else {
                for (child, link, parent, after) in pack.folds() {
                    if entry.kind != *child || idle < *after {
                        continue;
                    }
                    if let Some(edge) = snap.out(id, *link).iter().find(|e| {
                        snap.node(e.other)
                            .is_some_and(|p| p.kind == *parent && !p.archived)
                    }) && let Some(p) = snap.node(edge.other)
                    {
                        out.batch.op(Op::Fold {
                            child: key.clone(),
                            parent: p.key.to_string(),
                        });
                        out.counts.folded += 1;
                        break;
                    }
                }
            }
        }
        if let Some(same) = same_as
            && mergeable(pack, entry)
        {
            // A `same_as` pair the words agree on is merged.
            for edge in snap.out(id, same) {
                let Some(other) = snap.node(edge.other) else {
                    continue;
                };
                if !mergeable(pack, other)
                    || other.kind != entry.kind
                    || merging.contains(&edge.other)
                {
                    continue;
                }
                if alike(conn, id, edge.other)? >= MERGE_COSINE {
                    let (from, into) = if (entry.use_count, edge.other) <= (other.use_count, id) {
                        (id, edge.other)
                    } else {
                        (edge.other, id)
                    };
                    let key_of =
                        |n: NodeId| snap.node(n).map(|e| e.key.to_string()).unwrap_or_default();
                    out.batch.op(Op::Merge {
                        from: key_of(from),
                        into: key_of(into),
                    });
                    merging.insert(from);
                    merging.insert(into);
                    out.counts.merged += 1;
                    break;
                }
            }
            if merging.contains(&id) {
                continue;
            }
            // Near-duplicates: the same neighbours and alike words.
            let mine = neighbours(snap, id);
            if let Some(&(link, first)) = mine.iter().next() {
                let have: BTreeSet<NodeId> = snap.out(id, same).iter().map(|e| e.other).collect();
                for edge in snap.inward(first, link).iter().take(CANDIDATES) {
                    let c = edge.other;
                    if c <= id || have.contains(&c) || proposed.get(&c) == Some(&1) {
                        continue;
                    }
                    let Some(other) = snap.node(c) else {
                        continue;
                    };
                    if other.kind != entry.kind || !mergeable(pack, other) {
                        continue;
                    }
                    let overlap = jaccard(&mine, &neighbours(snap, c));
                    if overlap < SAME_NEIGHBOURS || alike(conn, id, c)? < PROPOSE_COSINE {
                        continue;
                    }
                    let mut link = NewLink::new(&key, pack.link_name(same), &other.key, "dream");
                    link.weight = overlap;
                    out.batch.link(link);
                    proposed.insert(c, 1);
                    out.counts.proposed += 1;
                    break;
                }
            }
        }
        if idle >= spec.tiers.demote_after_days {
            for group in snap.out_groups(id) {
                for edge in &group.edges {
                    let weak = edge.weight < opts.prune_below;
                    let other_idle = snap
                        .node(edge.other)
                        .is_some_and(|o| idle_days(o, opts.now) >= spec.tiers.demote_after_days);
                    if weak && other_idle && Some(group.link) != same_as {
                        out.batch.op(Op::Unlink {
                            src: key.clone(),
                            kind: pack.link_name(group.link).to_owned(),
                            dst: snap
                                .node(edge.other)
                                .map(|o| o.key.to_string())
                                .unwrap_or_default(),
                        });
                        out.counts.pruned += 1;
                    }
                }
            }
        }
    }
    if let Some(over) = overflow(&mut out, last, opts.max_ops) {
        out.next = Some(over);
    }
    Ok(out)
}

/// When the last node's writes took the batch past `max`: they are taken
/// back (the next pass starts at that node), or, for a first node that
/// alone overflows, cut to `max` (the rest is found again next pass).
/// Returns where the next pass starts.
fn overflow(
    out: &mut Dreamed,
    last: Option<(NodeId, usize, DreamCounts)>,
    max: usize,
) -> Option<NodeId> {
    let (id, start, counts) = last?;
    if out.batch.ops.len() <= max {
        return None;
    }
    if start == 0 {
        out.batch.ops.truncate(max);
        return Some(id);
    }
    out.batch.ops.truncate(start);
    out.counts = counts;
    Some(id - 1)
}
