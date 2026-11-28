//! The SQLite side: every write inside the caller's transaction, returning
//! the change set ([`Pending`]) the in-memory index applies after commit.
//! SQLite is the record; nothing reaches the index that wasn't committed.

use std::collections::{BTreeSet, HashMap};

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::error::{Error, Refusal, Result};
use crate::pack::Pack;
use crate::schema::{bucket_of, fts_table};
use crate::types::{
    ALIAS_MAX, ALIASES_MAX, Authority, Batch, DIM_MAX, LABEL_MAX, LinkId, NewLink, NewNode, NodeId,
    Op, PROPS_MAX, Tier, key_like, normalise,
};

/// A node as stored (what the index is built from).
#[derive(Clone, Debug, PartialEq)]
pub struct NodeRow {
    /// Its id.
    pub id: NodeId,
    /// Its kind.
    pub kind: String,
    /// Its key.
    pub key: String,
    /// Its label.
    pub label: String,
    /// The label is another agent's words.
    pub untrusted: bool,
    /// Its properties.
    pub props: Value,
    /// Its tier.
    pub tier: Tier,
    /// Its priority.
    pub priority: u8,
    /// Its salience.
    pub salience: f32,
    /// Its confidence.
    pub confidence: f32,
    /// Its authority.
    pub authority: Authority,
    /// Valid from.
    pub valid_from: Option<i64>,
    /// Valid until.
    pub valid_to: Option<i64>,
    /// As of.
    pub as_of: i64,
    /// Last used.
    pub last_used: Option<i64>,
    /// Times used.
    pub use_count: i64,
    /// Archived (out of walks).
    pub archived: bool,
    /// Its aliases, normalised.
    pub aliases: Vec<String>,
}

/// A link as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkRow {
    /// Its id.
    pub id: LinkId,
    /// The source node.
    pub src: NodeId,
    /// The target node.
    pub dst: NodeId,
    /// Its kind.
    pub kind: String,
    /// Its weight.
    pub weight: f32,
    /// Its confidence.
    pub confidence: f32,
    /// Valid from.
    pub valid_from: Option<i64>,
    /// Valid until.
    pub valid_to: Option<i64>,
}

/// What one committed write changed, for the in-memory index. Apply it
/// only after the transaction that made it committed
/// ([`crate::Graph::publish`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pending {
    /// The write generation this change made.
    pub generation: i64,
    /// Nodes written (their full row as stored).
    pub nodes: Vec<NodeRow>,
    /// Links written.
    pub links: Vec<LinkRow>,
    /// Links removed.
    pub unlinked: Vec<LinkId>,
    /// Nodes removed (their links with them).
    pub removed: Vec<NodeId>,
}

impl Pending {
    /// Nothing changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
            && self.links.is_empty()
            && self.unlinked.is_empty()
            && self.removed.is_empty()
    }

    fn merge(&mut self, other: Self) {
        self.generation = self.generation.max(other.generation);
        self.nodes.extend(other.nodes);
        self.links.extend(other.links);
        self.unlinked.extend(other.unlinked);
        self.removed.extend(other.removed);
    }
}

const NODE_COLUMNS: &str = "id, kind, key, label, untrusted, props, tier, priority, salience, \
    confidence, authority, valid_from, valid_to, as_of, last_used, use_count, archived_at";

fn bad(column: usize, what: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        rusqlite::types::Type::Text,
        format!("unknown code {what}").into(),
    )
}

fn node_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeRow> {
    let props: String = row.get(5)?;
    let tier: String = row.get(6)?;
    let authority: String = row.get(10)?;
    let priority: i64 = row.get(7)?;
    let salience: f64 = row.get(8)?;
    let confidence: f64 = row.get(9)?;
    Ok(NodeRow {
        id: row.get(0)?,
        kind: row.get(1)?,
        key: row.get(2)?,
        label: row.get(3)?,
        untrusted: row.get(4)?,
        props: serde_json::from_str(&props).unwrap_or(Value::Null),
        tier: Tier::parse(&tier).ok_or_else(|| bad(6, &tier))?,
        priority: u8::try_from(priority.clamp(0, 3)).unwrap_or(1),
        salience: narrow(salience),
        confidence: narrow(confidence),
        authority: Authority::parse(&authority).ok_or_else(|| bad(10, &authority))?,
        valid_from: row.get(11)?,
        valid_to: row.get(12)?,
        as_of: row.get(13)?,
        last_used: row.get(14)?,
        use_count: row.get(15)?,
        archived: row.get::<_, Option<i64>>(16)?.is_some(),
        aliases: Vec::new(),
    })
}

