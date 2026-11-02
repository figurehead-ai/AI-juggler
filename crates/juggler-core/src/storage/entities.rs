//! D-62 (AJ-E3202, schema v40): the Account and the App, stored.
//!
//! - An **Account** (`subscriptions`) is a subscription: keyed by its
//!   vendor and a hash of the sign-in (subject and organisation), never
//!   the raw ids, so one plan never splits across tools or folders. Its
//!   name is the user's; plan, renewal and fee are set on it once.
//! - An **App** (`apps`) is one app in one of its profiles: a CLI tool on a
//!   config folder (`kind = cli`, `folder_id` → `accounts.id`), or a
//!   desktop app in one of its instances (`kind = desktop`, `host_app_id`
//!   and `instance_dir`). It's remembered when it isn't running; Keep
//!   alive and what Open starts it with live on it.
//! - Additive: `accounts` (a config folder, in the code still) gains the
//!   Account its sign-in was last read as; `instances` gains its App. Which
//!   sessions an App has stays read from the tools' own files (D-37); these
//!   rows hold identity, names and settings only.
//!
//! Ids are made from what a row stands for (`cli:<folder>`,
//! `app:<host>:<instance>`, `acct:<vendor>:<hash prefix>`), so a re-read
//! never makes a second row.

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::CoreError;

/// v40: the two tables, their references, and one CLI App per folder row
/// that's still in use (a removed folder's App isn't made).
pub(super) const MIGRATION: &str = "
CREATE TABLE subscriptions (
    id            TEXT PRIMARY KEY,
    vendor        TEXT NOT NULL,
    identity_hash TEXT NOT NULL,
    name          TEXT,
    plan          TEXT,
    renewal_date  TEXT,
    fee_usd       REAL,
    created_at    INTEGER NOT NULL,
    removed_at    INTEGER,
    UNIQUE (vendor, identity_hash)
);
CREATE TABLE apps (
    id           TEXT PRIMARY KEY,
    kind         TEXT NOT NULL CHECK (kind IN ('cli', 'desktop')),
    tool         TEXT NOT NULL,
    host_app_id  TEXT,
    instance_dir TEXT,
    folder_id    TEXT REFERENCES accounts(id),
    label        TEXT,
    hidden       INTEGER NOT NULL DEFAULT 0,
    keep_alive   INTEGER NOT NULL DEFAULT 0,
    paused_json  TEXT,
    launch_json  TEXT,
    sessions_json TEXT,
    first_seen   INTEGER NOT NULL,
    last_seen    INTEGER NOT NULL
);
CREATE UNIQUE INDEX apps_cli ON apps(tool, folder_id) WHERE kind = 'cli';
CREATE UNIQUE INDEX apps_desktop ON apps(host_app_id, IFNULL(instance_dir, '')) WHERE kind = 'desktop';
ALTER TABLE accounts ADD COLUMN subscription_id TEXT REFERENCES subscriptions(id);
ALTER TABLE instances ADD COLUMN app_id TEXT REFERENCES apps(id);
INSERT INTO apps (id, kind, tool, folder_id, first_seen, last_seen)
    SELECT 'cli:' || id, 'cli', provider_id, id, created_at, created_at
    FROM accounts WHERE removed_at IS NULL;
UPDATE instances SET app_id = 'cli:' || (
    SELECT account_id FROM launch_recipes WHERE launch_recipes.id = instances.launch_recipe_id
) WHERE EXISTS (
    SELECT 1 FROM apps WHERE apps.id = 'cli:' || (
        SELECT account_id FROM launch_recipes WHERE launch_recipes.id = instances.launch_recipe_id
    )
);
";

