//! Session repository.

use std::path::PathBuf;

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::de::Error as _;

use crate::error::CoreError;
use crate::model::{
    ConversationId, InstanceKind, InstanceRecord, MachineId, ProfileId, ProjectId, SessionId,
    SessionStatus, Timestamp, TranscriptRef, WindowIdentity,
};

/// AJ-E1502: `session id → (label, pinned at ms)` for sessions with either.
pub type SessionMarks = std::collections::BTreeMap<String, (Option<String>, Option<i64>)>;

/// Converts a stored-JSON parse failure into a row-mapping failure.
fn json_err(e: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
}

/// Typed read/write access to the `sessions` table.
#[derive(Debug)]
pub struct SessionsRepo<'a> {
    conn: &'a Connection,
}

/// The lifecycle events that set a session's reviewed and archived state
/// ([`SessionsRepo::lifecycle_latest`]); any other event is history only.
pub const STATE_EVENTS: [&str; 4] = ["reviewed", "unreviewed", "archived", "unarchived"];

impl SessionsRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> SessionsRepo<'_> {
        SessionsRepo { conn }
    }

    /// Inserts a new session record.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on I/O or constraint violations
    /// (unknown profile, duplicate id).
    pub fn insert(&self, session: &InstanceRecord) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO instances
                 (id, conversation_id, launch_recipe_id, machine, kind, project, cwd, status,
                  started_at, last_activity, usage_json, transcript_ref_json, attention_json,
                  running_model_json, window_identity_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                session.id.as_str(),
                session.conversation.as_ref().map(ConversationId::as_str),
                session.profile.as_str(),
                session.machine.as_str(),
                session.kind.as_str(),
                session
                    .project
                    .as_ref()
                    .map(crate::model::ProjectId::as_str),
                session.cwd.to_string_lossy(),
                session.status.as_str(),
                session.started_at.millis(),
                session.last_activity.map(Timestamp::millis),
                serde_json::to_string(&session.usage)?,
                session
                    .transcript_ref
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()?,
                serde_json::to_string(&session.attention).ok(),
                serde_json::to_string(&session.running).ok(),
                serde_json::to_string(&session.window).ok(),
            ],
        )?;
        self.journal(session.id.as_str(), None, session.status.as_str())?;
        Ok(())
    }

    /// AJ-E1502: sets (or clears, with `None`) the label shown over a
    /// session's own title.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] for an unknown session, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn set_label(&self, id: &SessionId, label: Option<&str>) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE instances SET label = ?2 WHERE id = ?1",
            params![id.as_str(), label],
        )?;
        super::accounts::require_changed(changed, "session", id.as_str())
    }

    /// AJ-E1502: pins a session to the top of its group (now), or unpins it.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] for an unknown session, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn set_pinned(&self, id: &SessionId, pinned: bool) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE instances SET pinned_at = ?2 WHERE id = ?1",
            params![id.as_str(), pinned.then(|| Timestamp::now().millis())],
        )?;
        super::accounts::require_changed(changed, "session", id.as_str())
    }

    /// AJ-E1502: every session with a label or a pin, `session id →
    /// (label, pinned at)`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn marks(&self) -> Result<SessionMarks, CoreError> {
        let mut statement = self.conn.prepare(
            "SELECT id, label, pinned_at FROM instances
             WHERE label IS NOT NULL OR pinned_at IS NOT NULL",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                (
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ),
            ))
        })?;
        let mut out = std::collections::BTreeMap::new();
        for row in rows {
            let (id, marks) = row?;
            out.insert(id, marks);
        }
        Ok(out)
    }

    /// AJ-E1004: appends one status transition to the journal (the single
    /// source the overnight summary reads; never rewritten).
    fn journal(&self, id: &str, from: Option<&str>, to: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO instance_transitions (session_id, from_status, to_status, at_ms)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, from, to, Timestamp::now().millis()],
        )?;
        Ok(())
    }

    /// AJ-E1005/AJ-E1007: appends one lifecycle event (`reviewed`,
    /// `archived`, `unarchived`) for `id`, timestamped now.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] for an unknown Instance, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn record_lifecycle(&self, id: &SessionId, event: &str) -> Result<i64, CoreError> {
        self.get(id)?; // NotFound for an unknown Instance
        let at = Timestamp::now().millis();
        self.conn.execute(
            "INSERT INTO instance_lifecycle (session_id, event, at_ms) VALUES (?1, ?2, ?3)",
            params![id.as_str(), event, at],
        )?;
        Ok(at)
    }

    /// AJ-E1005/AJ-E1007: the latest state event (`STATE_EVENTS`: reviewed
    /// or archived, and their undoing) per Instance, `session id → (event,
    /// at_ms)`; Instances without one are absent. Other lifecycle events (a
    /// restart, AJ-E1509) are history only and never change that state.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn lifecycle_latest(
        &self,
    ) -> Result<std::collections::BTreeMap<String, (String, i64)>, CoreError> {
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT session_id, event, at_ms FROM instance_lifecycle WHERE event IN ({}) ORDER BY rowid",
                STATE_EVENTS.map(|event| format!("'{event}'")).join(", ")
            ))?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        let mut latest = std::collections::BTreeMap::new();
        for row in rows {
            let (id, event, at) = row?;
            latest.insert(id, (event, at));
        }
        Ok(latest)
    }

    /// AJ-E1007: every lifecycle event of `id`, oldest first:
    /// `(event, at_ms)`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn lifecycle_events(&self, id: &SessionId) -> Result<Vec<(String, i64)>, CoreError> {
        let mut statement = self.conn.prepare(
            "SELECT event, at_ms FROM instance_lifecycle WHERE session_id = ?1 ORDER BY rowid",
        )?;
        let rows =
            statement.query_map(params![id.as_str()], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// AJ-E1004: every status transition of `id`, oldest first:
    /// `(from, to, at_ms)` — `from` is `None` for the first one.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn transitions(
        &self,
        id: &SessionId,
    ) -> Result<Vec<(Option<String>, String, i64)>, CoreError> {
        let mut statement = self.conn.prepare(
            "SELECT from_status, to_status, at_ms FROM instance_transitions
             WHERE session_id = ?1 ORDER BY rowid",
        )?;
        let rows = statement.query_map(params![id.as_str()], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// AJ-E1004: the Instances that changed status since `since_ms`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn changed_since(&self, since_ms: i64) -> Result<Vec<SessionId>, CoreError> {
        let mut statement = self
            .conn
            .prepare("SELECT DISTINCT session_id FROM instance_transitions WHERE at_ms >= ?1")?;
        let rows = statement.query_map(params![since_ms], |row| row.get::<_, String>(0))?;
        rows.map(|row| row.map(SessionId::new))
            .collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// Updates an existing session record by id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the id does not exist,
    /// [`CoreError::Storage`] on SQLite failures, or
    /// [`CoreError::Serialization`] for values that cannot be stored.
    pub fn update(&self, session: &InstanceRecord) -> Result<(), CoreError> {
        let before: Option<String> = self
            .conn
            .query_row(
                "SELECT status FROM instances WHERE id = ?1",
                params![session.id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let changed = self.conn.execute(
            "UPDATE instances SET
                 conversation_id = ?2, status = ?3, last_activity = ?4,
                 usage_json = ?5, transcript_ref_json = ?6, project = ?7,
                 attention_json = ?8, running_model_json = ?9,
                 window_identity_json = ?10
             WHERE id = ?1",
            params![
                session.id.as_str(),
                session.conversation.as_ref().map(ConversationId::as_str),
                session.status.as_str(),
                session.last_activity.map(Timestamp::millis),
                serde_json::to_string(&session.usage)?,
                session
                    .transcript_ref
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()?,
                session
                    .project
                    .as_ref()
                    .map(crate::model::ProjectId::as_str),
                serde_json::to_string(&session.attention).ok(),
                serde_json::to_string(&session.running).ok(),
                serde_json::to_string(&session.window).ok(),
            ],
        )?;
        super::accounts::require_changed(changed, "session", session.id.as_str())?;
        if let Some(before) = before
            && before != session.status.as_str()
        {
            self.journal(session.id.as_str(), Some(&before), session.status.as_str())?;
        }
        Ok(())
    }

    /// Fetches one session by id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing, [`CoreError::Storage`]
    /// on SQLite failures, or [`CoreError::Serialization`] when stored state
    /// cannot be interpreted (degrades as an error, never a panic).
    pub fn get(&self, id: &SessionId) -> Result<InstanceRecord, CoreError> {
        self.conn
            .query_row(
                "SELECT * FROM instances WHERE id = ?1",
                params![id.as_str()],
                map_session,
            )
            .optional()?
            .ok_or_else(|| super::accounts::not_found("session", id.as_str()))
    }

    /// The working folders of this machine's sessions that haven't ended
    /// (one cheap query: the machine sampler's disks-in-use read, AJ-E1401 —
    /// never the whole history, never another Mac's paths).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn active_cwds(&self, machine: &MachineId) -> Result<Vec<std::path::PathBuf>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT DISTINCT cwd FROM instances
             WHERE machine = ?1 AND status NOT IN {ENDED}"
        ))?;
        let rows = stmt.query_map(params![machine.as_str()], |row| row.get::<_, String>(0))?;
        rows.map(|row| {
            row.map(std::path::PathBuf::from)
                .map_err(CoreError::Storage)
        })
        .collect()
    }

    /// The sessions that haven't ended, newest first: one query for a
    /// frequent tick, never the whole history (AJ-E1520).
    ///
    /// # Errors
    ///
    /// Same as [`SessionsRepo::get`].
    pub fn live(&self) -> Result<Vec<InstanceRecord>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT * FROM instances WHERE status NOT IN {ENDED} ORDER BY started_at DESC, id"
        ))?;
        let rows = stmt.query_map([], map_session)?;
        rows.map(|row| row.map_err(CoreError::Storage)).collect()
    }

    /// The working set, newest first: every session that hasn't ended, is
    /// pinned or was active since `all_since_ms` (the last day: the rail's
    /// "ended today" counts stay true), and the `recent` newest that ended
    /// since `since_ms`. What
    /// the rail and the app's lists hold (0.1.5 review: they read the
    /// whole history); History and the Sessions tabs page the rest.
    ///
    /// # Errors
    ///
    /// Same as [`SessionsRepo::get`].
    pub fn working_set(
        &self,
        all_since_ms: i64,
        since_ms: i64,
        recent: usize,
    ) -> Result<Vec<InstanceRecord>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT * FROM instances
              WHERE status NOT IN {ENDED} OR pinned_at IS NOT NULL
                 OR COALESCE(last_activity, started_at) >= ?3
                 OR id IN (SELECT id FROM instances
                            WHERE status IN {ENDED}
                              AND COALESCE(last_activity, started_at) >= ?1
                            ORDER BY COALESCE(last_activity, started_at) DESC, id
                            LIMIT ?2)
              ORDER BY started_at DESC, id"
        ))?;
        let rows = stmt.query_map(
            params![
                since_ms,
                i64::try_from(recent).unwrap_or(i64::MAX),
                all_since_ms
            ],
            map_session,
        )?;
        rows.map(|row| row.map_err(CoreError::Storage)).collect()
    }

    /// 0.1.8: the user's label of each session that has one and knows
    /// its conversation, `(account, conversation, label)` (the cost table
    /// names a session as the rail does). By account too: a switch to
    /// another account of the same tool keeps the conversation's id there
    /// (see [`Self::conversation_links`]). Only labelled sessions: a few
    /// rows.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn conversation_labels(&self) -> Result<Vec<(String, String, String)>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT r.account_id, i.conversation_id, i.label FROM instances i \
             JOIN launch_recipes r ON r.id = i.launch_recipe_id \
             WHERE i.conversation_id IS NOT NULL AND i.label IS NOT NULL",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        rows.map(|row| row.map_err(CoreError::Storage)).collect()
    }

    /// Each session's conversation, `(account, conversation, session)`,
    /// oldest session first: one cheap read for the conversations list
    /// (never the sessions' whole rows). The account tells one copy of a
    /// conversation from another: a switch to another account of the same
    /// tool continues the conversation under the same id there.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn conversation_links(&self) -> Result<Vec<(String, String, String)>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT r.account_id, i.conversation_id, i.id FROM instances i \
             JOIN launch_recipes r ON r.id = i.launch_recipe_id \
             WHERE i.conversation_id IS NOT NULL ORDER BY i.started_at, i.id",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        rows.map(|row| row.map_err(CoreError::Storage)).collect()
    }

    /// These sessions (the ones of them that exist), newest first.
    ///
    /// # Errors
    ///
    /// Same as [`SessionsRepo::get`].
    pub fn get_many(&self, ids: &[SessionId]) -> Result<Vec<InstanceRecord>, CoreError> {
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(record) = self
                .conn
                .query_row(
                    "SELECT * FROM instances WHERE id = ?1",
                    params![id.as_str()],
                    map_session,
                )
                .optional()?
            {
                out.push(record);
            }
        }
        out.sort_by(|a, b| {
            b.started_at
                .cmp(&a.started_at)
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });
        Ok(out)
    }

    /// Lists sessions, newest first, optionally filtered by profile.
    ///
    /// # Errors
    ///
    /// Same as [`SessionsRepo::get`].
    pub fn list(&self, profile: Option<&ProfileId>) -> Result<Vec<InstanceRecord>, CoreError> {
        let mut out = Vec::new();
        if let Some(profile) = profile {
            let mut stmt = self.conn.prepare(
                "SELECT * FROM instances WHERE launch_recipe_id = ?1 ORDER BY started_at DESC, id",
            )?;
            let rows = stmt.query_map(params![profile.as_str()], map_session)?;
            for row in rows {
                out.push(row?);
            }
        } else {
            let mut stmt = self
                .conn
                .prepare("SELECT * FROM instances ORDER BY started_at DESC, id")?;
            let rows = stmt.query_map([], map_session)?;
            for row in rows {
                out.push(row?);
            }
        }
        Ok(out)
    }
}