/// An `f64` from SQLite as the `f32` the index keeps.
#[expect(
    clippy::cast_possible_truncation,
    reason = "weights, confidences and saliences are 0..=1"
)]
const fn narrow(value: f64) -> f32 {
    value as f32
}

fn link_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LinkRow> {
    Ok(LinkRow {
        id: row.get(0)?,
        src: row.get(1)?,
        dst: row.get(2)?,
        kind: row.get(3)?,
        weight: narrow(row.get(4)?),
        confidence: narrow(row.get(5)?),
        valid_from: row.get(6)?,
        valid_to: row.get(7)?,
    })
}

const LINK_COLUMNS: &str = "id, src, dst, kind, weight, confidence, valid_from, valid_to";

/// Reads one node (with its aliases) by id.
///
/// # Errors
///
/// SQLite errors.
pub fn node_by_id(conn: &Connection, id: NodeId) -> Result<Option<NodeRow>> {
    let mut node = conn
        .prepare_cached(&format!(
            "SELECT {NODE_COLUMNS} FROM gyrus_nodes WHERE id = ?1"
        ))?
        .query_row([id], node_row)
        .optional()?;
    if let Some(node) = &mut node {
        node.aliases = conn
            .prepare_cached("SELECT alias FROM gyrus_aliases WHERE node = ?1 ORDER BY alias")?
            .query_map([id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
    }
    Ok(node)
}

/// A node's id by key.
///
/// # Errors
///
/// SQLite errors.
pub fn id_of(conn: &Connection, key: &str) -> Result<Option<NodeId>> {
    Ok(conn
        .prepare_cached("SELECT id FROM gyrus_nodes WHERE key = ?1")?
        .query_row([key], |row| row.get(0))
        .optional()?)
}

/// Labels of nodes by id, for the few nodes a context shows.
///
/// # Errors
///
/// SQLite errors.
pub fn labels(conn: &Connection, ids: &[NodeId]) -> Result<HashMap<NodeId, String>> {
    let mut stmt = conn.prepare_cached("SELECT label FROM gyrus_nodes WHERE id = ?1")?;
    let mut out = HashMap::new();
    for id in ids {
        if let Some(label) = stmt.query_row([id], |row| row.get(0)).optional()? {
            out.insert(*id, label);
        }
    }
    Ok(out)
}

fn link_by_id(conn: &Connection, id: LinkId) -> Result<Option<LinkRow>> {
    Ok(conn
        .prepare_cached(&format!(
            "SELECT {LINK_COLUMNS} FROM gyrus_links WHERE id = ?1"
        ))?
        .query_row([id], link_row)
        .optional()?)
}

fn bump(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "UPDATE gyrus_meta SET v = v + 1 WHERE k = 'gen' RETURNING v",
        [],
        |row| row.get(0),
    )?)
}

/// The write generation stored.
///
/// # Errors
///
/// SQLite errors.
pub fn generation(conn: &Connection) -> Result<i64> {
    Ok(
        conn.query_row("SELECT v FROM gyrus_meta WHERE k = 'gen'", [], |row| {
            row.get(0)
        })?,
    )
}

fn refuse<T>(why: Refusal) -> Result<T> {
    Err(Error::Refused(why))
}

fn unit(x: f32) -> bool {
    x.is_finite() && (0.0..=1.0).contains(&x)
}

fn props_ok(props: &Value) -> bool {
    props.is_object() && serde_json::to_string(props).is_ok_and(|s| s.len() <= PROPS_MAX)
}

/// Checks a node against the pack and the bounds, without the database.
///
/// # Errors
///
/// [`Error::Refused`] with the reason.
pub fn check_node(pack: &Pack, node: &NewNode) -> Result<()> {
    if pack.kind_index(&node.kind).is_none() {
        return refuse(Refusal::UndeclaredKind);
    }
    if !key_like(&node.key) {
        return refuse(Refusal::Key);
    }
    if node.tier == Tier::Immediate {
        return refuse(Refusal::Immediate);
    }
    if node.label.as_str().chars().count() > LABEL_MAX || !props_ok(&node.props) {
        return refuse(Refusal::Bounds);
    }
    if node.aliases.len() > ALIASES_MAX
        || node
            .aliases
            .iter()
            .any(|a| normalise(a).is_empty() || a.chars().count() > ALIAS_MAX)
    {
        return refuse(Refusal::Bounds);
    }
    if node
        .vector
        .as_ref()
        .is_some_and(|v| v.is_empty() || v.len() > DIM_MAX || v.iter().any(|x| !x.is_finite()))
    {
        return refuse(Refusal::Bounds);
    }
    if node.priority > 3 || !unit(node.salience) || !unit(node.confidence) {
        return refuse(Refusal::Range);
    }
    // An agent's words are untrusted by type: an agent node with a
    // trusted label, or an alias (aliases are found by exactly), is refused.
    if node.authority == Authority::Agent
        && ((!node.label.as_str().is_empty() && !node.label.is_untrusted())
            || !node.aliases.is_empty())
    {
        return refuse(Refusal::AgentNotUntrusted);
    }
    Ok(())
}

