//! The tables, in one batch a host applies as one migration of its own
//! (gyrus lives in the host's database: one file, one transaction with
//! the host's own records, one forget). [`install`] applies it on a
//! database of gyrus's own.
//!
//! - `gyrus_nodes`: an explicit `id INTEGER PRIMARY KEY`, so `VACUUM`
//!   never renumbers what the keyword index and the links point at;
//!   `indexed` is 1 while its label is in the keyword index, 2 once it was
//!   (a forget then rebuilds its bucket), 0 if never.
//! - `gyrus_links`: typed, both ends by node id, one per (src, dst, kind).
//! - `gyrus_aliases`: exact names a node is found by.
//! - `gyrus_vectors`: embeddings of length 1 as little-endian `f32`s,
//!   packed [`VECTOR_BLOCK`] nodes to a row (`id ÷ 64`, with the nodes' ids
//!   beside them), so the exact cosine scan in Rust reads a few large rows
//!   rather than one per node (3× faster at 10,000 vectors).
//! - `gyrus_fts_00` … `gyrus_fts_31`: the keyword index, contentless
//!   (the label lives only in `gyrus_nodes`), split by `id % 32` so a
//!   forget rebuilds only the one thirty-second that held the forgotten words
//!   (FTS5's `secure-delete` removes a row's terms but leaves page
//!   separators, which a rebuild clears). Each has `secure-delete` on;
//!   `gyrus_nodes_bucket` finds one bucket's rows without reading them all.
//! - `gyrus_meta`: the write generation (key `gen`), bumped by every write, so
//!   an in-memory index can tell it is behind.
//!
//! **Frozen once shipped:** a later version adds a new batch; this text
//! never changes (`tests/schema.rs` pins its hash).

/// How many keyword-index buckets there are (`id % FTS_BUCKETS`).
/// (The text of [`SCHEMA_V1`] spells the number out; a test checks they agree.)
pub const FTS_BUCKETS: i64 = 32;

/// How many nodes' vectors share one row of `gyrus_vectors`.
pub const VECTOR_BLOCK: i64 = 64;

/// gyrus's first schema. Every object `IF NOT EXISTS`, so it is safe to
/// run again.
pub const SCHEMA_V1: &str = r"
CREATE TABLE IF NOT EXISTS gyrus_nodes (
    id          INTEGER PRIMARY KEY,
    kind        TEXT NOT NULL,
    key         TEXT NOT NULL UNIQUE,
    label       TEXT NOT NULL DEFAULT '',
    untrusted   INTEGER NOT NULL DEFAULT 0,
    indexed     INTEGER NOT NULL DEFAULT 0,
    props       TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(props)),
    tier        TEXT NOT NULL,
    priority    INTEGER NOT NULL DEFAULT 1,
    salience    REAL NOT NULL DEFAULT 0.5,
    confidence  REAL NOT NULL DEFAULT 1,
    authority   TEXT NOT NULL,
    valid_from  INTEGER,
    valid_to    INTEGER,
    as_of       INTEGER NOT NULL,
    last_used   INTEGER,
    use_count   INTEGER NOT NULL DEFAULT 0,
    archived_at INTEGER,
    created     INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS gyrus_nodes_kind ON gyrus_nodes(kind);
