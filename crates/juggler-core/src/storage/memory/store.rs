//! The write gate and the store (ADR-0037 §1.3–§1.5): remember
//! (supersede, never overwrite), read back, forget.

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::erase::{erase_subject, reindex};
use super::log::{Actor, NewEvent, Stream, append_in};
use super::secure::{SecureHold, fold_wal};
use super::types::{
    Authority, EntityKind, EntityRef, Kind, KindWriter, MemoryError, Priority, RecordText, Refusal,
    Scope, SecretScan, SourceKind, Tier, UNTRUSTED_KEY, code_like,
};
use crate::storage::write_transaction;

/// The most characters a record's label holds.
pub const TEXT_MAX_CHARS: usize = 280;
/// The largest payload, in bytes of JSON.
pub const PAYLOAD_MAX_BYTES: usize = 16 * 1024;
/// The versions kept per subject key; older superseded ones are removed.
pub const KEPT_VERSIONS: i64 = 20;
/// A model-inferred record's least confidence (ADR-0037 §2.4).
pub const MODEL_CONFIDENCE_GATE: f64 = 0.8;
/// The longest subject key.
const SUBJECT_KEY_MAX: usize = 256;

/// A record to remember.
#[derive(Clone, Debug)]
pub struct NewRecord {
    /// The unit of supersede and forget (ADR-0037 §1.1), a code.
    pub subject_key: String,
    /// What it holds.
    pub kind: Kind,
    /// Where it applies.
    pub scope: Scope,
    /// How long it lives.
    pub tier: Tier,
    /// Its priority.
    pub priority: Priority,
    /// Whose words it holds.
    pub authority: Authority,
    /// Where it came from.
    pub source: SourceKind,
    /// The turn, window or view action (a code).
    pub source_id: Option<String>,
    /// The model's engine id, for a model-authored record (the log's actor).
    pub engine: Option<String>,
    /// 0..1; 1 for the user and the app.
    pub confidence: f64,
    /// When it was said, asked or observed.
    pub as_of_ms: i64,
    /// An answer's hash of its cited rows' state then.
    pub source_rev: Option<String>,
    /// The label shown and indexed, typed by whose words it holds.
    pub text: RecordText,
    /// Typed per kind, a JSON object.
    pub payload: serde_json::Value,
    /// When it ends on its own (digests, short-tier rows).
    pub expires_ms: Option<i64>,
    /// The entities it names.
    pub entities: Vec<EntityRef>,
}

/// One stored version.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// This version's id.
    pub id: String,
    /// Its subject key.
    pub subject_key: String,
    /// The subject's random id (the log and tombstones name it).
    pub subject_id: String,
    /// 1, 2, … per subject key.
    pub version: i64,
    /// What it holds.
    pub kind: Kind,
    /// Where it applies.
    pub scope: Scope,
    /// How long it lives.
    pub tier: Tier,
    /// Its priority.
    pub priority: Priority,
    /// Whose words.
    pub authority: Authority,
    /// Holds agent text.
    pub untrusted: bool,
    /// Where it came from.
    pub source: SourceKind,
    /// The source's id.
    pub source_id: Option<String>,
    /// 0..1.
    pub confidence: f64,
    /// When it was said, asked or observed: what "you asked this 2 days
    /// ago" is composed from (ADR-0037 §3).
    pub as_of_ms: i64,
    /// An answer's hash of its cited rows then.
    pub source_rev: Option<String>,
    /// The version it replaced.
    pub supersedes: Option<String>,
    /// The version that replaced it; `None` while current.
    pub superseded_by: Option<String>,
    /// The label.
    pub text: String,
    /// The payload.
    pub payload: serde_json::Value,
    /// Times used.
    pub use_count: i64,
    /// Last used.
    pub last_used_ms: Option<i64>,
    /// When it ends on its own.
    pub expires_ms: Option<i64>,
    /// When this version was written.
    pub created_ms: i64,
}

impl Record {
    /// How old what it says is, at `now_ms`.
    #[must_use]
    pub const fn age_ms(&self, now_ms: i64) -> i64 {
        now_ms.saturating_sub(self.as_of_ms)
    }
}

