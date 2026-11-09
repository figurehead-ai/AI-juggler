//! The switch types on the wire (exported to TypeScript).

use crate::features::redaction::dto::{RedactionItemInfo, RedactionPolicyInfo};

/// AJ-E205: the preflight verdict for one switch (rendered by the
/// freeze dialog / door before anything executes).
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
#[allow(
    clippy::struct_excessive_bools,
    reason = "wire DTO: each flag is its own fact the door renders"
)]
pub struct PreflightInfo {
    /// Mechanism wire name (`native-resume|transplant|replay|pack`).
    pub mechanism: String,
    /// The F-label shown (F0–F4).
    pub fidelity: String,
    /// AJ-E1543: the same label in plain words ("Continues exactly",
    /// "Full story, fresh start"): what the screen shows.
    pub fidelity_words: String,
    /// The capped score (internal honesty; the label is the UI).
    pub score: f64,
    /// §7.1 warning copy (verbatim table renders).
    pub warnings: Vec<String>,
    /// Zombie children listed at freeze (no action taken).
    pub zombies: Vec<String>,
    /// Whether an uncommitted diff snapshot rides the pack.
    pub diff_snapshotted: bool,
    /// Whether the source instance is live (freeze dialog needed).
    pub source_live: bool,
    /// 0.4: the conversation that moves, by its title and account, and
    /// (ADR-0028) the family that moves with it: the confirm step names
    /// them (flattened: `source_title`, `source_account`, `source_family`).
    #[serde(flatten)]
    pub moving: crate::features::handover::stays_behind::Moving,
    /// AJ-E1540: live, but not a session AI Juggler can stop (started
    /// outside it): the switch leaves it running and the copy is a
    /// separate continuation.
    pub source_keeps_running: bool,
    /// AJ-E2108: the source is a session another app runs: its
    /// conversation is only read, and the switch continues it as a new
    /// session here, leaving it as it is.
    pub source_outside: bool,
    /// The app that source is open in now ("Claude", "Terminal"), when it
    /// is open and the app is known.
    pub source_app: Option<String>,
    /// The F1-experimental transplant copy (until E210 proves pairs).
    pub experimental_copy: String,
    /// AJ-E206 §8 door verdict: `one-click` or `wizard`.
    #[ts(type = "\"one-click\" | \"wizard\"")]
    pub door: String,
    /// Wizard step titles in §8 order (empty for 1-click; only the
    /// non-trivial steps ever appear — never phantom steps).
    pub wizard_steps: Vec<String>,
    /// Why the wizard opened (one line per failed §8 condition;
    /// empty for 1-click).
    pub wizard_reasons: Vec<String>,
    /// The target is the project's dominant account (the usual
    /// test; unique max by session count, ties are not habits).
    pub usual_account: bool,
    /// The carry-list the door renders (carried / rebuilt / dropped
    /// names — the popover one-liner and the wizard's preview).
    pub carry: CarryListInfo,
    /// AJ-E208: the destination Provider's redaction policy (the
    /// verbatim confirm line; unset ⇒ the first-run chooser).
    pub redaction: RedactionPolicyInfo,
    /// AJ-E208: the source conversation's detected secrets (masked)
    /// — the per-item un-redact list.
    pub redaction_items: Vec<RedactionItemInfo>,
    /// AJ-E808: the pair is validated (the same verdict the switch rows
    /// and Settings › Beta show; strict on the installed versions).
    pub validated: bool,
    /// The carry list in plain words, a line on each.
    pub carry_items: Vec<crate::features::handover::switch_words::CarryItemInfo>,
    /// The Target step's first line: what moves, said positively.
    pub move_line: String,
    /// Whether the keys-and-passwords question applies, and who reads.
    pub secrets: crate::features::handover::switch_words::SecretsInfo,
    /// The target's tool (its id: the UI names it from the catalog).
    pub target_tool: String,
    /// The target's account (its profile id): the UI reads its room from
    /// the one runway, as Home does (the 0.2 walk, F1).
    pub target_account: String,
    /// 0.3.5: where it can continue, Desktop or CLI, and which is usual.
    pub destinations: crate::features::places::handoff::DestinationsInfo,
}

/// The carry-list grouped by disposition (AJ-E206; every name is a
/// concrete artifact — "some context" is forbidden copy).
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CarryListInfo {
    /// Artifacts that move or copy.
    pub carried: Vec<String>,
    /// Artifacts the target rebuilds (replay turn / re-derived).
    pub regenerated: Vec<String>,
    /// Artifacts that do not carry — must be listed (§8 1-click
    /// blocker; untranslated translates score here too, §H2).
    pub dropped: Vec<String>,
}

