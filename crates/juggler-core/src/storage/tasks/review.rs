//! Reviews: the user's own four actions on a task (D-44), and since
//! D-62 (AJ-E3005 req. 2) a reviewer's verdict on a task in review.

use rusqlite::params;

use super::{Task, TasksRepo, invalid, log_in};
use crate::error::CoreError;
use crate::model::Timestamp;
use crate::tasks::{TaskStatus, Verdict};

impl TasksRepo<'_> {
    /// D-62: a reviewer's verdict on a task in review: approved keeps it
    /// in review for the person's Approve (D-44) with the reviewer's word;
    /// sent back returns it to its holder (or the pool) with the note.
    ///
    /// # Errors
    ///
    /// [`CoreError::Validation`] for a task not in review, or sent back
    /// with no note; [`CoreError::NotFound`]; storage failures.
    pub fn review(
        &self,
        id: &str,
        verdict: Verdict,
        note: Option<&str>,
        actor: (&str, &str),
    ) -> Result<Task, CoreError> {
        let now = Timestamp::now().millis();
        let task = self
            .get(id)?
            .ok_or_else(|| super::super::accounts::not_found("task", id))?;
        if task.stored != TaskStatus::InReview {
            return Err(invalid("only a task in review gets a verdict"));
        }
        let note = note.map(str::trim).filter(|n| !n.is_empty());
        let tx = crate::storage::write_transaction(self.conn)?;
        if verdict == Verdict::SentBack {
            let note = note.ok_or_else(|| invalid("say what should change: a short note"))?;
            let to = if task.assignee_ref.is_some() {
                TaskStatus::InProgress
            } else {
                TaskStatus::Ready
            };
            tx.execute(
                "UPDATE tasks SET status = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, to.as_str(), now],
            )?;
            log_in(
                &tx,
                now,
                &task.pool_id,
                id,
                actor.0,
                actor.1,
                verdict.verb(),
                Some(note),
            )?;
        } else {
            tx.execute(
                "UPDATE tasks SET updated_at = ?2 WHERE id = ?1",
                params![id, now],
            )?;
            log_in(
                &tx,
                now,
                &task.pool_id,
                id,
                actor.0,
                actor.1,
                verdict.verb(),
                note,
            )?;
        }
        tx.commit()?;
        self.get(id)?
            .ok_or_else(|| super::super::accounts::not_found("task", id))
    }

    /// The user's action on a task (D-44), checked against the task as it
    /// is now ([`crate::tasks::may_user`]) and logged as one event, in one
    /// transaction:
    ///
    /// - **Approve**: in review → done.
    /// - **Send back** (`note` needed): in review → in progress, its holder
    ///   kept (ready, when no one holds it).
    /// - **Free**: its holder cleared, only while it's still `seen_holder`
    ///   (the holder whose sessions the caller found not running); in
    ///   progress goes back to ready, any other status stays.
    /// - **Cancel**: open → cancelled.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a task that isn't there;
    /// [`CoreError::Validation`] with the rule, in words, when the task
    /// isn't in a state for it (or its holder changed); storage failures.
    pub fn user_action(
        &self,
        id: &str,
        action: crate::tasks::UserAction,
        note: Option<&str>,
        seen_holder: Option<&str>,
        holder_running: bool,
        actor: (&str, &str),
    ) -> Result<Task, CoreError> {
        use crate::tasks::UserAction;
        let now = Timestamp::now().millis();
        let task = self
            .get(id)?
            .ok_or_else(|| super::super::accounts::not_found("task", id))?;
        crate::tasks::may_user(
            action,
            task.stored,
            task.assignee_ref.is_some(),
            holder_running,
        )
        .map_err(invalid)?;
        let note = note.map(str::trim).filter(|n| !n.is_empty());
        let tx = crate::storage::write_transaction(self.conn)?;
        let (verb, detail) = match action {
            UserAction::Approve | UserAction::Cancel => {
                let to = if action == UserAction::Approve {
                    TaskStatus::Done
                } else {
                    TaskStatus::Cancelled
                };
                tx.execute(
                    "UPDATE tasks SET status = ?2, closed_at = ?3, updated_at = ?3 WHERE id = ?1",
                    params![id, to.as_str(), now],
                )?;
                (action.verb(), note.map(str::to_owned))
            }
            UserAction::SendBack => {
                let note = note.ok_or_else(|| invalid("say what should change: a short note"))?;
                let to = if task.assignee_ref.is_some() {
                    TaskStatus::InProgress
                } else {
                    TaskStatus::Ready
                };
                tx.execute(
                    "UPDATE tasks SET status = ?2, closed_at = NULL, updated_at = ?3 WHERE id = ?1",
                    params![id, to.as_str(), now],
                )?;
                (action.verb(), Some(note.to_owned()))
            }
            UserAction::Free => {
                if task.assignee_ref.as_deref() != seen_holder {
                    return Err(invalid("someone else holds it now: look again"));
                }
                tx.execute(
                    "UPDATE tasks SET assignee_ref = NULL, assignee_name = NULL, updated_at = ?2,
                            status = CASE WHEN status = 'in_progress' THEN 'ready' ELSE status END
                      WHERE id = ?1",
                    params![id, now],
                )?;
                (action.verb(), task.assignee_name.clone())
            }
        };
        log_in(
            &tx,
            now,
            &task.pool_id,
            id,
            actor.0,
            actor.1,
            verb,
            detail.as_deref(),
        )?;
        tx.commit()?;
        self.get(id)?
            .ok_or_else(|| super::super::accounts::not_found("task", id))
    }
}
