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
//! Reviewed independently in `docs/security/review-memory-forget-step1.md`;
//! its highs and mediums are fixed here (named below). What forget must
//! leave: no trace of the forgotten words anywhere in the database file,
//! its WAL, the FTS index, the log, Chat's turns, or any copy AI Juggler
//! keeps.
//!
//! 1. **Rows.** One `IMMEDIATE` transaction erases every version of the
//!    subject (and of its `answer`, for `asked`), its entities, the turns'
//!    links. Nothing is removed unless all of it is.
//! 2. **Freed bytes.** Every memory and log write runs inside
//!    `secure::SecureHold` (`PRAGMA secure_delete` and `recursive_triggers`
//!    on, restored after), so SQLite zeroes the deleted cells and every page
//!    they free, and a `REPLACE` fires the index's delete trigger (L3).
//! 3. **The index.** `memory_fts` uses FTS5's `secure-delete`, which removes
//!    a row's terms from the leaves but not the page separators in
//!    `memory_fts_idx` (H1). So forget rewrites the index (`optimize`) in
//!    the same transaction, and every backup scrub builds it fresh from the
//!    rows left. Untrusted rows are never indexed; the index is keyed by an
//!    explicit `seq`, so a `VACUUM` can't misalign it.
//! 4. **Chat (H2).** The turns the subject came from (`source_id`) or was
//!    used in (`chat_turn_memory`, the receipts' link) are redacted (the
//!    question emptied, the shown answer cleared), their sidecars erased,
//!    and a conversation title taken from one renamed from the first turn
//!    left, else [`NEUTRAL_TITLE`].
//! 5. **The WAL.** After the commit, `wal_checkpoint(TRUNCATE)`. **Residual
//!    (L1):** another connection's open read can keep it from finishing
//!    ([`Forgotten::wal_folded`] is `false`); the frames stay until the
//!    next full checkpoint. Maintenance (step 3) retries it.
//! 6. **The log.** The spine holds codes only ([`log::text_free`]) and names
//!    a subject by its random `subject_id`, never its key. Sidecars go with
//!    the subject: by that id, by an event naming one of its versions (L4),
//!    or one of its turns.
//! 7. **Copies.** AI Juggler's own copies (before a migration, an update or
//!    a restore) hold no memory at all ([`backup::scrub_copy`],
//!    [`backup::Destination::OwnCopy`]); the before-restore copy goes after
//!    a day (H3). A restore applies this Mac's tombstones to the staged
//!    backup before it replaces anything, and again after (M1). The cloud
//!    copy is a filtered export: the stated settings only, written fresh
//!    into emptied tables and a fresh index (H1), and a stated setting is
//!    typed (a watch, or a preference naming an entity in at most 60
//!    characters, by the user, trusted, from a setting's source: M2, M3).
//! 8. **The writers (M2).** A [`KindWriter`] can't be made outside this
//!    module; [`Writers::issue`] hands each owner its one writer once, and
//!    the gate refuses a kind, authority or source it doesn't cover. Agent
//!    words in a label are [`RecordText::Untrusted`], by type.
//! 9. **Out of reach, by design:** backup files the user exported before
//!    the forget (the backup page and the forget sheet say so); the file
//!    system's own copies (APFS snapshots, Time Machine) of freed blocks;
//!    the `memory.forgot` spine row (a random id and counts).
//! 10. **Checked by tests,** each failing without its fix:
//!     `forget_leaves_no_trace_in_the_file` (raw file and WAL bytes, the
//!     turns and titles, `fts5vocab`), `forget_clears_the_index_page_separators`
//!     (fails without `optimize`), `the_cloud_copy_holds_no_local_only_or_forgotten_word`
//!     (fails without the fresh index), and the app's
//!     `a_restore_drops_the_forgotten_and_keeps_no_copy_holding_it`.

pub mod backup;
mod erase;
pub mod log;
mod retrieval;
pub mod schema;
mod secure;
mod store;
mod types;

#[cfg(test)]
mod tests;

pub use erase::NEUTRAL_TITLE;
pub use log::{Actor, AiLog, Event, NewEvent, NewSidecar, Sidecar, Stream};
pub use retrieval::{KeywordRetriever, RecallQuery, RecallReason, Recalled, Retriever, fts_query};
pub use schema::{V54, installed};
pub use store::{
    Forgotten, KEPT_VERSIONS, MODEL_CONFIDENCE_GATE, MemoryStore, NewRecord, PAYLOAD_MAX_BYTES,
    Record, Remembered, TEXT_MAX_CHARS,
};
pub use types::{
    Authority, EntityKind, EntityRef, Kind, KindWriter, MemoryError, PREFERENCE_MAX_CHARS,
    Priority, RecordText, Refusal, Scope, SecretScan, SourceKind, Tier, UNTRUSTED_KEY, Untrusted,
    Writers, code_like,
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
