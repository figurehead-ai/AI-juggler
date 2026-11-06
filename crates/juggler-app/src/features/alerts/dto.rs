//! The alerts types on the wire (exported to TypeScript).

use crate::features::quota::alert_words;

/// AJ-E611: one emitted alert on the wire (the notification region's
/// payload — polite live-region semantics client-side).
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct AlertEventInfo {
    /// `tray-amber|actionable|exhausted|reset-reached`.
    pub kind: String,
    /// Account the alert belongs to.
    pub account: String,
    /// The copy, in plain words (D-25: no window ids, no tier tags); a
    /// paired console shows it as it is.
    pub text: String,
    /// Emitted at, epoch ms.
    pub at_ms: i64,
    /// A quota alert's numbers, so this Mac's banner can say the reset
    /// in local time and offer its actions (wireframe 27).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota: Option<alert_words::AlertQuotaInfo>,
}
