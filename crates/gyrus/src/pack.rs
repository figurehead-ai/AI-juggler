//! Kind packs: the node kinds, the link kinds and their allowed ends, and
//! each kind's rules (how it is walked, how it decays and moves between
//! tiers, and the reasoning questions it brings into a context), as data.
//! A host writes its own pack; the store refuses any kind or link it
//! doesn't declare.

use std::collections::HashMap;

use crate::error::{Error, Result};

/// Which way a walk follows a link.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dir {
    /// From source to target.
    Out,
    /// From target to source.
    In,
    /// Either way.
    Both,
}

/// One link a kind's walk follows.
#[derive(Clone, Debug, PartialEq)]
pub struct WalkRule {
    /// The link kind.
    pub link: String,
    /// Which way.
    pub dir: Dir,
    /// How far from the seed this rule still applies (1 = only one step).
    pub depth: u8,
    /// The best K kept at this step from one node (never an arbitrary K).
    pub keep: u16,
    /// The path decay of the step, 0..=1.
    pub decay: f32,
}

impl WalkRule {
    /// Follows `link` outwards, `keep` best per step, up to `depth`.
    #[must_use]
    pub fn out(link: &str, depth: u8, keep: u16) -> Self {
        Self::new(link, Dir::Out, depth, keep)
    }

    /// Follows `link` inwards (from target to source).
    #[must_use]
    pub fn inward(link: &str, depth: u8, keep: u16) -> Self {
        Self::new(link, Dir::In, depth, keep)
    }

    /// Follows `link` either way.
    #[must_use]
    pub fn both(link: &str, depth: u8, keep: u16) -> Self {
        Self::new(link, Dir::Both, depth, keep)
    }

    fn new(link: &str, dir: Dir, depth: u8, keep: u16) -> Self {
        Self {
            link: link.to_owned(),
            dir,
            depth,
            keep,
            decay: 0.7,
        }
    }

    /// With this step's decay.
    #[must_use]
    pub const fn decay(mut self, decay: f32) -> Self {
        self.decay = decay;
        self
    }
}

/// How a kind moves between tiers in the idle pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TierRules {
    /// Short → mid after this many uses.
    pub promote_short_after: u32,
    /// Mid → long after this many uses.
    pub promote_mid_after: u32,
    /// Idle this many days: priority decays one step (never below low;
    /// pinned never decays), and a long node goes to mid.
    pub demote_after_days: f64,
    /// Idle this many days at low priority: archived (kept, out of walks).
    /// `None`: never archived by the idle pass.
    pub archive_after_days: Option<f64>,
}

impl Default for TierRules {
    fn default() -> Self {
        Self {
            promote_short_after: 2,
            promote_mid_after: 5,
            demote_after_days: 30.0,
            archive_after_days: Some(60.0),
        }
    }
}

/// One node kind's rules.
#[derive(Clone, Debug, PartialEq)]
pub struct KindSpec {
    /// Its name.
    pub name: String,
    /// Recency half-life, days.
    pub half_life_days: f64,
    /// The links a walk follows from it.
    pub walk: Vec<WalkRule>,
    /// Short texts included with the node when it enters a context.
    pub questions: Vec<String>,
    /// The most links of a kind a node of this kind may have as source.
    pub max_links: Vec<(String, u32)>,
    /// Tier movement.
    pub tiers: TierRules,
    /// Near-duplicates of this kind may be merged by the idle pass (when
    /// both are trusted and neither rests on an agent's words).
    pub mergeable: bool,
    /// Its label is put in the keyword index (trusted labels only).
    pub searchable: bool,
}

