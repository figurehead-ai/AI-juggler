//! D-62 decisions 8–16 (AJ-E3004): which pools a session is in, how it got
//! into each, with what role and level, and why it isn't in the others.
//!
//! One resolver answers every "where is this session" question: the
//! pools page, a pool's members, the session's Pools tab, the agent tools
//! (`whoami`, and the pool a write names), the tasks and the counts.
//!
//! - **Scope.** A broad member (App, Account, Tool, Mac, everyone) brings
//!   in only its sessions whose project is in the pool's scope. A session
//!   named on its own was checked against the scope when it was added, so
//!   it stays (0.3's rows move over with nothing lost).
//! - **Inside one pool the most specific member wins**: Session › App ›
//!   Account › Tool › Mac › everyone. Its role and its own level (else the
//!   pool's) are the session's there. Two rows of one kind (a session row
//!   from before a switch, beside its Work item's): the newest.
//! - **Across pools** a session is in every pool that matches it, and each
//!   pool's rules govern what happens in it.
//! - **Sealed.** A sealed pool takes its sessions out of every other pool;
//!   two sealed pools matching one session: the more specific match wins,
//!   then the older pool.
//! - **Skip held.** A pool marked so (a project's default pool) skips a
//!   session its broad members match when another pool names that session
//!   on its own: 0.3's "a session moved into a pool leaves its project's".
//! - An unreadable sign-in is "Account unknown": Account members can't
//!   match it, and it stays in through any other member.
//!
//! The index is built once from the pools and member rows (in-memory maps
//! by member key), and a session resolves in a handful of lookups; it
//! never reads sessions or transcripts.

pub mod talk;

use std::collections::HashMap;

use crate::model::{ProjectId, SessionId, WorkItemId};
use crate::storage::{MemberKind, MemberLevel, MemberRef, Pool, PoolMember, PoolRole, Scope};

/// What the resolver needs to know about one session: read from its own
/// row (its project, App, Account, Tool and Mac) and the switch graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFacts {
    /// The session.
    pub session: SessionId,
    /// Its Work item, when it switched.
    pub work: Option<WorkItemId>,
    /// Its project.
    pub project: Option<ProjectId>,
    /// Its App (`apps.id`).
    pub app: Option<String>,
    /// Its Account (`subscriptions.id`); `None` when the sign-in can't be
    /// read ("Account unknown").
    pub account: Option<String>,
    /// Its Tool (the tool's id).
    pub tool: Option<String>,
    /// The Mac it runs on.
    pub mac: Option<String>,
    /// Its conversation, `(folder, id)`, when known: a Watched member
    /// naming it names this session (continued in AI Juggler).
    pub conversation: Option<(String, String)>,
}

impl SessionFacts {
    /// A session known only by id and project (the rest unknown).
    #[must_use]
    pub fn bare(session: SessionId, project: Option<ProjectId>) -> Self {
        Self {
            session,
            work: None,
            project,
            app: None,
            account: None,
            tool: None,
            mac: None,
            conversation: None,
        }
    }

    /// The member ref that names this session on its own now.
    #[must_use]
    pub fn session_ref(&self) -> MemberRef {
        MemberRef::of_session(&self.session, self.work.as_ref())
    }
}

/// One pool a session is in, and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    /// The pool.
    pub pool_id: String,
    /// The member that brought it in (the most specific that matched).
    pub member: MemberRef,
    /// Its role there.
    pub role: PoolRole,
    /// Its own level there, when the member carries one (else the pool's).
    pub level: Option<MemberLevel>,
    /// When that member joined, epoch ms.
    pub joined_at: i64,
    /// Whether the pool is sealed.
    pub sealed: bool,
    /// Whether the pool is a project's default pool.
    pub default_pool: bool,
}

impl Placement {
    /// How it got in.
    #[must_use]
    pub const fn via(&self) -> MemberKind {
        self.member.kind()
    }
}