/// A stored Account (a subscription).
#[derive(Clone, Debug, PartialEq)]
pub struct Subscription {
    /// `acct:<vendor>:<hash prefix>`.
    pub id: String,
    /// Whose sign-in: `anthropic`, `openai`, or a service's id.
    pub vendor: String,
    /// sha256 of vendor, subject and organisation (hex); never the ids.
    pub identity_hash: String,
    /// The user's name for it (`None`: the default, worked out on read).
    pub name: Option<String>,
    /// Its plan, set once.
    pub plan: Option<String>,
    /// Its renewal date (`YYYY-MM-DD`), set once.
    pub renewal_date: Option<String>,
    /// Its monthly fee, USD, set once.
    pub fee_usd: Option<f64>,
    /// When it was first seen, epoch ms.
    pub created_at: i64,
    /// When it was removed (no folder signed in to it is kept), epoch ms.
    pub removed_at: Option<i64>,
}

/// A stored App.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct App {
    /// `cli:<folder>` or `app:<host app>:<instance>`.
    pub id: String,
    /// `cli` or `desktop`.
    pub kind: String,
    /// Its tool (`claude-code`, `codex`).
    pub tool: String,
    /// A desktop App's host registry id (`claude-app`).
    pub host_app_id: Option<String>,
    /// A desktop App's instance folder (`None`: its default one).
    pub instance_dir: Option<String>,
    /// The config folder it uses (a CLI App's own; a desktop App's once
    /// learned from its sessions).
    pub folder_id: Option<String>,
    /// The user's name for it (`None`: the default).
    pub label: Option<String>,
    /// Left out of lists by the user.
    pub hidden: bool,
    /// Keep alive (0.3.3, D-63): started again when it stops.
    pub keep_alive: bool,
    /// Why Keep alive is paused (JSON), when it is.
    pub paused_json: Option<String>,
    /// How Open starts it (JSON: bundle, executable, the folder it was
    /// given), as it last ran.
    pub launch_json: Option<String>,
    /// A desktop App's sessions seen open in it (JSON, `<tool>/<id>`, the
    /// newest last): the only way to know a desktop app's sessions.
    pub sessions_json: Option<String>,
    /// First seen, epoch ms.
    pub first_seen: i64,
    /// Last seen running, epoch ms.
    pub last_seen: i64,
}

/// The id of tool `tool`'s CLI App on folder `folder_id`.
#[must_use]
pub fn cli_app_id(folder_id: &str) -> String {
    format!("cli:{folder_id}")
}

/// The id of host app `host_app_id`'s App in `instance_dir` (`None`: its
/// default instance).
#[must_use]
pub fn desktop_app_id(host_app_id: &str, instance_dir: Option<&str>) -> String {
    format!("app:{host_app_id}:{}", instance_dir.unwrap_or(""))
}

/// The id of `vendor`'s Account with sign-in hash `identity_hash`.
#[must_use]
pub fn subscription_id(vendor: &str, identity_hash: &str) -> String {
    let prefix: String = identity_hash.chars().take(16).collect();
    format!("acct:{vendor}:{prefix}")
}

fn collect<T>(rows: impl Iterator<Item = rusqlite::Result<T>>) -> Result<Vec<T>, CoreError> {
    rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
}

const SUBSCRIPTION_COLUMNS: &str =
    "id, vendor, identity_hash, name, plan, renewal_date, fee_usd, created_at, removed_at";

fn subscription_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Subscription> {
    Ok(Subscription {
        id: row.get(0)?,
        vendor: row.get(1)?,
        identity_hash: row.get(2)?,
        name: row.get(3)?,
        plan: row.get(4)?,
        renewal_date: row.get(5)?,
        fee_usd: row.get(6)?,
        created_at: row.get(7)?,
        removed_at: row.get(8)?,
    })
}

/// The stored Accounts.
pub struct SubscriptionsRepo<'a> {
    conn: &'a Connection,
}

