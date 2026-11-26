//! The operations (ADR-0038 §3): a small fixed set, each bounded by
//! design, instead of a query language.
//!
//! | Operation | What it does |
//! |---|---|
//! | [`seed`] | starting nodes: the host's own matches and exact aliases first, then keyword (FTS5) and vector (exact cosine) search fused by reciprocal rank; at most K |
//! | [`expand`] | a best-first walk by the kinds' rules, keeping the **best** K per step (by score, never an arbitrary K), with a node cap and a depth cap |
//! | [`rank`] | the top K, one node per fact key |
//! | [`matches`] | a small typed pattern |
//! | `at` on every read | only what was valid at that time ("as of") |
//! | [`Graph::touch`](crate::Graph::touch) | uses queued, applied in the next writer hold |
//! | [`explain`] | why each node was chosen: its seed, path, rule and score |
//!
//! **The score** of a node reached from a seed:
//! relevance × priority weight × 0.5^(age ÷ half-life) × usefulness ×
//! confidence, times, for every step of its path, the rule's decay × the
//! link's weight × the link's confidence. Priority weights: low 0.5,
//! normal 1, high 1.5, pinned 2. Age: days since the later of its
//! `as_of` and its last use. Usefulness: 0.75 + 0.5 × salience (1 at the
//! default salience 0.5).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use rusqlite::Connection;

use crate::error::Result;
use crate::index::{Edge, NodeEntry, Snapshot};
use crate::pack::{Dir, Pack};
use crate::schema::{FTS_BUCKETS, fts_table};
use crate::types::{NodeId, normalise};

const DAY_MS: f64 = 86_400_000.0;
/// The reciprocal-rank constant (the usual 60).
const RRF_K: f64 = 60.0;
/// The most edges one step reads from one node before it stops, even if
/// they could still score (a hub's bound; the strongest come first).
pub const EDGES_PER_STEP: usize = 20_000;
/// The longest alias phrase looked for in a text, in words.
const ALIAS_WORDS: usize = 5;

/// How a seed was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Via {
    /// The host's own match (an entity the question named).
    Given,
    /// An exact alias in the text.
    Alias,
    /// Keyword search.
    Keyword,
    /// Vector search.
    Vector,
}

impl Via {
    /// The code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Given => "given",
            Self::Alias => "alias",
            Self::Keyword => "keyword",
            Self::Vector => "vector",
        }
    }
}

/// What [`seed`] looks for.
#[derive(Clone, Debug, Default)]
pub struct SeedQuery<'a> {
    /// The words (the user's own: never another agent's).
    pub text: &'a str,
    /// The text's embedding, if the host has an embedder.
    pub vector: Option<&'a [f32]>,
    /// The host's own matches: node keys with their relevance (0..=1).
    pub given: &'a [(String, f32)],
    /// Only these kinds (empty: any).
    pub kinds: &'a [&'a str],
    /// At most this many seeds.
    pub k: usize,
    /// Only what was valid then.
    pub at: Option<i64>,
}

/// A starting node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seed {
    /// The node.
    pub node: NodeId,
    /// How relevant, 0..=1.
    pub relevance: f32,
    /// How it was found.
    pub via: Via,
}

fn usable(entry: &NodeEntry, at: Option<i64>, untrusted: bool) -> bool {
    !entry.archived && entry.valid_at(at) && (untrusted || !entry.untrusted)
}

/// The words of a text as an FTS5 query: each word quoted (so the user's
/// words are never query syntax), joined by OR.
#[must_use]
pub fn fts_query(text: &str) -> Option<String> {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
        .take(16)
        .map(|w| format!("\"{}\"", w.to_lowercase()))
        .collect();
    (!words.is_empty()).then(|| words.join(" OR "))
}

/// Cosine similarity (0 when either is all zeros or the sizes differ).
#[must_use]
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

fn floats(blob: &[u8]) -> impl Iterator<Item = f32> + '_ {
    blob.as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
}

