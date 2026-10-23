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

/// AJ-E1538: a search index in a file of its own: the opt-in index of
/// the sessions started outside AI Juggler. It has the main store's
/// index tables (schema v27, applied as it was), so
/// [`TranscriptIndexRepo`] reads and writes it; removing the file
/// ([`SearchIndexFile::remove`]) removes every trace, deleted pages
/// included.
#[derive(Debug)]
pub struct SearchIndexFile {
    conn: Connection,
}

impl SearchIndexFile {
    /// Opens (creating) the index at `path`, readable by the user only.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] when the file can't be opened or
    /// its tables made.
    pub fn open(path: &std::path::Path) -> Result<Self, CoreError> {
        let conn = Connection::open(path)?;
        Self::set_up(conn, Some(path))
    }

    /// An index in memory (the size estimate's sample, and tests).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn in_memory() -> Result<Self, CoreError> {
        Self::set_up(Connection::open_in_memory()?, None)
    }

    fn set_up(conn: Connection, path: Option<&std::path::Path>) -> Result<Self, CoreError> {
        conn.busy_timeout(super::BUSY_TIMEOUT)?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version == 0 {
            conn.execute_batch(super::V27)?;
            conn.pragma_update(None, "user_version", 1)?;
        }
        #[cfg(unix)]
        if let Some(path) = path {
            use std::os::unix::fs::PermissionsExt as _;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
        #[cfg(not(unix))]
        let _ = path;
        Ok(Self { conn })
    }

    /// Merges the search index into one segment: the smallest it can
    /// be, the fastest to search, and a size that depends on what it
    /// holds, not on how it was written.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn optimize(&self) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO transcript_fts(transcript_fts) VALUES('optimize')",
            [],
        )?;
        self.conn.execute_batch("VACUUM")?;
        Ok(())
    }

    /// Its tables.
    #[must_use]
    pub fn repo(&self) -> TranscriptIndexRepo<'_> {
        TranscriptIndexRepo::new(&self.conn)
    }

    /// The index's size in bytes (its pages, what the file holds).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn size_bytes(&self) -> Result<u64, CoreError> {
        let pages: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let size: i64 = self
            .conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))?;
        Ok((pages * size).unsigned_abs())
    }

    /// Deletes the index at `path` and the files SQLite keeps beside it.
    /// A file that isn't there is not an error.
    ///
    /// # Errors
    ///
    /// The first removal that fails for any other reason.
    pub fn remove(path: &std::path::Path) -> std::io::Result<()> {
        let name = path.as_os_str().to_owned();
        for suffix in ["", "-journal", "-wal", "-shm"] {
            let mut file = name.clone();
            file.push(suffix);
            match std::fs::remove_file(&file) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
                _ => {}
            }
        }
        Ok(())
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
    use super::{SearchIndexFile, fts_expression, line_text};
    use crate::model::SessionId;

    /// AJ-E1538: the file has the main store's index; searched the same
    /// way; removed whole, with SQLite's side files.
    #[test]
    fn a_search_index_file_is_the_same_index_and_removes_whole() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("outside-search.sqlite");
        {
            let index = SearchIndexFile::open(&path).unwrap();
            let key = SessionId::new("acc:conv-1");
            index
                .repo()
                .append(
                    &key,
                    "/t.jsonl",
                    &[(0, 10, "the webhook retry".to_owned())],
                    11,
                )
                .unwrap();
            let hits = index.repo().search("webhook", 10).unwrap();
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].session, "acc:conv-1");
            assert!(index.size_bytes().unwrap() > 0);
        }
        let reopened = SearchIndexFile::open(&path).unwrap();
        assert_eq!(
            reopened
                .repo()
                .line_count(&SessionId::new("acc:conv-1"))
                .unwrap(),
            1
        );
        drop(reopened);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        std::fs::write(dir.path().join("outside-search.sqlite-journal"), b"x").unwrap();
        SearchIndexFile::remove(&path).unwrap();
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            0,
            "nothing left behind"
        );
        SearchIndexFile::remove(&path).unwrap();
    }

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
