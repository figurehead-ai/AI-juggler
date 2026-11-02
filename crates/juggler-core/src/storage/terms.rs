//! D-62 (AJ-E3202 req. 4, schema v41): plan, renewal and fee kept on the
//! Account.
//!
//! Before 0.4 each config folder carried its own plan and renewal date
//! (`accounts.plan`, `accounts.renewal_date`) and its own monthly fee
//! (`app_settings` `subscription_fee:<folder>`). They're the
//! subscription's, so they move to its Account (`subscriptions.plan`,
//! `renewal_date`, `fee_usd`), one term at a time:
//!
//! - Where the folders on an Account agree (or only one says), the value
//!   moves to the Account and each folder is marked as moved for that term
//!   (`accounts.terms_moved`).
//! - Where they disagree, nothing moves: each folder keeps its own value
//!   (and goes on being read with it) until the user chooses once
//!   ([`TermsRepo::set`]), which sets the Account's and marks every folder.
//! - A folder that joins an Account later is moved by the same rule: a
//!   value that differs from the Account's asks again; nothing is dropped.
//!
//! The folders' own columns are never cleared (the move loses nothing) and
//! are no longer written for a folder on an Account. What applies to a
//! folder ([`TermsRepo::effective`]) is, per term, its Account's once moved,
//! else its own: the one rule every reader (renewal reminders, the cost
//! view, runways, the Accounts cards) goes through.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::CoreError;

/// v41: which of a folder's terms were moved to its Account. Additive;
/// nothing existing changes meaning (no term is marked, so each folder's
/// own values are read until its Account is known and the move runs).
pub(super) const MIGRATION: &str = "ALTER TABLE accounts ADD COLUMN terms_moved TEXT;";

/// The `app_settings` key a folder's own monthly fee was kept under.
#[must_use]
pub fn fee_key(folder_id: &str) -> String {
    format!("subscription_fee:{folder_id}")
}

/// One of the terms a subscription is set with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Term {
    /// Its plan ("Max", "max-20x"), as the user named it.
    Plan,
    /// Its renewal date (`YYYY-MM-DD`).
    Renewal,
    /// Its monthly fee, USD.
    Fee,
}

impl Term {
    /// Every term, in the order they're shown.
    pub const ALL: [Self; 3] = [Self::Plan, Self::Renewal, Self::Fee];

    /// Its wire name: `plan`, `renewal`, `fee`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Renewal => "renewal",
            Self::Fee => "fee",
        }
    }

    /// The term a wire name names.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|term| term.as_str() == raw)
    }
}

/// A plan, renewal date and monthly fee.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Terms {
    /// Its plan.
    pub plan: Option<String>,
    /// Its renewal date, `YYYY-MM-DD`.
    pub renewal_date: Option<String>,
    /// Its monthly fee, USD.
    pub fee_usd: Option<f64>,
}

impl Terms {
    /// Term `term` as text (a fee as `200`, `19.5`); `None` when unset.
    #[must_use]
    pub fn get(&self, term: Term) -> Option<String> {
        match term {
            Term::Plan => self.plan.clone(),
            Term::Renewal => self.renewal_date.clone(),
            Term::Fee => self.fee_usd.map(fee_text),
        }
    }

    fn put(&mut self, term: Term, value: Option<String>) {
        match term {
            Term::Plan => self.plan = value,
            Term::Renewal => self.renewal_date = value,
            Term::Fee => self.fee_usd = value.and_then(|fee| fee.parse().ok()),
        }
    }
}

fn fee_text(fee: f64) -> String {
    format!("{fee}")
}

/// A term an Account's folders disagree on: the values to choose from.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// Which term.
    pub term: Term,
    /// Each value said, in the order its first folder is listed (the
    /// Account's own first, when it has one).
    pub options: Vec<ChoiceOption>,
}

/// One value a term could take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceOption {
    /// The value, as [`Terms::get`] gives it.
    pub value: String,
    /// The folders that say it.
    pub folders: Vec<String>,
    /// Whether it's the Account's already.
    pub account: bool,
}

/// One folder's terms as the move sees them.
struct Folder {
    id: String,
    own: Terms,
    moved: Vec<Term>,
}

fn moved_of(raw: Option<&str>) -> Vec<Term> {
    raw.unwrap_or_default()
        .split(',')
        .filter_map(Term::parse)
        .collect()
}

fn moved_text(moved: &[Term]) -> String {
    Term::ALL
        .into_iter()
        .filter(|term| moved.contains(term))
        .map(Term::as_str)
        .collect::<Vec<_>>()
        .join(",")
}