/// What a remember did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Remembered {
    /// The current version's id.
    pub id: String,
    /// Its version.
    pub version: i64,
    /// The subject's random id.
    pub subject_id: String,
    /// The same content was already current: nothing was written.
    pub unchanged: bool,
}

/// What a forget removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Forgotten {
    /// Subjects forgotten (an `asked` subject takes its `answer` with it).
    pub subjects: usize,
    /// Versions removed.
    pub versions: usize,
    /// Log sidecars erased.
    pub sidecars: usize,
    /// Chat turns the subject came from or was used in, redacted.
    pub turns: usize,
    /// Conversation titles taken from such a turn, renamed.
    pub titles: usize,
    /// The WAL was folded in and truncated (else another reader held it;
    /// the next full checkpoint does it).
    pub wal_folded: bool,
}

pub(super) const RECORD_COLUMNS: &str = "id, subject_key, subject_id, version, kind, scope_kind, \
    scope_id, tier, priority, authority, untrusted, source_kind, source_id, confidence, as_of_ms, \
    source_rev, supersedes, superseded_by, text, payload, use_count, last_used_ms, expires_ms, \
    created_ms";

/// [`RECORD_COLUMNS`] qualified by a table alias (for joins).
pub(super) fn record_columns(alias: &str) -> String {
    RECORD_COLUMNS
        .split(", ")
        .map(|column| format!("{alias}.{column}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn bad_code(column: usize, code: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        rusqlite::types::Type::Text,
        format!("unknown code {code}").into(),
    )
}

pub(super) fn record_row(row: &Row<'_>) -> rusqlite::Result<Record> {
    let code = |column: usize| row.get::<_, String>(column);
    let kind = code(4)?;
    let scope_kind = code(5)?;
    let tier = code(7)?;
    let authority = code(9)?;
    let source = code(11)?;
    let payload: String = row.get(19)?;
    Ok(Record {
        id: row.get(0)?,
        subject_key: row.get(1)?,
        subject_id: row.get(2)?,
        version: row.get(3)?,
        kind: Kind::parse(&kind).ok_or_else(|| bad_code(4, &kind))?,
        scope: Scope::from_columns(&scope_kind, row.get(6)?)
            .ok_or_else(|| bad_code(5, &scope_kind))?,
        tier: Tier::parse(&tier).ok_or_else(|| bad_code(7, &tier))?,
        priority: Priority::from_i64(row.get(8)?),
        authority: Authority::parse(&authority).ok_or_else(|| bad_code(9, &authority))?,
        untrusted: row.get(10)?,
        source: SourceKind::parse(&source).ok_or_else(|| bad_code(11, &source))?,
        source_id: row.get(12)?,
        confidence: row.get(13)?,
        as_of_ms: row.get(14)?,
        source_rev: row.get(15)?,
        supersedes: row.get(16)?,
        superseded_by: row.get(17)?,
        text: row.get(18)?,
        payload: serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null),
        use_count: row.get(20)?,
        last_used_ms: row.get(21)?,
        expires_ms: row.get(22)?,
        created_ms: row.get(23)?,
    })
}

/// The payload holds an [`super::Untrusted`] value somewhere.
fn holds_untrusted(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Array(items) => items.iter().any(holds_untrusted),
        serde_json::Value::Object(map) => {
            map.contains_key(UNTRUSTED_KEY) || map.values().any(holds_untrusted)
        }
        _ => false,
    }
}

/// Every string in the payload, for the secret pass.
fn strings(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(items) => items.iter().for_each(|item| strings(item, out)),
        serde_json::Value::Object(map) => {
            for (key, item) in map {
                out.push(key.clone());
                strings(item, out);
            }
        }
        _ => {}
    }
}

