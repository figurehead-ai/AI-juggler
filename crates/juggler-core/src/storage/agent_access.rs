//! D-28 storage (schema v33, with pools): each pool's access level, the
//! user's "always allow" rules, the "allow once" answers, and the audit
//! of every agent action.
//!
//! - Levels and rules change only through the app's own commands (the
//!   user, in the app). [`AgentAccessRepo::allowed`] is the one decision
//!   point an agent's request goes through; it writes every request to the
//!   audit, whatever the answer.
//! - A revoked rule stops applying on the very next call: every decision
//!   reads the rules afresh.

use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, params};

use crate::agent_access::{AccessLevel, AgentAction, Decision, HOP_LIMITS, PoolAccess, decide};
use crate::error::CoreError;
use crate::model::Timestamp;

/// Who made a rule. Only the user makes rules (D-28); the column says so
/// on every row.
pub const MADE_BY_USER: &str = "user";

/// One "always allow" rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentRule {
    /// Its id.
    pub id: i64,
    /// The pool it's for.
    pub pool_id: String,
    /// The action it allows.
    pub action: AgentAction,
    /// When the user made it, epoch ms.
    pub created_at: i64,
    /// Who made it ([`MADE_BY_USER`]).
    pub made_by: String,
}

/// One audited agent request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEntry {
    /// When, epoch ms.
    pub at: i64,
    /// The pool it was in.
    pub pool_id: String,
    /// Who asked (`session:<id>`).
    pub actor: String,
    /// What it asked for (the action's word).
    pub action: String,
    /// `allow` | `ask` | `deny`.
    pub decision: String,
}

/// Typed access to the D-28 tables.
#[derive(Debug)]
pub struct AgentAccessRepo<'a> {
    conn: &'a Connection,
}

