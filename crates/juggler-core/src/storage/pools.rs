//! Pools (AJ-E1702, schema v33; D-62's model, AJ-E3003, schema v45):
//! sessions grouped by the user.
//!
//! - v45 (D-62 decisions 8–16): a pool has a **scope** (every project, or
//!   the projects it names, "no project" among them) and **members** that
//!   are rules matching sessions: a Session (its Work item), an App, an
//!   Account, a Tool, a Mac, or everyone in scope. A session can be in
//!   several pools; which, and how, is [`crate::pools::Memberships`]'s
//!   answer. A member may carry its own level; who may talk to whom is
//!   set per role pair, any one connection overridable; a **sealed** pool
//!   keeps its sessions out of every other.
//!
//! - The user owns every pool. Pools, members, roles and access levels
//!   change only through the app's own commands — nothing an agent can
//!   reach (D-28).
//! - D-33: every project has its own default pool, holding all its
//!   sessions on any account, and sessions with no project have one too.
//!   A default pool always exists and can't be deleted or renamed. Since
//!   v45 it is "everyone in scope" over its project, skipping the sessions
//!   another pool names on their own (0.3's behaviour, kept).
//! - A session member hangs off the Work item its session belongs to
//!   (`work:<id>`), else the session (`session:<id>`); a switch keeps the
//!   Work item, so it keeps the pool. One active row per pool and member.
//! - Deleting a pool marks it deleted and ends its rows (`left_at`); its
//!   sessions are back in their projects' default pools and its history
//!   stays.
//! - What agents may do in a pool is its access level and the user's
//!   rules (`agent_access`, D-28).
//!
//! Nothing here starts, stops or types into a session: it reads and
//! writes the pools' own tables only.

use std::collections::{BTreeSet, HashMap};
use std::hash::BuildHasher;

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::CoreError;
use crate::model::{ProjectId, SessionId, Timestamp, WorkItemId};

