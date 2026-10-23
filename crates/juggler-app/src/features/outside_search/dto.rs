//! The opt-in outside search's wire types (AJ-E1538).

/// Where the index of outside sessions' words stands.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OutsideSearchInfo {
    /// The user turned it on (off by default).
    pub enabled: bool,
    /// The index's size on this Mac, bytes, while on.
    #[ts(type = "number | null")]
    pub size_bytes: Option<u64>,
    /// Sessions started outside AI Juggler.
    pub sessions: u32,
    /// How many of them are fully indexed.
    pub indexed: u32,
}

/// What building the index would take, before it is turned on.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OutsideSearchEstimateInfo {
    /// The index's likely size, bytes.
    #[ts(type = "number")]
    pub size_bytes: u64,
    /// Sessions started outside AI Juggler it would cover.
    pub sessions: u32,
}