/// Checks a link's bounds, without the database.
///
/// # Errors
///
/// [`Error::Refused`] with the reason.
pub fn check_link(pack: &Pack, link: &NewLink) -> Result<()> {
    if pack.link_index(&link.kind).is_none() {
        return refuse(Refusal::UndeclaredLink);
    }
    if !key_like(&link.src) || !key_like(&link.dst) || link.src == link.dst {
        return refuse(Refusal::Key);
    }
    if !unit(link.weight) || !unit(link.confidence) {
        return refuse(Refusal::Range);
    }
    if !props_ok(&link.props) || link.provenance.len() > crate::types::KEY_MAX {
        return refuse(Refusal::Bounds);
    }
    Ok(())
}

/// Checks a whole batch without the database (a host runs this, and its
/// own checks, before it opens its transaction).
///
/// # Errors
///
/// [`Error::Refused`] with the first reason.
pub fn check(pack: &Pack, batch: &Batch) -> Result<()> {
    for op in &batch.ops {
        match op {
            Op::Upsert(node) => check_node(pack, node)?,
            Op::Link(link) => check_link(pack, link)?,
            Op::Maintain {
                tier,
                priority,
                salience,
                ..
            } => {
                if *tier == Tier::Immediate {
                    return refuse(Refusal::Immediate);
                }
                if *priority > 3 || !unit(*salience) {
                    return refuse(Refusal::Range);
                }
            }
            Op::Unlink { .. }
            | Op::Fold { .. }
            | Op::Close { .. }
            | Op::Archive { .. }
            | Op::Restore { .. }
            | Op::Merge { .. } => {}
        }
    }
    Ok(())
}

struct Existing {
    id: NodeId,
    kind: String,
    authority: Authority,
    tier: Tier,
    priority: u8,
    as_of: i64,
}

fn existing(conn: &Connection, key: &str) -> Result<Option<Existing>> {
    Ok(conn
        .prepare_cached(
            "SELECT id, kind, authority, tier, priority, as_of FROM gyrus_nodes WHERE key = ?1",
        )?
        .query_row([key], |row| {
            let authority: String = row.get(2)?;
            let tier: String = row.get(3)?;
            let priority: i64 = row.get(4)?;
            Ok(Existing {
                id: row.get(0)?,
                kind: row.get(1)?,
                authority: Authority::parse(&authority).ok_or_else(|| bad(2, &authority))?,
                tier: Tier::parse(&tier).ok_or_else(|| bad(3, &tier))?,
                priority: u8::try_from(priority.clamp(0, 3)).unwrap_or(1),
                as_of: row.get(5)?,
            })
        })
        .optional()?)
}

fn fts_delete(conn: &Connection, id: NodeId) -> Result<()> {
    conn.prepare_cached(&format!(
        "DELETE FROM {} WHERE rowid = ?1",
        fts_table(bucket_of(id))
    ))?
    .execute([id])?;
    Ok(())
}

fn fts_insert(conn: &Connection, id: NodeId, label: &str) -> Result<()> {
    conn.prepare_cached(&format!(
        "INSERT INTO {}(rowid, label) VALUES (?1, ?2)",
        fts_table(bucket_of(id))
    ))?
    .execute(params![id, label])?;
    Ok(())
}

/// A vector as stored: scaled to length 1 (so a cosine is a dot product),
/// as little-endian `f32`s.
#[must_use]
pub fn vector_blob(v: &[f32]) -> Vec<u8> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    let scale = if norm > 0.0 { 1.0 / norm } else { 0.0 };
    v.iter().flat_map(|x| (x * scale).to_le_bytes()).collect()
}

/// One block of vectors: the nodes' ids, and their vectors in the same
/// order.
struct Block {
    row: i64,
    ids: Vec<NodeId>,
    v: Vec<u8>,
}

fn ids_of(blob: &[u8]) -> Vec<NodeId> {
    blob.as_chunks::<8>()
        .0
        .iter()
        .map(|c| i64::from_le_bytes(*c))
        .collect()
}

