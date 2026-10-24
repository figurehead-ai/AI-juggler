//! Pools (AJ-E1702, schema v33): sessions grouped by the user.
//!
//! - The user owns every pool. Pools, members, roles and access levels
//!   change only through the app's own commands — nothing an agent can
//!   reach (D-28).
//! - D-33: every project has its own default pool, holding all its
//!   sessions on any account, and sessions with no project have one too.
//!   A default pool always exists and can't be deleted or renamed. A
//!   session with no active member row is in its project's, as a member.
//! - A member hangs off the Work item its session belongs to
//!   (`work:<id>`), else the session (`session:<id>`); a switch keeps the
//!   Work item, so it keeps the pool. One active row per member.
//! - Deleting a pool marks it deleted and ends its rows (`left_at`); its
//!   sessions are back in their projects' default pools and its history
//!   stays.
//! - What agents may do in a pool is its access level and the user's
//!   rules (`agent_access`, D-28).
//!
//! Nothing here starts, stops or types into a session: it reads and
//! writes the pools' own tables only.

use std::collections::HashMap;
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
    /// Sees the pool, does nothing in it.
    Observer,
}

impl PoolRole {
    /// Every role, in the order the UI lists them.
    pub const ALL: [Self; 3] = [Self::Lead, Self::Member, Self::Observer];

    /// The stored word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lead => "lead",
            Self::Member => "member",
            Self::Observer => "observer",
        }
    }

    /// The role a stored or wire word names, when it names one.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "lead" => Some(Self::Lead),
            "member" => Some(Self::Member),
            "observer" => Some(Self::Observer),
            _ => None,
        }
    }
}

/// What a member row points at.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MemberRef {
    /// A Work item: every session of that line of work (ADR-0011).
    Work(WorkItemId),
    /// A session that has no Work item yet (it hasn't switched).
    Session(SessionId),
}

impl MemberRef {
    /// The stored form: `work:<id>` | `session:<id>`.
    #[must_use]
    pub fn to_wire(&self) -> String {
        match self {
            Self::Work(id) => format!("work:{}", id.as_str()),
            Self::Session(id) => format!("session:{}", id.as_str()),
        }
    }

    /// Reads the stored form; anything else is `None`.
    #[must_use]
    pub fn parse(wire: &str) -> Option<Self> {
        if let Some(id) = wire.strip_prefix("work:") {
            Some(Self::Work(WorkItemId::new(id)))
        } else {
            wire.strip_prefix("session:")
                .map(|id| Self::Session(SessionId::new(id)))
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
            Self::Work(_) => self.clone(),
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
    /// The one project it's for, when the user said so.
    pub project: Option<ProjectId>,
    /// When it was made, epoch ms (0 for the default pool).
    pub created_at: i64,
    /// D-32: the user let it span their paired Macs (off by default).
    pub spans_macs: bool,
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
}

/// Which pool each session (with its project) is in, and its role.
/// `works` maps a session to its Work item; a session no row matches is
/// in its project's default pool, as a member. When two rows stand for the same Work item (a session added on
/// its own, then switched), the newest wins.
#[must_use]
pub fn resolve_memberships<S: BuildHasher>(
    sessions: &[(SessionId, Option<ProjectId>)],
    members: &[PoolMember],
    works: &HashMap<SessionId, WorkItemId, S>,
) -> HashMap<SessionId, (String, PoolRole)> {
    let mut by_anchor: HashMap<MemberRef, &PoolMember> = HashMap::new();
    for member in members {
        let anchor = member.member.anchor(works);
        match by_anchor.get(&anchor) {
            Some(seen) if seen.joined_at > member.joined_at => {}
            _ => {
                by_anchor.insert(anchor, member);
            }
        }
    }
    sessions
        .iter()
        .map(|(session, project)| {
            let anchor = MemberRef::of_session(session, works.get(session));
            let place = by_anchor.get(&anchor).map_or_else(
                || (default_pool_id(project.as_ref()), PoolRole::Member),
                |member| (member.pool_id.clone(), member.role),
            );
            (session.clone(), place)
        })
        .collect()
}

/// Typed access to `pools` and `pool_members`.
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
        Ok(out)
    }

    /// One pool that isn't deleted.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] when missing or deleted;
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn get(&self, id: &str) -> Result<Pool, CoreError> {
        self.conn
            .query_row(
                "SELECT * FROM pools WHERE id = ?1 AND deleted_at IS NULL",
                params![id],
                map_pool,
            )
            .optional()?
            .ok_or_else(|| super::accounts::not_found("pool", id))
    }

    /// Makes a user pool.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn create(&self, name: &str, project: Option<&ProjectId>) -> Result<Pool, CoreError> {
        let now = Timestamp::now().millis();
        let pool = Pool {
            id: format!("pool-{}", unique(now)),
            name: name.to_owned(),
            kind: PoolKind::User,
            project: project.cloned(),
            created_at: now,
            spans_macs: false,
        };
        self.conn.execute(
            "INSERT INTO pools (id, name, kind, project, created_at) VALUES (?1, ?2, 'user', ?3, ?4)",
            params![
                pool.id,
                pool.name,
                pool.project.as_ref().map(ProjectId::as_str),
                now
            ],
        )?;
        Ok(pool)
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
        let changed = self.conn.execute(
            "UPDATE pools SET spans_macs = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id, spans],
        )?;
        if changed == 0 {
            return Err(super::accounts::not_found("pool", id));
        }
        Ok(())
    }

    /// Deletes a user pool: marks it and ends its member rows, so its
    /// sessions are back in the default pool. The default pool can't be
    /// deleted. Returns how many members went back.
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
            "SELECT pool_id, member_ref, role, joined_at FROM pool_members
              WHERE left_at IS NULL ORDER BY joined_at, id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (pool_id, member, role, joined_at) = row?;
            let Some(member) = MemberRef::parse(&member) else {
                continue;
            };
            out.push(PoolMember {
                pool_id,
                member,
                role: PoolRole::parse(&role).unwrap_or(PoolRole::Member),
                joined_at,
            });
        }
        Ok(out)
    }

    /// Puts `member` in `pool_id` with `role`: ends every active row in
    /// `replaces` (the rows that stand for the same member: its own, and
    /// a session row from before it switched), then adds the new one.
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
}

fn map_pool(row: &Row<'_>) -> rusqlite::Result<Pool> {
    Ok(Pool {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: PoolKind::parse(&row.get::<_, String>("kind")?),
        project: row.get::<_, Option<String>>("project")?.map(ProjectId::new),
        created_at: row.get("created_at")?,
        spans_macs: row.get("spans_macs")?,
    })
}

/// A unique suffix: the time, plus a counter for two in one millisecond.
fn unique(now: i64) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed) % 1_000;
    format!("{now}-{seq:03}")
}
