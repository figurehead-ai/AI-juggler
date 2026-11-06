//! The ranking types on the wire (exported to TypeScript).

/// AJ-E611: one ranked (or filtered) row on the wire — greyed rows
/// carry their filter reason; survivors carry the score + their
/// explanation (top-2 factors in words; the full five in `factors`).
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct RankedRowInfo {
    /// Account id.
    pub account: String,
    /// The account's name as the user gave it (rows never show the id).
    pub account_label: String,
    /// The weighted score, when the account survived the hard filters.
    pub score: Option<f64>,
    /// The hard filter's reason (greyed row copy).
    pub filtered_reason: Option<String>,
    /// Top-2 contributing factors in words.
    pub top_factors: Vec<String>,
    /// All five named inputs (the "Why this order?" expander).
    pub factors: Vec<(String, f64)>,
    /// Unknown headroom ranked without quota.
    pub headroom_unknown: bool,
    /// The confidence badge copy.
    pub confidence: String,
    /// TTE minutes for the pill, when a forecast exists.
    pub tte_min: Option<f64>,
    /// True for the top row (badged "Suggested").
    pub suggested: bool,
    /// The launch admission verdict (`ok|tight|no`, SPEC §5.2) for
    /// this account on the requested project — None when headroom or
    /// burn is unknown (rendered as "headroom unknown", never a
    /// guess). Warns; never blocks (AJ-E306 Req.7).
    pub admission: Option<String>,
}