/// v33: the pools tables. A pool has a name, a kind (`default` | `user`)
/// and an optional project. D-33: every project has its own default pool
/// (`pool-default:<project>`, one per project, made here for the projects
/// there are and by a trigger for each new one), and sessions with no
/// project have theirs (`pool-default:none`). Members hang off the Work
/// item (`work:<id>`, else `session:<id>`) so a switch keeps them; one
/// active row per member (no row: its project's default pool, as a
/// member). Deleting a pool marks it and ends its rows, so its history
/// stays.
///
/// D-28/D-29: each pool's access level (and its Custom set of actions),
/// its delivery hop limit and stop-all flag, whether it spans the user's
/// paired Macs (D-32, off until the user turns it on), the
/// user's "always allow" rules, the "allow once" answers (counted, so the
/// second offers a rule), and the audit of every agent action.
pub(super) const MIGRATION: &str = "
CREATE TABLE pools (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    kind       TEXT NOT NULL,
    project    TEXT,
    created_at INTEGER NOT NULL,
    deleted_at INTEGER,
    access_level   TEXT NOT NULL DEFAULT 'work_together',
    custom_actions TEXT NOT NULL DEFAULT '',
    hop_limit        INTEGER NOT NULL DEFAULT 5,
    delivery_stopped INTEGER NOT NULL DEFAULT 0,
    spans_macs       INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX idx_pools_one_default ON pools(COALESCE(project, '')) WHERE kind = 'default';
INSERT INTO pools (id, name, kind, project, created_at)
    VALUES ('pool-default:none', 'No project', 'default', NULL, 0);
INSERT INTO pools (id, name, kind, project, created_at)
    SELECT 'pool-default:' || id, name, 'default', id, 0 FROM projects;
CREATE TRIGGER pools_default_for_new_project AFTER INSERT ON projects
BEGIN
    INSERT OR IGNORE INTO pools (id, name, kind, project, created_at)
        VALUES ('pool-default:' || NEW.id, NEW.name, 'default', NEW.id,
                CAST(strftime('%s', 'now') AS INTEGER) * 1000);
END;
CREATE TABLE pool_members (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    pool_id    TEXT NOT NULL REFERENCES pools(id),
    member_ref TEXT NOT NULL,
    role       TEXT NOT NULL,
    joined_at  INTEGER NOT NULL,
    left_at    INTEGER
);
CREATE UNIQUE INDEX idx_pool_members_active ON pool_members(member_ref) WHERE left_at IS NULL;
CREATE INDEX idx_pool_members_pool ON pool_members(pool_id);
CREATE TABLE agent_rules (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    pool_id    TEXT NOT NULL REFERENCES pools(id),
    action     TEXT NOT NULL,
    made_by    TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    revoked_at INTEGER
);
CREATE UNIQUE INDEX idx_agent_rules_live ON agent_rules(pool_id, action) WHERE revoked_at IS NULL;
CREATE TABLE agent_approvals (
    pool_id TEXT NOT NULL REFERENCES pools(id),
    action  TEXT NOT NULL,
    at      INTEGER NOT NULL
);
CREATE INDEX idx_agent_approvals ON agent_approvals(pool_id, action);
CREATE TABLE agent_audit (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    at        INTEGER NOT NULL,
    pool_id   TEXT NOT NULL,
    actor_ref TEXT NOT NULL,
    action    TEXT NOT NULL,
    decision  TEXT NOT NULL
);
CREATE INDEX idx_agent_audit_at ON agent_audit(at);
";

/// v45 (D-62, AJ-E3003): the pools model. Additive: nothing is dropped
/// or rewritten, except 0.3's one-pool-per-member index, which becomes one
/// active row per pool and member (a session can be in several pools).
///
/// - Scope: `pools.scope_all` (every project, sessions with no project
///   too), else the `pool_scope` rows (a project id, `''` for "no
///   project").
/// - Members: new `member_ref` kinds `app:<App id>`, `account:<Account
///   id>`, `tool:<tool id>`, `mac:<machine>` and `all`; each row may
///   carry its own level (and Custom set).
/// - `pool_talk`: allowed, ask first or blocked, per role pair
///   (`role:<role>`) or for one connection (two member refs).
/// - `sealed`, and `skip_held` ("skip sessions another pool names").
/// - Tasks gain the project they're for (a pool of several projects).
///
/// Today's pools move with their ids, so their messages, tasks, rules,
/// audit and layout stay attached: a project's default pool becomes its
/// project, everyone in scope, skipping held sessions; the no-project one
/// "no project", everyone; a user pool with a project that project; one
/// with none every project. The database is copied before any migration
/// runs (`<file>.before-v<N>`).
pub(super) const MODEL_MIGRATION: &str = "
ALTER TABLE pools ADD COLUMN scope_all INTEGER NOT NULL DEFAULT 0;
ALTER TABLE pools ADD COLUMN sealed INTEGER NOT NULL DEFAULT 0;
ALTER TABLE pools ADD COLUMN skip_held INTEGER NOT NULL DEFAULT 0;
CREATE TABLE pool_scope (
    pool_id TEXT NOT NULL REFERENCES pools(id),
    project TEXT NOT NULL,
    PRIMARY KEY (pool_id, project)
);
ALTER TABLE pool_members ADD COLUMN level TEXT;
ALTER TABLE pool_members ADD COLUMN custom_actions TEXT;
DROP INDEX idx_pool_members_active;
CREATE UNIQUE INDEX idx_pool_members_active ON pool_members(pool_id, member_ref) WHERE left_at IS NULL;
CREATE INDEX idx_pool_members_ref ON pool_members(member_ref) WHERE left_at IS NULL;
CREATE TABLE pool_talk (
    pool_id  TEXT NOT NULL REFERENCES pools(id),
    from_key TEXT NOT NULL,
    to_key   TEXT NOT NULL,
    rule     TEXT NOT NULL CHECK (rule IN ('allowed', 'ask', 'blocked')),
    PRIMARY KEY (pool_id, from_key, to_key)
);
ALTER TABLE tasks ADD COLUMN project TEXT;
UPDATE tasks SET project = (SELECT project FROM pools WHERE pools.id = tasks.pool_id);
UPDATE pools SET skip_held = 1 WHERE kind = 'default';
INSERT INTO pool_scope (pool_id, project)
    SELECT id, COALESCE(project, '') FROM pools WHERE kind = 'default';
INSERT INTO pool_scope (pool_id, project)
    SELECT id, project FROM pools WHERE kind = 'user' AND project IS NOT NULL;
UPDATE pools SET scope_all = 1 WHERE kind = 'user' AND project IS NULL;
INSERT INTO pool_members (pool_id, member_ref, role, joined_at)
    SELECT id, 'all', 'member', created_at FROM pools WHERE kind = 'default';
DROP TRIGGER pools_default_for_new_project;
CREATE TRIGGER pools_default_for_new_project AFTER INSERT ON projects
BEGIN
    INSERT OR IGNORE INTO pools (id, name, kind, project, created_at, skip_held)
        VALUES ('pool-default:' || NEW.id, NEW.name, 'default', NEW.id,
                CAST(strftime('%s', 'now') AS INTEGER) * 1000, 1);
    INSERT OR IGNORE INTO pool_scope (pool_id, project)
        VALUES ('pool-default:' || NEW.id, NEW.id);
    INSERT OR IGNORE INTO pool_members (pool_id, member_ref, role, joined_at)
        VALUES ('pool-default:' || NEW.id, 'all', 'member',
                CAST(strftime('%s', 'now') AS INTEGER) * 1000);
END;
";

/// The default pool of the sessions that have no project.
pub const NO_PROJECT_POOL_ID: &str = "pool-default:none";

/// D-33: the default pool a session with `project` is in when no row
/// says otherwise — its project's, else the no-project one. Each exists
/// from the project's first appearance (the v33 trigger).
#[must_use]
pub fn default_pool_id(project: Option<&ProjectId>) -> String {
    project.map_or_else(
        || NO_PROJECT_POOL_ID.to_owned(),
        |project| format!("pool-default:{}", project.as_str()),
    )
}

/// A default pool's name on screen: the project's name (as it is now)
/// and what the pool is, or "No project".
#[must_use]
pub fn default_pool_name(project_name: Option<&str>) -> String {
    format!("{} · all sessions", project_name.unwrap_or("No project"))
}

/// A pool's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolKind {
    /// A project's own default pool (or the no-project one): always
    /// there, never renamed or deleted.
    Default,
    /// Made by the user.
    User,
}