impl KindSpec {
    /// A kind named `name`: half-life 30 days, searchable, no walk rules.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            half_life_days: 30.0,
            walk: Vec::new(),
            questions: Vec::new(),
            max_links: Vec::new(),
            tiers: TierRules::default(),
            mergeable: false,
            searchable: true,
        }
    }

    /// With a half-life.
    #[must_use]
    pub const fn half_life(mut self, days: f64) -> Self {
        self.half_life_days = days;
        self
    }

    /// With a walk rule.
    #[must_use]
    pub fn walk(mut self, rule: WalkRule) -> Self {
        self.walk.push(rule);
        self
    }

    /// With a reasoning question.
    #[must_use]
    pub fn question(mut self, text: &str) -> Self {
        self.questions.push(text.to_owned());
        self
    }

    /// At most `n` links of `link` from one node.
    #[must_use]
    pub fn max_links(mut self, link: &str, n: u32) -> Self {
        self.max_links.push((link.to_owned(), n));
        self
    }

    /// With tier rules.
    #[must_use]
    pub const fn tiers(mut self, tiers: TierRules) -> Self {
        self.tiers = tiers;
        self
    }

    /// Mergeable by the idle pass.
    #[must_use]
    pub const fn mergeable(mut self) -> Self {
        self.mergeable = true;
        self
    }

    /// Kept out of the keyword index (a pointer whose name lives elsewhere).
    #[must_use]
    pub const fn not_searchable(mut self) -> Self {
        self.searchable = false;
        self
    }
}

/// A link kind and its allowed ends.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkSpec {
    /// Its name.
    pub name: String,
    /// The source kinds allowed (empty: any).
    pub from: Vec<String>,
    /// The target kinds allowed (empty: any).
    pub to: Vec<String>,
}

impl LinkSpec {
    /// `name` from any of `from` to any of `to` (an empty list: any kind).
    #[must_use]
    pub fn new(name: &str, from: &[&str], to: &[&str]) -> Self {
        Self {
            name: name.to_owned(),
            from: from.iter().map(|s| (*s).to_owned()).collect(),
            to: to.iter().map(|s| (*s).to_owned()).collect(),
        }
    }
}

/// The idle pass's fold: children of `child` kind linked by `link` to a
/// `parent` and idle `after_days` are archived into the parent (its
/// `folded` count goes up).
#[derive(Clone, Debug, PartialEq)]
pub struct Fold {
    /// The child kind (a turn summary).
    pub child: String,
    /// The link from child to parent.
    pub link: String,
    /// The parent kind (a topic).
    pub parent: String,
    /// Idle days before a child folds.
    pub after_days: f64,
}

/// What the pack names for the generic machinery.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Roles {
    /// The kind of a rule node (it overrides a kind's walk for one scope).
    pub rule: Option<String>,
    /// The link from a rule to the node it governs.
    pub governs: Option<String>,
    /// The link the idle pass proposes between near-duplicates.
    pub same_as: Option<String>,
}

/// A host's kinds, links and rules, compiled.
#[derive(Clone, Debug)]
pub struct Pack {
    kinds: Vec<KindSpec>,
    links: Vec<LinkSpec>,
    kind_ix: HashMap<String, u16>,
    link_ix: HashMap<String, u16>,
    /// Per kind, its walk rules with the link as an index.
    walks: Vec<Vec<(u16, WalkRule)>>,
    /// Per link, the allowed source and target kinds (empty: any).
    ends: Vec<(Vec<u16>, Vec<u16>)>,
    /// Per kind, per link index, the most links.
    caps: Vec<Vec<(u16, u32)>>,
    roles: Roles,
    rule_kind: Option<u16>,
    governs: Option<u16>,
    same_as: Option<u16>,
    folds: Vec<(u16, u16, u16, f64)>,
}

/// Builds a [`Pack`].
#[derive(Clone, Debug, Default)]
pub struct PackBuilder {
    kinds: Vec<KindSpec>,
    links: Vec<LinkSpec>,
    roles: Roles,
    folds: Vec<Fold>,
}

impl PackBuilder {
    /// Declares a kind.
    #[must_use]
    pub fn kind(mut self, spec: KindSpec) -> Self {
        self.kinds.push(spec);
        self
    }