/// Why a pool that has a member matching the session doesn't hold it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkipReason {
    /// Its project isn't in the pool's scope ("Add it to the scope").
    OutOfScope,
    /// A sealed pool holds it.
    SealedElsewhere {
        /// The sealed pool.
        sealed_pool: String,
    },
    /// The pool skips sessions another pool names, and one does.
    HeldElsewhere {
        /// The pool that names it.
        by_pool: String,
    },
}

/// A pool a member matched the session in, and why it isn't there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skip {
    /// The pool.
    pub pool_id: String,
    /// The member that matched.
    pub member: MemberRef,
    /// Why it was left out.
    pub reason: SkipReason,
}

/// A session's pools and the ones it was left out of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Resolution {
    /// The pools it's in, the one it's shown in first: sealed first, then
    /// by how specific the match, the user's pools before the default
    /// pools, the newest member first.
    pub placements: Vec<Placement>,
    /// The pools a member matched it in that don't hold it, and why.
    pub skipped: Vec<Skip>,
    /// Its sign-in can't be read while some pool has an Account member.
    pub account_unknown: bool,
}

impl Resolution {
    /// The pool it's shown in first, when it's in any.
    #[must_use]
    pub fn primary(&self) -> Option<&Placement> {
        self.placements.first()
    }

    /// Its place in `pool_id`, when it's there.
    #[must_use]
    pub fn in_pool(&self, pool_id: &str) -> Option<&Placement> {
        self.placements
            .iter()
            .find(|place| place.pool_id == pool_id)
    }
}

/// One member row in the index.
#[derive(Clone, Debug)]
struct Entry {
    pool: usize,
    member: MemberRef,
    role: PoolRole,
    level: Option<MemberLevel>,
    joined_at: i64,
}

/// A pool's own rules, as the resolver needs them.
#[derive(Clone, Debug)]
struct Rules {
    id: String,
    scope: Scope,
    sealed: bool,
    skip_held: bool,
    default_pool: bool,
    created_at: i64,
}

const SLOT_WORK: usize = 0;
const SLOT_SESSION: usize = 1;
const SLOT_APP: usize = 2;
const SLOT_ACCOUNT: usize = 3;
const SLOT_TOOL: usize = 4;
const SLOT_MAC: usize = 5;
const SLOT_CONVERSATION: usize = 6;
/// Everyone shares the session slot's map under the empty id: no session
/// id is empty.
const SLOT_EVERYONE: usize = SLOT_SESSION;

/// Where a member's rows are indexed: its kind's map and the id it names.
fn slot_of(member: &MemberRef) -> (usize, String) {
    match member {
        MemberRef::Work(id) => (SLOT_WORK, id.as_str().to_owned()),
        MemberRef::Session(id) => (SLOT_SESSION, id.as_str().to_owned()),
        MemberRef::App(id) => (SLOT_APP, id.clone()),
        MemberRef::Account(id) => (SLOT_ACCOUNT, id.clone()),
        MemberRef::Tool(id) => (SLOT_TOOL, id.clone()),
        MemberRef::Mac(id) => (SLOT_MAC, id.clone()),
        MemberRef::Everyone => (SLOT_EVERYONE, String::new()),
        MemberRef::Conversation { folder, id } => (SLOT_CONVERSATION, format!("{folder}/{id}")),
    }
}

/// Who is where: the pools and their member rows, indexed by member key.
#[derive(Clone, Debug, Default)]
pub struct Memberships {
    pools: Vec<Rules>,
    entries: Vec<Entry>,
    /// Each kind's rows by the id they name (a session row under its Work
    /// item when its session switched), as indexes into `entries`.
    by_kind: [HashMap<String, Vec<u32>>; 7],
    /// Some pool has an Account member (an unknown sign-in matters).
    account_members: bool,
    works: HashMap<SessionId, WorkItemId>,
}

