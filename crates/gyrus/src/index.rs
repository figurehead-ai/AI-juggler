//! The typed index kept in memory: per node its kind, tier and the numbers
//! ranking reads; links in both directions, grouped by link kind and
//! sorted by weight, so a hub is walked from either end; aliases and keys
//! for exact lookups. Built from SQLite ([`Snapshot::build`]) and changed
//! only by a committed change set ([`Snapshot::apply`]).

use std::collections::{HashMap, HashSet};

use crate::pack::Pack;
use crate::store::{LinkRow, NodeRow, Pending};
use crate::types::{Authority, LinkId, NodeId, Tier};

/// What the index keeps of a node (never its label: labels are read from
/// SQLite for the few nodes a context shows).
#[derive(Clone, Debug, PartialEq)]
pub struct NodeEntry {
    /// The kind's index in the pack.
    pub kind: u16,
    /// Its key.
    pub key: Box<str>,
    /// Its tier.
    pub tier: Tier,
    /// 0 low … 3 pinned.
    pub priority: u8,
    /// How much it stands out.
    pub salience: f32,
    /// How sure.
    pub confidence: f32,
    /// Whose.
    pub authority: Authority,
    /// Its label is another agent's words.
    pub untrusted: bool,
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
    /// Out of walks.
    pub archived: bool,
    /// Its fact key (a ranking keeps one node per fact key).
    pub fact: Option<Box<str>>,
    /// For a rule node: what it overrides.
    pub rule: Option<RuleOverride>,
    /// Folded children (the idle pass's count).
    pub folded: u32,
}

impl NodeEntry {
    /// Valid at `t` (always, when `t` is `None`).
    #[must_use]
    pub fn valid_at(&self, t: Option<i64>) -> bool {
        valid(self.valid_from, self.valid_to, t)
    }
}

pub(crate) fn valid(from: Option<i64>, to: Option<i64>, t: Option<i64>) -> bool {
    t.is_none_or(|t| from.is_none_or(|f| f <= t) && to.is_none_or(|e| t < e))
}

/// A rule node's override of a walk, for the nodes it governs: links not
/// to follow from them, and a smaller best-K.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleOverride {
    /// Link kinds (indexes) never followed from a governed node.
    pub block: Vec<u16>,
    /// At most this many kept per step from a governed node.
    pub keep: Option<u16>,
}

/// One end of a link, as seen from a node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edge {
    /// The link.
    pub id: LinkId,
    /// The node at the other end.
    pub other: NodeId,
    /// Its weight.
    pub weight: f32,
    /// Its confidence.
    pub confidence: f32,
    /// Valid from.
    pub valid_from: Option<i64>,
    /// Valid until.
    pub valid_to: Option<i64>,
}

impl Edge {
    /// Valid at `t`.
    #[must_use]
    pub fn valid_at(&self, t: Option<i64>) -> bool {
        valid(self.valid_from, self.valid_to, t)
    }
}

/// A node's links of one kind in one direction, strongest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Group {
    /// The link kind's index.
    pub link: u16,
    /// Strongest first.
    pub edges: Vec<Edge>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ends {
    src: NodeId,
    dst: NodeId,
    kind: u16,
}

/// One consistent view of the graph. Readers hold an `Arc` of it; a
/// committed batch makes the next one, so a walk never sees half a batch.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    generation: i64,
    nodes: HashMap<NodeId, NodeEntry>,
    by_key: HashMap<Box<str>, NodeId>,
    by_kind: HashMap<u16, HashSet<NodeId>>,
    aliases: HashMap<Box<str>, Vec<NodeId>>,
    alias_of: HashMap<NodeId, Vec<Box<str>>>,
    out: HashMap<NodeId, Vec<Group>>,
    inn: HashMap<NodeId, Vec<Group>>,
    links: HashMap<LinkId, Ends>,
    /// Per governed node, the rule nodes governing it.
    governed: HashMap<NodeId, Vec<NodeId>>,
}

fn entry(pack: &Pack, row: &NodeRow) -> Option<NodeEntry> {
    let kind = pack.kind_index(&row.kind)?;
    let fact = row
        .props
        .get("fact")
        .and_then(serde_json::Value::as_str)
        .map(Into::into);
    let rule = (pack.rule_kind() == Some(kind)).then(|| RuleOverride {
        block: row
            .props
            .get("block")
            .and_then(serde_json::Value::as_array)
            .map(|links| {
                links
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .filter_map(|l| pack.link_index(l))
                    .collect()
            })
            .unwrap_or_default(),
        keep: row
            .props
            .get("keep")
            .and_then(serde_json::Value::as_u64)
            .and_then(|k| u16::try_from(k).ok()),
    });
    let folded = row
        .props
        .get("folded")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0);
    Some(NodeEntry {
        kind,
        key: row.key.as_str().into(),
        tier: row.tier,
        priority: row.priority,
        salience: row.salience,
        confidence: row.confidence,
        authority: row.authority,
        untrusted: row.untrusted,
        valid_from: row.valid_from,
        valid_to: row.valid_to,
        as_of: row.as_of,
        last_used: row.last_used,
        use_count: row.use_count,
        archived: row.archived,
        fact,
        rule,
        folded,
    })
}