    /// Declares a link kind.
    #[must_use]
    pub fn link(mut self, spec: LinkSpec) -> Self {
        self.links.push(spec);
        self
    }

    /// Names the rule kind and its `governs` link.
    #[must_use]
    pub fn rules(mut self, rule_kind: &str, governs: &str) -> Self {
        self.roles.rule = Some(rule_kind.to_owned());
        self.roles.governs = Some(governs.to_owned());
        self
    }

    /// Names the link the idle pass proposes between near-duplicates.
    #[must_use]
    pub fn same_as(mut self, link: &str) -> Self {
        self.roles.same_as = Some(link.to_owned());
        self
    }

    /// Adds a fold.
    #[must_use]
    pub fn fold(mut self, fold: Fold) -> Self {
        self.folds.push(fold);
        self
    }

    /// Checks and compiles the pack: names unique, every walk rule's,
    /// cap's, role's and fold's names declared.
    ///
    /// # Errors
    ///
    /// [`Error::Pack`] naming what is wrong.
    pub fn build(self) -> Result<Pack> {
        let bad = |what: String| Err(Error::Pack(what));
        let mut kind_ix = HashMap::new();
        for (i, kind) in self.kinds.iter().enumerate() {
            let Ok(i) = u16::try_from(i) else {
                return bad("too many kinds".into());
            };
            if kind.name.is_empty() || kind_ix.insert(kind.name.clone(), i).is_some() {
                return bad(format!("kind {} declared twice or empty", kind.name));
            }
            if !kind.half_life_days.is_finite() || kind.half_life_days <= 0.0 {
                return bad(format!("kind {} has no half-life", kind.name));
            }
        }
        let mut link_ix = HashMap::new();
        for (i, link) in self.links.iter().enumerate() {
            let Ok(i) = u16::try_from(i) else {
                return bad("too many links".into());
            };
            if link.name.is_empty() || link_ix.insert(link.name.clone(), i).is_some() {
                return bad(format!("link {} declared twice or empty", link.name));
            }
        }
        let kind_of = |name: &str| kind_ix.get(name).copied();
        let link_of = |name: &str| link_ix.get(name).copied();
        let mut ends = Vec::new();
        for link in &self.links {
            let mut sides = (Vec::new(), Vec::new());
            for (names, side) in [(&link.from, &mut sides.0), (&link.to, &mut sides.1)] {
                for name in names {
                    match kind_of(name) {
                        Some(k) => side.push(k),
                        None => return bad(format!("link {} names kind {name}", link.name)),
                    }
                }
            }
            ends.push(sides);
        }
        let mut walks = Vec::new();
        let mut caps = Vec::new();
        for kind in &self.kinds {
            let mut rules = Vec::new();
            for rule in &kind.walk {
                let Some(l) = link_of(&rule.link) else {
                    return bad(format!("kind {} walks link {}", kind.name, rule.link));
                };
                if rule.keep == 0 || rule.depth == 0 || !(0.0..=1.0).contains(&rule.decay) {
                    return bad(format!(
                        "kind {}'s walk of {} keeps nothing",
                        kind.name, rule.link
                    ));
                }
                rules.push((l, rule.clone()));
            }
            walks.push(rules);
            let mut kind_caps = Vec::new();
            for (link, n) in &kind.max_links {
                let Some(l) = link_of(link) else {
                    return bad(format!("kind {} caps link {link}", kind.name));
                };
                kind_caps.push((l, *n));
            }
            caps.push(kind_caps);
        }
        let role =
            |name: &Option<String>, of: &dyn Fn(&str) -> Option<u16>| -> Result<Option<u16>> {
                name.as_deref().map_or(Ok(None), |n| {
                    of(n)
                        .map(Some)
                        .ok_or_else(|| Error::Pack(format!("role names {n}")))
                })
            };
        let rule_kind = role(&self.roles.rule, &kind_of)?;
        let governs = role(&self.roles.governs, &link_of)?;
        let same_as = role(&self.roles.same_as, &link_of)?;
        let mut folds = Vec::new();
        for fold in &self.folds {
            match (
                kind_of(&fold.child),
                link_of(&fold.link),
                kind_of(&fold.parent),
            ) {
                (Some(c), Some(l), Some(p)) => folds.push((c, l, p, fold.after_days)),
                _ => return bad(format!("fold of {} names something undeclared", fold.child)),
            }
        }
        Ok(Pack {
            kinds: self.kinds,
            links: self.links,
            kind_ix,
            link_ix,
            walks,
            ends,
            caps,
            roles: self.roles,
            rule_kind,
            governs,
            same_as,
            folds,
        })
    }
}

