//! The data model: nodes, links, tiers, authority, and the writes a batch
//! holds.

use serde_json::Value;

/// A node's id: its row's `INTEGER PRIMARY KEY`, stable across `VACUUM`.
pub type NodeId = i64;
/// A link's id.
pub type LinkId = i64;

/// The longest node key, in bytes.
pub const KEY_MAX: usize = 256;
/// The longest label, in characters.
pub const LABEL_MAX: usize = 280;
/// The largest `props` of a node or link, in bytes of JSON.
pub const PROPS_MAX: usize = 16 * 1024;
/// The longest alias, in characters.
pub const ALIAS_MAX: usize = 80;
/// The most aliases one node keeps.
pub const ALIASES_MAX: usize = 16;
/// The largest vector, in dimensions.
pub const DIM_MAX: usize = 4096;

/// How long a node lives, and how it is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tier {
    /// The current request's working set: held in memory only, never
    /// stored (a write of an immediate node is refused).
    Immediate,
    /// New, not yet reused.
    Short,
    /// Reused.
    Mid,
    /// Kept until forgotten.
    Long,
}

impl Tier {
    /// The stored code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Immediate => "immediate",
            Self::Short => "short",
            Self::Mid => "mid",
            Self::Long => "long",
        }
    }

    /// From the stored code.
    #[must_use]
    pub fn parse(code: &str) -> Option<Self> {
        Some(match code {
            "immediate" => Self::Immediate,
            "short" => Self::Short,
            "mid" => Self::Mid,
            "long" => Self::Long,
            _ => return None,
        })
    }

    /// One tier up (long stays long).
    #[must_use]
    pub const fn up(self) -> Self {
        match self {
            Self::Immediate | Self::Short => Self::Mid,
            Self::Mid | Self::Long => Self::Long,
        }
    }

    /// One tier down (short stays short).
    #[must_use]
    pub const fn down(self) -> Self {
        match self {
            Self::Long => Self::Mid,
            Self::Mid | Self::Short | Self::Immediate => Self::Short,
        }
    }
}

/// Whose words or judgement a node rests on. A write may change a node
/// only with at least the node's authority (user > app > model > agent).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Authority {
    /// The person using the host.
    User,
    /// The host application's own code.
    App,
    /// A model, through the host's confidence gate.
    Model,
    /// Text another agent wrote: always untrusted, never a source for a
    /// merge or a rule.
    Agent,
}

impl Authority {
    /// The stored code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::App => "app",
            Self::Model => "model",
            Self::Agent => "agent",
        }
    }

    /// From the stored code.
    #[must_use]
    pub fn parse(code: &str) -> Option<Self> {
        Some(match code {
            "user" => Self::User,
            "app" => Self::App,
            "model" => Self::Model,
            "agent" => Self::Agent,
            _ => return None,
        })
    }

    /// Higher outranks lower.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::User => 3,
            Self::App => 2,
            Self::Model => 1,
            Self::Agent => 0,
        }
    }
}

/// A node's label, typed by whose words it holds. Untrusted labels are
/// stored but never put in the keyword index, and a walk skips untrusted
/// nodes unless it asks for them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Label {
    /// The user's words, or text the host or a gated model wrote.
    Trusted(String),
    /// Another agent's words.
    Untrusted(String),
}

impl Label {
    /// The words.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Trusted(text) | Self::Untrusted(text) => text,
        }
    }

    /// Another agent's words.
    #[must_use]
    pub const fn is_untrusted(&self) -> bool {
        matches!(self, Self::Untrusted(_))
    }

    /// No label (a pointer node: the host reads what it points at).
    #[must_use]
    pub const fn none() -> Self {
        Self::Trusted(String::new())
    }
}

/// A node to write. Its `key` is the canonical subject: one node per key,
/// the unit of merging and forgetting. Writing a key that exists updates
/// that node (the same key is the same node).
#[derive(Clone, Debug, PartialEq)]
pub struct NewNode {
    /// Its kind, declared by the pack.
    pub kind: String,
    /// The canonical key (a code: no spaces, at most [`KEY_MAX`] bytes).
    pub key: String,
    /// What it says (empty for a pointer).
    pub label: Label,
    /// Properties, a JSON object. A string `fact` names its fact key: a
    /// ranking keeps one node per fact key.
    pub props: Value,
    /// How long it lives (never [`Tier::Immediate`]).
    pub tier: Tier,
    /// 0 low, 1 normal, 2 high, 3 pinned.
    pub priority: u8,
    /// How much it stands out, 0..=1 (the idle pass recomputes it).
    pub salience: f32,
    /// How sure, 0..=1.
    pub confidence: f32,
    /// Whose words or judgement.
    pub authority: Authority,
    /// Valid from (epoch ms), `None` for always.
    pub valid_from: Option<i64>,
    /// Valid until (epoch ms, exclusive), `None` for open.
    pub valid_to: Option<i64>,
    /// When it was said or observed (epoch ms): recency is measured from
    /// the later of this and its last use.
    pub as_of: i64,
    /// Exact names it is found by (the user's own names for it).
    pub aliases: Vec<String>,
    /// An embedding, compared by exact cosine (stored on disk).
    pub vector: Option<Vec<f32>>,
}