CREATE INDEX IF NOT EXISTS gyrus_nodes_bucket ON gyrus_nodes ((id % 32)) WHERE indexed = 1;
CREATE TABLE IF NOT EXISTS gyrus_links (
    id          INTEGER PRIMARY KEY,
    src         INTEGER NOT NULL REFERENCES gyrus_nodes(id) ON DELETE CASCADE,
    dst         INTEGER NOT NULL REFERENCES gyrus_nodes(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,
    weight      REAL NOT NULL DEFAULT 1,
    confidence  REAL NOT NULL DEFAULT 1,
    props       TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(props)),
    valid_from  INTEGER,
    valid_to    INTEGER,
    provenance  TEXT NOT NULL DEFAULT '',
    created     INTEGER NOT NULL,
    UNIQUE (src, dst, kind)
);
CREATE INDEX IF NOT EXISTS gyrus_links_dst ON gyrus_links(dst);
CREATE TABLE IF NOT EXISTS gyrus_aliases (
    alias TEXT NOT NULL,
    node  INTEGER NOT NULL REFERENCES gyrus_nodes(id) ON DELETE CASCADE,
    PRIMARY KEY (alias, node)
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS gyrus_aliases_node ON gyrus_aliases(node);
CREATE TABLE IF NOT EXISTS gyrus_vectors (
    id    INTEGER PRIMARY KEY,
    dim   INTEGER NOT NULL,
    block INTEGER NOT NULL,
    ids   BLOB NOT NULL,
    v     BLOB NOT NULL,
    UNIQUE (dim, block)
);
CREATE TABLE IF NOT EXISTS gyrus_meta (
    k TEXT PRIMARY KEY,
    v INTEGER NOT NULL
) WITHOUT ROWID;
INSERT OR IGNORE INTO gyrus_meta (k, v) VALUES ('gen', 0), ('schema', 1);
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_00 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_01 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_02 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_03 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_04 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_05 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_06 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_07 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_08 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_09 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_10 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_11 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_12 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_13 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_14 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_15 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_16 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_17 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_18 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_19 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_20 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_21 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_22 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_23 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_24 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_25 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_26 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_27 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_28 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_29 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_30 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE IF NOT EXISTS gyrus_fts_31 USING fts5(label, content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2');
INSERT INTO gyrus_fts_00(gyrus_fts_00, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_01(gyrus_fts_01, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_02(gyrus_fts_02, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_03(gyrus_fts_03, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_04(gyrus_fts_04, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_05(gyrus_fts_05, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_06(gyrus_fts_06, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_07(gyrus_fts_07, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_08(gyrus_fts_08, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_09(gyrus_fts_09, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_10(gyrus_fts_10, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_11(gyrus_fts_11, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_12(gyrus_fts_12, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_13(gyrus_fts_13, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_14(gyrus_fts_14, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_15(gyrus_fts_15, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_16(gyrus_fts_16, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_17(gyrus_fts_17, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_18(gyrus_fts_18, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_19(gyrus_fts_19, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_20(gyrus_fts_20, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_21(gyrus_fts_21, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_22(gyrus_fts_22, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_23(gyrus_fts_23, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_24(gyrus_fts_24, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_25(gyrus_fts_25, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_26(gyrus_fts_26, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_27(gyrus_fts_27, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_28(gyrus_fts_28, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_29(gyrus_fts_29, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_30(gyrus_fts_30, rank) VALUES ('secure-delete', 1);
INSERT INTO gyrus_fts_31(gyrus_fts_31, rank) VALUES ('secure-delete', 1);
";

/// The keyword-index table of a bucket.
#[must_use]
pub fn fts_table(bucket: i64) -> String {
    format!("gyrus_fts_{:02}", bucket.rem_euclid(FTS_BUCKETS))
}

/// The bucket a node's keyword entry lives in.
#[must_use]
pub const fn bucket_of(id: crate::NodeId) -> i64 {
    id.rem_euclid(FTS_BUCKETS)
}

/// Applies [`SCHEMA_V1`] (for a database of gyrus's own; a host applies
/// it as one of its own migrations instead).
///
/// # Errors
///
/// SQLite errors.
pub fn install(conn: &rusqlite::Connection) -> crate::Result<()> {
    conn.execute_batch(SCHEMA_V1)?;
    Ok(())
}

/// gyrus's tables are in this database.
///
/// # Errors
///
/// SQLite errors.
pub fn installed(conn: &rusqlite::Connection) -> crate::Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'gyrus_nodes'",
        [],
        |row| row.get(0),
    )?;
    Ok(n > 0)
}