/// AJ-E206: the executed switch's handles — the new instance plus
/// the graph edge that `switch_undo` compensates.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SwitchExecutedInfo {
    /// The continuation instance's session id.
    pub session_id: String,
    /// The continuation instance's process id.
    pub pid: u32,
    /// The recorded edge (the undo handle).
    pub edge_id: String,
    /// The resting source session (what undo resumes).
    pub source_session_id: String,
    /// 0.3.5: where it continues, `desktop` or `cli`.
    pub continued_in: String,
    /// 0.3.5: in Desktop, the key of the conversation the app took
    /// (`conversation:<account>:<id>`): the page to open.
    pub conversation_key: Option<String>,
    /// ADR-0028: how many sessions started from it moved with it.
    pub family_moved: u32,
    /// ADR-0028: where they nest in the target app: `now`, `restart` (at
    /// the app's next start; [`Self::family_place`] restarts it), or empty
    /// (no app to nest them in: a terminal switch).
    pub family_nests: String,
    /// ADR-0028: the target app profile the family waits for.
    pub family_place: Option<String>,
    /// ADR-0028: what woke the original by itself and the copy now
    /// takes over (Auto-fix on a pull request, a goal), in words.
    pub automation_moved: Vec<String>,
    /// The copy's app profile when the copy takes it over only once that
    /// app restarts (the done step offers Restart).
    pub automation_copy_place: Option<String>,
    /// Whether the original's is turned off only once its app closes.
    pub automation_original_waits: bool,
    /// What stays with the original (the switch went to a terminal), in
    /// words.
    pub automation_kept: Vec<String>,
}

// ---------------------------------------------------------------------------
// AJ-E207: switch-back — resume the original native session plus ONE
// delta turn; never a copy-back (D-05; pass2/02 §6.1-6.2).

/// One listed conflict for the review UI (three radio choices each,
/// keyboard-completable — never auto-merged).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FileConflictInfo {
    /// Pack-relative path (the carry-list name).
    pub rel_path: String,
    /// Fork sha256, when the base manifest tracked it.
    pub base_sha: Option<String>,
    /// Away-side sha256 today (`null` = deleted there).
    pub away_sha: Option<String>,
    /// Original-side sha256 today (`null` = deleted there).
    pub original_sha: Option<String>,
}

impl FileConflictInfo {
    pub(crate) fn of(conflict: &juggler_handover::reconcile::FileConflict) -> Self {
        Self {
            rel_path: conflict.rel_path.clone(),
            base_sha: conflict.base_sha.clone(),
            away_sha: conflict.away_sha.clone(),
            original_sha: conflict.original_sha.clone(),
        }
    }
}

/// The switch-back plan the suggestion card / review modal renders
/// (suggest-only; the user clicks — SPEC:26,263).
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SwitchBackPlanInfo {
    /// The verbatim quota-reset suggestion (Req.3), naming the original
    /// folder's Account (D-62: the reset is its).
    pub suggestion: String,
    /// The original account's label (where work returns).
    pub original_account: String,
    /// The away account's label.
    pub away_account: String,
    /// The native id the return resumes (empty when S-F7).
    pub original_native_id: String,
    /// The original transcript still exists.
    pub original_alive: bool,
    /// S-F7 fallback wire name ("" | "nearest-epoch" | "fresh-f0").
    pub sf7_fallback: String,
    /// §7.1 S-F7 copy when the original is gone.
    pub sf7_copy: String,
    /// The away instance is still live (freeze-or-fork offer).
    pub away_live: bool,
    /// The both-live warning copy (§6.2 — silence is how
    /// wrong-account billing happens).
    pub both_live_copy: String,
    /// The divergence headline (§7.1 shape: counts, never "some
    /// context"; nothing merged silently).
    pub divergence: String,
    /// The rendered "While you were away" delta turn (preview).
    pub delta_preview: String,
    /// Away-only turn count (the delta body).
    pub away_only_turns: u32,
    /// Original-only turn count (already native; no-op).
    pub original_only_turns: u32,
    /// Shared (fork) turn count.
    pub base_turns: u32,
    /// Tracked-artifact conflicts needing a choice.
    pub conflicts: Vec<FileConflictInfo>,
    /// AJ-E1541: every account this work can go back to, nearest first
    /// (A → B → Codex: B, then A, where it started).
    pub returns: Vec<SwitchBackReturnInfo>,
    /// The epoch this plan returns to (one of `returns`).
    pub return_to: String,
}

/// One account a switch back can return to (AJ-E1541).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SwitchBackReturnInfo {
    /// The epoch to pass back as `to_epoch`.
    pub epoch_id: String,
    /// The account's label.
    pub account: String,
    /// Whether this is where the work started.
    pub start: bool,
}

/// One user resolution from the review UI.
#[derive(Clone, PartialEq, Eq, Debug, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SwitchBackChoiceIn {
    /// The conflicting path.
    pub rel_path: String,
    /// `keep-original|take-away|keep-both`.
    pub choice: String,
}

/// One suggested return (suggest-only — the user clicks).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SwitchBackSuggestionInfo {
    /// The away instance to return FROM (the card's target).
    pub session_id: String,
    /// The original account's label (where work returns).
    pub original_account: String,
    /// The original account's id (its "has room again" notice offers this).
    pub original_account_id: String,
    /// The away account's label.
    pub away_account: String,
    /// The verbatim suggestion copy (Req.3), naming the original
    /// folder's Account (D-62: the reset is its).
    pub copy: String,
}