impl<'a> SubscriptionsRepo<'a> {
    pub(super) const fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// The Account for `vendor`'s sign-in `identity_hash`, made the first
    /// time it's seen (and brought back if it was removed). Its id.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn upsert(
        &self,
        vendor: &str,
        identity_hash: &str,
        now_ms: i64,
    ) -> Result<String, CoreError> {
        let id = subscription_id(vendor, identity_hash);
        self.conn.execute(
            "INSERT INTO subscriptions (id, vendor, identity_hash, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (vendor, identity_hash) DO UPDATE SET removed_at = NULL",
            params![id, vendor, identity_hash, now_ms],
        )?;
        Ok(self.conn.query_row(
            "SELECT id FROM subscriptions WHERE vendor = ?1 AND identity_hash = ?2",
            params![vendor, identity_hash],
            |row| row.get(0),
        )?)
    }

    /// One Account.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn get(&self, id: &str) -> Result<Option<Subscription>, CoreError> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {SUBSCRIPTION_COLUMNS} FROM subscriptions WHERE id = ?1"),
                params![id],
                subscription_row,
            )
            .optional()?)
    }

    /// Every Account not removed, oldest first.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn list(&self) -> Result<Vec<Subscription>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {SUBSCRIPTION_COLUMNS} FROM subscriptions WHERE removed_at IS NULL ORDER BY created_at, id"
        ))?;
        collect(stmt.query_map([], subscription_row)?)
    }

    /// The user's name for it (`None`: back to the default).
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn rename(&self, id: &str, name: Option<&str>) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE subscriptions SET name = ?2 WHERE id = ?1",
            params![id, name.map(str::trim).filter(|name| !name.is_empty())],
        )?;
        Ok(())
    }

    /// Its plan, renewal date and fee, set once for the Account.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn set_terms(
        &self,
        id: &str,
        plan: Option<&str>,
        renewal_date: Option<&str>,
        fee_usd: Option<f64>,
    ) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE subscriptions SET plan = ?2, renewal_date = ?3, fee_usd = ?4 WHERE id = ?1",
            params![id, plan, renewal_date, fee_usd],
        )?;
        Ok(())
    }

    /// Folder `folder_id`'s sign-in was last read as Account `account`
    /// (`None`: not signed in, or it can't be read). Whether it changed.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn set_folder(&self, folder_id: &str, account: Option<&str>) -> Result<bool, CoreError> {
        let changed = self.conn.execute(
            "UPDATE accounts SET subscription_id = ?2 WHERE id = ?1 AND subscription_id IS NOT ?2",
            params![folder_id, account],
        )?;
        Ok(changed > 0)
    }

    /// The Account each folder was last read as, by folder.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn folders(&self) -> Result<Vec<(String, Option<String>)>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, subscription_id FROM accounts WHERE removed_at IS NULL ORDER BY id",
        )?;
        collect(stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?)
    }
}

const APP_COLUMNS: &str = "id, kind, tool, host_app_id, instance_dir, folder_id, label, hidden, keep_alive, paused_json, launch_json, sessions_json, first_seen, last_seen";

fn app_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<App> {
    Ok(App {
        id: row.get(0)?,
        kind: row.get(1)?,
        tool: row.get(2)?,
        host_app_id: row.get(3)?,
        instance_dir: row.get(4)?,
        folder_id: row.get(5)?,
        label: row.get(6)?,
        hidden: row.get::<_, i64>(7)? != 0,
        keep_alive: row.get::<_, i64>(8)? != 0,
        paused_json: row.get(9)?,
        launch_json: row.get(10)?,
        sessions_json: row.get(11)?,
        first_seen: row.get(12)?,
        last_seen: row.get(13)?,
    })
}

/// The stored Apps.
pub struct AppsRepo<'a> {
    conn: &'a Connection,
}

