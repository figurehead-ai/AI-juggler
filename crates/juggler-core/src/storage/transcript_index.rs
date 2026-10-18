//! AJ-E1007: the local transcript search index.
//!
//! An FTS5 index over the text of each Instance's transcript, one row per
//! transcript line (message). The index is contentless: it holds the
//! search terms, not a second copy of the user's conversations. A hit
//! points back at `(session, byte offset, length)` in the original file,
//! and the snippet is read from there. Indexing is incremental: each
//! session remembers how many bytes of its transcript are indexed and
//! only reads what was appended. A file that shrank or moved is
//! reindexed from the start.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::CoreError;
use crate::model::SessionId;

/// One indexed line: where it is in its transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedLine {
    /// The Instance.
    pub session: String,
    /// Byte offset of the line in the transcript.
    pub offset: u64,
    /// Line length in bytes (without the newline).
    pub len: u64,
}

/// What the index already holds for one session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexState {
    /// The transcript path indexed.
    pub path: String,
    /// Bytes indexed from the start of the file.
    pub indexed_bytes: u64,
}

/// Typed access to the transcript index tables.
#[derive(Debug)]
pub struct TranscriptIndexRepo<'a> {
    conn: &'a Connection,
}

impl TranscriptIndexRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> TranscriptIndexRepo<'_> {
        TranscriptIndexRepo { conn }
    }

    /// What is indexed for `session`, if anything.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn state(&self, session: &SessionId) -> Result<Option<IndexState>, CoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT path, indexed_bytes FROM transcript_index_state WHERE session_id = ?1",
                params![session.as_str()],
                |row| {
                    Ok(IndexState {
                        path: row.get(0)?,
                        indexed_bytes: row.get::<_, i64>(1)?.unsigned_abs(),
                    })
                },
            )
            .optional()?)
    }

    /// Every session's index state (one query for the incremental pass).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn states(&self) -> Result<BTreeMap<String, IndexState>, CoreError> {
        let mut statement = self
            .conn
            .prepare("SELECT session_id, path, indexed_bytes FROM transcript_index_state")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                IndexState {
                    path: row.get(1)?,
                    indexed_bytes: row.get::<_, i64>(2)?.unsigned_abs(),
                },
            ))
        })?;
        rows.collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(CoreError::Storage)
    }

    /// Appends lines `(offset, len, text)` for `session` and records that
    /// `indexed_bytes` of `path` are now indexed — in one transaction.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn append(
        &self,
        session: &SessionId,
        path: &str,
        lines: &[(u64, u64, String)],
        indexed_bytes: u64,
    ) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        for (offset, len, text) in lines {
            tx.execute(
                "INSERT INTO transcript_lines (session_id, offset, len) VALUES (?1, ?2, ?3)",
                params![session.as_str(), offset.cast_signed(), len.cast_signed()],
            )?;
            let rowid = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO transcript_fts (rowid, body) VALUES (?1, ?2)",
                params![rowid, text],
            )?;
        }
        tx.execute(
            "INSERT INTO transcript_index_state (session_id, path, indexed_bytes)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(session_id) DO UPDATE SET path = ?2, indexed_bytes = ?3",
            params![session.as_str(), path, indexed_bytes.cast_signed()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Drops everything indexed for `session` (reindex or delete).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn forget(&self, session: &SessionId) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM transcript_fts WHERE rowid IN
                 (SELECT rowid FROM transcript_lines WHERE session_id = ?1)",
            params![session.as_str()],
        )?;
        tx.execute(
            "DELETE FROM transcript_lines WHERE session_id = ?1",
            params![session.as_str()],
        )?;
        tx.execute(
            "DELETE FROM transcript_index_state WHERE session_id = ?1",
            params![session.as_str()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Indexed line count for `session`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn line_count(&self, session: &SessionId) -> Result<u64, CoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM transcript_lines WHERE session_id = ?1",
            params![session.as_str()],
            |row| row.get(0),
        )?;
        Ok(count.unsigned_abs())
    }

    /// Full-text search: the best-ranked matching line per session, at
    /// most `limit` sessions. `query` is plain words; each becomes a
    /// quoted prefix term, all of which must match (never FTS syntax
    /// from the user).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<IndexedLine>, CoreError> {
        let Some(expression) = fts_expression(query) else {
            return Ok(Vec::new());
        };
        let mut statement = self.conn.prepare(
            "SELECT l.session_id, l.offset, l.len
               FROM transcript_fts f JOIN transcript_lines l ON l.rowid = f.rowid
              WHERE transcript_fts MATCH ?1
              ORDER BY f.rank",
        )?;
        let rows = statement.query_map(params![expression], |row| {
            Ok(IndexedLine {
                session: row.get(0)?,
                offset: row.get::<_, i64>(1)?.unsigned_abs(),
                len: row.get::<_, i64>(2)?.unsigned_abs(),
            })
        })?;
        let mut seen = std::collections::BTreeSet::new();
        let mut out = Vec::new();
        for row in rows {
            let line = row?;
            if seen.insert(line.session.clone()) {
                out.push(line);
                if out.len() >= limit {
                    break;
                }
            }
        }
        Ok(out)
    }
}

/// The FTS5 expression for plain user words: every word quoted (so no
/// operator or column syntax can be injected) as a prefix term.
#[must_use]
pub fn fts_expression(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| format!("\"{}\"*", word.to_lowercase()))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

/// The searchable text of one transcript line: for JSON lines, the
/// strings under `text`/`content` keys (message text, not ids, paths or
/// signatures); otherwise the line itself.
#[must_use]
pub fn line_text(line: &str) -> String {
    fn walk(value: &serde_json::Value, key: Option<&str>, out: &mut Vec<String>) {
        match value {
            serde_json::Value::String(text) => {
                if matches!(key, Some("text" | "content" | "thinking")) && !text.is_empty() {
                    out.push(text.clone());
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    walk(item, key, out);
                }
            }
            serde_json::Value::Object(map) => {
                for (name, inner) in map {
                    walk(inner, Some(name.as_str()), out);
                }
            }
            _ => {}
        }
    }
    match serde_json::from_str::<serde_json::Value>(line) {
        Ok(value) => {
            let mut out = Vec::new();
            walk(&value, None, &mut out);
            out.join(" ")
        }
        Err(_) => line.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{fts_expression, line_text};

    #[test]
    fn user_words_never_become_fts_syntax() {
        assert_eq!(
            fts_expression("Stripe webhook* OR NEAR(x)").as_deref(),
            Some("\"stripe\"* \"webhook\"* \"or\"* \"near\"* \"x\"*")
        );
        assert_eq!(fts_expression("  -- \" "), None);
    }

    #[test]
    fn json_lines_index_message_text_only() {
        let line = r#"{"type":"assistant","cwd":"/Users/x","message":{"content":[{"type":"text","text":"Fixed the webhook retry"}]},"uuid":"abc"}"#;
        assert_eq!(line_text(line), "Fixed the webhook retry");
        assert_eq!(line_text("plain log line"), "plain log line");
    }
}