// ---------------------------------------------------------------------------
// AJ-E209: the badged switch entry point, the Switch Graph, and the
// per-node actions (Resume / Export pack) — ranking suggests only.

/// 0.3.4: why a switch target is held back, when a step of the user's
/// fixes it (the words are its `filtered_reason`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
#[serde(rename_all = "snake_case")]
pub enum HeldBack {
    /// Its tool's first-run setup isn't done there: a session started on
    /// it once does it.
    SetUp,
}

/// One ranked, badged switch destination (the handover section's
/// rows). Suggest-only: the click is the user's.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
#[allow(
    clippy::struct_excessive_bools,
    reason = "a target's facts, each its own: validated, cross-tool, suggested, first run"
)]
pub struct SwitchTargetInfo {
    /// The launch recipe the switch would run under.
    pub recipe_id: String,
    /// Its Provider id (AJ-E1103: a suggestion is counted by Provider).
    pub provider: String,
    /// Its account id.
    pub account_id: String,
    /// Its account label.
    pub account_label: String,
    /// The ordered-pair badge wire name ("Full story, fresh start · Beta" — never
    /// "lossless" below F4; cross-Provider always carries Beta).
    pub badge: String,
    /// Validated state (false until E210's fixtures flip cells).
    pub validated: bool,
    /// Cross-Provider pair (badge + Drop list are mandatory).
    pub cross_provider: bool,
    /// The pair's Drop list (the badge's honest companion).
    pub drops: Vec<String>,
    /// The §Q6 score (`None` = hard-filtered, reason alongside).
    pub score: Option<f64>,
    /// Hard-filter reason, when filtered; 0.3.4: also what would stop a
    /// launch there (its tool's own check), so it's never offered as
    /// ready.
    pub filtered_reason: Option<String>,
    /// 0.3.4: why it's held back, when a step of the user's fixes it.
    pub held_back: Option<HeldBack>,
    /// 0.3.5: its tool hasn't done its own first run there (a Claude app
    /// profile's folder): the switch finishes it, so it's offered, and its
    /// row says so.
    pub first_run: bool,
    /// 0.4.1 (the rc5 walk): its tool isn't signed in there, in a
    /// terminal. Its account's room isn't its own to show: the row says
    /// "Not signed in" instead of a percentage, as New session does.
    pub signed_out: bool,
    /// 0.4.1: a desktop app profile signed in on its account takes the
    /// switch (Desktop), whatever the terminal's sign-in: its row shows
    /// the account's room, and "Terminal: not signed in" beside.
    pub desktop: bool,
    /// Top-2 contributing factors in words.
    pub top_factors: Vec<String>,
    /// All inputs for the "Why this order?" expander.
    pub factors: Vec<(String, f64)>,
    /// Confidence badge (Measured/Computed/Estimate).
    pub confidence: String,
    /// Minutes-to-exhaustion, when known.
    pub tte_min: Option<f64>,
    /// The top survivor (suggest-only).
    pub suggested: bool,
}

/// One epoch node on the timeline.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct EpochNodeInfo {
    /// Epoch id.
    pub id: String,
    /// Provider id (the icon).
    pub provider: String,
    /// Account label.
    pub account_label: String,
    /// Instance-kind badge.
    pub kind: String,
    /// A native session exists AND its transcript survives (the
    /// Resume action's honest gate).
    pub resumable: bool,
    /// The live session id, when one is attached.
    pub session_id: Option<String>,
    /// AJ-E1904: its session's task (`aj-7f3c · title`), when it has one.
    pub task: Option<String>,
}

/// One labelled edge on the timeline.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct EdgeInfo {
    /// The edge's id (what opens its handover).
    pub id: String,
    /// From epoch id.
    pub from: String,
    /// To epoch id (None until launched).
    pub to: Option<String>,
    /// The arrow's label in plain words: what carried ("Full story,
    /// fresh start"), "Switched back · …" for a return, or the failure
    /// (AJ-E1535: never "transplant F1" or "switch-back ↺").
    pub label: String,
    /// The §9 reason, when the edge failed (dashed red + recovery).
    pub failed_reason: Option<String>,
    /// Switch-back edge (renders ↺).
    pub switch_back: bool,
    /// Its handover in one line, when the switch wrote one (another
    /// tool's); `None` for one that copied the session.
    pub handover: Option<crate::features::handover::view::HandoverSummaryInfo>,
}

/// One work item's timeline.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct WorkGraphInfo {
    /// Work title.
    pub title: String,
    /// Epochs newer than the last fork/merge, in order.
    pub epochs: Vec<EpochNodeInfo>,
    /// Their edges.
    pub edges: Vec<EdgeInfo>,
    /// Older epochs collapsed away ("+n switches" — never unbounded).
    pub collapsed: u32,
    /// The S-F9 hop line (inform, never block).
    pub hop_note: Option<String>,
}