/// The gate's checks that need no database (ADR-0037 §1.3 steps 1, 2 and
/// 4–7); step 3, authority against the current row, runs in the write's
/// transaction.
fn gate(
    writer: &KindWriter,
    secrets: &dyn SecretScan,
    record: &NewRecord,
    untrusted: bool,
) -> Result<(), Refusal> {
    if !writer.covers(record.kind, record.authority, record.source) {
        return Err(Refusal::WrongWriter);
    }
    if !record.kind.is_built() || record.authority == Authority::Bot {
        return Err(Refusal::Reserved);
    }
    if !code_like(&record.subject_key, SUBJECT_KEY_MAX) {
        return Err(Refusal::SubjectKey);
    }
    let text = record.text.as_str();
    let chars = text.chars().count();
    if text.trim().is_empty() || chars > TEXT_MAX_CHARS {
        return Err(Refusal::TextBounds);
    }
    // A preference is a setting: it names an entity, in a short label
    // that is a `SettingLabel` (the security review's M3); free words are
    // a note.
    if record.kind == Kind::Preference
        && (record.entities.is_empty() || !matches!(record.text, RecordText::Setting(_)))
    {
        return Err(Refusal::PreferenceShape);
    }
    let payload_len = serde_json::to_string(&record.payload).map_or(usize::MAX, |s| s.len());
    if !record.payload.is_object() || payload_len > PAYLOAD_MAX_BYTES {
        return Err(Refusal::PayloadBounds);
    }
    let mut words = vec![text.to_owned()];
    strings(&record.payload, &mut words);
    if words.iter().any(|word| secrets.finds_secret(word)) {
        return Err(Refusal::Secret);
    }
    if record.authority == Authority::Agent && !untrusted {
        return Err(Refusal::AgentNotUntrusted);
    }
    let c = record.confidence;
    let confident = c.is_finite()
        && (0.0..=1.0).contains(&c)
        && match record.authority {
            Authority::User | Authority::App => (c - 1.0).abs() < f64::EPSILON,
            Authority::Model => c >= MODEL_CONFIDENCE_GATE,
            Authority::Agent | Authority::Bot => true,
        };
    if !confident {
        return Err(Refusal::Confidence);
    }
    Ok(())
}

fn actor_of(record: &NewRecord) -> Actor {
    match record.authority {
        Authority::User => Actor::User,
        Authority::Model => Actor::Model(record.engine.clone().unwrap_or_else(|| "model".into())),
        Authority::App | Authority::Agent | Authority::Bot => Actor::App,
    }
}

/// The subject keys a forget of `key` reaches: an `asked` subject takes
/// its `answer` with it (ADR-0037 §1.5 step 4).
fn linked_keys(key: &str) -> Vec<String> {
    let mut keys = vec![key.to_owned()];
    if let Some(suffix) = key.strip_prefix("asked:") {
        keys.push(format!("answer:{suffix}"));
    }
    keys
}

fn new_id(conn: &Connection, now_ms: i64) -> rusqlite::Result<String> {
    conn.query_row(
        "SELECT printf('%012x', ?1) || lower(hex(randomblob(10)))",
        [now_ms.max(0)],
        |row| row.get(0),
    )
}

/// The memory store over one connection, under the caller's writer lock
/// (ADR-0030). Every write runs in one `IMMEDIATE` transaction with its
/// log event, with `secure_delete` on.
#[derive(Debug)]
pub struct MemoryStore<'c> {
    pub(super) conn: &'c Connection,
}

impl<'c> MemoryStore<'c> {
    /// The store on `conn` (V54's tables must be there).
    #[must_use]
    pub const fn new(conn: &'c Connection) -> Self {
        Self { conn }
    }

    /// Remembers `record` through the write gate: a new subject is version
    /// 1; a changed one becomes version n+1, the old row marked superseded
    /// (never changed), the newest [`KEPT_VERSIONS`] kept. The same content
    /// again writes nothing (`unchanged`). The record and its log event
    /// commit together.
    ///
    /// # Errors
    ///
    /// [`MemoryError::Refused`] naming the gate's reason (the refusal is
    /// logged, never the text); storage failures.
    pub fn remember(
        &self,
        writer: &KindWriter,
        secrets: &dyn SecretScan,
        record: &NewRecord,
        now_ms: i64,
    ) -> Result<Remembered, MemoryError> {
        self.write(writer, secrets, record, None, now_ms)
    }

