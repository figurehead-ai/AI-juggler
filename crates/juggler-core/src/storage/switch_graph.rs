//! Switch Graph repository (AJ-E201; pass2/02 §2).
//!
//! Work items, instance-epochs, and append-only switch edges.
//! Invariant S-G1 is enforced structurally: the API exposes INSERTs
//! plus exactly three state transitions on an edge (launch fill,
//! confirm, fail) — there is no update or delete for epochs, edges,
//! or works, and no path rewrites `created_at`, lineage, or
//! mechanism. Undo and merge insert compensating rows.
//!
//! Invariant S-G2 is structural too: every edge and epoch hangs off
//! a `work_items` row that references exactly one project, so a
//! cross-project "edge" cannot be expressed (cross-project moves are
//! T4 pack exports, never edges — `cwd` is baked into every native
//! format).

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::CoreError;
use crate::model::{
    AccountId, EdgeActor, EdgeState, Epoch, EpochId, FailureStage, HarnessId, ManifestEntry,
    PackId, ProjectId, SessionId, SwitchEdge, SwitchEdgeId, SwitchMechanism, Timestamp, WorkItem,
    WorkItemId,
};

/// Typed append-only access to the Switch Graph tables (schema v20).
#[derive(Debug)]
pub struct SwitchGraphRepo<'a> {
    conn: &'a Connection,
}