impl PoolKind {
    /// The stored word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::User => "user",
        }
    }

    fn parse(word: &str) -> Self {
        if word == "default" {
            Self::Default
        } else {
            Self::User
        }
    }
}

/// A member's role. The user is the implicit owner of every pool and
/// isn't a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PoolRole {
    /// Leads the pool's work.
    Lead,
    /// Works in the pool.
    Member,
    /// D-62 decision 12: reads every task, doc and card, messages anyone,
    /// takes review tasks and sends one back or approves it as its own
    /// action; it doesn't claim build tasks.
    Reviewer,
    /// Sees the pool, does nothing in it.
    Observer,
}

impl PoolRole {
    /// Every role, in the order the UI lists them.
    pub const ALL: [Self; 4] = [Self::Lead, Self::Member, Self::Reviewer, Self::Observer];

    /// The stored word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lead => "lead",
            Self::Member => "member",
            Self::Reviewer => "reviewer",
            Self::Observer => "observer",
        }
    }

    /// The role a stored or wire word names, when it names one.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.as_str() == word)
    }
}

/// How a member matches sessions, most specific first (D-62 decision
/// 11: inside one pool the most specific member wins).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MemberKind {
    /// One session, named on its own (its Work item, so a switch keeps it).
    Session,
    /// An App: every session of it, its future ones too.
    App,
    /// An Account: every session of every App signed in to it.
    Account,
    /// A Tool: every session of it.
    Tool,
    /// A Mac: every session on it (live with pools across Macs, D-70).
    Mac,
    /// Everyone in the pool's scope.
    Everyone,
}

impl MemberKind {
    /// Every kind, most specific first.
    pub const ALL: [Self; 6] = [
        Self::Session,
        Self::App,
        Self::Account,
        Self::Tool,
        Self::Mac,
        Self::Everyone,
    ];

    /// The wire word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::App => "app",
            Self::Account => "account",
            Self::Tool => "tool",
            Self::Mac => "mac",
            Self::Everyone => "everyone",
        }
    }

    /// Whether its sessions join by themselves (every kind but Session).
    #[must_use]
    pub const fn is_broad(self) -> bool {
        !matches!(self, Self::Session)
    }
}