fn ids_blob(ids: &[NodeId]) -> Vec<u8> {
    ids.iter().flat_map(|id| id.to_le_bytes()).collect()
}

/// The blocks (one per vector size) a node's vector would live in.
fn blocks_of(conn: &Connection, id: NodeId) -> Result<Vec<(i64, Block)>> {
    let block = id.div_euclid(crate::schema::VECTOR_BLOCK);
    let mut stmt =
        conn.prepare_cached("SELECT id, dim, ids, v FROM gyrus_vectors WHERE block = ?1")?;
    let rows = stmt.query_map([block], |row| {
        let ids: Vec<u8> = row.get(2)?;
        Ok((
            row.get::<_, i64>(1)?,
            Block {
                row: row.get(0)?,
                ids: ids_of(&ids),
                v: row.get(3)?,
            },
        ))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn write_block(conn: &Connection, dim: i64, block: i64, b: &Block) -> Result<()> {
    if b.ids.is_empty() {
        conn.prepare_cached("DELETE FROM gyrus_vectors WHERE id = ?1")?
            .execute([b.row])?;
    } else {
        conn.prepare_cached(
            "INSERT INTO gyrus_vectors (dim, block, ids, v) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (dim, block) DO UPDATE SET ids = excluded.ids, v = excluded.v",
        )?
        .execute(params![dim, block, ids_blob(&b.ids), b.v])?;
    }
    Ok(())
}

/// Removes a node's vector from its block (the block rewritten, or
/// deleted when it empties; with `secure_delete` on, the old pages are
/// zeroed).
///
/// # Errors
///
/// SQLite errors.
pub fn drop_vector(conn: &Connection, id: NodeId) -> Result<()> {
    let block = id.div_euclid(crate::schema::VECTOR_BLOCK);
    for (dim, mut b) in blocks_of(conn, id)? {
        let Some(at) = b.ids.iter().position(|n| *n == id) else {
            continue;
        };
        let width = usize::try_from(dim).unwrap_or(0) * 4;
        b.ids.remove(at);
        b.v.drain(at * width..(at + 1) * width);
        write_block(conn, dim, block, &b)?;
    }
    Ok(())
}

/// Stores a node's vector (scaled to length 1, in its block), replacing
/// one it had.
///
/// # Errors
///
/// SQLite errors.
pub fn put_vector(conn: &Connection, id: NodeId, v: &[f32]) -> Result<()> {
    drop_vector(conn, id)?;
    let dim = i64::try_from(v.len()).unwrap_or(0);
    let block = id.div_euclid(crate::schema::VECTOR_BLOCK);
    let mut b = blocks_of(conn, id)?
        .into_iter()
        .find(|(d, _)| *d == dim)
        .map_or(
            Block {
                row: 0,
                ids: Vec::new(),
                v: Vec::new(),
            },
            |(_, b)| b,
        );
    b.ids.push(id);
    b.v.extend(vector_blob(v));
    write_block(conn, dim, block, &b)
}

/// A node's stored vector (of length 1).
///
/// # Errors
///
/// SQLite errors.
pub fn vector_of(conn: &Connection, id: NodeId) -> Result<Option<Vec<f32>>> {
    for (dim, b) in blocks_of(conn, id)? {
        if let Some(at) = b.ids.iter().position(|n| *n == id) {
            let width = usize::try_from(dim).unwrap_or(0) * 4;
            return Ok(Some(
                b.v[at * width..(at + 1) * width]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| f32::from_le_bytes(*c))
                    .collect(),
            ));
        }
    }
    Ok(None)
}

fn upsert(conn: &Connection, pack: &Pack, node: &NewNode, now: i64) -> Result<NodeId> {
    let spec = pack
        .kind_index(&node.kind)
        .and_then(|k| pack.kind(k))
        .ok_or(Error::Refused(Refusal::UndeclaredKind))?;
    let untrusted = node.label.is_untrusted();
    let indexed = spec.searchable && !untrusted && !node.label.as_str().trim().is_empty();
    let props = serde_json::to_string(&node.props).map_err(|_| Error::Refused(Refusal::Bounds))?;
    let id = match existing(conn, &node.key)? {
        Some(old) => {
            if old.kind != node.kind {
                return refuse(Refusal::Key);
            }
            if node.authority.rank() < old.authority.rank() {
                return refuse(Refusal::LowerAuthority);
            }
            // Re-stating never demotes: the tier and priority stay at
            // least what they were; a write brings an archived node back.
            conn.prepare_cached(
                "UPDATE gyrus_nodes SET label = ?2, untrusted = ?3,
                     indexed = CASE WHEN ?4 THEN 1 WHEN indexed != 0 THEN 2 ELSE 0 END, props = ?5,
                     tier = ?6, priority = ?7, salience = MAX(salience, ?8), confidence = ?9,
                     authority = ?10, valid_from = ?11, valid_to = ?12, as_of = ?13,
                     archived_at = NULL
                 WHERE id = ?1",
            )?
            .execute(params![
                old.id,
                node.label.as_str(),
                untrusted,
                indexed,
                props,
                old.tier.max(node.tier).as_str(),
                old.priority.max(node.priority),
                f64::from(node.salience),
                f64::from(node.confidence),
                node.authority.as_str(),
                node.valid_from,
                node.valid_to,
                old.as_of.max(node.as_of),
            ])?;
            fts_delete(conn, old.id)?;
            old.id
        }
        None => conn
            .prepare_cached(
                "INSERT INTO gyrus_nodes (kind, key, label, untrusted, indexed, props, tier,
                     priority, salience, confidence, authority, valid_from, valid_to, as_of,
                     created)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                 RETURNING id",
            )?
            .query_row(
                params![
                    node.kind,
                    node.key,
                    node.label.as_str(),
                    untrusted,
                    indexed,
                    props,
                    node.tier.as_str(),
                    node.priority,
                    f64::from(node.salience),
                    f64::from(node.confidence),
                    node.authority.as_str(),
                    node.valid_from,
                    node.valid_to,
                    node.as_of,
                    now,
                ],
                |row| row.get(0),
            )?,
    };
    if indexed {
        fts_insert(conn, id, node.label.as_str())?;
    }
    let have: i64 = conn
        .prepare_cached("SELECT COUNT(*) FROM gyrus_aliases WHERE node = ?1")?
        .query_row([id], |row| row.get(0))?;
    let mut room = i64::try_from(ALIASES_MAX).unwrap_or(16) - have;
    for alias in &node.aliases {
        if room <= 0 {
            break;
        }
        room -= i64::try_from(
            conn.prepare_cached(
                "INSERT OR IGNORE INTO gyrus_aliases (alias, node) VALUES (?1, ?2)",
            )?
            .execute(params![normalise(alias), id])?,
        )
        .unwrap_or(0);
    }
    if let Some(v) = &node.vector {
        put_vector(conn, id, v)?;
    }
    Ok(id)
}

fn kind_of(conn: &Connection, id: NodeId) -> Result<String> {
    Ok(conn
        .prepare_cached("SELECT kind FROM gyrus_nodes WHERE id = ?1")?
        .query_row([id], |row| row.get(0))?)
}

fn link(conn: &Connection, pack: &Pack, link: &NewLink, now: i64) -> Result<LinkId> {
    let (Some(src), Some(dst)) = (id_of(conn, &link.src)?, id_of(conn, &link.dst)?) else {
        return refuse(Refusal::NoSuchNode);
    };
    let l = pack
        .link_index(&link.kind)
        .ok_or(Error::Refused(Refusal::UndeclaredLink))?;
    let (sk, dk) = (kind_of(conn, src)?, kind_of(conn, dst)?);
    let (Some(sk), Some(dk)) = (pack.kind_index(&sk), pack.kind_index(&dk)) else {
        return refuse(Refusal::UndeclaredKind);
    };
    if !pack.allows(l, sk, dk) {
        return refuse(Refusal::UndeclaredLink);
    }
    if let Some(cap) = pack.cap(sk, l) {
        let have: i64 = conn
            .prepare_cached(
                "SELECT COUNT(*) FROM gyrus_links WHERE src = ?1 AND kind = ?2 AND dst != ?3",
            )?
            .query_row(params![src, link.kind, dst], |row| row.get(0))?;
        if have >= i64::from(cap) {
            return refuse(Refusal::TooManyLinks);
        }
    }
    let props = serde_json::to_string(&link.props).map_err(|_| Error::Refused(Refusal::Bounds))?;
    Ok(conn
        .prepare_cached(
            "INSERT INTO gyrus_links (src, dst, kind, weight, confidence, props, valid_from,
                 valid_to, provenance, created)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT (src, dst, kind) DO UPDATE SET weight = excluded.weight,
                 confidence = excluded.confidence, props = excluded.props,
                 valid_from = excluded.valid_from, valid_to = excluded.valid_to,
                 provenance = excluded.provenance
             RETURNING id",
        )?
        .query_row(
            params![
                src,
                dst,
                link.kind,
                f64::from(link.weight),
                f64::from(link.confidence),
                props,
                link.valid_from,
                link.valid_to,
                link.provenance,
                now,
            ],
            |row| row.get(0),
        )?)
}