fn insert_edge(groups: &mut Vec<Group>, link: u16, edge: Edge) {
    let group = if let Some(at) = groups.iter().position(|g| g.link == link) {
        &mut groups[at]
    } else {
        groups.push(Group {
            link,
            edges: Vec::new(),
        });
        let last = groups.len() - 1;
        &mut groups[last]
    };
    // Strongest first; ties by id, so the order is the same every build.
    let at = group.edges.partition_point(|e| {
        e.weight
            .total_cmp(&edge.weight)
            .then(edge.id.cmp(&e.id))
            .is_gt()
    });
    group.edges.insert(at, edge);
}

fn remove_edge(groups: Option<&mut Vec<Group>>, link: u16, id: LinkId) {
    if let Some(groups) = groups
        && let Some(group) = groups.iter_mut().find(|g| g.link == link)
    {
        group.edges.retain(|e| e.id != id);
    }
}

impl Snapshot {
    /// Builds the index from stored rows (nodes or links of kinds the
    /// pack doesn't declare are left out).
    #[must_use]
    pub fn build(pack: &Pack, generation: i64, nodes: &[NodeRow], links: &[LinkRow]) -> Self {
        let mut snap = Self {
            generation,
            ..Self::default()
        };
        for row in nodes {
            snap.put_node(pack, row);
        }
        for row in links {
            snap.put_link(pack, row);
        }
        snap
    }

    /// The write generation this view includes.
    #[must_use]
    pub const fn generation(&self) -> i64 {
        self.generation
    }

    /// Applies a committed change set.
    pub fn apply(&mut self, pack: &Pack, pending: &Pending) {
        for id in &pending.unlinked {
            self.drop_link(*id);
        }
        for id in &pending.removed {
            self.drop_node(*id);
        }
        for row in &pending.nodes {
            self.put_node(pack, row);
        }
        for row in &pending.links {
            self.put_link(pack, row);
        }
        self.generation = self.generation.max(pending.generation);
    }

    fn put_node(&mut self, pack: &Pack, row: &NodeRow) {
        let Some(entry) = entry(pack, row) else {
            return;
        };
        if let Some(old) = self.nodes.get(&row.id)
            && old.kind != entry.kind
            && let Some(set) = self.by_kind.get_mut(&old.kind)
        {
            set.remove(&row.id);
        }
        for alias in self.alias_of.remove(&row.id).unwrap_or_default() {
            if let Some(ids) = self.aliases.get_mut(&alias) {
                ids.retain(|id| *id != row.id);
                if ids.is_empty() {
                    self.aliases.remove(&alias);
                }
            }
        }
        let aliases: Vec<Box<str>> = row.aliases.iter().map(|a| a.as_str().into()).collect();
        for alias in &aliases {
            self.aliases.entry(alias.clone()).or_default().push(row.id);
        }
        if !aliases.is_empty() {
            self.alias_of.insert(row.id, aliases);
        }
        self.by_key.insert(entry.key.clone(), row.id);
        self.by_kind.entry(entry.kind).or_default().insert(row.id);
        self.nodes.insert(row.id, entry);
    }

    fn put_link(&mut self, pack: &Pack, row: &LinkRow) {
        let Some(kind) = pack.link_index(&row.kind) else {
            return;
        };
        if self.links.contains_key(&row.id) {
            self.drop_link(row.id);
        }
        if !self.nodes.contains_key(&row.src) || !self.nodes.contains_key(&row.dst) {
            return;
        }
        let edge = |other| Edge {
            id: row.id,
            other,
            weight: row.weight,
            confidence: row.confidence,
            valid_from: row.valid_from,
            valid_to: row.valid_to,
        };
        insert_edge(self.out.entry(row.src).or_default(), kind, edge(row.dst));
        insert_edge(self.inn.entry(row.dst).or_default(), kind, edge(row.src));
        self.links.insert(
            row.id,
            Ends {
                src: row.src,
                dst: row.dst,
                kind,
            },
        );
        if pack.governs() == Some(kind) {
            self.governed.entry(row.dst).or_default().push(row.src);
        }
    }

    fn drop_link(&mut self, id: LinkId) {
        let Some(ends) = self.links.remove(&id) else {
            return;
        };
        remove_edge(self.out.get_mut(&ends.src), ends.kind, id);
        remove_edge(self.inn.get_mut(&ends.dst), ends.kind, id);
        if let Some(rules) = self.governed.get_mut(&ends.dst) {
            rules.retain(|r| *r != ends.src);
        }
    }