/// What a member row points at.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MemberRef {
    /// A Work item: every session of that line of work (ADR-0011).
    Work(WorkItemId),
    /// A session that has no Work item yet (it hasn't switched).
    Session(SessionId),
    /// An App (`apps.id`): every session it runs, future ones too.
    App(String),
    /// An Account (`subscriptions.id`): every session of every App signed
    /// in to it, worked out from each App's sign-in.
    Account(String),
    /// A Tool (its id): every session of it.
    Tool(String),
    /// A Mac (`instances.machine`): every session on it.
    Mac(String),
    /// Everyone in the pool's scope.
    Everyone,
    /// D-62 decision 15: a session another app runs, by its folder and
    /// conversation: a Watched member (no tools, no rules) until it's
    /// continued in AI Juggler, whose session then matches it as itself.
    Conversation {
        /// The folder (config folder row) it was read from.
        folder: String,
        /// The tool's own conversation id.
        id: String,
    },
}

impl MemberRef {
    /// The stored form: `work:<id>` | `session:<id>` | `app:<id>` |
    /// `account:<id>` | `tool:<id>` | `mac:<id>` | `all` |
    /// `conversation:<folder>/<id>`.
    #[must_use]
    pub fn to_wire(&self) -> String {
        match self {
            Self::Work(id) => format!("work:{}", id.as_str()),
            Self::Session(id) => format!("session:{}", id.as_str()),
            Self::App(id) => format!("app:{id}"),
            Self::Account(id) => format!("account:{id}"),
            Self::Tool(id) => format!("tool:{id}"),
            Self::Mac(id) => format!("mac:{id}"),
            Self::Everyone => "all".to_owned(),
            Self::Conversation { folder, id } => format!("conversation:{folder}/{id}"),
        }
    }

    /// Reads the stored form; anything else is `None`.
    #[must_use]
    pub fn parse(wire: &str) -> Option<Self> {
        if wire == "all" {
            return Some(Self::Everyone);
        }
        let (kind, id) = wire.split_once(':')?;
        if id.is_empty() {
            return None;
        }
        Some(match kind {
            "work" => Self::Work(WorkItemId::new(id)),
            "session" => Self::Session(SessionId::new(id)),
            "app" => Self::App(id.to_owned()),
            "account" => Self::Account(id.to_owned()),
            "tool" => Self::Tool(id.to_owned()),
            "mac" => Self::Mac(id.to_owned()),
            "conversation" => {
                let (folder, id) = id.split_once('/')?;
                if folder.is_empty() || id.is_empty() {
                    return None;
                }
                Self::Conversation {
                    folder: folder.to_owned(),
                    id: id.to_owned(),
                }
            }
            _ => return None,
        })
    }

    /// Whether it names a session another app runs (a Watched member
    /// until continued in AI Juggler).
    #[must_use]
    pub const fn is_watched(&self) -> bool {
        matches!(self, Self::Conversation { .. })
    }

    /// How it matches sessions.
    #[must_use]
    pub const fn kind(&self) -> MemberKind {
        match self {
            Self::Work(_) | Self::Session(_) | Self::Conversation { .. } => MemberKind::Session,
            Self::App(_) => MemberKind::App,
            Self::Account(_) => MemberKind::Account,
            Self::Tool(_) => MemberKind::Tool,
            Self::Mac(_) => MemberKind::Mac,
            Self::Everyone => MemberKind::Everyone,
        }
    }

    /// The member ref for a session: its Work item when it has one.
    #[must_use]
    pub fn of_session(session: &SessionId, work: Option<&WorkItemId>) -> Self {
        work.map_or_else(
            || Self::Session(session.clone()),
            |work| Self::Work(work.clone()),
        )
    }

    /// What this ref stands for now: a session ref whose session has
    /// since got a Work item (it switched) stands for that Work item.
    #[must_use]
    pub fn anchor<S: BuildHasher>(&self, works: &HashMap<SessionId, WorkItemId, S>) -> Self {
        match self {
            Self::Session(id) => Self::of_session(id, works.get(id)),
            _ => self.clone(),
        }
    }
}

/// Which projects' sessions a pool takes (D-62 decision 8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Every project, sessions with no project too.
    Every,
    /// These projects; `None` stands for "no project".
    These(BTreeSet<Option<ProjectId>>),
}

impl Scope {
    /// One project's scope (`None`: no project).
    #[must_use]
    pub fn one(project: Option<&ProjectId>) -> Self {
        Self::These(std::iter::once(project.cloned()).collect())
    }

