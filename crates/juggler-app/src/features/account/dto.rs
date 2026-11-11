//! The account types on the wire (exported to TypeScript).

use serde::Serialize;
use ts_rs::TS;

/// An account row for the UI.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AccountInfo {
    /// Account id.
    pub id: String,
    /// Provider id (wire field `harness` aliased one version; AJ-E107).
    #[serde(alias = "harness")]
    pub provider: String,
    /// User label.
    pub label: String,
    /// Plan tier, if set: its Account's once moved there (AJ-E3202).
    pub plan: Option<String>,
    /// Login state: `logged_in` or `logged_out`.
    pub login_state: String,
    /// D-08 Tier-1 cleanup opt-in (off by default — notify-only tier).
    pub cleanup_opt_in: bool,
    /// Optional user-entered renewal date, local calendar `YYYY-MM-DD`
    /// (D-12; its Account's once moved); `null`: `Expiry: unknown`.
    pub renewal_date: Option<String>,
    /// AJ-E415 per-Account live-preview switch (off by default; the
    /// enable is the Screen-Recording consent moment).
    pub preview_opt_in: bool,
    /// AJ-E307 Req.1/2: import provenance (source canonical path /
    /// origin / imported-at). `None` = provenance-unknown (pre-E307
    /// accounts — never guessed).
    #[ts(optional = nullable)]
    pub import_source_path: Option<String>,
    /// The discovery origin (`cc-switch|t3|alias|home-pattern|manual`).
    #[ts(optional = nullable)]
    pub import_origin: Option<String>,
    /// Import completion time, epoch ms (None = never imported).
    #[ts(optional = nullable)]
    pub imported_at: Option<i64>,
    /// ADR-0018: the account uses the user's own folder where it is.
    pub in_place: bool,
    /// AJ-E1501: new sessions start with this account unless a project
    /// names its own (set by the accounts list; `null` elsewhere).
    #[ts(optional = nullable)]
    pub is_default: Option<bool>,
    /// AJ-E1601: the service it runs through (`OpenCode Go`), for a
    /// profile added through another service; its sign-in state then says
    /// whether its key is in the Keychain.
    #[ts(optional = nullable)]
    pub service: Option<String>,
    /// D-62: the Account its sign-in was last read as (its plan, renewal
    /// and fee are set there), when known.
    #[ts(optional = nullable)]
    pub subscription_id: Option<String>,
    /// AJ-E3206: the colour of the folder's CLI App (`cli:<id>`), given
    /// once and stored (an index into the UI's `MARK_COLOURS`).
    #[ts(optional = nullable)]
    pub app_colour_slot: Option<u8>,
    /// AJ-E3206: the colour of its Account (`subscription_id`), when it
    /// has one, given the same way.
    #[ts(optional = nullable)]
    pub account_colour_slot: Option<u8>,
    /// AJ-E3002 R3: why its sessions get no agent tools (its adapter's
    /// words: they work through their shell); `null` when they get them.
    #[ts(optional = nullable)]
    pub agent_tools_why: Option<String>,
}

/// Result of importing a discovered setup as a managed account.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ImportResult {
    /// The created account.
    pub account: AccountInfo,
    /// Files copied into the managed dir.
    #[ts(type = "number")]
    pub files_copied: u64,
    /// Bytes copied into the managed dir.
    #[ts(type = "number")]
    pub bytes_copied: u64,
    /// Cache/noise entries skipped, human-readable.
    pub skipped: Vec<String>,
    /// Login state of the imported account: `logged_in` | `logged_out`.
    pub login: String,
    /// AJ-E307 Req.3: per top level entry bytes, largest first — the
    /// copy receipt's breakdown.
    pub folders: Vec<FolderSizeInfo>,
    /// True when this was the settings-only degraded import.
    pub degraded: bool,
    /// When login did not carry over, the plain-language reason and next
    /// step (e.g. macOS keychain path-locking).
    pub login_note: Option<String>,
}

/// AJ-E307 Req.3: one breakdown row (top-level entry + bytes).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FolderSizeInfo {
    /// Top-level folder (with `/`) or file name.
    pub name: String,
    /// Importable bytes under it.
    #[ts(type = "number")]
    pub bytes: u64,
}

/// AJ-E307 Req.3: the pre-copy source inspection on the wire.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
// A flat one-row wire payload for one panel — the honest shape here
// is four independent flags, not a state machine.
#[allow(
    clippy::struct_excessive_bools,
    reason = "wire DTO: each flag is its own fact the import sheet shows"
)]
pub struct SourceInspectInfo {
    /// Total importable bytes.
    #[ts(type = "number")]
    pub total_bytes: u64,
    /// Importable file count.
    #[ts(type = "number")]
    pub file_count: u64,
    /// Per-entry breakdown, largest first.
    pub folders: Vec<FolderSizeInfo>,
    /// The bounded walk hit its file cap (sizes are a floor).
    pub truncated: bool,
    /// The copy ceiling in bytes — the UI offers the degraded path
    /// from `total_bytes > ceiling_bytes`, never a raw ceiling string.
    #[ts(type = "number")]
    pub ceiling_bytes: u64,
    /// Config evidence — false renders the plain-language rejection.
    pub is_config: bool,
    /// Already managed by AI Juggler.
    pub already_managed: bool,
    /// Source login state: `logged_in` | `logged_out`.
    pub login: String,
    /// Keychain-bound login (signed in, no credentials file) — the
    /// re-login note shows at Confirm-source time (Req.8).
    pub keychain_bound: bool,
}