/// The dot product of a query (already of length 1) with a stored
/// vector's bytes, eight lanes at a time.
fn dot(query: &[f32], blob: &[u8]) -> f32 {
    let mut acc = [0.0f32; 8];
    let lanes = query
        .as_chunks::<8>()
        .0
        .iter()
        .zip(blob.as_chunks::<32>().0);
    for (q, b) in lanes {
        let bytes = b.as_chunks::<4>().0;
        for i in 0..8 {
            acc[i] += q[i] * f32::from_le_bytes(bytes[i]);
        }
    }
    let done = query.len() / 8 * 8;
    let tail: f32 = query[done..]
        .iter()
        .zip(floats(&blob[done * 4..]))
        .map(|(x, y)| x * y)
        .sum();
    acc.iter().sum::<f32>() + tail
}

/// The nearest stored vectors to `query` by exact cosine (stored vectors
/// are of length 1, so it is one dot product each), best first, at most
/// `k`.
///
/// # Errors
///
/// SQLite errors.
pub fn nearest(conn: &Connection, query: &[f32], k: usize) -> Result<Vec<(NodeId, f32)>> {
    let norm: f32 = query.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 || k == 0 {
        return Ok(Vec::new());
    }
    let unit: Vec<f32> = query.iter().map(|x| x / norm).collect();
    let width = unit.len() * 4;
    let mut stmt = conn.prepare_cached("SELECT ids, v FROM gyrus_vectors WHERE dim = ?1")?;
    let mut rows = stmt.query([i64::try_from(unit.len()).unwrap_or(i64::MAX)])?;
    let mut best: Vec<(NodeId, f32)> = Vec::with_capacity(k + 1);
    while let Some(row) = rows.next()? {
        let as_blob = |at: usize| {
            row.get_ref(at)?.as_blob().map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    at,
                    rusqlite::types::Type::Blob,
                    Box::new(e),
                )
            })
        };
        let (ids, vectors) = (as_blob(0)?, as_blob(1)?);
        for (id, v) in ids
            .as_chunks::<8>()
            .0
            .iter()
            .zip(vectors.chunks_exact(width))
        {
            let sim = dot(&unit, v);
            if best.len() < k || best.last().is_some_and(|(_, s)| sim > *s) {
                let id = NodeId::from_le_bytes(*id);
                let at = best.partition_point(|(_, s)| *s >= sim);
                best.insert(at, (id, sim));
                best.truncate(k);
            }
        }
    }
    Ok(best)
}

/// A word in more than this share of the keyword index's labels is a
/// stop word: it ranks nothing (its BM25 weight is near nothing) and
/// costs a score per label, so keyword search leaves it out.
pub const STOP_WORD_SHARE: f64 = 0.02;

/// The query's informative words, by their share of one bucket's labels
/// (a sample of one bucket): stop words left out ([`STOP_WORD_SHARE`]).
fn informative(conn: &Connection, text: &str) -> Result<Option<String>> {
    let Some(all) = fts_query(text) else {
        return Ok(None);
    };
    let sample = fts_table(0);
    let total: i64 = conn
        .prepare_cached(&format!("SELECT count(*) FROM {sample}"))?
        .query_row([], |row| row.get(0))?;
    // A small index: every word counts (and costs little).
    if total < 400 {
        return Ok(Some(all));
    }
    let mut kept = Vec::new();
    let mut count = conn.prepare_cached(&format!(
        "SELECT count(*) FROM {sample} WHERE {sample} MATCH ?1"
    ))?;
    for word in all.split(" OR ") {
        let n: i64 = count.query_row([word], |row| row.get(0))?;
        #[expect(
            clippy::cast_precision_loss,
            reason = "label counts are far below 2^52"
        )]
        let share = n as f64 / total as f64;
        if share <= STOP_WORD_SHARE {
            kept.push(word);
        }
    }
    Ok((!kept.is_empty()).then(|| kept.join(" OR ")))
}