fn links_of(conn: &Connection, id: NodeId) -> Result<Vec<LinkRow>> {
    Ok(conn
        .prepare_cached(&format!(
            "SELECT {LINK_COLUMNS} FROM gyrus_links WHERE src = ?1
             UNION ALL SELECT {LINK_COLUMNS} FROM gyrus_links WHERE dst = ?1 AND src != ?1"
        ))?
        .query_map([id], link_row)?
        .collect::<rusqlite::Result<_>>()?)
}

/// Removes one node and everything that points at it (links both ways,
/// aliases, vector, keyword entry). Returns the links removed.
fn remove_node(conn: &Connection, id: NodeId) -> Result<Vec<LinkId>> {
    let links: Vec<LinkId> = links_of(conn, id)?.into_iter().map(|l| l.id).collect();
    conn.prepare_cached("DELETE FROM gyrus_links WHERE src = ?1 OR dst = ?1")?
        .execute([id])?;
    conn.prepare_cached("DELETE FROM gyrus_aliases WHERE node = ?1")?
        .execute([id])?;
    drop_vector(conn, id)?;
    fts_delete(conn, id)?;
    conn.prepare_cached("DELETE FROM gyrus_nodes WHERE id = ?1")?
        .execute([id])?;
    Ok(links)
}

fn merge(conn: &Connection, from: NodeId, into: NodeId, out: &mut Pending) -> Result<()> {
    for row in links_of(conn, from)? {
        let (src, dst) = (
            if row.src == from { into } else { row.src },
            if row.dst == from { into } else { row.dst },
        );
        if src != dst {
            let id: Option<LinkId> = conn
                .prepare_cached(
                    "INSERT INTO gyrus_links (src, dst, kind, weight, confidence, props,
                         valid_from, valid_to, provenance, created)
                     SELECT ?2, ?3, kind, weight, confidence, props, valid_from, valid_to,
                            provenance, created FROM gyrus_links WHERE id = ?1
                     ON CONFLICT (src, dst, kind) DO UPDATE SET
                         weight = MAX(gyrus_links.weight, excluded.weight)
                     RETURNING id",
                )?
                .query_row(params![row.id, src, dst], |r| r.get(0))
                .optional()?;
            if let Some(id) = id
                && let Some(moved) = link_by_id(conn, id)?
            {
                out.links.push(moved);
            }
        }
    }
    conn.prepare_cached(
        "INSERT OR IGNORE INTO gyrus_aliases (alias, node)
         SELECT alias, ?2 FROM gyrus_aliases WHERE node = ?1",
    )?
    .execute(params![from, into])?;
    conn.prepare_cached(
        "UPDATE gyrus_nodes SET use_count = use_count + (SELECT use_count FROM gyrus_nodes
             WHERE id = ?1),
             last_used = MAX(COALESCE(last_used, 0),
                             COALESCE((SELECT last_used FROM gyrus_nodes WHERE id = ?1), 0))
         WHERE id = ?2",
    )?
    .execute(params![from, into])?;
    out.unlinked.extend(remove_node(conn, from)?);
    out.removed.push(from);
    Ok(())
}