/// A stored value as it's compared: trimmed, blank as none.
fn known(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Whether two values of `term` say the same: a plan by its words
/// ("Max 20x" is "max-20x"), the rest exactly.
fn same(term: Term, a: &str, b: &str) -> bool {
    let words = |plan: &str| {
        plan.to_lowercase()
            .split(|c: char| c == '-' || c == '_' || c.is_whitespace())
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    };
    match term {
        Term::Plan => words(a) == words(b),
        Term::Renewal | Term::Fee => a == b,
    }
}

/// The terms on the Accounts and their folders.
pub struct TermsRepo<'a> {
    conn: &'a Connection,
}

impl<'a> TermsRepo<'a> {
    pub(super) const fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    fn own_fee(&self, folder_id: &str) -> Result<Option<f64>, CoreError> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                params![fee_key(folder_id)],
                |row| row.get(0),
            )
            .optional()?;
        Ok(raw.and_then(|raw| raw.trim().parse().ok()))
    }

    /// Folder `folder_id`'s own terms, as set on it (before 0.4, or while
    /// it has no Account); `None` for a folder AI Juggler doesn't know.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn own(&self, folder_id: &str) -> Result<Option<Terms>, CoreError> {
        let row: Option<(Option<String>, Option<String>)> = self
            .conn
            .query_row(
                "SELECT plan, renewal_date FROM accounts WHERE id = ?1",
                params![folder_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((plan, renewal_date)) = row else {
            return Ok(None);
        };
        Ok(Some(Terms {
            plan: known(plan),
            renewal_date: known(renewal_date),
            fee_usd: self.own_fee(folder_id)?,
        }))
    }

    /// Account `account_id`'s terms; `None` for one that isn't stored.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn account(&self, account_id: &str) -> Result<Option<Terms>, CoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT plan, renewal_date, fee_usd FROM subscriptions WHERE id = ?1",
                params![account_id],
                |row| {
                    Ok(Terms {
                        plan: known(row.get(0)?),
                        renewal_date: known(row.get(1)?),
                        fee_usd: row.get(2)?,
                    })
                },
            )
            .optional()?)
    }

    /// The Account folder `folder_id` was last read as, if any.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn owner(&self, folder_id: &str) -> Result<Option<String>, CoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT subscription_id FROM accounts WHERE id = ?1",
                params![folder_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten())
    }

    /// What applies to folder `folder_id`: per term, its Account's once
    /// moved there, else its own (a term still to be chosen, or a folder
    /// with no Account). Default terms for a folder AI Juggler doesn't
    /// know.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn effective(&self, folder_id: &str) -> Result<Terms, CoreError> {
        match self.own(folder_id)? {
            Some(own) => self.overlay(folder_id, own),
            None => Ok(Terms::default()),
        }
    }

    /// [`Self::effective`] for a folder row already read (its own plan and
    /// renewal as the row has them; a row not stored keeps its own).
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn applying(&self, folder: &crate::model::Account) -> Result<Terms, CoreError> {
        let id = folder.id.as_str();
        let own = Terms {
            plan: known(folder.plan.as_ref().map(|plan| plan.as_str().to_owned())),
            renewal_date: known(folder.renewal_date.clone()),
            fee_usd: self.own_fee(id)?,
        };
        self.overlay(id, own)
    }

    /// `terms` (a folder's own) with each term moved to its Account read
    /// from the Account.
    fn overlay(&self, folder_id: &str, mut terms: Terms) -> Result<Terms, CoreError> {
        let row: Option<(Option<String>, Option<String>)> = self
            .conn
            .query_row(
                "SELECT subscription_id, terms_moved FROM accounts WHERE id = ?1",
                params![folder_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((Some(owner), moved)) = row else {
            return Ok(terms);
        };
        let Some(account) = self.account(&owner)? else {
            return Ok(terms);
        };
        for term in moved_of(moved.as_deref()) {
            terms.put(term, account.get(term));
        }
        Ok(terms)
    }

    fn folders(&self, account_id: &str) -> Result<Vec<Folder>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, terms_moved FROM accounts
             WHERE subscription_id = ?1 AND removed_at IS NULL ORDER BY created_at, id",
        )?;
        let rows: Vec<(String, Option<String>)> = stmt
            .query_map(params![account_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        rows.into_iter()
            .map(|(id, moved)| {
                Ok(Folder {
                    own: self.own(&id)?.unwrap_or_default(),
                    moved: moved_of(moved.as_deref()),
                    id,
                })
            })
            .collect()
    }

    fn mark_moved(&self, folder: &Folder, term: Term) -> Result<(), CoreError> {
        let mut moved = folder.moved.clone();
        moved.push(term);
        self.conn.execute(
            "UPDATE accounts SET terms_moved = ?2 WHERE id = ?1",
            params![folder.id, moved_text(&moved)],
        )?;
        Ok(())
    }

    fn put_account(
        &self,
        account_id: &str,
        term: Term,
        value: Option<&str>,
    ) -> Result<(), CoreError> {
        match term {
            Term::Plan => self.conn.execute(
                "UPDATE subscriptions SET plan = ?2 WHERE id = ?1",
                params![account_id, value],
            )?,
            Term::Renewal => self.conn.execute(
                "UPDATE subscriptions SET renewal_date = ?2 WHERE id = ?1",
                params![account_id, value],
            )?,
            Term::Fee => self.conn.execute(
                "UPDATE subscriptions SET fee_usd = ?2 WHERE id = ?1",
                params![account_id, value.and_then(|fee| fee.parse::<f64>().ok())],
            )?,
        };
        Ok(())
    }

    /// Moves Account `account_id`'s folders' terms to it where they agree
    /// (with each other and with what the Account already has), and says
    /// which terms wait for the user's choice. Run each time the folders
    /// are read: a term moved is never moved again, so it's cheap and
    /// changes nothing the second time.
    ///
    /// # Errors
    ///
    /// The database fails (nothing is half-moved: one transaction).
    pub fn settle(&self, account_id: &str) -> Result<Vec<Choice>, CoreError> {
        let Some(account) = self.account(account_id)? else {
            return Ok(Vec::new());
        };
        let tx = self.conn.unchecked_transaction()?;
        let mut folders = self.folders(account_id)?;
        let mut choices = Vec::new();
        for term in Term::ALL {
            let open: Vec<usize> = (0..folders.len())
                .filter(|&at| !folders[at].moved.contains(&term))
                .collect();
            if open.is_empty() {
                continue;
            }
            let mut said: Vec<ChoiceOption> = Vec::new();
            if let Some(value) = account.get(term) {
                said.push(ChoiceOption {
                    value,
                    folders: Vec::new(),
                    account: true,
                });
            }
            for &at in &open {
                let Some(value) = folders[at].own.get(term) else {
                    continue;
                };
                match said
                    .iter_mut()
                    .find(|option| same(term, &option.value, &value))
                {
                    Some(option) => option.folders.push(folders[at].id.clone()),
                    None => said.push(ChoiceOption {
                        value,
                        folders: vec![folders[at].id.clone()],
                        account: false,
                    }),
                }
            }
            if said.len() > 1 {
                choices.push(Choice {
                    term,
                    options: said,
                });
                continue;
            }
            if let Some(only) = said.first().filter(|only| !only.account) {
                self.put_account(account_id, term, Some(&only.value))?;
            }
            for &at in &open {
                self.mark_moved(&folders[at], term)?;
                folders[at].moved.push(term);
            }
        }
        tx.commit()?;
        Ok(choices)
    }

    /// Sets term `term` on Account `account_id` (`None` clears it): the
    /// user's own value, or their choice where its folders disagreed. Every
    /// folder on it is marked moved for that term, so the choice is made
    /// once.
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn set(&self, account_id: &str, term: Term, value: Option<&str>) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        self.put_account(account_id, term, value)?;
        for folder in self.folders(account_id)? {
            if !folder.moved.contains(&term) {
                self.mark_moved(&folder, term)?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Sets term `term` for folder `folder_id`: on its Account when it has
    /// one ([`Self::set`]); else on the folder itself, which then stands as
    /// its own until its sign-in is read (and moves by the same rule).
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn set_for_folder(
        &self,
        folder_id: &str,
        term: Term,
        value: Option<&str>,
    ) -> Result<(), CoreError> {
        if let Some(owner) = self.owner(folder_id)? {
            return self.set(&owner, term, value);
        }
        match term {
            Term::Plan => {
                self.conn.execute(
                    "UPDATE accounts SET plan = ?2 WHERE id = ?1",
                    params![folder_id, value],
                )?;
            }
            Term::Renewal => {
                self.conn.execute(
                    "UPDATE accounts SET renewal_date = ?2 WHERE id = ?1",
                    params![folder_id, value],
                )?;
            }
            Term::Fee => {
                super::AppSettingsRepo::new(self.conn)
                    .set(&fee_key(folder_id), value.unwrap_or(""))?;
            }
        }
        Ok(())
    }

    /// What applies to every folder in use, by folder (one read for a
    /// list).
    ///
    /// # Errors
    ///
    /// The database fails.
    pub fn effective_all(&self) -> Result<BTreeMap<String, Terms>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM accounts WHERE removed_at IS NULL ORDER BY id")?;
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        ids.into_iter()
            .map(|id| Ok((id.clone(), self.effective(&id)?)))
            .collect()
    }
}