impl AgentAccessRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> AgentAccessRepo<'_> {
        AgentAccessRepo { conn }
    }

    /// A pool's level, Custom set and rules; no level when the pool isn't
    /// there (or was deleted).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn access(&self, pool_id: &str) -> Result<PoolAccess, CoreError> {
        let row: Option<(String, String, i64, bool)> = self
            .conn
            .query_row(
                "SELECT access_level, custom_actions, hop_limit, delivery_stopped FROM pools
                  WHERE id = ?1 AND deleted_at IS NULL",
                params![pool_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let Some((level, custom, hop_limit, delivery_stopped)) = row else {
            return Ok(PoolAccess::default());
        };
        Ok(PoolAccess {
            // An unknown word (a newer build's level) reads as the
            // narrowest level: never wider than the user chose.
            level: Some(AccessLevel::parse(&level).unwrap_or(AccessLevel::Read)),
            custom: custom.split(',').filter_map(AgentAction::parse).collect(),
            rules: self
                .rules(Some(pool_id))?
                .into_iter()
                .map(|rule| rule.action)
                .collect(),
            // Outside the range (a newer build's): the narrowest.
            hop_limit: u32::try_from(hop_limit)
                .ok()
                .filter(|limit| HOP_LIMITS.contains(limit))
                .unwrap_or(*HOP_LIMITS.start()),
            delivery_stopped,
        })
    }

    /// D-29: the user sets a pool's hop limit and its delivery stop-all.
    ///
    /// # Errors
    ///
    /// [`CoreError::Validation`] for a hop limit outside [`HOP_LIMITS`];
    /// [`CoreError::NotFound`] for a missing or deleted pool;
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn set_delivery(
        &self,
        pool_id: &str,
        hop_limit: u32,
        stopped: bool,
    ) -> Result<(), CoreError> {
        if !HOP_LIMITS.contains(&hop_limit) {
            return Err(CoreError::Validation {
                subject: "hop limit",
                message: format!(
                    "a hop limit is {} to {}",
                    HOP_LIMITS.start(),
                    HOP_LIMITS.end()
                ),
            });
        }
        let changed = self.conn.execute(
            "UPDATE pools SET hop_limit = ?2, delivery_stopped = ?3 WHERE id = ?1 AND deleted_at IS NULL",
            params![pool_id, hop_limit, stopped],
        )?;
        if changed == 0 {
            return Err(super::accounts::not_found("pool", pool_id));
        }
        Ok(())
    }

    /// Sets a pool's level (the user, in the app). `custom` is kept only
    /// for Custom.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a missing or deleted pool;
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn set_level(
        &self,
        pool_id: &str,
        level: AccessLevel,
        custom: &BTreeSet<AgentAction>,
    ) -> Result<(), CoreError> {
        let words = if level == AccessLevel::Custom {
            custom
                .iter()
                .map(|action| action.as_str())
                .collect::<Vec<_>>()
                .join(",")
        } else {
            String::new()
        };
        let changed = self.conn.execute(
            "UPDATE pools SET access_level = ?2, custom_actions = ?3 WHERE id = ?1 AND deleted_at IS NULL",
            params![pool_id, level.as_str(), words],
        )?;
        if changed == 0 {
            return Err(super::accounts::not_found("pool", pool_id));
        }
        Ok(())
    }

    /// The rules in force, for one pool or every pool, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn rules(&self, pool_id: Option<&str>) -> Result<Vec<AgentRule>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT r.id, r.pool_id, r.action, r.created_at, r.made_by FROM agent_rules r
               JOIN pools p ON p.id = r.pool_id
              WHERE r.revoked_at IS NULL AND p.deleted_at IS NULL
                AND (?1 IS NULL OR r.pool_id = ?1)
              ORDER BY r.created_at, r.id",
        )?;
        let rows = stmt.query_map(params![pool_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, pool_id, action, created_at, made_by) = row?;
            if let Some(action) = AgentAction::parse(&action) {
                out.push(AgentRule {
                    id,
                    pool_id,
                    action,
                    created_at,
                    made_by,
                });
            }
        }
        Ok(out)
    }

    /// The user's "Always allow `<action>` in `<pool>`": a rule, made by the
    /// user. Making one that's already there changes nothing.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a missing pool; storage failures.
    pub fn always_allow(&self, pool_id: &str, action: AgentAction) -> Result<AgentRule, CoreError> {
        if self.access(pool_id)?.level.is_none() {
            return Err(super::accounts::not_found("pool", pool_id));
        }
        self.conn.execute(
            "INSERT OR IGNORE INTO agent_rules (pool_id, action, made_by, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                pool_id,
                action.as_str(),
                MADE_BY_USER,
                Timestamp::now().millis()
            ],
        )?;
        self.rules(Some(pool_id))?
            .into_iter()
            .find(|rule| rule.action == action)
            .ok_or_else(|| super::accounts::not_found("rule", action.as_str()))
    }

    /// Revokes a rule: it stops applying on the next call.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a rule that isn't in force; storage
    /// failures.
    pub fn revoke(&self, rule_id: i64) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE agent_rules SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL",
            params![rule_id, Timestamp::now().millis()],
        )?;
        if changed == 0 {
            return Err(super::accounts::not_found("rule", &rule_id.to_string()));
        }
        Ok(())
    }

    /// The user's "Allow once": counted, so the next identical ask can
    /// offer a rule.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn allow_once(&self, pool_id: &str, action: AgentAction) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO agent_approvals (pool_id, action, at) VALUES (?1, ?2, ?3)",
            params![pool_id, action.as_str(), Timestamp::now().millis()],
        )?;
        Ok(())
    }

    /// How often the user has allowed `action` once in `pool_id`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn approvals(&self, pool_id: &str, action: AgentAction) -> Result<u32, CoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM agent_approvals WHERE pool_id = ?1 AND action = ?2",
            params![pool_id, action.as_str()],
            |row| row.get(0),
        )?;
        Ok(u32::try_from(count).unwrap_or(u32::MAX))
    }

    /// The one decision point (D-28): may `actor` (`session:<id>`), a
    /// member of `pool_id` or not, as an observer or not, do `action`
    /// there? `hop` is a delivery's thread hop (D-29). Every request is
    /// written to the audit, whatever the answer.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures (the caller
    /// treats that as a deny).
    pub fn allowed(
        &self,
        pool_id: &str,
        actor: &str,
        member: bool,
        observer: bool,
        action: AgentAction,
        hop: Option<u32>,
    ) -> Result<Decision, CoreError> {
        let access = self.access(pool_id)?;
        let approvals = self.approvals(pool_id, action)?;
        let decision = decide(&access, member, observer, action, approvals, hop);
        self.conn.execute(
            "INSERT INTO agent_audit (at, pool_id, actor_ref, action, decision) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                Timestamp::now().millis(),
                pool_id,
                actor,
                action.as_str(),
                decision.as_str()
            ],
        )?;
        Ok(decision)
    }

    /// The newest audit entries, newest first.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn audit(&self, limit: u32) -> Result<Vec<AuditEntry>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT at, pool_id, actor_ref, action, decision FROM agent_audit
              ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(AuditEntry {
                at: row.get(0)?,
                pool_id: row.get(1)?,
                actor: row.get(2)?,
                action: row.get(3)?,
                decision: row.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}