/// Applies `batch` inside the caller's transaction (all or nothing: an
/// error leaves the transaction for the caller to roll back), bumps the
/// write generation, and returns the change set.
///
/// # Errors
///
/// [`Error::Refused`] for a write the pack doesn't declare or the bounds
/// don't allow; SQLite errors.
#[expect(clippy::too_many_lines, reason = "one arm per write kind, in order")]
pub fn stage(conn: &Connection, pack: &Pack, batch: &Batch, now: i64) -> Result<Pending> {
    check(pack, batch)?;
    let mut out = Pending::default();
    let mut nodes: BTreeSet<NodeId> = BTreeSet::new();
    let mut links: BTreeSet<LinkId> = BTreeSet::new();
    let need = |conn: &Connection, key: &str| -> Result<NodeId> {
        id_of(conn, key)?.ok_or(Error::Refused(Refusal::NoSuchNode))
    };
    for op in &batch.ops {
        match op {
            Op::Upsert(node) => {
                nodes.insert(upsert(conn, pack, node, now)?);
            }
            Op::Link(new) => {
                links.insert(link(conn, pack, new, now)?);
            }
            Op::Unlink { src, kind, dst } => {
                let (src, dst) = (need(conn, src)?, need(conn, dst)?);
                let gone: Option<LinkId> = conn
                    .prepare_cached(
                        "DELETE FROM gyrus_links WHERE src = ?1 AND dst = ?2 AND kind = ?3
                         RETURNING id",
                    )?
                    .query_row(params![src, dst, kind], |row| row.get(0))
                    .optional()?;
                if let Some(id) = gone {
                    links.remove(&id);
                    out.unlinked.push(id);
                }
            }
            Op::Close { key, at } => {
                let id = need(conn, key)?;
                conn.prepare_cached("UPDATE gyrus_nodes SET valid_to = ?2 WHERE id = ?1")?
                    .execute(params![id, at])?;
                nodes.insert(id);
            }
            Op::Archive { key } => {
                let id = need(conn, key)?;
                conn.prepare_cached(
                    "UPDATE gyrus_nodes SET archived_at = ?2 WHERE id = ?1 AND archived_at IS NULL",
                )?
                .execute(params![id, now])?;
                nodes.insert(id);
            }
            Op::Fold { child, parent } => {
                let (c, p) = (need(conn, child)?, need(conn, parent)?);
                conn.prepare_cached(
                    "UPDATE gyrus_nodes SET archived_at = COALESCE(archived_at, ?2) WHERE id = ?1",
                )?
                .execute(params![c, now])?;
                conn.prepare_cached(
                    "UPDATE gyrus_nodes SET props = json_set(props, '$.folded',
                         COALESCE(json_extract(props, '$.folded'), 0) + 1) WHERE id = ?1",
                )?
                .execute([p])?;
                nodes.insert(c);
                nodes.insert(p);
            }
            Op::Restore { key } => {
                let id = need(conn, key)?;
                conn.prepare_cached("UPDATE gyrus_nodes SET archived_at = NULL WHERE id = ?1")?
                    .execute([id])?;
                nodes.insert(id);
            }
            Op::Maintain {
                key,
                tier,
                priority,
                salience,
            } => {
                let id = need(conn, key)?;
                conn.prepare_cached(
                    "UPDATE gyrus_nodes SET tier = ?2, priority = ?3, salience = ?4 WHERE id = ?1",
                )?
                .execute(params![
                    id,
                    tier.as_str(),
                    priority,
                    f64::from(*salience)
                ])?;
                nodes.insert(id);
            }
            Op::Merge { from, into } => {
                let (f, i) = (need(conn, from)?, need(conn, into)?);
                if kind_of(conn, f)? != kind_of(conn, i)? {
                    return refuse(Refusal::Key);
                }
                let mut merged = Pending::default();
                merge(conn, f, i, &mut merged)?;
                nodes.remove(&f);
                for gone in &merged.unlinked {
                    links.remove(gone);
                }
                for moved in &merged.links {
                    links.insert(moved.id);
                }
                merged.links.clear();
                nodes.insert(i);
                out.merge(merged);
            }
        }
    }
    for id in nodes {
        if let Some(node) = node_by_id(conn, id)? {
            out.nodes.push(node);
        }
    }
    for id in links {
        if let Some(row) = link_by_id(conn, id)? {
            out.links.push(row);
        }
    }
    out.generation = bump(conn)?;
    Ok(out)
}

