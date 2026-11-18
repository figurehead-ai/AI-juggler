//! V54 (ADR-0037 §1.2, §1.6, §1.7, §7.1): every 0.5 table of the memory
//! store and the AI log, in one migration that 0.6 only adds to.
//!
//! **Registered by the 0.4.6 → 0.5 merge.** The runner's version is the
//! index in `MIGRATIONS` + 1, so versions are contiguous by construction:
//! V54 can't be listed before release/0.4.6's V53 is. Until that merge
//! this batch is applied only by the tests (and `installed` reports the
//! tables absent, so every hook is a no-op on a real database). The test
//! `v54_waits_for_v53` fails the moment `MIGRATIONS` reaches V53, as the
//! reminder to list `memory::V54` right after it; `scripts/migration_clash.py`
//! fails if another branch claims 54 meanwhile.
//!
//! Where it differs from ADR-0037 §1.2's sketch, and why:
//! - `memory_records` has an explicit `seq INTEGER PRIMARY KEY` and `id`
//!   is `UNIQUE`: `memory_fts` is an external-content index keyed by
//!   rowid, and `VACUUM` (the backup's snapshot and scrub) may renumber an
//!   implicit rowid, which would point the index at other rows.
//! - `subject_id`, a random id per subject (shared by its versions): the
//!   log, its sidecars and the forget tombstones name a subject by it,
//!   never by its key (a note's key is a hash of its words, so a key kept
//!   in the append-only spine would outlive the forget as a fingerprint).
//! - `chat_turn_memory` and `chat_conversations.title_turn`: the turns a
//!   subject came from or was used in (the receipts' link, written with
//!   the record and by the turn's receipt), and the turn a conversation's
//!   title came from, so a forget redacts them too (the security review's
//!   H2).
//! - `supersedes … ON DELETE SET NULL`: removing an old version (the
//!   20-version bound, the cloud backup's scrub) cuts the newer version's
//!   link to it, the one change the content trigger allows on that column.

/// The V54 batch. Applied in one `IMMEDIATE` transaction by the runner.
pub const V54: &str = r"
CREATE TABLE memory_records (
    seq           INTEGER PRIMARY KEY,
    id            TEXT NOT NULL UNIQUE,
    subject_key   TEXT NOT NULL,
    subject_id    TEXT NOT NULL,
    version       INTEGER NOT NULL CHECK (version >= 1),
    kind          TEXT NOT NULL,
    scope_kind    TEXT NOT NULL,
    scope_id      TEXT,
    tier          TEXT NOT NULL,
    priority      INTEGER NOT NULL,
    authority     TEXT NOT NULL,
    untrusted     INTEGER NOT NULL,
    source_kind   TEXT NOT NULL,
    source_id     TEXT,
    confidence    REAL NOT NULL,
    as_of_ms      INTEGER NOT NULL,
    source_rev    TEXT,
    supersedes    TEXT REFERENCES memory_records(id) ON DELETE SET NULL DEFERRABLE INITIALLY DEFERRED,
    superseded_by TEXT REFERENCES memory_records(id) DEFERRABLE INITIALLY DEFERRED,
    text          TEXT NOT NULL,
    payload       TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(payload)),
    use_count     INTEGER NOT NULL DEFAULT 0,
    last_used_ms  INTEGER,
    expires_ms    INTEGER,
    created_ms    INTEGER NOT NULL,
    UNIQUE (subject_key, version)
);
CREATE UNIQUE INDEX memory_one_current ON memory_records(subject_key) WHERE superseded_by IS NULL;
CREATE INDEX memory_by_kind_scope ON memory_records(kind, scope_kind, scope_id) WHERE superseded_by IS NULL;
CREATE INDEX memory_by_tier_use ON memory_records(tier, last_used_ms);
CREATE INDEX memory_by_subject_id ON memory_records(subject_id);
CREATE INDEX memory_by_supersedes ON memory_records(supersedes);
CREATE INDEX memory_by_superseded_by ON memory_records(superseded_by);

CREATE TABLE memory_entities (
    record_id   TEXT NOT NULL REFERENCES memory_records(id) ON DELETE CASCADE,
    entity_kind TEXT NOT NULL,
    entity_id   TEXT NOT NULL,
    PRIMARY KEY (record_id, entity_kind, entity_id)
) WITHOUT ROWID;
CREATE INDEX memory_entities_by_entity ON memory_entities(entity_kind, entity_id);

