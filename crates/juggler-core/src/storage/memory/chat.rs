//! Bot's one conversation (D-136, ADR-0037 §1.6, AJ-E3808): the turns on
//! this Mac, shown as one stream. Each New topic starts a new
//! `chat_conversations` row; the window joins the rows into one stream
//! and draws a divider where a row begins. Nothing here is ever sent off
//! the Mac (ADR-0037 §9).
//!
//! - **What a turn keeps:** the question (its secrets masked by the
//!   caller), the outcome, the written answer when one was shown, the
//!   ids it rests on with their state *then* (`cites`), and the scope it
//!   was asked in (`entities`). Never the raw rows, a tool call, a
//!   rejected draft or a transcript.
//! - **Bounded:** at most [`MAX_BYTES`] of words, the oldest topics
//!   removed first ([`ChatHistory::append`]).
//! - **Cleared whole** ([`ChatHistory::clear`]) with `secure_delete` on and
//!   the WAL folded after, as a forget is (ADR-0037 §1.5).

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::secure::{SecureHold, fold_wal};
use super::store::new_id;
use super::types::MemoryError;
use crate::storage::write_transaction;

/// The conversation's words kept at most (ADR-0034 §6.7: 20 MB).
pub const MAX_BYTES: i64 = 20 * 1024 * 1024;
/// The longest question kept, in characters (ADR-0037 §1.6).
pub const QUESTION_MAX_CHARS: usize = 600;
/// A conversation row's title, at most (characters of its first question).
const TITLE_MAX_CHARS: usize = 80;
/// The most turns one page holds.
pub const PAGE_MAX: u32 = 50;
/// The most matches a search gives.
pub const FIND_MAX: u32 = 500;

/// A turn to keep.
#[derive(Clone, Debug)]
pub struct NewTurn<'a> {
    /// When it was asked.
    pub at_ms: i64,
    /// The user's words, secrets masked.
    pub question: &'a str,
    /// `rows`, `answered`, `dont_know`, `out_of_scope`, `help`,
    /// `watch_set`, `remembered`, `refused` (a code).
    pub outcome: &'a str,
    /// The written answer, when one was shown.
    pub shown: Option<&'a str>,
    /// The ids the result rests on, with their state then (JSON).
    pub cites: &'a str,
    /// The scope it was asked in (JSON).
    pub entities: &'a str,
    /// Whether what was shown quoted an agent's words.
    pub has_agent_words: bool,
}

/// A kept turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Turn {
    /// Its id.
    pub id: String,
    /// Its topic (a `chat_conversations` row).
    pub topic: String,
    /// When it was asked.
    pub at_ms: i64,
    /// The question (empty once forgotten).
    pub question: String,
    /// Its outcome code.
    pub outcome: String,
    /// The written answer shown, if any.
    pub shown: Option<String>,
    /// The cites (JSON).
    pub cites: String,
    /// The scope (JSON).
    pub entities: String,
    /// Whether it quoted an agent's words.
    pub has_agent_words: bool,
    /// Forget cleared its words.
    pub redacted: bool,
}

/// Where a topic begins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Topic {
    /// Its id.
    pub id: String,
    /// When it began.
    pub started_ms: i64,
    /// The conversation's very first topic (no divider above it).
    pub first: bool,
}

/// One page of the stream, newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChatPage {
    /// The turns, newest first.
    pub turns: Vec<Turn>,
    /// The topics that begin within the page (or after its newest turn,
    /// on the first page: a New topic with nothing asked yet).
    pub topics: Vec<Topic>,
    /// Older turns remain.
    pub more: bool,
}

/// What the stream holds, for the clear sheet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChatCount {
    /// Questions kept.
    pub turns: u32,
    /// The oldest one's time.
    pub since_ms: Option<i64>,
}

/// What a clear removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cleared {
    /// Turns removed.
    pub turns: u32,
    /// Topics removed.
    pub topics: u32,
    /// The WAL was folded at once.
    pub wal_folded: bool,
}

/// The conversation over one connection, under the caller's writer lock.
#[derive(Debug)]
pub struct ChatHistory<'c> {
    conn: &'c Connection,
}

const TURN_COLUMNS: &str = "id, conversation_id, at_ms, question, outcome, shown, cites, entities, has_agent_words, redacted";

fn turn_row(row: &Row<'_>) -> rusqlite::Result<Turn> {
    Ok(Turn {
        id: row.get(0)?,
        topic: row.get(1)?,
        at_ms: row.get(2)?,
        question: row.get(3)?,
        outcome: row.get(4)?,
        shown: row.get(5)?,
        cites: row.get(6)?,
        entities: row.get(7)?,
        has_agent_words: row.get::<_, i64>(8)? != 0,
        redacted: row.get::<_, i64>(9)? != 0,
    })
}

