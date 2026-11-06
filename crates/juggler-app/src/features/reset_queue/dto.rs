//! The reset queue types on the wire (exported to TypeScript).

/// AJ-E611: one reset-queue row.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct ResetQueueInfo {
    /// Queue entry id.
    pub id: String,
    /// The session that hit the wall.
    pub session: String,
    /// Its account.
    pub account: String,
    /// Resume-at instant, epoch ms.
    pub resume_at_ms: i64,
    /// `queued|notified|ready|resumed|dismissed|stale`.
    pub state: String,
}