/// Every bucket's matches in one statement, best first (BM25 is negative,
/// better is lower), at most `k`.
fn keyword(conn: &Connection, query: &str, k: usize) -> Result<Vec<(NodeId, f64)>> {
    let union: Vec<String> = (0..FTS_BUCKETS)
        .map(|bucket| {
            let table = fts_table(bucket);
            format!(
                "SELECT rowid AS id, bm25({table}) AS score FROM {table} WHERE {table} MATCH ?1"
            )
        })
        .collect();
    let sql = format!("{} ORDER BY score, id LIMIT ?2", union.join(" UNION ALL "));
    let limit = i64::try_from(k).unwrap_or(i64::MAX);
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(rusqlite::params![query, limit], |row| {
        Ok((row.get::<_, NodeId>(0)?, row.get::<_, f64>(1)?))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Finds the starting nodes for a question (see the module).
///
/// # Errors
///
/// SQLite errors of the keyword or vector search.
pub fn seed(
    conn: &Connection,
    snap: &Snapshot,
    pack: &Pack,
    q: &SeedQuery<'_>,
) -> Result<Vec<Seed>> {
    let kinds: Vec<u16> = q.kinds.iter().filter_map(|k| pack.kind_index(k)).collect();
    let fits = |id: NodeId| {
        snap.node(id).is_some_and(|n| {
            usable(n, q.at, false) && (kinds.is_empty() || kinds.contains(&n.kind))
        })
    };
    let mut out: Vec<Seed> = Vec::new();
    let mut seen: HashSet<NodeId> = HashSet::new();
    let mut add = |out: &mut Vec<Seed>, seed: Seed| {
        if seen.insert(seed.node) {
            out.push(seed);
        } else if let Some(have) = out.iter_mut().find(|s| s.node == seed.node) {
            have.relevance = have.relevance.max(seed.relevance);
        }
    };
    for (key, relevance) in q.given {
        if let Some(id) = snap.id_of(key).filter(|id| fits(*id)) {
            add(
                &mut out,
                Seed {
                    node: id,
                    relevance: relevance.clamp(0.0, 1.0),
                    via: Via::Given,
                },
            );
        }
    }
    let words: Vec<String> = normalise(q.text)
        .split(' ')
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_owned())
        .filter(|w| !w.is_empty())
        .collect();
    for len in (1..=ALIAS_WORDS.min(words.len())).rev() {
        for window in words.windows(len) {
            for id in snap.by_alias(&window.join(" ")) {
                if fits(*id) {
                    add(
                        &mut out,
                        Seed {
                            node: *id,
                            relevance: 1.0,
                            via: Via::Alias,
                        },
                    );
                }
            }
        }
    }
    let k = q.k.max(1);
    let mut fused: HashMap<NodeId, (f64, Via)> = HashMap::new();
    if let Some(query) = informative(conn, q.text)? {
        let hits = keyword(conn, &query, k * 2)?;
        for (rank, (id, _)) in hits
            .iter()
            .filter(|(id, _)| fits(*id))
            .take(k * 2)
            .enumerate()
        {
            let e = fused.entry(*id).or_insert((0.0, Via::Keyword));
            e.0 += 1.0 / (RRF_K + 1.0 + f64::from(u32::try_from(rank).unwrap_or(u32::MAX)));
        }
    }
    if let Some(vector) = q.vector {
        let near = nearest(conn, vector, k * 4)?;
        for (rank, (id, _)) in near
            .iter()
            .filter(|(id, _)| fits(*id))
            .take(k * 2)
            .enumerate()
        {
            let e = fused.entry(*id).or_insert((0.0, Via::Vector));
            e.0 += 1.0 / (RRF_K + 1.0 + f64::from(u32::try_from(rank).unwrap_or(u32::MAX)));
        }
    }
    let top = fused.values().map(|(s, _)| *s).fold(0.0, f64::max);
    let mut fused: Vec<(NodeId, f64, Via)> =
        fused.into_iter().map(|(id, (s, v))| (id, s, v)).collect();
    fused.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    for (id, score, via) in fused {
        let relevance = 0.25 + 0.65 * (score / top);
        add(
            &mut out,
            Seed {
                node: id,
                relevance: narrow(relevance),
                via,
            },
        );
    }
    out.truncate(k);
    Ok(out)
}

#[expect(clippy::cast_possible_truncation, reason = "scores are 0..=1")]
const fn narrow(x: f64) -> f32 {
    x as f32
}

/// The priority weight (low 0.5, normal 1, high 1.5, pinned 2).
#[must_use]
pub fn priority_weight(priority: u8) -> f32 {
    match priority {
        0 => 0.5,
        1 => 1.0,
        2 => 1.5,
        _ => 2.0,
    }
}

/// A node's own factor: priority weight × recency × usefulness ×
/// confidence, at `now`.
#[must_use]
pub fn node_factor(pack: &Pack, entry: &NodeEntry, now: i64) -> f32 {
    let half_life = pack.kind(entry.kind).map_or(30.0, |k| k.half_life_days);
    let last = entry.last_used.unwrap_or(entry.as_of).max(entry.as_of);
    #[expect(
        clippy::cast_precision_loss,
        reason = "ages in ms fit an f64 well enough"
    )]
    let age_days = (now.saturating_sub(last).max(0) as f64) / DAY_MS;
    let recency = narrow(0.5f64.powf(age_days / half_life));
    priority_weight(entry.priority) * recency * (0.75 + 0.5 * entry.salience) * entry.confidence
}