/// Applies queued uses inside the caller's transaction: `use_count + n`,
/// `last_used = now`, and an archived node is brought back (a later use
/// brings it back).
///
/// # Errors
///
/// SQLite errors.
pub fn touch_in(conn: &Connection, uses: &[(NodeId, u32)], now: i64) -> Result<Pending> {
    let mut out = Pending::default();
    for (id, n) in uses {
        let changed = conn
            .prepare_cached(
                "UPDATE gyrus_nodes SET use_count = use_count + ?2, last_used = ?3,
                     archived_at = NULL WHERE id = ?1",
            )?
            .execute(params![id, n, now])?;
        if changed > 0
            && let Some(node) = node_by_id(conn, *id)?
        {
            out.nodes.push(node);
        }
    }
    out.generation = bump(conn)?;
    Ok(out)
}

/// What a forget removed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Erased {
    /// Nodes removed.
    pub nodes: usize,
    /// Links removed.
    pub links: usize,
    /// Keyword buckets rebuilt.
    pub buckets: usize,
}

/// Forgets the nodes with these keys inside the caller's transaction: each
/// node, its links both ways, its aliases, its vector and its keyword
/// entry; then every keyword bucket that held one is rebuilt from the
/// rows left, so no page separator keeps a prefix of a forgotten word.
///
/// The caller runs it with `PRAGMA secure_delete` on (so SQLite zeroes
/// what it frees), commits, truncates the WAL, then publishes the
/// [`Pending`] ([`crate::Graph::forget`] does all of it for a database of
/// gyrus's own).
///
/// # Errors
///
/// SQLite errors (nothing is removed unless all of it is: the caller
/// rolls back).
pub fn forget_in(conn: &Connection, keys: &[&str]) -> Result<(Pending, Erased)> {
    let mut out = Pending::default();
    let mut erased = Erased::default();
    let mut buckets = BTreeSet::new();
    for key in keys {
        let Some((id, indexed)) = conn
            .prepare_cached("SELECT id, indexed FROM gyrus_nodes WHERE key = ?1")?
            .query_row([key], |row| {
                Ok((row.get::<_, NodeId>(0)?, row.get::<_, i64>(1)?))
            })
            .optional()?
        else {
            continue;
        };
        let links = remove_node(conn, id)?;
        erased.nodes += 1;
        erased.links += links.len();
        // Only a bucket that ever held its words is rebuilt (`indexed` is
        // 1 while it is in the index, 2 once it was and no longer is).
        if indexed != 0 {
            buckets.insert(bucket_of(id));
        }
        out.unlinked.extend(links);
        out.removed.push(id);
    }
    for bucket in &buckets {
        rebuild_bucket(conn, *bucket)?;
    }
    erased.buckets = buckets.len();
    out.generation = bump(conn)?;
    Ok((out, erased))
}

