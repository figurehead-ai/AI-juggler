//! Account repository.

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::CoreError;
use crate::model::{Account, AccountId, HarnessId, PlanTier};

/// Typed read/write access to the `accounts` table.
#[derive(Debug)]
pub struct AccountsRepo<'a> {
    conn: &'a Connection,
}

impl AccountsRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> AccountsRepo<'_> {
        AccountsRepo { conn }
    }

    /// Inserts a new account.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on I/O or constraint violations
    /// (e.g. a duplicate id).
    pub fn insert(&self, account: &Account) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO accounts (id, provider_id, label, plan, state_dir, created_at, cleanup_opt_in, renewal_date, preview_opt_in, import_source_path, import_origin, imported_at, in_place)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                account.id.as_str(),
                account.harness.as_str(),
                account.label,
                account.plan.as_ref().map(PlanTier::as_str),
                account.state_dir.to_string_lossy(),
                crate::model::Timestamp::now().millis(),
                i64::from(account.cleanup_opt_in),
                account.renewal_date,
                i64::from(account.preview_opt_in),
                account.import_source_path,
                account.import_origin,
                account.imported_at,
                i64::from(account.in_place),
            ],
        )?;
        Ok(())
    }

    /// Updates an existing account by id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the id does not exist, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn update(&self, account: &Account) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE accounts SET provider_id = ?2, label = ?3, plan = ?4, state_dir = ?5,
             cleanup_opt_in = ?6, renewal_date = ?7, preview_opt_in = ?8,
             import_source_path = ?9, import_origin = ?10, imported_at = ?11, in_place = ?12
             WHERE id = ?1",
            params![
                account.id.as_str(),
                account.harness.as_str(),
                account.label,
                account.plan.as_ref().map(PlanTier::as_str),
                account.state_dir.to_string_lossy(),
                i64::from(account.cleanup_opt_in),
                account.renewal_date,
                i64::from(account.preview_opt_in),
                account.import_source_path,
                account.import_origin,
                account.imported_at,
                i64::from(account.in_place),
            ],
        )?;
        require_changed(changed, "account", account.id.as_str())
    }

    /// Fetches one account by id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn get(&self, id: &AccountId) -> Result<Account, CoreError> {
        self.conn
            .query_row(
                "SELECT * FROM accounts WHERE id = ?1",
                params![id.as_str()],
                map_account,
            )
            .optional()?
            .ok_or_else(|| not_found("account", id.as_str()))
    }

    /// Lists the accounts in use, oldest first. A removed account
    /// (AJ-E1501) is not among them; [`Self::get`] still finds it, so its
    /// sessions' history keeps a name.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn list(&self) -> Result<Vec<Account>, CoreError> {
        self.query("SELECT * FROM accounts WHERE removed_at IS NULL ORDER BY created_at, id")
    }

    /// Lists every account, removed ones too, oldest first — for what
    /// must cover all of them (history joins, a backup's redaction).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn list_all(&self) -> Result<Vec<Account>, CoreError> {
        self.query("SELECT * FROM accounts ORDER BY created_at, id")
    }

    /// The ids of the removed accounts.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn removed_ids(&self) -> Result<std::collections::BTreeSet<String>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM accounts WHERE removed_at IS NOT NULL")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut out = std::collections::BTreeSet::new();
        for row in rows {
            out.insert(row?);
        }
        Ok(out)
    }

    /// Whether an account was removed.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn is_removed(&self, id: &AccountId) -> Result<bool, CoreError> {
        self.conn
            .query_row(
                "SELECT removed_at IS NOT NULL FROM accounts WHERE id = ?1",
                params![id.as_str()],
                |row| row.get::<_, bool>(0),
            )
            .optional()?
            .ok_or_else(|| not_found("account", id.as_str()))
    }

    /// Forgets an account (AJ-E1501): it leaves [`Self::list`], its
    /// launch settings and sessions stay, and nothing on disk is touched.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn mark_removed(&self, id: &AccountId) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE accounts SET removed_at = ?2 WHERE id = ?1 AND removed_at IS NULL",
            params![id.as_str(), crate::model::Timestamp::now().millis()],
        )?;
        require_changed(changed, "account", id.as_str())
    }

    fn query(&self, sql: &str) -> Result<Vec<Account>, CoreError> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], map_account)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Deletes an account by id (cascades to profiles and sessions).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn delete(&self, id: &AccountId) -> Result<(), CoreError> {
        let changed = self
            .conn
            .execute("DELETE FROM accounts WHERE id = ?1", params![id.as_str()])?;
        require_changed(changed, "account", id.as_str())
    }
}

/// Builds [`CoreError::NotFound`] for an entity id.
pub(super) fn not_found(entity: &'static str, id: &str) -> CoreError {
    CoreError::NotFound {
        entity,
        id: id.to_owned(),
    }
}

/// Maps `changed == 0` to [`CoreError::NotFound`].
pub(super) fn require_changed(
    changed: usize,
    entity: &'static str,
    id: &str,
) -> Result<(), CoreError> {
    if changed == 0 {
        Err(not_found(entity, id))
    } else {
        Ok(())
    }
}

/// Maps a row to an [`Account`].
fn map_account(row: &Row<'_>) -> rusqlite::Result<Account> {
    Ok(Account {
        id: AccountId::new(row.get::<_, String>("id")?),
        harness: HarnessId::new(row.get::<_, String>("provider_id")?),
        label: row.get("label")?,
        plan: row.get::<_, Option<String>>("plan")?.map(PlanTier::new),
        state_dir: row.get::<_, String>("state_dir")?.into(),
        cleanup_opt_in: row.get::<_, i64>("cleanup_opt_in")? != 0,
        renewal_date: row.get("renewal_date")?,
        preview_opt_in: row.get::<_, i64>("preview_opt_in")? != 0,
        import_source_path: row.get("import_source_path")?,
        import_origin: row.get("import_origin")?,
        imported_at: row.get("imported_at")?,
        in_place: row.get::<_, i64>("in_place")? != 0,
    })
}