CREATE TABLE memory_forgotten (
    subject_id   TEXT PRIMARY KEY,
    forgotten_ms INTEGER NOT NULL
) WITHOUT ROWID;

CREATE VIRTUAL TABLE memory_fts USING fts5(
    text, content = 'memory_records', content_rowid = 'seq',
    tokenize = 'unicode61 remove_diacritics 2'
);
INSERT INTO memory_fts(memory_fts, rank) VALUES ('secure-delete', 1);

CREATE TRIGGER memory_fts_insert AFTER INSERT ON memory_records
WHEN new.superseded_by IS NULL AND new.untrusted = 0 BEGIN
    INSERT INTO memory_fts(rowid, text) VALUES (new.seq, new.text);
END;
CREATE TRIGGER memory_fts_superseded AFTER UPDATE OF superseded_by ON memory_records
WHEN old.superseded_by IS NULL AND new.superseded_by IS NOT NULL AND old.untrusted = 0 BEGIN
    INSERT INTO memory_fts(memory_fts, rowid, text) VALUES ('delete', old.seq, old.text);
END;
CREATE TRIGGER memory_fts_delete AFTER DELETE ON memory_records
WHEN old.superseded_by IS NULL AND old.untrusted = 0 BEGIN
    INSERT INTO memory_fts(memory_fts, rowid, text) VALUES ('delete', old.seq, old.text);
END;

CREATE TRIGGER memory_records_frozen BEFORE UPDATE ON memory_records
WHEN new.seq IS NOT old.seq OR new.id IS NOT old.id
  OR new.subject_key IS NOT old.subject_key OR new.subject_id IS NOT old.subject_id
  OR new.version IS NOT old.version OR new.kind IS NOT old.kind
  OR new.scope_kind IS NOT old.scope_kind OR new.scope_id IS NOT old.scope_id
  OR new.authority IS NOT old.authority OR new.untrusted IS NOT old.untrusted
  OR new.source_kind IS NOT old.source_kind OR new.source_id IS NOT old.source_id
  OR new.confidence IS NOT old.confidence OR new.as_of_ms IS NOT old.as_of_ms
  OR new.source_rev IS NOT old.source_rev OR new.text IS NOT old.text
  OR new.payload IS NOT old.payload OR new.created_ms IS NOT old.created_ms
  OR (new.supersedes IS NOT old.supersedes AND new.supersedes IS NOT NULL)
  OR (old.superseded_by IS NOT NULL AND new.superseded_by IS NOT old.superseded_by)
  OR new.superseded_by = old.id
BEGIN
    SELECT RAISE(ABORT, 'memory_records: a record is superseded, never changed');
END;
CREATE TRIGGER memory_reserved_kinds BEFORE INSERT ON memory_records
WHEN new.kind IN ('policy', 'decision', 'action', 'outcome') OR new.authority = 'bot' BEGIN
    SELECT RAISE(ABORT, 'memory_records: reserved for the 0.6 bot');
END;
CREATE TRIGGER memory_agent_untrusted BEFORE INSERT ON memory_records
WHEN new.authority = 'agent' AND new.untrusted = 0 BEGIN
    SELECT RAISE(ABORT, 'memory_records: agent text is always untrusted');
END;
CREATE TRIGGER memory_entities_frozen BEFORE UPDATE ON memory_entities BEGIN
    SELECT RAISE(ABORT, 'memory_entities: written with their record, never changed');
END;