/// The most a node's own factor can be (pinned, fresh, salient, sure):
/// what lets a step stop reading a hub's weaker links.
const FACTOR_MAX: f32 = 2.0 * 1.25;

/// How a walk runs.
#[derive(Clone, Copy, Debug)]
pub struct Walk {
    /// Steps from a seed, at most (3 by default).
    pub depth: u8,
    /// Nodes kept in all, at most.
    pub cap: usize,
    /// Only what was valid then ("as of").
    pub at: Option<i64>,
    /// Now, for recency.
    pub now: i64,
    /// Also walk through nodes whose label is another agent's words.
    pub untrusted: bool,
}

impl Walk {
    /// Depth 3, at most `cap` nodes, at `now`.
    #[must_use]
    pub const fn new(cap: usize, now: i64) -> Self {
        Self {
            depth: 3,
            cap,
            at: None,
            now,
            untrusted: false,
        }
    }

    /// As of `t`.
    #[must_use]
    pub const fn at(mut self, t: i64) -> Self {
        self.at = Some(t);
        self
    }
}

/// One step of a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hop {
    /// From.
    pub from: NodeId,
    /// The link kind's index.
    pub link: u16,
    /// Followed outwards (source → target) or inwards.
    pub out: bool,
    /// To.
    pub to: NodeId,
}

/// A node a walk reached.
#[derive(Clone, Debug, PartialEq)]
pub struct Reached {
    /// The node.
    pub node: NodeId,
    /// Its score.
    pub score: f32,
    /// The seed it was reached from.
    pub seed: NodeId,
    /// How that seed was found.
    pub via: Via,
    /// The path from the seed (empty for the seed itself).
    pub path: Vec<Hop>,
}

#[derive(PartialEq)]
struct Queued(f32, NodeId);

impl Eq for Queued {}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Queued {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0).then(other.1.cmp(&self.1))
    }
}

fn edges(snap: &Snapshot, id: NodeId, link: u16, dir: Dir) -> [(&[Edge], bool); 2] {
    match dir {
        Dir::Out => [(snap.out(id, link), true), (&[], true)],
        Dir::In => [(snap.inward(id, link), false), (&[], false)],
        Dir::Both => [(snap.out(id, link), true), (snap.inward(id, link), false)],
    }
}

