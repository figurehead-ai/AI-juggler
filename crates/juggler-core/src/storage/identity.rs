//! AJ-E1001: the per-Instance identity assertions — who an Instance ran
//! as, checked at launch, resume and switch-back (schema v24). Append-only:
//! the history of where work ran is never rewritten.

use std::collections::HashMap;

use rusqlite::{Connection, params};

use crate::error::CoreError;

/// One recorded assertion.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IdentityAssertion {
    /// The Instance (session id).
    pub session_id: String,
    /// The Account it was launched under.
    pub account_id: String,
    /// `verified` | `first_seen` | `signed_out` | `unverified`.
    pub state: String,
    /// The masked label detected (`o…@example.com · Acme`), when any.
    pub label: Option<String>,
    /// When, epoch ms.
    pub at_ms: i64,
    /// `launch` | `resume` | `switch` | `switch_back` | `handover`.
    pub moment: String,
}

/// Repository over `identity_assertions`.
pub struct IdentityRepo<'a> {
    conn: &'a Connection,
}

impl IdentityRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> IdentityRepo<'_> {
        IdentityRepo { conn }
    }

    /// Appends one assertion.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn record(&self, assertion: &IdentityAssertion) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO identity_assertions (session_id, account_id, state, label, at_ms, moment)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                assertion.session_id,
                assertion.account_id,
                assertion.state,
                assertion.label,
                assertion.at_ms,
                assertion.moment
            ],
        )?;
        Ok(())
    }

    /// The latest assertion per Instance.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn latest_by_session(&self) -> Result<HashMap<String, IdentityAssertion>, CoreError> {
        let mut statement = self.conn.prepare(
            "SELECT session_id, account_id, state, label, at_ms, moment FROM identity_assertions
             WHERE rowid IN (SELECT MAX(rowid) FROM identity_assertions GROUP BY session_id)",
        )?;
        let rows = statement.query_map([], map_row)?;
        let mut out = HashMap::new();
        for row in rows {
            let row = row?;
            out.insert(row.session_id.clone(), row);
        }
        Ok(out)
    }

    /// Every assertion for one Instance, oldest first (its history).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn history(&self, session_id: &str) -> Result<Vec<IdentityAssertion>, CoreError> {
        let mut statement = self.conn.prepare(
            "SELECT session_id, account_id, state, label, at_ms, moment FROM identity_assertions
             WHERE session_id = ?1 ORDER BY rowid",
        )?;
        let rows = statement.query_map(params![session_id], map_row)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IdentityAssertion> {
    Ok(IdentityAssertion {
        session_id: row.get(0)?,
        account_id: row.get(1)?,
        state: row.get(2)?,
        label: row.get(3)?,
        at_ms: row.get(4)?,
        moment: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    fn assertion(session: &str, state: &str, at: i64) -> IdentityAssertion {
        IdentityAssertion {
            session_id: session.into(),
            account_id: "acc-1".into(),
            state: state.into(),
            label: Some("o…@example.com".into()),
            at_ms: at,
            moment: "launch".into(),
        }
    }

    #[test]
    fn history_is_append_only_and_latest_is_per_instance() {
        let storage = Storage::open_in_memory().unwrap();
        let repo = storage.identity();
        repo.record(&assertion("s1", "first_seen", 1)).unwrap();
        repo.record(&assertion("s1", "verified", 2)).unwrap();
        repo.record(&assertion("s2", "unverified", 3)).unwrap();
        let latest = repo.latest_by_session().unwrap();
        assert_eq!(latest["s1"].state, "verified");
        assert_eq!(latest["s2"].state, "unverified");
        assert_eq!(repo.history("s1").unwrap().len(), 2);
    }
}