impl<'a> AppsRepo<'a> {
    pub(super) const fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Tool `tool`'s CLI App on folder `folder_id`, made the first time.
    /// Its id.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn upsert_cli(
        &self,
        tool: &str,
        folder_id: &str,
        now_ms: i64,
    ) -> Result<String, CoreError> {
        let id = cli_app_id(folder_id);
        self.conn.execute(
            "INSERT INTO apps (id, kind, tool, folder_id, first_seen, last_seen)
             VALUES (?1, 'cli', ?2, ?3, ?4, ?4)
             ON CONFLICT (id) DO NOTHING",
            params![id, tool, folder_id, now_ms],
        )?;
        Ok(id)
    }

    /// Host app `host_app_id`'s App in `instance_dir`, running tool
    /// `tool`, made the first time it's seen; `last_seen` moves on. Its id.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn upsert_desktop(
        &self,
        tool: &str,
        host_app_id: &str,
        instance_dir: Option<&str>,
        now_ms: i64,
    ) -> Result<String, CoreError> {
        let id = desktop_app_id(host_app_id, instance_dir);
        self.conn.execute(
            "INSERT INTO apps (id, kind, tool, host_app_id, instance_dir, first_seen, last_seen)
             VALUES (?1, 'desktop', ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT (id) DO UPDATE SET last_seen = MAX(last_seen, excluded.last_seen)",
            params![id, tool, host_app_id, instance_dir, now_ms],
        )?;
        Ok(id)
    }

    /// Writes desktop App `app` whole, as the places book holds it (its
    /// name and whether it's hidden stay the user's). A folder that isn't
    /// stored is left unset.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn put_desktop(&self, app: &App) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO apps (id, kind, tool, host_app_id, instance_dir, folder_id, keep_alive,
                               paused_json, launch_json, sessions_json, first_seen, last_seen)
             VALUES (?1, 'desktop', ?2, ?3, ?4, (SELECT id FROM accounts WHERE id = ?5), ?6,
                     ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT (id) DO UPDATE SET
                 tool = excluded.tool, folder_id = excluded.folder_id,
                 keep_alive = excluded.keep_alive, paused_json = excluded.paused_json,
                 launch_json = excluded.launch_json, sessions_json = excluded.sessions_json,
                 last_seen = excluded.last_seen",
            params![
                app.id,
                app.tool,
                app.host_app_id,
                app.instance_dir,
                app.folder_id,
                i64::from(app.keep_alive),
                app.paused_json,
                app.launch_json,
                app.sessions_json,
                app.first_seen,
                app.last_seen
            ],
        )?;
        Ok(())
    }

    /// One App.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn get(&self, id: &str) -> Result<Option<App>, CoreError> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {APP_COLUMNS} FROM apps WHERE id = ?1"),
                params![id],
                app_row,
            )
            .optional()?)
    }

    /// Every App, the CLI ones first, then by when each was first seen.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn list(&self) -> Result<Vec<App>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {APP_COLUMNS} FROM apps ORDER BY kind, first_seen, id"
        ))?;
        collect(stmt.query_map([], app_row)?)
    }

    /// The user's name for it (`None`: back to the default).
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn rename(&self, id: &str, label: Option<&str>) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE apps SET label = ?2 WHERE id = ?1",
            params![id, label.map(str::trim).filter(|label| !label.is_empty())],
        )?;
        Ok(())
    }

    /// A desktop App's config folder, once its sessions name it.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn set_folder(&self, id: &str, folder_id: Option<&str>) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE apps SET folder_id = ?2 WHERE id = ?1 AND kind = 'desktop'",
            params![id, folder_id],
        )?;
        Ok(())
    }

    /// Keep alive on or off, and why it's paused (`None`: it isn't).
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn set_keep_alive(
        &self,
        id: &str,
        on: bool,
        paused_json: Option<&str>,
    ) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE apps SET keep_alive = ?2, paused_json = ?3 WHERE id = ?1",
            params![id, i64::from(on), paused_json],
        )?;
        Ok(())
    }

    /// How Open starts it, as it last ran (JSON).
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn set_launch(&self, id: &str, launch_json: Option<&str>) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE apps SET launch_json = ?2 WHERE id = ?1",
            params![id, launch_json],
        )?;
        Ok(())
    }

    /// Session `instance_id` runs in App `app_id`.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn set_session(&self, instance_id: &str, app_id: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE instances SET app_id = ?2 WHERE id = ?1",
            params![instance_id, app_id],
        )?;
        Ok(())
    }

    /// Forgets an App (the user's Forget). Its sessions keep their rows;
    /// their App reads as not known.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn forget(&self, id: &str) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE instances SET app_id = NULL WHERE app_id = ?1",
            params![id],
        )?;
        tx.execute("DELETE FROM apps WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(())
    }
}