impl SwitchGraphRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> SwitchGraphRepo<'_> {
        SwitchGraphRepo { conn }
    }

    /// Creates a new Work item under one project (Invariant S-G2:
    /// the graph is per-project from birth).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn create_work(&self, project: &ProjectId, title: &str) -> Result<WorkItem, CoreError> {
        let now = Timestamp::now();
        let record = WorkItem {
            id: WorkItemId::new(format!("work-{}", time_suffix(now))),
            project: project.clone(),
            title: title.to_owned(),
            created_at: now,
        };
        self.conn.execute(
            "INSERT INTO work_items (id, project, title, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                record.id.as_str(),
                record.project.as_str(),
                record.title,
                record.created_at.millis()
            ],
        )?;
        Ok(record)
    }

    /// Fetches a work item by id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn work(&self, id: &WorkItemId) -> Result<WorkItem, CoreError> {
        self.conn
            .query_row(
                "SELECT * FROM work_items WHERE id = ?1",
                params![id.as_str()],
                map_work,
            )
            .optional()?
            .ok_or_else(|| super::accounts::not_found("work item", id.as_str()))
    }

    /// Lists a project's work items, newest first.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn works_for_project(&self, project: &ProjectId) -> Result<Vec<WorkItem>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM work_items WHERE project = ?1
             ORDER BY created_at DESC, id",
        )?;
        let rows = stmt.query_map(params![project.as_str()], map_work)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Appends an epoch node (INSERT only — epochs are immutable).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures (including
    /// foreign-key violations for a missing work item).
    pub fn insert_epoch(&self, epoch: &Epoch) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO epochs (id, work_id, session_ref, native_id, account, provider, resumed_from_epoch)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                epoch.id.as_str(),
                epoch.work.as_str(),
                epoch.session_ref.as_ref().map(SessionId::as_str),
                epoch.native_id,
                epoch.account.as_str(),
                epoch.provider.as_str(),
                epoch.resumed_from_epoch.as_ref().map(EpochId::as_str),
            ],
        )?;
        Ok(())
    }

    /// Appends a switch edge in a pre-launch state (`Staged` or
    /// `Failed`); `to_epoch` is filled later by
    /// [`Self::mark_launched`] (INSERT only — edges are append-only,
    /// Invariant S-G1).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures (including
    /// foreign-key violations for missing epochs/works/parents), or
    /// [`CoreError::Storage`] with a constraint message when the
    /// edge carries a `to_epoch` or a non-staged state — use the
    /// state transitions for those.
    pub fn append_edge(&self, edge: &SwitchEdge) -> Result<(), CoreError> {
        if edge.to_epoch.is_some() {
            return Err(CoreError::Storage(rusqlite::Error::ToSqlConversionFailure(
                box_error("to_epoch is filled by mark_launched, never at append"),
            )));
        }
        if !matches!(edge.state, EdgeState::Staged | EdgeState::Failed { .. }) {
            return Err(CoreError::Storage(rusqlite::Error::ToSqlConversionFailure(
                box_error("edges append as Staged or Failed; use transitions after"),
            )));
        }
        let (fail_stage, fail_reason) = match &edge.state {
            EdgeState::Failed { stage, reason } => (Some(stage.as_str()), Some(reason.as_str())),
            _ => (None, None),
        };
        self.conn.execute(
            "INSERT INTO switch_edges (
                id, work_id, from_epoch, to_epoch,
                from_account, to_account, from_provider, to_provider,
                mechanism, pack_id, fidelity_shown, manifest_sha256,
                state, fail_stage, fail_reason, actor,
                parent_edge_a, parent_edge_b, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                edge.id.as_str(),
                edge.work.as_str(),
                edge.from_epoch.as_str(),
                edge.to_epoch.as_ref().map(EpochId::as_str),
                edge.from_account.as_str(),
                edge.to_account.as_str(),
                edge.from_provider.as_str(),
                edge.to_provider.as_str(),
                edge.mechanism.as_str(),
                edge.pack_id.as_ref().map(PackId::as_str),
                edge.fidelity_shown,
                edge.manifest_sha256,
                edge.state.as_str(),
                fail_stage,
                fail_reason,
                edge.actor.as_str(),
                edge.parent_edge_a.as_ref().map(SwitchEdgeId::as_str),
                edge.parent_edge_b.as_ref().map(SwitchEdgeId::as_str),
                edge.created_at.millis(),
            ],
        )?;
        Ok(())
    }

    /// State transition: launch succeeded — fills `to_epoch` and
    /// moves the edge to `Launched`. The ONLY write that touches
    /// `to_epoch` (Invariant S-G1: a state transition within one
    /// edge, never a rewrite).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the edge is missing or
    /// [`CoreError::Storage`] on SQLite failures (including the
    /// FK check when `to_epoch` does not exist).
    pub fn mark_launched(&self, id: &SwitchEdgeId, to_epoch: &EpochId) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE switch_edges SET state = 'launched', to_epoch = ?2
             WHERE id = ?1 AND state IN ('staged', 'verified')",
            params![id.as_str(), to_epoch.as_str()],
        )?;
        if changed == 0 {
            // Distinguish missing edge from an illegal transition.
            self.edge(id)?;
            return Err(CoreError::Storage(rusqlite::Error::ToSqlConversionFailure(
                box_error("edge is not in a launchable state"),
            )));
        }
        Ok(())
    }

    /// State transition: the target confirmed alive — `Launched` →
    /// `Confirmed`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the edge is missing or
    /// [`CoreError::Storage`] on SQLite failures / illegal
    /// transitions.
    pub fn mark_confirmed(&self, id: &SwitchEdgeId) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE switch_edges SET state = 'confirmed' WHERE id = ?1 AND state = 'launched'",
            params![id.as_str()],
        )?;
        if changed == 0 {
            self.edge(id)?;
            return Err(CoreError::Storage(rusqlite::Error::ToSqlConversionFailure(
                box_error("edge is not launched"),
            )));
        }
        Ok(())
    }

    /// State transition: the switch failed at `stage` — records the
    /// failure on the SAME edge (a failed edge is history, not
    /// something to rewrite; a retry appends a NEW edge).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the edge is missing or
    /// [`CoreError::Storage`] on SQLite failures / illegal
    /// transitions (a terminal edge cannot fail again).
    pub fn mark_failed(
        &self,
        id: &SwitchEdgeId,
        stage: FailureStage,
        reason: &str,
    ) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE switch_edges
             SET state = 'failed', fail_stage = ?2, fail_reason = ?3
             WHERE id = ?1 AND state IN ('staged', 'verified', 'launched')",
            params![id.as_str(), stage.as_str(), reason],
        )?;
        if changed == 0 {
            self.edge(id)?;
            return Err(CoreError::Storage(rusqlite::Error::ToSqlConversionFailure(
                box_error("edge is not in a failable state"),
            )));
        }
        Ok(())
    }

    /// Fetches an edge by id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing or
    /// [`CoreError::Storage`] on SQLite failures / corrupt enum
    /// values.
    pub fn edge(&self, id: &SwitchEdgeId) -> Result<SwitchEdge, CoreError> {
        self.conn
            .query_row(
                "SELECT * FROM switch_edges WHERE id = ?1",
                params![id.as_str()],
                map_edge,
            )
            .optional()?
            .ok_or_else(|| super::accounts::not_found("switch edge", id.as_str()))
    }

    /// All epochs of one work item (one query).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn epochs_for_work(&self, work: &WorkItemId) -> Result<Vec<Epoch>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM epochs WHERE work_id = ?1 ORDER BY rowid")?;
        let rows = stmt.query_map(params![work.as_str()], map_epoch)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// All edges of one work item in insertion order (one query —
    /// no N+1; the graph read is two queries plus this).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn edges_for_work(&self, work: &WorkItemId) -> Result<Vec<SwitchEdge>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM switch_edges WHERE work_id = ?1 ORDER BY rowid")?;
        let rows = stmt.query_map(params![work.as_str()], map_edge)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Appends an edge's artifact manifest rows (INSERT only).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures (including
    /// FK violations for a missing edge).
    pub fn append_manifest(
        &self,
        edge: &SwitchEdgeId,
        entries: &[ManifestEntry],
    ) -> Result<(), CoreError> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO artifact_manifests (edge, rel_path, sha256, bytes, kind, disposition)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for entry in entries {
            stmt.execute(params![
                edge.as_str(),
                entry.rel_path,
                entry.sha256,
                i64::try_from(entry.bytes).unwrap_or(i64::MAX),
                entry.kind,
                entry.disposition,
            ])?;
        }
        Ok(())
    }

    /// Reads an edge's manifest rows (insertion order).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn manifest_for(&self, edge: &SwitchEdgeId) -> Result<Vec<ManifestEntry>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT rel_path, sha256, bytes, kind, disposition
             FROM artifact_manifests WHERE edge = ?1 ORDER BY rowid",
        )?;
        let rows = stmt.query_map(params![edge.as_str()], |row| {
            Ok(ManifestEntry {
                rel_path: row.get("rel_path")?,
                sha256: row.get("sha256")?,
                bytes: row.get::<_, i64>("bytes")?.cast_unsigned(),
                kind: row.get("kind")?,
                disposition: row.get("disposition")?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

/// The whole graph for one work item — the E209 read shape. Two
/// queries, no N+1.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkGraph {
    /// The work item.
    pub work: WorkItem,
    /// Every epoch node, in insertion order.
    pub epochs: Vec<Epoch>,
    /// Every edge, in insertion order.
    pub edges: Vec<SwitchEdge>,
}

impl SwitchGraphRepo<'_> {
    /// Reads one work item's full graph (two queries).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] for a missing work item or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn graph(&self, work: &WorkItemId) -> Result<WorkGraph, CoreError> {
        Ok(WorkGraph {
            work: self.work(work)?,
            epochs: self.epochs_for_work(work)?,
            edges: self.edges_for_work(work)?,
        })
    }
}

fn map_work(row: &Row<'_>) -> rusqlite::Result<WorkItem> {
    let id: String = row.get("id")?;
    let project: String = row.get("project")?;
    Ok(WorkItem {
        id: WorkItemId::new(id),
        project: ProjectId::new(project),
        title: row.get("title")?,
        created_at: Timestamp::from_millis(row.get("created_at")?),
    })
}

fn map_epoch(row: &Row<'_>) -> rusqlite::Result<Epoch> {
    let id: String = row.get("id")?;
    let work: String = row.get("work_id")?;
    let account: String = row.get("account")?;
    let provider: String = row.get("provider")?;
    Ok(Epoch {
        id: EpochId::new(id),
        work: WorkItemId::new(work),
        session_ref: row
            .get::<_, Option<String>>("session_ref")?
            .map(SessionId::new),
        native_id: row.get("native_id")?,
        account: AccountId::new(account),
        provider: HarnessId::new(provider),
        resumed_from_epoch: row
            .get::<_, Option<String>>("resumed_from_epoch")?
            .map(EpochId::new),
    })
}

fn map_edge(row: &Row<'_>) -> rusqlite::Result<SwitchEdge> {
    let state_raw: String = row.get("state")?;
    let state = if state_raw == "failed" {
        let stored_stage: String = row
            .get::<_, Option<String>>("fail_stage")?
            .unwrap_or_else(|| "stage".to_owned());
        EdgeState::Failed {
            stage: FailureStage::parse(&stored_stage).unwrap_or(FailureStage::Stage),
            reason: row
                .get::<_, Option<String>>("fail_reason")?
                .unwrap_or_default(),
        }
    } else {
        // Degrade-never-panic: an unknown state string reads as the
        // earliest non-failure state (the write path is typed, so
        // this only guards against hand-edited databases).
        match state_raw.as_str() {
            "verified" => EdgeState::Verified,
            "launched" => EdgeState::Launched,
            "confirmed" => EdgeState::Confirmed,
            _ => EdgeState::Staged,
        }
    };
    let mechanism_raw: String = row.get("mechanism")?;
    let actor_raw: String = row.get("actor")?;
    let id: String = row.get("id")?;
    let work: String = row.get("work_id")?;
    let from_epoch: String = row.get("from_epoch")?;
    let from_account: String = row.get("from_account")?;
    let to_account: String = row.get("to_account")?;
    let from_provider: String = row.get("from_provider")?;
    let to_provider: String = row.get("to_provider")?;
    Ok(SwitchEdge {
        id: SwitchEdgeId::new(id),
        work: WorkItemId::new(work),
        from_epoch: EpochId::new(from_epoch),
        to_epoch: row.get::<_, Option<String>>("to_epoch")?.map(EpochId::new),
        from_account: AccountId::new(from_account),
        to_account: AccountId::new(to_account),
        from_provider: HarnessId::new(from_provider),
        to_provider: HarnessId::new(to_provider),
        mechanism: SwitchMechanism::parse(&mechanism_raw).unwrap_or(SwitchMechanism::Pack),
        pack_id: row.get::<_, Option<String>>("pack_id")?.map(PackId::new),
        fidelity_shown: row.get("fidelity_shown")?,
        manifest_sha256: row.get("manifest_sha256")?,
        state,
        actor: EdgeActor::parse(&actor_raw).unwrap_or(EdgeActor::User),
        parent_edge_a: row
            .get::<_, Option<String>>("parent_edge_a")?
            .map(SwitchEdgeId::new),
        parent_edge_b: row
            .get::<_, Option<String>>("parent_edge_b")?
            .map(SwitchEdgeId::new),
        created_at: Timestamp::from_millis(row.get("created_at")?),
    })
}

/// Boxed rusqlite error for the API-level append-only guardrails.
fn box_error(message: &str) -> Box<dyn std::error::Error + Send + Sync> {
    Box::<dyn std::error::Error + Send + Sync>::from(message.to_owned())
}

/// Time-based id suffix with a same-millisecond disambiguator (a
/// burst of epochs/edges can share a millisecond).
fn time_suffix(now: Timestamp) -> u128 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = u128::from(COUNTER.fetch_add(1, Ordering::Relaxed));
    let millis = u128::from(now.millis().max(0).cast_unsigned()) % 1_000_000_000_000;
    millis * 1_000 + (seq % 1_000)
}