    /// Whether a session of `project` is in it.
    #[must_use]
    pub fn holds(&self, project: Option<&ProjectId>) -> bool {
        match self {
            Self::Every => true,
            Self::These(projects) => projects.contains(&project.cloned()),
        }
    }

    /// The one project it's for, when it names exactly one real project.
    #[must_use]
    pub fn only_project(&self) -> Option<&ProjectId> {
        match self {
            Self::These(projects) if projects.len() == 1 => projects.iter().next()?.as_ref(),
            _ => None,
        }
    }
}

/// One pool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pool {
    /// Stable id (`pool-…`; a default pool's is [`default_pool_id`]).
    pub id: String,
    /// What the user called it.
    pub name: String,
    /// Default or the user's.
    pub kind: PoolKind,
    /// The one project it was made for (0.3's field; [`Self::scope`] is
    /// what governs since v45).
    pub project: Option<ProjectId>,
    /// When it was made, epoch ms (0 for the default pool).
    pub created_at: i64,
    /// D-32: the user let it span their paired Macs (off by default).
    pub spans_macs: bool,
    /// Which projects' sessions it takes.
    pub scope: Scope,
    /// D-62 decision 14: its sessions are in no other pool and nothing
    /// leaves it.
    pub sealed: bool,
    /// Its broad members skip the sessions another pool names on their
    /// own (a project's default pool: 0.3's behaviour).
    pub skip_held: bool,
}

/// One active member row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolMember {
    /// The pool.
    pub pool_id: String,
    /// Who.
    pub member: MemberRef,
    /// Their role.
    pub role: PoolRole,
    /// When they joined, epoch ms.
    pub joined_at: i64,
    /// Its own level, replacing the pool's for the sessions it brings in
    /// (D-62 decision 12), with its Custom set; `None`: the pool's.
    pub level: Option<MemberLevel>,
}

/// A member's own level: a D-28 level word and, for Custom, its actions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberLevel {
    /// The level's stored word (`read`, `work_together`, `full`, `custom`).
    pub level: String,
    /// The Custom level's actions, comma-separated; empty otherwise.
    pub custom_actions: String,
}

/// Who may tell whom in a pool (D-62 decision 13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TalkRule {
    /// Delivered to the inbox (woken only at Full or by a rule, D-29).
    Allowed,
    /// Held in Needs you until the person delivers it.
    AskFirst,
    /// Refused, and the sender is told why.
    Blocked,
}

impl TalkRule {
    /// Every rule, most open first.
    pub const ALL: [Self; 3] = [Self::Allowed, Self::AskFirst, Self::Blocked];

    /// The stored word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::AskFirst => "ask",
            Self::Blocked => "blocked",
        }
    }

    /// The rule a stored word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rule| rule.as_str() == word)
    }
}

/// One stored talk setting: a role pair, or one connection between two
/// members (which overrides its roles' pair).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TalkKey {
    /// From one role to another.
    Roles(PoolRole, PoolRole),
    /// From one member to another (their stored refs).
    Connection(MemberRef, MemberRef),
}

impl TalkKey {
    fn wire(&self) -> (String, String) {
        match self {
            Self::Roles(from, to) => (
                format!("role:{}", from.as_str()),
                format!("role:{}", to.as_str()),
            ),
            Self::Connection(from, to) => (from.to_wire(), to.to_wire()),
        }
    }

    fn parse(from: &str, to: &str) -> Option<Self> {
        match (from.strip_prefix("role:"), to.strip_prefix("role:")) {
            (Some(from), Some(to)) => {
                Some(Self::Roles(PoolRole::parse(from)?, PoolRole::parse(to)?))
            }
            (None, None) => Some(Self::Connection(
                MemberRef::parse(from)?,
                MemberRef::parse(to)?,
            )),
            _ => None,
        }
    }
}

/// Typed access to `pools`, `pool_scope`, `pool_members` and `pool_talk`.
#[derive(Debug)]
pub struct PoolsRepo<'a> {
    conn: &'a Connection,
}