/// Walks from the seeds by the kinds' rules (see the module), returning
/// every node kept, best first, at most `walk.cap`.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one best-first loop; its steps (rules, governing rules, the bounded scan) read in order"
)]
pub fn expand(snap: &Snapshot, pack: &Pack, seeds: &[Seed], walk: &Walk) -> Vec<Reached> {
    let mut best: HashMap<NodeId, Reached> = HashMap::new();
    let mut heap = BinaryHeap::new();
    for s in seeds {
        let Some(entry) = snap.node(s.node) else {
            continue;
        };
        let score = s.relevance * node_factor(pack, entry, walk.now);
        let better = best.get(&s.node).is_none_or(|r| score > r.score);
        if better {
            best.insert(
                s.node,
                Reached {
                    node: s.node,
                    score,
                    seed: s.node,
                    via: s.via,
                    path: Vec::new(),
                },
            );
            heap.push(Queued(score, s.node));
        }
    }
    let mut done: HashSet<NodeId> = HashSet::new();
    while let Some(Queued(score, id)) = heap.pop() {
        if done.len() >= walk.cap {
            break;
        }
        let Some(here) = best.get(&id) else {
            continue;
        };
        if score < here.score || !done.insert(id) {
            continue;
        }
        let here = here.clone();
        let depth = u8::try_from(here.path.len()).unwrap_or(u8::MAX);
        if depth >= walk.depth {
            continue;
        }
        let Some(entry) = snap.node(id) else {
            continue;
        };
        // A rule governing this node narrows its walk.
        let mut blocked: Vec<u16> = Vec::new();
        let mut keep_cap: Option<u16> = None;
        for rule in snap.rules_over(id) {
            if let Some(over) = snap.node(*rule).and_then(|r| r.rule.as_ref())
                && snap.node(*rule).is_some_and(|r| usable(r, walk.at, false))
            {
                blocked.extend(&over.block);
                keep_cap = match (keep_cap, over.keep) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (a, b) => a.or(b),
                };
            }
        }
        for (link, rule) in pack.walks(entry.kind) {
            if depth >= rule.depth || blocked.contains(link) {
                continue;
            }
            let keep = usize::from(keep_cap.map_or(rule.keep, |k| k.min(rule.keep)));
            let mut found: Vec<(f32, NodeId, bool)> = Vec::new();
            for (list, out) in edges(snap, id, *link, rule.dir) {
                let mut kth = f32::MIN;
                for edge in list.iter().take(EDGES_PER_STEP) {
                    // The strongest links come first, so once even a perfect
                    // node at this weight can't beat the K-th kept, no later
                    // link can: the best K are found without reading the rest.
                    if found.len() >= keep && score * rule.decay * edge.weight * FACTOR_MAX < kth {
                        break;
                    }
                    let step = score * rule.decay * edge.weight * edge.confidence;
                    if !edge.valid_at(walk.at) {
                        continue;
                    }
                    let Some(other) = snap.node(edge.other) else {
                        continue;
                    };
                    if !usable(other, walk.at, walk.untrusted) {
                        continue;
                    }
                    let cand = step * node_factor(pack, other, walk.now);
                    found.push((cand, edge.other, out));
                    if found.len() >= keep {
                        found.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
                        found.truncate(keep);
                        kth = found.last().map_or(f32::MIN, |f| f.0);
                    }
                }
            }
            found.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            found.truncate(keep);
            for (cand, other, out) in found {
                if done.contains(&other) {
                    continue;
                }
                if best.get(&other).is_none_or(|r| cand > r.score) {
                    let mut path = here.path.clone();
                    path.push(Hop {
                        from: id,
                        link: *link,
                        out,
                        to: other,
                    });
                    best.insert(
                        other,
                        Reached {
                            node: other,
                            score: cand,
                            seed: here.seed,
                            via: here.via,
                            path,
                        },
                    );
                    heap.push(Queued(cand, other));
                }
            }
        }
    }
    let mut out: Vec<Reached> = best.into_values().collect();
    out.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.node.cmp(&b.node)));
    out.truncate(walk.cap);
    out
}

/// The top `k`, best first, one node per fact key (the first, best one
/// kept).
#[must_use]
pub fn rank(snap: &Snapshot, reached: &[Reached], k: usize) -> Vec<Reached> {
    let mut sorted: Vec<&Reached> = reached.iter().collect();
    sorted.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.node.cmp(&b.node)));
    let mut facts: HashSet<&str> = HashSet::new();
    let mut out = Vec::new();
    for r in sorted {
        if out.len() >= k {
            break;
        }
        let fact = snap.node(r.node).and_then(|n| n.fact.as_deref());
        if let Some(fact) = fact
            && !facts.insert(fact)
        {
            continue;
        }
        out.push(r.clone());
    }
    out
}