    fn drop_node(&mut self, id: NodeId) {
        let ids: Vec<LinkId> = self
            .out
            .get(&id)
            .into_iter()
            .chain(self.inn.get(&id))
            .flat_map(|groups| groups.iter().flat_map(|g| g.edges.iter().map(|e| e.id)))
            .collect();
        for link in ids {
            self.drop_link(link);
        }
        self.out.remove(&id);
        self.inn.remove(&id);
        self.governed.remove(&id);
        if let Some(entry) = self.nodes.remove(&id) {
            self.by_key.remove(&entry.key);
            if let Some(set) = self.by_kind.get_mut(&entry.kind) {
                set.remove(&id);
            }
        }
        for alias in self.alias_of.remove(&id).unwrap_or_default() {
            if let Some(ids) = self.aliases.get_mut(&alias) {
                ids.retain(|n| *n != id);
                if ids.is_empty() {
                    self.aliases.remove(&alias);
                }
            }
        }
    }

    /// A node by id.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&NodeEntry> {
        self.nodes.get(&id)
    }

    /// A node's id by key.
    #[must_use]
    pub fn id_of(&self, key: &str) -> Option<NodeId> {
        self.by_key.get(key).copied()
    }

    /// The nodes an alias names (normalised first).
    #[must_use]
    pub fn by_alias(&self, alias: &str) -> &[NodeId] {
        self.aliases
            .get(crate::types::normalise(alias).as_str())
            .map_or(&[], Vec::as_slice)
    }

    /// Every alias held (normalised), for matching phrases in a text.
    pub fn aliases(&self) -> impl Iterator<Item = (&str, &[NodeId])> {
        self.aliases.iter().map(|(a, ids)| (&**a, ids.as_slice()))
    }

    /// Every node of a kind.
    pub fn of_kind(&self, kind: u16) -> impl Iterator<Item = NodeId> + '_ {
        self.by_kind.get(&kind).into_iter().flatten().copied()
    }

    /// A node's links of `link` kind, outwards, strongest first.
    #[must_use]
    pub fn out(&self, id: NodeId, link: u16) -> &[Edge] {
        groups_of(self.out.get(&id), link)
    }

    /// A node's links of `link` kind, inwards, strongest first.
    #[must_use]
    pub fn inward(&self, id: NodeId, link: u16) -> &[Edge] {
        groups_of(self.inn.get(&id), link)
    }

    /// Every link group of a node, outwards.
    #[must_use]
    pub fn out_groups(&self, id: NodeId) -> &[Group] {
        self.out.get(&id).map_or(&[], Vec::as_slice)
    }

    /// Every link group of a node, inwards.
    #[must_use]
    pub fn in_groups(&self, id: NodeId) -> &[Group] {
        self.inn.get(&id).map_or(&[], Vec::as_slice)
    }

    /// The rule nodes governing a node.
    #[must_use]
    pub fn rules_over(&self, id: NodeId) -> &[NodeId] {
        self.governed.get(&id).map_or(&[], Vec::as_slice)
    }

    /// How many nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// No nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// How many links.
    #[must_use]
    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    /// Every node id.
    pub fn ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }

    /// A canonical listing of what the index holds, for comparing two
    /// builds (the rebuild tests): nodes by key with their numbers, links
    /// by (src key, kind, dst key) with weight, aliases.
    #[must_use]
    pub fn fingerprint(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .nodes
            .values()
            .map(|n| {
                format!(
                    "n {} k{} {} p{} s{:.3} c{:.3} {} u{} a{} l{:?} f{:?}-{:?} x{}",
                    n.key,
                    n.kind,
                    n.tier.as_str(),
                    n.priority,
                    n.salience,
                    n.confidence,
                    n.authority.as_str(),
                    n.use_count,
                    n.as_of,
                    n.last_used,
                    n.valid_from,
                    n.valid_to,
                    n.archived
                )
            })
            .collect();
        for ends in self.links.values() {
            let key = |id| self.nodes.get(&id).map_or("?", |n| &*n.key);
            let weight = self
                .out
                .get(&ends.src)
                .and_then(|g| g.iter().find(|g| g.link == ends.kind))
                .and_then(|g| g.edges.iter().find(|e| e.other == ends.dst))
                .map_or(-1.0, |e| e.weight);
            out.push(format!(
                "l {} {} {} w{weight:.3}",
                key(ends.src),
                ends.kind,
                key(ends.dst)
            ));
        }
        for (alias, ids) in &self.aliases {
            let mut keys: Vec<&str> = ids
                .iter()
                .filter_map(|id| self.nodes.get(id).map(|n| &*n.key))
                .collect();
            keys.sort_unstable();
            out.push(format!("a {alias} {keys:?}"));
        }
        out.sort();
        out
    }

    /// Whether any key, alias or fact key the index holds contains
    /// `needle` (the forget tests scan the index with it).
    #[must_use]
    pub fn holds_text(&self, needle: &str) -> bool {
        self.nodes.values().any(|n| {
            n.key.contains(needle) || n.fact.as_deref().is_some_and(|f| f.contains(needle))
        }) || self.aliases.keys().any(|a| a.contains(needle))
    }
}

fn groups_of(groups: Option<&Vec<Group>>, link: u16) -> &[Edge] {
    groups
        .and_then(|g| g.iter().find(|g| g.link == link))
        .map_or(&[], |g| g.edges.as_slice())
}