impl Memberships {
    /// From the live pools, their active member rows and each session's
    /// Work item. Rows of pools not listed are ignored.
    #[must_use]
    pub fn new(
        pools: &[Pool],
        members: &[PoolMember],
        works: HashMap<SessionId, WorkItemId>,
    ) -> Self {
        let index: HashMap<&str, usize> = pools
            .iter()
            .enumerate()
            .map(|(at, pool)| (pool.id.as_str(), at))
            .collect();
        let rules: Vec<Rules> = pools
            .iter()
            .map(|pool| Rules {
                id: pool.id.clone(),
                scope: pool.scope.clone(),
                sealed: pool.sealed,
                skip_held: pool.skip_held,
                default_pool: pool.kind == crate::storage::PoolKind::Default,
                created_at: pool.created_at,
            })
            .collect();
        let mut entries = Vec::new();
        let mut by_kind: [HashMap<String, Vec<u32>>; 7] = Default::default();
        let mut account_members = false;
        for row in members {
            let Some(&pool) = index.get(row.pool_id.as_str()) else {
                continue;
            };
            account_members |= row.member.kind() == MemberKind::Account;
            let (slot, id) = slot_of(&row.member.anchor(&works));
            let at = u32::try_from(entries.len()).unwrap_or(u32::MAX);
            by_kind[slot].entry(id).or_default().push(at);
            entries.push(Entry {
                pool,
                member: row.member.clone(),
                role: row.role,
                level: row.level.clone(),
                joined_at: row.joined_at,
            });
        }
        Self {
            pools: rules,
            entries,
            by_kind,
            account_members,
            works,
        }
    }

    /// The Work item `session` belongs to, when it switched.
    #[must_use]
    pub fn work_of(&self, session: &SessionId) -> Option<&WorkItemId> {
        self.works.get(session)
    }

    /// The session's facts with its Work item filled in from the index.
    #[must_use]
    pub fn with_work(&self, mut facts: SessionFacts) -> SessionFacts {
        if facts.work.is_none() {
            facts.work = self.works.get(&facts.session).cloned();
        }
        facts
    }

    /// The most specific member matching `facts`' session in each pool.
    fn matches(&self, facts: &SessionFacts) -> Vec<(usize, &Entry)> {
        let work = facts
            .work
            .as_ref()
            .or_else(|| self.works.get(&facts.session));
        // The most specific match in each pool.
        let conversation = facts
            .conversation
            .as_ref()
            .map(|(folder, id)| format!("{folder}/{id}"));
        let lookups: [(usize, Option<&str>); 8] = [
            (SLOT_WORK, work.map(WorkItemId::as_str)),
            (SLOT_SESSION, Some(facts.session.as_str())),
            (SLOT_APP, facts.app.as_deref()),
            (SLOT_ACCOUNT, facts.account.as_deref()),
            (SLOT_TOOL, facts.tool.as_deref()),
            (SLOT_MAC, facts.mac.as_deref()),
            (SLOT_EVERYONE, Some("")),
            (SLOT_CONVERSATION, conversation.as_deref()),
        ];
        let mut best: Vec<(usize, &Entry)> = Vec::new();
        for (slot, id) in lookups {
            let Some(id) = id else { continue };
            for &at in self.by_kind[slot].get(id).into_iter().flatten() {
                let entry = &self.entries[at as usize];
                match best.iter_mut().find(|(pool, _)| *pool == entry.pool) {
                    Some((_, held))
                        if (held.member.kind(), -held.joined_at)
                            > (entry.member.kind(), -entry.joined_at) =>
                    {
                        *held = entry;
                    }
                    Some(_) => {}
                    None => best.push((entry.pool, entry)),
                }
            }
        }
        best
    }

