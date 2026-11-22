//! Digests on ADR-0037's store (ADR-0036 §1.9): one record of the digest
//! kind per digest, written by the digest's own writer, scope global,
//! authority "observed by AI Juggler" for the rollup, as-of the window's
//! end, kept 30 days. The agents' names and words inside are marked
//! untrusted. Its log event ("made", with the agents counted and why the
//! rollup wrote it) commits with it.

use juggler_core::storage::Storage;
use juggler_core::storage::memory::{
    Actor, Authority, Kind, KindWriter, MemoryError, NewEvent, NewRecord, Priority, RecordText,
    Scope, SecretScan, SourceKind, Stream, Tier, Untrusted,
};

use super::rollup::DigestInfo;

/// How long a digest is kept (like notices).
pub const KEEP_MS: i64 = 30 * 24 * 3_600_000;

/// A digest's subject key: its window's end.
#[must_use]
pub fn subject_key(end_ms: i64) -> String {
    format!("digest:{end_ms}")
}

/// Why the rollup wrote it, for the log (a code): the AI digest's flag is
/// off (D-131: "not yet"), or no engine writes digests in this build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// The job's flag is off.
    FlagOff,
    /// The flag is on, and no model writer is built for it.
    NoWriter,
}

impl Why {
    /// Its code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::FlagOff => "flag_off",
            Self::NoWriter => "no_writer",
        }
    }
}

/// Writes `digest` through the gate, its log event with it.
///
/// # Errors
///
/// The gate's refusal; storage failures.
pub fn remember(
    storage: &Storage,
    writer: &KindWriter,
    secrets: &dyn SecretScan,
    digest: &DigestInfo,
    why: Why,
    now_ms: i64,
) -> Result<(), MemoryError> {
    let agents = digest.lines.len();
    let payload = serde_json::json!({
        "digest": Untrusted { value: digest },
    });
    let record = NewRecord {
        subject_key: subject_key(digest.end_ms),
        kind: Kind::Digest,
        scope: Scope::Global,
        tier: Tier::Mid,
        priority: Priority::Normal,
        authority: Authority::App,
        source: SourceKind::DigestJob,
        source_id: Some(format!("{}-{}", digest.start_ms, digest.end_ms)),
        engine: None,
        confidence: 1.0,
        as_of_ms: digest.end_ms,
        source_rev: None,
        text: RecordText::Trusted(format!(
            "Digest · {agents} {}",
            if agents == 1 { "agent" } else { "agents" }
        )),
        payload,
        expires_ms: Some(digest.end_ms + KEEP_MS),
        entities: Vec::new(),
    };
    let mut event = NewEvent::new(now_ms, Stream::Digest, "made", Actor::App);
    event.payload = serde_json::json!({
        "agents": agents,
        "shown": digest.shown,
        "quiet": digest.quiet,
        "ai": digest.ai,
        "why": why.code(),
    });
    storage
        .memory()
        .remember_logged(writer, secrets, &record, &event, now_ms)
        .map(drop)
}

/// A stored digest's payload, read back (`None` for one this build can't
/// read).
#[must_use]
pub fn read(payload: &str) -> Option<DigestInfo> {
    let value: serde_json::Value = serde_json::from_str(payload).ok()?;
    serde_json::from_value(value.get("digest")?.get("$untrusted")?.clone()).ok()
}
