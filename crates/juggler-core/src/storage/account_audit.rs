//! Account audit rows (AJ-E1501 Req.5): who changed an account, what,
//! and when. Append-only; nothing here is ever updated or deleted.

use rusqlite::{Connection, params};

use crate::error::CoreError;
use crate::model::{AccountId, Timestamp};

/// One recorded change to an account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountAuditRecord {
    /// The account changed.
    pub account_id: AccountId,
    /// What was done: `rename`, `plan`, `remove`, `default`, …
    pub action: String,
    /// The change in a few words (old → new, the folder choice).
    pub detail: Option<String>,
    /// Who did it (`user` for a change made in the app).
    pub actor: String,
    /// When, epoch ms.
    pub at: i64,
}

/// Typed access to the `account_audit` table.
#[derive(Debug)]
pub struct AccountAuditRepo<'a> {
    conn: &'a Connection,
}

impl AccountAuditRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> AccountAuditRepo<'_> {
        AccountAuditRepo { conn }
    }

    /// Appends one row, stamped now.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn record(
        &self,
        account: &AccountId,
        action: &str,
        detail: Option<&str>,
        actor: &str,
    ) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO account_audit (account_id, action, detail, actor, at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                account.as_str(),
                action,
                detail,
                actor,
                Timestamp::now().millis()
            ],
        )?;
        Ok(())
    }

    /// An account's rows, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn list_for(&self, account: &AccountId) -> Result<Vec<AccountAuditRecord>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT account_id, action, detail, actor, at FROM account_audit
             WHERE account_id = ?1 ORDER BY at, id",
        )?;
        let rows = stmt.query_map(params![account.as_str()], |row| {
            Ok(AccountAuditRecord {
                account_id: AccountId::new(row.get::<_, String>(0)?),
                action: row.get(1)?,
                detail: row.get(2)?,
                actor: row.get(3)?,
                at: row.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}