impl PoolsRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> PoolsRepo<'_> {
        PoolsRepo { conn }
    }

    /// The pools that aren't deleted: the user's oldest first, then the
    /// default pools.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn list(&self) -> Result<Vec<Pool>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM pools WHERE deleted_at IS NULL
              ORDER BY kind = 'default', created_at, id",
        )?;
        let rows = stmt.query_map([], map_pool)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        let mut scopes = self.scope_rows()?;
        for pool in &mut out {
            fill_scope(pool, scopes.remove(&pool.id));
        }
        Ok(out)
    }

    /// One pool that isn't deleted.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] when missing or deleted;
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn get(&self, id: &str) -> Result<Pool, CoreError> {
        let mut pool = self
            .conn
            .query_row(
                "SELECT * FROM pools WHERE id = ?1 AND deleted_at IS NULL",
                params![id],
                map_pool,
            )
            .optional()?
            .ok_or_else(|| super::accounts::not_found("pool", id))?;
        let mut stmt = self
            .conn
            .prepare("SELECT project FROM pool_scope WHERE pool_id = ?1")?;
        let projects = stmt
            .query_map(params![id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        fill_scope(&mut pool, Some(projects));
        Ok(pool)
    }

    /// Each pool's scope rows (`''` for no project), by pool.
    fn scope_rows(&self) -> Result<HashMap<String, Vec<String>>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT pool_id, project FROM pool_scope ORDER BY pool_id, project")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut out: HashMap<String, Vec<String>> = HashMap::new();
        for row in rows {
            let (pool, project) = row?;
            out.entry(pool).or_default().push(project);
        }
        Ok(out)
    }

    /// Makes a user pool for `project` (0.3's call): that project's scope,
    /// or every project.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn create(&self, name: &str, project: Option<&ProjectId>) -> Result<Pool, CoreError> {
        let scope = project.map_or(Scope::Every, |project| Scope::one(Some(project)));
        self.create_scoped(name, &scope)
    }

    /// Makes a user pool over `scope`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn create_scoped(&self, name: &str, scope: &Scope) -> Result<Pool, CoreError> {
        let now = Timestamp::now().millis();
        let pool = Pool {
            id: format!("pool-{}", unique(now)),
            name: name.to_owned(),
            kind: PoolKind::User,
            project: scope.only_project().cloned(),
            created_at: now,
            spans_macs: false,
            scope: scope.clone(),
            sealed: false,
            skip_held: false,
        };
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO pools (id, name, kind, project, created_at) VALUES (?1, ?2, 'user', ?3, ?4)",
            params![
                pool.id,
                pool.name,
                pool.project.as_ref().map(ProjectId::as_str),
                now
            ],
        )?;
        write_scope(&tx, &pool.id, scope)?;
        tx.commit()?;
        Ok(pool)
    }

    /// Sets a pool's scope (a default pool keeps its project's).
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a missing, deleted or default pool;
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn set_scope(&self, id: &str, scope: &Scope) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE pools SET project = ?2 WHERE id = ?1 AND kind = 'user' AND deleted_at IS NULL",
            params![id, scope.only_project().map(ProjectId::as_str)],
        )?;
        if changed == 0 {
            return Err(super::accounts::not_found("pool", id));
        }
        write_scope(&tx, id, scope)?;
        tx.commit()?;
        Ok(())
    }

    /// Seals a pool, or opens it (D-62 decision 14).
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a missing or deleted pool;
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn set_sealed(&self, id: &str, sealed: bool) -> Result<(), CoreError> {
        self.set_flag(id, "sealed", sealed)
    }

    /// Whether a pool's broad members skip sessions another pool names.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a missing or deleted pool;
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn set_skip_held(&self, id: &str, skip: bool) -> Result<(), CoreError> {
        self.set_flag(id, "skip_held", skip)
    }

    fn set_flag(&self, id: &str, column: &str, on: bool) -> Result<(), CoreError> {
        let sql = match column {
            "sealed" => "UPDATE pools SET sealed = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            "skip_held" => "UPDATE pools SET skip_held = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            _ => "UPDATE pools SET spans_macs = ?2 WHERE id = ?1 AND deleted_at IS NULL",
        };
        if self.conn.execute(sql, params![id, on])? == 0 {
            return Err(super::accounts::not_found("pool", id));
        }
        Ok(())
    }

    /// Renames a user pool (the default keeps its name).
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a missing, deleted or default pool;
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn rename(&self, id: &str, name: &str) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE pools SET name = ?2 WHERE id = ?1 AND kind = 'user' AND deleted_at IS NULL",
            params![id, name],
        )?;
        if changed == 0 {
            return Err(super::accounts::not_found("pool", id));
        }
        Ok(())
    }

    /// D-32: lets a pool span the user's paired Macs, or not (the user,
    /// in the app). The link that carries it is later work.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a missing or deleted pool;
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn set_spans_macs(&self, id: &str, spans: bool) -> Result<(), CoreError> {
        self.set_flag(id, "spans_macs", spans)
    }

    /// Deletes a user pool: marks it and ends its member rows, so its
    /// sessions keep their other pools (or are back in their project's).
    /// The default pool can't be deleted. Returns how many rows ended.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a missing, deleted or default pool;
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn delete(&self, id: &str) -> Result<usize, CoreError> {
        let now = Timestamp::now().millis();
        let tx = self.conn.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE pools SET deleted_at = ?2 WHERE id = ?1 AND kind = 'user' AND deleted_at IS NULL",
            params![id, now],
        )?;
        if changed == 0 {
            return Err(super::accounts::not_found("pool", id));
        }
        let ended = tx.execute(
            "UPDATE pool_members SET left_at = ?2 WHERE pool_id = ?1 AND left_at IS NULL",
            params![id, now],
        )?;
        tx.commit()?;
        Ok(ended)
    }

    /// Every active member row, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn members(&self) -> Result<Vec<PoolMember>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.pool_id, m.member_ref, m.role, m.joined_at, m.level, m.custom_actions
               FROM pool_members m JOIN pools p ON p.id = m.pool_id
              WHERE m.left_at IS NULL AND p.deleted_at IS NULL ORDER BY m.joined_at, m.id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (pool_id, member, role, joined_at, level, custom) = row?;
            let Some(member) = MemberRef::parse(&member) else {
                continue;
            };
            out.push(PoolMember {
                pool_id,
                member,
                role: PoolRole::parse(&role).unwrap_or(PoolRole::Member),
                joined_at,
                level: level.map(|level| MemberLevel {
                    level,
                    custom_actions: custom.unwrap_or_default(),
                }),
            });
        }
        Ok(out)
    }

    /// Puts `member` in `pool_id` with `role` (0.3's move, kept for the
    /// one-pool callers): ends every active row in `replaces` (the rows
    /// that stand for the same member: its own, and a session row from
    /// before it switched) in every pool, then adds the new one.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn place(
        &self,
        member: &MemberRef,
        replaces: &[MemberRef],
        pool_id: &str,
        role: PoolRole,
    ) -> Result<(), CoreError> {
        let now = Timestamp::now().millis();
        let tx = self.conn.unchecked_transaction()?;
        for old in replaces.iter().chain(std::iter::once(member)) {
            tx.execute(
                "UPDATE pool_members SET left_at = ?2 WHERE member_ref = ?1 AND left_at IS NULL",
                params![old.to_wire(), now],
            )?;
        }
        tx.execute(
            "INSERT INTO pool_members (pool_id, member_ref, role, joined_at) VALUES (?1, ?2, ?3, ?4)",
            params![pool_id, member.to_wire(), role.as_str(), now],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Adds `member` to `pool_id` with `role` and its own `level`, or
    /// changes its row there; its rows in other pools stay (a session can
    /// be in several, D-62 decision 10). `replaces` are rows in this pool
    /// that stand for the same member (a session row from before its
    /// switch); they end.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn join(
        &self,
        pool_id: &str,
        member: &MemberRef,
        replaces: &[MemberRef],
        role: PoolRole,
        level: Option<&MemberLevel>,
    ) -> Result<(), CoreError> {
        let now = Timestamp::now().millis();
        let tx = self.conn.unchecked_transaction()?;
        for old in replaces.iter().filter(|old| *old != member) {
            tx.execute(
                "UPDATE pool_members SET left_at = ?3 WHERE pool_id = ?1 AND member_ref = ?2 AND left_at IS NULL",
                params![pool_id, old.to_wire(), now],
            )?;
        }
        let (level, custom) = level.map_or((None, None), |level| {
            (
                Some(level.level.as_str()),
                Some(level.custom_actions.as_str()),
            )
        });
        let changed = tx.execute(
            "UPDATE pool_members SET role = ?3, level = ?4, custom_actions = ?5
              WHERE pool_id = ?1 AND member_ref = ?2 AND left_at IS NULL",
            params![pool_id, member.to_wire(), role.as_str(), level, custom],
        )?;
        if changed == 0 {
            tx.execute(
                "INSERT INTO pool_members (pool_id, member_ref, role, joined_at, level, custom_actions)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![pool_id, member.to_wire(), role.as_str(), now, level, custom],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Ends `member`'s rows in `pool_id` (and the rows standing for it
    /// there). Returns how many ended; its other pools stay.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn leave(&self, pool_id: &str, members: &[MemberRef]) -> Result<usize, CoreError> {
        let now = Timestamp::now().millis();
        let tx = self.conn.unchecked_transaction()?;
        let mut ended = 0;
        for member in members {
            ended += tx.execute(
                "UPDATE pool_members SET left_at = ?3 WHERE pool_id = ?1 AND member_ref = ?2 AND left_at IS NULL",
                params![pool_id, member.to_wire(), now],
            )?;
        }
        tx.commit()?;
        Ok(ended)
    }

    /// Every pool's talk settings: `(pool, key, rule)`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn talk(&self) -> Result<Vec<(String, TalkKey, TalkRule)>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT pool_id, from_key, to_key, rule FROM pool_talk ORDER BY pool_id, from_key, to_key")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (pool, from, to, rule) = row?;
            if let (Some(key), Some(rule)) = (TalkKey::parse(&from, &to), TalkRule::parse(&rule)) {
                out.push((pool, key, rule));
            }
        }
        Ok(out)
    }

    /// Sets one talk setting, or clears it (`None`: back to the role
    /// pair's, or to allowed).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn set_talk(
        &self,
        pool_id: &str,
        key: &TalkKey,
        rule: Option<TalkRule>,
    ) -> Result<(), CoreError> {
        let (from, to) = key.wire();
        match rule {
            Some(rule) => self.conn.execute(
                "INSERT INTO pool_talk (pool_id, from_key, to_key, rule) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (pool_id, from_key, to_key) DO UPDATE SET rule = excluded.rule",
                params![pool_id, from, to, rule.as_str()],
            )?,
            None => self.conn.execute(
                "DELETE FROM pool_talk WHERE pool_id = ?1 AND from_key = ?2 AND to_key = ?3",
                params![pool_id, from, to],
            )?,
        };
        Ok(())
    }
}