CREATE TABLE chat_conversations (
    id              TEXT PRIMARY KEY,
    started_ms      INTEGER NOT NULL,
    last_ms         INTEGER NOT NULL,
    title           TEXT NOT NULL,
    title_turn      TEXT,
    has_agent_words INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE chat_turns (
    id              TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES chat_conversations(id) ON DELETE CASCADE,
    seq             INTEGER NOT NULL,
    at_ms           INTEGER NOT NULL,
    question        TEXT NOT NULL,
    outcome         TEXT NOT NULL,
    shown           TEXT,
    cites           TEXT NOT NULL DEFAULT '[]',
    entities        TEXT NOT NULL DEFAULT '[]',
    has_agent_words INTEGER NOT NULL DEFAULT 0,
    redacted        INTEGER NOT NULL DEFAULT 0,
    UNIQUE (conversation_id, seq)
);
CREATE TABLE chat_turn_memory (
    turn_id    TEXT NOT NULL REFERENCES chat_turns(id) ON DELETE CASCADE,
    subject_id TEXT NOT NULL,
    PRIMARY KEY (turn_id, subject_id)
) WITHOUT ROWID;
CREATE INDEX chat_turn_memory_by_subject ON chat_turn_memory(subject_id);

CREATE TABLE ai_log (
    seq            INTEGER PRIMARY KEY AUTOINCREMENT,
    at_ms          INTEGER NOT NULL,
    stream         TEXT NOT NULL,
    verb           TEXT NOT NULL,
    actor          TEXT NOT NULL,
    subject_kind   TEXT,
    subject_id     TEXT,
    memory_subject TEXT,
    version        INTEGER,
    correlation    TEXT,
    cause_seq      INTEGER,
    schema         INTEGER NOT NULL,
    payload        TEXT NOT NULL DEFAULT '{}'
                   CHECK (json_valid(payload) AND length(CAST(payload AS BLOB)) <= 4096)
);
CREATE INDEX ai_log_stream_at ON ai_log(stream, at_ms);
CREATE INDEX ai_log_subject ON ai_log(subject_kind, subject_id);
CREATE TABLE ai_log_text (
    seq            INTEGER PRIMARY KEY REFERENCES ai_log(seq) ON DELETE CASCADE,
    memory_subject TEXT,
    body           TEXT NOT NULL CHECK (length(CAST(body AS BLOB)) <= 2048),
    untrusted      INTEGER NOT NULL,
    expires_ms     INTEGER NOT NULL
);
CREATE INDEX ai_log_text_by_subject ON ai_log_text(memory_subject) WHERE memory_subject IS NOT NULL;
CREATE INDEX ai_log_text_by_expiry ON ai_log_text(expires_ms);
CREATE TABLE ai_log_prune (
    stream    TEXT PRIMARY KEY,
    before_ms INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TRIGGER ai_log_no_update BEFORE UPDATE ON ai_log BEGIN
    SELECT RAISE(ABORT, 'ai_log: the log is append-only');
END;
CREATE TRIGGER ai_log_no_early_delete BEFORE DELETE ON ai_log
WHEN NOT EXISTS (SELECT 1 FROM ai_log_prune p WHERE p.stream = old.stream AND old.at_ms < p.before_ms)
BEGIN
    SELECT RAISE(ABORT, 'ai_log: only rows older than their stream''s watermark may be removed');
END;
CREATE TRIGGER ai_log_text_no_update BEFORE UPDATE ON ai_log_text BEGIN
    SELECT RAISE(ABORT, 'ai_log_text: a sidecar is erased, never changed');
END;
CREATE TRIGGER ai_log_prune_forward BEFORE UPDATE ON ai_log_prune
WHEN new.stream IS NOT old.stream OR new.before_ms < old.before_ms BEGIN
    SELECT RAISE(ABORT, 'ai_log_prune: a watermark only moves forward');
END;
CREATE TRIGGER ai_log_prune_kept BEFORE DELETE ON ai_log_prune BEGIN
    SELECT RAISE(ABORT, 'ai_log_prune: a watermark is never removed');
END;
";

/// The store's tables are in this database (V54 applied).
///
/// # Errors
///
/// The SQLite error when the schema can't be read.
pub fn installed(conn: &rusqlite::Connection) -> rusqlite::Result<bool> {
    installed_in(conn, "main")
}

/// [`installed`] for an attached schema (`prior` in a restore).
pub(super) fn installed_in(conn: &rusqlite::Connection, schema: &str) -> rusqlite::Result<bool> {
    let sql = format!(
        "SELECT COUNT(*) FROM \"{}\".sqlite_master WHERE type = 'table' AND name = 'memory_records'",
        schema.replace('"', "")
    );
    conn.query_row(&sql, [], |row| row.get::<_, i64>(0))
        .map(|n| n > 0)
}