impl NewNode {
    /// A trusted node of `kind` and `key`, short tier, normal priority,
    /// by the host, as of `as_of`.
    #[must_use]
    pub fn new(kind: &str, key: &str, label: Label, as_of: i64) -> Self {
        Self {
            kind: kind.to_owned(),
            key: key.to_owned(),
            label,
            props: Value::Object(serde_json::Map::new()),
            tier: Tier::Short,
            priority: 1,
            salience: 0.5,
            confidence: 1.0,
            authority: Authority::App,
            valid_from: None,
            valid_to: None,
            as_of,
            aliases: Vec::new(),
            vector: None,
        }
    }
}

/// A link to write, between two nodes named by key. The same source,
/// target and kind is the same link (written again, it is updated).
#[derive(Clone, Debug, PartialEq)]
pub struct NewLink {
    /// The source node's key.
    pub src: String,
    /// The target node's key.
    pub dst: String,
    /// Its kind, declared by the pack with its allowed ends.
    pub kind: String,
    /// How strong, 0..=1 (walks follow the strongest first).
    pub weight: f32,
    /// How sure, 0..=1.
    pub confidence: f32,
    /// Properties, a JSON object.
    pub props: Value,
    /// Valid from (epoch ms), `None` for always.
    pub valid_from: Option<i64>,
    /// Valid until (epoch ms, exclusive), `None` for open.
    pub valid_to: Option<i64>,
    /// Who or what wrote it, and from which turn or record (a code).
    pub provenance: String,
}

impl NewLink {
    /// A link of `kind` from `src` to `dst`, weight and confidence 1.
    #[must_use]
    pub fn new(src: &str, kind: &str, dst: &str, provenance: &str) -> Self {
        Self {
            src: src.to_owned(),
            dst: dst.to_owned(),
            kind: kind.to_owned(),
            weight: 1.0,
            confidence: 1.0,
            props: Value::Object(serde_json::Map::new()),
            valid_from: None,
            valid_to: None,
            provenance: provenance.to_owned(),
        }
    }
}

/// One write in a [`Batch`].
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// Writes a node (a new key, or an update of the node with that key).
    Upsert(NewNode),
    /// Writes a link.
    Link(NewLink),
    /// Removes a link.
    Unlink {
        /// The source's key.
        src: String,
        /// The link's kind.
        kind: String,
        /// The target's key.
        dst: String,
    },
    /// Ends a node's validity at `at` (it stays, out of reads after `at`).
    Close {
        /// The node's key.
        key: String,
        /// When it stopped being valid (epoch ms).
        at: i64,
    },
    /// Moves a node out of every walk (kept; a later use brings it back).
    Archive {
        /// The node's key.
        key: String,
    },
    /// Brings an archived node back.
    Restore {
        /// The node's key.
        key: String,
    },
    /// The maintenance columns, as the idle pass sets them.
    Maintain {
        /// The node's key.
        key: String,
        /// The new tier.
        tier: Tier,
        /// The new priority.
        priority: u8,
        /// The new salience.
        salience: f32,
    },
    /// Folds `from` into `into`: its links and aliases move to `into`,
    /// then `from` is deleted (securely, as a forget deletes).
    Merge {
        /// The duplicate's key.
        from: String,
        /// The node kept.
        into: String,
    },
}

/// The writes of one transaction, applied together or not at all.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Batch {
    /// In order.
    pub ops: Vec<Op>,
}

impl Batch {
    /// An empty batch.
    #[must_use]
    pub const fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// Adds a node.
    pub fn node(&mut self, node: NewNode) -> &mut Self {
        self.ops.push(Op::Upsert(node));
        self
    }

    /// Adds a link.
    pub fn link(&mut self, link: NewLink) -> &mut Self {
        self.ops.push(Op::Link(link));
        self
    }

    /// Adds any write.
    pub fn op(&mut self, op: Op) -> &mut Self {
        self.ops.push(op);
        self
    }

    /// Nothing to write.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Every label, alias and string in the props of the batch: what a
    /// host's secret check reads before anything is written.
    #[must_use]
    pub fn words(&self) -> Vec<String> {
        let mut out = Vec::new();
        for op in &self.ops {
            match op {
                Op::Upsert(node) => {
                    out.push(node.label.as_str().to_owned());
                    out.extend(node.aliases.iter().cloned());
                    strings(&node.props, &mut out);
                }
                Op::Link(link) => {
                    strings(&link.props, &mut out);
                    out.push(link.provenance.clone());
                }
                _ => {}
            }
        }
        out.retain(|word| !word.is_empty());
        out
    }
}

pub(crate) fn strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(text) => out.push(text.clone()),
        Value::Array(items) => items.iter().for_each(|item| strings(item, out)),
        Value::Object(map) => {
            for (key, item) in map {
                out.push(key.clone());
                strings(item, out);
            }
        }
        _ => {}
    }
}

/// A key is a code: printable, no whitespace, not empty, bounded.
#[must_use]
pub fn key_like(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= KEY_MAX
        && key.chars().all(|c| !c.is_whitespace() && !c.is_control())
}

/// An alias as it is matched: lower case, single spaces, trimmed.
#[must_use]
pub fn normalise(text: &str) -> String {
    text.split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}