    /// Which pools `facts`' session is in, how, and why not the others.
    #[must_use]
    pub fn resolve(&self, facts: &SessionFacts) -> Resolution {
        let mut skipped = Vec::new();
        let mut placed: Vec<(&Rules, &Entry)> = Vec::new();
        for (pool, entry) in self.matches(facts) {
            let rules = &self.pools[pool];
            if entry.member.kind().is_broad() && !rules.scope.holds(facts.project.as_ref()) {
                skipped.push(Skip {
                    pool_id: rules.id.clone(),
                    member: entry.member.clone(),
                    reason: SkipReason::OutOfScope,
                });
            } else {
                placed.push((rules, entry));
            }
        }
        let mut kept = sealed_and_held(placed, &mut skipped);
        kept.sort_by(|(a, ea), (b, eb)| {
            (
                !a.sealed,
                ea.member.kind(),
                a.default_pool,
                -ea.joined_at,
                &a.id,
            )
                .cmp(&(
                    !b.sealed,
                    eb.member.kind(),
                    b.default_pool,
                    -eb.joined_at,
                    &b.id,
                ))
        });
        skipped.sort_by(|a, b| a.pool_id.cmp(&b.pool_id));
        Resolution {
            placements: kept
                .into_iter()
                .map(|(rules, entry)| Placement {
                    pool_id: rules.id.clone(),
                    member: entry.member.clone(),
                    role: entry.role,
                    level: entry.level.clone(),
                    joined_at: entry.joined_at,
                    sealed: rules.sealed,
                    default_pool: rules.default_pool,
                })
                .collect(),
            skipped,
            account_unknown: facts.account.is_none() && self.account_members,
        }
    }

    /// The pool `facts`' session is shown in first and its role there;
    /// `None` when it's in none.
    #[must_use]
    pub fn place(&self, facts: &SessionFacts) -> Option<(String, PoolRole)> {
        self.resolve(facts)
            .primary()
            .map(|place| (place.pool_id.clone(), place.role))
    }

    /// Whether `pool_id` is one of the pools indexed.
    #[must_use]
    pub fn knows(&self, pool_id: &str) -> bool {
        self.pools.iter().any(|pool| pool.id == pool_id)
    }

    /// Whether `pool_id` is sealed.
    #[must_use]
    pub fn sealed(&self, pool_id: &str) -> bool {
        self.pools
            .iter()
            .any(|pool| pool.id == pool_id && pool.sealed)
    }
}

/// A sealed pool takes the session alone; then a pool that skips held
/// sessions lets a broad match give way to another pool's own name.
fn sealed_and_held<'a>(
    mut placed: Vec<(&'a Rules, &'a Entry)>,
    skipped: &mut Vec<Skip>,
) -> Vec<(&'a Rules, &'a Entry)> {
    // Sealed: the sealed pool holding it most specifically, alone.
    if let Some(&(seal, seal_entry)) = placed
        .iter()
        .filter(|(rules, _)| rules.sealed)
        .min_by_key(|(rules, entry)| (entry.member.kind(), rules.created_at, rules.id.clone()))
    {
        for (rules, entry) in &placed {
            if rules.id != seal.id {
                skipped.push(Skip {
                    pool_id: rules.id.clone(),
                    member: entry.member.clone(),
                    reason: SkipReason::SealedElsewhere {
                        sealed_pool: seal.id.clone(),
                    },
                });
            }
        }
        placed = vec![(seal, seal_entry)];
    }
    // Skip held: a broad match gives way to another pool's own name.
    let named: Vec<&str> = placed
        .iter()
        .filter(|(_, entry)| !entry.member.kind().is_broad())
        .map(|(rules, _)| rules.id.as_str())
        .collect();
    let mut kept = Vec::new();
    for (rules, entry) in placed {
        let by = named.iter().find(|pool| **pool != rules.id);
        match by {
            Some(by) if rules.skip_held && entry.member.kind().is_broad() => skipped.push(Skip {
                pool_id: rules.id.clone(),
                member: entry.member.clone(),
                reason: SkipReason::HeldElsewhere {
                    by_pool: (*by).to_owned(),
                },
            }),
            _ => kept.push((rules, entry)),
        }
    }
    kept
}
