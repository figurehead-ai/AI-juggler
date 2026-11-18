//! The memory store and the append-only AI log (ADR-0037, AJ-E3819 step 1):
//! one store for Chat's memory, ADR-0036's watches and digests, and the
//! 0.6 bot; one log of what happened, by whom, to which id.
//!
//! - **One table, one gate.** Every record has the same columns; its kind
//!   decides its payload and who may write it ([`KindWriter`]). Every write
//!   passes [`MemoryStore::remember`]'s gate: the writer, the kind built in
//!   0.5, authority against the current row (user > app > model > agent),
//!   no secrets (refused, never masked), the bounds, agent text untrusted
//!   by type ([`Untrusted`]), the model confidence gate.
//! - **Supersede, never overwrite.** One current row per subject key (a
//!   partial unique index); a change is version n+1 and the old row's
//!   `superseded_by`; a trigger refuses any other change to a written row.
//! - **Forget really deletes** ([`MemoryStore::forget`]); see the review
//!   below.
//! - **The log's spine is append-only and text-free; its words are in
//!   erasable sidecars** ([`AiLog`]).
//! - **Retrieval behind one trait** ([`Retriever`]; v1 [`KeywordRetriever`]).
//! - **Backups** follow D-133's ruling ([`backup`]): the stated watches and
//!   preferences travel as settings, Chat's words only in local backups.
//! - Nothing here crosses to the UI yet (no command, no `ts-rs` type).
//!
//! The tables are V54 ([`V54`]); until the 0.4.6 → 0.5 merge lists it in
//! the runner, only the tests apply it ([`schema`]'s note).
//!
//! # Security self-review: the forget and secure-delete path
//!
//! For the independent review before merge. What forget must leave: no
//! trace of the forgotten words anywhere in the database file, its WAL,
//! the FTS index, the log, or any copy AI Juggler makes afterwards.
//!
//! 1. **Rows.** One `IMMEDIATE` transaction deletes every version of the
//!    subject (and of its `answer`, for `asked`), cascading to
//!    `memory_entities`. Nothing is removed unless all of it is.
//! 2. **Freed bytes.** Every memory and log write runs inside
//!    `secure::SecureHold` (`PRAGMA secure_delete = ON`, restored after),
//!    so SQLite zeroes the deleted cells and every page they free —
//!    including FTS segment pages freed by an automatic merge during an
//!    ordinary `remember`, which is why the hold isn't only on forget.
//! 3. **The index.** `memory_fts` is created with FTS5's `secure-delete`
//!    option: a delete removes the row's terms from the segments at once
//!    rather than leaving a tombstone until a merge. Untrusted rows are
//!    never indexed. The external-content index is keyed by an explicit
//!    `seq INTEGER PRIMARY KEY`, so a `VACUUM` can't misalign it.
//! 4. **The WAL.** After the commit, `wal_checkpoint(TRUNCATE)` folds the
//!    zeroed pages into the file and truncates the WAL, so frames holding
//!    the old page images are gone. **Residual risk:** another connection's
//!    open read transaction can keep the checkpoint from finishing
//!    ([`Forgotten::wal_folded`] is then `false`); the frames stay until
//!    the next full checkpoint. Maintenance (AJ-E3819 step 3) must retry it.
//! 5. **The log.** The spine never holds words (codes checked by
//!    [`log::text_free`]) and names a subject only by its random
//!    `subject_id`, never its key (a note's key is a hash of its words; in
//!    the 90-day spine it would be a fingerprint). Sidecars about the
//!    subject are deleted in the same transaction.
//! 6. **Copies.** A pre-migration copy is emptied of memory, conversations
//!    and sidecars before it is named ([`backup::scrub_copy`]). A backup is
//!    a `VACUUM INTO` snapshot taken after the forget, so the subject isn't
//!    in it; its scrub runs with `secure_delete` and compacts. A forget
//!    leaves a tombstone (the random `subject_id`, no words) that a restore
//!    on this Mac honours ([`backup::after_restore`]).
//! 7. **Out of reach, by design:** backup files made before the forget
//!    (the backup page says so, ADR-0037 §1.8); the file system's own
//!    copies (APFS snapshots, Time Machine) of freed blocks; the
//!    `memory.forgot` spine row, which says only that a subject with that
//!    random id lost N versions.
//! 8. **Checked by tests:** `forget_leaves_no_trace_in_the_file` reads the
//!    raw database and WAL bytes for a canary (in the text, the FTS index,
//!    a sidecar and the subject key's words) after a forget, having first
//!    proved the same search finds it before; the FTS vocabulary is read
//!    through `fts5vocab` too.

pub mod backup;
pub mod log;
mod retrieval;
pub mod schema;
mod secure;
mod store;
mod types;

#[cfg(test)]
mod tests;

pub use log::{Actor, AiLog, Event, NewEvent, NewSidecar, Sidecar, Stream};
pub use retrieval::{KeywordRetriever, RecallQuery, RecallReason, Recalled, Retriever, fts_query};
pub use schema::{V54, installed};
pub use store::{
    Forgotten, KEPT_VERSIONS, MODEL_CONFIDENCE_GATE, MemoryStore, NewRecord, PAYLOAD_MAX_BYTES,
    Record, Remembered, TEXT_MAX_CHARS,
};
pub use types::{
    Authority, EntityKind, EntityRef, Kind, KindWriter, MemoryError, Priority, Refusal, Scope,
    SecretScan, SourceKind, Tier, UNTRUSTED_KEY, Untrusted, code_like,
};

impl super::Storage {
    /// The memory store (ADR-0037). V54's tables must be there
    /// ([`installed`]); until the 0.4.6 → 0.5 merge registers V54 they
    /// aren't on a real database, and every call fails with "no such table".
    #[must_use]
    pub fn memory(&self) -> MemoryStore<'_> {
        MemoryStore::new(&self.conn)
    }

    /// The AI log (ADR-0037 §7), on the same terms as [`Self::memory`].
    #[must_use]
    pub fn ai_log(&self) -> AiLog<'_> {
        AiLog::new(&self.conn)
    }
}
