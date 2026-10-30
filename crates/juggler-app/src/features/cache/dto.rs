//! The local cache's wire types (ADR-0023).

/// Where the local cache stands, for Settings › Advanced › Local cache.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CacheInfo {
    /// The user keeps a cache on this Mac (on by default).
    pub on: bool,
    /// The limit is worked out from this Mac (the default).
    pub automatic: bool,
    /// The fixed limit the user picked, megabytes.
    #[ts(type = "number | null")]
    pub limit_mb: Option<u64>,
    /// What the cache may use now, bytes (the automatic or fixed limit,
    /// or the floor on a nearly full disk).
    #[ts(type = "number")]
    pub budget_bytes: u64,
    /// What it uses, bytes.
    #[ts(type = "number")]
    pub used_bytes: u64,
    /// The disk is nearly full: the cache keeps to its floor and writes
    /// nothing new.
    pub low_disk: bool,
    /// Free space on the disk AI Juggler's folder is on, when known.
    #[ts(type = "number | null")]
    pub free_bytes: Option<u64>,
    /// Each cache.
    pub caches: Vec<CacheRowInfo>,
}

/// One cache.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CacheRowInfo {
    /// Its key, for the list (never shown).
    pub key: String,
    /// Its name.
    pub name: String,
    /// What it speeds up.
    pub speeds_up: String,
    /// What is slower once it's cleared.
    pub slower: String,
    /// Bytes on disk.
    #[ts(type = "number")]
    pub bytes: u64,
    /// Entries.
    #[ts(type = "number")]
    pub entries: u64,
    /// Kept only whole: never trimmed for the limit.
    pub whole_only: bool,
    /// Where its own switch is, when it has one.
    pub own_switch: Option<String>,
}
