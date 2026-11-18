//! Retrieval behind one trait (ADR-0037 §4). v1 is [`KeywordRetriever`]:
//! BM25 × priority × a recency half-life, no embeddings. Recall never
//! writes: a caller that includes or shows a record touches it afterwards
//! (`MemoryStore::touch`).

use std::collections::HashMap;

use rusqlite::params;

use super::store::{MemoryStore, RECORD_COLUMNS, Record, record_columns, record_row};
use super::types::{Authority, EntityRef, Kind, MemoryError, Scope, Tier};

/// The relevance a same-entity match gets at least (ADR-0037 §4.2).
const SAME_ENTITY_FLOOR: f64 = 0.5;
/// The relevance a word match gets at least. FTS5's BM25 gives a word
/// found in half the rows or more an IDF of almost 0, so in a small store
/// (a few notes, the usual case) a real match would score nothing and be
/// dropped as below [`SCORE_FLOOR`]; BM25 still orders the matches above
/// the floor.
const MATCH_FLOOR: f64 = 0.25;
/// Scores below this are dropped.
const SCORE_FLOOR: f64 = 0.05;
/// The most FTS matches scored per query.
const MATCH_CANDIDATES: u32 = 200;
/// The most words of a question searched.
const QUERY_WORDS: usize = 32;
const DAY_MS: f64 = 86_400_000.0;

/// What a caller wants recalled. The job's budget row picks `kinds`,
/// `min_authority` and `limit`; the text is the user's own words, never
/// agent text.
#[derive(Clone, Debug)]
pub struct RecallQuery<'a> {
    /// The user's words.
    pub text: &'a str,
    /// The entities the question resolved to (by code, aliases included).
    pub entities: &'a [EntityRef],
    /// Global, plus the project, pool or session in play.
    pub scopes: &'a [Scope],
    /// The kinds the job may use.
    pub kinds: &'a [Kind],
    /// The least authority recalled (0.5: model). Agent-authored rows are
    /// never recalled, whatever this says.
    pub min_authority: Authority,
    /// At most this many.
    pub limit: usize,
}

/// Why a record was recalled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecallReason {
    /// An always-on preference in scope.
    AlwaysOn,
    /// Its words matched.
    Matched,
    /// It names an entity of the question.
    SameEntity,
}

/// One recalled record.
#[derive(Clone, Debug, PartialEq)]
pub struct Recalled {
    /// The version's id.
    pub record: String,
    /// Its version.
    pub version: i64,
    /// One fact, one slot: its subject key.
    pub fact_key: String,
    /// Relevance × priority weight × recency.
    pub score: f64,
    /// Why.
    pub reason: RecallReason,
}

/// The one retrieval interface; callers see only this (ADR-0037 §4.1).
pub trait Retriever: Send + Sync {
    /// Ranked records for `q`. Pure over the store's read handle.
    ///
    /// # Errors
    ///
    /// Storage failures.
    fn recall(
        &self,
        store: &MemoryStore<'_>,
        q: &RecallQuery<'_>,
        now_ms: i64,
    ) -> Result<Vec<Recalled>, MemoryError>;
}

/// v1: keyword × priority × recency (ADR-0037 §4.2).
#[derive(Clone, Copy, Debug, Default)]
pub struct KeywordRetriever;

/// The question's words as an FTS5 query: each word quoted (so the user's
/// words are never FTS syntax), joined with OR.
#[must_use]
pub fn fts_query(text: &str) -> Option<String> {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .take(QUERY_WORDS)
        .map(|word| format!("\"{}\"", word.replace('"', "\"\"")))
        .collect();
    (!words.is_empty()).then(|| words.join(" OR "))
}

fn recency(record: &Record, now_ms: i64) -> f64 {
    let since = record.last_used_ms.unwrap_or(i64::MIN).max(record.as_of_ms);
    #[expect(
        clippy::cast_precision_loss,
        reason = "an age in days; precision past 2^52 ms is irrelevant"
    )]
    let days = (now_ms.saturating_sub(since)).max(0) as f64 / DAY_MS;
    let half_life = record.kind.half_life_days(record.tier == Tier::Long);
    0.5_f64.powf(days / half_life)
}