/// Rebuilds one keyword bucket from the indexed rows left.
///
/// # Errors
///
/// SQLite errors.
pub fn rebuild_bucket(conn: &Connection, bucket: i64) -> Result<()> {
    let table = fts_table(bucket);
    conn.execute_batch(&format!(
        "INSERT INTO {table}({table}) VALUES ('delete-all');
         INSERT INTO {table}(rowid, label)
             SELECT id, label FROM gyrus_nodes
             WHERE indexed = 1 AND (id % {n}) = {bucket};",
        n = crate::schema::FTS_BUCKETS,
    ))?;
    Ok(())
}

/// Empties every gyrus table and the keyword index (a copy that must hold
/// no memory). The caller runs it with `secure_delete` on.
///
/// # Errors
///
/// SQLite errors.
pub fn clear_in(conn: &Connection) -> Result<usize> {
    conn.execute("DELETE FROM gyrus_links", [])?;
    conn.execute("DELETE FROM gyrus_aliases", [])?;
    conn.execute("DELETE FROM gyrus_vectors", [])?;
    let removed = conn.execute("DELETE FROM gyrus_nodes", [])?;
    for bucket in 0..crate::schema::FTS_BUCKETS {
        let table = fts_table(bucket);
        conn.execute_batch(&format!(
            "INSERT INTO {table}({table}) VALUES ('delete-all');"
        ))?;
    }
    bump(conn)?;
    Ok(removed)
}

/// Everything stored, for a rebuild of the index: the generation, every
/// node (with its aliases) and every link, read in one read transaction
/// (a savepoint), so the three agree.
///
/// # Errors
///
/// SQLite errors.
pub fn load(conn: &Connection) -> Result<(i64, Vec<NodeRow>, Vec<LinkRow>)> {
    conn.execute_batch("SAVEPOINT gyrus_load")?;
    let read = (|| -> Result<(i64, Vec<NodeRow>, Vec<LinkRow>)> {
        let generation = generation(conn)?;
        let mut nodes: Vec<NodeRow> = conn
            .prepare(&format!(
                "SELECT {NODE_COLUMNS} FROM gyrus_nodes ORDER BY id"
            ))?
            .query_map([], node_row)?
            .collect::<rusqlite::Result<_>>()?;
        let mut aliases: HashMap<NodeId, Vec<String>> = HashMap::new();
        {
            let mut stmt = conn.prepare("SELECT node, alias FROM gyrus_aliases ORDER BY alias")?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            for row in rows {
                let (node, alias): (NodeId, String) = row?;
                aliases.entry(node).or_default().push(alias);
            }
        }
        for node in &mut nodes {
            if let Some(list) = aliases.remove(&node.id) {
                node.aliases = list;
            }
        }
        let links = conn
            .prepare(&format!(
                "SELECT {LINK_COLUMNS} FROM gyrus_links ORDER BY id"
            ))?
            .query_map([], link_row)?
            .collect::<rusqlite::Result<_>>()?;
        Ok((generation, nodes, links))
    })();
    conn.execute_batch("RELEASE gyrus_load")?;
    read
}
