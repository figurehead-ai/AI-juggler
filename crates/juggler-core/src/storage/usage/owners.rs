//! Who keeps a keyed call (the shared-message rule, [`Span::rank`]) and
//! where its one row is: every session that offered it, in any account
//! (0.3.8), and the keeper's account and session.

use std::collections::{BTreeSet, HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, params};

use super::{as_sql, from_sql};
use crate::error::CoreError;
use crate::model::TokenCounts;

/// One session's standing for the messages it shares, from every message
/// it offered (kept or not).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Span {
    /// Its first message.
    first: i64,
    /// Its last message.
    last: i64,
    /// Its first message no other session holds; `None` when it has none.
    own_start: Option<i64>,
}

impl Span {
    /// Who keeps a message two sessions share (a resumed copy or a fork
    /// repeats its original's history, with the original times): the
    /// smallest of these.
    /// 1. The earliest first message.
    /// 2. The session whose own messages start latest, none counting as
    ///    latest: a copy starts its own right after it was made, while the
    ///    original has none then, or starts them later.
    /// 3. Between sessions with none of their own, the earlier last
    ///    message (in a chain of copies, the shorter one is older).
    /// 4. The smaller id, then (0.3.8) the smaller account.
    ///
    /// 0.3.8: the holders are every account's. A conversation copied into
    /// another account outside AI Juggler is a resumed copy for this rule:
    /// the original keeps the history, the copy its own new messages.
    ///
    /// None of these moves as a session grows past its own start, so a
    /// message's keeper doesn't either.
    ///
    /// Why 2 holds: Claude's resume writes a copy holding everything up to
    /// the point it was resumed, so the original has messages of its own
    /// only after the fork. That puts the shared part on the original in a
    /// chain of copies, in a fork, and when the original never writes again.
    ///
    /// Known limits, each pinned by a test. Only the split between sessions
    /// is affected; an account's totals never are (each message is counted
    /// once, whoever keeps it).
    /// - The original writes first after a fork and the copy later: the
    ///   copy's own start is the later one, so it is credited the shared
    ///   part.
    /// - A fork that is itself resumed before writing anything (O goes on,
    ///   C forks from it, D resumes C): C has nothing of its own, so it
    ///   takes O's history.
    /// - A resume that never writes: the original and the copy tie on
    ///   everything but the id, so the smaller id keeps the history.
    fn rank<'s>(
        &self,
        who: &'s SessionKey,
    ) -> (i64, std::cmp::Reverse<i64>, i64, &'s str, &'s str) {
        (
            self.first,
            std::cmp::Reverse(self.own_start.unwrap_or(i64::MAX)),
            if self.own_start.is_none() {
                self.last
            } else {
                0
            },
            &who.1,
            &who.0,
        )
    }

    /// Whether a change from `before` can change who keeps a message.
    fn moved_from(&self, before: Option<Self>) -> bool {
        before.is_none_or(|was| {
            was.first != self.first
                || was.own_start != self.own_start
                || (self.own_start.is_none() && was.last != self.last)
        })
    }
}

/// `(account, session)`.
pub(super) type SessionKey = (String, String);

/// Settles who keeps each shared message (see [`Span::rank`]). Every
/// session that offers a message is recorded, kept or not; each pass
/// updates the standing of the sessions it touched and settles every
/// shared message whose keeper could have changed. The answer depends
/// only on what has been offered, never on the order or the passes.
///
/// 0.3.8: a holder is an account's session, and a message is shared when
/// another account's session offers it too; its one row goes to the
/// keeper's account (any copy another account held merged into it).
pub(super) struct Owners<'c> {
    hold: rusqlite::Statement<'c>,
    holders_of: rusqlite::Statement<'c>,
    load_span: rusqlite::Statement<'c>,
    save_span: rusqlite::Statement<'c>,
    count_span: rusqlite::Statement<'c>,
    shared_keys: rusqlite::Statement<'c>,
    rows: Rows<'c>,
    /// Standing as stored when the pass first touched each session.
    before: HashMap<SessionKey, Option<Span>>,
    /// Standing now.
    spans: HashMap<SessionKey, Span>,
    /// Sessions whose own start must be counted again in full (one of
    /// their own messages became shared, or a session they shared with
    /// was removed).
    pub(super) recount: HashSet<SessionKey>,
    /// Shared messages to settle, by key.
    pub(super) keys: BTreeSet<String>,
}