/// Why one node was chosen.
#[derive(Clone, Debug, PartialEq)]
pub struct Why {
    /// The node.
    pub node: NodeId,
    /// Its key.
    pub key: String,
    /// Its kind.
    pub kind: String,
    /// The seed's key.
    pub seed: String,
    /// How the seed was found.
    pub via: Via,
    /// The path, one line per step (`session:web -part_of-> project:api`).
    pub path: Vec<String>,
    /// The last step's walk rule (`part_of out`), `None` for a seed.
    pub rule: Option<String>,
    /// Its score.
    pub score: f32,
    /// The reasoning questions its kind brings.
    pub questions: Vec<String>,
}

/// Says why each node was chosen: its seed and how it was found, the path
/// and the rule of its last step, its score, and its kind's questions.
#[must_use]
pub fn explain(snap: &Snapshot, pack: &Pack, chosen: &[Reached]) -> Vec<Why> {
    let key = |id: NodeId| {
        snap.node(id)
            .map_or_else(String::new, |n| n.key.to_string())
    };
    chosen
        .iter()
        .map(|r| {
            let kind = snap.node(r.node).map_or(0, |n| n.kind);
            Why {
                node: r.node,
                key: key(r.node),
                kind: pack.kind_name(kind).to_owned(),
                seed: key(r.seed),
                via: r.via,
                path: r
                    .path
                    .iter()
                    .map(|h| {
                        let name = pack.link_name(h.link);
                        if h.out {
                            format!("{} -{name}-> {}", key(h.from), key(h.to))
                        } else {
                            format!("{} <-{name}- {}", key(h.from), key(h.to))
                        }
                    })
                    .collect(),
                rule: r.path.last().map(|h| {
                    format!(
                        "{} {}",
                        pack.link_name(h.link),
                        if h.out { "out" } else { "in" }
                    )
                }),
                score: r.score,
                questions: pack
                    .kind(kind)
                    .map(|k| k.questions.clone())
                    .unwrap_or_default(),
            }
        })
        .collect()
}

/// One variable of a [`Pattern`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Var {
    /// Its kind (`None`: any).
    pub kind: Option<String>,
}

/// One link a [`Pattern`] requires, between two of its variables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Need {
    /// The source variable.
    pub from: usize,
    /// The link kind.
    pub link: String,
    /// The target variable.
    pub to: usize,
}

/// A small typed pattern: variables with kinds, the links between them,
/// which variables must be different nodes, and an optional anchor.
/// "Sessions of the same project that share a memory, across Apps":
/// s1, s2 sessions, p a project, m a memory, a1, a2 Apps; s1 → p, s2 → p
/// (`part_of`), m → s1, m → s2 (`about`), s1 → a1, s2 → a2 (`runs_in`);
/// s1 ≠ s2, a1 ≠ a2.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pattern {
    /// The variables.
    pub vars: Vec<Var>,
    /// The links required.
    pub needs: Vec<Need>,
    /// Pairs of variables that must be different nodes.
    pub distinct: Vec<(usize, usize)>,
    /// A variable bound to one node from the start.
    pub anchor: Option<(usize, NodeId)>,
    /// At most this many matches.
    pub limit: usize,
    /// Only what was valid then.
    pub at: Option<i64>,
}

/// The most candidate bindings [`matches`] tries before it stops (its
/// bound, whatever the pattern).
pub const MATCH_STEPS: usize = 200_000;

/// Every binding of the pattern's variables (each a node id per
/// variable), at most `limit`, in a stable order.
#[must_use]
pub fn matches(snap: &Snapshot, pack: &Pack, pattern: &Pattern) -> Vec<Vec<NodeId>> {
    let n = pattern.vars.len();
    if n == 0 || pattern.limit == 0 {
        return Vec::new();
    }
    // Each variable's kind (`None`: any); a kind the pack doesn't know
    // matches nothing.
    let mut kinds: Vec<Option<u16>> = Vec::with_capacity(n);
    for var in &pattern.vars {
        match var.kind.as_deref() {
            None => kinds.push(None),
            Some(name) => match pack.kind_index(name) {
                Some(kind) => kinds.push(Some(kind)),
                None => return Vec::new(),
            },
        }
    }
    let mut needs = Vec::new();
    for need in &pattern.needs {
        let Some(link) = pack.link_index(&need.link) else {
            return Vec::new();
        };
        if need.from >= n || need.to >= n {
            return Vec::new();
        }
        needs.push((need.from, link, need.to));
    }
    // Bind in an order where each next variable touches a bound one when
    // it can (the anchor first).
    let mut order: Vec<usize> = Vec::new();
    if let Some((var, _)) = pattern.anchor {
        order.push(var);
    }
    while order.len() < n {
        let next = (0..n)
            .filter(|v| !order.contains(v))
            .find(|v| {
                needs
                    .iter()
                    .any(|(f, _, t)| (f == v && order.contains(t)) || (t == v && order.contains(f)))
            })
            .or_else(|| (0..n).find(|v| !order.contains(v)));
        if let Some(v) = next {
            order.push(v);
        }
    }
    let mut state = MatchState {
        snap,
        kinds: &kinds,
        needs: &needs,
        pattern,
        order: &order,
        bound: vec![None; n],
        out: Vec::new(),
        steps: 0,
    };
    state.bind(0);
    state.out
}