impl KeywordRetriever {
    fn eligible(record: &Record, q: &RecallQuery<'_>, now_ms: i64) -> bool {
        record.superseded_by.is_none()
            && !record.untrusted
            && record.authority != Authority::Agent
            && record.authority.rank() >= q.min_authority.rank()
            && q.kinds.contains(&record.kind)
            && q.scopes.contains(&record.scope)
            && record.expires_ms.is_none_or(|end| end > now_ms)
    }
}

impl Retriever for KeywordRetriever {
    fn recall(
        &self,
        store: &MemoryStore<'_>,
        q: &RecallQuery<'_>,
        now_ms: i64,
    ) -> Result<Vec<Recalled>, MemoryError> {
        let conn = store.conn;
        // subject key -> (record, relevance, reason)
        let mut found: HashMap<String, (Record, f64, RecallReason)> = HashMap::new();
        let mut offer = |record: Record, relevance: f64, reason: RecallReason| {
            if !Self::eligible(&record, q, now_ms) {
                return;
            }
            let entry = found
                .entry(record.subject_key.clone())
                .or_insert_with(|| (record, relevance, reason));
            if relevance > entry.1 {
                entry.1 = relevance;
                entry.2 = reason;
            }
        };
        // 1. Always on: current long-tier preferences in scope.
        if q.kinds.contains(&Kind::Preference) {
            let mut stmt = conn.prepare_cached(&format!(
                "SELECT {RECORD_COLUMNS} FROM memory_records
                 WHERE kind = 'preference' AND tier = 'long' AND superseded_by IS NULL
                   AND untrusted = 0"
            ))?;
            for record in stmt.query_map([], record_row)? {
                offer(record?, 1.0, RecallReason::AlwaysOn);
            }
        }
        // 2a. Matched: the question's words.
        if let Some(query) = fts_query(q.text) {
            let mut stmt = conn.prepare_cached(&format!(
                "SELECT {}, -bm25(memory_fts) FROM memory_fts
                 JOIN memory_records r ON r.seq = memory_fts.rowid
                 WHERE memory_fts MATCH ?1 AND r.superseded_by IS NULL AND r.untrusted = 0
                 ORDER BY rank LIMIT ?2",
                record_columns("r")
            ))?;
            let rows = stmt.query_map(params![query, MATCH_CANDIDATES], |row| {
                Ok((record_row(row)?, row.get::<_, f64>(24)?))
            })?;
            for row in rows {
                let (record, b) = row?;
                let b = b.max(0.0);
                offer(
                    record,
                    (b / (1.0 + b)).max(MATCH_FLOOR),
                    RecallReason::Matched,
                );
            }
        }
        // 2b. Same entity.
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM memory_entities e
             JOIN memory_records r ON r.id = e.record_id
             WHERE e.entity_kind = ?1 AND e.entity_id = ?2 AND r.superseded_by IS NULL
               AND r.untrusted = 0",
            record_columns("r")
        ))?;
        for entity in q.entities {
            for record in stmt.query_map(params![entity.kind.as_str(), entity.id], record_row)? {
                offer(record?, SAME_ENTITY_FLOOR, RecallReason::SameEntity);
            }
        }
        // 3–4. Score, one per fact key, drop the weak, keep the top.
        let mut out: Vec<Recalled> = found
            .into_values()
            .map(|(record, relevance, reason)| Recalled {
                score: relevance * record.priority.weight() * recency(&record, now_ms),
                record: record.id,
                version: record.version,
                fact_key: record.subject_key,
                reason,
            })
            .filter(|recalled| recalled.score >= SCORE_FLOOR)
            .collect();
        out.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.fact_key.cmp(&b.fact_key))
        });
        out.truncate(q.limit);
        Ok(out)
    }
}