impl<'c> Owners<'c> {
    pub(super) fn new(conn: &'c Connection) -> Result<Self, CoreError> {
        Ok(Self {
            hold: conn.prepare(
                "INSERT OR IGNORE INTO usage_holders (account, msg_uuid, session, ts)
                 VALUES (?1, ?2, ?3, ?4)",
            )?,
            holders_of: conn.prepare(
                "SELECT account, session, ts FROM usage_holders WHERE msg_uuid = ?1",
            )?,
            load_span: conn.prepare(
                "SELECT first, last, own_start FROM usage_spans WHERE account = ?1 AND session = ?2",
            )?,
            save_span: conn.prepare(
                "INSERT OR REPLACE INTO usage_spans (account, session, first, last, own_start)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?,
            count_span: conn.prepare(
                "SELECT MIN(h.ts), MAX(h.ts), MIN(CASE WHEN NOT EXISTS (
                     SELECT 1 FROM usage_holders o
                     WHERE o.msg_uuid = h.msg_uuid
                       AND (o.account <> h.account OR o.session <> h.session)) THEN h.ts END)
                 FROM usage_holders h WHERE h.account = ?1 AND h.session = ?2",
            )?,
            shared_keys: conn.prepare(
                "SELECT h.msg_uuid FROM usage_holders h
                 WHERE h.account = ?1 AND h.session = ?2 AND EXISTS (
                     SELECT 1 FROM usage_holders o
                     WHERE o.msg_uuid = h.msg_uuid
                       AND (o.account <> h.account OR o.session <> h.session))",
            )?,
            rows: Rows::new(conn)?,
            before: HashMap::new(),
            spans: HashMap::new(),
            recount: HashSet::new(),
            keys: BTreeSet::new(),
        })
    }

    /// The stored standing, or one counted from the holders (a session
    /// recorded before standings were kept).
    fn stored(&mut self, who: &SessionKey) -> Result<Option<Span>, CoreError> {
        if let Some(span) = self
            .load_span
            .query_row(params![who.0, who.1], |row| {
                Ok(Span {
                    first: row.get(0)?,
                    last: row.get(1)?,
                    own_start: row.get(2)?,
                })
            })
            .optional()?
        {
            return Ok(Some(span));
        }
        self.counted(who)
    }

    /// The standing counted in full from the holders.
    fn counted(&mut self, who: &SessionKey) -> Result<Option<Span>, CoreError> {
        let (first, last, own_start): (Option<i64>, Option<i64>, Option<i64>) =
            self.count_span.query_row(params![who.0, who.1], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;
        Ok(first.zip(last).map(|(first, last)| Span {
            first,
            last,
            own_start,
        }))
    }

    /// The standing as this pass sees it (touching the session).
    fn span(&mut self, who: &SessionKey) -> Result<Option<Span>, CoreError> {
        if let Some(span) = self.spans.get(who) {
            return Ok(Some(*span));
        }
        let stored = self.stored(who)?;
        self.before.insert(who.clone(), stored);
        if let Some(span) = stored {
            self.spans.insert(who.clone(), span);
        }
        Ok(stored)
    }

    /// Every account's session that offered `key`, with when.
    fn holders(&mut self, key: &str) -> Result<Vec<(SessionKey, i64)>, CoreError> {
        let rows = self.holders_of.query_map(params![key], |row| {
            Ok(((row.get(0)?, row.get(1)?), row.get(2)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// The accounts holding `key` (where its one row can be).
    pub(super) fn holding_accounts(&mut self, key: &str) -> Result<BTreeSet<String>, CoreError> {
        Ok(self
            .holders(key)?
            .into_iter()
            .map(|((account, _), _)| account)
            .collect())
    }

    /// `session` of `account` offered the message `key` at `ts`.
    pub(super) fn offer(
        &mut self,
        account: &str,
        key: &str,
        session: &str,
        ts: i64,
    ) -> Result<(), CoreError> {
        let who = (account.to_owned(), session.to_owned());
        let holders = self.holders(key)?;
        if holders.iter().any(|(holder, _)| *holder == who) {
            return Ok(()); // it had offered it before
        }
        // Every standing this offer can move is read before it is written:
        // a session without a stored standing is counted from the holders,
        // which must not include this offer yet, or its move goes unseen.
        let was = self.span(&who)?;
        for (holder, _) in &holders {
            self.span(holder)?;
        }
        self.hold.execute(params![account, key, session, ts])?;
        let own = holders.is_empty();
        let span = was.map_or(
            Span {
                first: ts,
                last: ts,
                own_start: own.then_some(ts),
            },
            |was| Span {
                first: was.first.min(ts),
                last: was.last.max(ts),
                own_start: if own {
                    Some(was.own_start.map_or(ts, |start| start.min(ts)))
                } else {
                    was.own_start
                },
            },
        );
        self.spans.insert(who, span);
        if own {
            return Ok(());
        }
        // A message that was another session's own is now shared: its
        // own start may have been this one.
        if let [(other, at)] = holders.as_slice()
            && self.span(other)?.and_then(|span| span.own_start) == Some(*at)
        {
            self.recount.insert(other.clone());
        }
        self.keys.insert(key.to_owned());
        Ok(())
    }

    /// Settles every shared message whose keeper could have changed, and
    /// stores the new standings.
    pub(super) fn finish(mut self) -> Result<(), CoreError> {
        for who in std::mem::take(&mut self.recount) {
            self.span(&who)?;
            match self.counted(&who)? {
                Some(span) => {
                    self.spans.insert(who, span);
                }
                None => {
                    self.spans.remove(&who);
                }
            }
        }
        let touched: Vec<(SessionKey, Span)> = self
            .spans
            .iter()
            .map(|(who, span)| (who.clone(), *span))
            .collect();
        for (who, span) in touched {
            let before = self.before.get(&who).copied().flatten();
            if before != Some(span) {
                self.save_span.execute(params![
                    who.0,
                    who.1,
                    span.first,
                    span.last,
                    span.own_start
                ])?;
            }
            if span.moved_from(before) {
                let keys = self
                    .shared_keys
                    .query_map(params![who.0, who.1], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                self.keys.extend(keys);
            }
        }
        for key in std::mem::take(&mut self.keys) {
            self.settle(&key)?;
        }
        Ok(())
    }

    /// Gives `key`'s row to its best holder (account and session); copies
    /// of it other accounts hold go, merged into it.
    fn settle(&mut self, key: &str) -> Result<(), CoreError> {
        let mut best: Option<(Span, SessionKey)> = None;
        for (holder, _) in self.holders(key)? {
            let Some(span) = self.span(&holder)? else {
                continue;
            };
            if best
                .as_ref()
                .is_none_or(|(kept, by)| span.rank(&holder) < kept.rank(by))
            {
                best = Some((span, holder));
            }
        }
        let Some((_, keeper)) = best else {
            return Ok(());
        };
        let accounts = self.holding_accounts(key)?;
        self.rows.give(key, &accounts, &keeper)
    }
}

/// The one row of a keyed call among the accounts that hold it (0.3.8).
pub(super) struct Rows<'c> {
    find: rusqlite::Statement<'c>,
    has_detail: rusqlite::Statement<'c>,
    set_tokens: rusqlite::Statement<'c>,
    drop_row: rusqlite::Statement<'c>,
    drop_detail: rusqlite::Statement<'c>,
    move_detail: rusqlite::Statement<'c>,
    move_row: rusqlite::Statement<'c>,
    move_session: rusqlite::Statement<'c>,
}

impl<'c> Rows<'c> {
    pub(super) fn new(conn: &'c Connection) -> Result<Self, CoreError> {
        Ok(Self {
            find: conn.prepare(
                "SELECT input, output, cache_r, cache_w, reasoning, source FROM usage_events
                 WHERE account = ?1 AND msg_uuid = ?2",
            )?,
            has_detail: conn
                .prepare("SELECT 1 FROM usage_detail WHERE account = ?1 AND msg_uuid = ?2")?,
            set_tokens: conn.prepare(
                "UPDATE usage_events
                 SET input = ?3, output = ?4, cache_r = ?5, cache_w = ?6, reasoning = ?7
                 WHERE account = ?1 AND msg_uuid = ?2",
            )?,
            drop_row: conn
                .prepare("DELETE FROM usage_events WHERE account = ?1 AND msg_uuid = ?2")?,
            drop_detail: conn
                .prepare("DELETE FROM usage_detail WHERE account = ?1 AND msg_uuid = ?2")?,
            move_detail: conn.prepare(
                "UPDATE usage_detail SET account = ?3 WHERE account = ?1 AND msg_uuid = ?2",
            )?,
            move_row: conn.prepare(
                "UPDATE usage_events SET account = ?3, session = ?4
                 WHERE account = ?1 AND msg_uuid = ?2",
            )?,
            move_session: conn.prepare(
                "UPDATE usage_events SET session = ?3
                 WHERE account = ?1 AND msg_uuid = ?2 AND session IS NOT ?3",
            )?,
        })
    }

    /// `key`'s row in `account`, with its source.
    fn of(&mut self, account: &str, key: &str) -> Result<Option<(TokenCounts, String)>, CoreError> {
        self.find
            .query_row(params![account, key], |row| {
                Ok((
                    TokenCounts {
                        input: from_sql(row.get(0)?),
                        output: from_sql(row.get(1)?),
                        cache_read: from_sql(row.get(2)?),
                        cache_write: from_sql(row.get(3)?),
                        reasoning: from_sql(row.get(4)?),
                    },
                    row.get(5)?,
                ))
            })
            .optional()
            .map_err(CoreError::Storage)
    }

    /// The account among `accounts` (`first` looked at first) holding a
    /// row of `key` from `source`, with its tokens.
    pub(super) fn find_among(
        &mut self,
        key: &str,
        source: &str,
        first: &str,
        accounts: &BTreeSet<String>,
    ) -> Result<Option<(String, TokenCounts)>, CoreError> {
        let order = std::iter::once(first).chain(
            accounts
                .iter()
                .map(String::as_str)
                .filter(|account| *account != first),
        );
        for account in order {
            // Its own account's row is the call whatever reader stored it
            // (as before 0.3.8); another account's only from the same one.
            if let Some((tokens, from)) = self.of(account, key)?
                && (from == source || account == first)
            {
                return Ok(Some((account.to_owned(), tokens)));
            }
        }
        Ok(None)
    }

    /// Puts `key`'s one row on `keeper`: the keeper account's row if it has
    /// one, else another holding account's, moved there; any further copy
    /// (rows two accounts stored before 0.3.8) merged in, each class at its
    /// largest, and removed.
    pub(super) fn give(
        &mut self,
        key: &str,
        accounts: &BTreeSet<String>,
        keeper: &SessionKey,
    ) -> Result<(), CoreError> {
        let mut rows: Vec<(String, TokenCounts, String)> = Vec::new();
        for account in accounts {
            if let Some((tokens, source)) = self.of(account, key)? {
                rows.push((account.clone(), tokens, source));
            }
        }
        let Some(at) = rows
            .iter()
            .position(|(account, _, _)| *account == keeper.0)
            .or(if rows.is_empty() { None } else { Some(0) })
        else {
            return Ok(());
        };
        let (target, mut tokens, source) = rows.swap_remove(at);
        let mut detail = self.has_detail.exists(params![target, key])?;
        let was = tokens;
        for (other, copy, from) in rows {
            if from != source {
                continue; // another tool's row: never one call
            }
            tokens = TokenCounts {
                input: tokens.input.max(copy.input),
                output: tokens.output.max(copy.output),
                cache_read: tokens.cache_read.max(copy.cache_read),
                cache_write: tokens.cache_write.max(copy.cache_write),
                reasoning: tokens.reasoning.max(copy.reasoning),
            };
            self.drop_row.execute(params![other, key])?;
            if detail {
                self.drop_detail.execute(params![other, key])?;
            } else {
                detail = self.move_detail.execute(params![other, key, target])? > 0;
            }
        }
        if tokens != was {
            self.set_tokens.execute(params![
                target,
                key,
                as_sql(tokens.input),
                as_sql(tokens.output),
                as_sql(tokens.cache_read),
                as_sql(tokens.cache_write),
                as_sql(tokens.reasoning),
            ])?;
        }
        if target == keeper.0 {
            self.move_session.execute(params![target, key, keeper.1])?;
        } else {
            self.move_row
                .execute(params![target, key, keeper.0, keeper.1])?;
            self.move_detail.execute(params![target, key, keeper.0])?;
        }
        Ok(())
    }
}