/// Writes `scope` as the pool's: `scope_all`, or its rows.
fn write_scope(conn: &Connection, id: &str, scope: &Scope) -> Result<(), CoreError> {
    conn.execute("DELETE FROM pool_scope WHERE pool_id = ?1", params![id])?;
    match scope {
        Scope::Every => {
            conn.execute("UPDATE pools SET scope_all = 1 WHERE id = ?1", params![id])?;
        }
        Scope::These(projects) => {
            conn.execute("UPDATE pools SET scope_all = 0 WHERE id = ?1", params![id])?;
            for project in projects {
                conn.execute(
                    "INSERT INTO pool_scope (pool_id, project) VALUES (?1, ?2)",
                    params![id, project.as_ref().map_or("", ProjectId::as_str)],
                )?;
            }
        }
    }
    Ok(())
}

/// Fills a pool read with `scope_all` with its scope rows.
fn fill_scope(pool: &mut Pool, rows: Option<Vec<String>>) {
    if pool.scope == Scope::Every {
        return;
    }
    pool.scope = Scope::These(
        rows.unwrap_or_default()
            .into_iter()
            .map(|project| (!project.is_empty()).then(|| ProjectId::new(project)))
            .collect(),
    );
}

fn map_pool(row: &Row<'_>) -> rusqlite::Result<Pool> {
    let scope_all: bool = row.get("scope_all")?;
    Ok(Pool {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: PoolKind::parse(&row.get::<_, String>("kind")?),
        project: row.get::<_, Option<String>>("project")?.map(ProjectId::new),
        created_at: row.get("created_at")?,
        spans_macs: row.get("spans_macs")?,
        scope: if scope_all {
            Scope::Every
        } else {
            Scope::These(BTreeSet::new())
        },
        sealed: row.get("sealed")?,
        skip_held: row.get("skip_held")?,
    })
}

/// A unique suffix: the time, plus a counter for two in one millisecond.
fn unique(now: i64) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed) % 1_000;
    format!("{now}-{seq:03}")
}