    /// [`Self::remember`], and `also` appended to the log in the same
    /// transaction, naming the version written (its `memory_subject` and
    /// `version` filled in), so a feature's own event commits with its
    /// record (ADR-0037 §7.1: a watch's `created` with its record,
    /// AJ-E3814). When the content was already current, nothing is
    /// written and `also` isn't appended.
    ///
    /// # Errors
    ///
    /// As [`Self::remember`]; [`Refusal::NotTextFree`] when `also` holds
    /// free text (nothing is written then).
    pub fn remember_logged(
        &self,
        writer: &KindWriter,
        secrets: &dyn SecretScan,
        record: &NewRecord,
        also: &NewEvent,
        now_ms: i64,
    ) -> Result<Remembered, MemoryError> {
        self.write(writer, secrets, record, Some(also), now_ms)
    }

    /// The current versions of one kind, newest change first (ties by id),
    /// at most `limit`, after `after` (the last row's `created_ms` and id):
    /// a keyset page (the Watches view's list).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn current_of_kind(
        &self,
        kind: Kind,
        after: Option<(i64, &str)>,
        limit: u32,
    ) -> Result<Vec<Record>, MemoryError> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {RECORD_COLUMNS} FROM memory_records
             WHERE kind = ?1 AND superseded_by IS NULL
               AND (?2 IS NULL OR created_ms < ?2 OR (created_ms = ?2 AND id > ?3))
             ORDER BY created_ms DESC, id ASC LIMIT ?4"
        ))?;
        let (at, id) = after.map_or((None, ""), |(at, id)| (Some(at), id));
        let rows = stmt
            .query_map(params![kind.as_str(), at, id, limit], record_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// The gate, then the write, with `also` appended in the same
    /// transaction when given.
    fn write(
        &self,
        writer: &KindWriter,
        secrets: &dyn SecretScan,
        record: &NewRecord,
        also: Option<&NewEvent>,
        now_ms: i64,
    ) -> Result<Remembered, MemoryError> {
        let untrusted = record.text.is_untrusted() || holds_untrusted(&record.payload);
        if let Err(refusal) = gate(writer, secrets, record, untrusted) {
            let tx = write_transaction(self.conn)?;
            log_refusal(&tx, record, refusal, now_ms)?;
            tx.commit()?;
            return Err(MemoryError::Refused(refusal));
        }
        let _hold = SecureHold::on(self.conn)?;
        let tx = write_transaction(self.conn)?;
        let current = current_in(&tx, &record.subject_key)?;
        if let Some(current) = &current {
            if record.authority.rank() < current.authority.rank() {
                log_refusal(&tx, record, Refusal::LowerAuthority, now_ms)?;
                tx.commit()?;
                return Err(MemoryError::Refused(Refusal::LowerAuthority));
            }
            if current.text == record.text.as_str()
                && current.payload == record.payload
                && current.scope == record.scope
                && current.authority == record.authority
                && current.untrusted == untrusted
            {
                return Ok(Remembered {
                    id: current.id.clone(),
                    version: current.version,
                    subject_id: current.subject_id.clone(),
                    unchanged: true,
                });
            }
        }
        let written = write_version(&tx, record, current.as_ref(), untrusted, now_ms)?;
        if let Some(also) = also {
            let mut event = also.clone();
            event.memory_subject = Some(written.subject_id.clone());
            event.version = Some(written.version);
            append_in(&tx, &event, None)?;
        }
        tx.commit()?;
        Ok(written)
    }

    /// The current version of a subject.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn current(&self, subject_key: &str) -> Result<Option<Record>, MemoryError> {
        Ok(current_in(self.conn, subject_key)?)
    }

    /// Every kept version of a subject, newest first.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn history(&self, subject_key: &str) -> Result<Vec<Record>, MemoryError> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {RECORD_COLUMNS} FROM memory_records WHERE subject_key = ?1 ORDER BY version DESC"
        ))?;
        let rows = stmt
            .query_map([subject_key], record_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// The entities a version names.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn entities(&self, record_id: &str) -> Result<Vec<EntityRef>, MemoryError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT entity_kind, entity_id FROM memory_entities WHERE record_id = ?1
             ORDER BY entity_kind, entity_id",
        )?;
        let rows = stmt
            .query_map([record_id], |row| {
                let kind: String = row.get(0)?;
                Ok(EntityRef {
                    kind: EntityKind::parse(&kind).ok_or_else(|| bad_code(0, &kind))?,
                    id: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Forgets a subject (ADR-0037 §1.5): every version of it (and of its
    /// `answer`, for an `asked` subject), their entity rows and index
    /// entries, the log's sidecars about it, and the words of the Chat turns
    /// it came from or was used in (redacted, a title taken from one
    /// renamed), in one transaction with `secure_delete` on; the index
    /// rebuilt, so no page separator keeps a prefix of its
    /// words; a tombstone (the subject's random id and when)
    /// so a restore of an older backup on this Mac drops it; one
    /// `memory.forgot` event (ids and counts only); then the WAL folded in
    /// and truncated.
    ///
    /// # Errors
    ///
    /// Storage failures (nothing is removed unless all of it is).
    pub fn forget(
        &self,
        subject_key: &str,
        actor: &Actor,
        now_ms: i64,
    ) -> Result<Forgotten, MemoryError> {
        let mut out = Forgotten::default();
        {
            let _hold = SecureHold::on(self.conn)?;
            let tx = write_transaction(self.conn)?;
            for key in linked_keys(subject_key) {
                let subjects: Vec<String> = tx
                    .prepare_cached(
                        "SELECT DISTINCT subject_id FROM memory_records WHERE subject_key = ?1",
                    )?
                    .query_map([&key], |row| row.get(0))?
                    .collect::<rusqlite::Result<_>>()?;
                for subject in subjects {
                    let erased = erase_subject(&tx, &subject)?;
                    out.subjects += 1;
                    out.versions += erased.versions;
                    out.sidecars += erased.sidecars;
                    out.turns += erased.turns;
                    out.titles += erased.titles;
                    tx.execute(
                        "INSERT INTO memory_forgotten (subject_id, forgotten_ms) VALUES (?1, ?2)
                         ON CONFLICT (subject_id) DO UPDATE SET forgotten_ms = excluded.forgotten_ms",
                        params![subject, now_ms],
                    )?;
                    let mut event = NewEvent::new(now_ms, Stream::Memory, "forgot", actor.clone());
                    event.memory_subject = Some(subject);
                    event.payload = serde_json::json!({
                        "versions": erased.versions,
                        "turns": erased.turns,
                    });
                    append_in(&tx, &event, None)?;
                }
            }
            if out.subjects > 0 {
                reindex(&tx)?;
            }
            tx.commit()?;
        }
        out.wal_folded = fold_wal(self.conn)?;
        Ok(out)
    }

    /// Links a Chat turn to a subject it used (the turn's receipt, ADR-0037
    /// §6), so forgetting the subject redacts the turn too. A turn that
    /// isn't there is skipped.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn link_turn(&self, turn_id: &str, subject_id: &str) -> Result<(), MemoryError> {
        let tx = write_transaction(self.conn)?;
        link_turn_in(&tx, turn_id, subject_id)?;
        tx.commit()?;
        Ok(())
    }

    /// Records that these current records were used (included in an
    /// assembled context or shown), ADR-0037 §4.3: `use_count + 1`,
    /// `last_used_ms`. Superseded or forgotten ids are skipped.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn touch(&self, ids: &[&str], now_ms: i64) -> Result<usize, MemoryError> {
        let _hold = SecureHold::on(self.conn)?;
        let tx = write_transaction(self.conn)?;
        let mut touched = 0;
        for id in ids {
            touched += tx.execute(
                "UPDATE memory_records SET use_count = use_count + 1, last_used_ms = ?2
                 WHERE id = ?1 AND superseded_by IS NULL",
                params![id, now_ms],
            )?;
        }
        tx.commit()?;
        Ok(touched)
    }
}

/// Writes version n+1 of a subject (or version 1) inside the caller's
/// transaction: the old row marked superseded first (so the one-current
/// index never sees two), the new row and its entities, the history bound,
/// and the log event.
fn write_version(
    tx: &Connection,
    record: &NewRecord,
    current: Option<&Record>,
    untrusted: bool,
    now_ms: i64,
) -> Result<Remembered, MemoryError> {
    let payload = serde_json::to_string(&record.payload)?;
    let (scope_kind, scope_id) = record.scope.columns();
    let id = new_id(tx, now_ms)?;
    let version = current.map_or(1, |c| c.version + 1);
    let subject_id = match current {
        Some(current) => current.subject_id.clone(),
        None => new_id(tx, now_ms)?,
    };
    if let Some(current) = current {
        // First, so the one-current index never sees two.
        tx.execute(
            "UPDATE memory_records SET superseded_by = ?1 WHERE id = ?2",
            params![id, current.id],
        )?;
    }
    tx.execute(
        "INSERT INTO memory_records (id, subject_key, subject_id, version, kind, scope_kind,
             scope_id, tier, priority, authority, untrusted, source_kind, source_id, confidence,
             as_of_ms, source_rev, supersedes, text, payload, expires_ms, created_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                 ?18, ?19, ?20, ?21)",
        params![
            id,
            record.subject_key,
            subject_id,
            version,
            record.kind.as_str(),
            scope_kind,
            scope_id,
            record.tier.as_str(),
            record.priority.as_i64(),
            record.authority.as_str(),
            untrusted,
            record.source.as_str(),
            record.source_id,
            record.confidence,
            record.as_of_ms,
            record.source_rev,
            current.map(|c| c.id.as_str()),
            record.text.as_str(),
            payload,
            record.expires_ms,
            now_ms,
        ],
    )?;
    for entity in &record.entities {
        tx.execute(
            "INSERT OR IGNORE INTO memory_entities (record_id, entity_kind, entity_id)
             VALUES (?1, ?2, ?3)",
            params![id, entity.kind.as_str(), entity.id],
        )?;
    }
    if record.source == SourceKind::ChatTurn
        && let Some(turn) = &record.source_id
    {
        link_turn_in(tx, turn, &subject_id)?;
    }
    tx.execute(
        "DELETE FROM memory_records WHERE subject_key = ?1 AND version <= ?2",
        params![record.subject_key, version - KEPT_VERSIONS],
    )?;
    let mut event = NewEvent::new(
        now_ms,
        Stream::Memory,
        if version == 1 { "remembered" } else { "edited" },
        actor_of(record),
    );
    event.subject = Some(("memory", id.clone()));
    event.memory_subject = Some(subject_id.clone());
    event.version = Some(version);
    event.payload = serde_json::json!({
        "kind": record.kind.as_str(),
        "tier": record.tier.as_str(),
        "authority": record.authority.as_str(),
        "untrusted": untrusted,
    });
    append_in(tx, &event, None)?;
    Ok(Remembered {
        id,
        version,
        subject_id,
        unchanged: false,
    })
}

fn link_turn_in(conn: &Connection, turn_id: &str, subject_id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO chat_turn_memory (turn_id, subject_id)
         SELECT id, ?2 FROM chat_turns WHERE id = ?1",
        params![turn_id, subject_id],
    )
    .map(drop)
}

fn current_in(conn: &Connection, subject_key: &str) -> rusqlite::Result<Option<Record>> {
    conn.query_row(
        &format!(
            "SELECT {RECORD_COLUMNS} FROM memory_records
             WHERE subject_key = ?1 AND superseded_by IS NULL"
        ),
        [subject_key],
        record_row,
    )
    .optional()
}

/// A refusal's log event: the kind and the reason, never the text.
fn log_refusal(
    conn: &Connection,
    record: &NewRecord,
    refusal: Refusal,
    now_ms: i64,
) -> Result<(), MemoryError> {
    let mut event = NewEvent::new(now_ms, Stream::Memory, "refused", actor_of(record));
    event.payload = serde_json::json!({
        "kind": record.kind.as_str(),
        "authority": record.authority.as_str(),
        "reason": refusal.code(),
    });
    append_in(conn, &event, None).map(|_| ())
}