struct MatchState<'a> {
    snap: &'a Snapshot,
    kinds: &'a [Option<u16>],
    needs: &'a [(usize, u16, usize)],
    pattern: &'a Pattern,
    order: &'a [usize],
    bound: Vec<Option<NodeId>>,
    out: Vec<Vec<NodeId>>,
    steps: usize,
}

impl MatchState<'_> {
    fn candidates(&self, var: usize) -> Vec<NodeId> {
        if let Some((a, id)) = self.pattern.anchor
            && a == var
        {
            return vec![id];
        }
        for (f, link, t) in self.needs {
            if *t == var
                && let Some(src) = self.bound[*f]
            {
                return self
                    .snap
                    .out(src, *link)
                    .iter()
                    .filter(|e| e.valid_at(self.pattern.at))
                    .map(|e| e.other)
                    .collect();
            }
            if *f == var
                && let Some(dst) = self.bound[*t]
            {
                return self
                    .snap
                    .inward(dst, *link)
                    .iter()
                    .filter(|e| e.valid_at(self.pattern.at))
                    .map(|e| e.other)
                    .collect();
            }
        }
        let mut all: Vec<NodeId> = match self.kinds[var] {
            Some(kind) => self.snap.of_kind(kind).collect(),
            None => self.snap.ids().collect(),
        };
        all.sort_unstable();
        all
    }

    fn fits(&self, var: usize, id: NodeId) -> bool {
        let Some(entry) = self.snap.node(id) else {
            return false;
        };
        if !usable(entry, self.pattern.at, false) {
            return false;
        }
        if let Some(kind) = self.kinds[var]
            && entry.kind != kind
        {
            return false;
        }
        for (a, b) in &self.pattern.distinct {
            let other = if *a == var {
                *b
            } else if *b == var {
                *a
            } else {
                continue;
            };
            if self.bound[other] == Some(id) {
                return false;
            }
        }
        // Every need between this variable and a bound one holds.
        self.needs.iter().all(|(f, link, t)| {
            let (src, dst) = match (*f == var, *t == var) {
                (true, _) => (Some(id), self.bound[*t]),
                (_, true) => (self.bound[*f], Some(id)),
                _ => return true,
            };
            match (src, dst) {
                (Some(s), Some(d)) => self
                    .snap
                    .out(s, *link)
                    .iter()
                    .any(|e| e.other == d && e.valid_at(self.pattern.at)),
                _ => true,
            }
        })
    }

    fn bind(&mut self, i: usize) {
        if self.out.len() >= self.pattern.limit || self.steps >= MATCH_STEPS {
            return;
        }
        if i == self.order.len() {
            self.out
                .push(self.bound.iter().map(|b| b.unwrap_or_default()).collect());
            return;
        }
        let var = self.order[i];
        let mut cands = self.candidates(var);
        cands.sort_unstable();
        cands.dedup();
        for id in cands {
            self.steps += 1;
            if self.steps >= MATCH_STEPS || self.out.len() >= self.pattern.limit {
                return;
            }
            if self.fits(var, id) {
                self.bound[var] = Some(id);
                self.bind(i + 1);
                self.bound[var] = None;
            }
        }
    }
}