fn cut(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

impl<'c> ChatHistory<'c> {
    /// The conversation on `conn` (V54's tables must be there).
    #[must_use]
    pub const fn new(conn: &'c Connection) -> Self {
        Self { conn }
    }

    fn current_topic(conn: &Connection) -> rusqlite::Result<Option<(String, Option<String>)>> {
        conn.query_row(
            "SELECT id, title_turn FROM chat_conversations ORDER BY started_ms DESC, id DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
    }

    fn open_topic(conn: &Connection, at_ms: i64) -> rusqlite::Result<String> {
        let id = new_id(conn, at_ms)?;
        conn.execute(
            "INSERT INTO chat_conversations (id, started_ms, last_ms, title) VALUES (?1, ?2, ?2, '')",
            params![id, at_ms],
        )?;
        Ok(id)
    }

    /// Keeps one turn in the current topic (the first one starts it), then
    /// removes the oldest topics past [`MAX_BYTES`] (never the current).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn append(&self, turn: &NewTurn<'_>) -> Result<Turn, MemoryError> {
        let _hold = SecureHold::on(self.conn)?;
        let tx = write_transaction(self.conn)?;
        let (topic, titled) = match Self::current_topic(&tx)? {
            Some((id, title_turn)) => (id, title_turn.is_some()),
            None => (Self::open_topic(&tx, turn.at_ms)?, false),
        };
        let id = new_id(&tx, turn.at_ms)?;
        let seq: i64 = tx.query_row(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM chat_turns WHERE conversation_id = ?1",
            [&topic],
            |row| row.get(0),
        )?;
        let question = cut(turn.question, QUESTION_MAX_CHARS);
        tx.execute(
            "INSERT INTO chat_turns (id, conversation_id, seq, at_ms, question, outcome, shown, cites, entities, has_agent_words)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![id, topic, seq, turn.at_ms, question, turn.outcome, turn.shown, turn.cites, turn.entities, i64::from(turn.has_agent_words)],
        )?;
        if titled {
            tx.execute(
                "UPDATE chat_conversations SET last_ms = MAX(last_ms, ?2),
                     has_agent_words = MAX(has_agent_words, ?3) WHERE id = ?1",
                params![topic, turn.at_ms, i64::from(turn.has_agent_words)],
            )?;
        } else {
            tx.execute(
                "UPDATE chat_conversations SET last_ms = MAX(last_ms, ?2), title = ?3, title_turn = ?4,
                     has_agent_words = MAX(has_agent_words, ?5) WHERE id = ?1",
                params![topic, turn.at_ms, cut(&question, TITLE_MAX_CHARS), id, i64::from(turn.has_agent_words)],
            )?;
        }
        Self::prune(&tx, &topic)?;
        let kept = tx.query_row(
            &format!("SELECT {TURN_COLUMNS} FROM chat_turns WHERE id = ?1"),
            [&id],
            turn_row,
        )?;
        tx.commit()?;
        Ok(kept)
    }

    /// The oldest topics go while the words kept pass [`MAX_BYTES`].
    fn prune(conn: &Connection, current: &str) -> rusqlite::Result<()> {
        loop {
            let size: i64 = conn.query_row(
                "SELECT COALESCE(SUM(length(question) + COALESCE(length(shown), 0) + length(cites) + length(entities)), 0) FROM chat_turns",
                [],
                |row| row.get(0),
            )?;
            if size <= MAX_BYTES {
                return Ok(());
            }
            let oldest: Option<String> = conn
                .query_row(
                    "SELECT id FROM chat_conversations WHERE id != ?1 ORDER BY started_ms, id LIMIT 1",
                    [current],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(oldest) = oldest else {
                return Ok(());
            };
            conn.execute("DELETE FROM chat_conversations WHERE id = ?1", [oldest])?;
        }
    }

    /// Starts a new topic: follow-ups stop drawing on what came before,
    /// and nothing is deleted. A topic with nothing asked yet is kept as
    /// it is (two clicks make one divider). Its start time.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn new_topic(&self, at_ms: i64) -> Result<Topic, MemoryError> {
        let tx = write_transaction(self.conn)?;
        let current = Self::current_topic(&tx)?;
        let reuse = match &current {
            Some((id, None)) => Some(id.clone()),
            _ => None,
        };
        let id = match reuse {
            Some(id) => id,
            None => Self::open_topic(&tx, at_ms)?,
        };
        let topic = Self::topic_of(&tx, &id)?;
        tx.commit()?;
        Ok(topic)
    }

    fn topic_of(conn: &Connection, id: &str) -> rusqlite::Result<Topic> {
        let started_ms: i64 = conn.query_row(
            "SELECT started_ms FROM chat_conversations WHERE id = ?1",
            [id],
            |row| row.get(0),
        )?;
        let first_id: Option<String> = conn
            .query_row(
                "SELECT id FROM chat_conversations ORDER BY started_ms, id LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(Topic {
            id: id.to_owned(),
            started_ms,
            first: first_id.as_deref() == Some(id),
        })
    }

    /// The current topic's turns, oldest first, at most `limit` of the
    /// newest: what a follow-up may draw on (New topic resets it).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn recent(&self, limit: u32) -> Result<Vec<Turn>, MemoryError> {
        let Some((topic, _)) = Self::current_topic(self.conn)? else {
            return Ok(Vec::new());
        };
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {TURN_COLUMNS} FROM chat_turns WHERE conversation_id = ?1 ORDER BY seq DESC LIMIT ?2"
        ))?;
        let mut turns = stmt
            .query_map(params![topic, limit], turn_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        turns.reverse();
        Ok(turns)
    }

    /// A page of the stream, newest first: the turns before `before`
    /// (`(at_ms, id)` of the oldest turn shown), or the newest.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn page(&self, before: Option<(i64, &str)>, limit: u32) -> Result<ChatPage, MemoryError> {
        let limit = limit.clamp(1, PAGE_MAX);
        let (at, id) = before.map_or((i64::MAX, ""), |(at, id)| (at, id));
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {TURN_COLUMNS} FROM chat_turns
             WHERE at_ms < ?1 OR (at_ms = ?1 AND id < ?2)
             ORDER BY at_ms DESC, id DESC LIMIT ?3"
        ))?;
        let id_bound = if before.is_some() { id } else { "\u{10FFFF}" };
        let mut turns = stmt
            .query_map(params![at, id_bound, i64::from(limit) + 1], turn_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let more = turns.len() > limit as usize;
        turns.truncate(limit as usize);
        // The topics that begin within the page: after its oldest turn
        // (every one, when nothing older is left), up to its upper bound.
        let low = if more {
            turns.last().map_or(i64::MIN, |turn| turn.at_ms)
        } else {
            i64::MIN
        };
        let high = before.map_or(i64::MAX, |(at, _)| at);
        let first_id: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM chat_conversations ORDER BY started_ms, id LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, started_ms FROM chat_conversations
             WHERE started_ms >= ?1 AND started_ms <= ?2 ORDER BY started_ms DESC, id DESC",
        )?;
        let topics = stmt
            .query_map(params![low, high], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .map(|(id, started_ms)| Topic {
                first: first_id.as_deref() == Some(id.as_str()),
                id,
                started_ms,
            })
            .collect();
        Ok(ChatPage {
            turns,
            topics,
            more,
        })
    }

    /// The turns whose question, answer or cited names hold `words` (any
    /// case), oldest first, at most [`FIND_MAX`] ids.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn find(&self, words: &str) -> Result<Vec<(String, i64)>, MemoryError> {
        let words = words.trim();
        if words.is_empty() {
            return Ok(Vec::new());
        }
        let escaped = words
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let like = format!("%{escaped}%");
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, at_ms FROM chat_turns
             WHERE question LIKE ?1 ESCAPE '\\' OR COALESCE(shown, '') LIKE ?1 ESCAPE '\\'
                OR cites LIKE ?1 ESCAPE '\\'
             ORDER BY at_ms, id LIMIT ?2",
        )?;
        let found = stmt
            .query_map(params![like, FIND_MAX], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(found)
    }

    /// How many questions the stream holds, and since when.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn count(&self) -> Result<ChatCount, MemoryError> {
        let (turns, since_ms): (i64, Option<i64>) =
            self.conn
                .query_row("SELECT COUNT(*), MIN(at_ms) FROM chat_turns", [], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })?;
        Ok(ChatCount {
            turns: u32::try_from(turns).unwrap_or(u32::MAX),
            since_ms,
        })
    }

    /// Removes the whole conversation, every topic and turn, with
    /// `secure_delete` on, then folds the WAL so no frame keeps the words.
    /// The user's notes, preferences and watches stay (they are memory
    /// records of their own).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn clear(&self) -> Result<Cleared, MemoryError> {
        let cleared = {
            let _hold = SecureHold::on(self.conn)?;
            let tx = write_transaction(self.conn)?;
            let turns = tx.execute("DELETE FROM chat_turns", [])?;
            let topics = tx.execute("DELETE FROM chat_conversations", [])?;
            tx.commit()?;
            Cleared {
                turns: u32::try_from(turns).unwrap_or(u32::MAX),
                topics: u32::try_from(topics).unwrap_or(u32::MAX),
                wal_folded: false,
            }
        };
        Ok(Cleared {
            wal_folded: fold_wal(self.conn)?,
            ..cleared
        })
    }

    /// One turn by id.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn turn(&self, id: &str) -> Result<Option<Turn>, MemoryError> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {TURN_COLUMNS} FROM chat_turns WHERE id = ?1"),
                [id],
                turn_row,
            )
            .optional()?)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a test: a failed step fails the test")]

    use super::*;
    use crate::storage::Storage;

    fn turn(at_ms: i64, question: &str) -> NewTurn<'_> {
        NewTurn {
            at_ms,
            question,
            outcome: "rows",
            shown: None,
            cites: "[]",
            entities: "{}",
            has_agent_words: false,
        }
    }

    #[test]
    fn the_stream_pages_newest_first_and_topics_mark_where_they_begin() {
        let storage = Storage::open_in_memory().unwrap();
        let chat = storage.chat();
        for n in 0..5 {
            chat.append(&turn(1_000 + n, &format!("question {n}")))
                .unwrap();
        }
        let topic = chat.new_topic(2_000).unwrap();
        assert!(!topic.first);
        // A second click before asking anything makes no second topic.
        assert_eq!(chat.new_topic(2_001).unwrap().id, topic.id);
        chat.append(&turn(2_010, "after the topic")).unwrap();

        let first = chat.page(None, 3).unwrap();
        assert_eq!(
            first
                .turns
                .iter()
                .map(|t| t.question.as_str())
                .collect::<Vec<_>>(),
            ["after the topic", "question 4", "question 3"]
        );
        assert!(first.more);
        assert_eq!(
            first
                .topics
                .iter()
                .map(|t| t.started_ms)
                .collect::<Vec<_>>(),
            [2_000]
        );
        let oldest = first.turns.last().unwrap();
        let next = chat.page(Some((oldest.at_ms, &oldest.id)), 3).unwrap();
        assert_eq!(
            next.turns
                .iter()
                .map(|t| t.question.as_str())
                .collect::<Vec<_>>(),
            ["question 2", "question 1", "question 0"]
        );
        assert!(!next.more);
        assert!(
            next.topics.iter().all(|t| t.first),
            "only the first topic, with no divider"
        );
        // A follow-up draws on the current topic only.
        assert_eq!(chat.recent(10).unwrap().len(), 1);
    }

    #[test]
    fn find_matches_questions_and_cited_names_in_any_case() {
        let storage = Storage::open_in_memory().unwrap();
        let chat = storage.chat();
        chat.append(&turn(1, "Is the LOGIN redirect stuck?"))
            .unwrap();
        chat.append(&NewTurn {
            cites: r#"[{"title":"Fix the login redirect"}]"#,
            ..turn(2, "what changed")
        })
        .unwrap();
        chat.append(&turn(3, "100% done?")).unwrap();
        assert_eq!(chat.find("login").unwrap().len(), 2);
        assert_eq!(
            chat.find("%").unwrap().len(),
            1,
            "a wildcard is a plain character"
        );
        assert!(chat.find("  ").unwrap().is_empty());
    }

    #[test]
    fn clear_removes_every_turn_and_topic_and_leaves_no_words_in_the_file() {
        const CANARY: &str = "qzxwvclearcanary";
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("j.sqlite");
        let storage = Storage::open(&path).unwrap();
        let chat = storage.chat();
        chat.append(&turn(1, &format!("what about {CANARY}")))
            .unwrap();
        chat.new_topic(2).unwrap();
        chat.append(&turn(3, "second")).unwrap();
        assert_eq!(
            chat.count().unwrap(),
            ChatCount {
                turns: 2,
                since_ms: Some(1)
            }
        );
        let cleared = chat.clear().unwrap();
        assert_eq!((cleared.turns, cleared.topics), (2, 2));
        assert_eq!(chat.count().unwrap().turns, 0);
        assert!(chat.page(None, 10).unwrap().turns.is_empty());
        drop(storage);
        for entry in std::fs::read_dir(dir.path()).unwrap().flatten() {
            let bytes = std::fs::read(entry.path()).unwrap();
            assert!(
                !bytes.windows(CANARY.len()).any(|w| w == CANARY.as_bytes()),
                "{} still holds it",
                entry.path().display()
            );
        }
    }

    #[test]
    fn a_long_question_is_cut_and_the_topic_takes_its_title_from_the_first() {
        let storage = Storage::open_in_memory().unwrap();
        let chat = storage.chat();
        let long = "x".repeat(QUESTION_MAX_CHARS + 50);
        let kept = chat.append(&turn(1, &long)).unwrap();
        assert_eq!(kept.question.chars().count(), QUESTION_MAX_CHARS);
        let title: String = storage
            .raw_conn()
            .query_row("SELECT title FROM chat_conversations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(title.chars().count(), TITLE_MAX_CHARS);
    }
}