/// The statuses of a session that has ended, as SQL (`SessionStatus`'s
/// `Finished`, `Stopped`, `Failed`, `HandedOver`).
const ENDED: &str = "('finished', 'stopped', 'failed', 'handed_over')";

/// Maps a row to an [`InstanceRecord`] (row values unchanged; ADR-0008 A-1).
fn map_session(row: &Row<'_>) -> rusqlite::Result<InstanceRecord> {
    let status_raw: String = row.get("status")?;
    let kind_raw: String = row.get("kind")?;
    let usage_json: String = row.get("usage_json")?;
    let transcript_json: Option<String> = row.get("transcript_ref_json")?;
    let attention_json: Option<String> = row.get("attention_json")?;
    let running_json: Option<String> = row.get("running_model_json")?;
    let window_json: Option<String> = row.get("window_identity_json")?;
    Ok(InstanceRecord {
        id: SessionId::new(row.get::<_, String>("id")?),
        conversation: row
            .get::<_, Option<String>>("conversation_id")?
            .map(ConversationId::new),
        profile: ProfileId::new(row.get::<_, String>("launch_recipe_id")?),
        machine: MachineId::new(row.get::<_, String>("machine")?),
        kind: InstanceKind::parse(&kind_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(serde_json::Error::custom(format!(
                    "unknown instance kind: {kind_raw}"
                ))),
            )
        })?,
        project: row.get::<_, Option<String>>("project")?.map(ProjectId::new),
        cwd: PathBuf::from(row.get::<_, String>("cwd")?),
        // Unknown status values (e.g. written by a future build that somehow
        // shares this database) degrade to an error, never a panic.
        status: SessionStatus::parse(&status_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(serde_json::Error::custom(format!(
                    "unknown session status: {status_raw}"
                ))),
            )
        })?,
        started_at: crate::model::Timestamp::from_millis(row.get("started_at")?),
        last_activity: row
            .get::<_, Option<i64>>("last_activity")?
            .map(crate::model::Timestamp::from_millis),
        usage: serde_json::from_str(&usage_json).map_err(json_err)?,
        transcript_ref: transcript_json
            .as_deref()
            .map(serde_json::from_str::<TranscriptRef>)
            .transpose()
            .map_err(json_err)?,
        // A malformed/legacy attention blob degrades to `None` — the
        // reason is advisory display state, never a row-breaking value.
        attention: attention_json
            .as_deref()
            .map(serde_json::from_str::<crate::model::AttentionReason>)
            .transpose()
            .ok()
            .flatten(),
        // Same degrade rule for the observed running model (AJ-E412):
        // never a row-breaking value, never guessed back.
        running: running_json
            .as_deref()
            .map(serde_json::from_str::<crate::model::RunningModel>)
            .transpose()
            .ok()
            .flatten(),
        // Same degrade rule for the window identity (AJ-E414): advisory
        // targeting state, never a row-breaking value.
        window: window_json
            .as_deref()
            .map(serde_json::from_str::<WindowIdentity>)
            .transpose()
            .ok()
            .flatten(),
    })
}