impl Pack {
    /// A new builder.
    #[must_use]
    pub fn builder() -> PackBuilder {
        PackBuilder::default()
    }

    /// A kind's index.
    #[must_use]
    pub fn kind_index(&self, name: &str) -> Option<u16> {
        self.kind_ix.get(name).copied()
    }

    /// A link's index.
    #[must_use]
    pub fn link_index(&self, name: &str) -> Option<u16> {
        self.link_ix.get(name).copied()
    }

    /// A kind's spec by index.
    #[must_use]
    pub fn kind(&self, ix: u16) -> Option<&KindSpec> {
        self.kinds.get(usize::from(ix))
    }

    /// A link's spec by index.
    #[must_use]
    pub fn link(&self, ix: u16) -> Option<&LinkSpec> {
        self.links.get(usize::from(ix))
    }

    /// Every kind.
    #[must_use]
    pub fn kinds(&self) -> &[KindSpec] {
        &self.kinds
    }

    /// Every link kind.
    #[must_use]
    pub fn links(&self) -> &[LinkSpec] {
        &self.links
    }

    /// The roles named.
    #[must_use]
    pub const fn roles(&self) -> &Roles {
        &self.roles
    }

    /// A kind's walk rules, each with its link's index.
    #[must_use]
    pub fn walks(&self, kind: u16) -> &[(u16, WalkRule)] {
        self.walks.get(usize::from(kind)).map_or(&[], Vec::as_slice)
    }

    /// The most links of `link` a node of `kind` may have as source.
    #[must_use]
    pub fn cap(&self, kind: u16, link: u16) -> Option<u32> {
        self.caps
            .get(usize::from(kind))?
            .iter()
            .find(|(l, _)| *l == link)
            .map(|(_, n)| *n)
    }

    /// The link may go from a `src` kind node to a `dst` kind node.
    #[must_use]
    pub fn allows(&self, link: u16, src: u16, dst: u16) -> bool {
        self.ends.get(usize::from(link)).is_some_and(|(from, to)| {
            (from.is_empty() || from.contains(&src)) && (to.is_empty() || to.contains(&dst))
        })
    }

    /// The rule kind's index, if the pack names one.
    #[must_use]
    pub const fn rule_kind(&self) -> Option<u16> {
        self.rule_kind
    }

    /// The `governs` link's index, if the pack names one.
    #[must_use]
    pub const fn governs(&self) -> Option<u16> {
        self.governs
    }

    /// The `same_as` link's index, if the pack names one.
    #[must_use]
    pub const fn same_as_link(&self) -> Option<u16> {
        self.same_as
    }

    /// The folds, as (child kind, link, parent kind, idle days) indexes.
    #[must_use]
    pub fn folds(&self) -> &[(u16, u16, u16, f64)] {
        &self.folds
    }

    /// A kind's name by index (`""` if unknown).
    #[must_use]
    pub fn kind_name(&self, ix: u16) -> &str {
        self.kind(ix).map_or("", |k| k.name.as_str())
    }

    /// A link's name by index (`""` if unknown).
    #[must_use]
    pub fn link_name(&self, ix: u16) -> &str {
        self.link(ix).map_or("", |l| l.name.as_str())
    }
}
